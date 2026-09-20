use nodex_messenger::crypto::{decrypt_envelope, encrypt_envelope, UserIdentity, ReplayProtectionCache};
use nodex_messenger::db::{MessengerDb, SavedChatMessage, get_or_create_instance_key};

#[test]
fn test_sender_id_spoofing_rejected() {
    let alice = UserIdentity::generate();
    let bob = UserIdentity::generate();
    let eve = UserIdentity::generate();

    let secret_msg = b"I am Alice (impersonated by Eve)";
    // Eve signs with her own key but sets sender_id = alice.user_id
    let mut envelope = encrypt_envelope(&eve, bob.user_id, &bob.x25519_public, secret_msg).expect("Encrypt");
    envelope.sender_id = alice.user_id;

    // Bob must reject this envelope because sender_id != NodeId::from_key(sender_ed25519_pub)
    let res = decrypt_envelope(&bob, &envelope);
    assert!(res.is_err(), "Must reject envelope when sender_id is spoofed");
    let err = res.unwrap_err();
    assert!(
        err.contains("sender_id does not match sender ed25519 public key"),
        "Error must be specific about key mismatch: {}",
        err
    );
}

#[test]
fn test_envelope_v3_all_fields_tamper_detection() {
    let alice = UserIdentity::generate();
    let bob = UserIdentity::generate();

    let payload = b"Cryptographic Integrity Test V3";
    let envelope = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, payload).expect("Encrypt");

    // 1. Decrypt succeeds on pristine envelope
    assert_eq!(decrypt_envelope(&bob, &envelope).unwrap(), payload);

    // 2. Tampering version
    let mut bad_ver = envelope.clone();
    bad_ver.version = 99;
    assert!(decrypt_envelope(&bob, &bad_ver).is_err());

    // 3. Tampering recipient_id
    let mut bad_rcpt = envelope.clone();
    bad_rcpt.recipient_id = alice.user_id;
    assert!(decrypt_envelope(&bob, &bad_rcpt).is_err());

    // 4. Tampering nonce
    let mut bad_nonce = envelope.clone();
    bad_nonce.nonce[0] ^= 0x55;
    assert!(decrypt_envelope(&bob, &bad_nonce).is_err());

    // 5. Tampering timestamp
    let mut bad_ts = envelope.clone();
    bad_ts.timestamp += 10;
    assert!(decrypt_envelope(&bob, &bad_ts).is_err());

    // 6. Tampering ephemeral key
    let mut bad_eph = envelope.clone();
    if let Some(ref mut eph) = bad_eph.ephemeral_x25519_pub {
        eph[0] ^= 0xAA;
    }
    assert!(decrypt_envelope(&bob, &bad_eph).is_err());
}

#[test]
fn test_db_key_loss_fails_closed_no_db_overwrite() {
    let unique_id = rand::random::<u32>();
    let tmp_db = std::env::temp_dir().join(format!("test_safe_db_{}.json", unique_id));
    let tmp_key = std::env::temp_dir().join(format!("test_safe_db_{}.json.key", unique_id));

    let mut db = MessengerDb::default();
    db.display_name = "AliceCriticalAccount".into();
    db.save_to_file(&tmp_db).expect("Save DB must succeed initially");

    assert!(tmp_db.exists(), "DB file must exist");
    assert!(tmp_key.exists(), "Key file must exist");

    // Simulate accidental deletion/corruption of .key file
    std::fs::remove_file(&tmp_key).expect("Remove key file");

    // Attempting to get key for existing DB must FAIL CLOSED (not create new random key)
    let key_res = get_or_create_instance_key(&tmp_db);
    assert!(key_res.is_err(), "Must return Err when DB exists but .key is missing");
    let err_msg = key_res.unwrap_err();
    assert!(err_msg.contains("missing or corrupted"), "Error message: {}", err_msg);

    // Attempting to load DB must fail safely without wiping DB
    let load_res = MessengerDb::load_from_file(&tmp_db);
    assert!(load_res.is_err(), "Loading DB without key must return error");

    // Verify DB file was NOT wiped or overwritten
    assert!(tmp_db.exists(), "DB file on disk must remain intact for user recovery");
    let raw = std::fs::read(&tmp_db).expect("Read DB");
    assert!(!raw.is_empty(), "DB file must not be empty");

    // Cleanup
    let _ = std::fs::remove_file(&tmp_db);
    let _ = std::fs::remove_file(&tmp_key);
}

#[test]
fn test_unauthorized_tombstone_and_edit_rejected() {
    let mut db = MessengerDb::default();
    let alice_id = "alice_hex_0001";
    let bob_id = "bob_hex_0002";

    let msg1 = SavedChatMessage {
        id: "msg_alice_1".into(),
        sender_id_hex: alice_id.into(),
        recipient_id_hex: bob_id.into(),
        text: "Original Alice Text".into(),
        incoming: false,
        delivered: true,
        timestamp: 100,
        ..Default::default()
    };
    db.add_message(msg1);

    // Bob attempts to edit Alice's message -> MUST FAIL
    let edit_res = db.edit_message_by_author("msg_alice_1", "Hacked text by Bob", 200, bob_id);
    assert!(!edit_res, "Non-author must not be able to edit message");
    assert_eq!(db.messages[0].text, "Original Alice Text", "Text must remain unchanged");

    // Alice edits her own message -> SUCCEEDS
    let edit_alice_res = db.edit_message_by_author("msg_alice_1", "Updated Alice Text", 200, alice_id);
    assert!(edit_alice_res, "Author must be able to edit message");
    assert_eq!(db.messages[0].text, "Updated Alice Text");

    // Bob attempts to remote-delete (tombstone) Alice's message -> MUST BE REJECTED
    let deleted = db.delete_messages_by_author(&["msg_alice_1".to_string()], bob_id);
    assert!(deleted.is_empty(), "Non-author cannot delete messages via tombstone");
    assert_eq!(db.messages.len(), 1, "Message must not be deleted");

    // Alice deletes her own message -> SUCCEEDS
    let alice_deleted = db.delete_messages_by_author(&["msg_alice_1".to_string()], alice_id);
    assert_eq!(alice_deleted, vec!["msg_alice_1".to_string()]);
    assert!(db.messages.is_empty(), "Message should be deleted by author");
}

