use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::crypto::UserIdentity;

const INVITE_PREFIX: &str = "nodex://invite/";
const DEFAULT_INVITE_TTL_SECS: u64 = 7 * 24 * 3600; // 7 days

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct InvitePayload {
    pub user_id: String,
    pub ed25519_pub_hex: String,
    pub x25519_pub_hex: String,
    pub display_name: String,
    pub fingerprint: String,
    pub endpoints: Vec<SocketAddr>,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature_hex: String,
}

pub struct InviteManager;

impl InviteManager {
    /// Generates a signed `nodex://invite/<base64>` invite link.
    pub fn create_invite(
        identity: &UserIdentity,
        display_name: &str,
        endpoints: Vec<SocketAddr>,
        ttl_seconds: Option<u64>,
    ) -> Result<String, String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs();

        let expires_at = now + ttl_seconds.unwrap_or(DEFAULT_INVITE_TTL_SECS);

        let ed_hex: String = identity.verifying_key.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let x_hex: String = identity.x25519_public.as_bytes().iter().map(|b| format!("{:02x}", b)).collect();
        let user_id = identity.user_id_hex();
        let fingerprint = identity.fingerprint();

        // Message to sign: user_id || ed25519_pub || x25519_pub || created_at || expires_at
        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(user_id.as_bytes());
        sign_data.extend_from_slice(ed_hex.as_bytes());
        sign_data.extend_from_slice(x_hex.as_bytes());
        sign_data.extend_from_slice(&created_at_to_bytes(created_at_val(now)));
        sign_data.extend_from_slice(&expires_at.to_be_bytes());

        let sig = identity.signing_key.sign(&sign_data);
        let sig_hex: String = sig.to_bytes().iter().map(|b| format!("{:02x}", b)).collect();

        let payload = InvitePayload {
            user_id,
            ed25519_pub_hex: ed_hex,
            x25519_pub_hex: x_hex,
            display_name: display_name.to_string(),
            fingerprint,
            endpoints,
            created_at: now,
            expires_at,
            signature_hex: sig_hex,
        };

        let json = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
        let b64 = URL_SAFE_NO_PAD.encode(json);
        Ok(format!("{}{}", INVITE_PREFIX, b64))
    }

    /// Parses and verifies a `nodex://invite/<base64>` invite link.
    pub fn parse_and_verify_invite(invite_str: &str, current_timestamp: u64) -> Result<InvitePayload, String> {
        let clean_str = invite_str.trim();
        let b64_part = if let Some(stripped) = clean_str.strip_prefix(INVITE_PREFIX) {
            stripped
        } else {
            clean_str
        };

        let raw_bytes = URL_SAFE_NO_PAD
            .decode(b64_part)
            .map_err(|e| format!("Invalid Base64 in invite: {}", e))?;

        let payload: InvitePayload = serde_json::from_slice(&raw_bytes)
            .map_err(|e| format!("Invalid JSON payload in invite: {}", e))?;

        // 1. Check expiration
        if payload.expires_at <= current_timestamp {
            return Err(format!("Invite has expired (expired at {}, now is {})", payload.expires_at, current_timestamp));
        }

        // 2. Parse Ed25519 Public Key
        let ed_bytes = hex::decode(&payload.ed25519_pub_hex)
            .map_err(|e| format!("Invalid Ed25519 public key hex: {}", e))?;
        if ed_bytes.len() != 32 {
            return Err("Ed25519 public key must be exactly 32 bytes".into());
        }
        let mut key_arr = [0u8; 32];
        key_arr.copy_from_slice(&ed_bytes);

        // 3. Verify user_id consistency
        let expected_user_id = nodex_kademlia::node::NodeId::from_key(&key_arr).to_hex();
        if payload.user_id != expected_user_id {
            return Err("User ID does not match Ed25519 public key".into());
        }

        // 4. Verify Signature
        let sig_bytes = hex::decode(&payload.signature_hex)
            .map_err(|e| format!("Invalid signature hex: {}", e))?;
        if sig_bytes.len() != 64 {
            return Err("Signature must be exactly 64 bytes".into());
        }
        let mut sig_arr = [0u8; 64];
        sig_arr.copy_from_slice(&sig_bytes);
        let signature = Signature::from_bytes(&sig_arr);

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(payload.user_id.as_bytes());
        sign_data.extend_from_slice(payload.ed25519_pub_hex.as_bytes());
        sign_data.extend_from_slice(payload.x25519_pub_hex.as_bytes());
        sign_data.extend_from_slice(&created_at_to_bytes(payload.created_at));
        sign_data.extend_from_slice(&payload.expires_at.to_be_bytes());

        let verifying_key = VerifyingKey::from_bytes(&key_arr)
            .map_err(|e| format!("Invalid verifying key: {}", e))?;

        verifying_key
            .verify(&sign_data, &signature)
            .map_err(|_| "Cryptographic signature verification failed on invite".to_string())?;

        Ok(payload)
    }
}

fn created_at_val(v: u64) -> u64 {
    v
}

fn created_at_to_bytes(v: u64) -> [u8; 8] {
    v.to_be_bytes()
}

mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if !s.len().is_multiple_of(2) {
            return Err("Invalid hex string length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}
