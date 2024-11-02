// ./src_new/core/state/smt/sparse_merkle_tree_wasm.rs

// This module contains the SparseMerkleTreeWasm struct, which represents a Sparse Merkle Tree (SMT) data structure in the context of the WebAssembly (WASM) environment.

use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

const TREE_HEIGHT: usize = 256;
const EMPTY_NODE: [u8; 32] = [0; 32];

/// This struct represents a Sparse Merkle Tree (SMT) data structure in the context of the WebAssembly (WASM) environment.  
pub struct SparseMerkleTreeWasm {
    root_hash: [u8; 32],
    nodes: Vec<[u8; 32]>,
}

pub struct LeafNodeWasm {
    hash: [u8; 32],
    value: Option<Vec<u8>>,
    left: Option<Box<LeafNodeWasm>>,
    right: Option<Box<LeafNodeWasm>>,
}
impl SparseMerkleTreeWasm {
    pub fn new() -> Self {
        let mut default_nodes = [[0; 32]; TREE_HEIGHT + 1];
        // Generate default nodes for each level
        for i in (0..TREE_HEIGHT).rev() {
            default_nodes[i] = hash_pair(&default_nodes[i + 1], &default_nodes[i + 1]);
        }

        Self {
            root_hash: default_nodes[0],
            nodes: Vec::new(),
        }
    }

    pub fn update(&mut self, key: &[u8], value: &[u8]) -> Result<(), JsValue> {
        let key_hash = hash_key(key);
        let value_hash = hash_value(value);

        // Create leaf node
        let leaf = LeafNodeWasm {
            hash: value_hash,
            value: Some(value.to_vec()),
            left: None,
            right: None,
        };

        self.nodes.insert(value_hash, leaf);

        // Update path to root
        self.update_path(&key_hash, value_hash, 0)?;

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, JsValue> {
        let key_hash = hash_key(key);

        let mut current_hash = self.root;
        let mut level = 0;

        while level < TREE_HEIGHT {
            // Get current node
            let node = match self.nodes.get(&current_hash) {
                Some(n) => n,
                None => return Ok(None),
            };

            // Return value if leaf node
            if let Some(value) = &node.value {
                return Ok(Some(value.clone()));
            }

            // Go left or right based on key bit
            let bit = get_bit(&key_hash, level);
            current_hash = if bit {
                node.right.unwrap_or(self.default_nodes[level + 1])
            } else {
                node.left.unwrap_or(self.default_nodes[level + 1])
            };

            level += 1;
        }

        Ok(None)
    }

    pub fn verify(&self, key: &[u8], value: &[u8], proof: &[u8]) -> Result<bool, JsValue> {
        let key_hash = hash_key(key);
        let value_hash = hash_value(value);

        // Parse proof
        let proof_nodes = parse_proof(proof)?;
        let mut computed_root = value_hash;

        // Verify proof path
        for (i, sibling) in proof_nodes.iter().enumerate() {
            let bit = get_bit(&key_hash, i);
            computed_root = if bit {
                hash_pair(sibling, &computed_root)
            } else {
                hash_pair(&computed_root, sibling)
            };
        }

        Ok(computed_root == self.root)
    }

    pub fn get_proof(&self, key: &[u8]) -> Result<Vec<u8>, JsValue> {
        let key_hash = hash_key(key);
        let mut proof = Vec::with_capacity(TREE_HEIGHT);
        let mut current_hash = self.root;
        let mut level = 0;

        while level < TREE_HEIGHT {
            let node = match self.nodes.get(&current_hash) {
                Some(n) => n,
                None => break,
            };

            let bit = get_bit(&key_hash, level);
            if bit {
                if let Some(left) = node.left {
                    proof.push(left);
                }
                current_hash = node.right.unwrap_or(self.default_nodes[level + 1]);
            } else {
                if let Some(right) = node.right {
                    proof.push(right);
                }
                current_hash = node.left.unwrap_or(self.default_nodes[level + 1]);
            }

            level += 1;
        }

        Ok(serialize_proof(&proof))
    }

    pub fn get_root(&self) -> [u8; 32] {
        self.root
    }

    fn update_path(
        &mut self,
        key: &[u8; 32],
        value_hash: [u8; 32],
        level: usize,
    ) -> Result<[u8; 32], JsValue> {
        if level == TREE_HEIGHT {
            return Ok(value_hash);
        }

        let bit = get_bit(key, level);
        let default_hash = self.default_nodes[level + 1];

        let (left, right) = if bit {
            let left = self
                .nodes
                .get(&self.root)
                .and_then(|n| n.left)
                .unwrap_or(default_hash);
            let right = self.update_path(key, value_hash, level + 1)?;
            (left, right)
        } else {
            let left = self.update_path(key, value_hash, level + 1)?;
            let right = self
                .nodes
                .get(&self.root)
                .and_then(|n| n.right)
                .unwrap_or(default_hash);
            (left, right)
        };

        let node_hash = hash_pair(&left, &right);
        self.nodes.insert(
            node_hash,
            LeafNodeWasm {
                hash: node_hash,
                value: None,
                left: Some(left),
                right: Some(right),
            },
        );

        if level == 0 {
            self.root = node_hash;
        }

        Ok(node_hash)
    }
}

#[derive(Clone, Debug)]
pub struct MerkleProofClient {
    siblings: Vec<[u8; 32]>,
    path: Vec<bool>,
    leaf_hash: [u8; 32],
    root_hash: [u8; 32],
}

impl MerkleProofClient {
    pub fn new(
        siblings: Vec<[u8; 32]>,
        path: Vec<bool>,
        leaf_hash: [u8; 32],
        root_hash: [u8; 32],
    ) -> Self {
        Self {
            siblings,
            path,
            leaf_hash,
            root_hash,
        }
    }

