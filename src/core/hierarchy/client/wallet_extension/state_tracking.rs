// src/core/hierarchy/client/wallet_extension/state_tracking.rs

use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::{
    PrivateChannelState, StateTransition,
};
use crate::core::types::boc::{Cell, BOC};
use crate::core::zkps::proof::ZkProof;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasm_bindgen::JsValue;

/// Custom Error type for this module
#[derive(Debug)]
pub enum Error {
    CustomError(String),
    SerializationError(String),
    DeserializationError(String),
    LockError(String),
    ChannelNotFound(String),
    StateNotFound(String),
    InvalidBOC(String),
    ArithmeticError(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CustomError(msg) => write!(f, "Custom Error: {}", msg),
            Error::SerializationError(msg) => write!(f, "Serialization Error: {}", msg),
            Error::DeserializationError(msg) => write!(f, "Deserialization Error: {}", msg),
            Error::LockError(msg) => write!(f, "Lock Error: {}", msg),
            Error::ChannelNotFound(msg) => write!(f, "Channel Not Found: {}", msg),
            Error::StateNotFound(msg) => write!(f, "State Not Found: {}", msg),
            Error::InvalidBOC(msg) => write!(f, "Invalid BOC: {}", msg),
            Error::ArithmeticError(msg) => write!(f, "Arithmetic Error: {}", msg),
        }
    }
}

impl std::error::Error for Error {}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerializationError(err.to_string())
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_err: std::sync::PoisonError<T>) -> Self {
        Error::LockError("Lock Poisoned".to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::CustomError(format!("IO Error: {}", err))
    }
}

impl From<JsValue> for Error {
    fn from(err: JsValue) -> Self {
        Error::CustomError(format!("JsValue Error: {:?}", err))
    }
}

/// Tracks wallet balances and state transitions
#[derive(Clone, Debug)]
pub struct WalletBalanceTracker {
    pub wallet_balances: HashMap<[u8; 32], u64>, // channel_id -> balance
    pub state_transitions: HashMap<[u8; 32], Vec<BOC>>, // channel_id -> state BOCs
    pub state_tree: Arc<RwLock<SparseMerkleTreeWasm>>, // SMT for state tracking
    last_root: [u8; 32],
    pending_updates: Vec<StateTransition>,
}

impl WalletBalanceTracker {
    pub fn new() -> Self {
        Self {
            wallet_balances: HashMap::new(),
            state_transitions: HashMap::new(),
            state_tree: Arc::new(RwLock::new(SparseMerkleTreeWasm::new())), // 256-bit keys
            last_root: [0u8; 32],
            pending_updates: Vec::new(),
        }
    }

    pub fn track_balance_update(
        &mut self,
        channel_id: [u8; 32],
        old_balance: u64,
        new_balance: u64,
        state_boc: BOC,
    ) -> Result<(), Error> {
        // Update balance tracking
        self.wallet_balances.insert(channel_id, new_balance);

        // Add state BOC to transition history
        self.state_transitions
            .entry(channel_id)
            .or_insert_with(Vec::new)
            .push(state_boc.clone());

        // Create state transition record
        let transition = StateTransition {
            old_state: self.get_previous_state(&channel_id)?,
            new_state: self.extract_state_from_boc(&state_boc)?,
            proof: self.generate_transition_proof(&channel_id, old_balance, new_balance)?,
            timestamp: current_timestamp(),
        };

        // Add to pending updates
        self.pending_updates.push(transition);

        // Update state tree
        let mut state_tree = self.state_tree.write().map_err(|e| Error::from(e))?;

        let serialized_boc = state_boc
            .serialize()
            .map_err(|e| Error::CustomError(format!("Serialization error: {}", e)))?;

        state_tree
            .update(&channel_id, &serialized_boc)
            .map_err(|e| Error::CustomError(format!("State tree update error: {:?}", e)))?;

        // Update last root
        let new_root = state_tree.root();
        self.last_root.copy_from_slice(&new_root);

        Ok(())
    }
    pub fn get_current_balance(&self, channel_id: &[u8; 32]) -> Result<u64, Error> {
        self.wallet_balances
            .get(channel_id)
            .copied()
            .ok_or_else(|| Error::ChannelNotFound("Channel not found".to_string()))
    }

