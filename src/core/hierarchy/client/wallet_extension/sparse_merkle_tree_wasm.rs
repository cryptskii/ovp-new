use sha2::{Digest, Sha256};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

const TREE_HEIGHT: usize = 256;

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct SparseMerkleTreeWasm {
    root_hash: Vec<u8>,
    nodes: HashMap<Vec<u8>, LeafNodeWasm>,
    default_nodes: Vec<Vec<u8>>,
}

#[derive(Clone, Debug)]
struct LeafNodeWasm {
    hash: Vec<u8>,
    value: Option<Vec<u8>>,
    left: Option<Box<LeafNodeWasm>>,
    right: Option<Box<LeafNodeWasm>>,
}

impl Default for SparseMerkleTreeWasm {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
impl SparseMerkleTreeWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let mut default_nodes = vec![vec![0; 32]; TREE_HEIGHT + 1];
        // Generate default nodes for each level
        for i in (0..TREE_HEIGHT).rev() {
            default_nodes[i] = hash_pair(&default_nodes[i + 1], &default_nodes[i + 1]);
        }

        Self {
            root_hash: default_nodes[0].clone(),
            nodes: HashMap::new(),
            default_nodes,
        }
    }

    pub fn update(&mut self, key: &[u8], value: &[u8]) -> Result<(), JsValue> {
        let key_hash = hash_key(key);
        let value_hash = hash_value(value);

        // Create leaf node
        let leaf = LeafNodeWasm {
            hash: value_hash.clone(),
            value: Some(value.to_vec()),
            left: None,
            right: None,
        };

        self.nodes.insert(value_hash.clone(), leaf);

        // Update path to root
        self.update_path(&key_hash, &value_hash, 0)?;

        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, JsValue> {
        let key_hash = hash_key(key);
        let mut current_hash = self.root_hash.clone();

        for level in 0..TREE_HEIGHT {
            match self.nodes.get(&current_hash) {
                Some(node) => {
                    // Return value if leaf node
                    if let Some(value) = &node.value {
                        return Ok(Some(value.clone()));
                    }

                    // Go left or right based on key bit
                    let bit = get_bit(&key_hash, level);
                    current_hash = if bit {
                        match &node.right {
                            Some(right) => right.hash.clone(),
                            None => self.default_nodes[level + 1].clone(),
                        }
                    } else {
                        match &node.left {
                            Some(left) => left.hash.clone(),
                            None => self.default_nodes[level + 1].clone(),
                        }
                    };
                }
                None => return Ok(None),
            }
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

        Ok(computed_root == self.root_hash)
    }

    pub fn get_proof(&self, key: &[u8]) -> Result<Vec<u8>, JsValue> {
        let key_hash = hash_key(key);
        let mut proof = Vec::with_capacity(TREE_HEIGHT * 32);
        let mut current_hash = self.root_hash.clone();

        for level in 0..TREE_HEIGHT {
            match self.nodes.get(&current_hash) {
                Some(node) => {
                    let bit = get_bit(&key_hash, level);
                    if bit {
                        if let Some(left) = &node.left {
                            proof.extend_from_slice(&left.hash);
                        }
                        current_hash = match &node.right {
                            Some(right) => right.hash.clone(),
                            None => self.default_nodes[level + 1].clone(),
                        };
                    } else {
                        if let Some(right) = &node.right {
                            proof.extend_from_slice(&right.hash);
                        }
                        current_hash = match &node.left {
                            Some(left) => left.hash.clone(),
                            None => self.default_nodes[level + 1].clone(),
                        };
                    }
                }
                None => break,
            }
        }

        Ok(proof)
    }

    #[wasm_bindgen(js_name = getRoot)]
    pub fn root(&self) -> Vec<u8> {
        self.root_hash.clone()
    }

    fn update_path(
        &mut self,
        key: &Vec<u8>,
        value_hash: &Vec<u8>,
        level: usize,
    ) -> Result<Vec<u8>, JsValue> {
        if level == TREE_HEIGHT {
            return Ok(value_hash.clone());
        }

        let bit = get_bit(key, level);
        let default_hash = self.default_nodes[level + 1].clone();

        let (left_hash, right_hash) = if bit {
            let left = match self.nodes.get(&self.root_hash) {
                Some(node) => match &node.left {
                    Some(left) => left.hash.clone(),
                    None => default_hash,
                },
                None => default_hash,
            };
            let right = self.update_path(key, value_hash, level + 1)?;
            (left, right)
        } else {
            let left = self.update_path(key, value_hash, level + 1)?;
            let right = match self.nodes.get(&self.root_hash) {
                Some(node) => match &node.right {
                    Some(right) => right.hash.clone(),
                    None => default_hash,
                },
                None => default_hash,
            };
            (left, right)
        };

        let node_hash = hash_pair(&left_hash, &right_hash);

        let new_node = LeafNodeWasm {
            hash: node_hash.clone(),
            value: None,
            left: Some(Box::new(LeafNodeWasm {
                hash: left_hash,
                value: None,
                left: None,
                right: None,
            })),
            right: Some(Box::new(LeafNodeWasm {
                hash: right_hash,
                value: None,
                left: None,
                right: None,
            })),
        };

        self.nodes.insert(node_hash.clone(), new_node);

        if level == 0 {
            self.root_hash = node_hash.clone();
        }

        Ok(node_hash)
    }
}

fn get_bit(bytes: &Vec<u8>, index: usize) -> bool {
    let byte_index = index / 8;
    if byte_index >= bytes.len() {
        return false;
    }
    let bit_index = 7 - (index % 8);
    (bytes[byte_index] >> bit_index) & 1 == 1
}

fn hash_key(key: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(key);
    hasher.finalize().to_vec()
}

pub(crate) fn hash_value(value: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hasher.finalize().to_vec()
}

fn hash_pair(left: &Vec<u8>, right: &Vec<u8>) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().to_vec()
}

fn parse_proof(proof: &[u8]) -> Result<Vec<Vec<u8>>, JsValue> {
    if proof.len() % 32 != 0 {
        return Err(JsValue::from_str("Invalid proof length"));
    }

    Ok(proof.chunks(32).map(|chunk| chunk.to_vec()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_tree() {
        let tree = SparseMerkleTreeWasm::new();
        assert_eq!(tree.root_hash.len(), 32);
    }

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
    fn test_hash_consistency() {
        let left = vec![1u8; 32];
        let right = vec![2u8; 32];
        let hash1 = hash_pair(&left, &right);
        let hash2 = hash_pair(&left, &right);
        assert_eq!(hash1, hash2);
    }
}
