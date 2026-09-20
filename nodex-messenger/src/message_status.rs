use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageStatus {
    Composing,
    Sending,
    Sent,
    Delivered,
    Read,
    Failed,
    Expired,
}

impl MessageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageStatus::Composing => "Composing",
            MessageStatus::Sending => "Sending",
            MessageStatus::Sent => "Sent",
            MessageStatus::Delivered => "Delivered",
            MessageStatus::Read => "Read",
            MessageStatus::Failed => "Failed",
            MessageStatus::Expired => "Expired",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTimestamps {
    pub created_at: u64,
    pub sent_at: Option<u64>,
    pub delivered_at: Option<u64>,
    pub read_at: Option<u64>,
    pub failed_at: Option<u64>,
}

impl MessageTimestamps {
    pub fn new(created_at: u64) -> Self {
        Self {
            created_at,
            sent_at: None,
            delivered_at: None,
            read_at: None,
            failed_at: None,
        }
    }
}
