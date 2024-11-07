use crate::core::error::errors::Error;
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::Transaction;
use sha2::Digest;
use std::collections::{HashMap, HashSet};

pub struct TreeManager {
    channel_states: HashMap<[u8; 32], ChannelState>,
    processed_transactions: HashSet<[u8; 32]>,
}

impl TreeManager {
    pub fn new() -> Self {
        TreeManager {
            channel_states: HashMap::new(),
            processed_transactions: HashSet::new(),
        }
    }

    pub fn process_transaction(&mut self, transaction: &Transaction) -> Result<(), Error> {
        // Verify transaction hasn't been processed
        if self.processed_transactions.contains(&transaction.id) {
            return Err(Error::InvalidTransaction);
        }

        // Verify transaction state matches channel state
        if let Some(current_state) = self.channel_states.get(&transaction.channel_id) {
            if transaction.nonce != current_state.nonce + 1 {
                return Err(Error::InvalidNonce);
            }

            if transaction.sequence_number != current_state.sequence_number + 1 {
                return Err(Error::InvalidSequence);
            }

            // Verify signature
            if !self.verify_signature(transaction, &current_state.signatures)? {
                return Err(Error::InvalidSignature);
            }
        } else {
            // For new channels, verify initial state
            if transaction.nonce != 0 || transaction.sequence_number != 0 {
                return Err(Error::InvalidTransaction);
            }
        }

        // Update channel state
        let new_state = self.apply_state_change(transaction)?;
        self.update_channel_state(transaction.channel_id, new_state)?;

        // Mark transaction as processed
        self.processed_transactions.insert(transaction.id);

        Ok(())
    }

    pub fn update_channel_state(
        &mut self,
        channel_id: [u8; 32],
        new_state: ChannelState,
    ) -> Result<(), Error> {
        // Validate state transition
        if let Some(current_state) = self.channel_states.get(&channel_id) {
            if !self.is_valid_state_transition(current_state, &new_state) {
                return Err(Error::InvalidChannel);
            }
        }

        // Store new state
        self.channel_states.insert(channel_id, new_state);
        Ok(())
    }

    pub fn generate_wallet_root(&self) -> Result<[u8; 32], Error> {
        let mut state_bytes = self.serialize_wallet_state();
        state_bytes.sort(); // Ensure deterministic ordering

        let mut hasher = sha2::Sha256::new();
        hasher.update(&state_bytes);
        let mut root = [0u8; 32];
        root.copy_from_slice(&hasher.finalize());
        Ok(root)
    }

    pub fn serialize_wallet_state(&self) -> Vec<u8> {
        let mut serialized = Vec::new();
        let mut states: Vec<_> = self.channel_states.iter().collect();
        states.sort_by_key(|&(k, _)| k); // Deterministic ordering

        for (channel_id, state) in states {
            serialized.extend_from_slice(channel_id);
            serialized.extend_from_slice(&state.serialize());
        }
        serialized
    }

    pub fn get_channel_state(&self, channel_id: &[u8; 32]) -> Option<&ChannelState> {
        self.channel_states.get(channel_id)
    }

    pub fn verify_signature(
        &self,
        transaction: &Transaction,
        signatures: &[[u8; 64]],
    ) -> Result<bool, Error> {
        // Verify transaction signature against latest channel state signatures
        // In a real implementation, this would use proper signature verification

        if signatures.is_empty() {
            return Ok(false);
        }

        // Verify signature matches one of the authorized signers
        let mut message = Vec::new();
        message.extend_from_slice(&transaction.channel_id);
        message.extend_from_slice(&transaction.nonce.to_le_bytes());
        message.extend_from_slice(&transaction.amount.to_le_bytes());

        let mut hasher = sha2::Sha256::new();
        hasher.update(&message);
        let message_hash = hasher.finalize();

        // In a real implementation, we would verify the signature against message_hash
        // For now, just check signature length is valid
        if transaction.signature.len() != 64 {
            return Ok(false);
        }

        Ok(true)
    }

