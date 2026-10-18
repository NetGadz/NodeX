use ed25519_dalek::SigningKey;
use nodex_blockchain::{Block, Transaction, TransactionReceipt};
use nodex_storage::BlockchainStorage;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::mempool::Mempool;

pub const BLOCK_INTERVAL_SECS: u64 = 3;

pub struct PoSEngine {
    pub validator_key: SigningKey,
    pub storage: Arc<BlockchainStorage>,
    pub mempool: Arc<Mempool>,
}

impl PoSEngine {
    pub fn new(
        validator_key: SigningKey,
        storage: Arc<BlockchainStorage>,
        mempool: Arc<Mempool>,
    ) -> Self {
        Self {
            validator_key,
            storage,
            mempool,
        }
    }

    pub fn produce_next_block(&self) -> Result<Block, String> {
        let latest_height = self.storage.get_latest_height()?;
        let parent_block = self
            .storage
            .get_block_by_number(latest_height)?
            .ok_or_else(|| format!("Parent block at height {} not found", latest_height))?;

        let parent_hash = parent_block.header.hash();
        let next_height = latest_height + 1;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let candidate_txs = self.mempool.get_pending_sorted_by_fee(100);
        let validator_pub = self.validator_key.verifying_key().to_bytes();

        let mut valid_txs = Vec::new();
        let mut receipts = Vec::new();

        for tx in candidate_txs {
            if let Ok(receipt) = self.storage.apply_transaction(&tx, &validator_pub) {
                valid_txs.push(tx);
                receipts.push(receipt);
            }
        }

        let new_block = Block::new(
            &self.validator_key,
            parent_hash,
            next_height,
            now,
            [0u8; 32],
            valid_txs.clone(),
            receipts,
        );

        self.storage.put_block(&new_block)?;
        self.mempool.remove_transactions(&valid_txs);

        Ok(new_block)
    }
}
