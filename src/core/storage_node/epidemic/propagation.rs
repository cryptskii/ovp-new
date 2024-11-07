// ./src/core/storage_node/epidemic/propagation.rs

// Battery Charging Propagation
// This module implements the battery charging protocol for maintaining node synchronization.
// It uses a battery-based mechanism to ensure nodes remain synchronized and properly
// overlapping with other nodes in the network.

use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::time::Duration;

use crate::core::storage_node::storage_node_contract::StorageNode;

pub struct BatteryPropagation<RootTree, IntermediateTreeManager> {
    storage_node: Arc<StorageNode<RootTree, IntermediateTreeManager>>,
    battery_level: AtomicU64, // 0-100%
    charge_interval: Duration,
    optimal_threshold: f64,    // 98%
    high_threshold: f64,       // 80%
    suspension_threshold: f64, // 0%
    charge_rate: f64,          // Rate of charging based on overlapping nodes
    discharge_rate: f64,       // Rate of discharge when out of sync
    suspension_duration: Duration,
    min_nodes_for_charging: u64,
}
