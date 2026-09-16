use nodex_messenger::mnemonic::MnemonicManager;

#[test]
fn test_mnemonic_manager_generate_validate() {
    let phrase = MnemonicManager::generate_12_words().expect("Generate 12 words");
    assert!(MnemonicManager::validate(&phrase));

    let seed = MnemonicManager::to_seed(&phrase).expect("Derive seed");
    assert_ne!(seed, [0u8; 32]);

    assert!(!MnemonicManager::validate("invalid words phrase not twelve"));
}
