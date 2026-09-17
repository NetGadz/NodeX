use serde::{Deserialize, Serialize};
use crate::db::SavedContact;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyRotationAlert {
    pub contact_user_id: String,
    pub contact_name: String,
    pub old_ed25519_pub_hex: String,
    pub new_ed25519_pub_hex: String,
    pub detected_at: u64,
}

pub struct ContactVerifier;

impl ContactVerifier {
    /// Formats a 32-byte public key into human-readable 4-character blocks: `XXXX-XXXX-XXXX-...`
    pub fn format_security_fingerprint(pubkey: &[u8; 32]) -> String {
        let hex_str: String = pubkey.iter().map(|b| format!("{:02X}", b)).collect();
        hex_str
            .as_bytes()
            .chunks(4)
            .map(|chunk| std::str::from_utf8(chunk).unwrap_or("????"))
            .collect::<Vec<&str>>()
            .join("-")
    }

    /// Checks if an incoming presence update or invite conflicts with an existing saved contact's key.
    pub fn detect_key_rotation(
        existing_contact: &SavedContact,
        incoming_ed25519_pub_hex: &str,
        current_time: u64,
    ) -> Option<KeyRotationAlert> {
        if !existing_contact.ed25519_pub_hex.is_empty()
            && !incoming_ed25519_pub_hex.is_empty()
            && existing_contact.ed25519_pub_hex != incoming_ed25519_pub_hex
        {
            Some(KeyRotationAlert {
                contact_user_id: existing_contact.user_id_hex.clone(),
                contact_name: existing_contact.name.clone(),
                old_ed25519_pub_hex: existing_contact.ed25519_pub_hex.clone(),
                new_ed25519_pub_hex: incoming_ed25519_pub_hex.to_string(),
                detected_at: current_time,
            })
        } else {
            None
        }
    }
}
