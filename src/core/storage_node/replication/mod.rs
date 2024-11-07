// ./src/core/storage_node/replication/mod.rs

use crate::core::error::SystemErrorType;
use crate::core::storage_node::replication::consistency::ReplicationConsistencyManager;
use crate::core::storage_node::replication::distribution::ReplicationDistributionManager;
use crate::core::storage_node::replication::verification::ReplicationVerificationManager;
use crate::core::storage_node::storage_node_contract::StorageNode;
use std::sync::Arc;
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;
pub mod consistency;
pub mod distribution;
pub mod verification;

// Define ReplicationManager with generics on the verification manager
pub struct ReplicationManager<RootTree, IntermediateTreeManager> {
    pub storage_node: Arc<StorageNode<RootTree, IntermediateTreeManager>>,
    pub distribution_manager: ReplicationDistributionManager,
    pub consistency_manager: ReplicationConsistencyManager,
    pub verification_manager: ReplicationVerificationManager<RootTree, IntermediateTreeManager>, // Add the required generics here
}
impl<RootTree, IntermediateTreeManager> ReplicationManager<RootTree, IntermediateTreeManager> {
    pub fn new(
        storage_node: Arc<StorageNode<RootTree, IntermediateTreeManager>>,
        replication_threshold: u64,
        replication_interval: Duration,
        response_threshold: u64,
        response_interval: Duration,
    ) -> Self {
        let distribution_manager = ReplicationDistributionManager::new(
            wasm_bindgen::JsValue::NULL,
            replication_threshold,
            replication_interval.as_secs() as u32,
        );

        let consistency_manager = ReplicationConsistencyManager::new(
            storage_node.clone(),
            replication_threshold,
            replication_interval,
        );

        let verification_manager = ReplicationVerificationManager::new(
            storage_node.clone(),
            response_threshold,
            response_interval,
        );

        Self {
            storage_node,
            distribution_manager,
            consistency_manager,
            verification_manager,
        }
    }
    pub async fn start_replication(self: Arc<Self>) -> Result<(), SystemErrorType>
    where
        RootTree: 'static,
        IntermediateTreeManager: 'static,
    {
        let self_clone = self.clone();
        spawn_local(async move {
            let _ = self_clone
                .distribution_manager
                .start_replication_distribution();
        });

        let self_clone = self.clone();
        spawn_local(async move {
            let _ = self_clone.consistency_manager.check_consistency();
        });

        let self_clone = self.clone();
        spawn_local(async move {
            let _ = self_clone.verification_manager.verify_replication();
        });

        Ok(())
    }
}
