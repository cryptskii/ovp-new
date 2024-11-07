use crate::core::error::errors::{SystemError, SystemErrorType};
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::Transaction;
use crate::core::hierarchy::root::sparse_merkle_tree_r::SparseMerkleTreeR;
use crate::core::types::boc::{Cell, CellType, BOC};
use plonky2::hash::merkle_proofs::MerkleProof;
use std::collections::HashMap;

pub struct RootContract {
    // Global sparse merkle tree storing intermediate contract roots
    global_tree: SparseMerkleTreeR,
    // Map of intermediate contract addresses to their latest roots
    intermediate_roots: HashMap<Address, Hash>,
    // Current epoch number
    epoch: u64,
    // Minimum time between global root submissions
    epoch_duration: u64,
    // Last submission timestamp
    last_submission: u64,
    // Whether to verify settlement state
    verify_settlement_state: bool,
    // Whether to verify intermediate contract state
    verify_intermediate_state: bool,
    // Whether to verify channel state
    verify_channel_state: bool,
    // Whether to verify transaction state
    verify_transaction_state: bool,
    // Whether to verify storage state
    verify_storage_state: bool,
    // Whether to verify global state
    verify_global_state: bool,
    // Whether to verify root state
    verify_root_state: bool,
    // Whether to submit settlement state
    submit_settlement: bool,
}

impl RootContract {
    pub fn new(epoch_duration: u64) -> Self {
        Self {
            global_tree: SparseMerkleTreeR::new(),
            intermediate_roots: HashMap::new(),
            epoch: 0,
            epoch_duration,
            last_submission: 0,
            verify_settlement_state: true,
            verify_intermediate_state: false,
            verify_channel_state: false,
            verify_transaction_state: false,
            verify_storage_state: false,
            verify_global_state: false,
            verify_root_state: false,
            submit_settlement: true,
        }
    }

    // Process a new intermediate contract root submission
    pub fn process_intermediate_root(
        &mut self,
        contract_addr: Address,
        root: Hash,
        proof: MerkleProof,
    ) -> Result<(), SystemError> {
        // Verify the proof matches the submitted root
        if !proof.verify(&root) {
            return Err(SystemError {
                error_type: SystemErrorType::InvalidProof,
                message: "Invalid proof".to_string(),
            });
        }

        // Update the intermediate contract root
        self.intermediate_roots.insert(contract_addr, root);

        // Update global tree
        self.global_tree.update_global_tree(&contract_addr, &root)?;

        Ok(())
    }

    // Submit global root on-chain if epoch has elapsed
    pub fn try_submit_global_root(&mut self, now: u64) -> Option<(Hash, MerkleProof)> {
        if now - self.last_submission < self.epoch_duration {
            return None;
        }

        let root = self.global_tree.get_global_root_hash();

        // For now, creating a placeholder proof since SparseMerkleTreeR doesn't expose proof generation
        let proof = MerkleProof::default(); // You'll need to implement this

        self.epoch += 1;
        self.last_submission = now;
        self.verify_settlement_state = true;
        self.submit_settlement = true;

        // Verify the global root
        if !self.verify_global_state {
            return None;
        }
        Some((root, proof))
    }

    // Verify a transaction against the global state
    pub fn verify_transaction(
        &self,
        tx: Transaction,
        proof: MerkleProof,
    ) -> Result<bool, SystemError> {
        // Get intermediate contract root
        let intermediate_root =
            self.intermediate_roots
                .get(&tx.contract_addr)
                .ok_or(SystemError {
                    error_type: SystemErrorType::NotFound,
                    message: "Unknown contract".to_string(),
                })?;

        // Verify transaction proof against intermediate root
        if !proof.verify_against_root(tx.hash(), intermediate_root) {
            return Ok(false);
        }

        // Generate a merkle path for verification
        let path = self
            .global_tree
            .generate_global_merkle_path(&tx.contract_addr)?;

        // Verify by recalculating root with the path
        let calculated_root = self
            .global_tree
            .calculate_new_global_root(&path, intermediate_root)?;

        Ok(calculated_root == self.global_tree.get_global_root_hash())
    }

