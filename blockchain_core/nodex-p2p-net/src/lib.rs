use nodex_blockchain::{Block, Transaction};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum BlockchainP2PMessage {
    BroadcastTx(Transaction),
    BroadcastBlock(Block),
    GetBlockHeight,
    BlockHeightResponse(u64),
}

pub struct BlockchainNetworkNode {
    pub socket: UdpSocket,
}

impl BlockchainNetworkNode {
    pub async fn bind(addr: &str) -> Result<Self, String> {
        let socket = UdpSocket::bind(addr).await.map_err(|e| format!("Failed to bind P2P UDP: {}", e))?;
        Ok(Self { socket })
    }

    pub async fn broadcast_message(&self, peers: &[SocketAddr], msg: &BlockchainP2PMessage) -> Result<(), String> {
        let bytes = bincode::serialize(msg).map_err(|e| e.to_string())?;
        for peer in peers {
            let _ = self.socket.send_to(&bytes, peer).await;
        }
        Ok(())
    }
}
