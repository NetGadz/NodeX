//! 100% Pure Safe Rust implementation of core types, hashing, serialization, and BIP-39 mnemonic.
//! No C compiler, build scripts, or raw unsafe pointers are used.

use sha1::{Digest, Sha1};

pub const ID_SIZE: usize = 20;
pub const MAX_VALUE_SIZE: usize = 65536;
pub const MAX_CONTACTS: usize = 20;

// Message type constants matching the NodeX wire protocol
pub const MSG_PING: u8 = 1;
pub const MSG_PONG: u8 = 2;
pub const MSG_STORE: u8 = 3;
pub const MSG_STORE_ACK: u8 = 4;
pub const MSG_FIND_NODE: u8 = 5;
pub const MSG_FIND_NODE_RESP: u8 = 6;
pub const MSG_FIND_VALUE: u8 = 7;
pub const MSG_FIND_VALUE_RESP: u8 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CSerializedContact {
    pub id: [u8; ID_SIZE],
    pub ip_type: u8,
    pub ip: [u8; 16],
    pub port: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CStorePayload {
    pub key: [u8; ID_SIZE],
    pub value_len: u32,
    pub value: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CStoreAckPayload {
    pub status: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CFindNodePayload {
    pub target_id: [u8; ID_SIZE],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CFindNodeRespPayload {
    pub count: u8,
    pub contacts: [CSerializedContact; MAX_CONTACTS],
}

impl Default for CFindNodeRespPayload {
    fn default() -> Self {
        Self {
            count: 0,
            contacts: [CSerializedContact::default(); MAX_CONTACTS],
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CFindValuePayload {
    pub key: [u8; ID_SIZE],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CFindValueRespPayload {
    pub has_value: u8,
    pub value_len: u32,
    pub value: Vec<u8>,
    pub count: u8,
    pub contacts: [CSerializedContact; MAX_CONTACTS],
}

impl Default for CFindValueRespPayload {
    fn default() -> Self {
        Self {
            has_value: 0,
            value_len: 0,
            value: Vec::new(),
            count: 0,
            contacts: [CSerializedContact::default(); MAX_CONTACTS],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CMessagePayload {
    None,
    Store(CStorePayload),
    StoreAck(CStoreAckPayload),
    FindNode(CFindNodePayload),
    FindNodeResp(CFindNodeRespPayload),
    FindValue(CFindValuePayload),
    FindValueResp(CFindValueRespPayload),
}

impl Default for CMessagePayload {
    fn default() -> Self {
        CMessagePayload::None
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CMessage {
    pub msg_type: u8,
    pub request_id: u64,
    pub sender_id: [u8; ID_SIZE],
    pub payload: CMessagePayload,
}

/// SHA-1 hash of `input`, returns a 20-byte node ID.
pub fn ffi_hash_node_id(input: &[u8]) -> [u8; ID_SIZE] {
    let mut hasher = Sha1::new();
    hasher.update(input);
    let result = hasher.finalize();
    let mut out = [0u8; ID_SIZE];
    out.copy_from_slice(&result[..ID_SIZE]);
    out
}

/// Count of shared prefix bits between two 20-byte IDs.
pub fn ffi_shared_prefix_bits(id1: &[u8; ID_SIZE], id2: &[u8; ID_SIZE]) -> u32 {
    let mut count = 0u32;
    for i in 0..ID_SIZE {
        let xor = id1[i] ^ id2[i];
        if xor == 0 {
            count += 8;
        } else {
            count += xor.leading_zeros();
            break;
        }
    }
    count
}

/// Encode a CMessage into a byte buffer safely. Returns number of bytes written, or negative on error.
pub fn ffi_encode_message(msg: &CMessage, out_buf: &mut [u8]) -> i32 {
    let mut cursor = 0usize;
    if out_buf.len() < 1 + 8 + ID_SIZE {
        return -1;
    }

    // Header
    out_buf[cursor] = msg.msg_type;
    cursor += 1;

    out_buf[cursor..cursor + 8].copy_from_slice(&msg.request_id.to_be_bytes());
    cursor += 8;

    out_buf[cursor..cursor + ID_SIZE].copy_from_slice(&msg.sender_id);
    cursor += ID_SIZE;

    match &msg.payload {
        CMessagePayload::None => {
            if msg.msg_type != MSG_PING && msg.msg_type != MSG_PONG {
                return -1;
            }
        }
        CMessagePayload::Store(st) => {
            let vlen = (st.value_len as usize).min(st.value.len()).min(MAX_VALUE_SIZE);
            if out_buf.len() < cursor + ID_SIZE + 4 + vlen {
                return -1;
            }
            out_buf[cursor..cursor + ID_SIZE].copy_from_slice(&st.key);
            cursor += ID_SIZE;
            out_buf[cursor..cursor + 4].copy_from_slice(&(vlen as u32).to_be_bytes());
            cursor += 4;
            out_buf[cursor..cursor + vlen].copy_from_slice(&st.value[..vlen]);
            cursor += vlen;
        }
        CMessagePayload::StoreAck(ack) => {
            if out_buf.len() < cursor + 1 {
                return -1;
            }
            out_buf[cursor] = ack.status;
            cursor += 1;
        }
        CMessagePayload::FindNode(node) => {
            if out_buf.len() < cursor + ID_SIZE {
                return -1;
            }
            out_buf[cursor..cursor + ID_SIZE].copy_from_slice(&node.target_id);
            cursor += ID_SIZE;
        }
        CMessagePayload::FindNodeResp(resp) => {
            let count = (resp.count as usize).min(MAX_CONTACTS);
            let contact_size = ID_SIZE + 1 + 16 + 2;
            if out_buf.len() < cursor + 1 + count * contact_size {
                return -1;
            }
            out_buf[cursor] = count as u8;
            cursor += 1;
            for i in 0..count {
                let c = &resp.contacts[i];
                out_buf[cursor..cursor + ID_SIZE].copy_from_slice(&c.id);
                cursor += ID_SIZE;
                out_buf[cursor] = c.ip_type;
                cursor += 1;
                out_buf[cursor..cursor + 16].copy_from_slice(&c.ip);
                cursor += 16;
                out_buf[cursor..cursor + 2].copy_from_slice(&c.port.to_be_bytes());
                cursor += 2;
            }
        }
        CMessagePayload::FindValue(val) => {
            if out_buf.len() < cursor + ID_SIZE {
                return -1;
            }
            out_buf[cursor..cursor + ID_SIZE].copy_from_slice(&val.key);
            cursor += ID_SIZE;
        }
        CMessagePayload::FindValueResp(resp) => {
            if out_buf.len() < cursor + 1 {
                return -1;
            }
            out_buf[cursor] = resp.has_value;
            cursor += 1;
            if resp.has_value != 0 {
                let vlen = (resp.value_len as usize).min(resp.value.len()).min(MAX_VALUE_SIZE);
                if out_buf.len() < cursor + 4 + vlen {
                    return -1;
                }
                out_buf[cursor..cursor + 4].copy_from_slice(&(vlen as u32).to_be_bytes());
                cursor += 4;
                out_buf[cursor..cursor + vlen].copy_from_slice(&resp.value[..vlen]);
                cursor += vlen;
            }
            let count = (resp.count as usize).min(MAX_CONTACTS);
            let contact_size = ID_SIZE + 1 + 16 + 2;
            if out_buf.len() < cursor + 1 + count * contact_size {
                return -1;
            }
            out_buf[cursor] = count as u8;
            cursor += 1;
            for i in 0..count {
                let c = &resp.contacts[i];
                out_buf[cursor..cursor + ID_SIZE].copy_from_slice(&c.id);
                cursor += ID_SIZE;
                out_buf[cursor] = c.ip_type;
                cursor += 1;
                out_buf[cursor..cursor + 16].copy_from_slice(&c.ip);
                cursor += 16;
                out_buf[cursor..cursor + 2].copy_from_slice(&c.port.to_be_bytes());
                cursor += 2;
            }
        }
    }

    cursor as i32
}

/// Decode bytes into a CMessage safely. Returns 0 on success, negative on error.
pub fn ffi_decode_message(buf: &[u8], out_msg: &mut CMessage) -> i32 {
    let mut cursor = 0usize;
    if buf.len() < 1 + 8 + ID_SIZE {
        return -1;
    }

    *out_msg = CMessage::default();

    out_msg.msg_type = buf[cursor];
    cursor += 1;

    let mut req_bytes = [0u8; 8];
    req_bytes.copy_from_slice(&buf[cursor..cursor + 8]);
    out_msg.request_id = u64::from_be_bytes(req_bytes);
    cursor += 8;

    out_msg.sender_id.copy_from_slice(&buf[cursor..cursor + ID_SIZE]);
    cursor += ID_SIZE;

    match out_msg.msg_type {
        MSG_PING | MSG_PONG => {
            out_msg.payload = CMessagePayload::None;
            0
        }
        MSG_STORE => {
            if buf.len() < cursor + ID_SIZE + 4 {
                return -1;
            }
            let mut key = [0u8; ID_SIZE];
            key.copy_from_slice(&buf[cursor..cursor + ID_SIZE]);
            cursor += ID_SIZE;

            let mut len_bytes = [0u8; 4];
            len_bytes.copy_from_slice(&buf[cursor..cursor + 4]);
            cursor += 4;
            let vlen = u32::from_be_bytes(len_bytes) as usize;
            if vlen > MAX_VALUE_SIZE || buf.len() < cursor + vlen {
                return -1;
            }

            let value = buf[cursor..cursor + vlen].to_vec();
            out_msg.payload = CMessagePayload::Store(CStorePayload {
                key,
                value_len: vlen as u32,
                value,
            });
            0
        }
        MSG_STORE_ACK => {
            if buf.len() < cursor + 1 {
                return -1;
            }
            out_msg.payload = CMessagePayload::StoreAck(CStoreAckPayload {
                status: buf[cursor],
            });
            0
        }
        MSG_FIND_NODE => {
            if buf.len() < cursor + ID_SIZE {
                return -1;
            }
            let mut target_id = [0u8; ID_SIZE];
            target_id.copy_from_slice(&buf[cursor..cursor + ID_SIZE]);
            out_msg.payload = CMessagePayload::FindNode(CFindNodePayload { target_id });
            0
        }
        MSG_FIND_NODE_RESP => {
            if buf.len() < cursor + 1 {
                return -1;
            }
            let count = (buf[cursor] as usize).min(MAX_CONTACTS);
            cursor += 1;
            let contact_size = ID_SIZE + 1 + 16 + 2;
            if buf.len() < cursor + count * contact_size {
                return -1;
            }
            let mut resp = CFindNodeRespPayload {
                count: count as u8,
                contacts: [CSerializedContact::default(); MAX_CONTACTS],
            };
            for i in 0..count {
                let mut c = CSerializedContact::default();
                c.id.copy_from_slice(&buf[cursor..cursor + ID_SIZE]);
                cursor += ID_SIZE;
                c.ip_type = buf[cursor];
                cursor += 1;
                c.ip.copy_from_slice(&buf[cursor..cursor + 16]);
                cursor += 16;
                let mut port_bytes = [0u8; 2];
                port_bytes.copy_from_slice(&buf[cursor..cursor + 2]);
                c.port = u16::from_be_bytes(port_bytes);
                cursor += 2;
                resp.contacts[i] = c;
            }
            out_msg.payload = CMessagePayload::FindNodeResp(resp);
            0
        }
        MSG_FIND_VALUE => {
            if buf.len() < cursor + ID_SIZE {
                return -1;
            }
            let mut key = [0u8; ID_SIZE];
            key.copy_from_slice(&buf[cursor..cursor + ID_SIZE]);
            out_msg.payload = CMessagePayload::FindValue(CFindValuePayload { key });
            0
        }
        MSG_FIND_VALUE_RESP => {
            if buf.len() < cursor + 1 {
                return -1;
            }
            let has_value = buf[cursor];
            cursor += 1;
            let mut resp = CFindValueRespPayload::default();
            resp.has_value = has_value;
            if has_value != 0 {
                if buf.len() < cursor + 4 {
                    return -1;
                }
                let mut len_bytes = [0u8; 4];
                len_bytes.copy_from_slice(&buf[cursor..cursor + 4]);
                cursor += 4;
                let vlen = u32::from_be_bytes(len_bytes) as usize;
                if vlen > MAX_VALUE_SIZE || buf.len() < cursor + vlen {
                    return -1;
                }
                resp.value_len = vlen as u32;
                resp.value = buf[cursor..cursor + vlen].to_vec();
                cursor += vlen;
            }
            if buf.len() < cursor + 1 {
                return -1;
            }
            let count = (buf[cursor] as usize).min(MAX_CONTACTS);
            cursor += 1;
            let contact_size = ID_SIZE + 1 + 16 + 2;
            if buf.len() < cursor + count * contact_size {
                return -1;
            }
            resp.count = count as u8;
            for i in 0..count {
                let mut c = CSerializedContact::default();
                c.id.copy_from_slice(&buf[cursor..cursor + ID_SIZE]);
                cursor += ID_SIZE;
                c.ip_type = buf[cursor];
                cursor += 1;
                c.ip.copy_from_slice(&buf[cursor..cursor + 16]);
                cursor += 16;
                let mut port_bytes = [0u8; 2];
                port_bytes.copy_from_slice(&buf[cursor..cursor + 2]);
                c.port = u16::from_be_bytes(port_bytes);
                cursor += 2;
                resp.contacts[i] = c;
            }
            out_msg.payload = CMessagePayload::FindValueResp(resp);
            0
        }
        _ => -1,
    }
}


/// Convert a 20-byte node ID to a 40-char hex string.
pub fn ffi_node_id_to_hex(id_bytes: &[u8; ID_SIZE]) -> String {
    id_bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// CRC32 checksum of data.
pub fn ffi_calculate_checksum(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// Generate a 12-word BIP-39 mnemonic from 16 bytes of entropy.
pub fn ffi_mnemonic_generate_12(entropy_16: &[u8; 16]) -> Result<String, String> {
    let mnemonic = bip39::Mnemonic::from_entropy(entropy_16)
        .map_err(|e| format!("BIP39 error: {}", e))?;
    Ok(mnemonic.to_string())
}

/// Generate a 24-word BIP-39 mnemonic from 32 bytes of entropy.
pub fn ffi_mnemonic_generate_24(entropy_32: &[u8; 32]) -> Result<String, String> {
    let mnemonic = bip39::Mnemonic::from_entropy(entropy_32)
        .map_err(|e| format!("BIP39 error: {}", e))?;
    Ok(mnemonic.to_string())
}

/// Convert a 12-word BIP-39 mnemonic into a 32-byte master seed.
pub fn ffi_mnemonic_to_seed(mnemonic_str: &str) -> Result<[u8; 32], String> {
    let mnemonic = bip39::Mnemonic::parse(mnemonic_str.trim())
        .map_err(|e| format!("Invalid mnemonic phrase: {}", e))?;
    let seed_64 = mnemonic.to_seed("");
    let mut seed_32 = [0u8; 32];
    seed_32.copy_from_slice(&seed_64[..32]);
    Ok(seed_32)
}

/// Validate a 12-word mnemonic phrase. Returns true if valid.
pub fn ffi_mnemonic_validate(mnemonic_str: &str) -> bool {
    bip39::Mnemonic::parse(mnemonic_str.trim()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_rust_mnemonic_roundtrip() {
        let entropy = [
            0x1a, 0x2b, 0x3c, 0x4d, 0x5e, 0x6f, 0x70, 0x81,
            0x92, 0xa3, 0xb4, 0xc5, 0xd6, 0xe7, 0xf8, 0x09,
        ];
        let mnemonic = ffi_mnemonic_generate_12(&entropy).expect("Should generate 12 words");
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12, "Must be exactly 12 words: {}", mnemonic);

        assert!(ffi_mnemonic_validate(&mnemonic));

        let seed1 = ffi_mnemonic_to_seed(&mnemonic).expect("Seed derivation 1");
        let seed2 = ffi_mnemonic_to_seed(&mnemonic).expect("Seed derivation 2");
        assert_eq!(seed1, seed2, "Seed derivation must be deterministic");
        assert_ne!(seed1, [0u8; 32]);
    }

    #[test]
    fn test_pure_rust_serialize_roundtrip() {
        let mut msg = CMessage::default();
        msg.msg_type = MSG_PING;
        msg.request_id = 42;
        msg.sender_id = [7u8; ID_SIZE];

        let mut buf = [0u8; 1024];
        let bytes_written = ffi_encode_message(&msg, &mut buf);
        assert!(bytes_written > 0);

        let mut decoded = CMessage::default();
        let decode_res = ffi_decode_message(&buf[..bytes_written as usize], &mut decoded);
        assert_eq!(decode_res, 0);
        assert_eq!(decoded.msg_type, MSG_PING);
        assert_eq!(decoded.request_id, 42);
        assert_eq!(decoded.sender_id, [7u8; ID_SIZE]);
    }

    #[test]
    fn test_store_and_find_node_roundtrip() {
        let mut msg = CMessage::default();
        msg.msg_type = MSG_STORE;
        msg.request_id = 1001;
        msg.sender_id = [3u8; ID_SIZE];
        let val_bytes = b"Hello NodeX Secure Mesh Protocol";
        msg.payload = CMessagePayload::Store(CStorePayload {
            key: [5u8; ID_SIZE],
            value_len: val_bytes.len() as u32,
            value: val_bytes.to_vec(),
        });

        let mut buf = vec![0u8; 2048];
        let bytes_written = ffi_encode_message(&msg, &mut buf);
        assert!(bytes_written > 0);

        let mut decoded = CMessage::default();
        let decode_res = ffi_decode_message(&buf[..bytes_written as usize], &mut decoded);
        assert_eq!(decode_res, 0);
        assert_eq!(decoded.msg_type, MSG_STORE);
        if let CMessagePayload::Store(st) = decoded.payload {
            assert_eq!(st.key, [5u8; ID_SIZE]);
            assert_eq!(&st.value, val_bytes);
        } else {
            panic!("Expected Store payload");
        }
    }
}


