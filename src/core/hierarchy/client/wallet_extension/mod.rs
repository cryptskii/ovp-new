// src/core/hierarchy/client/wallet_extension/mod.rs
pub mod balance;
pub mod channel_manager;
pub mod client_proof_exporter;
pub mod grouping;
pub mod sparse_merkle_tree_wasm;
pub mod state_tracking;
pub mod transactions_manager;
pub mod tree_manager;
pub mod wallet_extension_contract;

// re-exporting the types for convenience
pub use balance::*;
pub use channel_manager::*;
pub use client_proof_exporter::*;
pub use grouping::*;
pub use sparse_merkle_tree_wasm::*;
pub use state_tracking::*;
pub use transactions_manager::*;
pub use tree_manager::*;
pub use wallet_extension_contract::*;
