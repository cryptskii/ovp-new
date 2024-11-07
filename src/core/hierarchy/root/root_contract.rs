use crate::core::error::errors::{SystemError, SystemErrorType};
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::Transaction;
use crate::core::hierarchy::root::sparse_merkle_tree_r::SparseMerkleTreeR;
use crate::core::types::boc::{Cell, CellType, BOC};
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::hash::merkle_proofs::MerkleProof;
use plonky2::hash::poseidon::PoseidonHash;
use std::collections::HashMap;

pub struct RootContract {
    global_tree: SparseMerkleTreeR,
    intermediate_roots: HashMap<Address, Hash>,
    epoch: u64,
    epoch_duration: u64,
    last_submission: u64,
    verify_settlement_state: bool,
    verify_intermediate_state: bool,
    verify_channel_state: bool,
    verify_transaction_state: bool,
    verify_storage_state: bool,
    verify_global_state: bool,
    verify_root_state: bool,
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

    pub fn process_intermediate_root(
        &mut self,
        contract_addr: Address,
        root: Hash,
        proof: MerkleProof<GoldilocksField, PoseidonHash>,
    ) -> Result<(), SystemError> {
        if !proof.verify(&root) {
            return Err(SystemError {
                error_type: SystemErrorType::NotFound,
                message: "Invalid proof".to_string(),
            });
        }

        self.intermediate_roots.insert(contract_addr, root);
        self.global_tree.update_global_tree(&contract_addr, &root)?;

        Ok(())
    }

    pub fn try_submit_global_root(
        &mut self,
        now: u64,
    ) -> Option<(Hash, MerkleProof<GoldilocksField, PoseidonHash>)> {
        if now - self.last_submission < self.epoch_duration {
            return None;
        }

        let root = self.global_tree.get_global_root_hash();
        let proof = MerkleProof::<GoldilocksField, PoseidonHash>::new(vec![], vec![]);

        self.epoch += 1;
        self.last_submission = now;
        self.verify_settlement_state = true;
        self.submit_settlement = true;

        if !self.verify_global_state {
            return None;
        }
        Some((root, proof))
    }

    pub fn verify_transaction(
        &self,
        tx: Transaction,
        proof: MerkleProof<GoldilocksField, PoseidonHash>,
    ) -> Result<bool, SystemError> {
        let intermediate_root = self
            .intermediate_roots
            .get(&tx.recipient)
            .ok_or(SystemError {
                error_type: SystemErrorType::NotFound,
                message: "Unknown contract".to_string(),
            })?;

        if !proof.verify_against_root(tx.id.to_le_bytes(), intermediate_root) {
            return Ok(false);
        }

        Ok(true)
    }

    pub fn serialize(&self) -> Result<BOC, SystemError> {
        let mut boc = BOC::new();

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
            [0u8; 32],
            None,
        ));

        let tree_boc = self.global_tree.serialize_global_state()?;
        boc.add_cell(Cell::new(
            tree_boc.get_data(),
            vec![],
            CellType::Ordinary,
            [0u8; 32],
            None,
        ));

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

    pub fn deserialize(boc: BOC) -> Result<Self, SystemError> {
        let root_cell = boc.get_root_cell().ok_or(SystemError {
            error_type: SystemErrorType::NotFound,
            message: "Empty BOC".to_string(),
        })?;

        let state_data = root_cell.get_data();

        if state_data.len() < 25 {
            return Err(SystemError {
                error_type: SystemErrorType::NotFound,
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

        let global_tree = SparseMerkleTreeR::new();

        let mut intermediate_roots = HashMap::new();

        let root_refs = root_cell.get_references();
        for ref_cell in root_refs {
            let data = ref_cell.get_data();
            if data.len() == 64 {
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

type Hash = [u8; 32];
type Address = [u8; 32];
