use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

use crate::lookup::{LookupEngine, RPC_RETRIES, RPC_TIMEOUT};
use crate::node::{NodeId, RoutingTable};
use crate::rpc::{NetworkManager, RpcPayload};

pub const DEFAULT_TTL: Duration = Duration::from_secs(24 * 3600); // 24 hours
pub const REPUBLISH_INTERVAL: Duration = Duration::from_secs(3600); // 1 hour
const MAX_STORAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_STORAGE_RECORDS: usize = 50_000;

#[derive(Clone, Debug)]
pub struct Record {
    pub value: Vec<u8>,
    pub created_at: Instant,
    pub ttl: Duration,
    pub republish_at: Instant,
}

impl Record {
    pub fn new(value: Vec<u8>, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            ttl,
            republish_at: now + REPUBLISH_INTERVAL,
        }
    }

    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

#[derive(Default, Debug)]
pub struct Storage {
    data: HashMap<NodeId, Record>,
    stored_bytes: usize,
}

impl Storage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&mut self, key: NodeId, value: Vec<u8>, ttl: Duration) {
        if value.len() > MAX_STORAGE_BYTES {
            eprintln!("[STORE] Refusing oversized record ({} bytes)", value.len());
            return;
        }
        if let Some(previous) = self.data.remove(&key) {
            self.stored_bytes = self.stored_bytes.saturating_sub(previous.value.len());
        }
        while self.stored_bytes.saturating_add(value.len()) > MAX_STORAGE_BYTES
            || self.data.len() >= MAX_STORAGE_RECORDS
        {
            let oldest_key = self.data.iter()
                .min_by_key(|(_, record)| record.created_at)
                .map(|(key, _)| *key);
            let Some(oldest_key) = oldest_key else { break };
            if let Some(expired) = self.data.remove(&oldest_key) {
                self.stored_bytes = self.stored_bytes.saturating_sub(expired.value.len());
            }
        }
        println!("[STORE] Storing key {} ({} bytes, TTL={:?})", key, value.len(), ttl);
        self.stored_bytes = self.stored_bytes.saturating_add(value.len());
        self.data.insert(key, Record::new(value, ttl));
    }

    pub fn get(&self, key: &NodeId) -> Option<Vec<u8>> {
        if let Some(record) = self.data.get(key) {
            if !record.is_expired() {
                return Some(record.value.clone());
            }
        }
        None
    }

    pub fn remove(&mut self, key: &NodeId) -> Option<Record> {
        let removed = self.data.remove(key);
        if let Some(record) = &removed {
            self.stored_bytes = self.stored_bytes.saturating_sub(record.value.len());
        }
        removed
    }

    pub fn clear(&mut self) {
        self.data.clear();
        self.stored_bytes = 0;
    }

    pub fn cleanup_stale(&mut self) -> usize {
        let before = self.data.len();
        self.data.retain(|_, record| !record.is_expired());
        let removed = before - self.data.len();
        self.stored_bytes = self.data.values().map(|record| record.value.len()).sum();
        if removed > 0 {
            println!("[STORE] Cleaned up {} expired records", removed);
        }
        removed
    }

    pub fn get_keys_to_republish(&mut self) -> Vec<(NodeId, Vec<u8>)> {
        let now = Instant::now();
        let mut to_republish = Vec::new();
        for (key, record) in self.data.iter_mut() {
            if !record.is_expired() && now >= record.republish_at {
                record.republish_at = now + REPUBLISH_INTERVAL;
                to_republish.push((*key, record.value.clone()));
            }
        }
        to_republish
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn list_keys(&self) -> Vec<NodeId> {
        self.data.keys().cloned().collect()
    }
}

pub fn start_storage_maintenance_task(
    storage: Arc<RwLock<Storage>>,
    network: Arc<NetworkManager>,
    routing_table: Arc<RwLock<RoutingTable>>,
    check_interval: Duration,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(check_interval);
        loop {
            interval.tick().await;

            // 1. Clean up stale records
            {
                let mut st = storage.write().await;
                st.cleanup_stale();
            }

            // 2. Republish records to nearest k nodes
            let to_republish = {
                let mut st = storage.write().await;
                st.get_keys_to_republish()
            };

            for (key, value) in to_republish {
                println!("[STORE] Republishing key {} to nearest nodes...", key);
                let closest_nodes = LookupEngine::lookup_nodes(&network, &routing_table, key).await;
                for contact in closest_nodes {
                    let net = Arc::clone(&network);
                    let k = key;
                    let v = value.clone();
                    tokio::spawn(async move {
                        let _ = net
                            .call(
                                contact.addr,
                                RpcPayload::Store { key: k, value: v },
                                RPC_TIMEOUT,
                                RPC_RETRIES,
                            )
                            .await;
                    });
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_put_and_get() {
        let mut st = Storage::new();
        let key = NodeId::from_key(b"test_key");
        st.put(key, b"test_val".to_vec(), Duration::from_secs(60));
        assert_eq!(st.get(&key), Some(b"test_val".to_vec()));
    }

    #[test]
    fn storage_expiration_cleanup() {
        let mut st = Storage::new();
        let key = NodeId::from_key(b"expiring_key");
        st.put(key, b"temporary".to_vec(), Duration::from_millis(1));
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(st.get(&key), None);
        let removed = st.cleanup_stale();
        assert_eq!(removed, 1);
        assert_eq!(st.len(), 0);
    }
}
