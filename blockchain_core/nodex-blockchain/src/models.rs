use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use crate::merkle::calculate_merkle_root;

pub const NANO_NDX_PER_NDX: u64 = 1_000_000_000;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransactionType {
    Transfer,
    StakeDeposit { amount: u64 },
    StakeWithdraw { amount: u64 },
    ContractDeploy { code: Vec<u8> },
    ContractCall { contract: [u8; 32], method: String, args: Vec<u8> },
    ValidatorRegister { commission_bps: u16 },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
    pub stake: u64,
    pub is_validator: bool,
    pub validator_commission_bps: u16,
    pub storage_root: [u8; 32],
    pub code_hash: Option<[u8; 32]>,
    pub storage: HashMap<String, Vec<u8>>,
}

impl Default for AccountState {
    fn default() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            stake: 0,
            is_validator: false,
            validator_commission_bps: 500, // 5% default
            storage_root: [0u8; 32],
            code_hash: None,
            storage: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Transaction {
    pub tx_type: TransactionType,
    pub sender: [u8; 32],
    pub recipient: [u8; 32],
    pub amount: u64,
    pub nonce: u64,
    pub fee: u64,
    pub gas_limit: u64,
    pub signature: Vec<u8>,
}

impl Transaction {
    pub fn new_transfer(
        signing_key: &SigningKey,
        recipient: [u8; 32],
        amount: u64,
        nonce: u64,
        fee: u64,
    ) -> Self {
        let sender = signing_key.verifying_key().to_bytes();
        let mut tx = Self {
            tx_type: TransactionType::Transfer,
            sender,
            recipient,
            amount,
            nonce,
            fee,
            gas_limit: 21000,
            signature: Vec::new(),
        };
        let hash = tx.signing_hash();
        let sig = signing_key.sign(&hash);
        tx.signature = sig.to_vec();
        tx
    }

    pub fn new_stake(
        signing_key: &SigningKey,
        amount: u64,
        nonce: u64,
        fee: u64,
    ) -> Self {
        let sender = signing_key.verifying_key().to_bytes();
        let mut tx = Self {
            tx_type: TransactionType::StakeDeposit { amount },
            sender,
            recipient: sender,
            amount,
            nonce,
            fee,
            gas_limit: 50000,
            signature: Vec::new(),
        };
        let hash = tx.signing_hash();
        let sig = signing_key.sign(&hash);
        tx.signature = sig.to_vec();
        tx
    }

    pub fn signing_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NODEX_ADVANCED_TX_V2");
        hasher.update(bincode::serialize(&self.tx_type).unwrap_or_default());
        hasher.update(&self.sender);
        hasher.update(&self.recipient);
        hasher.update(&self.amount.to_be_bytes());
        hasher.update(&self.nonce.to_be_bytes());
        hasher.update(&self.fee.to_be_bytes());
        hasher.update(&self.gas_limit.to_be_bytes());
        let res = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&res);
        out
    }

    pub fn verify_signature(&self) -> Result<(), String> {
        let vk = VerifyingKey::from_bytes(&self.sender)
            .map_err(|e| format!("Invalid sender Ed25519 public key: {}", e))?;
        if self.signature.len() != 64 {
            return Err("Signature must be 64 bytes".to_string());
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.signature);
        let sig = Signature::from_bytes(&sig_bytes);

        let hash = self.signing_hash();
        vk.verify(&hash, &sig)
            .map_err(|e| format!("Transaction signature verification failed: {}", e))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BlockHeader {
    pub parent_hash: [u8; 32],
    pub block_number: u64,
    pub timestamp: u64,
    pub tx_merkle_root: [u8; 32],
    pub state_root: [u8; 32],
    pub receipts_root: [u8; 32],
    pub validator: [u8; 32],
    pub gas_used: u64,
    pub gas_limit: u64,
    pub extra_data: Vec<u8>,
}

impl BlockHeader {
    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"NODEX_FULL_BLOCK_HEADER_V2");
        hasher.update(&self.parent_hash);
        hasher.update(&self.block_number.to_be_bytes());
        hasher.update(&self.timestamp.to_be_bytes());
        hasher.update(&self.tx_merkle_root);
        hasher.update(&self.state_root);
        hasher.update(&self.receipts_root);
        hasher.update(&self.validator);
        hasher.update(&self.gas_used.to_be_bytes());
        hasher.update(&self.gas_limit.to_be_bytes());
        hasher.update(&self.extra_data);
        let res = hasher.finalize();
        let mut out = [0u8; 32];
        out.copy_from_slice(&res);
        out
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransactionReceipt {
    pub tx_hash: [u8; 32],
    pub status: u8, // 1 = Success, 0 = Failed
    pub gas_used: u64,
    pub logs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
    pub receipts: Vec<TransactionReceipt>,
    pub signature: Vec<u8>,
}

impl Block {
    pub fn new(
        validator_key: &SigningKey,
        parent_hash: [u8; 32],
        block_number: u64,
        timestamp: u64,
        state_root: [u8; 32],
        transactions: Vec<Transaction>,
        receipts: Vec<TransactionReceipt>,
    ) -> Self {
        let tx_hashes: Vec<[u8; 32]> = transactions.iter().map(|tx| tx.signing_hash()).collect();
        let tx_merkle_root = calculate_merkle_root(&tx_hashes);

        let receipt_hashes: Vec<[u8; 32]> = receipts
            .iter()
            .map(|r| {
                let mut h = Sha256::new();
                h.update(&r.tx_hash);
                h.update(&[r.status]);
                h.update(&r.gas_used.to_be_bytes());
                let res = h.finalize();
                let mut out = [0u8; 32];
                out.copy_from_slice(&res);
                out
            })
            .collect();
        let receipts_root = calculate_merkle_root(&receipt_hashes);

        let total_gas: u64 = receipts.iter().map(|r| r.gas_used).sum();

        let header = BlockHeader {
            parent_hash,
            block_number,
            timestamp,
            tx_merkle_root,
            state_root,
            receipts_root,
            validator: validator_key.verifying_key().to_bytes(),
            gas_used: total_gas,
            gas_limit: 30_000_000,
            extra_data: Vec::new(),
        };

        let header_hash = header.hash();
        let sig = validator_key.sign(&header_hash);

        Self {
            header,
            transactions,
            receipts,
            signature: sig.to_vec(),
        }
    }

    pub fn verify_signature(&self) -> Result<(), String> {
        let vk = VerifyingKey::from_bytes(&self.header.validator)
            .map_err(|e| format!("Invalid validator Ed25519 public key: {}", e))?;
        if self.signature.len() != 64 {
            return Err("Validator signature must be 64 bytes".to_string());
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(&self.signature);
        let sig = Signature::from_bytes(&sig_bytes);

        let header_hash = self.header.hash();
        vk.verify(&header_hash, &sig)
            .map_err(|e| format!("Block validator signature verification failed: {}", e))
    }
}
