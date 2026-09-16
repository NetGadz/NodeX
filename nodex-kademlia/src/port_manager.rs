use std::net::SocketAddr;
use tokio::net::UdpSocket;
use crate::network_error::NetworkError;

pub struct PortManager;

impl PortManager {
    /// Attempts to bind a UDP socket starting at `preferred_port`, automatically incrementing
    /// and falling back on `WSAEADDRINUSE` (or other bind conflicts) up to `max_attempts`.
    pub async fn bind_auto(ip: &str, preferred_port: u16, max_attempts: u16) -> Result<(UdpSocket, SocketAddr), NetworkError> {
        let mut last_err = String::new();

        for offset in 0..max_attempts {
            let port = preferred_port.saturating_add(offset);
            let addr_str = format!("{}:{}", ip, port);
            
            match UdpSocket::bind(&addr_str).await {
                Ok(socket) => {
                    let local_addr = socket.local_addr().map_err(|e| NetworkError::SocketBindFailed(e.to_string()))?;
                    println!("[PORT_MANAGER] Successfully bound UDP socket to {}", local_addr);
                    return Ok((socket, local_addr));
                }
                Err(e) => {
                    last_err = e.to_string();
                    // If WSAEADDRINUSE, continue trying next port in range
                    println!("[PORT_MANAGER] Port {} busy (WSAEADDRINUSE / {}), trying next port...", port, e);
                }
            }
        }

        // Fallback to ephemeral port 0 if all specific ports in range failed
        let fallback_addr = format!("{}:0", ip);
        match UdpSocket::bind(&fallback_addr).await {
            Ok(socket) => {
                let local_addr = socket.local_addr().map_err(|e| NetworkError::SocketBindFailed(e.to_string()))?;
                println!("[PORT_MANAGER] Bound to fallback ephemeral port {}", local_addr);
                Ok((socket, local_addr))
            }
            Err(e) => Err(NetworkError::SocketBindFailed(format!("Failed to bind port after {} attempts. Last error: {}", max_attempts, last_err.max(e.to_string())))),
        }
    }
}
