use serde::{Deserialize, Serialize};
use crate::crypto::EncryptedEnvelope;
use crate::message_status::MessageStatus;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MailboxMessage {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: MessageStatus,
    pub encrypted_payload: EncryptedEnvelope,
}

pub struct MailboxManager;

impl MailboxManager {
    pub fn format_key(recipient_id_hex: &str, timestamp: u64) -> String {
        crate::crypto::derive_secure_mailbox_key(recipient_id_hex, timestamp)
    }
}
