use nodex_kademlia::config::NodeConfig;
use nodex_kademlia::KademliaNode;
use nodex_messenger::KadMessenger;

#[tokio::test]
async fn test_direct_udp_e2ee_message_delivery() {
    let mut cfg1 = NodeConfig::default();
    cfg1.port = 19900;
    let node1 = KademliaNode::start_with_config(cfg1).await.expect("Node 1");
    let alice = KadMessenger::start(node1.clone(), "test_db_alice.json".into(), "Alice".into()).await.expect("Alice");

    let mut cfg2 = NodeConfig::default();
    cfg2.port = 19901;
    cfg2.bootstrap = vec![node1.network.local_addr];
    let node2 = KademliaNode::start_with_config(cfg2).await.expect("Node 2");
    let bob = KadMessenger::start(node2.clone(), "test_db_bob.json".into(), "Bob".into()).await.expect("Bob");

    node2.bootstrap(node1.network.local_addr).await.expect("Bootstrap");
    alice.publish_presence().await.expect("Alice presence");
    bob.publish_presence().await.expect("Bob presence");

    let bob_uid = bob.user_id_hex().await;

    // Send direct E2EE message from Alice to Bob
    let sent_msg = alice.send_message(&bob_uid, "Hello Bob direct E2EE!", None).await.expect("Send message");
    assert_eq!(sent_msg.text, "Hello Bob direct E2EE!");

    // Clean up test DBs
    std::fs::remove_file("test_db_alice.json").ok();
    std::fs::remove_file("test_db_bob.json").ok();
}
