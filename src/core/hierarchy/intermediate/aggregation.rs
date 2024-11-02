// src/core/hierarchy/intermediate/aggregation.rs

use crate::core::hierarchy::intermediate::intermediate_tree_manager::IntermediateTreeManager;
use crate::core::types::ovp_types::*;

pub struct Aggregator;

impl Aggregator {
    /// Aggregates wallet root states into an intermediate root.
    pub fn aggregate_wallet_roots(
        wallet_states: &[WalletRootState],
    ) -> Result<[u8; 32], SystemError> {
        let mut tree_manager = IntermediateTreeManager::new();
        for state in wallet_states {
            tree_manager.insert_wallet_state(state)?;
        }
        let root_hash = tree_manager.calculate_root_hash()?;
        Ok(root_hash)
    }
}
