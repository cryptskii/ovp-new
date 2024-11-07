use crate::core::error::errors::SystemError;
use crate::core::storage_node::storage_node_contract::StorageNode;
use crate::core::types::boc::{Cell, BOC};
use crate::core::zkps::circuit_builder::{Column, VirtualCell, ZkCircuitBuilder};
use crate::core::zkps::plonky2::Plonky2System;
use crate::core::zkps::proof::ZkProof;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::circuit_data::CircuitConfig;
use sha2::{Digest, Sha256};
use std::collections::HashMap;

/// Intermediate Tree Trait
pub trait IntermediateTreeManagerTrait {
    fn update_trees(
        &mut self,
        boc: &BOC,
        intermediate_trees: &mut HashMap<[u8; 32], SparseMerkleTreeI<GoldilocksField, 2>>,
        root_trees: &mut HashMap<[u8; 32], SparseMerkleTreeI<GoldilocksField, 2>>,
    ) -> Result<(), SystemError>;
}

/// Sparse Merkle Tree Implementation
pub struct SparseMerkleTreeI<RootTree, IntermediateTree> {
    circuit_builder: ZkCircuitBuilder<GoldilocksField, 2>,
    plonky2_system: Plonky2System,
    root_hash: [u8; 32],
    nodes: HashMap<[u8; 32], StorageNode<RootTree, IntermediateTree>>,
    height: usize,
}

impl<RootTree, IntermediateTree> SparseMerkleTreeI<RootTree, IntermediateTree> {
    /// Create a new Sparse Merkle Tree
    pub fn new() -> Self {
        let config = CircuitConfig::standard_recursion_config();
        Self {
            circuit_builder: ZkCircuitBuilder::new(config),
            plonky2_system: Plonky2System::default(), // Assuming `Default` trait implemented
            root_hash: [0u8; 32],
            nodes: HashMap::new(),
            height: 256,
        }
    }

    /// Update a leaf in the Merkle tree
    pub fn update(&mut self, key: &[u8], value: &[u8]) -> Result<(), SystemError> {
        let leaf_hash = self.hash_leaf(key, value);
        let path = self.generate_merkle_path(key)?;
        let value_cell = self.circuit_builder.add_virtual_target();
        let key_cell = self.circuit_builder.add_virtual_target();
        self.add_path_constraints(&path, key_cell, value_cell)?;
        self.root_hash = self.calculate_new_root(&path, &leaf_hash)?;
        self.generate_update_proof(key, value, &path)?;
        Ok(())
    }

    /// Add constraints to the path in the zk-SNARK circuit
    fn add_path_constraints(
        &mut self,
        path: &[([u8; 32], bool)],
        key_cell: VirtualCell,
        value_cell: VirtualCell,
    ) -> Result<(), SystemError> {
        let mut current = self.circuit_builder.poseidon(&[key_cell, value_cell]);

        for (sibling, is_left) in path {
            let sibling_cell = self.circuit_builder.add_virtual_target();
            self.circuit_builder.assert_equal(
                sibling_cell,
                self.circuit_builder
                    .constant_target(self.hash_to_field(sibling)),
            );

            let cells = if *is_left {
                [current, sibling_cell]
            } else {
                [sibling_cell, current]
            };
            current = self.circuit_builder.poseidon(&cells);
        }

        let root_cell = self.circuit_builder.add_public_input();
        self.circuit_builder.assert_equal(current, root_cell);

        Ok(())
    }

    /// Verify a proof for a given key-value pair
    pub fn verify(&self, key: &[u8], value: &[u8], proof: &ZkProof) -> Result<bool, SystemError> {
        let circuit = self
            .circuit_builder
            .build()
            .map_err(|_| SystemError::InvalidProof)?;
        self.plonky2_system
            .verify_proof(&circuit, proof)
            .map_err(|_| SystemError::InvalidProof)
    }

    /// Generate proof for an update
    fn generate_update_proof(
        &self,
        key: &[u8],
        value: &[u8],
        path: &[([u8; 32], bool)],
    ) -> Result<ZkProof, SystemError> {
        let mut public_inputs = Vec::new();
        public_inputs.extend_from_slice(key);
        public_inputs.extend_from_slice(value);

        for (hash, _) in path {
            public_inputs.extend_from_slice(hash);
        }

        let witness = self.generate_witness(key, value, path)?;
        let proof = self
            .plonky2_system
            .generate_proof(&self.circuit_builder, &public_inputs, &witness)
            .map_err(|_| SystemError::InvalidProof)?;

        Ok(proof)
    }

    /// Generate Merkle path for a given key
    fn generate_merkle_path(&self, key: &[u8]) -> Result<Vec<([u8; 32], bool)>, SystemError> {
        let mut path = Vec::new();
        let mut current = self.root_hash;

        for i in 0..self.height {
            let bit = self.get_bit(key, i);
            let node = self.nodes.get(&current).ok_or(SystemError::NodeNotFound)?;

            if bit {
                if let Some(left) = node.left {
                    path.push((left, true));
                    current = node.right.ok_or(SystemError::InvalidPath)?;
                }
            } else if let Some(right) = node.right {
                path.push((right, false));
                current = node.left.ok_or(SystemError::InvalidPath)?;
            }
        }

        Ok(path)
    }

    /// Hash a leaf node
    fn hash_leaf(&self, key: &[u8], value: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(value);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Calculate the new root hash after updating a leaf
    fn calculate_new_root(
        &self,
        path: &[([u8; 32], bool)],
        leaf_hash: &[u8; 32],
    ) -> Result<[u8; 32], SystemError> {
        let mut current = *leaf_hash;

        for (sibling, is_left) in path.iter().rev() {
            let mut hasher = Sha256::new();
            if *is_left {
                hasher.update(&current);
                hasher.update(sibling);
            } else {
                hasher.update(sibling);
                hasher.update(&current);
            }
            current = hasher.finalize().into();
        }

        Ok(current)
    }

    /// Extract a bit from the key at a specific index
    fn get_bit(&self, key: &[u8], index: usize) -> bool {
        let byte_index = index / 8;
        let bit_index = 7 - (index % 8);
        if byte_index < key.len() {
            (key[byte_index] >> bit_index) & 1 == 1
        } else {
            false
        }
    }

    /// Convert hash to a field element
    fn hash_to_field(&self, bytes: &[u8; 32]) -> usize {
        bytes
            .iter()
            .take(8)
            .fold(0, |acc, &byte| (acc << 8) | (byte as usize))
    }

    /// Return the current root hash of the tree
    pub fn root(&self) -> [u8; 32] {
        self.root_hash
    }

    /// Serialize the tree state to a BOC format
    pub fn serialize_state(&self) -> Result<BOC, SystemError> {
        let mut boc = BOC::new();
        boc.add_cell(Cell::new(self.root_hash.to_vec(), None)?);

        for (hash, node) in &self.nodes {
            let mut node_data = Vec::new();
            node_data.extend_from_slice(hash);
            node_data.extend_from_slice(&node.key);
            node_data.extend_from_slice(&node.value);
            boc.add_cell(Cell::new(node_data, None)?)?;
        }

        Ok(boc)
    }
}
