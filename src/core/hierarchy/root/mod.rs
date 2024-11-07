// src/core/hierarchy/root/mod.rs

pub mod audit_interface;
pub mod epoch;
pub mod global_state;
pub mod global_tree_manager;
pub mod intermediate_verifier;
pub mod root_contract;
pub mod sparse_merkle_tree_r;

// Re-exporting for ease of access
pub use self::audit_interface::AuditInterface;
pub use self::epoch::Epoch;
pub use self::global_state::GlobalState;
pub use self::global_tree_manager::GlobalTreeManager;
pub use self::intermediate_verifier::IntermediateVerifier;

pub use self::sparse_merkle_tree_r::SparseMerkleTreeR;
