#![forbid(unsafe_code)]

use borsh::{BorshDeserialize, BorshSerialize};
use nodex_crypto::{derive_address, hash_blake3, verify_ed25519};
use nodex_state::{Account, StateTree};
use nodex_types::{Address, Hash, Receipt, TxStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum TxBody {
    Transfer { to: Address, amount: u128 },
    ValidatorRegister { consensus_pubkey: [u8; 32], commission_bps: u16, self_stake: u128 },
    Stake { validator: Address, amount: u128 },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
pub struct Transaction {
    pub chain_id: u32,
    pub nonce: u64,
    pub gas_limit: u64,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
    pub body: TxBody,
    pub public_key: [u8; 32],
    pub signature: Vec<u8>,
}

impl Transaction {
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&self.chain_id.to_be_bytes());
        data.extend_from_slice(&self.nonce.to_be_bytes());
        data.extend_from_slice(&self.gas_limit.to_be_bytes());
        data.extend_from_slice(&self.max_fee_per_gas.to_be_bytes());
        data.extend_from_slice(&self.max_priority_fee_per_gas.to_be_bytes());
        data.extend_from_slice(&self.public_key);
        data.extend_from_slice(&borsh::to_vec(&self.body).unwrap_or_default());
        data
    }

    pub fn tx_hash(&self) -> Hash {
        let sb = self.signing_bytes();
        hash_blake3("NDX_TX_V1", &sb)
    }

    pub fn sender_address(&self) -> Address {
        derive_address(&self.public_key)
    }

    pub fn verify_signature(&self) -> Result<(), String> {
        if self.signature.len() != 64 {
            return Err("Signature must be 64 bytes".to_string());
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.signature);
        let sb = self.signing_bytes();
        let msg_hash = hash_blake3("NDX_TX_SIG_V1", &sb);
        verify_ed25519(&self.public_key, &msg_hash, &sig_bytes)
    }
}

pub struct Executor;

impl Executor {
    pub fn apply_transaction(state: &mut StateTree, tx: &Transaction, base_fee: u128) -> Result<Receipt, String> {
        tx.verify_signature()?;

        let sender_addr = tx.sender_address();
        let mut sender_acc = state.get_account(&sender_addr);

        if tx.nonce != sender_acc.nonce + 1 {
            return Err(format!("Invalid nonce: expected {}, got {}", sender_acc.nonce + 1, tx.nonce));
        }

        let max_cost = (tx.gas_limit as u128).checked_mul(tx.max_fee_per_gas).ok_or("Fee overflow")?;
        if sender_acc.balance < max_cost {
            return Err("Insufficient balance for gas limit".to_string());
        }

        match &tx.body {
            TxBody::Transfer { to, amount } => {
                let total_needed = max_cost.checked_add(*amount).ok_or("Amount overflow")?;
                if sender_acc.balance < total_needed {
                    return Err("Insufficient balance for transfer + gas".to_string());
                }

                sender_acc.balance -= total_needed;
                sender_acc.nonce += 1;
                state.set_account(sender_addr, sender_acc);

                let mut recipient_acc = state.get_account(to);
                recipient_acc.balance += amount;
                state.set_account(*to, recipient_acc);

                let gas_used = 21000;
                let actual_fee = (gas_used as u128) * base_fee;

                // Refund unspent gas limit fee
                let mut sender_final = state.get_account(&sender_addr);
                sender_final.balance += max_cost.saturating_sub(actual_fee);
                state.set_account(sender_addr, sender_final);

                Ok(Receipt {
                    tx_hash: tx.tx_hash(),
                    status: TxStatus::SUCCESS,
                    gas_used,
                    fee_paid: actual_fee,
                    contract_address: None,
                    return_data: vec![],
                    events: vec![],
                })
            }
            _ => Err("Unsupported transaction type in v1 exec".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nodex_crypto::KeyPair;

    #[test]
    fn test_exec_transfer() {
        let mut state = StateTree::new();
        let sender_kp = KeyPair::from_secret_bytes(&[1u8; 32]);
        let recipient_kp = KeyPair::from_secret_bytes(&[2u8; 32]);

        let sender_addr = sender_kp.address();
        let recipient_addr = recipient_kp.address();

        state.set_account(sender_addr, Account {
            balance: 10_000_000_000,
            nonce: 0,
            code_hash: None,
            storage_deposit: 0,
        });

        let mut tx = Transaction {
            chain_id: 1,
            nonce: 1,
            gas_limit: 21000,
            max_fee_per_gas: 100,
            max_priority_fee_per_gas: 10,
            body: TxBody::Transfer { to: recipient_addr, amount: 1_000_000_000 },
            public_key: sender_kp.verifying_key_bytes(),
            signature: [0u8; 64],
        };

        let sb = tx.signing_bytes();
        let msg_hash = hash_blake3("NDX_TX_SIG_V1", &sb);
        tx.signature = sender_kp.sign(&msg_hash);

        let receipt = Executor::apply_transaction(&mut state, &tx, 50).unwrap();
        assert_eq!(receipt.status, TxStatus::SUCCESS);
        assert_eq!(state.get_account(&recipient_addr).balance, 1_000_000_000);
    }
}
