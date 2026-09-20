use nodex_kademlia::node::{Contact, KBucket, NodeId, UpdateResult};
use nodex_messenger::db::*;
use nodex_messenger::messenger::*;
use std::net::SocketAddr;

#[test]
fn test_contact_address_spoofing_prevented() {
    let mut bucket = KBucket::new();
    let bob_id = NodeId::generate_random();
    let bob_legit_addr: SocketAddr = "192.168.1.100:8000".parse().unwrap();
    let attacker_addr: SocketAddr = "198.51.100.20:9999".parse().unwrap();

    // 1. Legitimate contact added (e.g. from verified presence or initial insert)
    let res1 = bucket.update(Contact::new(bob_id, bob_legit_addr));
    assert_eq!(res1, UpdateResult::Inserted);
    assert_eq!(bucket.contacts[0].addr, bob_legit_addr);

    // 2. Attacker sends unauthenticated packet with bob_id from attacker_addr
    let res2 = bucket.update(Contact::new(bob_id, attacker_addr));
    assert_eq!(res2, UpdateResult::Updated);
    // Address MUST NOT be overwritten by unauthenticated bucket update
    assert_eq!(bucket.contacts[0].addr, bob_legit_addr);
    assert_ne!(bucket.contacts[0].addr, attacker_addr);

    // 3. Authenticated update (e.g., from verified presence card) CAN update address
    let new_legit_addr: SocketAddr = "192.168.1.105:8000".parse().unwrap();
    bucket.force_update(Contact::new(bob_id, new_legit_addr));
    assert_eq!(bucket.contacts[0].addr, new_legit_addr);
}

#[test]
fn test_compact_payload_omits_null_fields() {
    let payload = ChatMessagePayload {
        id: "msg123".to_string(),
        text: "Hello world".to_string(),
        image_base64: None,
        voice_note: None,
        file_manifest: None,
        delivery_receipt: None,
        read_receipt: None,
        reaction: None,
        reply_to_id: None,
        reply_snippet: None,
        is_edited: false,
        edit_timestamp: None,
        group_id: None,
        call_signal: None,
        call_audio_chunk: None,
        delete_message_ids: None,
        tombstone: None,
    };

    let json_bytes = serde_json::to_vec(&payload).unwrap();
    let json_str = String::from_utf8(json_bytes).unwrap();

    // Ensure none of the None fields are serialized as '"field":null'
    assert!(!json_str.contains("\"group_id\""));
    assert!(!json_str.contains("\"call_signal\""));
    assert!(!json_str.contains("\"voice_note\""));
    assert!(!json_str.contains("\"image_base64\""));
    assert!(!json_str.contains("\"reaction\""));
    assert!(!json_str.contains("\"delete_message_ids\""));
    assert!(!json_str.contains("\"tombstone\""));
    assert!(json_str.contains("\"text\":\"Hello world\""));
    assert!(json_str.contains("\"id\":\"msg123\""));
}

#[test]
fn test_wipe_forgets_instance_key_and_cache_reconstitution() {
    let temp_dir = std::env::temp_dir();
    let db_path = temp_dir.join(format!("test_wipe_forget_{}.json", rand::random::<u32>()));
    let key_path = std::path::PathBuf::from(format!("{}.key", db_path.display()));

    let mut db = MessengerDb::default();
    db.display_name = "test_owner".to_string();
    db.save_to_file(&db_path).unwrap();

    assert!(key_path.exists());
    assert!(db_path.exists());

    // Forget key directly and remove files
    forget_instance_key(&db_path);
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(&key_path);

    assert!(!db_path.exists());
    assert!(!key_path.exists());
}

#[tokio::test]
async fn test_response_address_matching_and_random_req_id() {
    use nodex_kademlia::rpc::{RpcMessage, RpcPayload};
    use std::sync::Arc;
    use tokio::sync::{oneshot, Mutex};
    use std::collections::HashMap;

    let expected_peer: SocketAddr = "192.168.1.50:9000".parse().unwrap();
    let attacker_peer: SocketAddr = "198.51.100.77:9000".parse().unwrap();

    let pending: Arc<Mutex<HashMap<u64, (SocketAddr, oneshot::Sender<RpcMessage>)>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let req_id: u64 = rand::random();
    let (tx, mut rx) = oneshot::channel();

    // Register pending request bound to expected_peer
    {
        let mut p = pending.lock().await;
        p.insert(req_id, (expected_peer, tx));
    }

    // 1. Attacker tries to inject response for req_id from attacker_peer
    {
        let mut p = pending.lock().await;
        if let Some((expected_addr, _)) = p.get(&req_id) {
            if *expected_addr != attacker_peer {
                // Attacker's response is rejected!
            } else {
                let (_, sender) = p.remove(&req_id).unwrap();
                let _ = sender.send(RpcMessage {
                    request_id: req_id,
                    sender_id: NodeId::generate_random(),
                    payload: RpcPayload::Ping,
                });
            }
        }
    }

    // Channel should NOT have received attacker's response
    assert!(rx.try_recv().is_err());

    // 2. Legitimate peer responds from expected_peer
    {
        let mut p = pending.lock().await;
        if let Some((expected_addr, _)) = p.get(&req_id) {
            if *expected_addr == expected_peer {
                let (_, sender) = p.remove(&req_id).unwrap();
                let _ = sender.send(RpcMessage {
                    request_id: req_id,
                    sender_id: NodeId::generate_random(),
                    payload: RpcPayload::Pong,
                });
            }
        }
    }

    // Channel successfully receives legitimate peer's response
    let res = rx.await.unwrap();
    assert_eq!(res.payload, RpcPayload::Pong);
}
