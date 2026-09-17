use std::net::SocketAddr;
use serde::{Deserialize, Serialize};
use nodex_kademlia::nat_type::NatCategory;
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
    #[serde(default)]
    pub endpoints: Vec<SocketAddr>,
    #[serde(default)]
    pub nat_category: NatCategory,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl UserPresenceCard {
    fn signing_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&(
            &self.user_id_hex,
            &self.display_name,
            &self.bio,
            &self.ed25519_pub,
            &self.x25519_pub,
            &self.socket_addr,
            &self.endpoints,
            &self.nat_category,
            self.timestamp,
        )).expect("presence signing payload serialization cannot fail")
    }

    pub fn create(
        identity: &UserIdentity,
        display_name: String,
        bio: String,
        socket_addr: SocketAddr,
        timestamp: u64,
    ) -> Self {
        Self::create_with_endpoints(
            identity,
            display_name,
            bio,
            socket_addr,
            vec![socket_addr],
            NatCategory::Unknown,
            timestamp,
        )
    }

    pub fn create_with_endpoints(
        identity: &UserIdentity,
        display_name: String,
        bio: String,
        socket_addr: SocketAddr,
        endpoints: Vec<SocketAddr>,
        nat_category: NatCategory,
        timestamp: u64,
    ) -> Self {
        let mut card = Self {
            user_id_hex: identity.user_id_hex(),
            display_name,
            bio,
            ed25519_pub: identity.verifying_key.to_bytes().to_vec(),
            x25519_pub: identity.x25519_public.as_bytes().to_vec(),
            socket_addr,
            endpoints,
            nat_category,
            timestamp,
            signature: Vec::new(),
        };
        card.signature = identity.sign(&card.signing_bytes());
        card
    }

    pub fn verify_signature(&self) -> Result<(), MessengerError> {
        if self.ed25519_pub.len() != 32 {
            return Err(MessengerError::InvalidKeyLength("Ed25519 pub key must be 32 bytes".into()));
        }
        let mut vk_arr = [0u8; 32];
        vk_arr.copy_from_slice(&self.ed25519_pub);
        let vk = ed25519_dalek::VerifyingKey::from_bytes(&vk_arr)
            .map_err(|e| MessengerError::CryptoError(e.to_string()))?;

        let expected_user_id = core_ffi::ffi_hash_node_id(&self.ed25519_pub)
            .iter()
            .map(|byte| format!("{:02x}", byte))
            .collect::<String>();
        if self.user_id_hex != expected_user_id {
            return Err(MessengerError::CryptoError("Presence user ID does not match Ed25519 public key".into()));
        }

        if self.signature.len() != 64 {
            return Err(MessengerError::CryptoError("Signature must be 64 bytes".into()));
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&self.signature);
        let sig = ed25519_dalek::Signature::from_bytes(&sig_arr);

        vk.verify_strict(&self.signing_bytes(), &sig)
            .map_err(|e| MessengerError::CryptoError(format!("Invalid presence signature: {}", e)))
    }
}