    pub fn verify_balance_transition(
        &self,
        channel_id: &[u8; 32],
        old_balance: u64,
        new_balance: u64,
        proof: &ZkProof,
    ) -> Result<bool, Error> {
        // Verify the proof
        let current_state = self.get_current_state(channel_id)?;

        // Create verification context
        let context = VerificationContext {
            channel_id: *channel_id,
            old_balance,
            new_balance,
            state_root: self.last_root,
            current_state,
        };

        // Verify transition proof
        let result = verify_plonk_proof(proof, &context)?;
        Ok(result)
    }

    pub fn get_state_history(&self, channel_id: &[u8; 32]) -> Result<Vec<BOC>, Error> {
        self.state_transitions
            .get(channel_id)
            .cloned()
            .ok_or_else(|| Error::ChannelNotFound("Channel not found".to_string()))
    }

    pub fn commit_pending_updates(&mut self) -> Result<[u8; 32], Error> {
        // Sort pending updates by timestamp
        self.pending_updates.sort_by_key(|update| update.timestamp);

        // Create aggregated update BOC
        let mut aggregated_boc = BOC::new();

        for update in self.pending_updates.drain(..) {
            // Add state data and proofs to BOC
            let state_data = serde_json::to_vec(&update.new_state)
                .map_err(|e| Error::CustomError(format!("Serialization error: {}", e)))?;
            let proof_data = serde_json::to_vec(&update.proof)
                .map_err(|e| Error::CustomError(format!("Serialization error: {}", e)))?;

            // Create Cells from data
            let state_cell = Cell::from_data(state_data);
            let proof_cell = Cell::from_data(proof_data);

            // Add cells for state and proof data
            aggregated_boc.add_cell(state_cell);
            aggregated_boc.add_cell(proof_cell);
        }

        // Update state tree with aggregated BOC
        let mut state_tree = self
            .state_tree
            .write()
            .map_err(|e| Error::CustomError(format!("Lock acquisition error: {:?}", e)))?;

        let serialized_boc = aggregated_boc
            .serialize()
            .map_err(|e| Error::CustomError(format!("Serialization error: {:?}", e)))?;

        state_tree
            .update(&self.last_root, &serialized_boc)
            .map_err(|e| Error::CustomError(format!("State tree update error: {:?}", e)))?;

        // Update last root
        let new_root = state_tree.root();
        self.last_root.copy_from_slice(&new_root);

        Ok(self.last_root)
    }

    // Helper methods
    fn get_previous_state(&self, channel_id: &[u8; 32]) -> Result<PrivateChannelState, Error> {
        let states = self
            .state_transitions
            .get(channel_id)
            .ok_or_else(|| Error::ChannelNotFound("Channel not found".to_string()))?;

        if let Some(last_boc) = states.last() {
            self.extract_state_from_boc(last_boc)
        } else {
            Err(Error::StateNotFound("No previous state found".to_string()))
        }
    }

    fn extract_state_from_boc(&self, boc: &BOC) -> Result<PrivateChannelState, Error> {
        // Use first cell as state data
        let state_cell = boc
            .cells
            .first()
            .ok_or_else(|| Error::InvalidBOC("BOC has no cells".to_string()))?;
        let state_data = state_cell.get_data();
        let state: PrivateChannelState = serde_json::from_slice(state_data)?;
        Ok(state)
    }

