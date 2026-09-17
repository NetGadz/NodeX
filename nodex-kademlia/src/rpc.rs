use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
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

    pub fn to_c_message(&self) -> Result<CMessage, String> {
        let mut c_msg: CMessage = unsafe { std::mem::zeroed() };
        c_msg.request_id = self.request_id;
        c_msg.sender_id = *self.sender_id.as_bytes();

        match &self.payload {
            RpcPayload::Ping => {
                c_msg.msg_type = MSG_PING;
            }
            RpcPayload::Pong => {
                c_msg.msg_type = MSG_PONG;
            }
            RpcPayload::Store { key, value } => {
                if value.len() > MAX_VALUE_SIZE {
                    return Err(format!("Value too large: {} > {}", value.len(), MAX_VALUE_SIZE));
                }
                c_msg.msg_type = MSG_STORE;
                let mut store = CStorePayload {
                    key: *key.as_bytes(),
                    value_len: value.len() as u32,
                    value: [0u8; MAX_VALUE_SIZE],
                };
                store.value[..value.len()].copy_from_slice(value);
                c_msg.payload.store = store;
            }
            RpcPayload::StoreAck { ok } => {
                c_msg.msg_type = MSG_STORE_ACK;
                c_msg.payload.store_ack = CStoreAckPayload {
                    status: if *ok { 0 } else { 1 },
                };
            }
            RpcPayload::FindNode { target_id } => {
                c_msg.msg_type = MSG_FIND_NODE;
                c_msg.payload.find_node = CFindNodePayload {
                    target_id: *target_id.as_bytes(),
                };
            }
            RpcPayload::FindNodeResp { contacts } => {
                c_msg.msg_type = MSG_FIND_NODE_RESP;
                let count = contacts.len().min(MAX_CONTACTS);
                let mut c_contacts: [CSerializedContact; MAX_CONTACTS] =
                    unsafe { std::mem::zeroed() };
                for (i, c) in contacts.iter().take(count).enumerate() {
                    c_contacts[i] = contact_to_c(c);
                }
                c_msg.payload.find_node_resp = CFindNodeRespPayload {
                    count: count as u8,
                    contacts: c_contacts,
                };
            }
            RpcPayload::FindValue { key } => {
                c_msg.msg_type = MSG_FIND_VALUE;
                c_msg.payload.find_value = CFindValuePayload {
                    key: *key.as_bytes(),
                };
            }
            RpcPayload::FindValueResp { value, contacts } => {
                c_msg.msg_type = MSG_FIND_VALUE_RESP;
                let mut resp: CFindValueRespPayload = unsafe { std::mem::zeroed() };
                if let Some(val) = value {
                    if val.len() > MAX_VALUE_SIZE {
                        return Err(format!("Value too large: {} > {}", val.len(), MAX_VALUE_SIZE));
                    }
                    resp.has_value = 1;
                    resp.value_len = val.len() as u32;
                    resp.value[..val.len()].copy_from_slice(val);
                } else {
                    resp.has_value = 0;
                    let count = contacts.len().min(MAX_CONTACTS);
                    resp.count = count as u8;
                    for (i, c) in contacts.iter().take(count).enumerate() {
                        resp.contacts[i] = contact_to_c(c);
                    }
                }
                c_msg.payload.find_value_resp = resp;
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
                let s = unsafe { &c_msg.payload.store };
                let key = NodeId::from_bytes(s.key);
                let vlen = s.value_len as usize;
                if vlen > MAX_VALUE_SIZE {
                    return Err("Store value len exceeds max".into());
                }
                let value = s.value[..vlen].to_vec();
                RpcPayload::Store { key, value }
            }
            MSG_STORE_ACK => {
                let s = unsafe { &c_msg.payload.store_ack };
                RpcPayload::StoreAck { ok: s.status == 0 }
            }
            MSG_FIND_NODE => {
                let f = unsafe { &c_msg.payload.find_node };
                let target_id = NodeId::from_bytes(f.target_id);
                RpcPayload::FindNode { target_id }
            }
            MSG_FIND_NODE_RESP => {
                let r = unsafe { &c_msg.payload.find_node_resp };
                let count = (r.count as usize).min(MAX_CONTACTS);
                let mut contacts = Vec::with_capacity(count);
                for i in 0..count {
                    contacts.push(c_to_contact(&r.contacts[i])?);
                }
                RpcPayload::FindNodeResp { contacts }
            }
            MSG_FIND_VALUE => {
                let f = unsafe { &c_msg.payload.find_value };
                let key = NodeId::from_bytes(f.key);
                RpcPayload::FindValue { key }
            }
            MSG_FIND_VALUE_RESP => {
                let r = unsafe { &c_msg.payload.find_value_resp };
                if r.has_value != 0 {
                    let vlen = r.value_len as usize;
                    if vlen > MAX_VALUE_SIZE {
                        return Err("Value len exceeds max".into());
                    }
                    let value = r.value[..vlen].to_vec();
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
        let mut buf = vec![0u8; MAX_VALUE_SIZE + 1024];
        let written = core_ffi::ffi_encode_message(&c_msg, &mut buf);
        if written < 0 {
            return Err(format!("encode_message failed with code {}", written));
        }
        buf.truncate(written as usize);
        Ok(buf)
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut c_msg: CMessage = unsafe { std::mem::zeroed() };
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
    let mut out: CSerializedContact = unsafe { std::mem::zeroed() };
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
    let id = NodeId::from_bytes(c.id);
    let ip = match c.ip_type {
        4 => {
            let mut oct = [0u8; 4];
            oct.copy_from_slice(&c.ip[..4]);
            IpAddr::V4(Ipv4Addr::from(oct))
        }
        6 => {
            let mut oct = [0u8; 16];
            oct.copy_from_slice(&c.ip[..16]);
            IpAddr::V6(Ipv6Addr::from(oct))
        }
        other => return Err(format!("Invalid ip_type: {}", other)),
    };
    Ok(Contact::new(id, SocketAddr::new(ip, c.port)))
}

type PendingMap = Arc<Mutex<HashMap<u64, oneshot::Sender<RpcMessage>>>>;

pub struct NetworkManager {
    pub local_id: NodeId,
    pub socket: Arc<UdpSocket>,
    pub local_addr: SocketAddr,
    pub metrics: Arc<MetricsTracker>,
    next_req_id: AtomicU64,
    pending: PendingMap,
}

impl NetworkManager {
    pub async fn bind(addr: SocketAddr, local_id: NodeId) -> Result<Arc<Self>, std::io::Error> {
        let socket = UdpSocket::bind(addr).await?;
        let local_addr = socket.local_addr()?;
        println!("[NET] Bound UDP socket on {}", local_addr);

        let initial_req_id = rand::random::<u64>();
        let nm = Arc::new(Self {
            local_id,
            socket: Arc::new(socket),
            local_addr,
            metrics: MetricsTracker::new(),
            next_req_id: AtomicU64::new(initial_req_id),
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
        self.next_req_id.fetch_add(1, Ordering::Relaxed)
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
            for chunk in &chunks {
                let chunk_bytes = serde_json::to_vec(chunk)
                    .map_err(|e| format!("Chunk serialization failed: {}", e))?;
                total_wire_bytes += chunk_bytes.len();
                self.socket
                    .send_to(&chunk_bytes, target)
                    .await
                    .map_err(|e| format!("[NET] Chunk send failed to {}: {}", target, e))?;
            }
            self.metrics.inc_bytes_sent(total_wire_bytes);
            println!(
                "[NET] Sent {} bytes as {} chunks to {} (original {} bytes)",
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
                map.insert(req_id, tx);
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

                // Chunk-aware decode: JSON chunks start with '{', binary RPC starts with 'KD'
                let msg = if len > 0 && buf[0] == b'{' {
                    // Attempt to parse as a UDP chunk (JSON-serialized)
                    match serde_json::from_slice::<UdpChunk>(&buf[..len]) {
                        Ok(chunk) => {
                            match chunk_assembler.add_chunk(chunk) {
                                Some(complete_bytes) => {
                                    // All chunks received — decode reassembled RPC message
                                    match RpcMessage::decode(&complete_bytes) {
                                        Ok(m) => m,
                                        Err(e) => {
                                            eprintln!(
                                                "[NET] Failed to decode reassembled chunked message from {}: {}",
                                                src_addr, e
                                            );
                                            continue;
                                        }
                                    }
                                }
                                None => continue, // Waiting for more chunks
                            }
                        }
                        Err(_) => {
                            eprintln!("[NET] JSON-like packet from {} is not a valid chunk, dropping", src_addr);
                            continue;
                        }
                    }
                } else {
                    // Standard binary RPC message (magic bytes 'KD')
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
                    if let Some(tx) = map.remove(&msg.request_id) {
                        let _ = tx.send(msg);
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
