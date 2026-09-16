use base64::prelude::*;
use crate::errors::MessengerError;

// The attachment is later base64-encoded and wrapped in an encrypted JSON
// envelope, so the raw limit must stay below the RPC value limit.
pub const MAX_ATTACHMENT_BYTES: usize = 24576;

pub struct AttachmentManager;

impl AttachmentManager {
    pub fn encode_attachment(file_name: &str, raw_bytes: &[u8]) -> Result<String, MessengerError> {
        if raw_bytes.len() > MAX_ATTACHMENT_BYTES {
            return Err(MessengerError::AttachmentTooLarge(raw_bytes.len(), MAX_ATTACHMENT_BYTES));
        }
        let b64 = BASE64_STANDARD.encode(raw_bytes);
        println!("[ATTACHMENT] Encoded attachment '{}' ({} bytes)", file_name, raw_bytes.len());
        Ok(b64)
    }

    pub fn decode_attachment(base64_str: &str) -> Result<Vec<u8>, MessengerError> {
        BASE64_STANDARD
            .decode(base64_str)
            .map_err(|e| MessengerError::CryptoError(format!("Base64 decode error: {}", e)))
    }
}
