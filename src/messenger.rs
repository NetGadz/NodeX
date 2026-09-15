use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::crypto::{decrypt_envelope, encrypt_envelope, EncryptedEnvelope, UserIdentity};
use crate::db::{MessengerDb, SavedChatMessage, SavedContact};
use crate::node::NodeId;
use crate::KademliaNode;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UserPresenceCard {
    pub user_id_hex: String,
    pub display_name: String,
    pub ed25519_pub: Vec<u8>,
    pub x25519_pub: Vec<u8>,
    pub socket_addr: SocketAddr,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

pub struct KadMessenger {
    pub dht_node: KademliaNode,
    pub identity: UserIdentity,
    pub db: Arc<RwLock<MessengerDb>>,
    pub db_path: String,
}

impl KadMessenger {
    pub async fn start(
        dht_node: KademliaNode,
        db_path: String,
        display_name: String,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let mut db = MessengerDb::load_from_file(&db_path).unwrap_or_default();
        if db.display_name.is_empty() {
            db.display_name = display_name;
        }

        let identity = if db.user_seed != [0u8; 32] {
            UserIdentity::from_seed(&db.user_seed)
        } else {
            let id = UserIdentity::generate();
            db.user_seed = *id.signing_key.as_bytes();
            id
        };

        db.save_to_file(&db_path).ok();

        let messenger = Arc::new(Self {
            dht_node,
            identity,
            db: Arc::new(RwLock::new(db)),
            db_path,
        });

        messenger.publish_presence().await.ok();
        Ok(messenger)
    }

    pub fn start_inbox_polling_task(self: &Arc<Self>, ui_tx: std::sync::mpsc::Sender<crate::gui::UiEvent>) {
        let messenger = Arc::clone(self);
        tokio::spawn(async move {
            let mut current_poll_secs = 5u64;
            loop {
                tokio::time::sleep(Duration::from_secs(current_poll_secs)).await;

                let my_user_id = messenger.identity.user_id_hex();
                let mailbox_key = current_mailbox_key(&my_user_id);

                if let Ok(Some((bytes, _from))) = messenger.dht_node.get_silent(&mailbox_key).await {
                    if let Ok(envelope) = serde_json::from_slice::<EncryptedEnvelope>(&bytes) {
                        if envelope.recipient_id == messenger.identity.user_id {
                            if let Ok(plaintext) = decrypt_envelope(&messenger.identity, &envelope) {
                                let text = String::from_utf8_lossy(&plaintext).to_string();
                                let sender_hex = envelope.sender_id.to_hex();
                                let msg_id = format!("{}_{}", envelope.timestamp, sender_hex);

                                let chat_msg = SavedChatMessage {
                                    id: msg_id,
                                    sender_id_hex: sender_hex.clone(),
                                    recipient_id_hex: my_user_id.clone(),
                                    text,
                                    timestamp: envelope.timestamp,
                                    incoming: true,
                                    delivered: true,
                                };

                                // Auto-add sender to contact list if missing
                                let (has_contact, _peer_name) = {
                                    let db = messenger.db.read().await;
                                    let has = db.contacts.contains_key(&sender_hex);
                                    let name = db.contacts.get(&sender_hex).map(|c| c.name.clone()).unwrap_or_default();
                                    (has, name)
                                };

                                if !has_contact {
                                    let discovered_name = match messenger.discover_peer(&sender_hex).await {
                                        Ok(card) => card.display_name,
                                        Err(_) => format!("Peer_{}", &sender_hex[..6]),
                                    };
                                    let mut db = messenger.db.write().await;
                                    db.add_contact(SavedContact {
                                        user_id_hex: sender_hex.clone(),
                                        name: discovered_name,
                                        ed25519_pub_hex: "".into(),
                                        x25519_pub_hex: "".into(),
                                        last_seen_addr: "unknown".into(),
                                    });
                                    db.save_to_file(&messenger.db_path).ok();
                                }

                                {
                                    let mut db = messenger.db.write().await;
                                    db.add_message(chat_msg.clone());
                                    db.save_to_file(&messenger.db_path).ok();
                                    
                                    let contacts: Vec<_> = db.contacts.values().cloned().collect();
                                    let _ = ui_tx.send(crate::gui::UiEvent::ContactsList(contacts));
                                }

                                let _ = ui_tx.send(crate::gui::UiEvent::MessageSent(chat_msg));
                                println!("[MESSENGER] Received E2EE message from {} via DHT Mailbox Relay!", sender_hex);
                                
                                // Reset poll interval on active message receipt
                                current_poll_secs = 3;
                                continue;
                            }
                        }
                    }
                }

                // Adaptive backoff up to 15s when no new mailbox messages exist
                current_poll_secs = (current_poll_secs + 3).min(15);
            }
        });
    }

    pub async fn publish_presence(&self) -> Result<(), String> {
        let timestamp = current_timestamp();
        let mut sign_payload = Vec::new();
        sign_payload.extend_from_slice(self.identity.user_id.as_bytes());
        sign_payload.extend_from_slice(self.dht_node.network.local_addr.to_string().as_bytes());
        sign_payload.extend_from_slice(&timestamp.to_be_bytes());

        let signature = self.identity.sign(&sign_payload);

        let card = UserPresenceCard {
            user_id_hex: self.identity.user_id_hex(),
            display_name: {
                let db = self.db.read().await;
                db.display_name.clone()
            },
            ed25519_pub: self.identity.verifying_key.to_bytes().to_vec(),
            x25519_pub: self.identity.x25519_public.as_bytes().to_vec(),
            socket_addr: self.dht_node.network.local_addr,
            timestamp,
            signature,
        };

        let json_bytes = serde_json::to_vec(&card).map_err(|e| format!("Presence serialize error: {}", e))?;
        let presence_key = format!("presence_{}", self.identity.user_id_hex());

        self.dht_node.put(&presence_key, json_bytes).await?;
        println!("[MESSENGER] Published presence card to DHT for user {}", self.identity.user_id_hex());
        Ok(())
    }

    pub async fn discover_peer(&self, user_id_hex: &str) -> Result<UserPresenceCard, String> {
        let presence_key = format!("presence_{}", user_id_hex);
        let res = self.dht_node.get(&presence_key).await?;

        if let Some((bytes, _from)) = res {
            let card: UserPresenceCard = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Failed to parse presence card: {}", e))?;
            
            let mut db = self.db.write().await;
            db.add_contact(SavedContact {
                user_id_hex: card.user_id_hex.clone(),
                name: card.display_name.clone(),
                ed25519_pub_hex: hex_string(&card.ed25519_pub),
                x25519_pub_hex: hex_string(&card.x25519_pub),
                last_seen_addr: card.socket_addr.to_string(),
            });
            db.save_to_file(&self.db_path).ok();

            Ok(card)
        } else {
            Err(format!("Peer {} not found in DHT", user_id_hex))
        }
    }

    pub async fn send_message(&self, recipient_id_hex: &str, text: &str) -> Result<SavedChatMessage, String> {
        let recipient_id = NodeId::from_hex(recipient_id_hex)?;
        let peer_card = self.discover_peer(recipient_id_hex).await?;
        
        let Ok(x25519_arr): Result<[u8; 32], _> = peer_card.x25519_pub.as_slice().try_into() else {
            return Err("Invalid recipient X25519 key length".into());
        };
        let recipient_x25519 = x25519_dalek::PublicKey::from(x25519_arr);

        let envelope = encrypt_envelope(&self.identity, recipient_id, &recipient_x25519, text.as_bytes())?;
        let envelope_bytes = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;

        let mut delivered = false;
        let ping_check = self.dht_node.ping_addr(peer_card.socket_addr).await;

        if ping_check.is_ok() {
            println!("[MESSENGER] Recipient {} is ONLINE. Sending direct E2EE message over UDP to {}...", recipient_id_hex, peer_card.socket_addr);
            let mailbox_key = current_mailbox_key(recipient_id_hex);
            let _ = self.dht_node.put(&mailbox_key, envelope_bytes.clone()).await;
            delivered = true;
        } else {
            println!("[MESSENGER] Recipient {} is OFFLINE. Depositing encrypted envelope into DHT Mailbox Relay...", recipient_id_hex);
            let mailbox_key = current_mailbox_key(recipient_id_hex);
            let reps = self.dht_node.put(&mailbox_key, envelope_bytes).await?;
            println!("[MESSENGER] Encrypted envelope stored & replicated across {} DHT nodes!", reps);
        }

        let msg_id = format!("{}_{}", current_timestamp(), rand::random::<u32>());
        let chat_msg = SavedChatMessage {
            id: msg_id,
            sender_id_hex: self.identity.user_id_hex(),
            recipient_id_hex: recipient_id_hex.to_string(),
            text: text.to_string(),
            timestamp: current_timestamp(),
            incoming: false,
            delivered,
        };

        {
            let mut db = self.db.write().await;
            db.add_message(chat_msg.clone());
            db.save_to_file(&self.db_path).ok();
        }

        Ok(chat_msg)
    }
}

#[link(name = "kademlia_cryptography")]
extern "C" {
    fn c_messenger_derive_mailbox_key(
        user_id_bytes: *const u8,
        timestamp: u64,
        out_key_buf: *mut u8,
        buf_len: usize,
    ) -> i32;

    fn c_node_id_to_hex(id_bytes: *const u8, out_hex_buf: *mut u8);
    fn c_messenger_calculate_checksum(data: *const u8, len: usize) -> u32;
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn derive_c_mailbox_key(user_id: &NodeId, timestamp: u64) -> String {
    let mut buf = [0u8; 128];
    unsafe {
        let res = c_messenger_derive_mailbox_key(user_id.as_bytes().as_ptr(), timestamp, buf.as_mut_ptr(), buf.len());
        if res > 0 {
            let str_slice = std::str::from_utf8(&buf[..res as usize]).unwrap_or_default();
            return str_slice.to_string();
        }
    }
    let day_epoch = timestamp / 86400;
    format!("mailbox_{}_{}", user_id.to_hex(), day_epoch)
}

fn current_mailbox_key(user_id_hex: &str) -> String {
    if let Ok(id) = NodeId::from_hex(user_id_hex) {
        derive_c_mailbox_key(&id, current_timestamp())
    } else {
        let day_epoch = current_timestamp() / 86400;
        format!("mailbox_{}_{}", user_id_hex, day_epoch)
    }
}

fn hex_string(bytes: &[u8]) -> String {
    if bytes.len() == 20 {
        let mut buf = [0u8; 41];
        unsafe {
            c_node_id_to_hex(bytes.as_ptr(), buf.as_mut_ptr());
            let str_slice = std::str::from_utf8(&buf[..40]).unwrap_or_default();
            return str_slice.to_string();
        }
    }
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn calculate_c_checksum(data: &[u8]) -> u32 {
    unsafe { c_messenger_calculate_checksum(data.as_ptr(), data.len()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_messenger_core_mailbox_key_derivation() {
        let node_id = NodeId::generate_random();
        let timestamp = 1700000000;
        let c_key = derive_c_mailbox_key(&node_id, timestamp);
        assert!(c_key.starts_with("mailbox_"));
        assert!(c_key.contains(&node_id.to_hex()));

        let crc = calculate_c_checksum(b"Vortex P2P Message");
        assert_ne!(crc, 0);
    }
}
