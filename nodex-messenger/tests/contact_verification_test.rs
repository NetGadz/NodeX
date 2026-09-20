use nodex_messenger::trust::TrustManager;

#[test]
fn test_trust_manager_fingerprint_verification() {
    let key1 = [42u8; 32];
    let key2 = [42u8; 32];
    let key3 = [99u8; 32];

    assert!(TrustManager::verify_key_unchanged(&key1, &key2));
    assert!(!TrustManager::verify_key_unchanged(&key1, &key3));

    let fp = TrustManager::compute_key_fingerprint(&key1);
    assert!(!fp.is_empty());
}
