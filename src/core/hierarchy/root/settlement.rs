// src/core/hierarchy/root/settlement.rs

use crate::core::error::SystemError;
use crate::core::hierarchy::root::epoch::EpochStatus;
use crate::core::hierarchy::root::GlobalState;
use crate::core::types::ovp_ops::RootOpCode;

use merkle::Proof as MerkleProof;

use super::global_state::StateTransitionRecord;
use super::root_contract::RootContract;

/// Handles the settlement process at the root level.
pub struct RootSettlement;

impl RootSettlement {
    /// Processes the finalization of an epoch, settling all pending operations.
    pub fn settle_epoch(
        epoch_state: &EpochStatus,
        global_state: &mut GlobalState<StateTransitionRecord>,
    ) -> Result<(), SystemError> {
        // 1. Validate the epoch state
        RootContract::validate_epoch_state(epoch_state)?;

        // 2. Apply pending operations to the global state
        RootOpCode::apply_epoch_operations(epoch_state, global_state)?;

        // 3. Generate a zero-knowledge proof for the epoch settlement
        let proof = MerkleProof::generate_for_epoch(epoch_state, global_state)?;

        // 4. Update the global root hash
        global_state.update_root_hash(epoch_state.new_root_hash);

        // 5. Store the proof and updated state in the database
        // (Assuming a database module is available)
        // db::store_epoch_settlement(epoch_state, global_state, &proof)?;

        Ok(())
    }
}
