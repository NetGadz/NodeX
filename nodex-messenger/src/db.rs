use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use argon2::{Algorithm, Argon2, Params, Version};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DB_MAGIC_V2: &[u8; 9] = b"NODEXENC2";
const BACKUP_MAGIC: &[u8; 9] = b"NODEXBAK1";
const BACKUP_ARGON2_MARKER: &[u8; 8] = b"ARGON2ID";

/// Resolve or generate a secure per-installation 32-byte database master key.
/// The key is stored in a separate restricted key file next to the database.
pub fn get_or_create_instance_key<P: AsRef<Path>>(db_path: P) -> [u8; 32] {
    let key_path = db_key_path(db_path.as_ref());
    if let Ok(data) = fs::read(&key_path) {
        if data.len() == 32 {
            let mut key = [0u8; 32];
            key.copy_from_slice(&data);
            restrict_key_file(&key_path);
            return key;
        }
    }

    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    if let Some(parent) = key_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&key_path, key);
    restrict_key_file(&key_path);
    key
}

fn restrict_key_file(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }

    #[cfg(windows)]
    {
        if let Ok(username) = std::env::var("USERNAME") {
            let _ = std::process::Command::new("icacls")
                .arg(path)
                .args(["/inheritance:r", "/grant:r"])
                .arg(format!("{}:F", username))
                .status();
        }
    }
}

fn db_key_path(db_path: &Path) -> PathBuf {
    let mut key_path = db_path.to_path_buf();
    let ext = key_path.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
    if ext.is_empty() {
        key_path.set_extension("key");
    } else {
        key_path.set_extension(format!("{}.key", ext));
    }
    key_path
}

fn derive_cipher_key(master_key: &[u8; 32], salt: &[u8; 16]) -> Key {
    let mut hasher = Sha256::new();
    hasher.update(b"NODEX_DB_CIPHER_KEY_DERIVATION_V2");
    hasher.update(master_key);
    hasher.update(salt);
    let result = hasher.finalize();
    *Key::from_slice(&result)
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
    #[serde(default)]
    pub deleted_message_ids: HashSet<String>,
    #[serde(default)]
    pub blocked_user_ids: HashSet<String>,
}

impl MessengerDb {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let p = path.as_ref();
        if !p.exists() {
            return Ok(Self::default());
        }
        let data = fs::read(p).map_err(|e| format!("Read DB failed: {}", e))?;
        if data.is_empty() {
            return Ok(Self::default());
        }

        let master_key = get_or_create_instance_key(p);

