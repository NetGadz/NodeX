#![forbid(unsafe_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use nodex_crypto::hash_blake3;
use nodex_exec::Transaction;
use nodex_types::{Address, Hash};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct BlockHeader {
    pub version: u16,
    pub chain_id: u32,
    pub height: u64,
    pub timestamp_ms: u64,
    pub parent_hash: Hash,
    pub proposer: Address,
    pub tx_root: Hash,
    pub receipts_root: Hash,
    pub state_root: Hash,
    pub validators_hash: Hash,
    pub next_validators_hash: Hash,
    pub gas_used: u64,
    pub gas_limit: u64,
    pub base_fee_per_gas: u128,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let bytes = borsh::to_vec(self).unwrap_or_default();
        hash_blake3("NDX_BLOCK_HEADER_V1", &bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub txs: Vec<Transaction>,
}

pub trait Application {
    fn check_tx(&self, tx: &[u8]) -> Result<(), String>;
    fn prepare_proposal(&self, max_bytes: usize, max_gas: u64) -> Vec<Vec<u8>>;
    fn process_proposal(&self, block: &Block) -> bool;
    fn finalize_block(&mut self, block: &Block) -> Hash;
    fn commit(&mut self) -> Hash;
}
