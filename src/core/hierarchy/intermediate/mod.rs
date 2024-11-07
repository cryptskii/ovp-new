// ./src/core/hierarchy/intermediate/mod.rs

// src/core/hierarchy/intermediate/mod.rs
pub mod aggregation;
pub mod destination_contract;
pub mod intermediate_contract;
pub mod intermediate_create;
pub mod intermediate_tree_manager;
pub mod proof_exporter_i;
pub mod settlement_i;
pub mod sparse_merkle_tree_i;
pub mod state_tracking_i;

// Re-exporting the types for convenience
pub use destination_contract::DestinationContract;
pub use intermediate_contract::IntermediateContract;
pub use intermediate_create::IntermediateContractCreate;
pub use intermediate_tree_manager::IntermediateTreeManager;
pub use proof_exporter_i::ProofExporterI;
pub use settlement_i::SettlementIntermediate;
pub use sparse_merkle_tree_i::SparseMerkleTreeI;
pub use state_tracking_i::ProofInputsI;
