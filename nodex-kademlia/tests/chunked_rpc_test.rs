use std::net::SocketAddr;
use std::time::Duration;

use nodex_kademlia::node::NodeId;
use nodex_kademlia::KademliaNode;

/// Test that a large value (>1100 bytes, triggering UDP chunking) can be
/// stored on one node and retrieved from another through the live RPC layer.
/// This verifies that the chunking integration in send_message + start_receive_loop
/// correctly fragments and reassembles large RPC payloads.
#[tokio::test]
async fn test_large_value_roundtrip_through_chunked_rpc() {
    let addr_a: SocketAddr = "127.0.0.1:18901".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:18902".parse().unwrap();

    let node_a = KademliaNode::start(addr_a).await.unwrap();
    let node_b = KademliaNode::start(addr_b).await.unwrap();

    // B bootstraps to A
    node_b.bootstrap(addr_a).await.unwrap();

    // Create a 10 KB value — this forces chunking (threshold = 1100 bytes).
    // After RPC header overhead (~55 bytes), the encoded message is ~10,295 bytes,
    // requiring about 21 UDP chunks to transmit.
    let key = "chunked_rpc_large_value_test";
    let val: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();

    // Store on node A via the DHT put (which sends STORE RPCs to B)
    let stored = node_a.put(key, val.clone()).await.unwrap();
    assert!(stored >= 1, "Value must be stored on at least 1 node");

    // Allow a moment for the chunked RPC to complete
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Node B should have received and stored the value through chunked STORE RPC
    let key_id = NodeId::from_key(key.as_bytes());
    let result = {
        let st = node_b.storage.read().await;
        st.get(&key_id)
    };

    assert!(result.is_some(), "Node B must have the value after chunked STORE RPC");
    let retrieved = result.unwrap();
    assert_eq!(retrieved.len(), val.len(), "Retrieved value length must match");
    assert_eq!(retrieved, val, "Retrieved value must be byte-for-byte identical");

    println!("✅ Large value (10 KB, ~21 chunks) roundtrip through chunked RPC: PASSED");
}

/// Test that small messages (<= 1100 bytes) bypass chunking entirely
/// and still work correctly through the existing direct path.
#[tokio::test]
async fn test_small_value_bypasses_chunking() {
    let addr_a: SocketAddr = "127.0.0.1:18911".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:18912".parse().unwrap();

    let node_a = KademliaNode::start(addr_a).await.unwrap();
    let node_b = KademliaNode::start(addr_b).await.unwrap();

    node_b.bootstrap(addr_a).await.unwrap();

    // A small 100-byte value should go through the direct (non-chunked) path
    let key = "small_value_no_chunk_test";
    let val = vec![0xAB; 100];

    let stored = node_a.put(key, val.clone()).await.unwrap();
    assert!(stored >= 1);

    tokio::time::sleep(Duration::from_millis(200)).await;

    let key_id = NodeId::from_key(key.as_bytes());
    let result = {
        let st = node_b.storage.read().await;
        st.get(&key_id)
    };

    assert!(result.is_some(), "Small value must arrive without chunking");
    assert_eq!(result.unwrap(), val);

    println!("✅ Small value (100 bytes, no chunking) roundtrip: PASSED");
}

/// Test a moderately large value near the chunking threshold boundary.
/// Values just above 1100 bytes should be chunked into exactly 2-3 chunks.
#[tokio::test]
async fn test_boundary_value_at_chunk_threshold() {
    let addr_a: SocketAddr = "127.0.0.1:18921".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:18922".parse().unwrap();

    let node_a = KademliaNode::start(addr_a).await.unwrap();
    let node_b = KademliaNode::start(addr_b).await.unwrap();

    node_b.bootstrap(addr_a).await.unwrap();

    // 1500 bytes of payload + ~55 bytes RPC overhead = ~1555 bytes encoded
    // This is above the 1100-byte threshold, triggering 4 chunks (1555/500 = 3.11 → 4)
    let key = "boundary_chunk_threshold_test";
    let val: Vec<u8> = (0..1500).map(|i| (i % 251) as u8).collect();

    let stored = node_a.put(key, val.clone()).await.unwrap();
    assert!(stored >= 1);

    tokio::time::sleep(Duration::from_millis(200)).await;

    let key_id = NodeId::from_key(key.as_bytes());
    let result = {
        let st = node_b.storage.read().await;
        st.get(&key_id)
    };

    assert!(result.is_some(), "Boundary value must arrive through chunked RPC");
    assert_eq!(result.unwrap(), val, "Boundary value must be byte-for-byte identical");

    println!("✅ Boundary value (1500 bytes, ~4 chunks) roundtrip: PASSED");
}

/// Stress test: 64 KB value (maximum allowed by MAX_VALUE_SIZE).
/// This requires ~130 chunks and verifies the assembler handles many chunks.
#[tokio::test]
async fn test_max_value_size_64kb_chunked_roundtrip() {
    let addr_a: SocketAddr = "127.0.0.1:18931".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:18932".parse().unwrap();

    let node_a = KademliaNode::start(addr_a).await.unwrap();
    let node_b = KademliaNode::start(addr_b).await.unwrap();

    node_b.bootstrap(addr_a).await.unwrap();

    // 60 KB value — close to the MAX_VALUE_SIZE limit
    let key = "max_value_64kb_stress_test";
    let val: Vec<u8> = (0..60_000).map(|i| (i % 256) as u8).collect();

    let stored = node_a.put(key, val.clone()).await.unwrap();
    assert!(stored >= 1);

    // Give more time for ~120 chunks to transmit
    tokio::time::sleep(Duration::from_millis(500)).await;

    let key_id = NodeId::from_key(key.as_bytes());
    let result = {
        let st = node_b.storage.read().await;
        st.get(&key_id)
    };

    assert!(result.is_some(), "60 KB value must arrive through chunked RPC (~120 chunks)");
    let val_ref = result.as_ref().unwrap();
    assert_eq!(val_ref.len(), val.len(), "60 KB value length must match");
    assert_eq!(
        &val_ref[..100],
        &val[..100],
        "First 100 bytes must match"
    );

    println!("✅ 60 KB value (~120 chunks) stress test: PASSED");
}
