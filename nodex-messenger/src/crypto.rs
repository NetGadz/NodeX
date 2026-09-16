use std::time::{SystemTime, UNIX_EPOCH};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Nonce};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret as X25519StaticSecret};

use nodex_kademlia::node::NodeId;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptedEnvelope {
    pub sender_id: NodeId,
    pub sender_ed25519_pub: Vec<u8>,
    pub sender_x25519_pub: Vec<u8>,
    pub recipient_id: NodeId,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub signature: Vec<u8>,
    pub timestamp: u64,
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

    pub fn from_mnemonic(mnemonic_str: &str) -> Result<Self, String> {
        let seed = core_ffi::ffi_mnemonic_to_seed(mnemonic_str)?;
        Ok(Self::from_seed(&seed))
    }

    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(seed);
        let verifying_key = signing_key.verifying_key();
        
        let x25519_secret = X25519StaticSecret::from(*seed);
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
}

pub fn encrypt_envelope(
    sender: &UserIdentity,
    recipient_user_id: NodeId,
    recipient_x25519_pub: &X25519PublicKey,
    plaintext: &[u8],
) -> Result<EncryptedEnvelope, String> {
    let shared_secret = sender.x25519_secret.diffie_hellman(recipient_x25519_pub);
    let cipher_key = shared_secret.as_bytes();

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher = ChaCha20Poly1305::new_from_slice(cipher_key)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut sign_payload = Vec::new();
    sign_payload.extend_from_slice(sender.user_id.as_bytes());
    sign_payload.extend_from_slice(recipient_user_id.as_bytes());
    sign_payload.extend_from_slice(&timestamp.to_be_bytes());
    sign_payload.extend_from_slice(&ciphertext);

    let signature = sender.sign(&sign_payload);

    Ok(EncryptedEnvelope {
        sender_id: sender.user_id,
        sender_ed25519_pub: sender.verifying_key.to_bytes().to_vec(),
        sender_x25519_pub: sender.x25519_public.as_bytes().to_vec(),
        recipient_id: recipient_user_id,
        nonce: nonce_bytes.to_vec(),
        ciphertext,
        signature,
        timestamp,
    })
}

pub fn decrypt_envelope(
    recipient: &UserIdentity,
    envelope: &EncryptedEnvelope,
) -> Result<Vec<u8>, String> {
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

    let Ok(x25519_arr): Result<[u8; 32], _> = envelope.sender_x25519_pub.as_slice().try_into() else {
        return Err("Invalid X25519 public key length".into());
    };
    let sender_x25519_pub = X25519PublicKey::from(x25519_arr);
    let shared_secret = recipient.x25519_secret.diffie_hellman(&sender_x25519_pub);
    let cipher_key = shared_secret.as_bytes();

    let cipher = ChaCha20Poly1305::new_from_slice(cipher_key)
        .map_err(|e| format!("Cipher init failed: {}", e))?;
    let nonce = Nonce::from_slice(&envelope.nonce);

    let plaintext = cipher
        .decrypt(nonce, envelope.ciphertext.as_slice())
        .map_err(|_| "Decryption failed! Key mismatch or corrupted ciphertext.".to_string())?;

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crypto_user_identity_and_e2ee() {
        let alice = UserIdentity::generate();
        let bob = UserIdentity::generate();

        let secret_msg = b"Hello Bob! This is a secure P2P E2EE message from Alice.";

        let envelope = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, secret_msg).unwrap();
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
}
