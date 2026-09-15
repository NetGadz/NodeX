use std::net::SocketAddr;
use std::time::Duration;

use kademlia_dht::messenger::KadMessenger;
use kademlia_dht::KademliaNode;

#[tokio::test]
async fn test_messenger_identity_and_presence() {
    let addr_alice: SocketAddr = "127.0.0.1:16001".parse().unwrap();
    let addr_bob: SocketAddr = "127.0.0.1:16002".parse().unwrap();

    let node_alice = KademliaNode::start(addr_alice).await.unwrap();
    let node_bob = KademliaNode::start(addr_bob).await.unwrap();

    node_bob.bootstrap(addr_alice).await.unwrap();

    let tmp_alice = std::env::temp_dir().join("test_alice_db.json");
    let tmp_bob = std::env::temp_dir().join("test_bob_db.json");

    let alice = KadMessenger::start(node_alice, tmp_alice.to_str().unwrap().into(), "Alice".into()).await.unwrap();
    let bob = KadMessenger::start(node_bob, tmp_bob.to_str().unwrap().into(), "Bob".into()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Bob discovers Alice's presence card from DHT
    let card_alice = bob.discover_peer(&alice.identity.user_id_hex()).await.unwrap();
    assert_eq!(card_alice.display_name, "Alice");
    assert_eq!(card_alice.user_id_hex, alice.identity.user_id_hex());

    println!("✅ Presence card publishing and discovery: PASSED");
}

#[tokio::test]
async fn test_messenger_offline_store_and_forward_e2ee() {
    let addr_alice: SocketAddr = "127.0.0.1:16011".parse().unwrap();
    let addr_bob: SocketAddr = "127.0.0.1:16012".parse().unwrap();

    let node_alice = KademliaNode::start(addr_alice).await.unwrap();
    let node_bob = KademliaNode::start(addr_bob).await.unwrap();

    node_bob.bootstrap(addr_alice).await.unwrap();

    let tmp_alice = std::env::temp_dir().join("test_alice_e2e_db.json");
    let tmp_bob = std::env::temp_dir().join("test_bob_e2e_db.json");

    let alice = KadMessenger::start(node_alice, tmp_alice.to_str().unwrap().into(), "Alice".into()).await.unwrap();
    let bob = KadMessenger::start(node_bob, tmp_bob.to_str().unwrap().into(), "Bob".into()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Alice sends E2EE message to Bob
    let secret = "Hello Bob, this is encrypted P2P message over Kademlia!";
    let sent_msg = alice.send_message(&bob.identity.user_id_hex(), secret).await.unwrap();
    assert_eq!(sent_msg.text, secret);

    // Wait for Bob's background inbox polling task to pick up offline envelope from DHT
    tokio::time::sleep(Duration::from_secs(6)).await;

    let bob_msgs = bob.db.read().await.get_messages_for_contact(&alice.identity.user_id_hex());
    assert!(!bob_msgs.is_empty(), "Bob should have received Alice's E2EE message!");
    assert_eq!(bob_msgs[0].text, secret);

    println!("✅ E2EE Offline Store-and-Forward Mailbox Relay: PASSED");
}