#[test]
fn test_replay_cache_tampered_envelope_does_not_censor_valid() {
    let cache = ReplayProtectionCache::new();
    let alice = UserIdentity::generate();
    let bob = UserIdentity::generate();

    let valid_envelope = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, b"Valid Msg").unwrap();

    // Attacker modifies nonce of valid envelope
    let mut attacker_envelope = valid_envelope.clone();
    attacker_envelope.nonce[0] ^= 0xFF;

    // Decrypting tampered envelope fails (signature check fails)
    assert!(decrypt_envelope(&bob, &attacker_envelope).is_err());

    // Because decryption failed, attacker's packet is NOT entered into replay cache.
    // The legitimate original envelope can now be decrypted and recorded normally!
    let decrypted = decrypt_envelope(&bob, &valid_envelope).expect("Original must decrypt");
    assert_eq!(decrypted, b"Valid Msg");

    let now = valid_envelope.timestamp;
    assert!(cache.check_and_insert(&valid_envelope.signature, valid_envelope.timestamp, now));
    // Subsequent duplicates of the valid envelope are blocked
    assert!(!cache.check_and_insert(&valid_envelope.signature, valid_envelope.timestamp, now));
}

#[test]
fn test_short_nonce_envelope_rejected_without_panic() {
    let alice = UserIdentity::generate();
    let bob = UserIdentity::generate();

    // Attacker crafts envelope with 5-byte short nonce and signs it correctly with Alice's key
    let short_nonce = vec![1, 2, 3, 4, 5];
    let ciphertext = vec![0xAA; 32];
    let timestamp: u64 = 100000;

    let mut sign_payload = Vec::new();
    sign_payload.extend_from_slice(b"NodeX-envelope-v3");
    sign_payload.push(3);
    sign_payload.extend_from_slice(alice.user_id.as_bytes());
    sign_payload.extend_from_slice(bob.user_id.as_bytes());
    sign_payload.extend_from_slice(alice.x25519_public.as_bytes());
    sign_payload.extend_from_slice(alice.x25519_public.as_bytes()); // eph dummy
    sign_payload.extend_from_slice(&short_nonce);
    sign_payload.extend_from_slice(&timestamp.to_be_bytes());
    sign_payload.extend_from_slice(&ciphertext);

    let signature = alice.sign(&sign_payload);

    let crafted_envelope = nodex_messenger::crypto::EncryptedEnvelope {
        version: 3,
        sender_id: alice.user_id,
        sender_ed25519_pub: alice.verifying_key.to_bytes().to_vec(),
        sender_x25519_pub: alice.x25519_public.as_bytes().to_vec(),
        ephemeral_x25519_pub: Some(alice.x25519_public.as_bytes().to_vec()),
        recipient_id: bob.user_id,
        nonce: short_nonce,
        ciphertext,
        signature,
        timestamp,
    };

    // decrypt_envelope MUST NOT panic on invalid nonce length!
    let res = decrypt_envelope(&bob, &crafted_envelope);
    assert!(res.is_err(), "Must return Err on short nonce");
    let err = res.unwrap_err();
    assert!(err.contains("Malformed envelope"), "Error: {}", err);
}

#[test]
fn test_offline_delivery_accepts_messages_within_7_days() {
    let cache = ReplayProtectionCache::new();
    let alice = UserIdentity::generate();
    let bob = UserIdentity::generate();

    let envelope = encrypt_envelope(&alice, bob.user_id, &bob.x25519_public, b"Offline msg from 2h ago").unwrap();

    // Recipient comes online 2 hours (7200s) later
    let now = envelope.timestamp + 7200;

    // Decryption works
    let decrypted = decrypt_envelope(&bob, &envelope).expect("Decrypting 2-hour old offline message must succeed");
    assert_eq!(decrypted, b"Offline msg from 2h ago");

    // Replay cache accepts message sent 2h ago because it's within 7-day retention window
    assert!(cache.check_and_insert(&envelope.signature, envelope.timestamp, now), "2-hour old message must be accepted");
}

#[test]
fn test_tombstone_before_message_arrival_discards_late_message() {
    let mut db = MessengerDb::default();
    let late_msg_id = "late_arrival_101".to_string();

    // 1. Tombstone arrives first before message is delivered
    db.deleted_message_ids.insert(late_msg_id.clone());

    // 2. Late message arrives later
    let msg = SavedChatMessage {
        id: late_msg_id.clone(),
        sender_id_hex: "alice_01".into(),
        recipient_id_hex: "bob_02".into(),
        text: "This message was already tombstoned".into(),
        ..Default::default()
    };
    db.add_message(msg);

    // 3. Verify message was discarded by add_message
    assert!(db.messages.is_empty(), "Late message must be dropped if already tombstoned");
}
