use serde::{Deserialize, Serialize};
use crate::identity::UserIdentity;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeliveryReceipt {
    pub message_id: String,
    pub recipient_id: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl DeliveryReceipt {
    pub fn create(identity: &UserIdentity, message_id: &str, timestamp: u64) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(message_id.as_bytes());
        data.extend_from_slice(&timestamp.to_be_bytes());
        let signature = identity.sign(&data);

        Self {
            message_id: message_id.to_string(),
            recipient_id: identity.user_id_hex(),
            timestamp,
            signature,
        }
    }
}
