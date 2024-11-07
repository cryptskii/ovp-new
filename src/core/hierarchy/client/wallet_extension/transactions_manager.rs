use crate::core::error::errors::{Error, SystemError, SystemErrorType};
use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;

use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::{
    Transaction, TransactionStatus,
};
use crate::core::types::boc::{Cell, BOC};
use crate::core::zkps::proof::{ProofGenerator, ZkProof};
use serde_wasm_bindgen;
use std::sync::{Arc, Mutex};
use wasm_bindgen::JsValue;

// Error conversions

impl From<crate::core::error::errors::Error> for SystemError {
    fn from(err: crate::core::error::errors::Error) -> Self {
        SystemError::new(
            SystemErrorType::InvalidTransaction,
            format!("State tracking error: {}", err),
        )
    }
}
impl From<JsValue> for SystemError {
    fn from(err: JsValue) -> Self {
        SystemError::new(
            SystemErrorType::InvalidTransaction,
            format!("JS error: {:?}", err),
        )
    }
}

impl From<serde_wasm_bindgen::Error> for SystemError {
    fn from(err: serde_wasm_bindgen::Error) -> Self {
        SystemError::new(
            SystemErrorType::InvalidTransaction,
            format!("Serialization error: {}", err),
        )
    }
}

// Remove the conflicting implementation
// impl From<errors::Error> for SystemError {
//     fn from(err: errors::Error) -> Self {
//         match err {
//             errors::Error::SerializationError(msg) => SystemError::new(
//                 SystemErrorType::InvalidTransaction,
//                 format!("BOC serialization error: {}", msg),
//             ),
//             _ => SystemError::new(
//                 SystemErrorType::InvalidTransaction,
//                 format!("BOC error: {:?}", err),
//             ),
//         }
//     }
// }

pub struct TransactionManager {
    proof_generator: Arc<Mutex<ProofGenerator>>,
    state_tracker: Arc<Mutex<WalletBalanceTracker>>,
    merkle_tree: Arc<Mutex<SparseMerkleTreeWasm>>,
}

impl TransactionManager {
    pub fn new() -> Result<Self, SystemError> {
        Ok(Self {
            proof_generator: Arc::new(Mutex::new(ProofGenerator::try_new().map_err(|e| {
                SystemError::new(
                    SystemErrorType::InvalidTransaction,
                    format!("Failed to initialize proof system: {:?}", e),
                )
            })?)),
            state_tracker: Arc::new(Mutex::new(WalletBalanceTracker::new())),
            merkle_tree: Arc::new(Mutex::new(SparseMerkleTreeWasm::new())),
        })
    }

    pub fn create_transaction(
        &self,
        channel_id: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
    ) -> Result<(Transaction, ZkProof), SystemError> {
        let state_tracker = self.state_tracker.lock().map_err(|_| {
            SystemError::new(
                SystemErrorType::InvalidTransaction,
                "Failed to acquire state tracker lock".to_string(),
            )
        })?;

        let current_balance = state_tracker.get_current_balance(&channel_id)?;
        let new_balance = current_balance.checked_sub(amount).ok_or_else(|| {
            SystemError::new(
                SystemErrorType::InsufficientBalance,
                "Insufficient balance for transaction".to_string(),
            )
        })?;

        let proof_generator = self.proof_generator.lock().map_err(|_| {
            SystemError::new(
                SystemErrorType::InvalidTransaction,
                "Failed to acquire proof generator lock".to_string(),
            )
        })?;

        // Convert channel_id for proof
        let channel_id_vec = channel_id.to_vec();
        let channel_id_boxed: Box<[u8]> = channel_id_vec.into_boxed_slice();

        let proof_result = proof_generator.generate_state_transition_proof(
            current_balance,
            new_balance,
            amount,
            Some(channel_id_boxed),
        )?;

        let proof: ZkProof = serde_wasm_bindgen::from_value(proof_result)?;

        let tx = Transaction {
            id: self.generate_id(channel_id, amount, recipient),
            channel_id,
            sender: channel_id,
            recipient,
            amount,
            nonce: 0,
            sequence_number: 0,
            timestamp: Self::get_timestamp(),
            status: TransactionStatus::Pending,
            signature: [0u8; 64].into(),
            zk_proof: proof.proof_data.clone(),
            merkle_proof: vec![],
            previous_state: vec![],
            new_state: vec![],
            fee: 0,
        };

        Ok((tx, proof))
    }

    pub fn verify_transaction(
        &self,
        tx: &Transaction,
        proof: &ZkProof,
    ) -> Result<bool, SystemError> {
        let proof_generator = self.proof_generator.lock().map_err(|_| {
            SystemError::new(
                SystemErrorType::InvalidTransaction,
                "Failed to acquire proof generator lock".to_string(),
            )
        })?;

        let mut state_tracker = self.state_tracker.lock().map_err(|_| {
            SystemError::new(
                SystemErrorType::InvalidTransaction,
                "Failed to acquire state tracker lock".to_string(),
            )
        })?;

        let current_balance = state_tracker.get_current_balance(&tx.channel_id)?;
        let new_balance = current_balance.checked_sub(tx.amount).ok_or_else(|| {
            SystemError::new(
                SystemErrorType::InsufficientBalance,
                "Insufficient balance for transaction".to_string(),
            )
        })?;

        let proof_js = serde_wasm_bindgen::to_value(proof)?;

        let is_valid = proof_generator.verify_state_transition(
            &proof_js,
            current_balance,
            new_balance,
            tx.amount,
        )?;

        if is_valid {
            let mut merkle_tree = self.merkle_tree.lock().map_err(|_| {
                SystemError::new(
                    SystemErrorType::InvalidTransaction,
                    "Failed to acquire merkle tree lock".to_string(),
                )
            })?;

            let mut state_boc = BOC::new();
            let state_bytes = self.serialize_state(new_balance)?;
            let state_cell = Cell::from_data(state_bytes);
            state_boc.add_cell(state_cell);
            state_boc.add_root(0);

            let serialized_boc = state_boc
                .serialize()
                .map_err(|e| errors::Error::SerializationError(e.to_string()))?;

            merkle_tree
                .update(&tx.channel_id, &serialized_boc)
                .map_err(|e| {
                    SystemError::new(
                        SystemErrorType::InvalidTransaction,
                        format!("Merkle tree update failed: {:?}", e),
                    )
                })?;

            state_tracker.track_balance_update(
                tx.channel_id,
                current_balance,
                new_balance,
                state_boc,
            )?;
        }

        Ok(is_valid)
    }

    // Helper methods
    fn generate_id(&self, channel_id: [u8; 32], amount: u64, recipient: [u8; 32]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(channel_id);
        hasher.update(amount.to_le_bytes());
        hasher.update(recipient);
        hasher.update(Self::get_timestamp().to_le_bytes());
        let result = hasher.finalize();
        let mut tx_id = [0u8; 32];
        tx_id.copy_from_slice(&result);
        tx_id
    }

    fn get_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    fn serialize_state(&self, balance: u64) -> Result<Vec<u8>, SystemError> {
        let mut state = Vec::with_capacity(8);
        state.extend_from_slice(&balance.to_le_bytes());
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation_and_verification() {
        let manager = TransactionManager::new().unwrap();
        let channel_id = [1u8; 32];
        let recipient = [2u8; 32];
        let amount = 100;

        let (tx, proof) = manager
            .create_transaction(channel_id, amount, recipient)
            .unwrap();

        let is_valid = manager.verify_transaction(&tx, &proof).unwrap();
        assert!(is_valid);
    }
}
