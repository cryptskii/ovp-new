// ./src/core/storage_node/verification/storage_and_retreival.rs

// Storage and Retrieval Verification
// This module is the entry point for accessing the storage nodes' storage and retrieval capabilities.
// It provides methods for storing and retrieving data, as well as verifying the proofs provided by the storage nodes.
use crate::core::error::errors::{SystemError, SystemErrorType};
use crate::core::hierarchy::intermediate::sparse_merkle_tree_i::SparseMerkleTreeI;
use crate::core::hierarchy::root::root_contract::RootContract;
use crate::core::hierarchy::root::sparse_merkle_tree_r::SparseMerkleTreeR;

use crate::core::storage_node::storage_node_contract::StorageNode;
use crate::core::types::boc::BOC;
use crate::core::zkps::proof::ZkProof;

use std::sync::Arc;

pub struct StorageAndRetrievalManager<RootTree, IntermediateTreeManager> {
    storage_node: Arc<StorageNode<RootTree, IntermediateTreeManager>>,
    _phantom: std::marker::PhantomData<IntermediateTreeManager>,
}

impl<RootTree, IntermediateTreeManager>
    StorageAndRetrievalManager<RootTree, IntermediateTreeManager>
{
    pub fn new(storage_node: Arc<StorageNode<RootTree, IntermediateTreeManager>>) -> Self {
        Self {
            storage_node,
            _phantom: std::marker::PhantomData,
        }
    }

    pub async fn store_data(&self, boc: BOC, proof: ZkProof) -> Result<(), SystemError> {
        self.storage_node
            .as_ref()
            .store_boc(&boc, &proof)
            .await
            .map_err(|e| SystemError::new(SystemErrorType::InvalidAmount, e.to_string()))?;
        Ok(())
    }

    pub async fn retrieve_data(&self, boc_id: &[u8; 32]) -> Result<BOC, SystemError> {
        self.storage_node.as_ref().retrieve_boc(boc_id).await
    }

    pub async fn retrieve_proof(&self, proof_id: &[u8; 32]) -> Result<ZkProof, SystemError> {
        self.storage_node.as_ref().retrieve_proof(proof_id).await
    }
}
#[derive(Debug, Clone)]
pub struct StorageAndRetrievalMetrics {
    pub last_check: u64,
}

impl StorageAndRetrievalMetrics {
    pub fn new(last_check: u64) -> Self {
        Self { last_check }
    }
}

impl Default for StorageAndRetrievalMetrics {
    fn default() -> Self {
        Self::new(0)
    }
}
