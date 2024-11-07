use crate::core::hierarchy::intermediate::intermediate_contract::IntermediateContract;
use crate::core::hierarchy::intermediate::sparse_merkle_tree_i::SparseMerkleTreeI;
use crate::core::hierarchy::intermediate::state_tracking_i::ProofInputsI;

use crate::core::storage_node::storage_node_contract::StorageNode;
use crate::core::types::boc::BOC;
use log::{error, warn};
use rand::{thread_rng, Rng};
use std::collections::HashSet;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(feature = "native")]
use async_std::task;

impl IntermediateContract {
    fn verify_storage_state<R, I>(
        storage_nodes: &Vec<StorageNode<R, I>>,
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

        let state_hashes: HashSet<_> = nodes_for_wallet
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
        let state_update_interval = Duration::from_secs(60);

        loop {
            #[cfg(feature = "native")]
            async_std::task::sleep(state_update_interval).await;

            if let Err(e) = self.prepare_and_submit_root_state().await {
                error!("Root state submission failed: {}", e);
                Self::handle_submission_error(e).await?;
            }
        }
    }

    async fn prepare_and_submit_root_state(&mut self) -> Result<(), anyhow::Error> {
        let new_root = self.calculate_intermediate_root()?;
        let zkp = self.generate_root_transition_proof(&new_root)?;
        let boc = self.generate_root_boc(&new_root, &zkp)?;
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let zk_proof = Vec::new(); // Placeholder, replace with actual ZK proof
        let public_inputs = Vec::new(); // Placeholder, replace with actual public inputs
        let merkle_root = new_root.clone();

        self.submit_to_root_contract(
            boc,
            zkp.clone(),
            timestamp,
            zk_proof,
            public_inputs,
            merkle_root,
        )
        .await?;
        self.update_storage_nodes_root_state(&new_root, &zkp)?;

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
        let old_root = self.get_last_root().unwrap_or(&[0u8; 32]);

        let inputs = ProofInputsI {
            old_root: old_root.to_vec(),
            new_root: new_root.to_vec(),
            epoch: self.current_epoch(),
            wallet_states: self.wallet_states.clone(),
        };

        self.proof_system.generate_root_transition_proof(&inputs)
    }

    fn generate_root_boc(&self, root: &[u8], zkp: &[u8]) -> Result<Vec<u8>, anyhow::Error> {
        let mut boc = BOC::new();
        boc.add_cell(root.to_vec())?;
        boc.add_cell(zkp.to_vec())?;
        boc.add_cell(self.current_epoch().to_le_bytes().to_vec())?;
        Ok(boc.serialize()?)
    }

    async fn submit_to_root_contract(
        &self,
        boc: Vec<u8>,
        zkp: Vec<u8>,
        timestamp: u64,
        zk_proof: Vec<u8>,
        public_inputs: Vec<u64>,
        merkle_root: Vec<u8>,
    ) -> Result<(), anyhow::Error> {
        let submission = RootSubmission {
            boc,
            zkp,
            timestamp,
            zk_proof,
            public_inputs,
            merkle_root,
        };

        self.destination_contract.submit_root(submission).await?;
        self.calculate_intermediate_root()?;
        Ok(())
    }
    fn current_epoch(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    async fn handle_submission_error(error: anyhow::Error) -> Result<(), anyhow::Error> {
        warn!("Handling submission error: {}", error);
        #[cfg(feature = "native")]
        task::sleep(Duration::from_secs(5)).await;
        Ok(())
    }

    fn get_available_storage_nodes<R, I>() -> Result<Vec<StorageNode<R, I>>, anyhow::Error> {
        let mut nodes = Vec::new();
        let available_nodes = StorageNode::<R, I>::get_all_nodes()?;

        for node in available_nodes {
            if node.is_available() && node.has_capacity() {
                nodes.push(node);
            }
        }

        Ok(nodes)
    }

    fn find_node_with_state<R, I>(
        wallet_id: &str,
        nodes: &[StorageNode<R, I>],
    ) -> Result<StorageNode<R, I>, anyhow::Error> {
        nodes
            .iter()
            .find(|node| node.has_wallet_state(wallet_id))
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("No node found with wallet state"))
    }

    fn select_replication_targets<R, I>(
        nodes: &[StorageNode<R, I>],
        count: usize,
    ) -> Result<Vec<StorageNode<R, I>>, anyhow::Error> {
        let mut rng = thread_rng();
        let mut selected = HashSet::new();
        let mut targets = Vec::new();

        while targets.len() < count && selected.len() < nodes.len() {
            let idx = rng.gen_range(0..nodes.len());
            if selected.insert(idx) {
                targets.push(nodes[idx].clone());
            }
        }

        if targets.len() < count {
            return Err(anyhow::anyhow!("Insufficient nodes for replication"));
        }

        Ok(targets)
    }

    fn replicate_state<R, I>(
        wallet_id: &str,
        source: &StorageNode<R, I>,
        target: &StorageNode<R, I>,
    ) -> Result<(), anyhow::Error> {
        let state_data = source.get_wallet_state(wallet_id)?;
        target.store_wallet_state(wallet_id, state_data)?;
        target.verify_stored_state(wallet_id)?;
        Ok(())
    }

    fn update_storage_nodes_root_state(
        &self,
        new_root: &[u8],
        zkp: &[u8],
    ) -> Result<(), anyhow::Error> {
        let nodes = Self::get_available_storage_nodes()?;
        let update_data = RootStateUpdate {
            root: new_root.to_vec(),
            proof: zkp.to_vec(),
            timestamp: SystemTime::now(),
        };

        for node in nodes {
            node.update_root_state(&update_data)?;
        }

        Ok(())
    }
}

#[derive(Clone)]
struct RootStateUpdate {
    root: Vec<u8>,
    proof: Vec<u8>,
    timestamp: SystemTime,
}
