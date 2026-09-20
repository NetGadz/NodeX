use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use crate::bootstrap::BootstrapEngine;
use crate::health::{HealthTracker, NodeHealthState};
use crate::node::{NodeId, RoutingTable};
use crate::rpc::NetworkManager;

pub struct ReconnectManager;

impl ReconnectManager {
    /// Launches a background reconnection supervisor that checks peer health and restores routing
    /// table upon link recovery.
    pub fn start_reconnect_supervisor(
        network: Arc<NetworkManager>,
        routing_table: Arc<tokio::sync::RwLock<RoutingTable>>,
        health: HealthTracker,
        local_id: NodeId,
        bootstrap_addrs: Vec<SocketAddr>,
        interval_secs: u64,
    ) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;

                let peer_count = {
                    let rt = routing_table.read().await;
                    rt.total_contacts()
                };

                if peer_count == 0 && !bootstrap_addrs.is_empty() {
                    println!("[RECONNECT] Routing table empty, attempting automatic network reconnection...");
                    health.set_state(NodeHealthState::Connecting);
                    let _ = BootstrapEngine::run_bootstrap(
                        &network,
                        &routing_table,
                        &health,
                        local_id,
                        &bootstrap_addrs,
                    ).await;
                } else if peer_count > 0 {
                    health.set_state(NodeHealthState::Online);
                }
            }
        });
    }
}
