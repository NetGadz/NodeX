use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum NodeHealthState {
    Starting = 0,
    Binding = 1,
    Connecting = 2,
    SearchingPeers = 3,
    Online = 4,
    Degraded = 5,
    Offline = 6,
    Stopping = 7,
}

impl NodeHealthState {
    pub fn as_str(&self) -> &'static str {
        match self {
            NodeHealthState::Starting => "Starting",
            NodeHealthState::Binding => "Binding",
            NodeHealthState::Connecting => "Connecting",
            NodeHealthState::SearchingPeers => "SearchingPeers",
            NodeHealthState::Online => "Online",
            NodeHealthState::Degraded => "Degraded",
            NodeHealthState::Offline => "Offline",
            NodeHealthState::Stopping => "Stopping",
        }
    }

    pub fn from_u8(val: u8) -> Self {
        match val {
            0 => NodeHealthState::Starting,
            1 => NodeHealthState::Binding,
            2 => NodeHealthState::Connecting,
            3 => NodeHealthState::SearchingPeers,
            4 => NodeHealthState::Online,
            5 => NodeHealthState::Degraded,
            6 => NodeHealthState::Offline,
            _ => NodeHealthState::Stopping,
        }
    }
}

#[derive(Clone, Debug)]
pub struct HealthTracker {
    state: Arc<AtomicU8>,
}

impl HealthTracker {
    pub fn new(initial: NodeHealthState) -> Self {
        Self {
            state: Arc::new(AtomicU8::new(initial as u8)),
        }
    }

    pub fn set_state(&self, new_state: NodeHealthState) {
        self.state.store(new_state as u8, Ordering::SeqCst);
    }

    pub fn get_state(&self) -> NodeHealthState {
        NodeHealthState::from_u8(self.state.load(Ordering::SeqCst))
    }

    pub fn is_online(&self) -> bool {
        matches!(self.get_state(), NodeHealthState::Online | NodeHealthState::Degraded)
    }
}

impl Default for HealthTracker {
    fn default() -> Self {
        Self::new(NodeHealthState::Starting)
    }
}
