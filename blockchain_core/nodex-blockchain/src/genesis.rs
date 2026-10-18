use ed25519_dalek::SigningKey;
use crate::models::{Block, BlockHeader};

pub const MAX_SUPPLY_NDX: u64 = 100_000_000;

pub struct GenesisConfig;

impl GenesisConfig {
    pub fn build_genesis_block(genesis_validator_key: &SigningKey) -> Block {
        let header = BlockHeader {
            parent_hash: [0u8; 32],
            block_number: 0,
            timestamp: 1700000000,
            tx_merkle_root: [0u8; 32],
            state_root: [0u8; 32],
            receipts_root: [0u8; 32],
            validator: genesis_validator_key.verifying_key().to_bytes(),
            gas_used: 0,
            gas_limit: 30_000_000,
            extra_data: b"NodeX Sovereign Layer-1 Genesis Block".to_vec(),
        };

        Block::new(
            genesis_validator_key,
            header.parent_hash,
            0,
            header.timestamp,
            [0u8; 32],
            Vec::new(),
            Vec::new(),
        )
    }
}
