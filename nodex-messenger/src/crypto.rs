use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use hmac::Mac;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

use nodex_kademlia::node::NodeId;

pub const ENVELOPE_VERSION: u8 = 3;
pub const MAX_REPLAY_WINDOW_SECS: u64 = 7 * 86400; // 7 days mailbox retention window

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedEnvelope {
    #[serde(default = "default_envelope_version")]
    pub version: u8,
    pub sender_id: NodeId,
    pub sender_ed25519_pub: Vec<u8>,
    pub sender_x25519_pub: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ephemeral_x25519_pub: Option<Vec<u8>>,
    pub recipient_id: NodeId,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: u64,
}

fn default_envelope_version() -> u8 {
    1
}

#[derive(Clone)]
pub struct UserIdentity {
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
    pub x25519_secret: X25519StaticSecret,
    pub x25519_public: X25519PublicKey,
    pub user_id: NodeId,
}

impl UserIdentity {
    pub fn user_id_hex(&self) -> String {
        self.user_id.to_hex()
    }

    pub fn fingerprint(&self) -> String {
        let raw = self.verifying_key.as_bytes();
        let hash = core_ffi::ffi_hash_node_id(raw);
        let hex = core_ffi::ffi_node_id_to_hex(&hash);
        let chunks: Vec<&str> = (0..hex.len()).step_by(4).map(|i| &hex[i..i + 4]).collect();
        chunks.join(":")
    }

    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut seed);
        Self::from_seed(&seed)
    }

    pub fn generate_mnemonic() -> Result<(String, Self), String> {
        let mut entropy = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = core_ffi::ffi_mnemonic_generate_12(&entropy)?;
        let seed = core_ffi::ffi_mnemonic_to_seed(&mnemonic)?;
        Ok((mnemonic, Self::from_seed(&seed)))
    }

    pub fn generate_mnemonic_24() -> Result<(String, Self), String> {
        let mut entropy = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut entropy);
        let mnemonic = core_ffi::ffi_mnemonic_generate_24(&entropy)?;
        let seed = core_ffi::ffi_mnemonic_to_seed(&mnemonic)?;
        Ok((mnemonic, Self::from_seed(&seed)))
    }

    pub fn from_mnemonic(mnemonic_str: &str) -> Result<Self, String> {
        let seed = core_ffi::ffi_mnemonic_to_seed(mnemonic_str)?;
        Ok(Self::from_seed(&seed))
    }

    /// Derive separate, cryptographically isolated keys for signing (Ed25519)
    /// and encryption (X25519) using HKDF-SHA256 domain separation.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let hk = Hkdf::<Sha256>::new(None, seed);

        let mut ed25519_seed = [0u8; 32];
        hk.expand(b"NodeX-Ed25519-Sign-v1", &mut ed25519_seed)
            .expect("32 bytes is valid length for HKDF-SHA256");
        let signing_key = SigningKey::from_bytes(&ed25519_seed);
        let verifying_key = signing_key.verifying_key();

        let mut x25519_seed = [0u8; 32];
        hk.expand(b"NodeX-X25519-Static-v1", &mut x25519_seed)
            .expect("32 bytes is valid length for HKDF-SHA256");
        let x25519_secret = X25519StaticSecret::from(x25519_seed);
        let x25519_public = X25519PublicKey::from(&x25519_secret);

        let user_id = NodeId::from_key(verifying_key.as_bytes());

        Self {
            signing_key,
            verifying_key,
            x25519_secret,
            x25519_public,
            user_id,
        }
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    pub fn verify(
        verifying_key_bytes: &[u8],
        message: &[u8],
        signature_bytes: &[u8],
    ) -> bool {
        let Ok(vk_arr): Result<[u8; 32], _> = verifying_key_bytes.try_into() else {
            return false;
        };
        let Ok(vk) = VerifyingKey::from_bytes(&vk_arr) else {
            return false;
        };
        let Ok(sig_arr): Result<[u8; 64], _> = signature_bytes.try_into() else {
            return false;
        };
        let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);
        vk.verify(message, &sig).is_ok()
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.verifying_key.as_bytes()
    }
}

/// In-memory replay protection cache tracking nonces and signatures within the sliding window.
pub struct ReplayProtectionCache {
    seen_hashes: RwLock<HashMap<[u8; 32], u64>>,
}

impl Default for ReplayProtectionCache {
    fn default() -> Self {
        Self {
            seen_hashes: RwLock::new(HashMap::new()),
        }
    }
}

impl ReplayProtectionCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check_and_insert(&self, signature: &[u8], timestamp: u64, now: u64) -> bool {
        // Discard expired messages outside sliding time window
        if now.abs_diff(timestamp) > MAX_REPLAY_WINDOW_SECS {
            return false;
        }

