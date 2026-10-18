#![forbid(unsafe_code)]

use nodex_types::Hash;
use rocksdb::{Options, DB};
use std::path::Path;

pub struct RocksStorage {
    db: DB,
}

impl RocksStorage {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).map_err(|e| format!("Failed to open RocksDB: {}", e))?;
        Ok(Self { db })
    }

    pub fn put_block_bytes(&self, height: u64, block_bytes: &[u8]) -> Result<(), String> {
        self.db
            .put(height.to_be_bytes(), block_bytes)
            .map_err(|e| format!("RocksDB put error: {}", e))
    }

    pub fn get_block_bytes(&self, height: u64) -> Result<Option<Vec<u8>>, String> {
        self.db
            .get(height.to_be_bytes())
            .map_err(|e| format!("RocksDB get error: {}", e))
    }
}
