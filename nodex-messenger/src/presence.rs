use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use crate::errors::MessengerError;
use crate::identity::UserIdentity;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPresenceCard {
    pub user_id_hex: String,
    pub display_name: String,
    #[serde(default)]
    pub bio: String,
    pub ed25519_pub: Vec<u8>,
    pub x25519_pub: Vec<u8>,
    pub socket_addr: SocketAddr,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl UserPresenceCard {
    pub fn create(
        identity: &UserIdentity,
        display_name: String,
        bio: String,
        socket_addr: SocketAddr,
        timestamp: u64,
    ) -> Self {
        let mut sign_payload = Vec::new();
        sign_payload.extend_from_slice(identity.user_id.as_bytes());
        sign_payload.extend_from_slice(socket_addr.to_string().as_bytes());
        sign_payload.extend_from_slice(&timestamp.to_be_bytes());

        let signature = identity.sign(&sign_payload);

        Self {
            user_id_hex: identity.user_id_hex(),
            display_name,
            bio,
            ed25519_pub: identity.verifying_key.to_bytes().to_vec(),
            x25519_pub: identity.x25519_public.as_bytes().to_vec(),
            socket_addr,
            timestamp,
            signature,
        }
    }

    pub fn verify_signature(&self) -> Result<(), MessengerError> {
        if self.ed25519_pub.len() != 32 {
            return Err(MessengerError::InvalidKeyLength("Ed25519 pub key must be 32 bytes".into()));
        }
        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&self.ed25519_pub);
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&vk_arr)
            .map_err(|e| MessengerError::CryptoError(e.to_string()))?;

        if self.signature.len() != 64 {
            return Err(MessengerError::CryptoError("Signature must be 64 bytes".into()));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&self.signature);
        let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

        let user_id_bytes = core_ffi::ffi_hash_node_id(&self.ed25519_pub);
        let mut sign_payload = Vec::new();
        sign_payload.extend_from_slice(&user_id_bytes);
        sign_payload.extend_from_slice(self.socket_addr.to_string().as_bytes());
        sign_payload.extend_from_slice(&self.timestamp.to_be_bytes());

        vk.verify_strict(&sign_payload, &sig)
            .map_err(|e| MessengerError::CryptoError(format!("Invalid presence signature: {}", e)))
    }
}
