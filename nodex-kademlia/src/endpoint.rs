use std::net::SocketAddr;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EndpointType {
    /// Public IPv6 address (direct global reachability)
    Ipv6 = 4,
    /// Confirmed UPnP/NAT-PMP port forwarding
    Upnp = 3,
    /// STUN reflexive public endpoint
    Stun = 2,
    /// Local subnet LAN address
    Local = 1,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndpointCandidate {
    pub addr: SocketAddr,
    pub endpoint_type: EndpointType,
}

impl EndpointCandidate {
    pub fn new(addr: SocketAddr, endpoint_type: EndpointType) -> Self {
        Self { addr, endpoint_type }
    }
}

pub struct EndpointManager;

impl EndpointManager {
    /// Sorts candidates by connection priority (IPv6 > UPnP > STUN > Local) and removes duplicates.
    pub fn rank_candidates(mut candidates: Vec<EndpointCandidate>) -> Vec<EndpointCandidate> {
        candidates.sort_by_key(|a| std::cmp::Reverse(a.endpoint_type));
        let mut unique = Vec::new();
        for c in candidates {
            if !unique.iter().any(|u: &EndpointCandidate| u.addr == c.addr) {
                unique.push(c);
            }
        }
        unique
    }
}
