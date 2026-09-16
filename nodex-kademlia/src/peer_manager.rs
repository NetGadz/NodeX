use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::node::NodeId;

#[derive(Clone, Debug)]
pub struct PeerInfo {
    pub node_id: NodeId,
    pub socket_addr: SocketAddr,
    pub last_seen: Instant,
    pub last_latency_ms: Option<u64>,
    pub failed_attempts: u32,
}

#[derive(Clone, Debug)]
pub struct PeerManager {
    peers: Arc<RwLock<HashMap<NodeId, PeerInfo>>>,
    peer_timeout: Duration,
}

impl PeerManager {
    pub fn new(peer_timeout_secs: u64) -> Self {
        Self {
            peers: Arc::new(RwLock::new(HashMap::new())),
            peer_timeout: Duration::from_secs(peer_timeout_secs),
        }
    }

    pub async fn record_success(&self, node_id: NodeId, socket_addr: SocketAddr, latency_ms: Option<u64>) {
        let mut peers = self.peers.write().await;
        peers.insert(node_id, PeerInfo {
            node_id,
            socket_addr,
            last_seen: Instant::now(),
            last_latency_ms: latency_ms,
            failed_attempts: 0,
        });
    }

    pub async fn record_failure(&self, node_id: &NodeId) {
        let mut peers = self.peers.write().await;
        if let Some(peer) = peers.get_mut(node_id) {
            peer.failed_attempts = peer.failed_attempts.saturating_add(1);
        }
    }

    pub async fn get_peer_addr(&self, node_id: &NodeId) -> Option<SocketAddr> {
        let peers = self.peers.read().await;
        peers.get(node_id).map(|p| p.socket_addr)
    }

    pub async fn get_active_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        let now = Instant::now();
        peers
            .values()
            .filter(|p| now.duration_since(p.last_seen) < self.peer_timeout)
            .cloned()
            .collect()
    }

    pub async fn cleanup_stale(&self) -> usize {
        let mut peers = self.peers.write().await;
        let now = Instant::now();
        let before = peers.len();
        peers.retain(|_, p| now.duration_since(p.last_seen) < self.peer_timeout && p.failed_attempts < 5);
        before.saturating_sub(peers.len())
    }

    pub async fn count(&self) -> usize {
        self.peers.read().await.len()
    }
}

impl Default for PeerManager {
    fn default() -> Self {
        Self::new(300) // 5 minutes
    }
}
