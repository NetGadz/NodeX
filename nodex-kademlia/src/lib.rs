pub mod bootstrap;
pub mod chunking;
pub mod config;
pub mod endpoint;
pub mod health;
pub mod hole_punch;
pub mod lookup;
pub mod metrics;
pub mod nat_type;
pub mod network_error;
pub mod node;
pub mod peer_manager;
pub mod port_manager;
pub mod reconnect;
pub mod relay_client;
pub mod rpc;
pub mod state;
pub mod storage;
pub mod stun;
pub mod upnp;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::RwLock;

use crate::config::NodeConfig;
use crate::health::HealthTracker;
use crate::lookup::{LookupEngine, RPC_RETRIES, RPC_TIMEOUT};
use crate::metrics::MetricsTracker;
use crate::node::{Contact, NodeId, RoutingTable, K};
use crate::peer_manager::PeerManager;
use crate::rpc::{NetworkManager, RpcMessage, RpcPayload};
use crate::state::StateManager;
use crate::storage::{start_storage_maintenance_task, Storage, DEFAULT_TTL};

#[derive(Clone)]
pub struct KademliaNode {
    pub node_id: NodeId,
    pub network: Arc<NetworkManager>,
    pub routing_table: Arc<RwLock<RoutingTable>>,
    pub storage: Arc<RwLock<Storage>>,
    pub health: HealthTracker,
    pub peer_manager: PeerManager,
    pub config: NodeConfig,
}

