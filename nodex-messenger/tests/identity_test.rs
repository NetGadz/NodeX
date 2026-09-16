use nodex_messenger::identity::UserIdentity;

#[test]
fn test_identity_creation_and_fingerprint() {
    let (mnemonic, identity) = UserIdentity::generate_mnemonic().expect("Generate mnemonic");
    assert_eq!(mnemonic.split_whitespace().count(), 12);
    assert_eq!(identity.user_id_hex().len(), 40);

    let fp = identity.fingerprint();
    assert!(!fp.is_empty());
    assert!(fp.contains(':'));

    let restored = UserIdentity::from_mnemonic(&mnemonic).expect("Restore identity");
    assert_eq!(identity.user_id_hex(), restored.user_id_hex());
    assert_eq!(identity.fingerprint(), restored.fingerprint());
}
