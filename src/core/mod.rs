// ./src/core/mod.rs

// Core
// This module is the core of the OVP network, responsible for managing the storage nodes, validation nodes, and the network.

// src/core/mod.rs
pub mod conversion;
pub mod error;
pub mod hierarchy;
pub mod state;
pub mod storage_node;
pub mod types;
pub mod validation;
pub mod zkp;

// re-export for convenience

pub use storage_node::*;

pub use validation::*;
pub use zkp::*;
