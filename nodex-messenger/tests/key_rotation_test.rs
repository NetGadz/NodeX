use nodex_messenger::contact_verification::ContactVerifier;
use nodex_messenger::db::SavedContact;

#[test]
fn test_security_fingerprint_formatting() {
    let pubkey = [0xAA; 32];
    let formatted = ContactVerifier::format_security_fingerprint(&pubkey);
    assert_eq!(formatted.len(), 32 * 2 + 15); // 64 hex chars + 15 dashes
    assert!(formatted.starts_with("AAAA-AAAA"));
}

#[test]
fn test_detect_key_rotation_alert() {
    let contact = SavedContact {
        user_id_hex: "alice_id".into(),
        name: "Alice".into(),
        bio: "".into(),
        ed25519_pub_hex: "11".repeat(32),
        x25519_pub_hex: "22".repeat(32),
        last_seen_addr: "127.0.0.1:8000".into(),
    };

    // Same key -> no alert
    let same = ContactVerifier::detect_key_rotation(&contact, &"11".repeat(32), 1000);
    assert!(same.is_none());

    // Changed key -> trigger KeyRotationAlert
    let new_key = "33".repeat(32);
    let alert = ContactVerifier::detect_key_rotation(&contact, &new_key, 1000);
    assert!(alert.is_some());
    let a = alert.unwrap();
    assert_eq!(a.contact_user_id, "alice_id");
    assert_eq!(a.old_ed25519_pub_hex, "11".repeat(32));
    assert_eq!(a.new_ed25519_pub_hex, new_key);
}
