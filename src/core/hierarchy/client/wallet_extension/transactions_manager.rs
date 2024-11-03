use crate::core::error::errors::SystemError;
use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::{
    Transaction, WalletBalanceTracker,
};
use crate::core::zkps::plonky2::Plonky2System;
use crate::core::zkps::proof::{ProofType, ZkProof};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

pub struct TransactionManager {
    plonky2_system: Arc<Mutex<Plonky2System>>,
    state_tracker: Arc<Mutex<WalletBalanceTracker>>,
    merkle_tree: Arc<Mutex<SparseMerkleTreeWasm>>,
}

impl TransactionManager {
    pub fn new() -> Self {
        Self {
            plonky2_system: Arc::new(Mutex::new(Plonky2System::new().unwrap())),
            state_tracker: Arc::new(Mutex::new(WalletBalanceTracker::new())),
            merkle_tree: Arc::new(Mutex::new(SparseMerkleTreeWasm::new())),
        }
    }

    pub fn create_transaction(
        &self,
        channel_id: [u8; 32],
        amount: u64,
        recipient: [u8; 32],
        secret_key: &[u8],
    ) -> Result<(Transaction, ZkProof), SystemError> {
        // Get current channel state
        let state_tracker = self.state_tracker.lock().unwrap();
        let current_balance = state_tracker.get_current_balance(&channel_id)?;
        let current_state = state_tracker.get_current_state(&channel_id)?;

        // Create transaction
        let tx = Transaction {
            sender: channel_id,
            recipient,
            amount,
            nonce: current_state.nonce + 1,
            sequence_number: current_state.seqno + 1,
            timestamp: current_timestamp(),
            id: todo!(),
            channel_id,
            status: todo!(),
            signature: todo!(),
            zk_proof: todo!(),
            merkle_proof: todo!(),
            previous_state: todo!(),
            new_state: todo!(),
            fee: todo!(),
        };

        // Generate zk-SNARK proof
        let proof = self.generate_transaction_proof(&tx, secret_key)?;

        Ok((tx, proof))
    }

    fn generate_transaction_proof(
        &self,
        tx: &Transaction,
        secret_key: &[u8],
    ) -> Result<ZkProof, SystemError> {
        let plonky2 = self.plonky2_system.lock().unwrap();

        // Extract public inputs
        let public_inputs = self.extract_public_inputs(tx)?;

        // Construct witness
        let witness = self.construct_witness(tx, secret_key)?;

        // Generate proof
        let proof =
            plonky2.generate_proof(&public_inputs, &witness, ProofType::Transaction, None, None)?;
        Ok(proof)
    }

    pub fn verify_transaction(
        &self,
        tx: &Transaction,
        proof: &ZkProof,
    ) -> Result<bool, SystemError> {
        // Verify proof
        let plonky2 = self.plonky2_system.lock().unwrap();
        let state_tracker = self.state_tracker.lock().unwrap();

        // Verify balance transition
        let is_valid = state_tracker.verify_balance_transition(
            &tx.sender,
            state_tracker.get_current_balance(&tx.sender)?,
            state_tracker.get_current_balance(&tx.sender)? - tx.amount,
            proof,
        )?;

        if !is_valid {
            return Ok(false);
        }

        // Update state if valid
        if is_valid {
            let mut merkle_tree = self.merkle_tree.lock().unwrap();
            let mut state_tracker = self.state_tracker.lock().unwrap();

            // Create channel contract
            let mut channel = crate::core::hierarchy::ChannelContract::new(tx.sender);

            // Process transaction
            let state_boc = channel.process_transaction(tx.clone())?;

            // Update state tracking
            state_tracker.track_balance_update(
                tx.sender,
                state_tracker.get_current_balance(&tx.sender)?,
                state_tracker.get_current_balance(&tx.sender)? - tx.amount,
                state_boc.clone(),
            )?;

            // Update merkle tree
            merkle_tree.update(&tx.sender.to_vec(), &state_boc.serialize()?)?;
        }

        Ok(is_valid)
    }
    fn extract_public_inputs(&self, tx: &Transaction) -> Result<Vec<u64>, SystemError> {
        let mut inputs = Vec::new();
        inputs.push(tx.amount);
        inputs.push(tx.nonce);
        inputs.push(tx.sequence_number);
        Ok(inputs)
    }

    fn construct_witness(
        &self,
        tx: &Transaction,
        secret_key: &[u8],
    ) -> Result<Vec<u64>, SystemError> {
        let mut witness = Vec::new();

        // Add secret key
        let mut hasher = Sha256::new();
        hasher.update(secret_key);
        let key_hash = hasher.finalize();
        witness.extend(key_hash.iter().map(|&b| b as u64));

        // Add transaction data
        witness.push(tx.amount);
        witness.push(tx.nonce);
        witness.push(tx.sequence_number);

        Ok(witness)
    }
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_creation_and_verification() {
        let manager = TransactionManager::new();
        let channel_id = [1u8; 32];
        let recipient = [2u8; 32];
        let secret_key = b"test_key";

        // Create transaction
        let (tx, proof) = manager
            .create_transaction(channel_id, 100, recipient, secret_key)
            .unwrap();

        // Verify transaction
        let is_valid = manager.verify_transaction(&tx, &proof).unwrap();
        assert!(is_valid);
    }
}