        let mut hasher = sha2::Sha256::default();
        use sha2::Digest;
        hasher.update(signature);
        let hash: [u8; 32] = hasher.finalize().into();

        let mut lock = self.seen_hashes.write().unwrap();
        // Evict expired entries if cache gets large
        if lock.len() > 10000 {
            lock.retain(|_, &mut entry_ts| now.saturating_sub(entry_ts) <= MAX_REPLAY_WINDOW_SECS);
        }

        if lock.contains_key(&hash) {
            return false; // Replay detected!
        }
        lock.insert(hash, timestamp);
        true
    }
}

/// Derive a forward-secret, privacy-preserving DHT Mailbox key using HMAC-SHA256 over current day bucket.
pub fn derive_secure_mailbox_key(user_id_hex: &str, timestamp: u64) -> String {
    let day_bucket = timestamp / 86400;
    let mut mac = <hmac::Hmac<Sha256> as Mac>::new_from_slice(user_id_hex.as_bytes())
        .expect("HMAC can take any key size");
    mac.update(&day_bucket.to_be_bytes());
    let result = mac.finalize().into_bytes();
    let hex: String = result[..16].iter().map(|b| format!("{:02x}", b)).collect();
    format!("mbx_{}", hex)
}

/// Encrypt an envelope with Perfect Forward Secrecy (PFS): generates a fresh ephemeral X25519 key.
pub fn encrypt_envelope(
    sender: &UserIdentity,
    recipient_user_id: NodeId,
    recipient_x25519_pub: &X25519PublicKey,
    plaintext: &[u8],
) -> Result<EncryptedEnvelope, String> {
    // 1. Generate ephemeral key pair for Perfect Forward Secrecy (PFS)
    let mut eph_seed = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut eph_seed);
    let eph_secret = X25519StaticSecret::from(eph_seed);
    let eph_public = X25519PublicKey::from(&eph_secret);

    // 2. Perform 2-step Diffie-Hellman (Ephemeral-Static + Static-Static)
    let dh_ephemeral = eph_secret.diffie_hellman(recipient_x25519_pub);
    let dh_static = sender.x25519_secret.diffie_hellman(recipient_x25519_pub);

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut ikm = Vec::with_capacity(64);
    ikm.extend_from_slice(dh_ephemeral.as_bytes());
    ikm.extend_from_slice(dh_static.as_bytes());

    let hk = Hkdf::<Sha256>::new(Some(&nonce_bytes), &ikm);
    let mut cipher_key = [0u8; 32];
    hk.expand(b"NodeX-ChaCha20Poly1305-PayloadKey-v2", &mut cipher_key)
        .map_err(|e| format!("HKDF expand failed: {:?}", e))?;

    let cipher = ChaCha20Poly1305::new_from_slice(&cipher_key)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Envelope v3 signs all fields including domain separator, public keys, nonce, and timestamp
    let mut sign_payload = Vec::new();
    sign_payload.extend_from_slice(b"NodeX-envelope-v3");
    sign_payload.push(ENVELOPE_VERSION);
    sign_payload.extend_from_slice(sender.user_id.as_bytes());
    sign_payload.extend_from_slice(recipient_user_id.as_bytes());
    sign_payload.extend_from_slice(sender.x25519_public.as_bytes());
    sign_payload.extend_from_slice(eph_public.as_bytes());
    sign_payload.extend_from_slice(&nonce_bytes);
    sign_payload.extend_from_slice(&timestamp.to_be_bytes());
    sign_payload.extend_from_slice(&ciphertext);

    let signature = sender.sign(&sign_payload);

    Ok(EncryptedEnvelope {
        version: ENVELOPE_VERSION,
        sender_id: sender.user_id,
        sender_ed25519_pub: sender.verifying_key.to_bytes().to_vec(),
        sender_x25519_pub: sender.x25519_public.as_bytes().to_vec(),
        ephemeral_x25519_pub: Some(eph_public.as_bytes().to_vec()),
        recipient_id: recipient_user_id,
        nonce: nonce_bytes.to_vec(),
        ciphertext,
        signature,
        timestamp,
    })
}

