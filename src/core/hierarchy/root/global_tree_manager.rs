use crate::core::error::errors::ZkProofError;
use crate::core::store::Store;
use crate::core::zkps::proof::ZkProof;
use blake2::{Blake2b, Digest};

#[derive(Debug)]
pub struct GlobalTreeManager {
    store: Store,
}

impl GlobalTreeManager {
    /// Create a new GlobalTreeManager
    pub fn new(store: Store) -> Self {
        Self { store }
    }

    /// Adds an intermediate root to the global tree, after verification.
    pub fn add_intermediate_root(
        &mut self,
        intermediate_root: [u8; 32],
        proof: ZkProof,
    ) -> Result<(), ZkProofError> {
        // Verify the proof first
        if !proof.is_valid() {
            return Err(ZkProofError {
                message: "Invalid proof".to_string(),
            });
        }

        // Store the intermediate root in persistent storage
        let mut roots = self.get_stored_roots();
        roots.push(intermediate_root);
        self.store_roots(&roots);

        Ok(())
    }

    /// Generates the global root by aggregating intermediate roots.
    pub fn generate_global_root(&self) -> [u8; 32] {
        // Get all stored intermediate roots
        let roots = self.get_stored_roots();

        // If no roots exist, return zero hash
        if roots.is_empty() {
            return [0u8; 32];
        }

        // Combine all intermediate roots using a Merkle tree structure
        let mut current_hash = roots[0];
        for root in roots.iter().skip(1) {
            current_hash = Self::hash_combine(&current_hash, root);
        }

        current_hash
    }

    /// Get all stored intermediate roots from storage
    fn get_stored_roots(&self) -> Vec<[u8; 32]> {
        self.store.get_roots().unwrap_or_else(|_| Vec::new())
    }

    /// Store intermediate roots in persistent storage
    fn store_roots(&mut self, roots: &[[u8; 32]]) {
        if let Err(e) = self.store.store_roots(roots) {
            log::error!("Failed to store roots: {:?}", e);
        }
    }

    /// Combine two hashes to create parent hash
    fn hash_combine(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        let mut hasher = Blake2b::new();
        hasher.update(left);
        hasher.update(right);
        let mut output = [0u8; 32];
        output.copy_from_slice(&hasher.finalize()[..32]);
        output
    }
}
