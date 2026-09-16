//! Safe wrappers over the C FFI layer (core/*.c).
//!
//! All `unsafe` FFI calls are isolated in this crate.
//! Downstream crates (`nodex-kademlia`, `nodex-messenger`) use only the safe `pub fn` wrappers.

pub const ID_SIZE: usize = 20;
pub const MAX_VALUE_SIZE: usize = 65536;
pub const MAX_CONTACTS: usize = 20;

// ── Raw C FFI declarations ──────────────────────────────────────────────────

// Message type constants matching serialize.h
pub const MSG_PING: u8 = 1;
pub const MSG_PONG: u8 = 2;
pub const MSG_STORE: u8 = 3;
pub const MSG_STORE_ACK: u8 = 4;
pub const MSG_FIND_NODE: u8 = 5;
pub const MSG_FIND_NODE_RESP: u8 = 6;
pub const MSG_FIND_VALUE: u8 = 7;
pub const MSG_FIND_VALUE_RESP: u8 = 8;

// ── C repr structs for message serialization ────────────────────────────────

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CSerializedContact {
    pub id: [u8; ID_SIZE],
    pub ip_type: u8,
    pub ip: [u8; 16],
    pub port: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CStorePayload {
    pub key: [u8; ID_SIZE],
    pub value_len: u32,
    pub value: [u8; MAX_VALUE_SIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CStoreAckPayload {
    pub status: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CFindNodePayload {
    pub target_id: [u8; ID_SIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CFindNodeRespPayload {
    pub count: u8,
    pub contacts: [CSerializedContact; MAX_CONTACTS],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CFindValuePayload {
    pub key: [u8; ID_SIZE],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CFindValueRespPayload {
    pub has_value: u8,
    pub value_len: u32,
    pub value: [u8; MAX_VALUE_SIZE],
    pub count: u8,
    pub contacts: [CSerializedContact; MAX_CONTACTS],
}

#[repr(C)]
pub union CMessagePayload {
    pub store: CStorePayload,
    pub store_ack: CStoreAckPayload,
    pub find_node: CFindNodePayload,
    pub find_node_resp: CFindNodeRespPayload,
    pub find_value: CFindValuePayload,
    pub find_value_resp: CFindValueRespPayload,
}

#[repr(C)]
pub struct CMessage {
    pub msg_type: u8,
    pub request_id: u64,
    pub sender_id: [u8; ID_SIZE],
    pub payload: CMessagePayload,
}

// ── Raw extern "C" declarations ─────────────────────────────────────────────

extern "C" {
    // hash.c
    fn c_shared_prefix_bits(id1: *const u8, id2: *const u8, len: usize) -> u32;
    fn hash_node_id(input: *const u8, len: usize, out_id: *mut u8);

    // serialize.c
    fn encode_message(msg: *const CMessage, out_buf: *mut u8, buf_len: usize) -> i32;
    fn decode_message(buf: *const u8, len: usize, out_msg: *mut CMessage) -> i32;

    // messenger_core.c
    fn c_messenger_derive_mailbox_key(
        user_id_bytes: *const u8,
        timestamp: u64,
        out_key_buf: *mut u8,
        buf_len: usize,
    ) -> i32;
    fn c_node_id_to_hex(id_bytes: *const u8, out_hex_buf: *mut u8);
    fn c_messenger_calculate_checksum(data: *const u8, len: usize) -> u32;

    // mnemonic.c
    fn c_mnemonic_generate_12(entropy_16: *const u8, out_words_buf: *mut u8, buf_len: usize) -> i32;
    fn c_mnemonic_to_seed(mnemonic_str: *const std::os::raw::c_char, out_seed_32: *mut u8) -> i32;
    fn c_mnemonic_validate(mnemonic_str: *const std::os::raw::c_char) -> i32;
}

// ── Safe public wrappers ────────────────────────────────────────────────────

/// SHA-1 hash of `input`, returns a 20-byte node ID.
pub fn ffi_hash_node_id(input: &[u8]) -> [u8; ID_SIZE] {
    let mut out = [0u8; ID_SIZE];
    unsafe {
        hash_node_id(input.as_ptr(), input.len(), out.as_mut_ptr());
    }
    out
}

/// Count of shared prefix bits between two 20-byte IDs.
pub fn ffi_shared_prefix_bits(id1: &[u8; ID_SIZE], id2: &[u8; ID_SIZE]) -> u32 {
    unsafe { c_shared_prefix_bits(id1.as_ptr(), id2.as_ptr(), ID_SIZE) }
}

/// Encode a CMessage into a byte buffer. Returns number of bytes written, or negative on error.
pub fn ffi_encode_message(msg: &CMessage, out_buf: &mut [u8]) -> i32 {
    unsafe { encode_message(msg as *const CMessage, out_buf.as_mut_ptr(), out_buf.len()) }
}

/// Decode bytes into a CMessage. Returns 0 on success, negative on error.
///
/// # Safety contract
/// The caller must ensure `out_msg` is zeroed before calling (use `std::mem::zeroed()`).
pub fn ffi_decode_message(buf: &[u8], out_msg: &mut CMessage) -> i32 {
    unsafe { decode_message(buf.as_ptr(), buf.len(), out_msg as *mut CMessage) }
}

/// Derive a mailbox key string from a user ID and timestamp.
pub fn ffi_derive_mailbox_key(user_id_bytes: &[u8; ID_SIZE], timestamp: u64) -> Option<String> {
    let mut buf = [0u8; 128];
    let res = unsafe {
        c_messenger_derive_mailbox_key(
            user_id_bytes.as_ptr(),
            timestamp,
            buf.as_mut_ptr(),
            buf.len(),
        )
    };
    if res > 0 {
        std::str::from_utf8(&buf[..res as usize])
            .ok()
            .map(|s| s.to_string())
    } else {
        None
    }
}

/// Convert a 20-byte node ID to a 40-char hex string.
pub fn ffi_node_id_to_hex(id_bytes: &[u8; ID_SIZE]) -> String {
    let mut buf = [0u8; 41]; // 40 hex chars + null terminator
    unsafe {
        c_node_id_to_hex(id_bytes.as_ptr(), buf.as_mut_ptr());
    }
    std::str::from_utf8(&buf[..40])
        .unwrap_or_default()
        .to_string()
}

/// CRC32 checksum of data.
pub fn ffi_calculate_checksum(data: &[u8]) -> u32 {
    unsafe { c_messenger_calculate_checksum(data.as_ptr(), data.len()) }
}

/// Generate a 12-word BIP-39 mnemonic from 16 bytes of entropy.
pub fn ffi_mnemonic_generate_12(entropy_16: &[u8; 16]) -> Result<String, String> {
    let mut buf = [0u8; 256];
    let res = unsafe {
        c_mnemonic_generate_12(entropy_16.as_ptr(), buf.as_mut_ptr(), buf.len())
    };
    if res > 0 {
        std::str::from_utf8(&buf[..res as usize])
            .map(|s| s.to_string())
            .map_err(|e| format!("Invalid utf8 mnemonic: {}", e))
    } else {
        Err(format!("Mnemonic generation failed with code {}", res))
    }
}

/// Convert a 12-word BIP-39 mnemonic into a 32-byte master seed.
pub fn ffi_mnemonic_to_seed(mnemonic_str: &str) -> Result<[u8; 32], String> {
    let c_str = std::ffi::CString::new(mnemonic_str).map_err(|e| e.to_string())?;
    let mut out_seed = [0u8; 32];
    let res = unsafe {
        c_mnemonic_to_seed(c_str.as_ptr(), out_seed.as_mut_ptr())
    };
    if res == 0 {
        Ok(out_seed)
    } else {
        Err(format!("Invalid mnemonic phrase (code {})", res))
    }
}

/// Validate a 12-word mnemonic phrase. Returns true if valid.
pub fn ffi_mnemonic_validate(mnemonic_str: &str) -> bool {
    let Ok(c_str) = std::ffi::CString::new(mnemonic_str) else {
        return false;
    };
    unsafe { c_mnemonic_validate(c_str.as_ptr()) == 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_c_mnemonic_roundtrip() {
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
}

