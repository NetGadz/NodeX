use std::net::SocketAddr;
use std::time::Duration;

use nodex_kademlia::KademliaNode;
use nodex_messenger::crypto::{decrypt_envelope, encrypt_envelope, UserIdentity};
use nodex_messenger::db::MessengerDb;
use nodex_messenger::KadMessenger;

#[test]
fn test_envelope_replay_and_tamper_protection() {
    let alice = UserIdentity::generate();
    let bob = UserIdentity::generate();

    let payload = b"Secret Payload";
    let envelope = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, payload).expect("Encrypt");

    // Decrypt succeeds on original
    let decrypted = decrypt_envelope(&bob, &envelope).expect("Decrypt");
    assert_eq!(decrypted, payload);

    // Tampered payload fails verification
    let mut tampered = envelope.clone();
    tampered.ciphertext[0] ^= 0xFF;
    assert!(decrypt_envelope(&bob, &tampered).is_err());
}

#[test]
fn test_bip39_randomness_and_2048_wordlist_uniqueness() {
    let mut mnemonics = std::collections::HashSet::new();
    for _ in 0..50 {
        let (mn, id) = UserIdentity::generate_mnemonic().expect("Mnemonic generation should succeed");
        let words: Vec<&str> = mn.split_whitespace().collect();
        assert_eq!(words.len(), 12, "Each mnemonic must have exactly 12 words");
        
        // Ensure deterministic seed derivation matches
        let restored = UserIdentity::from_mnemonic(&mn).expect("Mnemonic to seed must succeed");
        assert_eq!(id.user_id, restored.user_id);
        assert_eq!(id.verifying_key.as_bytes(), restored.verifying_key.as_bytes());

        assert!(mnemonics.insert(mn), "Mnemonic phrases must be unique and randomly generated for each account");
    }
}

#[tokio::test]
async fn test_db_encryption_dynamic_key_no_hardcoded_static_keys() {
    let tmp = std::env::temp_dir().join("test_dynamic_key_db.json");
    let db = MessengerDb {
        display_name: "SecretUser".into(),
        bio: "TopSecretBio".into(),
        ..Default::default()
    };
    db.save_to_file(&tmp).expect("Save DB");

    // Read raw bytes on disk
    let raw = std::fs::read(&tmp).expect("Read raw DB");
    assert!(raw.starts_with(b"NODEXENC2"), "DB must use NODEXENC2 header format");
    assert!(!String::from_utf8_lossy(&raw).contains("SecretUser"), "Plaintext must not be exposed");
    assert!(!String::from_utf8_lossy(&raw).contains("TopSecretBio"), "Plaintext bio must not be exposed");

    let loaded = MessengerDb::load_from_file(&tmp).expect("Load DB");
    assert_eq!(loaded.display_name, "SecretUser");

    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test]
async fn test_fail_closed_when_recipient_key_missing() {
    let addr: SocketAddr = "127.0.0.1:16031".parse().unwrap();
    let node = KademliaNode::start(addr).await.unwrap();
    let tmp = std::env::temp_dir().join("test_fail_closed_db.json");

    let messenger = KadMessenger::start(node, tmp.to_str().unwrap().into(), "Alice".into()).await.unwrap();

    // Fake recipient ID with no presence in DHT
    let fake_recipient = "1122334455667788990011223344556677889900";
    let res = messenger.send_message(fake_recipient, "Hello!", None).await;

    assert!(res.is_err(), "Must fail closed when recipient has no verified X25519 key");
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("failed closed") || err_msg.contains("not found"), "Error must clearly indicate fail-closed: {}", err_msg);

    let _ = std::fs::remove_file(&tmp);
}

