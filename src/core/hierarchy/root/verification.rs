use crate::core::hierarchy::root::global_state::GlobalState;
use crate::core::hierarchy::root::intermediate_verifier::VerificationError;
use crate::core::hierarchy::root::sparse_merkle_tree_r::*;
use plonky2::plonk::proof::Proof as PlonkyProof;
use subtle::ConstantTimeEq;

pub struct RootVerification;

impl RootVerification {
    /// Verifies the integrity of an epoch.
    pub fn verify_epoch<
        F: plonky2::hash::hash_types::RichField + plonky2::field::types::Field,
        C: plonky2::plonk::config::GenericConfig<D, F = F>,
        const D: usize,
    >(
        epoch: &GlobalState<StateTransitionRecord>,
        proof: &PlonkyProof<F, C, D>,
    ) -> Result<()> {
        // Verify epoch number is valid
        if epoch.number == 0 {
            return Err(VerificationError::InvalidEpochNumber.into());
        }

        // Verify proof matches epoch state
        let computed_hash = epoch.compute_hash();
        if computed_hash != proof.public_inputs[0] {
            return Err(VerificationError::HashMismatch.into());
        }

        // Verify proof
        if !proof.verify() {
            return Err(VerificationError::InvalidProof.into());
        }

        // Verify state transitions
        if !epoch.verify_state_transitions() {
            return Err(VerificationError::InvalidStateTransition.into());
        }

        // Verify validator set
        if !epoch.verify_validator_set() {
            return Err(VerificationError::InvalidValidatorSet.into());
        }

        Ok(())
    }
    /// Verifies the integrity of the global state.
    pub fn verify_global_state(global_state_hash: [u8; 32], expected_hash: [u8; 32]) -> bool {
        // Compare hashes using constant-time comparison to prevent timing attacks
        global_state_hash.ct_eq(&expected_hash).into()
    }
}