        // Header: NODEXENC2 (9 bytes) + Salt (16 bytes) + Nonce (12 bytes) + Ciphertext
        if data.starts_with(DB_MAGIC_V2) {
            if data.len() < 9 + 16 + 12 {
                return Err("Corrupted encrypted DB file header".into());
            }
            let mut salt = [0u8; 16];
            salt.copy_from_slice(&data[9..25]);
            let nonce = Nonce::from_slice(&data[25..37]);
            let ciphertext = &data[37..];

            let cipher_key = derive_cipher_key(&master_key, &salt);
            let cipher = ChaCha20Poly1305::new(&cipher_key);
            let plaintext = cipher
                .decrypt(nonce, ciphertext)
                .map_err(|_| "Failed to decrypt local messenger database (integrity check failed)".to_string())?;

            serde_json::from_slice(&plaintext).map_err(|e| format!("Parse decrypted DB failed: {}", e))
        } else {
            Err("Unencrypted or legacy database format is rejected for security. Database must use NODEXENC2 format.".into())
        }
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let p = path.as_ref();
        let master_key = get_or_create_instance_key(p);
        let plaintext = serde_json::to_vec(self).map_err(|e| format!("Serialize DB failed: {}", e))?;

        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher_key = derive_cipher_key(&master_key, &salt);
        let cipher = ChaCha20Poly1305::new(&cipher_key);
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| format!("Encrypt DB failed: {}", e))?;

        let mut out = Vec::with_capacity(DB_MAGIC_V2.len() + 16 + 12 + ciphertext.len());
        out.extend_from_slice(DB_MAGIC_V2);
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);

        fs::write(p, out).map_err(|e| format!("Write encrypted DB failed: {}", e))
    }

    /// Export an encrypted versioned backup with a user password or master key
    pub fn export_encrypted_backup<P: AsRef<Path>>(&self, path: P, password: &str) -> Result<(), String> {
        let plaintext = serde_json::to_vec(self).map_err(|e| format!("Serialize backup failed: {}", e))?;
        let mut salt = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut salt);

        let master_key = derive_backup_key_argon2(password, &salt)?;

        let mut nonce_bytes = [0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&master_key));
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_ref())
            .map_err(|e| format!("Encrypt backup failed: {}", e))?;

        let mut out = Vec::with_capacity(BACKUP_MAGIC.len() + BACKUP_ARGON2_MARKER.len() + 16 + 12 + ciphertext.len());
        out.extend_from_slice(BACKUP_MAGIC);
        out.extend_from_slice(BACKUP_ARGON2_MARKER);
        out.extend_from_slice(&salt);
        out.extend_from_slice(&nonce_bytes);
        out.extend_from_slice(&ciphertext);

        fs::write(path, out).map_err(|e| format!("Write backup failed: {}", e))
    }

    /// Import an encrypted versioned backup with a user password
    pub fn import_encrypted_backup<P: AsRef<Path>>(path: P, password: &str) -> Result<Self, String> {
        let data = fs::read(path).map_err(|e| format!("Read backup file failed: {}", e))?;
        if !data.starts_with(BACKUP_MAGIC) || data.len() < BACKUP_MAGIC.len() + 16 + 12 {
            return Err("Invalid or corrupted encrypted backup file".into());
        }

        let is_argon2 = data[BACKUP_MAGIC.len()..].starts_with(BACKUP_ARGON2_MARKER);
        let header_offset = if is_argon2 { BACKUP_MAGIC.len() + BACKUP_ARGON2_MARKER.len() } else { BACKUP_MAGIC.len() };
        if data.len() < header_offset + 16 + 12 {
            return Err("Invalid or corrupted encrypted backup header".into());
        }
        let mut salt = [0u8; 16];
        salt.copy_from_slice(&data[header_offset..header_offset + 16]);
        let nonce = Nonce::from_slice(&data[header_offset + 16..header_offset + 28]);
        let ciphertext = &data[header_offset + 28..];

        let master_key = if is_argon2 {
            derive_backup_key_argon2(password, &salt)?
        } else {
            let mut hasher = Sha256::new();
            hasher.update(b"NODEX_ENCRYPTED_BACKUP_V1");
            hasher.update(password.as_bytes());
            hasher.update(salt);
            hasher.finalize().into()
        };

        let cipher = ChaCha20Poly1305::new(Key::from_slice(&master_key));
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "Backup decryption failed (incorrect password or corrupted file)".to_string())?;

        serde_json::from_slice(&plaintext).map_err(|e| format!("Parse backup failed: {}", e))
    }

    pub fn is_blocked(&self, user_id_hex: &str) -> bool {
        self.blocked_user_ids.contains(user_id_hex)
    }

    pub fn block_user(&mut self, user_id_hex: &str) {
        self.blocked_user_ids.insert(user_id_hex.to_string());
    }

    pub fn unblock_user(&mut self, user_id_hex: &str) {
        self.blocked_user_ids.remove(user_id_hex);
    }

    pub fn add_contact(&mut self, contact: SavedContact) {
        self.contacts.insert(contact.user_id_hex.clone(), contact);
    }

    pub fn add_message(&mut self, msg: SavedChatMessage) {
        if self.deleted_message_ids.contains(&msg.id) {
            return;
        }
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
            .rfind(|m| m.sender_id_hex == contact_id_hex || m.recipient_id_hex == contact_id_hex)
            .cloned()
    }

    pub fn delete_message(&mut self, message_id: &str) {
        self.deleted_message_ids.insert(message_id.to_string());
        self.messages.retain(|m| m.id != message_id);
    }

    pub fn delete_messages(&mut self, message_ids: &[String]) {
        for id in message_ids {
            self.deleted_message_ids.insert(id.clone());
        }
        self.messages.retain(|m| !message_ids.contains(&m.id));
    }

    pub fn clear_messages_for_contact(&mut self, contact_id_hex: &str) {
        for m in &self.messages {
            if m.sender_id_hex == contact_id_hex || m.recipient_id_hex == contact_id_hex {
                self.deleted_message_ids.insert(m.id.clone());
            }
        }
        self.messages.retain(|m| m.sender_id_hex != contact_id_hex && m.recipient_id_hex != contact_id_hex);
    }

    pub fn delete_contact(&mut self, contact_id_hex: &str) {
        self.contacts.remove(contact_id_hex);
        self.clear_messages_for_contact(contact_id_hex);
    }
}

