use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use x25519_dalek::{EphemeralSecret, PublicKey, StaticSecret};
use sha2::{Digest, Sha256};
use crate::node::NodeId;

pub const ONION_MAGIC: &[u8; 4] = b"NXON";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnionPacket {
    pub ephemeral_x25519_pub: [u8; 32],
    pub nonce: [u8; 12],
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OnionRelayHeader {
    pub next_hop_addr: SocketAddr,
    pub next_hop_id: NodeId,
    pub inner_packet: OnionPacket,
}

pub struct OnionRouter;

impl OnionRouter {
    /// Derives symmetric shared key between ephemeral secret and peer public key with domain separation.
    fn derive_shared_key(ephemeral_secret: EphemeralSecret, peer_public: &PublicKey) -> ([u8; 32], Key) {
        let ephemeral_public = PublicKey::from(&ephemeral_secret);
        let shared_secret = ephemeral_secret.diffie_hellman(peer_public);

        let mut hasher = Sha256::new();
        hasher.update(b"NODEX_ONION_2HOP_V1");
        hasher.update(shared_secret.as_bytes());
        hasher.update(ephemeral_public.as_bytes());
        let derived = hasher.finalize();

        let key = *Key::from_slice(&derived);
        (*ephemeral_public.as_bytes(), key)
    }

    /// Derives symmetric shared key on receiver side using receiver static secret and incoming ephemeral public key.
    fn derive_receiver_key(static_secret: &StaticSecret, ephemeral_public_bytes: &[u8; 32]) -> Key {
        let ephemeral_public = PublicKey::from(*ephemeral_public_bytes);
        let shared_secret = static_secret.diffie_hellman(&ephemeral_public);

        let mut hasher = Sha256::new();
        hasher.update(b"NODEX_ONION_2HOP_V1");
        hasher.update(shared_secret.as_bytes());
        hasher.update(ephemeral_public_bytes);
        let derived = hasher.finalize();

        *Key::from_slice(&derived)
    }

    /// Creates a 2-hop blinded onion packet:
    /// [Relay Hop] -> [Final Recipient Hop]
    pub fn build_2hop_onion(
        final_recipient_pub: &PublicKey,
        relay_pub: &PublicKey,
        _relay_addr: SocketAddr,
        final_addr: SocketAddr,
        final_id: NodeId,
        plaintext_payload: &[u8],
    ) -> Result<OnionPacket, String> {
        // 1. Encrypt inner layer for final recipient
        let final_ephemeral = EphemeralSecret::random_from_rng(rand::rngs::OsRng);
        let (final_eph_pub, final_key) = Self::derive_shared_key(final_ephemeral, final_recipient_pub);

        let mut final_nonce_bytes = [0u8; 12];
        getrandom::getrandom(&mut final_nonce_bytes).map_err(|e| e.to_string())?;
        let final_cipher = ChaCha20Poly1305::new(&final_key);
        let final_ciphertext = final_cipher
            .encrypt(Nonce::from_slice(&final_nonce_bytes), plaintext_payload)
            .map_err(|e| format!("Final onion layer encryption failed: {:?}", e))?;

        let inner_packet = OnionPacket {
            ephemeral_x25519_pub: final_eph_pub,
            nonce: final_nonce_bytes,
            payload: final_ciphertext,
        };

        // 2. Encrypt outer layer for relay node
        let relay_header = OnionRelayHeader {
            next_hop_addr: final_addr,
            next_hop_id: final_id,
            inner_packet,
        };
        let relay_header_bytes = serde_json::to_vec(&relay_header)
            .map_err(|e| format!("Failed to serialize onion relay header: {}", e))?;

        let relay_ephemeral = EphemeralSecret::random_from_rng(rand::rngs::OsRng);
        let (relay_eph_pub, relay_key) = Self::derive_shared_key(relay_ephemeral, relay_pub);

        let mut relay_nonce_bytes = [0u8; 12];
        getrandom::getrandom(&mut relay_nonce_bytes).map_err(|e| e.to_string())?;
        let relay_cipher = ChaCha20Poly1305::new(&relay_key);
        let relay_ciphertext = relay_cipher
            .encrypt(Nonce::from_slice(&relay_nonce_bytes), relay_header_bytes.as_ref())
            .map_err(|e| format!("Relay onion layer encryption failed: {:?}", e))?;

        Ok(OnionPacket {
            ephemeral_x25519_pub: relay_eph_pub,
            nonce: relay_nonce_bytes,
            payload: relay_ciphertext,
        })
    }

    /// Process packet at Relay Hop: Decrypts outer layer, returns forwarding destination and inner packet.
    pub fn process_at_relay(
        relay_static_secret: &StaticSecret,
        packet: &OnionPacket,
    ) -> Result<OnionRelayHeader, String> {
        let key = Self::derive_receiver_key(relay_static_secret, &packet.ephemeral_x25519_pub);
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce = Nonce::from_slice(&packet.nonce);

        let plaintext = cipher
            .decrypt(nonce, packet.payload.as_ref())
            .map_err(|_| "Relay layer decryption failed".to_string())?;

        let header: OnionRelayHeader = serde_json::from_slice(&plaintext)
            .map_err(|e| format!("Failed to parse onion relay header: {}", e))?;

        Ok(header)
    }

    /// Process packet at Final Hop: Decrypts innermost layer to reveal final plaintext payload.
    pub fn process_at_destination(
        destination_static_secret: &StaticSecret,
        packet: &OnionPacket,
    ) -> Result<Vec<u8>, String> {
        let key = Self::derive_receiver_key(destination_static_secret, &packet.ephemeral_x25519_pub);
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce = Nonce::from_slice(&packet.nonce);

        let plaintext = cipher
            .decrypt(nonce, packet.payload.as_ref())
            .map_err(|_| "Destination onion layer decryption failed".to_string())?;

        Ok(plaintext)
    }
}
