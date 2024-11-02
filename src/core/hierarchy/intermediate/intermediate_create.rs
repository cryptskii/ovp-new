// ./src/core/hierarchy/intermediate/intermediate_create.rs

// Intermediate Create
// This module is responsible for creating the intermediate contracts and managing the state of the root contract.
// It handles the creation of the intermediate contracts, the management of the root contract state, and the
// communication between the root contract and the intermediate contracts. It also handles the submission of the proofs to the intermediate contracts.

use crate::core::hierarchy::intermediate::IntermediateContract;
use crate::core::hierarchy::root_contract::RootContract;
use crate::core::state::boc::cell_serialization::BOC;
use crate::core::storage_node::storage_node::StorageNode;
use crate::core::types::RootSubmission;
use std::collections::HashMap;
use std::time::SystemTime;

use crate::core::hierarchy::state_tracking_i::ProofInputsI;

#[cfg(feature = "native")]
use async_std::task;

use super::SparseMerkleTreeI;

pub struct IntermediateCreate {
    root_contract: RootContract,
    wallet_states: HashMap<String, Vec<u8>>,
    last_root_submission: Option<(u64, Vec<u8>)>,
    proof_system: ProofInputsI,
}

impl IntermediateContract {
    fn verify_storage_state(
        storage_nodes: &Vec<StorageNode>,
        wallet_id: &str,
    ) -> Result<(), anyhow::Error> {
        let nodes_for_wallet = storage_nodes
            .iter()
            .filter(|node| node.has_wallet_state(wallet_id))
            .collect::<Vec<_>>();

        const MIN_STORAGE_REPLICAS: usize = 3;
        if nodes_for_wallet.len() < MIN_STORAGE_REPLICAS {
            Self::trigger_state_replication(wallet_id)?;
        }

        let state_hashes: std::collections::HashSet<_> = nodes_for_wallet
            .iter()
            .map(|node| node.get_state_hash(wallet_id))
            .collect::<Result<_, anyhow::Error>>()?;

        if state_hashes.len() != 1 {
            return Err(anyhow::anyhow!("Inconsistent state across storage nodes"));
        }

        Ok(())
    }

    fn trigger_state_replication(wallet_id: &str) -> Result<(), anyhow::Error> {
        let available_nodes = Self::get_available_storage_nodes()?;
        let source_node = Self::find_node_with_state(wallet_id, &available_nodes)?;
        let target_nodes = Self::select_replication_targets(&available_nodes, 3)?;

        for target in target_nodes {
            Self::replicate_state(wallet_id, &source_node, &target)?;
        }

        Ok(())
    }

    pub async fn run_root_submission_loop(&mut self) -> Result<(), anyhow::Error> {
        let state_update_interval = std::time::Duration::from_secs(60);

        loop {
            #[cfg(feature = "native")]
            async_std::task::sleep(state_update_interval).await;

            if let Err(e) = self.prepare_and_submit_root_state().await {
                log::error!("Root state submission failed: {}", e);
                Self::handle_submission_error(e).await?;
            }
        }
    }

    async fn prepare_and_submit_root_state(&mut self) -> Result<(), anyhow::Error> {
        let new_root = self.calculate_intermediate_root()?;
        let zkp = self.generate_root_transition_proof(&new_root)?;
        let boc = self.generate_root_boc(&new_root, &zkp)?;

        self.submit_to_root_contract(boc, zkp).await?;
        self.update_storage_nodes_root_state(&new_root, &zkp)?;
        self.last_root_submission = Some((self.current_epoch(), new_root.clone()));

        Ok(())
    }

    fn calculate_intermediate_root(&self) -> Result<Vec<u8>, anyhow::Error> {
        let mut tree = SparseMerkleTreeI::new();

        for (wallet_id, state) in &self.wallet_states {
            let leaf = state.calculate_hash()?;
            tree.insert(wallet_id.as_bytes(), &leaf)?;
        }

        Ok(tree.root()?.to_vec())
    }

    fn generate_root_transition_proof(&self, new_root: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let old_root = self
            .last_root_submission
            .as_ref()
            .map(|(_, root)| root.as_slice())
            .unwrap_or(&[0u8; 32]);

        let inputs = ProofInputsI {
            old_root: old_root.to_vec(),
            new_root: new_root.to_vec(),
            epoch: self.current_epoch(),
            wallet_states: self.wallet_states.clone(),
        };

        self.proof_system.generate_root_transition_proof(&inputs)
    }

    fn generate_root_boc(&self, root: &[u8], zkp: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let mut boc = BOC::new(0, 0);
        boc.add_cell(root.to_vec())?;
        boc.add_cell(zkp.to_vec())?;
        boc.add_cell(self.current_epoch().to_le_bytes().to_vec())?;
        Ok(boc.serialize()?)
    }

    async fn submit_to_root_contract(
        &self,
        boc: Vec<u8>,
        zkp: Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        let submission = RootSubmission {
            boc,
            zkp,
            timestamp: SystemTime::now(),
            zk_proof: todo!(),
        };

        self.root_contract.submit_state(submission).await?;
        Ok(())
    }

    fn current_epoch(&self) -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    async fn handle_submission_error(error: anyhow::Error) -> Result<(), anyhow::Error> {
        log::warn!("Handling submission error: {}", error);
        #[cfg(feature = "native")]
        task::sleep(std::time::Duration::from_secs(5)).await;
        Ok(())
    }

    fn get_available_storage_nodes() -> Result<Vec<StorageNode>, anyhow::Error> {
        // Implementation needed
        unimplemented!()
    }

    fn find_node_with_state(
        wallet_id: &str,
        nodes: &[StorageNode],
    ) -> Result<StorageNode, anyhow::Error> {
        // Implementation needed
        unimplemented!()
    }

    fn select_replication_targets(
        nodes: &[StorageNode],
        count: usize,
    ) -> Result<Vec<StorageNode>, anyhow::Error> {
        // Implementation needed
        unimplemented!()
    }

    fn replicate_state(
        wallet_id: &str,
        source: &StorageNode,
        target: &StorageNode,
    ) -> Result<(), anyhow::Error> {
        // Implementation needed
        unimplemented!()
    }

    fn update_storage_nodes_root_state(
        &self,
        new_root: &[u8],
        zkp: &[u8],
    ) -> Result<(), anyhow::Error> {
        // Implementation needed
        unimplemented!()
    }
}
