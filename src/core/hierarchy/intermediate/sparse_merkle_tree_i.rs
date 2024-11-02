use crate::core::types::ovp_types::{SystemError, BOC};
use crate::core::types::Plonky2System;
use crate::core::zkp::circuit_builder::{Column, PlonkConfig, VirtualCell, ZkCircuitBuilder};
use crate::core::zkp::zkp::ZkProof;
use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
pub struct SparseMerkleTreeI {
    circuit_builder: ZkCircuitBuilder<GoldilocksField>,
    plonky2_system: Plonky2System,
    root_hash: [u8; 32],
    nodes: HashMap<[u8; 32], Node>,
    height: usize,
}

struct Node {
    key: Vec<u8>,
    value: Vec<u8>,
    left: Option<[u8; 32]>,
    right: Option<[u8; 32]>,
}

impl SparseMerkleTreeI {
    pub fn new() -> Self {
        let config = PlonkConfig::standard_recursion_config();
        Self {
            circuit_builder: ZkCircuitBuilder::new(config),
            plonky2_system: Plonky2System::new().unwrap(),
            root_hash: [0u8; 32],
            nodes: HashMap::new(),
            height: 256,
        }
    }

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

    fn add_path_constraints(
        &mut self,
        path: &[([u8; 32], bool)],
        key_cell: Column<GoldilocksField>,
        value_cell: Column<GoldilocksField>,
    ) -> Result<(), SystemError> {
        let mut current = self.circuit_builder.poseidon(&[
            VirtualCell::new(key_cell, 0),
            VirtualCell::new(value_cell, 0),
        ]);

        for (sibling, is_left) in path {
            let sibling_cell = self.circuit_builder.add_virtual_target();
            self.circuit_builder.assert_equal(
                VirtualCell::new(sibling_cell, 0),
                VirtualCell::new(Column::new(self.hash_to_field(sibling)), 0),
            );

            if *is_left {
                current = self
                    .circuit_builder
                    .poseidon(&[current, VirtualCell::new(sibling_cell, 0)]);
            } else {
                current = self
                    .circuit_builder
                    .poseidon(&[VirtualCell::new(sibling_cell, 0), current]);
            }
        }

        let root_cell = self.circuit_builder.add_virtual_public_input();
        self.circuit_builder
            .assert_equal(current, VirtualCell::new(root_cell, 0));

        Ok(())
    }

    pub fn verify(&self, key: &[u8], value: &[u8], proof: &ZkProof) -> Result<bool, SystemError> {
        let circuit = self.circuit_builder.build::<PoseidonGoldilocksConfig>()?;
        self.plonky2_system.verify_proof(proof)
    }

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
        self.plonky2_system.generate_proof(&public_inputs, &witness)
    }

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
            } else {
                if let Some(right) = node.right {
                    path.push((right, false));
                    current = node.left.ok_or(SystemError::InvalidPath)?;
                }
            }
        }

        Ok(path)
    }

    fn hash_leaf(&self, key: &[u8], value: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(value);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

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

    fn get_bit(&self, key: &[u8], index: usize) -> bool {
        let byte_index = index / 8;
        let bit_index = 7 - (index % 8);
        if byte_index < key.len() {
            (key[byte_index] >> bit_index) & 1 == 1
        } else {
            false
        }
    }

    fn hash_to_field(&self, bytes: &[u8; 32]) -> usize {
        let mut result = 0;
        for &byte in bytes.iter().take(8) {
            result = (result << 8) | (byte as usize);
        }
        result
    }

    pub fn root(&self) -> [u8; 32] {
        self.root_hash
    }

    pub fn serialize_state(&self) -> Result<BOC, SystemError> {
        let mut boc = BOC::new();
        boc.add_cell(self.root_hash.to_vec())?;

        for (hash, node) in &self.nodes {
            let mut node_data = Vec::new();
            node_data.extend_from_slice(hash);
            node_data.extend_from_slice(&node.key);
            node_data.extend_from_slice(&node.value);
            boc.add_cell(node_data)?;
        }

        Ok(boc)
    }
}