/// Decrypt an envelope verifying Ed25519 signature, sender_id identity binding, and deriving the PFS payload key.
pub fn decrypt_envelope(
    recipient: &UserIdentity,
    envelope: &EncryptedEnvelope,
) -> Result<Vec<u8>, String> {
    // 0. Strict structural validation to prevent Nonce/Key from_slice panics
    if envelope.nonce.len() != 12
        || envelope.signature.len() != 64
        || envelope.sender_ed25519_pub.len() != 32
        || envelope.sender_x25519_pub.len() != 32
        || envelope.ephemeral_x25519_pub.as_ref().map_or(false, |e| e.len() != 32)
        || envelope.recipient_id != recipient.user_id
    {
        return Err("Malformed envelope: invalid field lengths or recipient mismatch".into());
    }

    // 1. Mandatory P0 check: Cryptographic binding between sender_id and sender_ed25519_pub
    let expected_sender_id = NodeId::from_key(&envelope.sender_ed25519_pub);
    if expected_sender_id != envelope.sender_id {
        return Err("sender_id does not match sender ed25519 public key".into());
    }

    let Ok(sender_x25519_arr): Result<[u8; 32], _> = envelope.sender_x25519_pub.as_slice().try_into() else {
        return Err("Invalid X25519 public key length".into());
    };
    let sender_x25519_pub = X25519PublicKey::from(sender_x25519_arr);

    // Verify signature according to envelope version
    if envelope.version >= 3 {
        let Some(ref eph_bytes) = envelope.ephemeral_x25519_pub else {
            return Err("Missing ephemeral key in v3 envelope".into());
        };
        let mut sign_payload = Vec::new();
        sign_payload.extend_from_slice(b"NodeX-envelope-v3");
        sign_payload.push(envelope.version);
        sign_payload.extend_from_slice(envelope.sender_id.as_bytes());
        sign_payload.extend_from_slice(envelope.recipient_id.as_bytes());
        sign_payload.extend_from_slice(envelope.sender_x25519_pub.as_slice());
        sign_payload.extend_from_slice(eph_bytes.as_slice());
        sign_payload.extend_from_slice(envelope.nonce.as_slice());
        sign_payload.extend_from_slice(&envelope.timestamp.to_be_bytes());
        sign_payload.extend_from_slice(&envelope.ciphertext);

        if !UserIdentity::verify(
            &envelope.sender_ed25519_pub,
            &sign_payload,
            &envelope.signature,
        ) {
            return Err("Envelope signature verification failed! Message may be tampered.".into());
        }

        let Ok(eph_arr): Result<[u8; 32], _> = eph_bytes.as_slice().try_into() else {
            return Err("Invalid ephemeral key length".into());
        };
        let eph_pub = X25519PublicKey::from(eph_arr);

        let dh_ephemeral = recipient.x25519_secret.diffie_hellman(&eph_pub);
        let dh_static = recipient.x25519_secret.diffie_hellman(&sender_x25519_pub);

        let mut ikm = Vec::with_capacity(64);
        ikm.extend_from_slice(dh_ephemeral.as_bytes());
        ikm.extend_from_slice(dh_static.as_bytes());

        let hk = Hkdf::<Sha256>::new(Some(&envelope.nonce), &ikm);
        let mut cipher_key = [0u8; 32];
        hk.expand(b"NodeX-ChaCha20Poly1305-PayloadKey-v2", &mut cipher_key)
            .map_err(|e| format!("HKDF expand failed: {:?}", e))?;

        let cipher = ChaCha20Poly1305::new_from_slice(&cipher_key)
            .map_err(|e| format!("Cipher init failed: {}", e))?;
        let nonce = Nonce::from_slice(&envelope.nonce);

        let plaintext = cipher
            .decrypt(nonce, envelope.ciphertext.as_slice())
            .map_err(|_| "Decryption failed! Key mismatch or corrupted ciphertext.".to_string())?;

        Ok(plaintext)
    } else if envelope.version == 2 {
        let Some(ref eph_bytes) = envelope.ephemeral_x25519_pub else {
            return Err("Missing ephemeral key in v2 envelope".into());
        };
        let mut sign_payload = Vec::new();
        sign_payload.push(envelope.version);
        sign_payload.extend_from_slice(envelope.sender_id.as_bytes());
        sign_payload.extend_from_slice(envelope.recipient_id.as_bytes());
        sign_payload.extend_from_slice(eph_bytes);
        sign_payload.extend_from_slice(&envelope.timestamp.to_be_bytes());
        sign_payload.extend_from_slice(&envelope.ciphertext);

        if !UserIdentity::verify(
            &envelope.sender_ed25519_pub,
            &sign_payload,
            &envelope.signature,
        ) {
            return Err("Envelope signature verification failed! Message may be tampered.".into());
        }

        let Ok(eph_arr): Result<[u8; 32], _> = eph_bytes.as_slice().try_into() else {
            return Err("Invalid ephemeral key length".into());
        };
        let eph_pub = X25519PublicKey::from(eph_arr);

        let dh_ephemeral = recipient.x25519_secret.diffie_hellman(&eph_pub);
        let dh_static = recipient.x25519_secret.diffie_hellman(&sender_x25519_pub);

        let mut ikm = Vec::with_capacity(64);
        ikm.extend_from_slice(dh_ephemeral.as_bytes());
        ikm.extend_from_slice(dh_static.as_bytes());

        let hk = Hkdf::<Sha256>::new(Some(&envelope.nonce), &ikm);
        let mut cipher_key = [0u8; 32];
        hk.expand(b"NodeX-ChaCha20Poly1305-PayloadKey-v2", &mut cipher_key)
            .map_err(|e| format!("HKDF expand failed: {:?}", e))?;

        let cipher = ChaCha20Poly1305::new_from_slice(&cipher_key)
            .map_err(|e| format!("Cipher init failed: {}", e))?;
        let nonce = Nonce::from_slice(&envelope.nonce);

        let plaintext = cipher
            .decrypt(nonce, envelope.ciphertext.as_slice())
            .map_err(|_| "Decryption failed! Key mismatch or corrupted ciphertext.".to_string())?;

        Ok(plaintext)
    } else {
        // Legacy v1 static-static decryption for backward compatibility
        let mut sign_payload = Vec::new();
        sign_payload.extend_from_slice(envelope.sender_id.as_bytes());
        sign_payload.extend_from_slice(envelope.recipient_id.as_bytes());
        sign_payload.extend_from_slice(&envelope.timestamp.to_be_bytes());
        sign_payload.extend_from_slice(&envelope.ciphertext);

        if !UserIdentity::verify(
            &envelope.sender_ed25519_pub,
            &sign_payload,
            &envelope.signature,
        ) {
            return Err("Envelope signature verification failed! Message may be tampered.".into());
        }

        let shared_secret = recipient.x25519_secret.diffie_hellman(&sender_x25519_pub);
        let cipher = ChaCha20Poly1305::new_from_slice(shared_secret.as_bytes())
            .map_err(|e| format!("Cipher init failed: {}", e))?;
        let nonce = Nonce::from_slice(&envelope.nonce);

        let plaintext = cipher
            .decrypt(nonce, envelope.ciphertext.as_slice())
            .map_err(|_| "Decryption failed! Key mismatch or corrupted ciphertext.".to_string())?;

        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_user_identity_and_pfs_e2ee() {
        let alice = UserIdentity::generate();
        let bob = UserIdentity::generate();

        let secret_msg = b"Hello Bob! This is a secure P2P E2EE message with Perfect Forward Secrecy.";

        let envelope = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, secret_msg).unwrap();
        assert_eq!(envelope.version, ENVELOPE_VERSION);
        assert!(envelope.ephemeral_x25519_pub.is_some());

        let decrypted = decrypt_envelope(&bob, &envelope).unwrap();
        assert_eq!(decrypted, secret_msg);
    }

    #[test]
    fn crypto_tampered_signature_fails() {
        let alice = UserIdentity::generate();
        let bob = UserIdentity::generate();

        let secret_msg = b"Tamper test message";
        let mut envelope = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, secret_msg).unwrap();

        envelope.signature[0] ^= 0xFF;

        let res = decrypt_envelope(&bob, &envelope);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("signature verification failed"));
    }

    #[test]
    fn crypto_replay_protection_cache() {
        let cache = ReplayProtectionCache::new();
        let sig = [42u8; 64];
        let now = 1000000;

        assert!(cache.check_and_insert(&sig, now, now));
        assert!(!cache.check_and_insert(&sig, now, now), "Duplicate signature must be rejected");

        // Expired message outside sliding window (7 days = 604_800s)
        assert!(!cache.check_and_insert(&[99u8; 64], now - 700_000, now));
    }

    #[test]
    fn crypto_sender_id_spoofing_rejected() {
        let alice = UserIdentity::generate();
        let bob = UserIdentity::generate();
        let eve = UserIdentity::generate();

        let secret_msg = b"Spoofed message claiming to be from Alice";
        // Eve creates an envelope with Eve's keys but Alice's sender_id
        let mut envelope = encrypt_envelope(&eve, bob.user_id, &bob.x25519_public, secret_msg).unwrap();
        // Eve replaces sender_id with Alice's sender_id
        envelope.sender_id = alice.user_id;

        let res = decrypt_envelope(&bob, &envelope);
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("sender_id does not match sender ed25519 public key"));
    }

    #[test]
    fn crypto_v3_tampered_nonce_or_timestamp_fails() {
        let alice = UserIdentity::generate();
        let bob = UserIdentity::generate();

        let secret_msg = b"Integrity check";
        let mut env1 = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, secret_msg).unwrap();
        env1.nonce[0] ^= 0x01; // tamper nonce
        assert!(decrypt_envelope(&bob, &env1).is_err());

        let mut env2 = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, secret_msg).unwrap();
        env2.timestamp += 1; // tamper timestamp
        assert!(decrypt_envelope(&bob, &env2).is_err());
    }
}

