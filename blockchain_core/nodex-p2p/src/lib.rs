#![forbid(unsafe_code)]

use libp2p::{identity, PeerId};

pub struct P2pNetworkNode {
    pub peer_id: PeerId,
}

impl P2pNetworkNode {
    pub fn new() -> Self {
        let key = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from_public_key(&key.public());
        Self { peer_id }
    }
}

impl Default for P2pNetworkNode {
    fn default() -> Self {
        Self::new()
    }
}
