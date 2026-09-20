use base64::prelude::*;
use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use crate::errors::MessengerError;

/// Maximum single file size allowed for transfer (100 MB).
pub const MAX_MEDIA_FILE_BYTES: usize = 100 * 1024 * 1024;

/// Standard chunk size for parallel peer media transport (32 KB).
pub const MEDIA_CHUNK_SIZE: usize = 32 * 1024;

/// Legacy inline attachment limit (24 KB).
pub const MAX_LEGACY_INLINE_BYTES: usize = 24576;

/// Metadata manifest for large media and document transfers.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileManifest {
    pub file_id: String,
    pub file_name: String,
    pub file_size: usize,
    pub mime_type: String,
    pub sha256_root: String,
    pub chunk_size: usize,
    pub chunk_count: usize,
    pub chunk_hashes: Vec<String>,
    pub encryption_key_hex: String,
    #[serde(default)]
    pub thumbnail_base64: Option<String>,
}

/// Encrypted individual chunk of a large media transfer.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaChunk {
    pub file_id: String,
    pub chunk_index: usize,
    pub chunk_count: usize,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub checksum: u32,
}

pub struct AttachmentManager;

impl AttachmentManager {
    /// Create an encrypted FileManifest and chunk list from raw file bytes.
    pub fn create_file_transfer(
        file_name: &str,
        raw_bytes: &[u8],
        thumbnail_opt: Option<Vec<u8>>,
    ) -> Result<(FileManifest, Vec<MediaChunk>), MessengerError> {
        if raw_bytes.len() > MAX_MEDIA_FILE_BYTES {
            return Err(MessengerError::AttachmentTooLarge(raw_bytes.len(), MAX_MEDIA_FILE_BYTES));
        }

        let file_id = format!("{:x}_{}", rand::random::<u64>(), raw_bytes.len());
        let sha256_root = format!("{:x}", Sha256::digest(raw_bytes));

        let mut key_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut key_bytes);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));

        let chunk_count = raw_bytes.len().div_ceil(MEDIA_CHUNK_SIZE);
        let mut chunks = Vec::with_capacity(chunk_count);
        let mut chunk_hashes = Vec::with_capacity(chunk_count);

        for (idx, slice) in raw_bytes.chunks(MEDIA_CHUNK_SIZE).enumerate() {
            let chunk_hash = format!("{:x}", Sha256::digest(slice));
            chunk_hashes.push(chunk_hash);

            let mut nonce_bytes = [0u8; 12];
            rand::thread_rng().fill_bytes(&mut nonce_bytes);
            let nonce = Nonce::from_slice(&nonce_bytes);

            let ciphertext = cipher
                .encrypt(nonce, slice)
                .map_err(|e| MessengerError::CryptoError(format!("Chunk encryption failed: {}", e)))?;

            let checksum = crc32fast::hash(&ciphertext);

            chunks.push(MediaChunk {
                file_id: file_id.clone(),
                chunk_index: idx,
                chunk_count,
                nonce: nonce_bytes.to_vec(),
                ciphertext,
                checksum,
            });
        }

        let thumbnail_base64 = thumbnail_opt.map(|t| BASE64_STANDARD.encode(t));
        let mime_type = guess_mime_type(file_name);

        let manifest = FileManifest {
            file_id,
            file_name: file_name.to_string(),
            file_size: raw_bytes.len(),
            mime_type,
            sha256_root,
            chunk_size: MEDIA_CHUNK_SIZE,
            chunk_count,
            chunk_hashes,
            encryption_key_hex: hex_encode(&key_bytes),
            thumbnail_base64,
        };

        Ok((manifest, chunks))
    }

    /// Reassemble and decrypt all chunks according to the manifest with strict SHA-256 verification.
    pub fn assemble_file_transfer(
        manifest: &FileManifest,
        chunks: &[MediaChunk],
    ) -> Result<Vec<u8>, MessengerError> {
        if chunks.len() != manifest.chunk_count {
            return Err(MessengerError::Other(format!(
                "Incomplete transfer: received {}/{} chunks",
                chunks.len(),
                manifest.chunk_count
            )));
        }

        let key_bytes = hex_decode(&manifest.encryption_key_hex)
            .ok_or_else(|| MessengerError::CryptoError("Invalid manifest encryption key hex".into()))?;
        if key_bytes.len() != 32 {
            return Err(MessengerError::CryptoError("Manifest encryption key must be 32 bytes".into()));
        }
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key_bytes));

        let mut sorted_chunks = chunks.to_vec();
        sorted_chunks.sort_by_key(|c| c.chunk_index);

        let mut assembled_bytes = Vec::with_capacity(manifest.file_size);

        for (expected_idx, chunk) in sorted_chunks.iter().enumerate() {
            if chunk.chunk_index != expected_idx {
                return Err(MessengerError::Other(format!(
                    "Missing chunk index {}",
                    expected_idx
                )));
            }

            // Verify CRC32 checksum of chunk
            if crc32fast::hash(&chunk.ciphertext) != chunk.checksum {
                return Err(MessengerError::CryptoError(format!(
                    "CRC32 checksum mismatch on chunk {}",
                    chunk.chunk_index
                )));
            }

            let nonce = Nonce::from_slice(&chunk.nonce);
            let plaintext_slice = cipher
                .decrypt(nonce, chunk.ciphertext.as_slice())
                .map_err(|e| MessengerError::CryptoError(format!("Chunk decrypt failed: {}", e)))?;

            // Verify chunk hash matches manifest
            let actual_hash = format!("{:x}", Sha256::digest(&plaintext_slice));
            if actual_hash != manifest.chunk_hashes[expected_idx] {
                return Err(MessengerError::CryptoError(format!(
                    "SHA-256 mismatch on chunk {}",
                    expected_idx
                )));
            }

            assembled_bytes.extend_from_slice(&plaintext_slice);
        }

        // Verify root SHA-256 hash of entire reconstructed file
        let actual_root = format!("{:x}", Sha256::digest(&assembled_bytes));
        if actual_root != manifest.sha256_root {
            return Err(MessengerError::CryptoError("Root SHA-256 file hash mismatch".into()));
        }

        Ok(assembled_bytes)
    }

    /// Legacy inline attachment encoder.
    pub fn encode_attachment(file_name: &str, raw_bytes: &[u8]) -> Result<String, MessengerError> {
        if raw_bytes.len() > MAX_LEGACY_INLINE_BYTES {
            return Err(MessengerError::AttachmentTooLarge(raw_bytes.len(), MAX_LEGACY_INLINE_BYTES));
        }
        let b64 = BASE64_STANDARD.encode(raw_bytes);
        println!("[ATTACHMENT] Encoded inline attachment '{}' ({} bytes)", file_name, raw_bytes.len());
        Ok(b64)
    }

    /// Legacy inline attachment decoder.
    pub fn decode_attachment(base64_str: &str) -> Result<Vec<u8>, MessengerError> {
        BASE64_STANDARD
            .decode(base64_str)
            .map_err(|e| MessengerError::CryptoError(format!("Base64 decode error: {}", e)))
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn guess_mime_type(file_name: &str) -> String {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png".into()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".into()
    } else if lower.ends_with(".gif") {
        "image/gif".into()
    } else if lower.ends_with(".webp") {
        "image/webp".into()
    } else if lower.ends_with(".pdf") {
        "application/pdf".into()
    } else if lower.ends_with(".zip") {
        "application/zip".into()
    } else {
        "application/octet-stream".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_large_media_chunking_and_reassembly() {
        let fake_image: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        let (manifest, chunks) = AttachmentManager::create_file_transfer("photo.jpg", &fake_image, None).unwrap();

        assert_eq!(manifest.file_name, "photo.jpg");
        assert_eq!(manifest.file_size, 100_000);
        assert_eq!(manifest.chunk_count, 4);
        assert_eq!(chunks.len(), 4);

        let recovered = AttachmentManager::assemble_file_transfer(&manifest, &chunks).unwrap();
        assert_eq!(recovered.len(), fake_image.len());
        assert_eq!(recovered, fake_image);
    }
}

