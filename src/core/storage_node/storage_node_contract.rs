use crate::core::error::errors::SystemErrorType;
use crate::core::error::SystemError;

use crate::core::epidemic::EpidemicProtocol;
use crate::core::hierarchy::intermediate::IntermediateTreeManager;
use crate::core::storage_node::battery::BatteryChargingSystem;

use crate::core::types::boc::BOC;
use crate::core::zkps::proof::ZkProof;
use anyhow::Result;
use futures::lock::Mutex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

// Configuration settings for StorageNode
#[derive(Clone)]
pub struct StorageNodeConfig {
    pub battery_config: BatteryConfig,
    pub sync_config: SyncConfig,
    pub epidemic_protocol_config: EpidemicProtocolConfig,
    pub network_config: NetworkConfig,
}

#[derive(Clone)]
pub struct BatteryConfig;
#[derive(Clone)]
pub struct SyncConfig;
#[derive(Clone)]
pub struct EpidemicProtocolConfig {
    pub redundancy_factor: u32,
    pub propagation_probability: f64,
}
#[derive(Clone)]
pub struct NetworkConfig;

// Define the core StorageNode structure
pub struct StorageNode<RootTree, IntermediateTree> {
    pub node_id: [u8; 32],
    pub stake: u64,
    pub stored_bocs: Arc<Mutex<HashMap<[u8; 32], BOC>>>,
    pub stored_proofs: Arc<Mutex<HashMap<[u8; 32], ZkProof>>>,
    pub battery_system: Arc<Mutex<BatteryChargingSystem>>,

    pub root_tree: RootTree,
    pub intermediate_tree: IntermediateTree,
    pub intermediate_tree_manager: Arc<Mutex<IntermediateTreeManager<RootTree, IntermediateTree>>>,
    pub config: StorageNodeConfig,
    intermediate_trees: Arc<Mutex<HashMap<u64, RootTree>>>,
    root_trees: Arc<Mutex<HashMap<u64, RootTree>>>,
    peers: Arc<Mutex<HashSet<[u8; 32]>>>,
    epidemic_protocol: Arc<Mutex<EpidemicProtocol<[u8; 32], BOC, RootTree, IntermediateTree>>>,
}
impl<RootTree, IntermediateTree> StorageNode<RootTree, IntermediateTree> {
    pub fn new(
        node_id: [u8; 32],
        initial_stake: u64,
        config: StorageNodeConfig,
        root_tree: RootTree,
        intermediate_tree: IntermediateTree,
        intermediate_tree_manager: IntermediateTreeManager<RootTree, IntermediateTree>,
        peers: HashSet<[u8; 32]>,
    ) -> Result<Self, SystemError> {
        if initial_stake < 1000 {
            return Err(SystemError::new(
                SystemErrorType::InvalidStake,
                "Insufficient initial stake".to_string(),
            ));
        }

        let battery_charging_system =
            BatteryChargingSystem::new(config.battery_config.clone(), initial_stake);
        let epidemic_protocol = EpidemicProtocol::new(
            node_id,
            config.epidemic_protocol_config.redundancy_factor,
            config.epidemic_protocol_config.propagation_probability,
        );

        Ok(Self {
            node_id,
            stake: initial_stake,
            stored_bocs: Arc::new(Mutex::new(HashMap::new())),
            stored_proofs: Arc::new(Mutex::new(HashMap::new())),
            intermediate_trees: Arc::new(Mutex::new(HashMap::new())),
            root_trees: Arc::new(Mutex::new(HashMap::new())),
            peers: Arc::new(Mutex::new(peers)),
            config,
            battery_system: Arc::new(Mutex::new(battery_charging_system)),
            root_tree,
            intermediate_tree,
            intermediate_tree_manager: Arc::new(Mutex::new(intermediate_tree_manager)),
            epidemic_protocol: Arc::new(Mutex::new(epidemic_protocol)),
        })
    }
    // Store an update, including BOC and proof, in an async-compatible function
    pub async fn store_update(&self, boc: BOC, proof: ZkProof) -> Result<(), SystemError> {
        let mut battery_system = self.battery_system.lock().await;
        battery_system
            .charge_for_processing()
            .await
            .map_err(|e| SystemError::new(SystemErrorType::BatterySystemError, e.to_string()))?;

        if !self.verify_proof_internal(&proof, &boc)? {
            return Err(SystemError::new(
                SystemErrorType::InvalidProofError,
                "Invalid proof".to_string(),
            ));
        }

        let boc_id = self.hash_boc_internal(&boc);
        {
            let mut bocs = self.stored_bocs.lock().await;
            bocs.insert(boc_id, boc.clone());

            let mut proofs = self.stored_proofs.lock().await;
            proofs.insert(boc_id, proof.clone());

            let mut tree_manager = self.intermediate_tree_manager.lock().await;
            tree_manager.update_trees(
                &boc,
                &boc_id,
                &mut self.intermediate_trees.lock().await,
                &mut self.root_trees.lock().await,
            )?;
        }

        let mut epidemic_protocol = self.epidemic_protocol.lock().await;
        epidemic_protocol
            .propagate_update(boc, proof)
            .map_err(|e| SystemError::new(SystemErrorType::EpidemicProtocolError, e.to_string()))?;

        Ok(())
    }

    pub async fn retrieve_boc(&self, boc_id: &[u8; 32]) -> Result<BOC, SystemError> {
        let bocs = self.stored_bocs.lock().await;
        bocs.get(boc_id).cloned().ok_or_else(|| {
            SystemError::new(
                SystemErrorType::DataNotFoundError,
                "BOC not found".to_string(),
            )
        })
    }

    pub async fn retrieve_proof(&self, proof_id: &[u8; 32]) -> Result<ZkProof, SystemError> {
        let proofs = self.stored_proofs.lock().await;
        proofs.get(proof_id).cloned().ok_or_else(|| {
            SystemError::new(
                SystemErrorType::DataNotFoundError,
                "Proof not found".to_string(),
            )
        })
    }

    pub async fn add_peer(&self, peer_id: [u8; 32]) -> Result<(), SystemError> {
        let mut peers = self.peers.lock().await;
        peers.insert(peer_id);
        Ok(())
    }

    pub async fn remove_peer(&self, peer_id: &[u8; 32]) -> Result<(), SystemError> {
        let mut peers = self.peers.lock().await;
        peers.remove(peer_id);
        Ok(())
    }

    fn hash_boc_internal(&self, boc: &BOC) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(bincode::serialize(boc).unwrap());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    fn verify_proof_internal(&self, proof: &ZkProof, boc: &BOC) -> Result<bool, SystemError> {
        if !proof.verify_parameters() {
            return Ok(false);
        }

        let boc_hash = self.hash_boc_internal(boc);
        if !proof.verify_commitment(&boc_hash) {
            return Ok(false);
        }
        proof.verify()
    }
}
