use nodex_blockchain::Transaction;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct Mempool {
    pending_txs: Mutex<HashMap<[u8; 32], Transaction>>,
}

impl Default for Mempool {
    fn default() -> Self {
        Self {
            pending_txs: Mutex::new(HashMap::new()),
        }
    }
}

impl Mempool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_transaction(&self, tx: Transaction) -> Result<(), String> {
        tx.verify_signature()?;
        let hash = tx.signing_hash();
        let mut guard = self.pending_txs.lock().map_err(|e| e.to_string())?;
        guard.insert(hash, tx);
        Ok(())
    }

    pub fn get_pending_sorted_by_fee(&self, limit: usize) -> Vec<Transaction> {
        if let Ok(guard) = self.pending_txs.lock() {
            let mut list: Vec<Transaction> = guard.values().cloned().collect();
            // Sort highest fee first
            list.sort_by(|a, b| b.fee.cmp(&a.fee));
            list.into_iter().take(limit).collect()
        } else {
            Vec::new()
        }
    }

    pub fn remove_transactions(&self, txs: &[Transaction]) {
        if let Ok(mut guard) = self.pending_txs.lock() {
            for tx in txs {
                guard.remove(&tx.signing_hash());
            }
        }
    }

    pub fn len(&self) -> usize {
        self.pending_txs.lock().map(|g| g.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
