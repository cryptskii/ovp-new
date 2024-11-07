// ./src/core/hierarchy/intermediate/intermediate_contract_types.rs

use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::ChannelClosureRequest;

use crate::core::hierarchy::client::wallet_extension::tree_manager::TreeManager;
use crate::core::hierarchy::intermediate::destination_contract::DestinationContract;
use crate::core::hierarchy::intermediate::sparse_merkle_tree_i::SparseMerkleTreeI;
use crate::core::storage_node::storage_node_contract::StorageNode;
use crate::core::zkps::plonky2::Plonky2System;
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::time::{Duration, SystemTime};

pub struct IntermediateContract<RebalanceRequest> {
    pub auto_rebalance: bool,
    pub battery_charge_rate: f64,
    pub battery_discharge_rate: f64,
    pub battery_level: f64,
    pub battery_wait_time: Duration,
    pub challenge_interval: Duration,
    pub challenge_threshold: u64,
    pub closing_channels: HashMap<String, ChannelClosureRequest>,
    pub destination_contract: DestinationContract,
    pub intermediate_tree: SparseMerkleTreeI,
    pub last_sync: SystemTime,
    pub max_channel_density: u32,
    pub max_storage_nodes: u32,
    pub max_storage_node_batch_size: u32,
    pub max_updates_per_batch: u32,
    pub min_storage_nodes: u32,
    pub state_update_interval: Duration,
    pub storage_nodes: StorageNode<String, Vec<u8>>,
    pub rebalance_queue: VecDeque<RebalanceRequest>,
    pub tree_manager: TreeManager,
    pub zk_verifier: Plonky2System,
    _phantom: PhantomData<(
        SparseMerkleTreeI,
        StorageNode<String, Vec<u8>>,
        TreeManager,
        Plonky2System,
    )>,
}
