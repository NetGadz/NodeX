#![forbid(unsafe_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use nodex_sdk::{Address, NDX20Token};

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct MultisigWallet {
    pub owners: Vec<Address>,
    pub required_signatures: u8,
    pub tx_nonce: u64,
}

impl MultisigWallet {
    pub fn new(owners: Vec<Address>, required_signatures: u8) -> Self {
        Self {
            owners,
            required_signatures,
            tx_nonce: 0,
        }
    }
}

pub fn create_sample_ndx20() -> NDX20Token {
    NDX20Token {
        name: b"NodeX Wrapped Token".to_vec(),
        symbol: b"wNDX".to_vec(),
        decimals: 9,
        total_supply: 1_000_000_000_000_000_000,
    }
}
