use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SavedContact {
    pub user_id_hex: String,
    pub name: String,
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
    pub timestamp: u64,
    pub incoming: bool,
    pub delivered: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct MessengerDb {
    pub user_seed: [u8; 32],
    pub display_name: String,
    pub contacts: HashMap<String, SavedContact>,
    pub messages: Vec<SavedChatMessage>,
}

impl MessengerDb {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(path).map_err(|e| format!("Read DB failed: {}", e))?;
        serde_json::from_str(&content).map_err(|e| format!("Parse DB failed: {}", e))
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self).map_err(|e| format!("Serialize DB failed: {}", e))?;
        fs::write(path, content).map_err(|e| format!("Write DB failed: {}", e))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_save_and_load() {
        let mut db = MessengerDb::default();
        db.display_name = "Alice".into();
        db.add_contact(SavedContact {
            user_id_hex: "0102030405060708090a0b0c0d0e0f1011121314".into(),
            name: "Bob".into(),
            ed25519_pub_hex: "00".into(),
            x25519_pub_hex: "00".into(),
            last_seen_addr: "127.0.0.1:8001".into(),
        });

        let tmp = std::env::temp_dir().join("test_messenger_db.json");
        db.save_to_file(&tmp).unwrap();

        let loaded = MessengerDb::load_from_file(&tmp).unwrap();
        assert_eq!(loaded.display_name, "Alice");
        assert_eq!(loaded.contacts.len(), 1);

        let _ = fs::remove_file(tmp);
    }
}
