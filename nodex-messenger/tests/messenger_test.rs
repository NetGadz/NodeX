use std::net::SocketAddr;
use std::time::Duration;

use nodex_messenger::KadMessenger;
use nodex_kademlia::KademliaNode;

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

    let alice_id = alice.identity.read().await.user_id_hex();

    // Bob discovers Alice's presence card from DHT
    let card_alice = bob.discover_peer(&alice_id).await.unwrap();
    assert_eq!(card_alice.display_name, "Alice");
    assert_eq!(card_alice.user_id_hex, alice_id);

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
    bob.start_inbox_polling_task(None);

    tokio::time::sleep(Duration::from_millis(100)).await;

    let alice_id = alice.identity.read().await.user_id_hex();
    let bob_id = bob.identity.read().await.user_id_hex();

    // Alice sends E2EE message to Bob
    let secret = "Hello Bob, this is encrypted P2P message over Kademlia!";
    let sent_msg = alice.send_message(&bob_id, secret, None).await.unwrap();
    assert_eq!(sent_msg.text, secret);

    // Wait for Bob's background inbox polling task to pick up offline envelope from DHT
    tokio::time::sleep(Duration::from_secs(6)).await;

    let bob_msgs = bob.db.read().await.get_messages_for_contact(&alice_id);
    assert!(!bob_msgs.is_empty(), "Bob should have received Alice's E2EE message!");
    assert_eq!(bob_msgs[0].text, secret);

    println!("✅ E2EE Offline Store-and-Forward Mailbox Relay: PASSED");
}

#[tokio::test]
async fn test_messenger_mnemonic_restore() {
    let addr: SocketAddr = "127.0.0.1:16021".parse().unwrap();
    let node = KademliaNode::start(addr).await.unwrap();
    let tmp_db = std::env::temp_dir().join("test_mnemonic_db.json");

    let messenger = KadMessenger::start(node, tmp_db.to_str().unwrap().into(), "Charlie".into()).await.unwrap();
    let initial_mnemonic = messenger.get_mnemonic().await;
    assert!(!initial_mnemonic.is_empty(), "Mnemonic should be auto-generated");
    let words: Vec<&str> = initial_mnemonic.split_whitespace().collect();
    assert_eq!(words.len(), 12, "Should have 12 BIP-39 words");

    let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    messenger.restore_mnemonic(test_mnemonic, Some("RestoredCharlie".into())).await.unwrap();
    
    let restored_id = messenger.identity.read().await.user_id_hex();
    assert_eq!(messenger.db.read().await.display_name, "RestoredCharlie");
    assert_eq!(messenger.get_mnemonic().await, test_mnemonic);
    println!("✅ Mnemonic restore: PASSED (Restored user ID: {})", restored_id);
}
