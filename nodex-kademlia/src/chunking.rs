use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Maximum allowed total UDP packet size to prevent MTU fragmentation across Internet routers.
pub const MAX_UDP_PACKET_SIZE: usize = 1200;

/// Header size for binary chunk: 4 (magic) + 16 (msg_id) + 2 (idx) + 2 (count) + 4 (crc32) = 28 bytes
pub const BINARY_HEADER_SIZE: usize = 28;
pub const BINARY_MAGIC: &[u8; 4] = b"NXCK";

/// Conservative maximum payload size per chunk ensuring total UDP packet <= 1200 bytes.
pub const CHUNK_PAYLOAD_SIZE: usize = 500;

/// Chunk header and payload structure.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UdpChunk {
    pub message_id: [u8; 16],
    pub chunk_index: u16,
    pub chunk_count: u16,
    pub checksum: u32,
    pub payload_hex: String,
    #[serde(skip)]
    pub raw_data: Option<Vec<u8>>,
}

impl UdpChunk {
    /// Compute CRC32 checksum of payload for integrity verification.
    pub fn compute_checksum(data: &[u8]) -> u32 {
        crc32fast::hash(data)
    }

    /// Split arbitrary binary data into chunks that strictly fit into MAX_UDP_PACKET_SIZE.
    pub fn split_data(message_id: [u8; 16], data: &[u8]) -> Vec<UdpChunk> {
        let safe_payload_size = CHUNK_PAYLOAD_SIZE; // 500 bytes -> 1000 hex chars + 100 JSON overhead = ~1100 bytes <= 1200 bytes!
        if data.is_empty() {
            return vec![UdpChunk {
                message_id,
                chunk_index: 0,
                chunk_count: 1,
                checksum: Self::compute_checksum(&[]),
                payload_hex: String::new(),
                raw_data: Some(Vec::new()),
            }];
        }

        let chunk_count = data.len().div_ceil(safe_payload_size) as u16;
        let mut chunks = Vec::with_capacity(chunk_count as usize);

        for (idx, slice) in data.chunks(safe_payload_size).enumerate() {
            let chunk = UdpChunk {
                message_id,
                chunk_index: idx as u16,
                chunk_count,
                checksum: Self::compute_checksum(slice),
                payload_hex: slice.iter().map(|b| format!("{:02x}", b)).collect(),
                raw_data: Some(slice.to_vec()),
            };
            chunks.push(chunk);
        }

        chunks
    }

    /// Encode as efficient binary UDP datagram.
    pub fn to_bytes(&self) -> Vec<u8> {
        let raw = if let Some(ref r) = self.raw_data {
            r.clone()
        } else {
            self.decode_payload().unwrap_or_default()
        };

        let mut out = Vec::with_capacity(BINARY_HEADER_SIZE + raw.len());
        out.extend_from_slice(BINARY_MAGIC);
        out.extend_from_slice(&self.message_id);
        out.extend_from_slice(&self.chunk_index.to_be_bytes());
        out.extend_from_slice(&self.chunk_count.to_be_bytes());
        out.extend_from_slice(&self.checksum.to_be_bytes());
        out.extend_from_slice(&raw);
        out
    }

