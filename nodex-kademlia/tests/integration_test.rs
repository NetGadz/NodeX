use std::net::SocketAddr;
use std::time::Duration;

use nodex_kademlia::node::{Contact, NodeId};
use nodex_kademlia::storage::DEFAULT_TTL;
use nodex_kademlia::KademliaNode;


#[tokio::test]
async fn test_find_value_over_network() {
    let addr_a: SocketAddr = "127.0.0.1:17001".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:17002".parse().unwrap();

    let node_a = KademliaNode::start(addr_a).await.unwrap();
    let node_b = KademliaNode::start(addr_b).await.unwrap();

    // B bootstraps to A
    node_b.bootstrap(addr_a).await.unwrap();

    // Store the value ONLY on node A (directly into its storage, no replication)
    let key = "find_value_test_key";
    let val = b"This value exists only on node A".to_vec();
    {
        let key_id = NodeId::from_key(key.as_bytes());
        let mut st = node_a.storage.write().await;
        st.put(key_id, val.clone(), DEFAULT_TTL);
    }

    // Verify: node B does NOT have it locally
    {
        let key_id = NodeId::from_key(key.as_bytes());
        let st = node_b.storage.read().await;
        assert!(st.get(&key_id).is_none(), "Node B should NOT have the value locally yet");
    }

    // Node B does `get` — must find it via FIND_VALUE RPC to node A
    let result = node_b.get(key).await.unwrap();
    assert!(result.is_some(), "Node B should find the value via network FIND_VALUE");

    let (retrieved_val, from_addr) = result.unwrap();
    assert_eq!(retrieved_val, val, "Retrieved value must match original");
    assert_eq!(from_addr, Some(addr_a), "Value should come from node A");

    println!("✅ FIND_VALUE over real UDP network: PASSED");
}


#[tokio::test]
async fn test_store_replication_across_nodes() {
    let addr_a: SocketAddr = "127.0.0.1:17011".parse().unwrap();
    let addr_b: SocketAddr = "127.0.0.1:17012".parse().unwrap();
    let addr_c: SocketAddr = "127.0.0.1:17013".parse().unwrap();

    let node_a = KademliaNode::start(addr_a).await.unwrap();
    let node_b = KademliaNode::start(addr_b).await.unwrap();
    let node_c = KademliaNode::start(addr_c).await.unwrap();

    node_b.bootstrap(addr_a).await.unwrap();
    node_c.bootstrap(addr_b).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let key = "replicated_key";
    let val = b"Replicated across DHT!".to_vec();
    let stored_count = node_c.put(key, val.clone()).await.unwrap();
    assert!(stored_count >= 2, "Value should be replicated to at least 2 nodes, got {}", stored_count);

    {
        let key_id = NodeId::from_key(key.as_bytes());
        let st = node_a.storage.read().await;
        let local_val = st.get(&key_id);
        assert!(local_val.is_some(), "Node A should have the value via STORE replication");
        assert_eq!(local_val.unwrap(), val);
    }

    {
        let key_id = NodeId::from_key(key.as_bytes());
        let st = node_b.storage.read().await;
        let local_val = st.get(&key_id);
        assert!(local_val.is_some(), "Node B should have the value via STORE replication");
        assert_eq!(local_val.unwrap(), val);
    }

    println!("✅ STORE replication to k closest nodes: PASSED (replicated to {} nodes)", stored_count);
}


#[tokio::test]
async fn test_multi_hop_iterative_lookup() {
    let addrs: Vec<SocketAddr> = (0..5)
        .map(|i| format!("127.0.0.1:{}", 17021 + i).parse().unwrap())
        .collect();

    let mut nodes = Vec::new();
    for addr in &addrs {
        nodes.push(KademliaNode::start(*addr).await.unwrap());
    }

    for i in 1..nodes.len() {
        nodes[i].bootstrap(addrs[i - 1]).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(100)).await;

    let key = "deep_network_key";
    let val = b"Found through multi-hop lookup!".to_vec();
    {
        let key_id = NodeId::from_key(key.as_bytes());
        let mut st = nodes[0].storage.write().await;
        st.put(key_id, val.clone(), DEFAULT_TTL);
    }

    let result = nodes[4].get(key).await.unwrap();
    assert!(result.is_some(), "Node 4 must find value through multi-hop FIND_VALUE");

    let (retrieved_val, _from) = result.unwrap();
    assert_eq!(retrieved_val, val);

    println!("✅ Multi-hop iterative lookup (5 nodes): PASSED");
}


