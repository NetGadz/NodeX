use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlindMeshAnnouncement {
    pub target_id_hex: String,
    pub target_display_name: String,
    pub target_x25519_pub_hex: String,
    pub relay_id_hex: String,
    pub relay_addr: SocketAddr,
    pub hops: u8,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeshGossipPayload {
    pub sender_id_hex: String,
    pub announcements: Vec<BlindMeshAnnouncement>,
    pub timestamp: u64,
}

#[derive(Clone, Debug, Default)]
pub struct MeshDiscoveryTable {
    // Map: target_id_hex -> list of candidate routes
    routes: HashMap<String, Vec<BlindMeshAnnouncement>>,
}

impl MeshDiscoveryTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert_announcements(&mut self, announcements: Vec<BlindMeshAnnouncement>, max_hops: u8) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for ann in announcements {
            if ann.hops > max_hops || ann.expires_at <= now {
                continue;
            }

            let entry = self.routes.entry(ann.target_id_hex.clone()).or_default();
            // Remove older announcements from same relay
            entry.retain(|r| r.relay_id_hex != ann.relay_id_hex && r.expires_at > now);
            entry.push(ann);
        }
    }

    pub fn find_best_route(&self, target_id_hex: &str) -> Option<BlindMeshAnnouncement> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(list) = self.routes.get(target_id_hex) {
            list.iter()
                .filter(|r| r.expires_at > now)
                .min_by_key(|r| r.hops)
                .cloned()
        } else {
            None
        }
    }

    pub fn cleanup_expired(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.routes.retain(|_, list| {
            list.retain(|r| r.expires_at > now);
            !list.is_empty()
        });
    }

    pub fn all_mesh_peers(&self) -> Vec<BlindMeshAnnouncement> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut res = Vec::new();
        for list in self.routes.values() {
            for r in list {
                if r.expires_at > now {
                    res.push(r.clone());
                }
            }
        }
        res
    }
}

pub struct MeshManager {
    pub table: Arc<RwLock<MeshDiscoveryTable>>,
}

impl MeshManager {
    pub fn new() -> Self {
        Self {
            table: Arc::new(RwLock::new(MeshDiscoveryTable::new())),
        }
    }

    pub async fn add_gossip(&self, payload: MeshGossipPayload) {
        let mut t = self.table.write().await;
        t.insert_announcements(payload.announcements, 3);
    }

    pub async fn resolve_route(&self, target_id_hex: &str) -> Option<BlindMeshAnnouncement> {
        let t = self.table.read().await;
        t.find_best_route(target_id_hex)
    }
}
