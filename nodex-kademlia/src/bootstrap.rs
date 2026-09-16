use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use crate::health::{HealthTracker, NodeHealthState};
use crate::lookup::LookupEngine;
use crate::node::{Contact, NodeId, RoutingTable};
use crate::rpc::{NetworkManager, RpcPayload};

pub struct BootstrapEngine;

impl BootstrapEngine {
    /// Bootstraps to a list of known bootstrap addresses with automatic retries and exponential backoff.
    pub async fn run_bootstrap(
        network: &Arc<NetworkManager>,
        routing_table: &Arc<tokio::sync::RwLock<RoutingTable>>,
        health: &HealthTracker,
        local_id: NodeId,
        bootstrap_addrs: &[SocketAddr],
    ) -> Result<usize, String> {
        if bootstrap_addrs.is_empty() {
            health.set_state(NodeHealthState::Online);
            println!("[BOOTSTRAP] No bootstrap nodes provided; running in standalone mode.");
            return Ok(0);
        }

        health.set_state(NodeHealthState::Connecting);
        let mut connected_count = 0;

        for addr in bootstrap_addrs {
            let mut backoff = Duration::from_millis(200);
            let mut success = false;

            for attempt in 1..=3 {
                println!("[BOOTSTRAP] Pinging bootstrap node {} (attempt {})...", addr, attempt);
                match network.call(*addr, RpcPayload::Ping, Duration::from_millis(1500), 2).await {
                    Ok(reply) => {
                        println!("[BOOTSTRAP] Connected to bootstrap node {} (ID: {})", addr, reply.sender_id);
                        {
                            let mut rt = routing_table.write().await;
                            rt.update(Contact::new(reply.sender_id, *addr));
                        }
                        connected_count += 1;
                        success = true;
                        break;
                    }
                    Err(e) => {
                        println!("[BOOTSTRAP] Attempt {} failed for {}: {}", attempt, addr, e);
                        tokio::time::sleep(backoff).await;
                        backoff = (backoff * 2).min(Duration::from_secs(3));
                    }
                }
            }

            if !success {
                println!("[BOOTSTRAP] Bootstrap node {} unreachable after 3 attempts.", addr);
            }
        }

        if connected_count > 0 {
            health.set_state(NodeHealthState::SearchingPeers);
            println!("[BOOTSTRAP] Performing iterative lookup to discover neighborhood peers...");
            let discovered = LookupEngine::lookup_nodes(network, routing_table, local_id).await;
            println!("[BOOTSTRAP] Lookup finished. Discovered {} neighborhood nodes.", discovered.len());
            health.set_state(NodeHealthState::Online);
            Ok(connected_count)
        } else {
            health.set_state(NodeHealthState::Degraded);
            Err("All bootstrap nodes unreachable; operating in degraded standalone mode".into())
        }
    }
}
