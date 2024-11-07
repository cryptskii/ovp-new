// ./src/core/hierarchy/client/mod.rs

pub mod channel;
pub mod wallet_extension;
// Re-exporting the types for convenience
pub use channel::*;
pub use wallet_extension::*;
