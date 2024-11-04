use crate::core::hierarchy::root::sparse_merkle_tree_r::SparseMerkleTreeR;
use crate::core::state::boc::builder::Builder;
use crate::core::state::boc::cell::Cell;
use crate::core::types::MerkleProof;
use crate::core::types::{BuilderData, SliceData, Transaction};
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
}

impl RootContract {
    pub fn new(epoch_duration: u64) -> Self {
        Self {
            global_tree: SparseMerkleTreeR::new(),
            intermediate_roots: HashMap::new(),
            epoch: 0,
            epoch_duration,
            last_submission: 0,
        }
    }

    // Process a new intermediate contract root submission
    pub fn process_intermediate_root(
        &mut self,
        contract_addr: Address,
        root: Hash,
        proof: MerkleProof,
    ) -> Result<(), Error> {
        // Verify the proof matches the submitted root
        if !proof.verify(&root) {
            return Err(Error::InvalidProof);
        }

        // Update the intermediate contract root
        self.intermediate_roots.insert(contract_addr, root);

        // Update global tree
        self.global_tree.update_leaf(contract_addr.into(), root);

        Ok(())
    }

    // Submit global root on-chain if epoch has elapsed
    pub fn try_submit_global_root(&mut self, now: u64) -> Option<(Hash, MerkleProof)> {
        if now - self.last_submission < self.epoch_duration {
            return None;
        }

        let root = self.global_tree.root();
        let proof = self.global_tree.generate_proof();

        self.epoch += 1;
        self.last_submission = now;

        Some((root, proof))
    }

    // Verify a transaction against the global state
    pub fn verify_transaction(&self, tx: Transaction, proof: MerkleProof) -> Result<bool, Error> {
        // Get intermediate contract root
        let intermediate_root = self
            .intermediate_roots
            .get(&tx.contract_addr)
            .ok_or(Error::UnknownContract)?;

        // Verify transaction proof against intermediate root
        if !proof.verify_against_root(tx.hash(), intermediate_root) {
            return Ok(false);
        }

        // Verify intermediate root exists in global tree
        if !self
            .global_tree
            .verify_leaf(tx.contract_addr.into(), intermediate_root)
        {
            return Ok(false);
        }

        Ok(true)
    }

    // Serialize contract state to BOC (Bag of Cells)
    pub fn serialize(&self) -> Result<Cell, Error> {
        let mut builder = Builder::new();

        builder.append_u64(self.epoch)?;
        builder.append_u64(self.epoch_duration)?;
        builder.append_u64(self.last_submission)?;

        // Serialize global tree
        let tree_cell = self.global_tree.serialize()?;
        builder.append_reference(tree_cell)?;

        // Serialize intermediate roots map
        let mut roots_builder = BuilderData::new();
        for (addr, root) in &self.intermediate_roots {
            roots_builder.append_raw(addr.serialize()?, 256)?;
            roots_builder.append_raw(root.as_slice(), 256)?;
        }
        builder.append_reference(roots_builder.into_cell()?)?;

        Ok(builder.into_cell()?)
    }

    // Deserialize contract state from BOC
    pub fn deserialize(cell: Cell) -> Result<Self, Error> {
        let slice = SliceData::load_cell(cell)?;

        let epoch = slice.get_u64()?;
        let epoch_duration = slice.get_u64()?;
        let last_submission = slice.get_u64()?;

        // Deserialize global tree
        let tree_cell = slice.reference(0)?;
        let global_tree = SparseMerkleTreeR::deserialize(tree_cell)?;

        // Deserialize intermediate roots
        let roots_slice = slice.reference(1)?;
        let mut intermediate_roots = HashMap::new();

        while !roots_slice.is_empty() {
            let addr = Address::deserialize(roots_slice.get_next(256)?)?;
            let root = Hash::from_slice(roots_slice.get_next(256)?);
            intermediate_roots.insert(addr, root);
        }

        Ok(Self {
            global_tree,
            intermediate_roots,
            epoch,
            epoch_duration,
            last_submission,
        })
    }
}

// Error types
#[derive(Debug)]
pub enum Error {
    InvalidProof,
    UnknownContract,
    SerializationError(String),
    DeserializationError(String),
}

// Helper types
type Hash = [u8; 32];
type Address = [u8; 32];