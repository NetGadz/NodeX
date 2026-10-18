#![forbid(unsafe_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Hash = [u8; 32];
pub type Address = [u8; 32];
pub type Signature = [u8; 64];

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TypeError {
    #[error("Invalid address length")]
    InvalidAddressLength,
    #[error("Invalid hash length")]
    InvalidHashLength,
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct TxStatus(pub u8);

impl TxStatus {
    pub const SUCCESS: Self = Self(1);
    pub const FAILED: Self = Self(0);
    pub const OUT_OF_GAS: Self = Self(2);
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Event {
    pub address: Address,
    pub topics: Vec<Hash>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Receipt {
    pub tx_hash: Hash,
    pub status: TxStatus,
    pub gas_used: u64,
    pub fee_paid: u128,
    pub contract_address: Option<Address>,
    pub return_data: Vec<u8>,
    pub events: Vec<Event>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_borsh_roundtrip() {
        let receipt = Receipt {
            tx_hash: [1u8; 32],
            status: TxStatus::SUCCESS,
            gas_used: 21000,
            fee_paid: 1000000,
            contract_address: None,
            return_data: vec![1, 2, 3],
            events: vec![],
        };

        let encoded = borsh::to_vec(&receipt).unwrap();
        let decoded: Receipt = borsh::from_slice(&encoded).unwrap();
        assert_eq!(receipt, decoded);
    }
}
