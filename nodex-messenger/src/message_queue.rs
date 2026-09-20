use std::collections::VecDeque;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use crate::db::SavedChatMessage;
use crate::message_status::MessageStatus;

pub const MAX_RETRY_ATTEMPTS: u32 = 6;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
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
        // Avoid duplicate queuing
        if q.iter().any(|m| m.message.id == msg.id) {
            return;
        }
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
        if let Some(pos) = q.iter().position(|m| m.status == MessageStatus::Sending && m.next_retry_at <= now) {
            let mut item = q.remove(pos).unwrap();
            item.attempts += 1;
            Some(item)
        } else {
            None
        }
    }

    pub async fn schedule_retry(&self, mut queued: QueuedMessage, now: u64) {
        if queued.attempts >= MAX_RETRY_ATTEMPTS {
            queued.status = MessageStatus::Failed;
            println!("[QUEUE] Message {} reached max retries ({}), marked as Failed", queued.message.id, MAX_RETRY_ATTEMPTS);
        } else {
            // Exponential backoff: 2s, 4s, 8s, 16s, 32s, 64s
            let backoff_secs = 2u64.pow(queued.attempts.min(6));
            queued.next_retry_at = now + backoff_secs;
            queued.status = MessageStatus::Sending;
            println!(
                "[QUEUE] Rescheduled message {} attempt {}/{} in {}s",
                queued.message.id, queued.attempts, MAX_RETRY_ATTEMPTS, backoff_secs
            );
        }
        let mut q = self.queue.write().await;
        q.push_back(queued);
    }

    pub async fn mark_delivered(&self, msg_id: &str) {
        let mut q = self.queue.write().await;
        if let Some(pos) = q.iter().position(|m| m.message.id == msg_id) {
            q.remove(pos);
            println!("[QUEUE] Message {} confirmed delivered, removed from Outbox", msg_id);
        }
    }

    pub async fn len(&self) -> usize {
        self.queue.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.queue.read().await.is_empty()
    }

    pub async fn get_all(&self) -> Vec<QueuedMessage> {
        self.queue.read().await.iter().cloned().collect()
    }
}

