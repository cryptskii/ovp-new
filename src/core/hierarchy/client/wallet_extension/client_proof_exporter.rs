// client_proof_exporter.rs
// This module is responsible for exporting the wallet root and its associated proof in a BOC (Bag of Cells) format for submission to the intermediate layer.

use crate::core::error::SystemError;
use crate::core::types::boc::{Cell, BOC};
use crate::core::zkps::circuit_builder::ZkCircuitBuilder;
use crate::core::zkps::proof::ZkProof;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Enum representing different types of proofs.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ProofType {
    StateTransition,
    BalanceCheck,
    // Additional proof types as needed
}

/// Data structure representing a wallet root and its associated proof.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WalletRootProof {
    pub wallet_root: [u8; 32],
    pub proof: ZkProof,
    pub metadata: ProofMetadata,
}

/// Metadata for tracking proof context.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofMetadata {
    pub timestamp: u64,
    pub nonce: u64,
    pub wallet_id: [u8; 32],
    pub proof_type: ProofType,
}

impl WalletRootProof {
    /// Creates a new WalletRootProof with the given wallet root, proof, and metadata.
    pub fn new(wallet_root: [u8; 32], proof: ZkProof, metadata: ProofMetadata) -> Self {
        Self {
            wallet_root,
            proof,
            metadata,
        }
    }
}
