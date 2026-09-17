use serde::{Deserialize, Serialize};

/// Practical NAT classification categories for real-world P2P connectivity.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum NatCategory {
    /// Open Internet, public IP, or confirmed port forwarding via UPnP/NAT-PMP.
    DirectPossible,
    /// Standard Cone NAT; high probability of successful UDP Hole Punching.
    HolePunchLikely,
    /// Symmetric NAT, strict ISP CGNAT, or restrictive corporate firewall; Relay recommended.
    RelayRecommended,
    /// Inbound and outbound UDP traffic is blocked or unreachable.
    UdpBlocked,
    /// NAT behavior has not yet been determined (initial probing state).
    #[default]
    Unknown,
}

impl NatCategory {
    pub fn description(&self) -> &'static str {
        match self {
            NatCategory::DirectPossible => "Direct P2P Available (Public IP / UPnP)",
            NatCategory::HolePunchLikely => "Cone NAT (UDP Hole Punching likely)",
            NatCategory::RelayRecommended => "Complex NAT (Opt-in Relay recommended)",
            NatCategory::UdpBlocked => "UDP Blocked / Firewall restricted",
            NatCategory::Unknown => "Assessing network...",
        }
    }

    /// Whether this node can accept direct incoming UDP packets from peers.
    pub fn can_accept_direct(&self) -> bool {
        matches!(self, NatCategory::DirectPossible)
    }

    /// Whether this node should initiate UDP hole punching when connecting.
    pub fn should_hole_punch(&self) -> bool {
        matches!(self, NatCategory::HolePunchLikely | NatCategory::DirectPossible)
    }
}
