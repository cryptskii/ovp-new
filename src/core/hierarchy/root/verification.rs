// src/core/hierarchy/root/verification.rs
use crate::core::hierarchy::intermediate_verifier::VerificationError;
use crate::core::hierarchy::root::sparse_merkle_tree_r::*;
use crate::core::types::ovp_types::GlobalState;
use plonky2::plonk::proof::Proof as PlonkyProof;
use subtle::ConstantTimeEq;

pub struct RootVerification;

impl RootVerification {
    /// Verifies the integrity of an epoch.
    pub fn verify_epoch(epoch: &GlobalState, proof: &PlonkyProof) -> Result<(), VerificationError> {
        // Verify epoch number is valid
        if epoch.number == 0 {
            return Err(VerificationError::InvalidEpochNumber);
        }

        // Verify proof matches epoch state
        let computed_hash = epoch.compute_hash();
        if computed_hash != proof.epoch_hash {
            return Err(VerificationError::HashMismatch);
        }

        // Verify signatures in proof
        if !proof.verify_signatures() {
            return Err(VerificationError::InvalidSignatures);
        }

        // Verify state transitions
        if !epoch.verify_state_transitions() {
            return Err(VerificationError::InvalidStateTransition);
        }

        // Verify validator set
        if !epoch.verify_validator_set() {
            return Err(VerificationError::InvalidValidatorSet);
        }

        Ok(())
    }

    /// Verifies the integrity of the global state.
    pub fn verify_global_state(global_state_hash: [u8; 32], expected_hash: [u8; 32]) -> bool {
        // Compare hashes using constant-time comparison to prevent timing attacks
        global_state_hash.ct_eq(&expected_hash).into()
    }
}
