use nodex_kademlia::bootstrap::SignedBootstrapList;

#[test]
fn test_signed_bootstrap_expiration_validation() {
    let list = SignedBootstrapList {
        version: 1,
        expires_at: 1000,
        nodes: vec!["1.2.3.4:8000".into(), "5.6.7.8:8000".into()],
        signature_hex: "abcd".into(),
    };

    assert!(list.is_valid(900), "Valid before expiration");
    assert!(!list.is_valid(1000), "Invalid at expiration");
    assert!(!list.is_valid(1100), "Invalid after expiration");
}