    fn is_valid_state_transition(&self, current: &ChannelState, new: &ChannelState) -> bool {
        // Verify nonce increments by 1
        if new.nonce != current.nonce + 1 {
            return false;
        }

        // Verify sequence number increments by 1
        if new.sequence_number != current.sequence_number + 1 {
            return false;
        }

        // Verify balance changes are valid
        if new.balance < 0 {
            return false;
        }

        // Verify signatures are present
        if new.signatures.is_empty() {
            return false;
        }

        true
    }

    fn apply_state_change(&self, transaction: &Transaction) -> Result<ChannelState, Error> {
        let current_state = self
            .channel_states
            .get(&transaction.channel_id)
            .cloned()
            .unwrap_or_else(|| ChannelState::new());

        Ok(ChannelState {
            nonce: current_state.nonce + 1,
            sequence_number: current_state.sequence_number + 1,
            balance: current_state.balance + transaction.amount as i64,
            signatures: vec![transaction.signature],
            timestamp: transaction.timestamp,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ChannelState {
    pub nonce: u64,
    pub sequence_number: u64,
    pub balance: i64,
    pub signatures: Vec<[u8; 64]>,
    pub timestamp: u64,
}

impl ChannelState {
    pub fn new() -> Self {
        Self {
            nonce: 0,
            sequence_number: 0,
            balance: 0,
            signatures: Vec::new(),
            timestamp: 0,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.nonce.to_le_bytes());
        bytes.extend_from_slice(&self.sequence_number.to_le_bytes());
        bytes.extend_from_slice(&self.balance.to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());

        // Serialize signatures
        bytes.extend_from_slice(&(self.signatures.len() as u64).to_le_bytes());
        for signature in &self.signatures {
            bytes.extend_from_slice(signature);
        }

        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn create_test_transaction(channel_id: [u8; 32], nonce: u64, amount: u64) -> Transaction {
        Transaction {
            id: [1u8; 32],
            channel_id,
            sender: [2u8; 32],
            recipient: [3u8; 32],
            amount,
            nonce,
            sequence_number: nonce, // For testing, make these match
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            status: crate::core::hierarchy::client::wallet_extension::wallet_extension_types::TransactionStatus::Pending,
            signature: [4u8; 64],
            zk_proof: vec![],
            merkle_proof: vec![],
            previous_state: vec![],
            new_state: vec![],
            fee: 0,
        }
    }

    #[test]
    fn test_process_transaction() {
        let mut manager = TreeManager::new();
        let channel_id = [5u8; 32];

        // Process first transaction
        let tx1 = create_test_transaction(channel_id, 0, 100);
        assert!(manager.process_transaction(&tx1).is_ok());

        // Verify state update
        let state = manager.get_channel_state(&channel_id).unwrap();
        assert_eq!(state.nonce, 1);
        assert_eq!(state.balance, 100);

        // Try processing same transaction again
        assert!(manager.process_transaction(&tx1).is_err());

        // Process second transaction
        let tx2 = create_test_transaction(channel_id, 1, 50);
        assert!(manager.process_transaction(&tx2).is_ok());

        // Verify updated state
        let state = manager.get_channel_state(&channel_id).unwrap();
        assert_eq!(state.nonce, 2);
        assert_eq!(state.balance, 150);
    }

    #[test]
    fn test_wallet_root() {
        let mut manager = TreeManager::new();
        let channel_id = [5u8; 32];

        // Add some transactions
        let tx1 = create_test_transaction(channel_id, 0, 100);
        let tx2 = create_test_transaction(channel_id, 1, 50);

        assert!(manager.process_transaction(&tx1).is_ok());
        assert!(manager.process_transaction(&tx2).is_ok());

        // Generate root
        let root1 = manager.generate_wallet_root().unwrap();

        // Same state should generate same root
        let root2 = manager.generate_wallet_root().unwrap();
        assert_eq!(root1, root2);

        // Different state should generate different root
        let tx3 = create_test_transaction(channel_id, 2, 75);
        assert!(manager.process_transaction(&tx3).is_ok());

        let root3 = manager.generate_wallet_root().unwrap();
        assert_ne!(root1, root3);
    }
}
