// ./src/core/zkp/mod.rs

// src/core/zkp/mod.rs

pub mod circuit_builder;
pub mod plonky2;
pub mod proof;
pub mod zkp;

pub use circuit_builder::ZkCircuitBuilder;
pub use plonky2::Plonky2System;
pub use proof::{ProofType, ZkProof};

pub struct ProofManager {
    storage_node: Arc<StorageNode>,
    verification_manager: ProofVerificationManager,
}

impl ProofManager {
    pub fn new(
        storage_node: Arc<StorageNode>,
        verification_threshold: u64,
        verification_interval: Duration,
    ) -> Self {
        let verification_manager = ProofVerificationManager::new(
            storage_node,
            verification_threshold,
            verification_interval,
        );

        Self {
            storage_node,
            verification_manager,
        }
    }

    pub async fn start_proof_verification(&self) -> Result<(), SystemError> {
        let verification_task = tokio::spawn(self.verification_manager.start_proof_verification());

        verification_task.await?;

        Ok(())
    }
}
