// client_proof_exporter.rs
// This module is responsible for exporting the wallet root and its associated proof in a BOC (Bag of Cells) format for submission to the intermediate layer.

use crate::core::types::boc::BOC;
use crate::core::zkps::circuit_builder::ZkCircuitBuilder;
use crate::core::zkps::proof::ZkProof;
use serde::{Deserialize, Serialize};
use sha2::Digest;

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

/// Proof Exporter responsible for creating the BOC and exporting the wallet root and proof.
pub struct ProofExporter;

impl ProofExporter {
    /// Packages the WalletRootProof into a BOC format for submission.
    pub fn export_to_boc(proof_data: &WalletRootProof) -> Result<Vec<u8>, String> {
        let root_cell = Self::create_wallet_root_cell(proof_data)?;
        let boc = root_cell.serialize()?;
        Ok(boc)
    }

    /// Creates the root cell for the BOC, which contains the wallet root, proof, and metadata.
    fn create_wallet_root_cell(proof_data: &WalletRootProof) -> Result<Cell> {
        let mut builder = BuilderData::new();

        // Add wallet root
        builder.append_raw(&proof_data.wallet_root, 256)?;

        // Add proof data
        builder.append_raw(&proof_data.proof.proof_data, 256)?;

        // Add metadata
        builder.append_raw(&proof_data.metadata.wallet_id, 256)?;
        builder.append_u64(proof_data.metadata.timestamp)?;

        builder.append_u64(proof_data.metadata.nonce)?;
        builder.append_u8(proof_data.metadata.proof_type as u8)?;

        // Finalize as Cell
        builder.into_cell()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_to_boc() {
        let proof_data = WalletRootProof {
            wallet_root: [1; 32],
            proof: ZkProof::new(vec![0; 256], vec![42; 8], [1; 32], 1627383948),
            metadata: ProofMetadata {
                timestamp: 1627383948,
                nonce: 42,
                wallet_id: [2; 32],
                proof_type: ProofType::StateTransition,
            },
        };

        let boc = ProofExporter::export_to_boc(&proof_data).unwrap();
        assert!(boc.len() > 0, "BOC should be non-empty");
    }
}
