use crate::core::error::errors::SystemError;
use crate::core::hierarchy::intermediate::destination_contract::DestinationContract;
use crate::core::hierarchy::intermediate::sparse_merkle_tree_i::SparseMerkleTreeI;
use crate::core::storage_node::storage_node_contract::{StorageNode, StorageNodeConfig};
use crate::core::types::ovp_ops::IntermediateOpCode;
use crate::core::zkps::plonky2::Plonky2System;
use bincode::deserialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct IntermediateContract {
    pub wallet_states: HashMap<String, WalletRootState>,
    pub pending_updates: HashMap<String, Vec<WalletStateUpdate>>,
    pub pending_channels: HashMap<String, ChannelRequest>,
    pub closing_channels: HashMap<String, ChannelClosureRequest>,
    pub rebalance_queue: VecDeque<RebalanceRequest>,
    pub storage_nodes: StorageNode<String, Vec<u8>>,
    pub destination_contract: DestinationContract,
    pub intermediate_tree: SparseMerkleTreeI,
    pub zk_verifier: Plonky2System,
    pub tree_manager: TreeManager,
    pub challenge_threshold: u64,
    pub challenge_interval: Duration,
    pub response_threshold: u64,
    pub response_interval: Duration,
    pub battery_level: f64,
    pub last_sync: SystemTime,
}

