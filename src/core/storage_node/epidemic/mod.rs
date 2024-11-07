// ./src/core/storage_node/epidemic/mod.rs

pub mod overlap;
pub mod propagation;
pub mod sync;

pub use propagation::BatteryPropagation;
pub use sync::SynchronizationManager;

use crate::core::storage_node::storage_node_contract::StorageNode;
use crate::core::types::EpidemicProtocol;
use std::sync::{Arc, RwLock};

impl<K, V, RootTree, IntermediateTreeManager>
    EpidemicProtocol<K, V, RootTree, IntermediateTreeManager>
{
    pub fn new(
        storage_node: Arc<StorageNode<K, V>>,
        synchronization_manager: Arc<
            RwLock<SynchronizationManager<RootTree, IntermediateTreeManager>>,
        >,
        battery_propagation: Arc<RwLock<BatteryPropagation<RootTree, IntermediateTreeManager>>>,
        redundancy_factor: f64,
        propagation_probability: f64,
    ) -> Self {
        Self {
            storage_node,
            synchronization_manager,
            battery_propagation,
            last_check: 0,
            redundancy_factor,
            propagation_probability,
        }
    }

    pub async fn start(&mut self) {
        self.last_check = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.epidemic_sync().await;
    }

    async fn epidemic_sync(&self) {
        let p = self.propagation_probability;
        let rho = self.redundancy_factor;

        let random_val: f64 = rand::random();

        if random_val < p {
            let target_nodes = self.get_random_neighbors(rho as usize);

            for node in target_nodes {
                let sync_prob = self.calculate_sync_probability(&node);

                if rand::random::<f64>() < sync_prob {
                    self.propagate_data_to_node(&node).await;
                }
            }
        }

        let failed_fraction = 0.2;
        let availability = self.calculate_availability(failed_fraction);

        let epsilon = 0.001;
        let convergence_time = self.expected_convergence_time(epsilon);

        let metrics =
            EpidemicMetrics::new(self.last_check, self.redundancy_factor, p, availability);
    }

    async fn propagate_data_to_node(&self, node: &StorageNode<K, V>) {
        let data_pairs = self.storage_node.get_root_proof_pairs().await;

        for (root, proof) in data_pairs {
            if self.verify_proof(&root, &proof) {
                if let Err(e) = node.store_data(root.clone(), proof.clone()).await {
                    log::error!("Failed to propagate data to node: {}", e);
                    continue;
                }

                let p = self.propagation_probability;
                if rand::random::<f64>() < p {
                    let secondary_nodes =
                        self.get_random_neighbors(self.redundancy_factor as usize);

                    for sec_node in secondary_nodes {
                        if sec_node.id() != node.id() {
                            if let Err(e) = sec_node.store_data(root.clone(), proof.clone()).await {
                                log::error!("Failed secondary propagation: {}", e);
                            }
                        }
                    }
                }
            }
        }
    }

    fn verify_proof(&self, root: &K, proof: &V) -> bool {
        // Actual proof verification logic here
        true
    }

    fn calculate_sync_probability(&self, node: &StorageNode<K, V>) -> f64 {
        let p = self.propagation_probability;
        let t = 1.0;
        let k = self.redundancy_factor;

        let sync_prob = 1.0 - (1.0 - p).powf(t * k);

        sync_prob.max(0.0).min(1.0)
    }

    fn get_random_neighbors(&self, count: usize) -> Vec<StorageNode<K, V>> {
        let all_nodes = self.get_all_nodes();

        if all_nodes.len() <= count {
            return all_nodes;
        }

        let mut rng = rand::thread_rng();
        let mut selected_nodes = Vec::with_capacity(count);
        let mut indices: Vec<usize> = (0..all_nodes.len()).collect();

        for i in 0..count {
            let j = rng.gen_range(i..indices.len());
            indices.swap(i, j);
            selected_nodes.push(all_nodes[indices[i]].clone());
        }

        selected_nodes
    }

    fn get_all_nodes(&self) -> Vec<StorageNode<K, V>> {
        // Retrieves all available nodes in the network
        vec![]
    }

    pub fn calculate_availability(&self, failed_fraction: f64) -> f64 {
        let p = self.propagation_probability;
        let rho = self.redundancy_factor;

        let exponent = rho * (1.0 - failed_fraction);
        let availability = 1.0 - (1.0 - p).powf(exponent);

        availability.max(0.0).min(1.0)
    }

    pub fn expected_convergence_time(&self, epsilon: f64) -> f64 {
        let p = self.propagation_probability;

        if p >= 1.0 {
            return 0.0;
        }

        if p <= 0.0 {
            return f64::INFINITY;
        }

        let numerator = (1.0 - epsilon).ln();
        let denominator = (1.0 - p).ln();

        if denominator == 0.0 {
            return 0.0;
        }

        let time = numerator / denominator;
        time.max(0.0)
    }
}

#[derive(Debug, Clone)]
pub struct EpidemicMetrics {
    pub last_check: u64,
    pub redundancy_factor: f64,
    pub propagation_probability: f64,
    pub sync_probability: f64,
}

impl EpidemicMetrics {
    pub fn new(
        last_check: u64,
        redundancy_factor: f64,
        propagation_probability: f64,
        sync_probability: f64,
    ) -> Self {
        Self {
            last_check,
            redundancy_factor,
            propagation_probability,
            sync_probability,
        }
    }
}

impl Default for EpidemicMetrics {
    fn default() -> Self {
        Self::new(0, 3.0, 0.7, 0.0)
    }
}

impl<K, V> StorageNode<K, V> {
    pub async fn store_data(&self, root: K, proof: V) -> Result<(), String> {
        Ok(())
    }

    pub async fn get_root_proof_pairs(&self) -> Vec<(K, V)> {
        vec![]
    }

    pub fn id(&self) -> usize {
        0
    }
}