impl KademliaNode {
    pub async fn start(bind_addr: SocketAddr) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let cfg = NodeConfig {
            ip: bind_addr.ip().to_string(),
            port: bind_addr.port(),
            ..Default::default()
        };
        Self::start_with_config(cfg).await
    }

    pub async fn start_with_config(config: NodeConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let bind_addr: SocketAddr = format!("{}:{}", config.ip, config.port).parse()?;
        let (node_id, restored_contacts) = Self::load_or_create_identity(&config.state_file);
        let network = NetworkManager::bind(bind_addr, node_id).await?;
        Self::init_node(config, network, node_id, restored_contacts).await
    }

    pub async fn start_auto(mut config: NodeConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let (node_id, restored_contacts) = Self::load_or_create_identity(&config.state_file);
        let network = NetworkManager::bind_auto(&config.ip, config.port, 50, node_id).await?;
        config.port = network.local_addr.port();
        Self::init_node(config, network, node_id, restored_contacts).await
    }

    fn load_or_create_identity(state_file: &str) -> (NodeId, Vec<Contact>) {
        if std::path::Path::new(state_file).exists() {
            match StateManager::load_state(state_file) {
                Ok((id, contacts)) => {
                    println!("[STATE] Restored persistent Node ID {} and {} saved contacts from {}", id, contacts.len(), state_file);
                    (id, contacts)
                }
                Err(e) => {
                    println!("[STATE] Failed to load state from {}: {}. Generating new random Node ID.", state_file, e);
                    (NodeId::generate_random(), Vec::new())
                }
            }
        } else {
            (NodeId::generate_random(), Vec::new())
        }
    }

    async fn init_node(
        config: NodeConfig,
        network: Arc<NetworkManager>,
        node_id: NodeId,
        restored_contacts: Vec<Contact>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        println!("[NODE] Initializing node ID: {}", node_id);

        let routing_table = Arc::new(RwLock::new(RoutingTable::new(node_id)));
        let storage = Arc::new(RwLock::new(Storage::new()));
        let health = HealthTracker::new(crate::health::NodeHealthState::Online);
        let peer_manager = PeerManager::new(300);

        let node = Self {
            node_id,
            network: Arc::clone(&network),
            routing_table: Arc::clone(&routing_table),
            storage: Arc::clone(&storage),
            health,
            peer_manager,
            config: config.clone(),
        };

        // Populate routing table with restored contacts
        if !restored_contacts.is_empty() {
            let mut rt = routing_table.write().await;
            for c in restored_contacts {
                rt.update(c);
            }
        }

        // Start background UDP receive loop with RPC request dispatcher
        let rt_for_rx = Arc::clone(&routing_table);
        let st_for_rx = Arc::clone(&storage);
        let local_id = node_id;

        network.start_receive_loop(rt_for_rx.clone(), move |msg, _src| {
            let rt = Arc::clone(&rt_for_rx);
            let st = Arc::clone(&st_for_rx);
            async move {
                match msg.payload {
                    RpcPayload::Ping => {
                        println!("[RPC] Received PING from {}", msg.sender_id);
                        Some(RpcMessage::new(msg.request_id, local_id, RpcPayload::Pong))
                    }
                    RpcPayload::Store { key, value } => {
                        println!(
                            "[RPC] Received STORE for key {} ({} bytes) from {}",
                            key,
                            value.len(),
                            msg.sender_id
                        );
                        {
                            let mut s = st.write().await;
                            s.put(key, value, DEFAULT_TTL);
                        }
                        Some(RpcMessage::new(
                            msg.request_id,
                            local_id,
                            RpcPayload::StoreAck { ok: true },
                        ))
                    }
                    RpcPayload::FindNode { target_id } => {
                        println!(
                            "[RPC] Received FIND_NODE for target {} from {}",
                            target_id, msg.sender_id
                        );
                        let closest = {
                            let r = rt.read().await;
                            r.find_closest(&target_id, K)
                        };
                        Some(RpcMessage::new(
                            msg.request_id,
                            local_id,
                            RpcPayload::FindNodeResp { contacts: closest },
                        ))
                    }
                    RpcPayload::FindValue { key } => {
                        println!(
                            "[RPC] Received FIND_VALUE for key {} from {}",
                            key, msg.sender_id
                        );
                        let value_opt = {
                            let s = st.read().await;
                            s.get(&key)
                        };
                        if let Some(val) = value_opt {
                            Some(RpcMessage::new(
                                msg.request_id,
                                local_id,
                                RpcPayload::FindValueResp {
                                    value: Some(val),
                                    contacts: Vec::new(),
                                },
                            ))
                        } else {
                            let closest = {
                                let r = rt.read().await;
                                r.find_closest(&key, K)
                            };
                            Some(RpcMessage::new(
                                msg.request_id,
                                local_id,
                                RpcPayload::FindValueResp {
                                    value: None,
                                    contacts: closest,
                                },
                            ))
                        }
                    }
                    _ => None,
                }
            }
        });

        // Start background storage maintenance (TTL cleanup & republish)
        start_storage_maintenance_task(
            Arc::clone(&storage),
            Arc::clone(&network),
            Arc::clone(&routing_table),
            Duration::from_secs(config.republish_secs),
        );

        Ok(node)
    }

    pub async fn bootstrap(&self, bootstrap_addr: SocketAddr) -> Result<(), String> {
        println!("[BOOTSTRAP] Connecting to bootstrap node at {}...", bootstrap_addr);

        let reply = self
            .network
            .call(bootstrap_addr, RpcPayload::Ping, RPC_TIMEOUT, RPC_RETRIES)
            .await?;

        println!(
            "[BOOTSTRAP] Received PONG from bootstrap node {} ({})",
            reply.sender_id, bootstrap_addr
        );

        {
            let mut rt = self.routing_table.write().await;
            rt.update(Contact::new(reply.sender_id, bootstrap_addr));
        }

        println!("[BOOTSTRAP] Performing iterative FIND_NODE(self) to discover network...");
        let discovered = LookupEngine::lookup_nodes(&self.network, &self.routing_table, self.node_id).await;

        let total = {
            let rt = self.routing_table.read().await;
            rt.total_contacts()
        };

        println!(
            "[BOOTSTRAP] Bootstrap complete! Discovered {} nodes; total routing table contacts: {}",
            discovered.len(),
            total
        );

        Ok(())
    }

    pub async fn put(&self, key_str: &str, value: Vec<u8>) -> Result<usize, String> {
        let key_id = NodeId::from_key(key_str.as_bytes());
        println!(
            "[STORE] Putting key '{}' -> hash {} ({} bytes)...",
            key_str,
            key_id,
            value.len()
        );

        {
            let mut st = self.storage.write().await;
            st.put(key_id, value.clone(), Duration::from_secs(self.config.ttl_secs));
        }

        let closest = LookupEngine::lookup_nodes(&self.network, &self.routing_table, key_id).await;
        if closest.is_empty() {
            println!("[STORE] No other nodes known; saved only locally.");
            return Ok(1);
        }

        let mut stored_count = 1;
        for contact in closest {
            let net = Arc::clone(&self.network);
            let val = value.clone();
            match net
                .call(
                    contact.addr,
                    RpcPayload::Store { key: key_id, value: val },
                    RPC_TIMEOUT,
                    RPC_RETRIES,
                )
                .await
            {
                Ok(reply) => {
                    if let RpcPayload::StoreAck { ok: true } = reply.payload {
                        println!("[STORE] Successfully replicated to node {}", contact.addr);
                        stored_count += 1;
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] Failed to STORE on node {}: {}", contact.addr, e);
                }
            }
        }

        println!("[STORE] Replicated to {} node(s)", stored_count);
        Ok(stored_count)
    }

    pub async fn get(&self, key_str: &str) -> Result<Option<(Vec<u8>, Option<SocketAddr>)>, String> {
        let key_id = NodeId::from_key(key_str.as_bytes());
        println!("[LOOKUP] Searching for key '{}' -> hash {}...", key_str, key_id);

        {
            let st = self.storage.read().await;
            if let Some(val) = st.get(&key_id) {
                println!("[LOOKUP] Key found locally in storage!");
                return Ok(Some((val, Some(self.network.local_addr))));
            }
        }

        match LookupEngine::lookup_value(&self.network, &self.routing_table, key_id).await {
            lookup::LookupValueResult::Found { value, from } => {
                println!("[LOOKUP] Found value from node {}!", from.addr);
                {
                    let mut st = self.storage.write().await;
                    st.put(key_id, value.clone(), Duration::from_secs(self.config.ttl_secs));
                }
                Ok(Some((value, Some(from.addr))))
            }
            lookup::LookupValueResult::ClosestNodes(_) => {
                println!("[LOOKUP] Key '{}' not found in the DHT.", key_str);
                Ok(None)
            }
        }
    }

    pub async fn get_silent(&self, key_str: &str) -> Result<Option<(Vec<u8>, Option<SocketAddr>)>, String> {
        let key_id = NodeId::from_key(key_str.as_bytes());

        {
            let st = self.storage.read().await;
            if let Some(val) = st.get(&key_id) {
                return Ok(Some((val, Some(self.network.local_addr))));
            }
        }

        match LookupEngine::lookup_value(&self.network, &self.routing_table, key_id).await {
            lookup::LookupValueResult::Found { value, from } => {
                {
                    let mut st = self.storage.write().await;
                    st.put(key_id, value.clone(), Duration::from_secs(self.config.ttl_secs));
                }
                Ok(Some((value, Some(from.addr))))
            }
            lookup::LookupValueResult::ClosestNodes(_) => Ok(None),
        }
    }

    pub async fn list_nodes(&self) {
        let rt = self.routing_table.read().await;
        let contacts = rt.all_contacts();
        println!("\n=== Routing Table ({} contacts) ===", contacts.len());
        if contacts.is_empty() {
            println!("  (No contacts known yet)");
        } else {
            for (i, c) in contacts.iter().enumerate() {
                println!(
                    "  [{:02}] ID: {} | Addr: {} | Elapsed: {:.1}s",
                    i + 1,
                    c.id,
                    c.addr,
                    c.last_seen.elapsed().as_secs_f32()
                );
            }
        }
        println!("====================================\n");
    }

    pub async fn ping_addr(&self, addr: SocketAddr) -> Result<(), String> {
        println!("[RPC] Pinging {}...", addr);
        match self
            .network
            .call(addr, RpcPayload::Ping, RPC_TIMEOUT, RPC_RETRIES)
            .await
        {
            Ok(reply) => {
                println!("[RPC] Pong from {} (Node ID: {})", addr, reply.sender_id);
                Ok(())
            }
            Err(e) => {
                eprintln!("[ERROR] Ping failed to {}: {}", addr, e);
                Err(e)
            }
        }
    }

    pub async fn save_state(&self) -> Result<(), String> {
        let contacts = {
            let rt = self.routing_table.read().await;
            rt.all_contacts()
        };
        StateManager::save_state(self.node_id, &contacts, &self.config.state_file)?;
        println!("[STATE] Node state saved to {}", self.config.state_file);
        Ok(())
    }

    pub fn metrics(&self) -> Arc<MetricsTracker> {
        Arc::clone(&self.network.metrics)
    }
}
