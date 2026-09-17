use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub const MAX_RELAY_SESSIONS: usize = 10;
pub const MAX_PACKETS_PER_SEC: u32 = 20;
pub const MAX_DAILY_TRAFFIC_BYTES: u64 = 50 * 1024 * 1024; // 50 MB
pub const MAX_RELAY_PACKET_SIZE: usize = 1200;

/// Signed request to establish an ephemeral transit relay session.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct RelaySessionRequest {
    pub session_id: [u8; 16],
    pub sender_user_id: String,
    pub recipient_user_id: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub signature_hex: String,
}

/// Encrypted envelope forwarded through the relay node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RelayPacket {
    pub session_id: [u8; 16],
    pub sender_user_id: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RelayConfig {
    pub enabled: bool,
    pub max_sessions: usize,
    pub daily_traffic_limit_bytes: u64,
}

struct PeerRateTracker {
    last_reset: Instant,
    packet_count: u32,
}

#[allow(dead_code)]
struct ActiveSession {
    sender_addr: SocketAddr,
    recipient_addr: Option<SocketAddr>,
    sender_id: String,
    recipient_id: String,
    created_at: Instant,
}

/// Opt-in Encrypted Peer Relay Service with anti-abuse protections and resource limits.
pub struct OptInRelayService {
    config: RelayConfig,
    active_sessions: Arc<RwLock<HashMap<[u8; 16], ActiveSession>>>,
    rate_limiters: Arc<RwLock<HashMap<SocketAddr, PeerRateTracker>>>,
    blacklist: Arc<RwLock<HashSet<String>>>,
    daily_bytes_relayed: Arc<AtomicU64>,
}

impl OptInRelayService {
    pub fn new(config: RelayConfig) -> Self {
        Self {
            config,
            active_sessions: Arc::new(RwLock::new(HashMap::new())),
            rate_limiters: Arc::new(RwLock::new(HashMap::new())),
            blacklist: Arc::new(RwLock::new(HashSet::new())),
            daily_bytes_relayed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Check if the relay is enabled and ready to accept sessions.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get total bytes relayed today.
    pub fn daily_traffic_used(&self) -> u64 {
        self.daily_bytes_relayed.load(Ordering::Relaxed)
    }

    /// Add a malicious user_id to the blacklist.
    pub async fn blacklist_peer(&self, user_id: String) {
        let mut bl = self.blacklist.write().await;
        bl.insert(user_id);
    }

    /// Register a new signed relay session request.
    pub async fn handle_session_request(
        &self,
        request: RelaySessionRequest,
        from_addr: SocketAddr,
        current_timestamp: u64,
    ) -> Result<bool, &'static str> {
        if !self.config.enabled {
            return Err("Relay is disabled on this node");
        }

        if request.expires_at <= current_timestamp {
            return Err("Relay session request has expired");
        }

        // Check Blacklist
        {
            let bl = self.blacklist.read().await;
            if bl.contains(&request.sender_user_id) || bl.contains(&request.recipient_user_id) {
                return Err("Peer is blacklisted");
            }
        }

        // Check Traffic limits
        if self.daily_traffic_used() >= self.config.daily_traffic_limit_bytes {
            return Err("Relay daily traffic quota reached");
        }

        let mut sessions = self.active_sessions.write().await;
        if sessions.len() >= self.config.max_sessions.min(MAX_RELAY_SESSIONS) {
            return Err("Relay session capacity full (max 10 sessions)");
        }

        sessions.insert(request.session_id, ActiveSession {
            sender_addr: from_addr,
            recipient_addr: None,
            sender_id: request.sender_user_id,
            recipient_id: request.recipient_user_id,
            created_at: Instant::now(),
        });

        Ok(true)
    }

    /// Forward an encrypted packet to the destination peer in the session.
    pub async fn forward_packet(
        &self,
        packet: RelayPacket,
        from_addr: SocketAddr,
    ) -> Result<Option<SocketAddr>, &'static str> {
        if !self.config.enabled {
            return Err("Relay disabled");
        }

        if packet.payload.len() > MAX_RELAY_PACKET_SIZE {
            return Err("Packet size exceeds 1200 bytes limit");
        }

        // Check rate limiter
        {
            let mut raters = self.rate_limiters.write().await;
            let tracker = raters.entry(from_addr).or_insert_with(|| PeerRateTracker {
                last_reset: Instant::now(),
                packet_count: 0,
            });

            if tracker.last_reset.elapsed() >= Duration::from_secs(1) {
                tracker.last_reset = Instant::now();
                tracker.packet_count = 0;
            }

            tracker.packet_count += 1;
            if tracker.packet_count > MAX_PACKETS_PER_SEC {
                return Err("Rate limit exceeded (>20 pkts/sec)");
            }
        }

        let target_addr = {
            let sessions = self.active_sessions.read().await;
            if let Some(session) = sessions.get(&packet.session_id) {
                if from_addr == session.sender_addr {
                    session.recipient_addr
                } else if Some(from_addr) == session.recipient_addr {
                    Some(session.sender_addr)
                } else {
                    return Err("Unauthorized sender for session");
                }
            } else {
                return Err("Session not found or expired");
            }
        };

        // Track traffic in RAM
        self.daily_bytes_relayed.fetch_add(packet.payload.len() as u64, Ordering::Relaxed);

        Ok(target_addr)
    }
}
