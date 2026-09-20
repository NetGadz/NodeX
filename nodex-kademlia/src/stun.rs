use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;
use tokio::net::UdpSocket;

pub const DEFAULT_STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
    "stun.cloudflare.com:3478",
];

const STUN_MAGIC_COOKIE: u32 = 0x2112A442;
const BINDING_REQUEST: u16 = 0x0001;
const BINDING_RESPONSE: u16 = 0x0101;
const ATTR_MAPPED_ADDRESS: u16 = 0x0001;
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;

pub struct StunClient;

impl StunClient {
    /// Builds an RFC 5389 STUN Binding Request packet (20 bytes).
    pub fn build_binding_request(transaction_id: [u8; 12]) -> [u8; 20] {
        let mut buf = [0u8; 20];
        // Message Type: Binding Request (0x0001)
        buf[0] = (BINDING_REQUEST >> 8) as u8;
        buf[1] = (BINDING_REQUEST & 0xFF) as u8;
        // Message Length: 0 (no attributes)
        buf[2] = 0x00;
        buf[3] = 0x00;
        // Magic Cookie (0x2112A442)
        buf[4..8].copy_from_slice(&STUN_MAGIC_COOKIE.to_be_bytes());
        // Transaction ID (12 bytes)
        buf[8..20].copy_from_slice(&transaction_id);
        buf
    }

    /// Parses an RFC 5389 / RFC 3489 STUN Binding Response packet and returns the reflexive public endpoint.
    pub fn parse_binding_response(buf: &[u8], transaction_id: &[u8; 12]) -> Option<SocketAddr> {
        if buf.len() < 20 {
            return None;
        }

        let msg_type = u16::from_be_bytes([buf[0], buf[1]]);
        if msg_type != BINDING_RESPONSE {
            return None;
        }

        let msg_len = u16::from_be_bytes([buf[2], buf[3]]) as usize;
        let magic = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
        let resp_tid = &buf[8..20];

        if magic != STUN_MAGIC_COOKIE || resp_tid != transaction_id {
            return None;
        }

        let mut offset = 20;
        let end = (20 + msg_len).min(buf.len());

        while offset + 4 <= end {
            let attr_type = u16::from_be_bytes([buf[offset], buf[offset + 1]]);
            let attr_len = u16::from_be_bytes([buf[offset + 2], buf[offset + 3]]) as usize;
            offset += 4;

            if offset + attr_len > end {
                break;
            }

            let attr_val = &buf[offset..offset + attr_len];

            if attr_type == ATTR_XOR_MAPPED_ADDRESS && attr_len >= 8 {
                let family = attr_val[1];
                if family == 0x01 {
                    // IPv4
                    let xor_port = u16::from_be_bytes([attr_val[2], attr_val[3]]);
                    let port = xor_port ^ ((STUN_MAGIC_COOKIE >> 16) as u16);

                    let xor_ip = u32::from_be_bytes([attr_val[4], attr_val[5], attr_val[6], attr_val[7]]);
                    let ip_u32 = xor_ip ^ STUN_MAGIC_COOKIE;
                    let ip = Ipv4Addr::from(ip_u32);

                    return Some(SocketAddr::V4(SocketAddrV4::new(ip, port)));
                }
            } else if attr_type == ATTR_MAPPED_ADDRESS && attr_len >= 8 {
                let family = attr_val[1];
                if family == 0x01 {
                    // IPv4
                    let port = u16::from_be_bytes([attr_val[2], attr_val[3]]);
                    let ip = Ipv4Addr::new(attr_val[4], attr_val[5], attr_val[6], attr_val[7]);
                    return Some(SocketAddr::V4(SocketAddrV4::new(ip, port)));
                }
            }

            // Attributes are padded to 4-byte boundaries
            let padding = (4 - (attr_len % 4)) % 4;
            offset += attr_len + padding;
        }

        None
    }

    /// Query STUN servers using an existing local UDP socket to discover the reflexive public IP:Port.
    pub async fn query_reflexive_endpoint(socket: &UdpSocket) -> Option<SocketAddr> {
        let mut tid = [0u8; 12];
        for b in &mut tid {
            *b = rand::random();
        }

        let request = Self::build_binding_request(tid);

        for &server_str in DEFAULT_STUN_SERVERS {
            if let Ok(server_addr) = tokio::net::lookup_host(server_str).await {
                for addr in server_addr {
                    if socket.send_to(&request, addr).await.is_ok() {
                        let mut buf = [0u8; 512];
                        let result = tokio::time::timeout(
                            Duration::from_millis(800),
                            socket.recv_from(&mut buf),
                        )
                        .await;

                        if let Ok(Ok((len, from_addr))) = result {
                            if from_addr == addr {
                                if let Some(reflexive) = Self::parse_binding_response(&buf[..len], &tid) {
                                    return Some(reflexive);
                                }
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Creates a temporary socket and queries public STUN servers for current reflexive address.
    pub async fn discover_public_addr() -> Option<SocketAddr> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.ok()?;
        Self::query_reflexive_endpoint(&socket).await
    }
}
