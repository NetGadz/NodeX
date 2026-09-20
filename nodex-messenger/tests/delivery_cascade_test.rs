use std::time::Duration;
use nodex_kademlia::KademliaNode;
use nodex_messenger::KadMessenger;

#[tokio::test]
async fn test_direct_udp_e2ee_message_delivery() {
    let node1 = KademliaNode::start("127.0.0.1:0".parse().unwrap()).await.expect("Node 1");
    let node2 = KademliaNode::start("127.0.0.1:0".parse().unwrap()).await.expect("Node 2");

    node2.bootstrap(node1.network.local_addr).await.expect("Bootstrap");

    let tmp_alice = std::env::temp_dir().join(format!("test_db_alice_{}.json", rand::random::<u32>()));
    let tmp_bob = std::env::temp_dir().join(format!("test_db_bob_{}.json", rand::random::<u32>()));

    let alice = KadMessenger::start(node1.clone(), tmp_alice.to_str().unwrap().into(), "Alice".into()).await.expect("Alice");
    let bob = KadMessenger::start(node2.clone(), tmp_bob.to_str().unwrap().into(), "Bob".into()).await.expect("Bob");

    tokio::time::sleep(Duration::from_millis(100)).await;

    alice.publish_presence().await.expect("Alice presence");
    bob.publish_presence().await.expect("Bob presence");

    let bob_uid = bob.user_id_hex().await;

    // Send direct E2EE message from Alice to Bob
    let sent_msg = alice.send_message(&bob_uid, "Hello Bob direct E2EE!", None).await.expect("Send message");
    assert_eq!(sent_msg.text, "Hello Bob direct E2EE!");

    // Clean up test DBs
    alice.wipe_local_data().await;
    bob.wipe_local_data().await;
}