impl IntermediateContract {
    // Constants based on blueprint specifications
    const REBALANCE_THRESHOLD: f64 = 0.8;
    const MAX_CHANNEL_DENSITY: u32 = 1000;
    const MAX_BALANCE_SKEW: f64 = 0.2;
    const MAX_CHANNEL_DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 30); // 30 days
    const MAX_STORAGE_NODE_DURATION: Duration = Duration::from_secs(60 * 60 * 24 * 7); // 7 days
    const MAX_STORAGE_NODE_REPLICAS: usize = 3;
    const MAX_STORAGE_NODE_BATCH_SIZE: usize = 1000;
    const MAX_UPDATES_PER_BATCH: usize = 1000;
    const MIN_STORAGE_REPLICAS: usize = 3;
    const STATE_UPDATE_INTERVAL: Duration = Duration::from_secs(60);
    const BATTERY_CHARGE_RATE: f64 = 0.1;
    const BATTERY_DISCHARGE_RATE: f64 = 0.05;

    pub fn new(
        node_id: [u8; 32],
        stake_amount: u64,
        config: StorageNodeConfig,
        root_tree: impl RootTree,
        intermediate_tree_manager: impl IntermediateTreeManager,
        initial_peers: HashSet<[u8; 32]>,
    ) -> Result<Self, SystemError> {
        Ok(Self {
            wallet_states: HashMap::new(),
            pending_updates: HashMap::new(),
            pending_channels: HashMap::new(),
            closing_channels: HashMap::new(),
            rebalance_queue: VecDeque::new(),
            storage_nodes: StorageNode::new(
                node_id,
                stake_amount,
                config,
                root_tree,
                intermediate_tree_manager,
                initial_peers,
            )?,
            destination_contract: DestinationContract::new(String::from("default"), 1000),
            intermediate_tree: SparseMerkleTreeI::new(),
            zk_verifier: Plonky2System::default(),
            tree_manager: TreeManager::new(),
            challenge_threshold: 100,
            challenge_interval: Duration::from_secs(3600),
            response_threshold: 50,
            response_interval: Duration::from_secs(1800),
            battery_level: 100.0,
            last_sync: SystemTime::now(),
        })
    }

    pub fn dispatch(
        &mut self,
        op_code: IntermediateOpCode,
        sender: String,
        params: Vec<u8>,
    ) -> Result<(), SystemError> {
        // Update battery based on time since last sync
        self.update_battery_level();

        if self.battery_level < 80.0 {
            return Err(SystemError::InsufficientBattery);
        }

        match op_code {
            IntermediateOpCode::UpdateState => {
                let update: WalletStateUpdate = deserialize(&params)?;
                self.process_state_update(sender, update)
            }
            IntermediateOpCode::CreateChannel => {
                let request: ChannelRequest = deserialize(&params)?;
                self.process_channel_request(sender, request)
            }
            IntermediateOpCode::CloseChannel => {
                let closure: ChannelClosureRequest = deserialize(&params)?;
                self.process_channel_closure(sender, closure)
            }
            IntermediateOpCode::RequestRebalance => {
                let rebalance: RebalanceRequest = deserialize(&params)?;
                self.process_rebalance_request(sender, rebalance)
            }
            _ => Err(SystemError::InvalidOperation),
        }
    }

    fn update_battery_level(&mut self) {
        let now = SystemTime::now();
        let elapsed = now
            .duration_since(self.last_sync)
            .unwrap_or(Duration::from_secs(0));
        let hours = elapsed.as_secs_f64() / 3600.0;

        // Dynamic battery adjustment based on network conditions
        let network_load = self.calculate_network_load();
        let network_stress = self.calculate_network_stress();

        let charge_rate = Self::BATTERY_CHARGE_RATE * (1.0 + 0.5 * network_load);
        let discharge_rate = Self::BATTERY_DISCHARGE_RATE * (1.0 + 0.5 * network_stress);

        if self.is_synchronized() {
            self.battery_level = (self.battery_level + charge_rate * hours).min(100.0);
        } else {
            self.battery_level = (self.battery_level - discharge_rate * hours).max(0.0);
        }

        self.last_sync = now;
    }

    fn calculate_network_load(&self) -> f64 {
        let active_txs = self
            .pending_updates
            .values()
            .map(|v| v.len())
            .sum::<usize>();
        active_txs as f64 / Self::MAX_UPDATES_PER_BATCH as f64
    }

    fn calculate_network_stress(&self) -> f64 {
        // Simplified stress calculation based on failed syncs
        self.storage_nodes.get_sync_failure_rate()
    }

    fn is_synchronized(&self) -> bool {
        self.storage_nodes.is_synchronized()
    }

    fn process_state_update(
        &mut self,
        sender: String,
        update: WalletStateUpdate,
    ) -> Result<(), SystemError> {
        // Verify the update proof
        if !self.zk_verifier.verify(&update.proof) {
            return Err(SystemError::InvalidProof);
        }

        // Update the intermediate tree
        self.intermediate_tree
            .update(update.key.clone(), update.value.clone())?;

        // Add to pending updates
        self.pending_updates
            .entry(sender)
            .or_insert_with(Vec::new)
            .push(update);

        // Process batch if threshold reached
        if self
            .pending_updates
            .values()
            .map(|v| v.len())
            .sum::<usize>()
            >= Self::MAX_UPDATES_PER_BATCH
        {
            self.process_update_batch()?;
        }

        Ok(())
    }

    fn process_update_batch(&mut self) -> Result<(), SystemError> {
        let updates: Vec<_> = self.pending_updates.values().flatten().cloned().collect();

        // Generate aggregate proof
        let aggregate_proof = self
            .zk_verifier
            .aggregate_proofs(updates.iter().map(|u| &u.proof).collect())?;

        // Update destination contract
        self.destination_contract
            .apply_updates(updates, &aggregate_proof)?;

        // Clear pending updates
        self.pending_updates.clear();

        Ok(())
    }

    fn process_channel_request(
        &mut self,
        sender: String,
        request: ChannelRequest,
    ) -> Result<(), SystemError> {
        // Verify channel creation constraints
        if self.pending_channels.len() >= Self::MAX_CHANNEL_DENSITY as usize {
            return Err(SystemError::ChannelLimitExceeded);
        }

        // Verify the request proof
        if !self.zk_verifier.verify(&request.proof) {
            return Err(SystemError::InvalidProof);
        }

        // Add to pending channels
        self.pending_channels.insert(sender, request);

        Ok(())
    }

    fn process_channel_closure(
        &mut self,
        sender: String,
        closure: ChannelClosureRequest,
    ) -> Result<(), SystemError> {
        // Verify closure proof
        if !self.zk_verifier.verify(&closure.proof) {
            return Err(SystemError::InvalidProof);
        }

        // Add to closing channels
        self.closing_channels.insert(sender, closure);

        Ok(())
    }

    fn process_rebalance_request(
        &mut self,
        sender: String,
        request: RebalanceRequest,
    ) -> Result<(), SystemError> {
        // Verify rebalance conditions
        if !self.needs_rebalancing() {
            return Err(SystemError::RebalanceNotNeeded);
        }

        // Verify request proof
        if !self.zk_verifier.verify(&request.proof) {
            return Err(SystemError::InvalidProof);
        }

        // Add to rebalance queue
        self.rebalance_queue.push_back(request);

        Ok(())
    }

    fn needs_rebalancing(&self) -> bool {
        let total_balance: u64 = self.wallet_states.values().map(|w| w.balance).sum();
        let max_balance: u64 = self
            .wallet_states
            .values()
            .map(|w| w.balance)
            .max()
            .unwrap_or(0);

        (max_balance as f64 / total_balance as f64) > Self::MAX_BALANCE_SKEW
    }
}
