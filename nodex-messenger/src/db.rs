use std::collections::HashMap;
use std::fs;
use std::path::Path;

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};

const DB_MAGIC: &[u8; 8] = b"NODEXENC";
const DB_MASTER_SALT: &[u8; 32] = b"NODEX_P2P_E2EE_LOCAL_DB_KEY_V1__";

fn default_db_key() -> Key {
    *Key::from_slice(DB_MASTER_SALT)
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedContact {
    pub user_id_hex: String,
    pub name: String,
    #[serde(default)]
    pub bio: String,
    pub ed25519_pub_hex: String,
    pub x25519_pub_hex: String,
    pub last_seen_addr: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedChatMessage {
    pub id: String,
    pub sender_id_hex: String,
    pub recipient_id_hex: String,
    pub text: String,
    #[serde(default)]
    pub image_base64: Option<String>,
    pub timestamp: u64,
    pub incoming: bool,
    pub delivered: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MessengerDb {
    pub user_seed: [u8; 32],
    #[serde(default)]
    pub mnemonic: String,
    pub display_name: String,
    #[serde(default)]
    pub bio: String,
    pub contacts: HashMap<String, SavedContact>,
    pub messages: Vec<SavedChatMessage>,
}

impl MessengerDb {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }
        let data = fs::read(path).map_err(|e| format!("Read DB failed: {}", e))?;
        if data.is_empty() {
            return Ok(Self::default());
        }

        // Check if data is in encrypted binary format (starts with NODEXENC)
        if data.starts_with(DB_MAGIC) {
            if data.len() < 20 {
                return Err("Corrupted encrypted DB file".into());
            }
            let nonce = Nonce::from_slice(&data[8..20]);
            let ciphertext = &data[20..];

            let key = default_db_key();
            let cipher = ChaCha20Poly1305::new(&key);
            let plaintext = cipher
                .decrypt(nonce, ciphertext)
                .map_err(|_| "Failed to decrypt local messenger database (integrity check failed)".to_string())?;

            serde_json::from_slice(&plaintext).map_err(|e| format!("Parse decrypted DB failed: {}", e))
        } else {
            // Fallback for legacy plain JSON
            serde_json::from_slice(&data).map_err(|e| format!("Parse DB failed: {}", e))
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let plaintext = serde_json::to_vec(self).map_err(|e| format!("Serialize DB failed: {}", e))?;
        
        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let key = default_db_key();
        let cipher = ChaCha20Poly1305::new(&key);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| format!("Encrypt DB failed: {}", e))?;

        let mut out = Vec::with_capacity(DB_MAGIC.len() + 12 + ciphertext.len());
        out.extend_from_slice(DB_MAGIC);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);

        fs::write(path, out).map_err(|e| format!("Write encrypted DB failed: {}", e))
    }

    pub fn add_contact(&mut self, contact: SavedContact) {
        self.contacts.insert(contact.user_id_hex.clone(), contact);
    }

    pub fn add_message(&mut self, msg: SavedChatMessage) {
        if !self.messages.iter().any(|m| m.id == msg.id) {
            self.messages.push(msg);
        }
    }

    pub fn get_messages_for_contact(&self, contact_id_hex: &str) -> Vec<SavedChatMessage> {
        self.messages
            .iter()
            .filter(|m| m.sender_id_hex == contact_id_hex || m.recipient_id_hex == contact_id_hex)
            .cloned()
            .collect()
    }

    pub fn get_last_message_for_contact(&self, contact_id_hex: &str) -> Option<SavedChatMessage> {
        self.messages
            .iter()
            .filter(|m| m.sender_id_hex == contact_id_hex || m.recipient_id_hex == contact_id_hex)
            .last()
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_save_and_load_encrypted() {
        let mut db = MessengerDb::default();
        db.display_name = "Alice".into();
        db.add_contact(SavedContact {
            user_id_hex: "0102030405060708090a0b0c0d0e0f1011121314".into(),
            name: "Bob".into(),
            bio: "Bob's bio".into(),
            ed25519_pub_hex: "00".into(),
            x25519_pub_hex: "00".into(),
            last_seen_addr: "unknown".into(),
        });

        let tmp = std::env::temp_dir().join("test_encrypted_messenger_db.bin");
        db.save_to_file(&tmp).unwrap();

        // Verify the raw file on disk is NOT plaintext JSON
        let raw_bytes = fs::read(&tmp).unwrap();
        assert!(raw_bytes.starts_with(b"NODEXENC"));
        assert!(!String::from_utf8_lossy(&raw_bytes).contains("Alice"));

        let loaded = MessengerDb::load_from_file(&tmp).unwrap();
        assert_eq!(loaded.display_name, "Alice");
        assert_eq!(loaded.contacts.len(), 1);

        let _ = fs::remove_file(tmp);
    }
}
