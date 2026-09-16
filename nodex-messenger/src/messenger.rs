use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::crypto::{decrypt_envelope, encrypt_envelope, EncryptedEnvelope};
use crate::db::{MessengerDb, SavedChatMessage, SavedContact};
use crate::identity::UserIdentity;
use nodex_kademlia::lookup::{RPC_RETRIES, RPC_TIMEOUT};
use nodex_kademlia::node::NodeId;
use nodex_kademlia::rpc::RpcPayload;
use nodex_kademlia::KademliaNode;

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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatMessagePayload {
    pub text: String,
    #[serde(default)]
    pub image_base64: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MessengerEvent {
    ContactsUpdated(Vec<SavedContact>),
    MessageReceived(SavedChatMessage),
}

pub struct KadMessenger {
    pub dht_node: KademliaNode,
    pub identity: Arc<RwLock<UserIdentity>>,
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
        if !display_name.is_empty() && display_name != "UserNode" && display_name != "NodeX User" {
            db.display_name = display_name;
        }

        let (identity, mnemonic_opt) = if !db.mnemonic.is_empty() {
            match UserIdentity::from_mnemonic(&db.mnemonic) {
                Ok(id) => (id, None),
                Err(_) => {
                    let (mn, id) = UserIdentity::generate_mnemonic().unwrap_or_else(|_| {
                        let id = UserIdentity::generate();
                        (String::new(), id)
                    });
                    (id, Some(mn))
                }
            }
        } else if db.user_seed != [0u8; 32] {
            (UserIdentity::from_seed(&db.user_seed), None)
        } else {
            let (mn, id) = UserIdentity::generate_mnemonic().unwrap_or_else(|_| {
                let id = UserIdentity::generate();
                (String::new(), id)
            });
            (id, Some(mn))
        };

        db.user_seed = *identity.signing_key.as_bytes();
        if let Some(mn) = mnemonic_opt {
            db.mnemonic = mn;
        }

        db.save_to_file(&db_path).ok();

        let messenger = Arc::new(Self {
            dht_node,
            identity: Arc::new(RwLock::new(identity)),
            db: Arc::new(RwLock::new(db)),
            db_path,
        });

        // Publish initial presence
        messenger.publish_presence().await.ok();

        // Background Presence Heartbeat (re-publishes every 8s so all DHT nodes know about this peer)
        let messenger_heartbeat = Arc::clone(&messenger);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(8));
            loop {
                interval.tick().await;
                let _ = messenger_heartbeat.publish_presence().await;
            }
        });

        Ok(messenger)
    }

    pub async fn restore_mnemonic(&self, mnemonic: &str, display_name: Option<String>) -> Result<(), String> {
        let new_identity = UserIdentity::from_mnemonic(mnemonic.trim())
            .map_err(|e| e.to_string())?;
        {
            let mut db = self.db.write().await;
            db.mnemonic = mnemonic.trim().to_string();
            db.user_seed = *new_identity.signing_key.as_bytes();
            if let Some(name) = display_name {
                db.display_name = name;
            }
            db.save_to_file(&self.db_path).ok();
        }
        {
            let mut id = self.identity.write().await;
            *id = new_identity;
        }
        self.publish_presence().await?;
        Ok(())
    }

    pub async fn get_mnemonic(&self) -> String {
        let db = self.db.read().await;
        db.mnemonic.clone()
    }

    pub async fn user_id_hex(&self) -> String {
        self.identity.read().await.user_id_hex()
    }

    pub async fn update_display_name(&self, name: String) -> Result<(), String> {
        {
            let mut db = self.db.write().await;
            db.display_name = name;
            db.save_to_file(&self.db_path).ok();
        }
        self.publish_presence().await?;
        Ok(())
    }

    pub async fn update_profile(&self, display_name: String, bio: String) -> Result<(), String> {
        {
            let mut db = self.db.write().await;
            db.display_name = display_name;
            db.bio = bio;
            db.save_to_file(&self.db_path).ok();
        }
        self.publish_presence().await?;
        Ok(())
    }

    pub fn start_inbox_polling_task(self: &Arc<Self>, event_tx: Option<std::sync::mpsc::Sender<MessengerEvent>>) {
        let messenger = Arc::clone(self);
        tokio::spawn(async move {
            let mut current_poll_secs = 1u64;
            loop {
                tokio::time::sleep(Duration::from_secs(current_poll_secs)).await;

                let (my_user_id, my_node_id) = {
                    let id = messenger.identity.read().await;
                    (id.user_id_hex(), id.user_id)
                };
                let mailbox_key = current_mailbox_key(&my_user_id);

                if let Ok(Some((bytes, _from))) = messenger.dht_node.get_silent(&mailbox_key).await {
                    let envelopes: Vec<EncryptedEnvelope> = if let Ok(list) = serde_json::from_slice::<Vec<EncryptedEnvelope>>(&bytes) {
                        list
                    } else if let Ok(single) = serde_json::from_slice::<EncryptedEnvelope>(&bytes) {
                        vec![single]
                    } else {
                        Vec::new()
                    };

                    let mut received_any = false;
                    for envelope in envelopes {
                        if envelope.recipient_id == my_node_id {
                            let decrypt_res = {
                                let id = messenger.identity.read().await;
                                decrypt_envelope(&id, &envelope)
                            };
                            if let Ok(plaintext) = decrypt_res {
                                let (text, image_base64) = if let Ok(payload) = serde_json::from_slice::<ChatMessagePayload>(&plaintext) {
                                    (payload.text, payload.image_base64)
                                } else {
                                    (String::from_utf8_lossy(&plaintext).to_string(), None)
                                };

                                let sender_hex = envelope.sender_id.to_hex();
                                let msg_id = format!("{}_{}", envelope.timestamp, sender_hex);

                                let chat_msg = SavedChatMessage {
                                    id: msg_id,
                                    sender_id_hex: sender_hex.clone(),
                                    recipient_id_hex: my_user_id.clone(),
                                    text,
                                    image_base64,
                                    timestamp: envelope.timestamp,
                                    incoming: true,
                                    delivered: true,
                                };

                                // Auto-add sender to contact list if missing or update keys
                                let has_contact = {
                                    let db = messenger.db.read().await;
                                    db.contacts.contains_key(&sender_hex)
                                };

                                if !has_contact {
                                    let (discovered_name, discovered_bio) = match messenger.discover_peer(&sender_hex).await {
                                        Ok(card) => (card.display_name, card.bio),
                                        Err(_) => (format!("Peer_{}", &sender_hex[..6.min(sender_hex.len())]), String::new()),
                                    };
                                    let mut db = messenger.db.write().await;
                                    db.add_contact(SavedContact {
                                        user_id_hex: sender_hex.clone(),
                                        name: discovered_name,
                                        bio: discovered_bio,
                                        ed25519_pub_hex: hex_string(&envelope.sender_ed25519_pub),
                                        x25519_pub_hex: hex_string(&envelope.sender_x25519_pub),
                                        last_seen_addr: "unknown".into(),
                                    });
                                    db.save_to_file(&messenger.db_path).ok();
                                }

                                let is_new = {
                                    let mut db = messenger.db.write().await;
                                    let before = db.messages.len();
                                    db.add_message(chat_msg.clone());
                                    db.save_to_file(&messenger.db_path).ok();
                                    db.messages.len() > before
                                };

                                if is_new {
                                    received_any = true;
                                    if let Some(ref tx) = event_tx {
                                        let db = messenger.db.read().await;
                                        let contacts: Vec<_> = db.contacts.values().cloned().collect();
                                        let _ = tx.send(MessengerEvent::ContactsUpdated(contacts));
                                        let _ = tx.send(MessengerEvent::MessageReceived(chat_msg));
                                    }
                                    println!("[MESSENGER] Received E2EE message from {} via Direct/Mailbox Transport!", sender_hex);
                                }
                            }
                        }
                    }

                    if received_any {
                        current_poll_secs = 1;
                        continue;
                    }
                }

                current_poll_secs = (current_poll_secs + 1).min(4);
            }
        });
    }

    pub async fn publish_presence(&self) -> Result<(), String> {
        let timestamp = current_timestamp();
        let identity = self.identity.read().await;

        let (display_name, bio) = {
            let db = self.db.read().await;
            (db.display_name.clone(), db.bio.clone())
        };

        let mut sign_payload = Vec::new();
        sign_payload.extend_from_slice(identity.user_id.as_bytes());
        sign_payload.extend_from_slice(self.dht_node.network.local_addr.to_string().as_bytes());
        sign_payload.extend_from_slice(&timestamp.to_be_bytes());

        let signature = identity.sign(&sign_payload);

        let card = UserPresenceCard {
            user_id_hex: identity.user_id_hex(),
            display_name,
            bio,
            ed25519_pub: identity.verifying_key.to_bytes().to_vec(),
            x25519_pub: identity.x25519_public.as_bytes().to_vec(),
            socket_addr: self.dht_node.network.local_addr,
            timestamp,
            signature,
        };

        let json_bytes = serde_json::to_vec(&card).map_err(|e| format!("Presence serialize error: {}", e))?;
        let presence_key = format!("presence_{}", identity.user_id_hex());

        self.dht_node.put(&presence_key, json_bytes).await?;
        println!("[MESSENGER] Published presence card to DHT for user {}", identity.user_id_hex());
        Ok(())
    }

    pub async fn discover_peer(&self, user_id_hex: &str) -> Result<UserPresenceCard, String> {
        let presence_key = format!("presence_{}", user_id_hex.trim());
        let res = self.dht_node.get(&presence_key).await?;

        if let Some((bytes, _from)) = res {
            let card: UserPresenceCard = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Failed to parse presence card: {}", e))?;
            
            let mut db = self.db.write().await;
            db.add_contact(SavedContact {
                user_id_hex: card.user_id_hex.clone(),
                name: card.display_name.clone(),
                bio: card.bio.clone(),
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

    pub async fn send_message(&self, recipient_id_hex: &str, text: &str, image_base64: Option<String>) -> Result<SavedChatMessage, String> {
        let recipient_id = NodeId::from_hex(recipient_id_hex)?;

        // 1. Resolve recipient's X25519 public key and address from contacts cache or DHT discovery
        let (recipient_x25519_bytes, target_socket_addr) = {
            let (cached_key, cached_addr) = {
                let db = self.db.read().await;
                if let Some(c) = db.contacts.get(recipient_id_hex) {
                    (hex_decode(&c.x25519_pub_hex), c.last_seen_addr.parse::<SocketAddr>().ok())
                } else {
                    (None, None)
                }
            };

            if let (Some(k), addr) = (cached_key, cached_addr) {
                if k.len() == 32 {
                    (k, addr)
                } else {
                    match self.discover_peer(recipient_id_hex).await {
                        Ok(card) => (card.x25519_pub, Some(card.socket_addr)),
                        Err(_) => (vec![0u8; 32], None),
                    }
                }
            } else {
                match self.discover_peer(recipient_id_hex).await {
                    Ok(card) => (card.x25519_pub, Some(card.socket_addr)),
                    Err(_) => (vec![0u8; 32], None),
                }
            }
        };

        let mut x25519_arr = [0u8; 32];
        if recipient_x25519_bytes.len() == 32 {
            x25519_arr.copy_from_slice(&recipient_x25519_bytes);
        } else {
            // Fallback derived key from recipient node ID
            let mut h = [0u8; 32];
            h[..20].copy_from_slice(recipient_id.as_bytes());
            x25519_arr = h;
        }
        let recipient_x25519 = x25519_dalek::PublicKey::from(x25519_arr);

        let payload_struct = ChatMessagePayload {
            text: text.to_string(),
            image_base64: image_base64.clone(),
        };
        let payload_bytes = serde_json::to_vec(&payload_struct).map_err(|e| e.to_string())?;

        let envelope = {
            let id = self.identity.read().await;
            encrypt_envelope(&id, recipient_id, &recipient_x25519, &payload_bytes)?
        };

        let mailbox_key = current_mailbox_key(recipient_id_hex);
        let mailbox_key_id = NodeId::from_key(mailbox_key.as_bytes());

        // Prepare mailbox payload list
        let single_payload = serde_json::to_vec(&vec![envelope.clone()]).unwrap_or_default();

        let mut delivered = false;

        // 2. Direct UDP Instant Delivery: if socket address is known, send Store RPC directly to target
        if let Some(addr) = target_socket_addr {
            println!("[MESSENGER] Attempting direct UDP transport to {} at {}...", recipient_id_hex, addr);
            match self.dht_node.network.call(
                addr,
                RpcPayload::Store { key: mailbox_key_id, value: single_payload.clone() },
                RPC_TIMEOUT,
                RPC_RETRIES,
            ).await {
                Ok(reply) => {
                    if let RpcPayload::StoreAck { ok: true } = reply.payload {
                        println!("[MESSENGER] Direct UDP message successfully delivered to {} ({})!", recipient_id_hex, addr);
                        delivered = true;
                    }
                }
                Err(e) => {
                    println!("[MESSENGER] Direct delivery to {} failed: {}. Falling back to DHT Mailbox.", addr, e);
                }
            }
        }

        // 3. Deposit into DHT Mailbox relay so message is persisted across the network
        let _ = deposit_envelope_to_mailbox(&self.dht_node, recipient_id_hex, envelope).await;

        let my_user_id_hex = self.identity.read().await.user_id_hex();
        let msg_id = format!("{}_{}", current_timestamp(), rand::random::<u32>());
        let chat_msg = SavedChatMessage {
            id: msg_id,
            sender_id_hex: my_user_id_hex,
            recipient_id_hex: recipient_id_hex.to_string(),
            text: text.to_string(),
            image_base64,
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

async fn deposit_envelope_to_mailbox(dht_node: &KademliaNode, recipient_id_hex: &str, envelope: EncryptedEnvelope) -> Result<usize, String> {
    let mailbox_key = current_mailbox_key(recipient_id_hex);
    let mut envelopes: Vec<EncryptedEnvelope> = match dht_node.get_silent(&mailbox_key).await {
        Ok(Some((bytes, _))) => {
            if let Ok(list) = serde_json::from_slice::<Vec<EncryptedEnvelope>>(&bytes) {
                list
            } else if let Ok(single) = serde_json::from_slice::<EncryptedEnvelope>(&bytes) {
                vec![single]
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };

    if !envelopes.iter().any(|e| e.signature == envelope.signature && e.timestamp == envelope.timestamp) {
        envelopes.push(envelope);
    }

    if envelopes.len() > 100 {
        envelopes = envelopes.split_off(envelopes.len() - 100);
    }

    let payload = serde_json::to_vec(&envelopes).map_err(|e| e.to_string())?;
    dht_node.put(&mailbox_key, payload).await
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.is_empty() || s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn derive_c_mailbox_key(user_id: &NodeId, timestamp: u64) -> String {
    if let Some(key) = core_ffi::ffi_derive_mailbox_key(user_id.as_bytes(), timestamp) {
        key
    } else {
        let day_epoch = timestamp / 86400;
        format!("mailbox_{}_{}", user_id.to_hex(), day_epoch)
    }
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
        let mut b = [0u8; 20];
        b.copy_from_slice(bytes);
        return core_ffi::ffi_node_id_to_hex(&b);
    }
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn calculate_c_checksum(data: &[u8]) -> u32 {
    core_ffi::ffi_calculate_checksum(data)
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
