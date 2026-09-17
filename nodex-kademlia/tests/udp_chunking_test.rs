use nodex_kademlia::chunking::{ChunkAssembler, UdpChunk, MAX_UDP_PACKET_SIZE};
use std::time::Duration;

#[test]
fn test_chunk_splitting_respects_max_packet_size() {
    let msg_id = [7u8; 16];
    let original_data = vec![0xAB; 4500]; // 4.5 KB payload

    let chunks = UdpChunk::split_data(msg_id, &original_data);
    assert_eq!(chunks.len(), 9);

    for (i, chunk) in chunks.iter().enumerate() {
        assert_eq!(chunk.message_id, msg_id);
        assert_eq!(chunk.chunk_index, i as u16);
        assert_eq!(chunk.chunk_count, 9);

        // Serialize to verify it strictly fits in MAX_UDP_PACKET_SIZE
        let json_bytes = serde_json::to_vec(chunk).expect("Serialize chunk");
        assert!(
            json_bytes.len() <= MAX_UDP_PACKET_SIZE,
            "Serialized chunk size {} exceeds 1200 bytes",
            json_bytes.len()
        );
    }
}

#[test]
fn test_chunk_assembly_and_checksum_verification() {
    let msg_id = [42u8; 16];
    let payload = b"Hello world! This is a long encrypted message sent via NodeX P2P transport.";

    let chunks = UdpChunk::split_data(msg_id, payload);
    let mut assembler = ChunkAssembler::new(Duration::from_secs(5));

    let mut completed = None;
    for chunk in chunks {
        if let Some(res) = assembler.add_chunk(chunk) {
            completed = Some(res);
        }
    }

    assert_eq!(completed, Some(payload.to_vec()));
}

#[test]
fn test_corrupted_chunk_rejected() {
    let msg_id = [99u8; 16];
    let payload = vec![0x12; 2000];

    let mut chunks = UdpChunk::split_data(msg_id, &payload);
    assert!(chunks.len() >= 2);

    // Corrupt one hex character of chunk 0
    let mut chars: Vec<char> = chunks[0].payload_hex.chars().collect();
    chars[0] = if chars[0] == '0' { '1' } else { '0' };
    chunks[0].payload_hex = chars.into_iter().collect();

    let mut assembler = ChunkAssembler::new(Duration::from_secs(5));
    assert!(assembler.add_chunk(chunks[0].clone()).is_none());
}
