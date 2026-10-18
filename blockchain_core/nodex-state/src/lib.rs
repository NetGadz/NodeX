#![forbid(unsafe_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use nodex_crypto::hash_blake3;
use nodex_types::{Address, Hash};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Account {
    pub balance: u128,
    pub nonce: u64,
    pub code_hash: Option<Hash>,
    pub storage_deposit: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct StateTree {
    accounts: HashMap<Address, Account>,
    storage_slots: HashMap<(Address, Hash), Hash>,
}

impl StateTree {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_account(&self, addr: &Address) -> Account {
        self.accounts.get(addr).cloned().unwrap_or_default()
    }

    pub fn set_account(&mut self, addr: Address, acc: Account) {
        self.accounts.insert(addr, acc);
    }

    pub fn get_storage(&self, addr: &Address, key: &Hash) -> Hash {
        self.storage_slots.get(&(*addr, *key)).cloned().unwrap_or([0u8; 32])
    }

    pub fn set_storage(&mut self, addr: Address, key: Hash, val: Hash) {
        self.storage_slots.insert((addr, key), val);
    }

    pub fn state_root(&self) -> Hash {
        let mut sorted_accounts: Vec<(&Address, &Account)> = self.accounts.iter().collect();
        sorted_accounts.sort_by_key(|(addr, _)| *addr);

        let mut data = Vec::new();
        data.extend_from_slice(b"NDX_STATE_ROOT_V1");
        for (addr, acc) in sorted_accounts {
            data.extend_from_slice(addr);
            let bytes = borsh::to_vec(acc).unwrap_or_default();
            data.extend_from_slice(&bytes);
        }

        hash_blake3("NDX_STATE_PREFIX", &data)
    }
}
