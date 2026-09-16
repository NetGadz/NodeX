use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::db::SavedChatMessage;
use crate::message_status::MessageStatus;

#[derive(Clone, Debug)]
pub struct QueuedMessage {
    pub message: SavedChatMessage,
    pub recipient_id: String,
    pub attempts: u32,
    pub status: MessageStatus,
    pub next_retry_at: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MessageQueue {
    queue: Arc<RwLock<VecDeque<QueuedMessage>>>,
}

impl MessageQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn push(&self, msg: SavedChatMessage, recipient_id: String) {
        let mut q = self.queue.write().await;
        q.push_back(QueuedMessage {
            message: msg,
            recipient_id,
            attempts: 0,
            status: MessageStatus::Sending,
            next_retry_at: 0,
        });
    }

    pub async fn pop_pending(&self, now: u64) -> Option<QueuedMessage> {
        let mut q = self.queue.write().await;
        if let Some(pos) = q.iter().position(|m| m.next_retry_at <= now) {
            q.remove(pos)
        } else {
            None
        }
    }

    pub async fn len(&self) -> usize {
        self.queue.read().await.len()
    }
}