    fn get_current_state(&self, channel_id: &[u8; 32]) -> Result<PrivateChannelState, Error> {
        let state_tree = self
            .state_tree
            .read()
            .map_err(|e| Error::CustomError(format!("Lock acquisition error: {:?}", e)))?;

        // Get state BOC from tree
        let state_boc_bytes = state_tree
            .get(channel_id)
            .map_err(|e| Error::CustomError(format!("State tree error: {:?}", e)))?
            .ok_or_else(|| Error::StateNotFound("State not found in tree".to_string()))?;

        // Parse BOC and extract state
        let boc = BOC::deserialize(&state_boc_bytes)
            .map_err(|e| Error::CustomError(format!("BOC deserialization error: {:?}", e)))?;
        self.extract_state_from_boc(&boc)
    }

    fn generate_transition_proof(
        &self,
        channel_id: &[u8; 32],
        old_balance: u64,
        new_balance: u64,
    ) -> Result<ZkProof, Error> {
        let current_state = self.get_current_state(channel_id)?;

        // Create proof inputs
        let inputs = ProofInputs {
            channel_id: *channel_id,
            old_balance,
            new_balance,
            state_root: self.last_root,
            current_state,
        };

        // Generate PLONK proof
        let proof = generate_plonk_proof(&inputs)?;
        Ok(proof)
    }
}

/// Tracks root state transitions and aggregation
#[derive(Clone, Debug)]
pub struct RootStateTracker<WalletRoot> {
    pub root_history: Vec<WalletRoot>,
    current_epoch: u64,
    aggregation_threshold: u64,
    pending_roots: Vec<([u8; 32], u64)>,
}

impl<WalletRoot> RootStateTracker<WalletRoot> {
    pub fn new(aggregation_threshold: u64) -> Self {
        Self {
            root_history: Vec::new(),
            current_epoch: 0,
            aggregation_threshold,
            pending_roots: Vec::new(),
        }
    }

    pub fn track_root_update(
        &mut self,
        new_root: [u8; 32],
        merkle_proofs: Vec<[u8; 32]>,
        balance: u64,
    ) -> Result<Option<WalletRoot>, Error> {
        // Add to pending roots
        self.pending_roots.push((new_root, current_timestamp()));

        // Check if we should aggregate
        if self.pending_roots.len() as u64 >= self.aggregation_threshold {
            // Create aggregated root
            let aggregated_root = self.aggregate_pending_roots()?;

            // Clear pending roots
            self.pending_roots.clear();

            // Create wallet root
            let wallet_root = WalletRoot {
                root: aggregated_root,
                merkle_proofs,
                balance,
            };

            // Add to history
            self.root_history.push(wallet_root.clone());

            // Increment epoch
            self.current_epoch += 1;

            Ok(Some(wallet_root))
        } else {
            Ok(None)
        }
    }

    pub fn verify_root_transition(
        &self,
        old_root: [u8; 32],
        new_root: [u8; 32],
        proof: &ZkProof,
    ) -> Result<bool, Error> {
        // Create verification context
        let context = RootVerificationContext {
            old_root,
            new_root,
            current_epoch: self.current_epoch,
        };

        // Verify transition proof
        let result = verify_root_transition(proof, &context)?;
        Ok(result)
    }

    pub fn get_latest_root(&self) -> Option<&WalletRoot> {
        self.root_history.last()
    }

    pub fn get_root_at_epoch(&self, epoch: u64) -> Option<&WalletRoot> {
        if epoch <= self.current_epoch {
            self.root_history.get(epoch as usize)
        } else {
            None
        }
    }

    // Helper methods
    fn aggregate_pending_roots(&self) -> Result<[u8; 32], Error> {
        let mut hasher = Sha256::new();

        // Sort pending roots by timestamp
        let mut sorted_roots = self.pending_roots.clone();
        sorted_roots.sort_by_key(|&(_, timestamp)| timestamp);

        // Hash all roots together
        for (root, _) in sorted_roots {
            hasher.update(root);
        }

        let result = hasher.finalize();
        let mut aggregated = [0u8; 32];
        aggregated.copy_from_slice(&result);
        Ok(aggregated)
    }
}
// Helper structs
#[derive(Clone)]
struct VerificationContext {
    channel_id: [u8; 32],
    old_balance: u64,
    new_balance: u64,
    state_root: [u8; 32],
    current_state: PrivateChannelState,
}