fn derive_backup_key_argon2(password: &str, salt: &[u8; 16]) -> Result<[u8; 32], String> {
    let params = Params::new(64 * 1024, 3, 1, Some(32))
        .map_err(|e| format!("Invalid Argon2 parameters: {}", e))?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| format!("Backup key derivation failed: {}", e))?;
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_save_and_load_encrypted_v2() {
        let mut db = MessengerDb {
            display_name: "Alice".into(),
            ..Default::default()
        };
        db.add_contact(SavedContact {
            user_id_hex: "0102030405060708090a0b0c0d0e0f1011121314".into(),
            name: "Bob".into(),
            bio: "Bob's bio".into(),
            ed25519_pub_hex: "00".into(),
            x25519_pub_hex: "00".into(),
            last_seen_addr: "unknown".into(),
        });

        let tmp = std::env::temp_dir().join("test_encrypted_messenger_db_v2.bin");
        let key_file = db_key_path(&tmp);
        db.save_to_file(&tmp).unwrap();

        // Verify the raw file on disk is NOT plaintext JSON and has NODEXENC2 magic
        let raw_bytes = fs::read(&tmp).unwrap();
        assert!(raw_bytes.starts_with(b"NODEXENC2"));
        assert!(!String::from_utf8_lossy(&raw_bytes).contains("Alice"));

        let loaded = MessengerDb::load_from_file(&tmp).unwrap();
        assert_eq!(loaded.display_name, "Alice");
        assert_eq!(loaded.contacts.len(), 1);

        let _ = fs::remove_file(&tmp);
        let _ = fs::remove_file(&key_file);
    }

    #[test]
    fn db_encrypted_backup_roundtrip() {
        let mut db = MessengerDb {
            display_name: "Charlie".into(),
            bio: "Encrypted Backup Test".into(),
            ..Default::default()
        };
        db.add_message(SavedChatMessage {
            id: "msg1".into(),
            sender_id_hex: "01".into(),
            recipient_id_hex: "02".into(),
            text: "Top Secret Backup".into(),
            image_base64: None,
            timestamp: 123456,
            incoming: false,
            delivered: true,
        });

        let bak_path = std::env::temp_dir().join("test_backup.nbak");
        db.export_encrypted_backup(&bak_path, "StrongPassword123!").unwrap();

        let raw = fs::read(&bak_path).unwrap();
        assert!(raw.starts_with(b"NODEXBAK1"));
        assert!(!String::from_utf8_lossy(&raw).contains("Top Secret"));

        // Wrong password fails
        assert!(MessengerDb::import_encrypted_backup(&bak_path, "WrongPassword").is_err());

        // Correct password succeeds
        let restored = MessengerDb::import_encrypted_backup(&bak_path, "StrongPassword123!").unwrap();
        assert_eq!(restored.display_name, "Charlie");
        assert_eq!(restored.messages.len(), 1);
        assert_eq!(restored.messages[0].text, "Top Secret Backup");

        let _ = fs::remove_file(&bak_path);
    }
}

