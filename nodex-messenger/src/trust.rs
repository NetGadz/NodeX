pub struct TrustManager;

impl TrustManager {
    pub fn compute_key_fingerprint(public_key_bytes: &[u8]) -> String {
        let hash = core_ffi::ffi_hash_node_id(public_key_bytes);
        let hex = core_ffi::ffi_node_id_to_hex(&hash);
        let chunks: Vec<&str> = (0..hex.len()).step_by(4).map(|i| &hex[i..i + 4]).collect();
        chunks.join(":")
    }

    pub fn verify_key_unchanged(cached_key: &[u8], received_key: &[u8]) -> bool {
        cached_key == received_key
    }
}
