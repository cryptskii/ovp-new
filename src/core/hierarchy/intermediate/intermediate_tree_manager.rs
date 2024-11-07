use crate::core::error::errors::SystemError;
use crate::core::types::boc::BOC;
/// Intermediate tree manager
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

#[derive(Clone, Debug)]
pub struct IntermediateTreeManager<RootTree, IntermediateTree> {
    pub intermediate_trees: Arc<RwLock<HashMap<[u8; 32], IntermediateTree>>>,
    pub root_trees: Arc<RwLock<HashMap<[u8; 32], RootTree>>>,
}

impl<RootTree, IntermediateTree> IntermediateTreeManager<RootTree, IntermediateTree>
where
    IntermediateTree: Default,
    RootTree: Default,
{
    pub fn new() -> Self {
        Self {
            intermediate_trees: Arc::new(RwLock::new(HashMap::new())),
            root_trees: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn update_trees(
        &self,
        boc: &BOC,
        intermediate_trees: &mut HashMap<[u8; 32], IntermediateTree>,
        root_trees: &mut HashMap<[u8; 32], RootTree>,
    ) -> Result<(), SystemError> {
        // Generate a unique identifier for the BOC
        let boc_id = generate_boc_id(boc);

        // Update intermediate tree
        intermediate_trees.insert(boc_id, IntermediateTree::default());

        // Update root tree
        root_trees.insert(boc_id, RootTree::default());

        Ok(())
    }
}

// Helper function to generate a unique identifier for a BOC
fn generate_boc_id(boc: &BOC) -> [u8; 32] {
    // Implement a method to generate a unique identifier based on BOC contents
    // This is a placeholder implementation and should be replaced with a proper hashing method
    let mut id = [0u8; 32];
    // Use the first root as a simple identifier (not recommended for production use)
    if let Some(first_root) = boc.roots.first() {
        id[..first_root.len()].copy_from_slice(first_root);
    }
    id
}