#[tokio::test]
async fn test_ttl_expiration() {
    let addr: SocketAddr = "127.0.0.1:17031".parse().unwrap();
    let node = KademliaNode::start(addr).await.unwrap();

    let key = "ttl_test_key";
    let key_id = NodeId::from_key(key.as_bytes());
    let val = b"This will expire!".to_vec();

    {
        let mut st = node.storage.write().await;
        st.put(key_id, val.clone(), Duration::from_millis(50));
    }

    {
        let st = node.storage.read().await;
        assert!(st.get(&key_id).is_some(), "Value should exist before TTL expires");
    }

    tokio::time::sleep(Duration::from_millis(100)).await;

    {
        let st = node.storage.read().await;
        assert!(st.get(&key_id).is_none(), "Value should NOT exist after TTL expires");
    }

    {
        let mut st = node.storage.write().await;
        let removed = st.cleanup_stale();
        assert!(removed >= 1, "Cleanup should remove the expired record");
    }

    println!("✅ TTL expiration: PASSED");
}


#[tokio::test]
async fn test_timeout_on_unreachable_node() {
    let addr: SocketAddr = "127.0.0.1:17041".parse().unwrap();
    let node = KademliaNode::start(addr).await.unwrap();

    let dead_addr: SocketAddr = "127.0.0.1:19999".parse().unwrap();
    let start = std::time::Instant::now();
    let result = node.ping_addr(dead_addr).await;
    let elapsed = start.elapsed();

    assert!(result.is_err(), "Ping to dead node should fail");
    assert!(elapsed.as_secs() < 20, "Timeout should not hang; elapsed: {:?}", elapsed);

    println!("✅ Timeout handling: PASSED (failed in {:?})", elapsed);
}

#[tokio::test]
async fn test_routing_table_population() {
    let addrs: Vec<SocketAddr> = (0..4)
        .map(|i| format!("127.0.0.1:{}", 17051 + i).parse().unwrap())
        .collect();

    let mut nodes = Vec::new();
    for addr in &addrs {
        nodes.push(KademliaNode::start(*addr).await.unwrap());
    }

    for i in 1..nodes.len() {
        nodes[i].bootstrap(addrs[0]).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(100)).await;

    let rt0 = nodes[0].routing_table.read().await;
    let contacts_count = rt0.total_contacts();
    assert!(contacts_count >= 3, "Bootstrap node should know at least 3 nodes");

    let rt3 = nodes[3].routing_table.read().await;
    let contacts3 = rt3.total_contacts();
    assert!(contacts3 >= 1, "Node 3 should know at least the bootstrap node");

    println!("✅ Routing table population: PASSED");
}

#[tokio::test]
async fn test_e2e_put_on_one_get_on_another() {
    let addr1: SocketAddr = "127.0.0.1:17061".parse().unwrap();
    let addr2: SocketAddr = "127.0.0.1:17062".parse().unwrap();
    let addr3: SocketAddr = "127.0.0.1:17063".parse().unwrap();

    let node1 = KademliaNode::start(addr1).await.unwrap();
    let node2 = KademliaNode::start(addr2).await.unwrap();
    let node3 = KademliaNode::start(addr3).await.unwrap();

    node2.bootstrap(addr1).await.unwrap();
    node3.bootstrap(addr1).await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let key = "e2e_distributed_key";
    let val = b"Distributed across the Kademlia DHT!".to_vec();
    let replicated = node2.put(key, val.clone()).await.unwrap();

    let result = node3.get(key).await.unwrap();
    assert!(result.is_some(), "Node 3 must find the value via DHT");
    assert_eq!(result.unwrap().0, val);

    let result1 = node1.get(key).await.unwrap();
    assert!(result1.is_some(), "Node 1 must also find the value");
    assert_eq!(result1.unwrap().0, val);

    println!("✅ End-to-end put/get across 3-node DHT: PASSED (replicated to {})", replicated);
}

