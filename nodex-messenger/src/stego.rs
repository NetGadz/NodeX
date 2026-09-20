use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use image::{DynamicImage, GenericImageView, ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use sha2::{Digest, Sha256};

pub const STEGO_MAGIC: &[u8; 6] = b"NXSTG1";

/// Contact card data embedded stealthily inside images
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StegoContactCard {
    pub user_id_hex: String,
    pub display_name: String,
    pub bio: String,
    pub ed25519_pub_hex: String,
    pub x25519_pub_hex: String,
    pub endpoints: Vec<SocketAddr>,
    pub timestamp: u64,
    pub signature_hex: String,
}

pub struct StegoCarrier;

impl StegoCarrier {
    /// Derives an encryption key from an optional passphrase (or default NodeX public salt).
    fn derive_key(passphrase: Option<&str>, salt: &[u8; 32]) -> Key {
        let mut hasher = Sha256::new();
        hasher.update(b"NODEX_STEGO_CARRIER_KEY_V1");
        hasher.update(salt);
        if let Some(p) = passphrase {
            hasher.update(p.as_bytes());
        }
        let hash = hasher.finalize();
        *Key::from_slice(&hash)
    }

    /// Embeds a contact card into an image using LSB (Least Significant Bit) steganography.
    /// Returns the new PNG bytes with embedded hidden data.
    pub fn embed_contact_into_image(
        input_image: &DynamicImage,
        contact: &StegoContactCard,
        passphrase: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let json_bytes = serde_json::to_vec(contact)
            .map_err(|e| format!("Failed to serialize stego contact: {}", e))?;

        let mut salt = [0u8; 32];
        let mut nonce_bytes = [0u8; 12];
        getrandom::getrandom(&mut salt).map_err(|e| e.to_string())?;
        getrandom::getrandom(&mut nonce_bytes).map_err(|e| e.to_string())?;

        let key = Self::derive_key(passphrase, &salt);
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, json_bytes.as_ref())
            .map_err(|e| format!("Stego encryption failed: {:?}", e))?;

        // Format: [MAGIC (6)] [PAYLOAD_LEN (4)] [SALT (32)] [NONCE (12)] [CIPHERTEXT (N)] [CRC32 (4)]
        let payload_len = ciphertext.len() as u32;
        let mut raw_payload = Vec::with_capacity(6 + 4 + 32 + 12 + ciphertext.len() + 4);
        raw_payload.extend_from_slice(STEGO_MAGIC);
        raw_payload.extend_from_slice(&payload_len.to_be_bytes());
        raw_payload.extend_from_slice(&salt);
        raw_payload.extend_from_slice(&nonce_bytes);
        raw_payload.extend_from_slice(&ciphertext);

        let mut hasher = crc32fast::Hasher::new();
        hasher.update(&raw_payload);
        let crc = hasher.finalize();
        raw_payload.extend_from_slice(&crc.to_be_bytes());

        // Total bits required: raw_payload.len() * 8
        let total_bits_needed = raw_payload.len() * 8;
        let (width, height) = input_image.dimensions();
        let total_pixels = (width as usize) * (height as usize);
        let available_bits = total_pixels * 3; // R, G, B channels

        if total_bits_needed > available_bits {
            return Err(format!(
                "Image too small for stego payload. Need at least {} pixels, image has {}.",
                (total_bits_needed + 2) / 3,
                total_pixels
            ));
        }

        let mut rgba_img: ImageBuffer<Rgba<u8>, Vec<u8>> = input_image.to_rgba8();

        let mut bit_idx = 0;
        let _total_bytes = raw_payload.len();

        'outer: for y in 0..height {
            for x in 0..width {
                let pixel = rgba_img.get_pixel_mut(x, y);
                for c in 0..3 {
                    // Channels 0 (R), 1 (G), 2 (B)
                    if bit_idx >= total_bits_needed {
                        break 'outer;
                    }
                    let byte_idx = bit_idx / 8;
                    let bit_offset = 7 - (bit_idx % 8);
                    let bit = (raw_payload[byte_idx] >> bit_offset) & 1;

                    // Clear LSB and insert payload bit
                    pixel[c] = (pixel[c] & 0xFE) | bit;
                    bit_idx += 1;
                }
            }
        }

        // Encode result as PNG
        let mut png_bytes = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut png_bytes);
        image::ImageEncoder::write_image(
            encoder,
            &rgba_img,
            width,
            height,
            image::ExtendedColorType::Rgba8,
        )
        .map_err(|e| format!("Failed to encode output PNG: {}", e))?;

        Ok(png_bytes)
    }

    /// Extracts and decrypts a contact card from an image containing hidden LSB data.
    pub fn extract_contact_from_image(
        image_bytes: &[u8],
        passphrase: Option<&str>,
    ) -> Result<StegoContactCard, String> {
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| format!("Failed to read image bytes: {}", e))?;

        let (width, height) = img.dimensions();
        let rgba_img = img.to_rgba8();

        // 1. Extract first 10 bytes (80 bits) to check magic (6) and payload_len (4)
        let header_bits_needed = 10 * 8;
        let mut header_bytes = [0u8; 10];
        let mut bit_idx = 0;

        'hdr: for y in 0..height {
            for x in 0..width {
                let pixel = rgba_img.get_pixel(x, y);
                for c in 0..3 {
                    if bit_idx >= header_bits_needed {
                        break 'hdr;
                    }
                    let byte_idx = bit_idx / 8;
                    let bit_offset = 7 - (bit_idx % 8);
                    let bit = pixel[c] & 1;
                    header_bytes[byte_idx] |= bit << bit_offset;
                    bit_idx += 1;
                }
            }
        }

        if &header_bytes[0..6] != STEGO_MAGIC {
            return Err("No NodeX Stego-Carrier signature found in image".to_string());
        }

        let payload_len = u32::from_be_bytes([
            header_bytes[6],
            header_bytes[7],
            header_bytes[8],
            header_bytes[9],
        ]) as usize;

        if payload_len > 1_000_000 {
            return Err("Corrupted stego payload length".to_string());
        }

        // Full packet size: 6 (magic) + 4 (len) + 32 (salt) + 12 (nonce) + payload_len + 4 (crc)
        let total_packet_size = 6 + 4 + 32 + 12 + payload_len + 4;
        let total_bits = total_packet_size * 8;

        let total_pixels = (width as usize) * (height as usize);
        if total_bits > total_pixels * 3 {
            return Err("Stego payload exceeds image capacity".to_string());
        }

        let mut full_payload = vec![0u8; total_packet_size];
        bit_idx = 0;

        'full: for y in 0..height {
            for x in 0..width {
                let pixel = rgba_img.get_pixel(x, y);
                for c in 0..3 {
                    if bit_idx >= total_bits {
                        break 'full;
                    }
                    let byte_idx = bit_idx / 8;
                    let bit_offset = 7 - (bit_idx % 8);
                    let bit = pixel[c] & 1;
                    full_payload[byte_idx] |= bit << bit_offset;
                    bit_idx += 1;
                }
            }
        }

        // Verify CRC32
        let data_part = &full_payload[..total_packet_size - 4];
        let expected_crc = u32::from_be_bytes([
            full_payload[total_packet_size - 4],
            full_payload[total_packet_size - 3],
            full_payload[total_packet_size - 2],
            full_payload[total_packet_size - 1],
        ]);
        let mut hasher = crc32fast::Hasher::new();
        hasher.update(data_part);
        let actual_crc = hasher.finalize();

        if actual_crc != expected_crc {
            return Err("Stego carrier CRC32 checksum mismatch (image may be compressed or modified)".to_string());
        }

        let salt: [u8; 32] = full_payload[10..42].try_into().unwrap();
        let nonce_bytes: [u8; 12] = full_payload[42..54].try_into().unwrap();
        let ciphertext = &full_payload[54..54 + payload_len];

        let key = Self::derive_key(passphrase, &salt);
        let cipher = ChaCha20Poly1305::new(&key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "Stego decryption failed (invalid passphrase or corrupted key)".to_string())?;

        let contact: StegoContactCard = serde_json::from_slice(&plaintext)
            .map_err(|e| format!("Failed to parse decrypted stego contact: {}", e))?;

        Ok(contact)
    }
}