    pub fn verify(&self) -> bool {
        let mut current_hash = self.leaf_hash;

        for (sibling, is_right) in self.siblings.iter().zip(self.path.iter()) {
            current_hash = if *is_right {
                hash_pair(sibling, &current_hash)
            } else {
                hash_pair(&current_hash, sibling)
            };
        }

        current_hash == self.root_hash
    }
}
fn serialize_proof(proof: &[[u8; 32]]) -> Vec<u8> {
    proof.iter().flat_map(|hash| hash.to_vec()).collect()
}
fn deserialize_proof(proof: &[u8]) -> Result<Vec<[u8; 32]>, JsValue> {
    let mut proof_data = proof
        .chunks(32)
        .map(|chunk| {
            let mut node = [0u8; 32];
            node.copy_from_slice(chunk);
            Ok(node)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(proof_data)
}
fn parse_proof(proof: &[u8]) -> Result<Vec<[u8; 32]>, JsValue> {
    let mut proof_data = proof
        .chunks(32)
        .map(|chunk| {
            let mut node = [0u8; 32];
            node.copy_from_slice(chunk);
            Ok(node)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(proof_data)
}
fn get_bit(bytes: &[u8; 32], index: usize) -> bool {
    let byte_index = index / 8;
    let bit_index = 7 - (index % 8);
    (bytes[byte_index] >> bit_index) & 1 == 1
}

fn hash_key(key: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.finalize().into()
}

fn hash_value(value: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hasher.finalize().into()
}

fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_and_get() {
        let mut tree = SparseMerkleTreeWasm::new();

        let key = b"test_key";
        let value = b"test_value";

        tree.update(key, value).unwrap();
        let result = tree.get(key).unwrap().unwrap();

        assert_eq!(result, value);
    }

    #[test]
    fn test_proof_verification() {
        let mut tree = SparseMerkleTreeWasm::new();

        let key = b"test_key";
        let value = b"test_value";

        tree.update(key, value).unwrap();
        let proof = tree.get_proof(key).unwrap();

        assert!(tree.verify(key, value, &proof).unwrap());
    }

    #[test]
    fn test_multiple_updates() {
        let mut tree = SparseMerkleTreeWasm::new();

        for i in 0..10 {
            let key = format!("key{}", i).into_bytes();
            let value = format!("value{}", i).into_bytes();
            tree.update(&key, &value).unwrap();

            let result = tree.get(&key).unwrap().unwrap();
            assert_eq!(result, value);
        }
    }

    #[test]
    fn test_nonexistent_key() {
        let tree = SparseMerkleTreeWasm::new();
        let result = tree.get(b"nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_existing_key() {
        let mut tree = SparseMerkleTreeWasm::new();
        let key = b"test_key";

        tree.update(key, b"value1").unwrap();
        tree.update(key, b"value2").unwrap();

        let result = tree.get(key).unwrap().unwrap();
        assert_eq!(result, b"value2");
    }
}