    // Serialize contract state to BOC (Bag of Cells)
    pub fn serialize(&self) -> Result<BOC, SystemError> {
        let mut boc = BOC::new();

        // Create a cell for contract state
        let mut state_data = Vec::new();
        state_data.extend_from_slice(&self.epoch.to_le_bytes());
        state_data.extend_from_slice(&self.epoch_duration.to_le_bytes());
        state_data.extend_from_slice(&self.last_submission.to_le_bytes());
        state_data.push(self.verify_settlement_state as u8);
        state_data.push(self.submit_settlement as u8);

        boc.add_cell(Cell::new(
            state_data,
            vec![],
            CellType::Ordinary,
            [0u8; 32], // Hash will be calculated by BOC
            None,
        ));

        // Add global tree state
        let tree_boc = self.global_tree.serialize_global_state()?;
        for cell in tree_boc.get_cells() {
            boc.add_cell(cell.clone());
        }

        // Add intermediate roots
        for (addr, root) in &self.intermediate_roots {
            let mut root_data = Vec::new();
            root_data.extend_from_slice(addr);
            root_data.extend_from_slice(root);
            boc.add_cell(Cell::new(
                root_data,
                vec![],
                CellType::Ordinary,
                [0u8; 32],
                None,
            ));
        }

        Ok(boc)
    }

    // Deserialize contract state from BOC
    pub fn deserialize(boc: BOC) -> Result<Self, SystemError> {
        let cells = boc.get_cells();
        if cells.is_empty() {
            return Err(SystemError {
                error_type: SystemErrorType::DeserializationError,
                message: "Empty BOC".to_string(),
            });
        }

        // First cell contains contract state
        let state_cell = &cells[0];
        let state_data = state_cell.get_data();

        if state_data.len() < 25 {
            // 8 + 8 + 8 + 1 + 1 bytes minimum
            return Err(SystemError {
                error_type: SystemErrorType::DeserializationError,
                message: "Invalid state data length".to_string(),
            });
        }

        let mut epoch_bytes = [0u8; 8];
        epoch_bytes.copy_from_slice(&state_data[0..8]);
        let epoch = u64::from_le_bytes(epoch_bytes);

        let mut epoch_duration_bytes = [0u8; 8];
        epoch_duration_bytes.copy_from_slice(&state_data[8..16]);
        let epoch_duration = u64::from_le_bytes(epoch_duration_bytes);

        let mut last_submission_bytes = [0u8; 8];
        last_submission_bytes.copy_from_slice(&state_data[16..24]);
        let last_submission = u64::from_le_bytes(last_submission_bytes);

        let verify_settlement_state = state_data[24] != 0;
        let submit_settlement = state_data[25] != 0;

        // Create a new BOC for the global tree (cells[1..])
        let global_tree = SparseMerkleTreeR::new(); // You'll need to implement proper deserialization

        // Extract intermediate roots from remaining cells
        let mut intermediate_roots = HashMap::new();
        for cell in cells.iter().skip(1) {
            let data = cell.get_data();
            if data.len() == 64 {
                // addr(32) + root(32)
                let mut addr = [0u8; 32];
                let mut root = [0u8; 32];
                addr.copy_from_slice(&data[0..32]);
                root.copy_from_slice(&data[32..64]);
                intermediate_roots.insert(addr, root);
            }
        }

        Ok(Self {
            global_tree,
            intermediate_roots,
            epoch,
            epoch_duration,
            last_submission,
            verify_settlement_state,
            verify_intermediate_state: false,
            verify_channel_state: false,
            verify_transaction_state: false,
            verify_storage_state: false,
            verify_global_state: false,
            verify_root_state: false,
            submit_settlement,
        })
    }
}

// Helper types
type Hash = [u8; 32];
type Address = [u8; 32];