#[tokio::test]
async fn test_edge_case_churn_node_leaving() {
    let addr1: SocketAddr = "127.0.0.1:17071".parse().unwrap();
    let addr2: SocketAddr = "127.0.0.1:17072".parse().unwrap();
    let addr3: SocketAddr = "127.0.0.1:17073".parse().unwrap();

    let _node1 = KademliaNode::start(addr1).await.unwrap();
    let node2 = KademliaNode::start(addr2).await.unwrap();
    let mut node3_opt = Some(KademliaNode::start(addr3).await.unwrap());

    node2.bootstrap(addr1).await.unwrap();
    if let Some(ref n3) = node3_opt {
        n3.bootstrap(addr1).await.unwrap();
    }
    tokio::time::sleep(Duration::from_millis(100)).await;

    let key = "churn_resilience_key";
    let val = b"Survives node drop".to_vec();

    // Put data from node2 (replicates to node1 and node3)
    let count = node2.put(key, val.clone()).await.unwrap();
    assert!(count >= 2);

    // Drop/kill node 3 completely to simulate sudden churn/node crash
    drop(node3_opt.take());
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Node 2 queries value again — must still get value from node1 replica
    let res = node2.get(key).await.unwrap();
    assert!(res.is_some(), "Data must survive node churn via surviving replica!");
    assert_eq!(res.unwrap().0, val);

    println!("✅ Churn node leaving: PASSED");
}

#[tokio::test]
async fn test_edge_case_bootstrap_failure() {
    let addr: SocketAddr = "127.0.0.1:17081".parse().unwrap();
    let node = KademliaNode::start(addr).await.unwrap();

    // Attempt bootstrap against unreachable address
    let dead_bootstrap: SocketAddr = "127.0.0.1:19998".parse().unwrap();
    let res = node.bootstrap(dead_bootstrap).await;

    assert!(res.is_err(), "Bootstrap against dead address should return error");

    // Node must still be functional locally
    let put_res = node.put("local_key", b"local_val".to_vec()).await.unwrap();
    assert_eq!(put_res, 1);

    let get_res = node.get("local_key").await.unwrap();
    assert_eq!(get_res.unwrap().0, b"local_val".to_vec());

    println!("✅ Bootstrap failure handling: PASSED");
}

#[tokio::test]
async fn test_edge_case_eviction_policy() {
    let local_id = NodeId::generate_random();
    let mut rt = nodex_kademlia::node::RoutingTable::new(local_id);

    // Fill bucket up to capacity K (20)
    for i in 0..nodex_kademlia::node::K {
        let mut bytes = [0u8; 20];
        bytes[0] = 0b10000000; // Same bucket (shared prefix 0 with 0000...)
        bytes[19] = (i + 1) as u8;
        let c = Contact::new(NodeId::from_bytes(bytes), format!("127.0.0.1:{}", 19100 + i).parse().unwrap());
        rt.update(c);
    }

    // Attempting to insert 21st contact returns Full(oldest_contact) for pinging
    let mut new_bytes = [0u8; 20];
    new_bytes[0] = 0b10000000;
    new_bytes[19] = 99;
    let new_contact = Contact::new(NodeId::from_bytes(new_bytes), "127.0.0.1:19199".parse().unwrap());

    let res = rt.update(new_contact.clone());
    if let nodex_kademlia::node::UpdateResult::Full(oldest) = res {
        assert_eq!(oldest.addr, "127.0.0.1:19100".parse::<SocketAddr>().unwrap());
        // Simulate eviction: replace oldest contact with new_contact
        rt.replace_oldest(new_contact.clone());
        let contacts = rt.all_contacts();
        assert_eq!(contacts.len(), nodex_kademlia::node::K);
        assert!(contacts.iter().any(|c| c.id == new_contact.id));
    } else {
        panic!("Expected UpdateResult::Full");
    }

    println!("✅ Eviction policy & replacement: PASSED");
}
