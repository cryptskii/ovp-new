use std::collections::HashMap;

#[derive(Clone)]
pub struct GlobalState<StateTransitionRecord> {
    pub root_hash: [u8; 32],
    pub epoch_number: u64,
    pub accounts: HashMap<String, AccountState>,
    pub total_balance: u64,
    pub state_transitions: Vec<StateTransitionRecord>,
    last_epoch_id: u64,
    intermediate_roots: HashMap<u64, [u8; 32]>, // SEQNO -> Merkle root mapping
    global_merkle_tree: Vec<[u8; 32]>,
}

#[derive(Clone)]
pub struct AccountState {
    pub balance: u64,
    pub nonce: u64,
}

pub struct StateTransitionRecord {
    pub epoch_id: u64,
    pub root_hash: [u8; 32],
    pub affected_wallet_ids: Vec<[u8; 32]>,
    pub timestamp: u64,
}

impl<StateTransitionRecord> GlobalState<StateTransitionRecord> {
    /// Creates a new global state.
    pub fn new(root_hash: [u8; 32], epoch_number: u64) -> Self {
        Self {
            root_hash,
            epoch_number,
            accounts: HashMap::new(),
            total_balance: 0,
            state_transitions: Vec::new(),
            last_epoch_id: 0,
            intermediate_roots: HashMap::new(),
            global_merkle_tree: Vec::new(),
        }
    }

    /// Updates the root hash with the given value.
    pub fn update_root_hash(&mut self, new_root_hash: [u8; 32]) {
        self.root_hash = new_root_hash;
    }

    /// Updates the global state with a new root hash, balance, and epoch ID.
    pub fn update(&mut self, new_root_hash: [u8; 32], balance_update: i64, epoch_id: u64) {
        self.root_hash = new_root_hash;
        self.total_balance = (self.total_balance as i64 + balance_update).max(0) as u64;
        self.last_epoch_id = epoch_id;
    }

    /// Updates the intermediate contract state and recalculates global Merkle root
    pub fn update_intermediate_state(&mut self, seqno: u64, intermediate_root: [u8; 32]) {
        // Update intermediate root in mapping
        self.intermediate_roots.insert(seqno, intermediate_root);

        // Update global Merkle tree (O(log m) operation)
        self.update_global_merkle_tree(seqno, intermediate_root);
    }

    /// Updates the global Merkle tree after an intermediate contract update
    fn update_global_merkle_tree(&mut self, seqno: u64, new_root: [u8; 32]) {
        // Implementation would update the Merkle tree path from leaf to root
        // This is a O(log m) operation where m is number of intermediate contracts

        // Placeholder for actual Merkle tree update logic
        if let Some(index) = self.get_leaf_index(seqno) {
            self.global_merkle_tree[index] = new_root;
            self.recompute_global_root();
        }
    }

    fn get_leaf_index(&self, seqno: u64) -> Option<usize> {
        // Implementation to map SEQNO to Merkle tree leaf index
        Some(seqno as usize)
    }

    fn recompute_global_root(&mut self) {
        // Implementation to recompute global Merkle root
        // This is a O(log m) operation
    }

    /// Adds a state transition record.
    pub fn record_state_transition(&mut self, record: StateTransitionRecord) {
        self.state_transitions.push(record);
    }
}

impl StateTransitionRecord {
    pub fn new(
        epoch_id: u64,
        root_hash: [u8; 32],
        wallet_ids: Vec<[u8; 32]>,
        timestamp: u64,
    ) -> Self {
        Self {
            epoch_id,
            root_hash,
            affected_wallet_ids: wallet_ids,
            timestamp,
        }
    }
}
