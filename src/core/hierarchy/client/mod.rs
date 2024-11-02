// ./src/core/hierarchy/client/mod.rs

pub mod channel;
pub mod wallet_extension;
// Re-exporting the types for convenience
pub use self::channel::*;
pub use self::wallet_extension::*;