#[derive(Clone)]
struct RootVerificationContext {
    old_root: [u8; 32],
    new_root: [u8; 32],
    current_epoch: u64,
}

#[derive(Clone)]
struct ProofInputs {
    channel_id: [u8; 32],
    old_balance: u64,
    new_balance: u64,
    state_root: [u8; 32],
    current_state: PrivateChannelState,
}

// Helper functions
fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn generate_plonk_proof(_inputs: &ProofInputs) -> Result<ZkProof, Error> {
    // Implement proof generation using Plonky2
    // This is a placeholder implementation
    Ok(ZkProof {
        proof_data: vec![],
        merkle_root: vec![],
        public_inputs: vec![],
        timestamp: current_timestamp(),
    })
}

fn verify_plonk_proof(_proof: &ZkProof, _context: &VerificationContext) -> Result<bool, Error> {
    // Implement proof verification using Plonky2
    // This is a placeholder implementation
    Ok(true)
}

fn verify_root_transition(
    _proof: &ZkProof,
    _context: &RootVerificationContext,
) -> Result<bool, Error> {
    // Placeholder implementation
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::types::boc::{Cell, BOC};

    #[derive(Serialize, Deserialize, Default)]
    struct TestPrivateChannelState {
        balance: u64,
    }

    impl From<TestPrivateChannelState> for PrivateChannelState {
        fn from(state: TestPrivateChannelState) -> Self {
            // Implement conversion logic here
            PrivateChannelState {
                balance: state.balance,
                // Populate other fields as necessary
                ..Default::default()
            }
        }
    }

    #[test]
    fn test_balance_tracking() {
        let mut tracker = WalletBalanceTracker::new();
        let channel_id = [1u8; 32];

        // Create test BOC
        let state = TestPrivateChannelState { balance: 100 };
        let state_data = serde_json::to_vec(&state).unwrap();
        let state_cell = Cell::from_data(state_data);
        let mut boc = BOC::new();
        boc.add_cell(state_cell);
        boc.roots = vec![]; // Assuming no roots for this test

        // Track balance update
        let result = tracker.track_balance_update(
            channel_id, 0,   // old balance
            100, // new balance
            boc,
        );

        assert!(result.is_ok());
        assert_eq!(tracker.get_current_balance(&channel_id).unwrap(), 100);
    }

    #[test]
    fn test_root_tracking() {
        let mut tracker: RootStateTracker<[u8; 32]> = RootStateTracker::new(2); // Aggregate after 2 roots

        let root1 = [1u8; 32];
        let root2 = [2u8; 32];

        // Add first root
        let result1 = tracker.track_root_update(root1, vec![], 100);
        assert!(result1.unwrap().is_none()); // No aggregation yet

        // Add second root - should trigger aggregation
        let result2 = tracker.track_root_update(root2, vec![], 200);
        assert!(result2.unwrap().is_some());

        // Check epoch increased
        assert_eq!(tracker.current_epoch, 1);
    }

    #[test]
    fn test_boc_serialization() {
        let mut boc = BOC::new();
        let cell = Cell::with_data(vec![10, 20, 30]);
        boc.add_cell(cell);
        boc.add_root(0);

        let serialized = boc.serialize().unwrap();
        let deserialized = BOC::deserialize(&serialized).unwrap();

        assert_eq!(boc.cells.len(), deserialized.cells.len());
        assert_eq!(boc.roots.len(), deserialized.roots.len());
        assert_eq!(boc.cells[0].data, deserialized.cells[0].data);
    }
}
