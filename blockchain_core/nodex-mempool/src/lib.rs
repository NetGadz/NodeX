#![forbid(unsafe_code)]

use nodex_exec::Transaction;
use nodex_state::StateTree;
use nodex_types::{Address, Hash};
use std::collections::HashMap;

pub struct Mempool {
    txs: HashMap<Hash, Transaction>,
    by_sender: HashMap<Address, Vec<Transaction>>,
}

impl Default for Mempool {
    fn default() -> Self {
        Self {
            txs: HashMap::new(),
            by_sender: HashMap::new(),
        }
    }
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, state: &StateTree, tx: Transaction) -> Result<(), String> {
        tx.verify_signature()?;
        let sender = tx.sender_address();
        let acc = state.get_account(&sender);

        if tx.nonce <= acc.nonce {
            return Err("Nonce too low".to_string());
        }

        let hash = tx.tx_hash();
        self.txs.insert(hash, tx.clone());
        self.by_sender.entry(sender).or_default().push(tx);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }
}
