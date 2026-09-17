use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Maximum allowed total UDP packet size to prevent MTU fragmentation across Internet routers.
pub const MAX_UDP_PACKET_SIZE: usize = 1200;

/// Conservative maximum payload size per chunk ensuring total serialized JSON packet <= 1200 bytes.
pub const CHUNK_PAYLOAD_SIZE: usize = 750;

/// Chunk header and payload structure.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UdpChunk {
    pub message_id: [u8; 16],
    pub chunk_index: u16,
    pub chunk_count: u16,
    pub checksum: u32,
    pub payload_hex: String,
}

impl UdpChunk {
    /// Compute CRC32 checksum of payload for integrity verification.
    pub fn compute_checksum(data: &[u8]) -> u32 {
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(data);
        hasher.finalize()
    }

    /// Split arbitrary binary data into chunks that strictly fit into MAX_UDP_PACKET_SIZE.
    pub fn split_data(message_id: [u8; 16], data: &[u8]) -> Vec<UdpChunk> {
        let safe_payload_size = 500; // 500 bytes -> 1000 hex chars + 100 JSON overhead = ~1100 bytes <= 1200 bytes!
        if data.is_empty() {
            return vec![UdpChunk {
                message_id,
                chunk_index: 0,
                chunk_count: 1,
                checksum: Self::compute_checksum(&[]),
                payload_hex: String::new(),
            }];
        }

        let chunk_count = data.len().div_ceil(safe_payload_size) as u16;
        let mut chunks = Vec::with_capacity(chunk_count as usize);

        for (idx, slice) in data.chunks(safe_payload_size).enumerate() {
            let hex_str: String = slice.iter().map(|b| format!("{:02x}", b)).collect();
            let chunk = UdpChunk {
                message_id,
                chunk_index: idx as u16,
                chunk_count,
                checksum: Self::compute_checksum(slice),
                payload_hex: hex_str,
            };
            chunks.push(chunk);
        }

        chunks
    }

    /// Decode raw payload bytes from hex string.
    pub fn decode_payload(&self) -> Result<Vec<u8>, String> {
        if !self.payload_hex.len().is_multiple_of(2) {
            return Err("Invalid hex payload length".into());
        }
        (0..self.payload_hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&self.payload_hex[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

/// Helper struct for assembling chunks back into the complete original message.
struct PendingAssembly {
    chunks: HashMap<u16, Vec<u8>>,
    chunk_count: u16,
    first_received: Instant,
}

pub struct ChunkAssembler {
    pending: HashMap<[u8; 16], PendingAssembly>,
    reassembly_timeout: Duration,
}

impl ChunkAssembler {
    pub fn new(timeout: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            reassembly_timeout: timeout,
        }
    }

    /// Add a received chunk. Returns `Some(complete_data)` when all chunks are collected and verified.
    pub fn add_chunk(&mut self, chunk: UdpChunk) -> Option<Vec<u8>> {
        self.cleanup_expired();

        let raw_payload = chunk.decode_payload().ok()?;

        // Verify chunk checksum
        if UdpChunk::compute_checksum(&raw_payload) != chunk.checksum {
            eprintln!("[CHUNKING] Checksum mismatch on chunk {}/{}", chunk.chunk_index, chunk.chunk_count);
            return None;
        }

        let msg_id = chunk.message_id;
        let chunk_count = chunk.chunk_count;
        let chunk_index = chunk.chunk_index;

        if chunk_count == 1 && chunk_index == 0 {
            return Some(raw_payload);
        }

        let entry = self.pending.entry(msg_id).or_insert_with(|| PendingAssembly {
            chunks: HashMap::new(),
            chunk_count,
            first_received: Instant::now(),
        });

        entry.chunks.insert(chunk_index, raw_payload);

        if entry.chunks.len() == entry.chunk_count as usize {
            // All chunks received! Reassemble in order.
            let mut assembled = Vec::new();
            for i in 0..entry.chunk_count {
                let part = entry.chunks.get(&i)?;
                assembled.extend_from_slice(part);
            }
            self.pending.remove(&msg_id);
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
