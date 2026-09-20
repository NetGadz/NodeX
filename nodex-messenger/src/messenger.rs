use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::attachments::{AttachmentManager, FileManifest};
use crate::calls::CallSignalPayload;
use crate::crypto::{
    decrypt_envelope, derive_secure_mailbox_key, encrypt_envelope, EncryptedEnvelope,
    ReplayProtectionCache,
};
use crate::db::{MessengerDb, SavedChatMessage, SavedContact};
use crate::errors::MessengerError;
use crate::groups::P2PGroup;
use crate::identity::UserIdentity;
use crate::message_queue::MessageQueue;
use crate::presence::UserPresenceCard;
use crate::reactions::MessageReaction;
use crate::receipts::{DeliveryReceipt, ReadReceipt};
use crate::voice::VoiceNote;
use nodex_kademlia::hole_punch::HolePuncher;
use nodex_kademlia::lookup::{RPC_RETRIES, RPC_TIMEOUT};
use nodex_kademlia::node::{Contact, NodeId};
use nodex_kademlia::rpc::RpcPayload;
use nodex_kademlia::KademliaNode;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ChatMessagePayload {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_note: Option<VoiceNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_manifest: Option<FileManifest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery_receipt: Option<DeliveryReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_receipt: Option<ReadReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction: Option<MessageReaction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_snippet: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_edited: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit_timestamp: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_signal: Option<CallSignalPayload>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_audio_chunk: Option<crate::calls::CallAudioChunk>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_message_ids: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tombstone: Option<SignedTombstone>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedTombstone {
    pub recipient_id: String,
    pub timestamp: u64,
    pub deleted_ids: Vec<String>,
    pub signature: Vec<u8>,
}

impl SignedTombstone {
    fn signing_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&(&self.recipient_id, self.timestamp, &self.deleted_ids))
            .expect("tombstone signing payload serialization cannot fail")
    }
}

#[derive(Debug, Clone)]
pub enum MessengerEvent {
    ContactsUpdated(Vec<SavedContact>),
    MessageReceived(SavedChatMessage),
    MessageDelivered {
        message_id: String,
        recipient_id: String,
    },
    MessageDeleted {
        contact_id: String,
        message_ids: Vec<String>,
    },
    MessageEdited {
        message_id: String,
        new_text: String,
    },
    ReactionAdded {
        message_id: String,
        emoji: String,
        reactor_id: String,
    },
    CallSignalReceived(CallSignalPayload),
    CallAudioReceived {
        sender_id: String,
        chunk: crate::calls::CallAudioChunk,
    },
}

pub struct KadMessenger {
    pub dht_node: KademliaNode,
    pub identity: Arc<RwLock<UserIdentity>>,
    pub db: Arc<RwLock<MessengerDb>>,
    pub db_path: String,
    pub outbox: Arc<MessageQueue>,
    pub replay_cache: Arc<ReplayProtectionCache>,
}

impl KadMessenger {
    pub async fn start(
        dht_node: KademliaNode,
        db_path: String,
        display_name: String,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let mut db = MessengerDb::load_from_file(&db_path)
            .map_err(|e| format!("Failed to load messenger database: {}", e))?;
        if !display_name.is_empty() && display_name != "UserNode" && display_name != "NodeX User" {
            db.display_name = display_name;
        }

        let (identity, mnemonic_opt) = if !db.mnemonic.is_empty() {
            match UserIdentity::from_mnemonic(&db.mnemonic) {
                Ok(id) => (id, None),
                Err(e) => return Err(format!("Stored database mnemonic is invalid, refusing to overwrite identity: {}", e).into()),
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

        let outbox = Arc::new(MessageQueue::new());
        let replay_cache = Arc::new(ReplayProtectionCache::new());

        let messenger = Arc::new(Self {
            dht_node,
            identity: Arc::new(RwLock::new(identity)),
            db: Arc::new(RwLock::new(db)),
            db_path,
            outbox,
            replay_cache,
        });

        // Publish initial presence
        messenger.publish_presence().await.ok();

        // Background Presence Heartbeat (re-publishes every 4s)
        let messenger_heartbeat = Arc::clone(&messenger);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(4));
            loop {
                interval.tick().await;
                let _ = messenger_heartbeat.publish_presence().await;
            }
        });

