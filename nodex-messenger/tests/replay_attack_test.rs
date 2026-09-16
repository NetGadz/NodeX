use nodex_messenger::crypto::{decrypt_envelope, encrypt_envelope, UserIdentity};

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
