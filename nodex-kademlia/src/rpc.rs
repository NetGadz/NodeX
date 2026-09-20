use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::sync::{oneshot, Mutex, RwLock};

use core_ffi::*;

use crate::chunking::{ChunkAssembler, UdpChunk};
use crate::metrics::MetricsTracker;
use crate::node::{Contact, NodeId, RoutingTable, UpdateResult};

/// Messages larger than this threshold (in encoded bytes) are automatically
/// fragmented into UDP chunks to prevent IP-level fragmentation on WAN.
/// Standard Internet MTU is 1500; after IP (20) + UDP (8) headers = 1472 usable.
/// We use 1100 as a conservative threshold to allow for serialization overhead.
const CHUNK_THRESHOLD: usize = 1100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RpcPayload {
    Ping,
    Pong,
    Store {
        key: NodeId,
        value: Vec<u8>,
    },
    StoreAck {
        ok: bool,
    },
    FindNode {
        target_id: NodeId,
    },
    FindNodeResp {
        contacts: Vec<Contact>,
    },
    FindValue {
        key: NodeId,
    },
    FindValueResp {
        value: Option<Vec<u8>>,
        contacts: Vec<Contact>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RpcMessage {
    pub request_id: u64,
    pub sender_id: NodeId,
    pub payload: RpcPayload,
}

impl RpcMessage {
    pub fn new(request_id: u64, sender_id: NodeId, payload: RpcPayload) -> Self {
        Self {
            request_id,
            sender_id,
            payload,
        }
    }

    pub fn to_c_message(&self) -> Result<Box<CMessage>, String> {
        let mut c_msg = Box::new(CMessage {
            msg_type: 0,
            request_id: self.request_id,
            sender_id: *self.sender_id.as_bytes(),
            payload: CMessagePayload::None,
        });

        match &self.payload {
            RpcPayload::Ping => {
                c_msg.msg_type = MSG_PING;
                c_msg.payload = CMessagePayload::None;
            }
            RpcPayload::Pong => {
                c_msg.msg_type = MSG_PONG;
                c_msg.payload = CMessagePayload::None;
            }
            RpcPayload::Store { key, value } => {
                if value.len() > MAX_VALUE_SIZE {
                    return Err(format!("Value too large: {} > {}", value.len(), MAX_VALUE_SIZE));
                }
                c_msg.msg_type = MSG_STORE;
                c_msg.payload = CMessagePayload::Store(CStorePayload {
                    key: *key.as_bytes(),
                    value_len: value.len() as u32,
                    value: value.clone(),
                });
            }
            RpcPayload::StoreAck { ok } => {
                c_msg.msg_type = MSG_STORE_ACK;
                c_msg.payload = CMessagePayload::StoreAck(CStoreAckPayload {
                    status: if *ok { 0 } else { 1 },
                });
            }
            RpcPayload::FindNode { target_id } => {
                c_msg.msg_type = MSG_FIND_NODE;
                c_msg.payload = CMessagePayload::FindNode(CFindNodePayload {
                    target_id: *target_id.as_bytes(),
                });
            }
            RpcPayload::FindNodeResp { contacts } => {
                c_msg.msg_type = MSG_FIND_NODE_RESP;
                let count = contacts.len().min(MAX_CONTACTS);
                let mut resp = CFindNodeRespPayload {
                    count: count as u8,
                    contacts: [CSerializedContact::default(); MAX_CONTACTS],
                };
                for (i, c) in contacts.iter().take(count).enumerate() {
                    resp.contacts[i] = contact_to_c(c);
                }
                c_msg.payload = CMessagePayload::FindNodeResp(resp);
            }
            RpcPayload::FindValue { key } => {
                c_msg.msg_type = MSG_FIND_VALUE;
                c_msg.payload = CMessagePayload::FindValue(CFindValuePayload {
                    key: *key.as_bytes(),
                });
            }
            RpcPayload::FindValueResp { value, contacts } => {
                c_msg.msg_type = MSG_FIND_VALUE_RESP;
                if let Some(val) = value {
                    if val.len() > MAX_VALUE_SIZE {
                        return Err(format!("Value too large: {} > {}", val.len(), MAX_VALUE_SIZE));
                    }
                    c_msg.payload = CMessagePayload::FindValueResp(CFindValueRespPayload {
                        has_value: 1,
                        value_len: val.len() as u32,
                        value: val.clone(),
                        count: 0,
                        contacts: [CSerializedContact::default(); MAX_CONTACTS],
                    });
                } else {
                    let count = contacts.len().min(MAX_CONTACTS);
                    let mut resp = CFindValueRespPayload {
                        has_value: 0,
                        value_len: 0,
                        value: Vec::new(),
                        count: count as u8,
                        contacts: [CSerializedContact::default(); MAX_CONTACTS],
                    };
                    for (i, c) in contacts.iter().take(count).enumerate() {
                        resp.contacts[i] = contact_to_c(c);
                    }
                    c_msg.payload = CMessagePayload::FindValueResp(resp);
                }
            }
        }

        Ok(c_msg)
    }

    pub fn from_c_message(c_msg: &CMessage) -> Result<Self, String> {
        let sender_id = NodeId::from_bytes(c_msg.sender_id);
        let request_id = c_msg.request_id;

        let payload = match c_msg.msg_type {
            MSG_PING => RpcPayload::Ping,
            MSG_PONG => RpcPayload::Pong,
            MSG_STORE => {
                if let CMessagePayload::Store(ref s) = c_msg.payload {
                    let key = NodeId::from_bytes(s.key);
                    let vlen = s.value_len as usize;
                    if vlen > MAX_VALUE_SIZE {
                        return Err("Store value len exceeds max".into());
                    }
                    let value = s.value.clone();
                    RpcPayload::Store { key, value }
                } else {
                    return Err("Mismatched payload for MSG_STORE".into());
                }
            }
            MSG_STORE_ACK => {
                if let CMessagePayload::StoreAck(ref s) = c_msg.payload {
                    RpcPayload::StoreAck { ok: s.status == 0 }
                } else {
                    return Err("Mismatched payload for MSG_STORE_ACK".into());
                }
            }
            MSG_FIND_NODE => {
                if let CMessagePayload::FindNode(ref f) = c_msg.payload {
                    let target_id = NodeId::from_bytes(f.target_id);
                    RpcPayload::FindNode { target_id }
                } else {
                    return Err("Mismatched payload for MSG_FIND_NODE".into());
                }
            }
            MSG_FIND_NODE_RESP => {
                if let CMessagePayload::FindNodeResp(ref r) = c_msg.payload {
                    let count = (r.count as usize).min(MAX_CONTACTS);
                    let mut contacts = Vec::with_capacity(count);
                    for i in 0..count {
                        contacts.push(c_to_contact(&r.contacts[i])?);
                    }
                    RpcPayload::FindNodeResp { contacts }
                } else {
                    return Err("Mismatched payload for MSG_FIND_NODE_RESP".into());
                }
            }
            MSG_FIND_VALUE => {
                if let CMessagePayload::FindValue(ref f) = c_msg.payload {
                    let key = NodeId::from_bytes(f.key);
                    RpcPayload::FindValue { key }
                } else {
                    return Err("Mismatched payload for MSG_FIND_VALUE".into());
                }
            }
            MSG_FIND_VALUE_RESP => {
                if let CMessagePayload::FindValueResp(ref r) = c_msg.payload {
                    if r.has_value != 0 {
                        let vlen = r.value_len as usize;
                        if vlen > MAX_VALUE_SIZE {
                            return Err("Value len exceeds max".into());
                        }
                        let value = r.value.clone();
                        RpcPayload::FindValueResp {
                            value: Some(value),
                            contacts: Vec::new(),
                        }
                    } else {
                        let count = (r.count as usize).min(MAX_CONTACTS);
                        let mut contacts = Vec::with_capacity(count);
                        for i in 0..count {
                            contacts.push(c_to_contact(&r.contacts[i])?);
                        }
                        RpcPayload::FindValueResp {
                            value: None,
                            contacts,
                        }
                    }
                } else {
                    return Err("Mismatched payload for MSG_FIND_VALUE_RESP".into());
                }
            }
            other => return Err(format!("Unknown message type: {}", other)),
        };

        Ok(Self {
            request_id,
            sender_id,
            payload,
        })
    }

    pub fn encode(&self) -> Result<Vec<u8>, String> {
        let c_msg = self.to_c_message()?;
        let estimated_size = match &self.payload {
            RpcPayload::Store { value, .. } => 64 + value.len(),
            RpcPayload::FindValueResp { value: Some(v), .. } => 64 + v.len(),
            _ => 1024,
        };
        let mut buf = vec![0u8; estimated_size];
        let written = core_ffi::ffi_encode_message(&c_msg, &mut buf);
        if written < 0 {
            return Err(format!("encode_message failed with code {}", written));
        }
        buf.truncate(written as usize);
        Ok(buf)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut c_msg = Box::new(CMessage::default());
        let res = core_ffi::ffi_decode_message(bytes, &mut c_msg);
        if res < 0 {
            return Err(format!("decode_message failed with code {}", res));
        }
        Self::from_c_message(&c_msg)
    }

    pub fn is_response(&self) -> bool {
        matches!(
            self.payload,
            RpcPayload::Pong
                | RpcPayload::StoreAck { .. }
                | RpcPayload::FindNodeResp { .. }
                | RpcPayload::FindValueResp { .. }
        )
    }
}

fn contact_to_c(c: &Contact) -> CSerializedContact {
    let mut out = CSerializedContact::default();
    out.id = *c.id.as_bytes();
    out.port = c.addr.port();
    match c.addr.ip() {
        IpAddr::V4(ipv4) => {
            out.ip_type = 4;
            out.ip[..4].copy_from_slice(&ipv4.octets());
        }
        IpAddr::V6(ipv6) => {
            out.ip_type = 6;
            out.ip[..16].copy_from_slice(&ipv6.octets());
        }
    }
    out
}

fn c_to_contact(c: &CSerializedContact) -> Result<Contact, String> {
    if c.port == 0 {
        return Err("Port 0 is invalid for contact".into());
    }
    let id = NodeId::from_bytes(c.id);
    let ip = match c.ip_type {
        4 => {
            let mut oct = [0u8; 4];
            oct.copy_from_slice(&c.ip[..4]);
            let ipv4 = Ipv4Addr::from(oct);
            if ipv4.is_unspecified() || ipv4.is_broadcast() || ipv4.is_multicast() {
                return Err(format!("Unroutable IPv4 address: {}", ipv4));
            }
            IpAddr::V4(ipv4)
        }
        6 => {
            let mut oct = [0u8; 16];
            oct.copy_from_slice(&c.ip[..16]);
            let ipv6 = Ipv6Addr::from(oct);
            if ipv6.is_unspecified() || ipv6.is_multicast() {
                return Err(format!("Unroutable IPv6 address: {}", ipv6));
            }
            IpAddr::V6(ipv6)
        }
        other => return Err(format!("Invalid ip_type: {}", other)),
    };
    Ok(Contact::new(id, SocketAddr::new(ip, c.port)))
}

type PendingMap = Arc<Mutex<HashMap<u64, (SocketAddr, oneshot::Sender<RpcMessage>)>>>;

pub struct NetworkManager {
    pub local_id: NodeId,
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub metrics: Arc<MetricsTracker>,
    pending: PendingMap,
}

impl NetworkManager {
    pub async fn bind(addr: SocketAddr, local_id: NodeId) -> Result<Arc<Self>, std::io::Error> {
        let socket = UdpSocket::bind(addr).await?;
        let local_addr = socket.local_addr()?;
        println!("[NET] Bound UDP socket on {}", local_addr);

        let nm = Arc::new(Self {
            local_id,
            socket: Arc::new(socket),
            local_addr,
            metrics: MetricsTracker::new(),
            pending: Arc::new(Mutex::new(HashMap::new())),
        });

        Ok(nm)
    }

    pub async fn bind_auto(
        ip: &str,
        base_port: u16,
        max_attempts: u16,
        local_id: NodeId,
    ) -> Result<Arc<Self>, std::io::Error> {
        let mut last_err = None;
        for offset in 0..max_attempts {
            let port = base_port.wrapping_add(offset);
            let addr_str = format!("{}:{}", ip, port);
            if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                match Self::bind(addr, local_id).await {
                    Ok(nm) => return Ok(nm),
                    Err(e) => {
                        last_err = Some(e);
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::AddrNotAvailable,
                format!("Failed to bind any UDP port starting from {}:{}", ip, base_port),
            )
        }))
    }

    pub fn next_request_id(&self) -> u64 {
        rand::random::<u64>()
    }

    pub async fn send_message(&self, target: SocketAddr, msg: &RpcMessage) -> Result<(), String> {
        let bytes = msg.encode()?;
        let msg_type = match msg.to_c_message() {
            Ok(c) => c.msg_type,
            Err(_) => 0,
        };
        self.metrics.inc_rpc_sent(msg_type);

        if bytes.len() > CHUNK_THRESHOLD {
            // Fragment into UDP chunks for WAN safety (prevent IP-level fragmentation)
            let message_id: [u8; 16] = rand::random();
            let chunks = UdpChunk::split_data(message_id, &bytes);
            let chunk_count = chunks.len();
            let mut total_wire_bytes = 0usize;
            for (idx, chunk) in chunks.iter().enumerate() {
                let chunk_bytes = chunk.to_bytes();
                total_wire_bytes += chunk_bytes.len();
                self.socket
                    .send_to(&chunk_bytes, target)
                    .await
                    .map_err(|e| format!("[NET] Chunk send failed to {}: {}", target, e))?;
                if (idx + 1) % 4 == 0 {
                    tokio::time::sleep(Duration::from_micros(200)).await;
                }
            }
            self.metrics.inc_bytes_sent(total_wire_bytes);
            println!(
                "[NET] Sent {} bytes as {} binary chunks to {} (original {} bytes)",
                total_wire_bytes, chunk_count, target, bytes.len()
            );
        } else {
            // Direct send — fits in a single UDP datagram
            self.metrics.inc_bytes_sent(bytes.len());
            self.socket
                .send_to(&bytes, target)
                .await
                .map_err(|e| format!("[NET] Send failed to {}: {}", target, e))?;
        }

        Ok(())
    }

    pub async fn call(
        &self,
        target: SocketAddr,
        payload: RpcPayload,
        timeout_dur: Duration,
        max_retries: usize,
    ) -> Result<RpcMessage, String> {
        let req_id = self.next_request_id();
        let msg = RpcMessage::new(req_id, self.local_id, payload);

        for attempt in 0..=max_retries {
            let (tx, rx) = oneshot::channel();
            {
                let mut map = self.pending.lock().await;
                map.insert(req_id, (target, tx));
            }

            println!(
                "[RPC] Sending {:?} to {} (req_id={}, attempt={}/{})",
                rpc_type_name(&msg.payload),
                target,
                req_id,
                attempt + 1,
                max_retries + 1
            );

            if let Err(e) = self.send_message(target, &msg).await {
                let mut map = self.pending.lock().await;
                map.remove(&req_id);
                self.metrics.inc_rpc_failed();
                return Err(e);
            }

            match tokio::time::timeout(timeout_dur, rx).await {
                Ok(Ok(response)) => {
                    return Ok(response);
                }
                Ok(Err(_)) => {
                    let mut map = self.pending.lock().await;
                    map.remove(&req_id);
                    self.metrics.inc_rpc_failed();
                    return Err("[RPC] Channel canceled".into());
                }
                Err(_) => {
                    let mut map = self.pending.lock().await;
                    map.remove(&req_id);
                    println!(
                        "[RPC] Timeout waiting for response from {} (req_id={})",
                        target, req_id
                    );
                    if attempt == max_retries {
                        self.metrics.inc_rpc_failed();
                        return Err(format!("RPC call to {} timed out after retries", target));
                    }
                }
            }
        }

        self.metrics.inc_rpc_failed();
        Err("RPC failed".into())
    }

    pub fn start_receive_loop<F, Fut>(
        self: &Arc<Self>,
        routing_table: Arc<RwLock<RoutingTable>>,
        request_handler: F,
    ) where
        F: Fn(RpcMessage, SocketAddr) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Option<RpcMessage>> + Send + 'static,
    {
        let nm = Arc::clone(self);
        let socket = Arc::clone(&self.socket);
        let pending = Arc::clone(&self.pending);
        let handler = Arc::new(request_handler);
        let metrics = Arc::clone(&self.metrics);

        tokio::spawn(async move {
            let mut chunk_assembler = ChunkAssembler::new(Duration::from_secs(30));
            let mut buf = vec![0u8; 65536];
            loop {
                let (len, src_addr) = match socket.recv_from(&mut buf).await {
                    Ok(res) => res,
                    Err(e) => {
                        // Handle Windows UDP connection reset (WSAECONNRESET / 10054) gracefully
                        let raw_err = e.raw_os_error();
                        if raw_err != Some(10054) {
                            eprintln!("[ERROR] Socket recv error: {}", e);
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                };

                metrics.inc_bytes_received(len);

                // Chunk-aware decode: binary chunks start with 'NXCK'
                let msg = if len >= 4 && &buf[..4] == crate::chunking::BINARY_MAGIC {
                    match UdpChunk::from_bytes(&buf[..len]) {
                        Ok(chunk) => {
                            match chunk_assembler.add_chunk(src_addr, chunk) {
                                Some(complete_bytes) => match RpcMessage::decode(&complete_bytes) {
                                    Ok(m) => m,
                                    Err(e) => {
                                        eprintln!(
                                            "[NET] Failed to decode reassembled binary chunked message from {}: {}",
                                            src_addr, e
                                        );
                                        continue;
                                    }
                                },
                                None => continue,
                            }
                        }
                        Err(e) => {
                            eprintln!("[NET] Invalid binary chunk from {}: {}", src_addr, e);
                            continue;
                        }
                    }
                } else {
                    // Standard binary RPC message
                    match RpcMessage::decode(&buf[..len]) {
                        Ok(m) => m,
                        Err(e) => {
                            eprintln!("[NET] Failed to decode packet from {}: {}", src_addr, e);
                            continue;
                        }
                    }
                };

                if let Ok(c) = msg.to_c_message() {
                    metrics.inc_rpc_received(c.msg_type);
                }

                let contact = Contact::new(msg.sender_id, src_addr);
                let update_res = {
                    let mut rt = routing_table.write().await;
                    rt.update(contact.clone())
                };

                if let UpdateResult::Full(oldest) = update_res {
                    let nm_clone = Arc::clone(&nm);
                    let rt_clone = Arc::clone(&routing_table);
                    tokio::spawn(async move {
                        let ping_res = nm_clone
                            .call(oldest.addr, RpcPayload::Ping, Duration::from_secs(3), 1)
                            .await;
                        if ping_res.is_err() {
                            let mut rt = rt_clone.write().await;
                            rt.replace_oldest(contact);
                        }
                    });
                }

                if msg.is_response() {
                    let mut map = pending.lock().await;
                    if let Some((expected_addr, tx)) = map.remove(&msg.request_id) {
                        if expected_addr == src_addr {
                            let _ = tx.send(msg);
                        } else {
                            eprintln!(
                                "[SECURITY] Dropping response for req_id {} from unexpected source {} (expected {})",
                                msg.request_id, src_addr, expected_addr
                            );
                        }
                    }
                } else {
                    let nm_clone = Arc::clone(&nm);
                    let handler_clone = Arc::clone(&handler);
                    tokio::spawn(async move {
                        if let Some(reply) = handler_clone(msg, src_addr).await {
                            if let Err(e) = nm_clone.send_message(src_addr, &reply).await {
                                eprintln!("[ERROR] Failed to send reply to {}: {}", src_addr, e);
                            }
                        }
                    });
                }
            }
        });
    }
}

pub fn rpc_type_name(payload: &RpcPayload) -> &'static str {
    match payload {
        RpcPayload::Ping => "PING",
        RpcPayload::Pong => "PONG",
        RpcPayload::Store { .. } => "STORE",
        RpcPayload::StoreAck { .. } => "STORE_ACK",
        RpcPayload::FindNode { .. } => "FIND_NODE",
        RpcPayload::FindNodeResp { .. } => "FIND_NODE_RESP",
        RpcPayload::FindValue { .. } => "FIND_VALUE",
        RpcPayload::FindValueResp { .. } => "FIND_VALUE_RESP",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ping_pong() {
        let sender = NodeId::generate_random();
        let ping = RpcMessage::new(12345, sender, RpcPayload::Ping);
        let enc = ping.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        assert_eq!(ping, dec);

        let pong = RpcMessage::new(12345, sender, RpcPayload::Pong);
        let enc = pong.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        assert_eq!(pong, dec);
    }

    #[test]
    fn roundtrip_store_and_ack() {
        let sender = NodeId::generate_random();
        let key = NodeId::from_key(b"my_secret_key");
        let value = b"Hello, Kademlia DHT!".to_vec();

        let store = RpcMessage::new(999, sender, RpcPayload::Store { key, value });
        let enc = store.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        assert_eq!(store, dec);

        let ack = RpcMessage::new(999, sender, RpcPayload::StoreAck { ok: true });
        let enc = ack.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        assert_eq!(ack, dec);
    }

    #[test]
    fn roundtrip_find_node_and_resp() {
        let sender = NodeId::generate_random();
        let target = NodeId::generate_random();

        let find_node = RpcMessage::new(42, sender, RpcPayload::FindNode { target_id: target });
        let enc = find_node.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        assert_eq!(find_node, dec);

        let contacts = vec![
            Contact::new(NodeId::generate_random(), "127.0.0.1:8001".parse().unwrap()),
            Contact::new(NodeId::generate_random(), "192.168.1.50:9000".parse().unwrap()),
        ];
        let resp = RpcMessage::new(42, sender, RpcPayload::FindNodeResp { contacts: contacts.clone() });
        let enc = resp.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        if let RpcPayload::FindNodeResp { contacts: dec_contacts } = dec.payload {
            assert_eq!(dec_contacts.len(), 2);
            assert_eq!(dec_contacts[0].id, contacts[0].id);
            assert_eq!(dec_contacts[0].addr, contacts[0].addr);
            assert_eq!(dec_contacts[1].id, contacts[1].id);
            assert_eq!(dec_contacts[1].addr, contacts[1].addr);
        } else {
            panic!("Expected FindNodeResp");
        }
    }

    #[test]
    fn roundtrip_find_value_and_resp() {
        let sender = NodeId::generate_random();
        let key = NodeId::from_key(b"sample_key");

        let find_val = RpcMessage::new(77, sender, RpcPayload::FindValue { key });
        let enc = find_val.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        assert_eq!(find_val, dec);

        // Value found response
        let resp_val = RpcMessage::new(
            77,
            sender,
            RpcPayload::FindValueResp {
                value: Some(b"Found it!".to_vec()),
                contacts: Vec::new(),
            },
        );
        let enc = resp_val.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        assert_eq!(resp_val, dec);

        // Value not found, returning contacts
        let contacts = vec![
            Contact::new(NodeId::generate_random(), "10.0.0.1:1234".parse().unwrap()),
        ];
        let resp_nodes = RpcMessage::new(
            77,
            sender,
            RpcPayload::FindValueResp {
                value: None,
                contacts: contacts.clone(),
            },
        );
        let enc = resp_nodes.encode().unwrap();
        let dec = RpcMessage::decode(&enc).unwrap();
        if let RpcPayload::FindValueResp { value, contacts: dec_c } = dec.payload {
            assert!(value.is_none());
            assert_eq!(dec_c.len(), 1);
            assert_eq!(dec_c[0].id, contacts[0].id);
            assert_eq!(dec_c[0].addr, contacts[0].addr);
        } else {
            panic!("Expected FindValueResp with contacts");
        }
    }
}