    /// Decode from binary UDP datagram.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < BINARY_HEADER_SIZE {
            return Err("Packet too short for binary chunk".into());
        }
        if &bytes[..4] != BINARY_MAGIC {
            return Err("Invalid binary chunk magic".into());
        }

        let mut msg_id = [0u8; 16];
        msg_id.copy_from_slice(&bytes[4..20]);

        let chunk_index = u16::from_be_bytes([bytes[20], bytes[21]]);
        let chunk_count = u16::from_be_bytes([bytes[22], bytes[23]]);
        let checksum = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);

        let raw = bytes[BINARY_HEADER_SIZE..].to_vec();

        Ok(Self {
            message_id: msg_id,
            chunk_index,
            chunk_count,
            checksum,
            payload_hex: raw.iter().map(|b| format!("{:02x}", b)).collect(),
            raw_data: Some(raw),
        })
    }

    /// Decode raw payload bytes.
    pub fn decode_payload(&self) -> Result<Vec<u8>, String> {
        if let Some(ref r) = self.raw_data {
            return Ok(r.clone());
        }
        if !self.payload_hex.len().is_multiple_of(2) {
            return Err("Invalid hex payload length".into());
        }
        (0..self.payload_hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&self.payload_hex[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

use std::net::SocketAddr;

pub const MAX_CHUNKS_PER_MESSAGE: u16 = 256;
pub const MAX_PENDING_ASSEMBLIES: usize = 256;
pub const MAX_PENDING_PER_IP: usize = 16;

/// Helper struct for assembling chunks back into the complete original message.
struct PendingAssembly {
    chunks: HashMap<u16, Vec<u8>>,
    chunk_count: u16,
    first_received: Instant,
}

pub struct ChunkAssembler {
    pending: HashMap<(SocketAddr, [u8; 16]), PendingAssembly>,
    reassembly_timeout: Duration,
}

impl ChunkAssembler {
    pub fn new(timeout: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            reassembly_timeout: timeout,
        }
    }

    /// Add a received chunk bound to the sender socket address. Returns `Some(complete_data)` when all chunks are collected and verified.
    pub fn add_chunk(&mut self, src_addr: SocketAddr, chunk: UdpChunk) -> Option<Vec<u8>> {
        self.cleanup_expired();

        if chunk.chunk_count == 0 || chunk.chunk_count > MAX_CHUNKS_PER_MESSAGE || chunk.chunk_index >= chunk.chunk_count {
            eprintln!("[CHUNKING] Invalid chunk geometry ({}/{}) from {}", chunk.chunk_index, chunk.chunk_count, src_addr);
            return None;
        }

        let raw_payload = chunk.decode_payload().ok()?;

        // Verify chunk checksum
        if UdpChunk::compute_checksum(&raw_payload) != chunk.checksum {
            eprintln!("[CHUNKING] Checksum mismatch on chunk {}/{} from {}", chunk.chunk_index, chunk.chunk_count, src_addr);
            return None;
        }

        let msg_id = chunk.message_id;
        let chunk_count = chunk.chunk_count;
        let chunk_index = chunk.chunk_index;

        if chunk_count == 1 && chunk_index == 0 {
            return Some(raw_payload);
        }

        let key = (src_addr, msg_id);

        if !self.pending.contains_key(&key) {
            if self.pending.len() >= MAX_PENDING_ASSEMBLIES {
                eprintln!("[CHUNKING] Global pending assembly limit reached, dropping chunk from {}", src_addr);
                return None;
            }
            let per_ip_count = self.pending.keys().filter(|(addr, _)| addr.ip() == src_addr.ip()).count();
            if per_ip_count >= MAX_PENDING_PER_IP {
                eprintln!("[CHUNKING] Per-IP pending assembly limit reached for {}", src_addr.ip());
                return None;
            }
        }

        let entry = self.pending.entry(key).or_insert_with(|| PendingAssembly {
            chunks: HashMap::new(),
            chunk_count,
            first_received: Instant::now(),
        });

        if entry.chunk_count != chunk_count {
            eprintln!("[CHUNKING] Mismatched chunk count in message from {}", src_addr);
            return None;
        }

        entry.chunks.insert(chunk_index, raw_payload);

        if entry.chunks.len() == entry.chunk_count as usize {
            // All chunks received! Reassemble in order.
            let mut assembled = Vec::new();
            for i in 0..entry.chunk_count {
                let part = entry.chunks.get(&i)?;
                assembled.extend_from_slice(part);
            }
            self.pending.remove(&key);
            Some(assembled)
        } else {
            None
        }
    }

    /// Remove pending assemblies that exceeded the reassembly timeout.
    pub fn cleanup_expired(&mut self) {
        let timeout = self.reassembly_timeout;
        self.pending.retain(|_, v| v.first_received.elapsed() < timeout);
    }
}

