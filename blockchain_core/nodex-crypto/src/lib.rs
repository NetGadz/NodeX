#![forbid(unsafe_code)]

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use nodex_types::{Address, Hash};
use zeroize::Zeroize;

pub fn hash_blake3(domain_prefix: &str, data: &[u8]) -> Hash {
    let mut hasher = blake3::Hasher::new();
    hasher.update(domain_prefix.as_bytes());
    hasher.update(data);
    *hasher.finalize().as_bytes()
}

pub fn derive_address(pubkey: &[u8; 32]) -> Address {
    hash_blake3("NDX_ADDR_V1", pubkey)
}

pub fn derive_contract_address(deployer: &Address, nonce: u64, code_hash: &Hash, salt: &[u8; 32]) -> Address {
    let mut data = Vec::new();
    data.extend_from_slice(deployer);
    data.extend_from_slice(&nonce.to_be_bytes());
    data.extend_from_slice(code_hash);
    data.extend_from_slice(salt);
    hash_blake3("NDX_CONTRACT_V1", &data)
}

pub fn verify_ed25519(pubkey_bytes: &[u8; 32], message: &[u8], sig_bytes: &[u8; 64]) -> Result<(), String> {
    let vk = VerifyingKey::from_bytes(pubkey_bytes)
        .map_err(|e| format!("Invalid Ed25519 pubkey: {}", e))?;
    let sig = Signature::from_bytes(sig_bytes);
    vk.verify(message, &sig)
        .map_err(|e| format!("Ed25519 signature verification failed: {}", e))
}

#[derive(Zeroize)]
#[zeroize(drop)]
pub struct KeyPair {
    pub secret: [u8; 32],
}

impl KeyPair {
    pub fn from_secret_bytes(bytes: &[u8; 32]) -> Self {
        Self { secret: *bytes }
    }

    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from_bytes(&self.secret)
    }

    pub fn verifying_key_bytes(&self) -> [u8; 32] {
        self.signing_key().verifying_key().to_bytes()
    }

    pub fn address(&self) -> Address {
        derive_address(&self.verifying_key_bytes())
    }

    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let sk = self.signing_key();
        let sig = sk.sign(message);
        sig.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_derivation() {
        let kp = KeyPair::from_secret_bytes(&[7u8; 32]);
        let addr = kp.address();
        assert_ne!(addr, [0u8; 32]);
    }

    #[test]
    fn test_signature_verification() {
        let kp = KeyPair::from_secret_bytes(&[9u8; 32]);
        let msg = b"NDX_TEST_MSG";
        let sig = kp.sign(msg);
        let vk = kp.verifying_key_bytes();
        assert!(verify_ed25519(&vk, msg, &sig).is_ok());
    }
}