        // Background Outbox Retry Worker (retries pending messages with exponential backoff)
        let messenger_outbox = Arc::clone(&messenger);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(2));
            loop {
                interval.tick().await;
                let now = current_timestamp();
                while let Some(queued) = messenger_outbox.outbox.pop_pending(now).await {
                    let res = messenger_outbox
                        .try_deliver_message(&queued.recipient_id, &queued.message)
                        .await;
                    if res.is_ok() {
                        messenger_outbox.outbox.mark_delivered(&queued.message.id).await;
                        let mut db = messenger_outbox.db.write().await;
                        db.mark_message_delivered(&queued.message.id);
                        db.save_to_file(&messenger_outbox.db_path).ok();
                    } else {
                        messenger_outbox.outbox.schedule_retry(queued, now).await;
                    }
                }
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

    pub async fn create_invite_link(&self, ttl_days: Option<u64>) -> Result<String, String> {
        let identity = self.identity.read().await;
        let display_name = {
            let db = self.db.read().await;
            db.display_name.clone()
        };
        let endpoints = vec![self.dht_node.network.local_addr];
        let ttl_secs = ttl_days.map(|d| d * 86400);
        crate::invite::InviteManager::create_invite(&identity, &display_name, endpoints, ttl_secs)
    }

    pub async fn add_contact_by_invite(&self, invite_str: &str) -> Result<SavedContact, String> {
        let now = current_timestamp();
        let payload = crate::invite::InviteManager::parse_and_verify_invite(invite_str, now)?;

        let primary_addr = payload.endpoints.first().map(|a| a.to_string()).unwrap_or_else(|| "unknown".into());

        let contact = SavedContact {
            user_id_hex: payload.user_id.clone(),
            name: if payload.display_name.is_empty() { format!("Peer_{}", &payload.user_id[..6.min(payload.user_id.len())]) } else { payload.display_name },
            bio: "".into(),
            ed25519_pub_hex: payload.ed25519_pub_hex,
            x25519_pub_hex: payload.x25519_pub_hex,
            last_seen_addr: primary_addr,
        };

        {
            let mut db = self.db.write().await;
            db.add_contact(contact.clone());
            db.save_to_file(&self.db_path).ok();
        }

        Ok(contact)
    }

    pub fn start_inbox_polling_task(self: &Arc<Self>, event_tx: Option<std::sync::mpsc::Sender<MessengerEvent>>) {
        let messenger = Arc::clone(self);
        tokio::spawn(async move {
            let mut tick = 0u64;
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(500)) => {},
                    _ = messenger.dht_node.on_store_notify.notified() => {},
                }
                tick = tick.wrapping_add(1);

                let (my_user_id, my_node_id) = {
                    let id = messenger.identity.read().await;
                    (id.user_id_hex(), id.user_id)
                };
                let now = current_timestamp();

                // Adaptive polling schedule:
                // - Today: polled every 1s
                // - Yesterday: polled every 10s (tick % 10 == 0)
                // - Past 2..=6 days (7-day window): polled every 60s (tick % 60 == 0)
                // - Tomorrow: polled if in the last hour of day (UTC hour 23) or every 60s
                let mut keys_to_poll = Vec::new();
                keys_to_poll.push(derive_secure_mailbox_key(&my_user_id, now));

                if tick % 10 == 0 {
                    keys_to_poll.push(derive_secure_mailbox_key(&my_user_id, now.saturating_sub(86400)));
                }

                if tick % 60 == 0 {
                    for day_offset in 2..=6 {
                        keys_to_poll.push(derive_secure_mailbox_key(&my_user_id, now.saturating_sub(day_offset * 86400)));
                    }
                    keys_to_poll.push(derive_secure_mailbox_key(&my_user_id, now + 86400));
                } else {
                    let seconds_into_day = now % 86400;
                    if seconds_into_day >= 82800 { // Last hour of UTC day (23:00-23:59)
                        if tick % 10 == 0 {
                            keys_to_poll.push(derive_secure_mailbox_key(&my_user_id, now + 86400));
                        }
                    }
                }

                for mailbox_key in keys_to_poll {
                    if let Ok(Some((bytes, _from))) = messenger.dht_node.get_silent(&mailbox_key).await {
                        let envelopes: Vec<EncryptedEnvelope> = if let Ok(list) = serde_json::from_slice::<Vec<EncryptedEnvelope>>(&bytes) {
                            list
                        } else if let Ok(single) = serde_json::from_slice::<EncryptedEnvelope>(&bytes) {
                            vec![single]
                        } else {
                            Vec::new()
                        };

                        let mut received_any = false;
                        let mut unconsumed_envelopes = Vec::new();
                        for envelope in envelopes {
                        if envelope.recipient_id == my_node_id {
                            // Do not echo our own messages
                            if envelope.sender_id == my_node_id {
                                continue;
                            }

                            let sender_hex = envelope.sender_id.to_hex();

                            // Discard if sender is blocked
                            let is_blocked = {
                                let db = messenger.db.read().await;
                                db.is_blocked(&sender_hex)
                            };
                            if is_blocked {
                                continue;
                            }

                            // 1. Decrypt envelope and verify Ed25519 signature & sender_id key binding first
                            let decrypt_res = {
                                let id = messenger.identity.read().await;
                                decrypt_envelope(&id, &envelope)
                            };

                            let plaintext = match decrypt_res {
                                Ok(pt) => pt,
                                Err(e) => {
                                    println!("[MESSENGER] Rejected invalid/tampered envelope from {}: {}", sender_hex, e);
                                    continue;
                                }
                            };

                            // 2. Replay protection check: only record verified, genuine envelopes in replay cache
                            let now = current_timestamp();
                            if !messenger.replay_cache.check_and_insert(&envelope.signature, envelope.timestamp, now) {
                                println!("[MESSENGER] Rejected duplicate/expired envelope from {}", sender_hex);
                                continue;
                            }

                            let payload: ChatMessagePayload = if let Ok(p) = serde_json::from_slice::<ChatMessagePayload>(&plaintext) {
                                p
                            } else {
                                ChatMessagePayload {
                                    id: String::new(),
                                    text: String::from_utf8_lossy(&plaintext).to_string(),
                                    ..Default::default()
                                }
                            };

                            // 1. Handle Delivery Receipt acknowledgment
                            if let Some(rcpt) = payload.delivery_receipt {
                                if rcpt.verify(&envelope.sender_ed25519_pub) {
                                    let mut db = messenger.db.write().await;
                                    let is_valid_recipient = db.messages.iter().any(|m| m.id == rcpt.message_id && m.recipient_id_hex == sender_hex && !m.incoming);
                                    if is_valid_recipient {
                                        db.mark_message_delivered(&rcpt.message_id);
                                        db.save_to_file(&messenger.db_path).ok();
                                        messenger.outbox.mark_delivered(&rcpt.message_id).await;

                                        if let Some(ref tx) = event_tx {
                                            let _ = tx.send(MessengerEvent::MessageDelivered {
                                                message_id: rcpt.message_id.clone(),
                                                recipient_id: sender_hex.clone(),
                                            });
                                        }
                                        println!("[MESSENGER] Processed cryptographically verified Delivery Receipt for msg {}", rcpt.message_id);
                                        received_any = true;
                                        continue;
                                    } else {
                                        eprintln!("[SECURITY] Rejected Delivery Receipt for msg {} from non-recipient {}", rcpt.message_id, sender_hex);
                                        continue;
                                    }
                                }
                            }

                            // 2. Handle Read Receipt
                            if let Some(r_rcpt) = payload.read_receipt {
                                if r_rcpt.verify(&envelope.sender_ed25519_pub) {
                                    println!("[MESSENGER] Processed Read Receipt for {} msg(s) from {}", r_rcpt.message_ids.len(), sender_hex);
                                    received_any = true;
                                    continue;
                                }
                            }

                            // 3. Handle remote deletion commands
                            if let Some(tombstone) = payload.tombstone {
                                let valid_recipient = tombstone.recipient_id == my_user_id;
                                let valid_signature = UserIdentity::verify(
                                    &envelope.sender_ed25519_pub,
                                    &tombstone.signing_bytes(),
                                    &tombstone.signature,
                                );
                                if valid_recipient && valid_signature {
                                    let del_ids = tombstone.deleted_ids;
                                    let mut db = messenger.db.write().await;
                                    let actually_deleted = db.delete_messages_by_author(&del_ids, &sender_hex);
                                    // Register pending tombstones strictly scoped to sender_hex
                                    db.register_pending_tombstones(&sender_hex, &del_ids);
                                    db.save_to_file(&messenger.db_path).ok();
                                    
                                    if !actually_deleted.is_empty() {
                                        received_any = true;
                                        if let Some(ref tx) = event_tx {
                                            let _ = tx.send(MessengerEvent::MessageDeleted {
                                                contact_id: sender_hex.clone(),
                                                message_ids: actually_deleted,
                                            });
                                        }
                                    }
                                    println!("[MESSENGER] Processed remote deletion command from {}", sender_hex);
                                    continue;
                                }
                            }

                            // 4. Handle Call Signal
                            if let Some(call_sig) = payload.call_signal {
                                if call_sig.caller_id_hex != sender_hex {
                                    eprintln!("[SECURITY] Rejected spoofed call signal from {} claiming to be {}", sender_hex, call_sig.caller_id_hex);
                                    continue;
                                }
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs();
                                if now.saturating_sub(call_sig.timestamp) > 45 {
                                    println!("[MESSENGER] Discarded stale call signal {:?} (age: {}s)", call_sig.signal_type, now.saturating_sub(call_sig.timestamp));
                                    continue;
                                }
                                if let Some(ref tx) = event_tx {
                                    let _ = tx.send(MessengerEvent::CallSignalReceived(call_sig));
                                }
                                received_any = true;
                                continue;
                            }

                            // 4.1 Handle Real-time Call Audio Chunk
                            if let Some(audio_chunk) = payload.call_audio_chunk {
                                if let Some(ref tx) = event_tx {
                                    let _ = tx.send(MessengerEvent::CallAudioReceived {
                                        sender_id: sender_hex.clone(),
                                        chunk: audio_chunk,
                                    });
                                }
                                received_any = true;
                                continue;
                            }

                            // 5. Handle Reaction
                            if let Some(react) = payload.reaction {
                                if react.is_valid_emoji() {
                                    let mut db = messenger.db.write().await;
                                    let is_participant = db.messages.iter().any(|m| {
                                        if m.id == react.message_id {
                                            if m.sender_id_hex == sender_hex || m.recipient_id_hex == sender_hex {
                                                true
                                            } else if let Some(gid) = &m.group_id {
                                                db.groups.get(gid).map_or(false, |g| g.members.contains_key(&sender_hex))
                                            } else {
                                                false
                                            }
                                        } else {
                                            false
                                        }
                                    });
                                    if is_participant {
                                        db.add_reaction(&react.message_id, &react.emoji, &sender_hex);
                                        db.save_to_file(&messenger.db_path).ok();
                                        if let Some(ref tx) = event_tx {
                                            let _ = tx.send(MessengerEvent::ReactionAdded {
                                                message_id: react.message_id,
                                                emoji: react.emoji,
                                                reactor_id: sender_hex.clone(),
                                            });
                                        }
                                        received_any = true;
                                        continue;
                                    } else {
                                        eprintln!("[SECURITY] Rejected reaction for msg {} from non-participant {}", react.message_id, sender_hex);
                                        continue;
                                    }
                                }
                            }

                            // 6. Handle Message Edit
                            if payload.is_edited && !payload.id.is_empty() {
                                let mut db = messenger.db.write().await;
                                let edit_ts = payload.edit_timestamp.unwrap_or_else(current_timestamp);
                                if db.edit_message_by_author(&payload.id, &payload.text, edit_ts, &sender_hex) {
                                    db.save_to_file(&messenger.db_path).ok();
                                    if let Some(ref tx) = event_tx {
                                        let _ = tx.send(MessengerEvent::MessageEdited {
                                            message_id: payload.id,
                                            new_text: payload.text,
                                        });
                                    }
                                    received_any = true;
                                    continue;
                                } else {
                                    continue;
                                }
                            }

                                let text = payload.text;
                                let image_base64 = payload.image_base64;
                                let voice_note = payload.voice_note;
                                let reply_to_id = payload.reply_to_id;
                                let reply_snippet = payload.reply_snippet;
                                let group_id = payload.group_id;
                                let msg_id = if !payload.id.is_empty() {
                                    payload.id
                                } else {
                                    format!("{}_{}", envelope.timestamp, sender_hex)
                                };

                                let chat_msg = SavedChatMessage {
                                    id: msg_id.clone(),
                                    sender_id_hex: sender_hex.clone(),
                                    recipient_id_hex: my_user_id.clone(),
                                    text,
                                    image_base64,
                                    voice_note,
                                    reply_to_id,
                                    reply_snippet,
                                    is_edited: false,
                                    edit_timestamp: None,
                                    reactions: std::collections::HashMap::new(),
                                    group_id,
                                    is_pinned: false,
                                    expires_at: None,
                                    timestamp: envelope.timestamp,
                                    incoming: true,
                                    delivered: true,
                                };

                                // Auto-add sender to contact list if missing
                                let has_contact = {
                                    let db = messenger.db.read().await;
                                    db.contacts.contains_key(&sender_hex)
                                };

                                if !has_contact {
                                    let (discovered_name, discovered_bio) = match messenger.discover_peer(&sender_hex).await {
                                        Ok(card) => (card.display_name, card.bio),
                                        Err(_) => {
                                            let p: String = sender_hex.chars().take(6).collect();
                                            (format!("Peer_{}", p), String::new())
                                        }
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
                                    if db.is_tombstoned(&sender_hex, &chat_msg.id) {
                                        false
                                    } else {
                                        let before = db.messages.len();
                                        db.add_message(chat_msg.clone());
                                        db.save_to_file(&messenger.db_path).ok();
                                        db.messages.len() > before
                                    }
                                };

                                if is_new {
                                    received_any = true;

                                    // Automatically send DeliveryReceipt back to sender
                                    let messenger_ack = Arc::clone(&messenger);
                                    let ack_sender = sender_hex.clone();
                                    let ack_msg_id = msg_id.clone();
                                    tokio::spawn(async move {
                                        let _ = messenger_ack.send_delivery_receipt(&ack_sender, &ack_msg_id).await;
                                    });

                                    if let Some(ref tx) = event_tx {
                                        let db = messenger.db.read().await;
                                        let contacts: Vec<_> = db.contacts.values().cloned().collect();
                                        let _ = tx.send(MessengerEvent::ContactsUpdated(contacts));
                                        let _ = tx.send(MessengerEvent::MessageReceived(chat_msg));
                                    }
                                    println!("[MESSENGER] Received E2EE message from {} via Transport Cascade!", sender_hex);
                                }
                        } else {
                            unconsumed_envelopes.push(envelope);
                        }
                    }

                    // Drain consumed envelopes from mailbox storage
                    if unconsumed_envelopes.is_empty() {
                        let mailbox_node_id = NodeId::from_key(mailbox_key.as_bytes());
                        messenger.dht_node.storage.write().await.remove(&mailbox_node_id);
                    } else {
                        let payload = serde_json::to_vec(&unconsumed_envelopes).unwrap_or_default();
                        let _ = messenger.dht_node.put(&mailbox_key, payload).await;
                    }

                        let _ = received_any;
                    }
                }
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

        let card = UserPresenceCard::create_with_endpoints(
            &identity,
            display_name,
            bio,
            self.dht_node.network.local_addr,
            vec![self.dht_node.network.local_addr],
            nodex_kademlia::nat_type::NatCategory::Unknown,
            timestamp,
        );

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

            if card.ed25519_pub.len() != 32 {
                return Err("Invalid Ed25519 public key length in presence card".into());
            }
            let expected_node_id = NodeId::from_key(&card.ed25519_pub);
            if expected_node_id.to_hex() != card.user_id_hex {
                return Err("Presence card User ID does not match Ed25519 public key".into());
            }

            card.verify_signature().map_err(|e| e.to_string())?;

            let mut db = self.db.write().await;
            let existing_name = db.contacts.get(&card.user_id_hex).map(|c| c.name.clone());
            let name_to_use = match existing_name {
                Some(ref custom) if !custom.trim().is_empty() && !custom.starts_with("Peer_") => custom.clone(),
                _ => {
                    if !card.display_name.trim().is_empty() {
                        card.display_name.clone()
                    } else {
                        format!("Peer_{}", &card.user_id_hex[..6.min(card.user_id_hex.len())])
                    }
                }
            };
            db.add_contact(SavedContact {
                user_id_hex: card.user_id_hex.clone(),
                name: name_to_use,
                bio: card.bio.clone(),
                ed25519_pub_hex: hex_string(&card.ed25519_pub),
                x25519_pub_hex: hex_string(&card.x25519_pub),
                last_seen_addr: card.socket_addr.to_string(),
            });
            db.save_to_file(&self.db_path).ok();

            // Update routing table with cryptographically verified presence address
            self.dht_node.routing_table.write().await.force_update(Contact::new(expected_node_id, card.socket_addr));

            Ok(card)
        } else {
            Err(format!("Peer {} not found in DHT", user_id_hex))
        }
    }

    /// Embeds current user's contact information and public endpoints into an image file (PNG/JPG) using Stego-Carrier.
    pub async fn export_stego_avatar(
        &self,
        input_image_bytes: &[u8],
        passphrase: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let img = image::load_from_memory(input_image_bytes)
            .map_err(|e| format!("Failed to read input image for stego: {}", e))?;

        let identity = self.identity.read().await;
        let (display_name, bio) = {
            let db = self.db.read().await;
            (db.display_name.clone(), db.bio.clone())
        };

        let timestamp = current_timestamp();
        let endpoints = vec![self.dht_node.network.local_addr];

        // Sign contact metadata with Ed25519
        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(identity.user_id_hex().as_bytes());
        sign_data.extend_from_slice(identity.verifying_key.as_bytes());
        sign_data.extend_from_slice(identity.x25519_public.as_bytes());
        sign_data.extend_from_slice(&timestamp.to_be_bytes());

        use ed25519_dalek::Signer;
        let signature = identity.signing_key.sign(&sign_data);

        let stego_card = crate::stego::StegoContactCard {
            user_id_hex: identity.user_id_hex(),
            display_name,
            bio,
            ed25519_pub_hex: hex_string(identity.verifying_key.as_bytes()),
            x25519_pub_hex: hex_string(identity.x25519_public.as_bytes()),
            endpoints,
            timestamp,
            signature_hex: hex_string(&signature.to_bytes()),
        };

        crate::stego::StegoCarrier::embed_contact_into_image(&img, &stego_card, passphrase)
    }

    /// Extracts and cryptographically verifies a Stego-Carrier contact card from an image, adding it to saved contacts.
    pub async fn import_stego_avatar(
        &self,
        image_bytes: &[u8],
        passphrase: Option<&str>,
    ) -> Result<SavedContact, String> {
        let card = crate::stego::StegoCarrier::extract_contact_from_image(image_bytes, passphrase)?;

        let ed25519_bytes = hex_decode(&card.ed25519_pub_hex)
            .ok_or_else(|| "Invalid Ed25519 hex in stego contact".to_string())?;
        if ed25519_bytes.len() != 32 {
            return Err("Invalid Ed25519 public key length".to_string());
        }

        let expected_node_id = NodeId::from_key(&ed25519_bytes);
        if expected_node_id.to_hex() != card.user_id_hex {
            return Err("Stego contact User ID does not match Ed25519 key".to_string());
        }

        let sig_bytes = hex_decode(&card.signature_hex)
            .ok_or_else(|| "Invalid signature hex in stego contact".to_string())?;
        if sig_bytes.len() != 64 {
            return Err("Invalid signature length in stego contact".to_string());
        }

        let mut sign_data = Vec::new();
        sign_data.extend_from_slice(card.user_id_hex.as_bytes());
        sign_data.extend_from_slice(&ed25519_bytes);
        if let Some(x_bytes) = hex_decode(&card.x25519_pub_hex) {
            sign_data.extend_from_slice(&x_bytes);
        }
        sign_data.extend_from_slice(&card.timestamp.to_be_bytes());

        let verifier = ed25519_dalek::VerifyingKey::from_bytes(ed25519_bytes.as_slice().try_into().unwrap())
            .map_err(|e| format!("Invalid verifying key: {}", e))?;
        let sig = ed25519_dalek::Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());

        use ed25519_dalek::Verifier;
        verifier
            .verify(&sign_data, &sig)
            .map_err(|e| format!("Stego contact signature verification failed: {}", e))?;

        let last_seen_addr = card.endpoints.first().map(|a| a.to_string()).unwrap_or_else(|| "unknown".to_string());

        let saved = SavedContact {
            user_id_hex: card.user_id_hex.clone(),
            name: if card.display_name.trim().is_empty() {
                format!("Peer_{}", &card.user_id_hex[..8])
            } else {
                card.display_name.clone()
            },
            bio: card.bio.clone(),
            ed25519_pub_hex: card.ed25519_pub_hex.clone(),
            x25519_pub_hex: card.x25519_pub_hex.clone(),
            last_seen_addr,
        };

        {
            let mut db = self.db.write().await;
            db.contacts.insert(card.user_id_hex.clone(), saved.clone());
            db.save_to_file(&self.db_path).ok();
        }

        if let Some(first_addr) = card.endpoints.first() {
            self.dht_node.routing_table.write().await.force_update(Contact::new(expected_node_id, *first_addr));
        }

        println!("[STEGO] Successfully imported contact {} ({}) from image", saved.name, saved.user_id_hex);
        Ok(saved)
    }


    pub async fn send_message(
        &self,
        recipient_id_hex: &str,
        text: &str,
        image_base64: Option<String>,
    ) -> Result<SavedChatMessage, MessengerError> {
        if self.db.read().await.is_blocked(recipient_id_hex) {
            return Err(MessengerError::ContactBlocked(recipient_id_hex.to_string()));
        }

        let my_user_id_hex = self.identity.read().await.user_id_hex();
        let short_id: String = my_user_id_hex.chars().take(8).collect();
        let msg_id = format!("{}_{}_{:x}", current_timestamp(), short_id, rand::random::<u32>());

        let chat_msg = SavedChatMessage {
            id: msg_id.clone(),
            sender_id_hex: my_user_id_hex,
            recipient_id_hex: recipient_id_hex.to_string(),
            text: text.to_string(),
            image_base64,
            voice_note: None,
            reply_to_id: None,
            reply_snippet: None,
            is_edited: false,
            edit_timestamp: None,
            reactions: std::collections::HashMap::new(),
            group_id: None,
            is_pinned: false,
            expires_at: None,
            timestamp: current_timestamp(),
            incoming: false,
            delivered: false,
        };

        // 1. Try deliver immediately through the 5-tier cascade
        match self.try_deliver_message(recipient_id_hex, &chat_msg).await {
            Ok(()) => {
                let mut final_msg = chat_msg;
                // Delivered is true immediately only for self_saved_messages. Peer messages await cryptographic DeliveryReceipt.
                final_msg.delivered = recipient_id_hex == "self_saved_messages";
                let mut db = self.db.write().await;
                db.add_message(final_msg.clone());
                db.save_to_file(&self.db_path).ok();
                Ok(final_msg)
            }
            Err(MessengerError::RecipientVerifiedKeyMissing(id)) => {
                Err(MessengerError::RecipientVerifiedKeyMissing(id))
            }
            Err(MessengerError::ContactBlocked(id)) => {
                Err(MessengerError::ContactBlocked(id))
            }
            Err(_other_err) => {
                // If delivery failed due to network / peer temporarily offline, queue for background retries
                let mut final_msg = chat_msg;
                final_msg.delivered = false;
                {
                    let mut db = self.db.write().await;
                    db.add_message(final_msg.clone());
                    db.save_to_file(&self.db_path).ok();
                }
                self.outbox.push(final_msg.clone(), recipient_id_hex.to_string()).await;
                Ok(final_msg)
            }
        }
    }

    /// Send a large file transfer (up to 100 MB) with chunked manifest protocol.
    pub async fn send_file(
        &self,
        recipient_id_hex: &str,
        file_name: &str,
        file_bytes: &[u8],
    ) -> Result<(SavedChatMessage, FileManifest), MessengerError> {
        let (manifest, chunks) = AttachmentManager::create_file_transfer(file_name, file_bytes, None)?;

        // Store media chunks in local DHT storage for peer retrieval
        for chunk in &chunks {
            let chunk_key = format!("chunk_{}_{}", chunk.file_id, chunk.chunk_index);
            let chunk_payload = serde_json::to_vec(chunk).unwrap_or_default();
            let _ = self.dht_node.put(&chunk_key, chunk_payload).await;
        }

        let my_user_id_hex = self.identity.read().await.user_id_hex();
        let short_id: String = my_user_id_hex.chars().take(8).collect();
        let msg_id = format!("{}_{}_{:x}", current_timestamp(), short_id, rand::random::<u32>());

        let text_preview = format!("Shared a file: {} ({:.1} KB)", file_name, file_bytes.len() as f64 / 1024.0);

        let chat_msg = SavedChatMessage {
            id: msg_id.clone(),
            sender_id_hex: my_user_id_hex,
            recipient_id_hex: recipient_id_hex.to_string(),
            text: text_preview,
            image_base64: None,
            voice_note: None,
            reply_to_id: None,
            reply_snippet: None,
            is_edited: false,
            edit_timestamp: None,
            reactions: std::collections::HashMap::new(),
            group_id: None,
            is_pinned: false,
            expires_at: None,
            timestamp: current_timestamp(),
            incoming: false,
            delivered: false,
        };

        let payload_struct = ChatMessagePayload {
            id: msg_id.clone(),
            text: chat_msg.text.clone(),
            file_manifest: Some(manifest.clone()),
            ..Default::default()
        };

        let delivered = self.try_deliver_payload(recipient_id_hex, &payload_struct).await.is_ok();
        let mut final_msg = chat_msg;
        final_msg.delivered = delivered;

        {
            let mut db = self.db.write().await;
            db.add_message(final_msg.clone());
            db.save_to_file(&self.db_path).ok();
        }

        if !delivered {
            self.outbox.push(final_msg.clone(), recipient_id_hex.to_string()).await;
        }

        Ok((final_msg, manifest))
    }

    /// Helper method delivering payload through 5-tier cascade
    async fn try_deliver_payload(&self, recipient_id_hex: &str, payload_struct: &ChatMessagePayload) -> Result<(), MessengerError> {
        let recipient_id = NodeId::from_hex(recipient_id_hex)
            .map_err(|e| MessengerError::Other(format!("Invalid recipient hex: {}", e)))?;

        // Resolve recipient's public key and endpoints
        let cached = {
            let db = self.db.read().await;
            db.contacts.get(recipient_id_hex).map(|c| {
                let direct_addr = c.last_seen_addr.parse::<SocketAddr>().ok();
                let x_bytes = hex_decode(&c.x25519_pub_hex);
                (x_bytes, direct_addr)
            })
        };

        let (recipient_x25519_bytes, target_socket_addr, candidate_addrs) = match cached {
            Some((Some(x_bytes), direct_addr)) if x_bytes.len() == 32 => {
                (x_bytes, direct_addr, direct_addr.map(|a| vec![a]).unwrap_or_default())
            }
            _ => match self.discover_peer(recipient_id_hex).await {
                Ok(card) => (card.x25519_pub, Some(card.socket_addr), card.endpoints),
                Err(_) => (Vec::new(), None, Vec::new()),
            },
        };

        let target_socket_addr = target_socket_addr.or_else(|| {
            let rt = self.dht_node.routing_table.try_read().ok()?;
            rt.all_contacts().into_iter().find(|c| c.id == recipient_id).map(|c| c.addr)
        });

        if recipient_x25519_bytes.len() != 32 {
            return Err(MessengerError::RecipientVerifiedKeyMissing(recipient_id_hex.to_string()));
        }

        let mut x25519_arr = [0u8; 32];
        x25519_arr.copy_from_slice(&recipient_x25519_bytes);
        let recipient_x25519 = x25519_dalek::PublicKey::from(x25519_arr);

        let payload_bytes = serde_json::to_vec(payload_struct).map_err(|e| MessengerError::Other(e.to_string()))?;

        let envelope = {
            let id = self.identity.read().await;
            encrypt_envelope(&id, recipient_id, &recipient_x25519, &payload_bytes)
                .map_err(MessengerError::Other)?
        };

        let mailbox_key = current_mailbox_key(recipient_id_hex);
        let mailbox_key_id = NodeId::from_key(mailbox_key.as_bytes());
        let single_payload = serde_json::to_vec(&vec![envelope.clone()]).unwrap_or_default();

        // Tier 1: Direct UDP Instant Delivery
        if let Some(addr) = target_socket_addr {
            if let Ok(reply) = self.dht_node.network.call(
                addr,
                RpcPayload::Store { key: mailbox_key_id, value: single_payload.clone() },
                RPC_TIMEOUT,
                1,
            ).await {
                if let RpcPayload::StoreAck { ok: true } = reply.payload {
                    println!("[CASCADE] Tier 1: Direct UDP delivery succeeded to {}", addr);
                    return Ok(());
                }
            }
        }

        // Tier 2: UDP Hole Punching to candidate endpoints
        if !candidate_addrs.is_empty() {
            let punched = HolePuncher::punch_candidates_verified(&self.dht_node.network.socket, &candidate_addrs).await;
            for addr in punched {
                if let Ok(reply) = self.dht_node.network.call(
                    addr,
                    RpcPayload::Store { key: mailbox_key_id, value: single_payload.clone() },
                    RPC_TIMEOUT,
                    1,
                ).await {
                    if let RpcPayload::StoreAck { ok: true } = reply.payload {
                        println!("[CASCADE] Tier 2: Hole Punching delivery succeeded to {}", addr);
                        return Ok(());
                    }
                }
            }
        }

        // Tier 3: Deposit into DHT Mailbox (Offline/Relay delivery)
        deposit_envelope_to_mailbox(&self.dht_node, recipient_id_hex, envelope).await
            .map(|_| ())
            .map_err(MessengerError::Other)?;
        println!("[CASCADE] Tier 3: Deposited into DHT Mailbox for {}", recipient_id_hex);

        Ok(())
    }

    /// Internal 5-tier transport cascade for envelope delivery.
    async fn try_deliver_message(&self, recipient_id_hex: &str, msg: &SavedChatMessage) -> Result<(), MessengerError> {
        let payload_struct = ChatMessagePayload {
            id: msg.id.clone(),
            text: msg.text.clone(),
            image_base64: msg.image_base64.clone(),
            voice_note: msg.voice_note.clone(),
            file_manifest: None,
            delivery_receipt: None,
            read_receipt: None,
            reaction: None,
            reply_to_id: msg.reply_to_id.clone(),
            reply_snippet: msg.reply_snippet.clone(),
            is_edited: msg.is_edited,
            edit_timestamp: msg.edit_timestamp,
            group_id: msg.group_id.clone(),
            call_signal: None,
            call_audio_chunk: None,
            delete_message_ids: None,
            tombstone: None,
        };
        self.try_deliver_payload(recipient_id_hex, &payload_struct).await
    }

    pub async fn send_delivery_receipt(&self, recipient_id_hex: &str, message_id: &str) -> Result<(), String> {
        let recipient_id = NodeId::from_hex(recipient_id_hex)?;
        let rcpt = {
            let id = self.identity.read().await;
            DeliveryReceipt::create(&id, message_id, current_timestamp())
        };

        let (recipient_x25519_bytes, target_socket_addr) = {
            let db = self.db.read().await;
            if let Some(c) = db.contacts.get(recipient_id_hex) {
                (hex_decode(&c.x25519_pub_hex), c.last_seen_addr.parse::<SocketAddr>().ok())
            } else {
                (None, None)
            }
        };

        let Some(k) = recipient_x25519_bytes else { return Ok(()); };
        if k.len() != 32 { return Ok(()); }
        let mut x25519_arr = [0u8; 32];
        x25519_arr.copy_from_slice(&k);
        let recipient_x25519 = x25519_dalek::PublicKey::from(x25519_arr);

        let payload_struct = ChatMessagePayload {
            id: format!("rcpt_{}", message_id),
            delivery_receipt: Some(rcpt),
            ..Default::default()
        };
        let payload_bytes = serde_json::to_vec(&payload_struct).map_err(|e| e.to_string())?;

        let envelope = {
            let id = self.identity.read().await;
            encrypt_envelope(&id, recipient_id, &recipient_x25519, &payload_bytes)?
        };

        let mailbox_key = current_mailbox_key(recipient_id_hex);
        let mailbox_key_id = NodeId::from_key(mailbox_key.as_bytes());
        let single_payload = serde_json::to_vec(&vec![envelope.clone()]).unwrap_or_default();

        if let Some(addr) = target_socket_addr {
            let _ = self.dht_node.network.call(
                addr,
                RpcPayload::Store { key: mailbox_key_id, value: single_payload },
                RPC_TIMEOUT,
                1,
            ).await;
        }
        let _ = deposit_envelope_to_mailbox(&self.dht_node, recipient_id_hex, envelope).await;

        Ok(())
    }

    pub async fn delete_message(&self, message_id: &str) {
        let mut db = self.db.write().await;
        db.delete_message(message_id);
        db.save_to_file(&self.db_path).ok();
    }

    pub async fn delete_messages(&self, message_ids: &[String]) {
        let mut db = self.db.write().await;
        db.delete_messages(message_ids);
        db.save_to_file(&self.db_path).ok();
    }

    pub async fn delete_messages_for_everyone(&self, recipient_id_hex: &str, message_ids: Vec<String>) -> Result<(), String> {
        if message_ids.is_empty() {
            return Ok(());
        }

        let my_id = self.identity.read().await.user_id_hex();

        // 1. Delete locally only messages authored by ourselves
        let deleted_ids = {
            let mut db = self.db.write().await;
            let author_deleted = db.delete_messages_by_author(&message_ids, &my_id);
            db.save_to_file(&self.db_path).ok();
            author_deleted
        };

        if deleted_ids.is_empty() {
            return Ok(());
        }

        // 2. Deliver remote deletion payload to recipient
        let recipient_id = NodeId::from_hex(recipient_id_hex)?;
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
                        Err(_) => (Vec::new(), None),
                    }
                }
            } else {
                match self.discover_peer(recipient_id_hex).await {
                    Ok(card) => (card.x25519_pub, Some(card.socket_addr)),
                    Err(_) => (Vec::new(), None),
                }
            }
        };

        if recipient_x25519_bytes.len() == 32 {
            let mut x25519_arr = [0u8; 32];
            x25519_arr.copy_from_slice(&recipient_x25519_bytes);
            let recipient_x25519 = x25519_dalek::PublicKey::from(x25519_arr);

            let mut tombstone = SignedTombstone {
                recipient_id: recipient_id_hex.to_string(),
                timestamp: current_timestamp(),
                deleted_ids: deleted_ids.clone(),
                signature: Vec::new(),
            };
            {
                let identity = self.identity.read().await;
                tombstone.signature = identity.sign(&tombstone.signing_bytes());
            }
            let payload_struct = ChatMessagePayload {
                tombstone: Some(tombstone),
                ..Default::default()
            };

            if let Ok(payload_bytes) = serde_json::to_vec(&payload_struct) {
                if let Ok(envelope) = {
                    let id = self.identity.read().await;
                    encrypt_envelope(&id, recipient_id, &recipient_x25519, &payload_bytes)
                } {
                    let mailbox_key = current_mailbox_key(recipient_id_hex);
                    let mailbox_key_id = NodeId::from_key(mailbox_key.as_bytes());
                    let single_payload = serde_json::to_vec(&vec![envelope.clone()]).unwrap_or_default();

                    if let Some(addr) = target_socket_addr {
                        let _ = self.dht_node.network.call(
                            addr,
                            RpcPayload::Store { key: mailbox_key_id, value: single_payload },
                            RPC_TIMEOUT,
                            RPC_RETRIES,
                        ).await;
                    }
                    let _ = deposit_envelope_to_mailbox(&self.dht_node, recipient_id_hex, envelope).await;
                }
            }
        }

        Ok(())
    }

    pub async fn block_contact(&self, contact_id_hex: &str) {
        let mut db = self.db.write().await;
        db.block_user(contact_id_hex);
        db.save_to_file(&self.db_path).ok();
    }

    pub async fn unblock_contact(&self, contact_id_hex: &str) {
        let mut db = self.db.write().await;
        db.unblock_user(contact_id_hex);
        db.save_to_file(&self.db_path).ok();
    }

    pub async fn clear_chat(&self, contact_id_hex: &str) {
        {
            let mut db = self.db.write().await;
            db.clear_messages_for_contact(contact_id_hex);
            db.save_to_file(&self.db_path).ok();
        }
        let my_hex = self.user_id_hex().await;
        let my_mailbox = current_mailbox_key(&my_hex);
        let contact_mailbox = current_mailbox_key(contact_id_hex);
        self.dht_node.storage.write().await.remove(&NodeId::from_key(my_mailbox.as_bytes()));
        self.dht_node.storage.write().await.remove(&NodeId::from_key(contact_mailbox.as_bytes()));
    }

    pub async fn delete_contact(&self, contact_id_hex: &str) {
        self.clear_chat(contact_id_hex).await;
        let mut db = self.db.write().await;
        db.delete_contact(contact_id_hex);
        db.save_to_file(&self.db_path).ok();
    }

    /// Send a voice message note to a peer
    pub async fn send_voice_note(
        &self,
        recipient_id_hex: &str,
        voice_note: VoiceNote,
    ) -> Result<SavedChatMessage, MessengerError> {
        let text_preview = format!("🎤 Voice message ({})", voice_note.formatted_duration());
        let my_user_id_hex = self.identity.read().await.user_id_hex();
        let short_id: String = my_user_id_hex.chars().take(8).collect();
        let msg_id = format!("{}_{}_{:x}", current_timestamp(), short_id, rand::random::<u32>());

        let chat_msg = SavedChatMessage {
            id: msg_id.clone(),
            sender_id_hex: my_user_id_hex,
            recipient_id_hex: recipient_id_hex.to_string(),
            text: text_preview,
            image_base64: None,
            voice_note: Some(voice_note),
            reply_to_id: None,
            reply_snippet: None,
            is_edited: false,
            edit_timestamp: None,
            reactions: std::collections::HashMap::new(),
            group_id: None,
            is_pinned: false,
            expires_at: None,
            timestamp: current_timestamp(),
            incoming: false,
            delivered: false,
        };

        let delivered = self.try_deliver_message(recipient_id_hex, &chat_msg).await.is_ok();
        let mut final_msg = chat_msg;
        final_msg.delivered = delivered;

        {
            let mut db = self.db.write().await;
            db.add_message(final_msg.clone());
            db.save_to_file(&self.db_path).ok();
        }

        if !delivered {
            self.outbox.push(final_msg.clone(), recipient_id_hex.to_string()).await;
        }

        Ok(final_msg)
    }

    /// Add an emoji reaction to a message and send it to the recipient peer
    pub async fn react_to_message(
        &self,
        recipient_id_hex: &str,
        message_id: &str,
        emoji: &str,
    ) -> Result<(), MessengerError> {
        let my_user_id_hex = self.identity.read().await.user_id_hex();
        {
            let mut db = self.db.write().await;
            db.add_reaction(message_id, emoji, &my_user_id_hex);
            db.save_to_file(&self.db_path).ok();
        }

        let reaction = MessageReaction::new(message_id, emoji, &my_user_id_hex);
        let payload_struct = ChatMessagePayload {
            id: format!("react_{}", message_id),
            text: emoji.to_string(),
            reaction: Some(reaction),
            ..Default::default()
        };
        let _ = self.try_deliver_payload(recipient_id_hex, &payload_struct).await;
        Ok(())
    }

    /// Edit a previously sent message
    pub async fn edit_message(
        &self,
        recipient_id_hex: &str,
        message_id: &str,
        new_text: &str,
    ) -> Result<(), MessengerError> {
        let ts = current_timestamp();
        {
            let mut db = self.db.write().await;
            db.edit_message(message_id, new_text, ts);
            db.save_to_file(&self.db_path).ok();
        }

        let my_user_id_hex = self.identity.read().await.user_id_hex();
        let edit_msg = SavedChatMessage {
            id: message_id.to_string(),
            sender_id_hex: my_user_id_hex,
            recipient_id_hex: recipient_id_hex.to_string(),
            text: new_text.to_string(),
            image_base64: None,
            voice_note: None,
            reply_to_id: None,
            reply_snippet: None,
            is_edited: true,
            edit_timestamp: Some(ts),
            reactions: std::collections::HashMap::new(),
            group_id: None,
            is_pinned: false,
            expires_at: None,
            timestamp: ts,
            incoming: false,
            delivered: false,
        };
        let _ = self.try_deliver_message(recipient_id_hex, &edit_msg).await;
        Ok(())
    }

    /// Pin or unpin a message in local DB
    pub async fn pin_message(&self, message_id: &str) -> bool {
        let mut db = self.db.write().await;
        let res = db.toggle_pin_message(message_id);
        db.save_to_file(&self.db_path).ok();
        res
    }

    /// Create a new P2P E2EE Group Chat
    pub async fn create_group(&self, title: &str, description: &str) -> P2PGroup {
        let my_id = self.identity.read().await.user_id_hex();
        let my_name = {
            let db = self.db.read().await;
            db.display_name.clone()
        };
        let group = P2PGroup::new(title, description, &my_id, &my_name);
        {
            let mut db = self.db.write().await;
            db.add_group(group.clone());
            db.save_to_file(&self.db_path).ok();
        }
        group
    }

    /// Send a message to all members in a P2P Group
    pub async fn send_group_message(
        &self,
        group_id: &str,
        text: &str,
    ) -> Result<SavedChatMessage, MessengerError> {
        let my_id = self.identity.read().await.user_id_hex();
        let members = {
            let db = self.db.read().await;
            let grp = db.get_group(group_id).ok_or_else(|| MessengerError::Other("Group not found".into()))?;
            grp.members.keys().cloned().collect::<Vec<_>>()
        };

        let short_id: String = my_id.chars().take(8).collect();
        let msg_id = format!("{}_{}_{:x}", current_timestamp(), short_id, rand::random::<u32>());
        let chat_msg = SavedChatMessage {
            id: msg_id.clone(),
            sender_id_hex: my_id.clone(),
            recipient_id_hex: group_id.to_string(),
            text: text.to_string(),
            image_base64: None,
            voice_note: None,
            reply_to_id: None,
            reply_snippet: None,
            is_edited: false,
            edit_timestamp: None,
            reactions: std::collections::HashMap::new(),
            group_id: Some(group_id.to_string()),
            is_pinned: false,
            expires_at: None,
            timestamp: current_timestamp(),
            incoming: false,
            delivered: true,
        };

        {
            let mut db = self.db.write().await;
            db.add_message(chat_msg.clone());
            db.save_to_file(&self.db_path).ok();
        }

        // Broadcast to all other group members
        for member_id in members {
            if member_id != my_id {
                let _ = self.try_deliver_message(&member_id, &chat_msg).await;
            }
        }

        Ok(chat_msg)
    }

    /// Send Call Signaling message (Offer, Answer, Hangup, Candidate)
    pub async fn send_call_signal(
        &self,
        recipient_id_hex: &str,
        signal_type: crate::calls::CallSignalType,
        call_id: &str,
    ) -> Result<(), MessengerError> {
        let my_id = self.identity.read().await.user_id_hex();
        let sig = crate::calls::CallSignalPayload::new(call_id, &my_id, recipient_id_hex, signal_type);
        let rand_suffix: u64 = rand::random();
        let payload_struct = ChatMessagePayload {
            id: format!("call_{}_{}_{:x}", call_id, sig.timestamp, rand_suffix),
            text: format!("[Call Signal {:?}]", sig.signal_type),
            call_signal: Some(sig),
            ..Default::default()
        };
        self.try_deliver_payload(recipient_id_hex, &payload_struct).await
    }

    /// Send Real-time Voice Call Audio Chunk (PCM samples) directly via UDP Fast Path
    pub async fn send_call_audio(
        &self,
        recipient_id_hex: &str,
        call_id: &str,
        pcm_samples: &[i16],
        sample_rate: u32,
        seq: u64,
    ) -> Result<(), MessengerError> {
        use base64::Engine;
        let mut pcm_bytes = Vec::with_capacity(pcm_samples.len() * 2);
        for &s in pcm_samples {
            pcm_bytes.extend_from_slice(&s.to_le_bytes());
        }
        let pcm_base64 = base64::engine::general_purpose::STANDARD.encode(&pcm_bytes);
        let chunk = crate::calls::CallAudioChunk {
            call_id: call_id.to_string(),
            pcm_base64,
            sample_rate,
            seq,
        };
        let payload = ChatMessagePayload {
            id: format!("audio_{}_{}", call_id, seq),
            call_audio_chunk: Some(chunk),
            ..Default::default()
        };

        let recipient_id = NodeId::from_hex(recipient_id_hex)
            .map_err(|e| MessengerError::Other(format!("Invalid recipient hex: {}", e)))?;

        // Resolve recipient's public key and endpoints from local cache or routing table
        let (recipient_x25519_bytes, target_socket_addr) = {
            let db = self.db.read().await;
            if let Some(c) = db.contacts.get(recipient_id_hex) {
                let direct_addr = c.last_seen_addr.parse::<std::net::SocketAddr>().ok();
                let x_bytes = hex_decode(&c.x25519_pub_hex);
                (x_bytes, direct_addr)
            } else {
                (None, None)
            }
        };

        let target_socket_addr = target_socket_addr.or_else(|| {
            let rt = self.dht_node.routing_table.try_read().ok()?;
            rt.all_contacts().into_iter().find(|c| c.id == recipient_id).map(|c| c.addr)
        });

        let Some(x_bytes) = recipient_x25519_bytes else {
            return Ok(());
        };
        if x_bytes.len() != 32 {
            return Ok(());
        }

        let mut x25519_arr = [0u8; 32];
        x25519_arr.copy_from_slice(&x_bytes);
        let recipient_x25519 = x25519_dalek::PublicKey::from(x25519_arr);

        let payload_bytes = serde_json::to_vec(&payload).map_err(|e| MessengerError::Other(e.to_string()))?;

        let envelope = {
            let id = self.identity.read().await;
            encrypt_envelope(&id, recipient_id, &recipient_x25519, &payload_bytes)
                .map_err(MessengerError::Other)?
        };

        let mailbox_key = current_mailbox_key(recipient_id_hex);
        let mailbox_key_id = NodeId::from_key(mailbox_key.as_bytes());
        let single_payload = serde_json::to_vec(&vec![envelope]).unwrap_or_default();

        // Direct UDP Fast Stream: send lightweight Store RPC directly to peer's address without queuing heavy DHT lookups
        if let Some(addr) = target_socket_addr {
            let req_id = rand::random::<u64>();
            let rpc_msg = nodex_kademlia::rpc::RpcMessage::new(
                req_id,
                self.dht_node.node_id,
                nodex_kademlia::rpc::RpcPayload::Store {
                    key: mailbox_key_id,
                    value: single_payload,
                },
            );
            let _ = self.dht_node.network.send_message(addr, &rpc_msg).await;
        }

        Ok(())
    }

    /// Search all local messages by keyword query
    pub async fn search_messages(&self, query: &str, limit: usize) -> Vec<SavedChatMessage> {
        let db = self.db.read().await;
        db.search_messages(query, limit)
    }

    pub async fn wipe_local_data(&self) {
        {
            let mut db = self.db.write().await;
            db.contacts.clear();
            db.messages.clear();
            db.deleted_message_ids.clear();
            db.blocked_user_ids.clear();
            db.mnemonic.clear();
            db.user_seed = [0u8; 32];
            db.display_name.clear();
            db.bio.clear();
        }
        let _ = std::fs::remove_file(&self.db_path);
        let mut kpath = std::path::PathBuf::from(&self.db_path);
        let ext = kpath.extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_default();
        if ext.is_empty() {
            kpath.set_extension("key");
        } else {
            kpath.set_extension(format!("{}.key", ext));
        }
        let _ = std::fs::remove_file(kpath);
        crate::db::forget_instance_key(&self.db_path);

        self.dht_node.storage.write().await.clear();
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
    if s.is_empty() || !s.len().is_multiple_of(2) {
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
    derive_secure_mailbox_key(&user_id.to_hex(), timestamp)
}

fn current_mailbox_key(user_id_hex: &str) -> String {
    derive_secure_mailbox_key(user_id_hex, current_timestamp())
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
        assert!(c_key.starts_with("mbx_"));

        let crc = calculate_c_checksum(b"Vortex P2P Message");
        assert_ne!(crc, 0);
    }
}

