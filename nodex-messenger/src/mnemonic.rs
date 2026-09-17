use rand::RngCore;
use crate::errors::MessengerError;

pub struct MnemonicManager;

impl MnemonicManager {
    pub fn generate_12_words() -> Result<String, MessengerError> {
        let mut entropy = [0u8; 16];
        rand::thread_rng().fill_bytes(&mut entropy);
        core_ffi::ffi_mnemonic_generate_12(&entropy)
            .map_err(MessengerError::MnemonicError)
    }

    pub fn validate(phrase: &str) -> bool {
        let words: Vec<&str> = phrase.split_whitespace().collect();
        if words.len() != 12 {
            return false;
        }
        core_ffi::ffi_mnemonic_validate(phrase.trim())
    }

    pub fn to_seed(phrase: &str) -> Result<[u8; 32], MessengerError> {
        if !Self::validate(phrase) {
            return Err(MessengerError::MnemonicError("Invalid BIP-39 mnemonic phrase".into()));
        }
        core_ffi::ffi_mnemonic_to_seed(phrase.trim())
            .map_err(MessengerError::MnemonicError)
    }
}
