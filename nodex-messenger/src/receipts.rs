use serde::{Deserialize, Serialize};
use crate::identity::UserIdentity;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeliveryReceipt {
    pub message_id: String,
    pub recipient_id: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl DeliveryReceipt {
    pub fn create(identity: &UserIdentity, message_id: &str, timestamp: u64) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(b"DELIVERY_RECEIPT_V1");
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

    pub fn verify(&self, recipient_ed25519_pub: &[u8]) -> bool {
        let mut data = Vec::new();
        data.extend_from_slice(b"DELIVERY_RECEIPT_V1");
        data.extend_from_slice(self.message_id.as_bytes());
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        UserIdentity::verify(recipient_ed25519_pub, &data, &self.signature)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReadReceipt {
    pub message_ids: Vec<String>,
    pub reader_id: String,
    pub timestamp: u64,
    pub signature: Vec<u8>,
}

impl ReadReceipt {
    pub fn create(identity: &UserIdentity, message_ids: &[String], timestamp: u64) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(b"READ_RECEIPT_V1");
        for id in message_ids {
            data.extend_from_slice(id.as_bytes());
        }
        data.extend_from_slice(&timestamp.to_be_bytes());
        let signature = identity.sign(&data);

        Self {
            message_ids: message_ids.to_vec(),
            reader_id: identity.user_id_hex(),
            timestamp,
            signature,
        }
    }

    pub fn verify(&self, reader_ed25519_pub: &[u8]) -> bool {
        let mut data = Vec::new();
        data.extend_from_slice(b"READ_RECEIPT_V1");
        for id in &self.message_ids {
            data.extend_from_slice(id.as_bytes());
        }
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        UserIdentity::verify(reader_ed25519_pub, &data, &self.signature)
    }
}

