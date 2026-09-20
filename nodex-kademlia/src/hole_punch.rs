use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::UdpSocket;
use rand::RngCore;

pub struct HolePuncher;

impl HolePuncher {
    pub async fn respond_to_probe(socket: &UdpSocket, packet: &[u8], from: SocketAddr) -> bool {
        if packet.len() != 24 || &packet[..8] != b"NODEXHP1" {
            return false;
        }
        let mut response = b"NODEXACK".to_vec();
        response.extend_from_slice(&packet[8..]);
        socket.send_to(&response, from).await.is_ok()
    }

    pub async fn punch_candidates_verified(
        socket: &UdpSocket,
        candidates: &[SocketAddr],
    ) -> Vec<SocketAddr> {
        let mut nonce = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut nonce);
        let mut probe = b"NODEXHP1".to_vec();
        probe.extend_from_slice(&nonce);
        let mut verified = Vec::new();

        for &addr in candidates {
            for _ in 0..3 {
                let _ = socket.send_to(&probe, addr).await;
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        let mut response = [0u8; 64];
        while let Ok(Ok((len, from))) = tokio::time::timeout(
            Duration::from_millis(250),
            socket.recv_from(&mut response),
        ).await {
            if len == 24 && &response[..8] == b"NODEXACK" && response[8..24] == nonce
                && candidates.contains(&from) && !verified.contains(&from) {
                verified.push(from);
            }
        }
        verified
    }

    /// Sends synchronized burst UDP probe packets to candidate endpoints to punch symmetric/cone NAT pinholes.
    pub async fn punch_candidates(
        socket: &UdpSocket,
        candidates: &[SocketAddr],
        probe_payload: &[u8],
    ) -> Vec<SocketAddr> {
        let mut successfully_sent = Vec::new();

        for &addr in candidates {
            // Send a rapid burst of 3 probe packets with 10ms intervals
            for _ in 0..3 {
                if socket.send_to(probe_payload, addr).await.is_ok()
                    && !successfully_sent.contains(&addr)
                {
                    successfully_sent.push(addr);
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        successfully_sent
    }
}