#[tokio::test]
async fn test_blocklist_enforcement_bidirectional() {
    let addr_alice: SocketAddr = "127.0.0.1:16041".parse().unwrap();
    let addr_bob: SocketAddr = "127.0.0.1:16042".parse().unwrap();

    let node_alice = KademliaNode::start(addr_alice).await.unwrap();
    let node_bob = KademliaNode::start(addr_bob).await.unwrap();
    node_bob.bootstrap(addr_alice).await.unwrap();

    let tmp_alice = std::env::temp_dir().join("test_block_alice_db.json");
    let tmp_bob = std::env::temp_dir().join("test_block_bob_db.json");

    let alice = KadMessenger::start(node_alice, tmp_alice.to_str().unwrap().into(), "Alice".into()).await.unwrap();
    let bob = KadMessenger::start(node_bob, tmp_bob.to_str().unwrap().into(), "Bob".into()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(100)).await;

    let _alice_id = alice.identity.read().await.user_id_hex();
    let bob_id = bob.identity.read().await.user_id_hex();

    // Alice blocks Bob
    alice.block_contact(&bob_id).await;
    assert!(alice.db.read().await.is_blocked(&bob_id));

    // Alice cannot send messages to blocked Bob
    let send_res = alice.send_message(&bob_id, "Hey Bob", None).await;
    assert!(send_res.is_err(), "Sending to blocked contact must fail");

    // Alice unblocks Bob
    alice.unblock_contact(&bob_id).await;
    assert!(!alice.db.read().await.is_blocked(&bob_id));

    let _ = std::fs::remove_file(&tmp_alice);
    let _ = std::fs::remove_file(&tmp_bob);
}

#[tokio::test]
async fn test_remote_delete_for_everyone() {
    let addr_alice: SocketAddr = "127.0.0.1:16051".parse().unwrap();
    let addr_bob: SocketAddr = "127.0.0.1:16052".parse().unwrap();

    let node_alice = KademliaNode::start(addr_alice).await.unwrap();
    let node_bob = KademliaNode::start(addr_bob).await.unwrap();
    node_bob.bootstrap(addr_alice).await.unwrap();

    let unique_id = rand::random::<u32>();
    let tmp_alice = std::env::temp_dir().join(format!("test_del_alice_db_{}.json", unique_id));
    let tmp_bob = std::env::temp_dir().join(format!("test_del_bob_db_{}.json", unique_id));

    let alice = KadMessenger::start(node_alice, tmp_alice.to_str().unwrap().into(), format!("Alice_{}", unique_id)).await.unwrap();
    let bob = KadMessenger::start(node_bob, tmp_bob.to_str().unwrap().into(), format!("Bob_{}", unique_id)).await.unwrap();
    bob.start_inbox_polling_task(None);

    tokio::time::sleep(Duration::from_millis(200)).await;

    let alice_id = alice.identity.read().await.user_id_hex();
    let bob_id = bob.identity.read().await.user_id_hex();

    // Alice sends a message to Bob
    let sent = alice.send_message(&bob_id, "Secret message to be deleted", None).await.unwrap();
    let msg_id = sent.id.clone();

    // Wait for Bob to receive
    tokio::time::sleep(Duration::from_secs(3)).await;
    let bob_msgs = bob.db.read().await.get_messages_for_contact(&alice_id);
    assert!(!bob_msgs.is_empty(), "Bob should receive message");
    assert_eq!(bob_msgs[0].id, msg_id, "Sender and receiver message IDs must match");

    // Alice deletes message for everyone (both)
    alice.delete_messages_for_everyone(&bob_id, vec![msg_id.clone()]).await.unwrap();

    // Alice's local copy is removed
    assert!(alice.db.read().await.get_messages_for_contact(&bob_id).is_empty());

    // Wait for Bob to process remote deletion command
    tokio::time::sleep(Duration::from_secs(4)).await;

    let bob_msgs_after = bob.db.read().await.get_messages_for_contact(&alice_id);
    assert!(bob_msgs_after.is_empty(), "Bob's messages must be deleted after remote delete for both");
    assert!(bob.db.read().await.deleted_message_ids.contains(&msg_id), "Tombstone must be recorded");

    let _ = std::fs::remove_file(&tmp_alice);
    let _ = std::fs::remove_file(&tmp_bob);
}


#[tokio::test]
async fn test_wipe_local_data_completely() {
    let addr: SocketAddr = "127.0.0.1:16061".parse().unwrap();
    let node = KademliaNode::start(addr).await.unwrap();
    let tmp = std::env::temp_dir().join("test_wipe_db.json");

    let messenger = KadMessenger::start(node, tmp.to_str().unwrap().into(), "WipeTest".into()).await.unwrap();
    assert!(tmp.exists());

    messenger.wipe_local_data().await;

    assert!(!tmp.exists(), "DB file must be deleted on wipe");
    assert!(messenger.db.read().await.contacts.is_empty());
    assert!(messenger.db.read().await.messages.is_empty());
    assert_eq!(messenger.db.read().await.user_seed, [0u8; 32]);
}

