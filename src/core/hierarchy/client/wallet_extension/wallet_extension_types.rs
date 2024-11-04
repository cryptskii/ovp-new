use crate::core::error::errors::Error;
use crate::core::hierarchy::client::channel::channel_contract::ChannelContract;
use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::types::boc::{Cell, CellType, BOC};
use crate::core::zkps::plonky2::Plonky2SystemHandle;
use crate::core::zkps::proof::ZkProof;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

struct PlonkySystemHandleWrapper(Arc<Plonky2SystemHandle>);

impl fmt::Debug for PlonkySystemHandleWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PlonkySystemHandleWrapper").finish()
    }
}

impl Default for PlonkySystemHandleWrapper {
    fn default() -> Self {
        Self(Arc::new(Plonky2SystemHandle::new().unwrap()))
    }
}

impl Clone for PlonkySystemHandleWrapper {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug, Clone)]
pub struct Transaction {
    pub id: [u8; 32],
    pub channel_id: [u8; 32],
    pub sender: [u8; 32],
    pub recipient: [u8; 32],
    pub amount: u64,
    pub nonce: u64,
    pub sequence_number: u64,
    pub timestamp: u64,
    pub status: TransactionStatus,
    pub signature: [u8; 64],
    pub zk_proof: Vec<u8>,
    pub merkle_proof: Vec<u8>,
    pub previous_state: Vec<u8>,
    pub new_state: Vec<u8>,
    pub fee: u64,
}

#[derive(Debug, Clone)]
pub enum TransactionStatus {
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct ChannelClosureRequest {
    pub channel_id: [u8; 32],
    pub final_balance: u64,
    pub boc: Vec<u8>,
    pub proof: ZkProof,
    pub signature: [u8; 64],
}

#[derive(Clone)]
pub struct WalletExtension {
    pub wallet_id: [u8; 32],
    pub channels: HashMap<[u8; 32], Arc<RwLock<ChannelContract>>>,
    pub total_locked_balance: u64,
    pub rebalance_config: RebalanceConfig,
    pub proof_system: Arc<PlonkySystemHandleWrapper>,
    pub state_tree: SparseMerkleTreeWasm,
    pub encrypted_states: WalletStorageManager,
    pub balance_tracker: WalletBalanceTracker,
    pub root_tracker: RootStateTracker,
}

impl fmt::Debug for WalletExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletExtension")
            .field("wallet_id", &self.wallet_id)
            .field("total_locked_balance", &self.total_locked_balance)
            .field("rebalance_config", &self.rebalance_config)
            .field("state_tree", &self.state_tree)
            .field("encrypted_states", &self.encrypted_states)
            .field("balance_tracker", &self.balance_tracker)
            .field("root_tracker", &self.root_tracker)
            .finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Channel {
    pub channel_id: [u8; 32],
    pub wallet_id: [u8; 32],
    pub state: Arc<RwLock<PrivateChannelState>>,
    #[serde(skip)]
    pub state_history: Vec<StateTransition>,
    pub participants: Vec<[u8; 32]>,
    #[serde(skip)]
    pub config: ChannelConfig,
    pub spending_limit: u64,
    #[serde(skip)]
    pub proof_system: Arc<PlonkySystemHandleWrapper>,
    #[serde(skip)]
    pub(crate) boc_history: Vec<BOC>,
    pub(crate) proof: Vec<u8>,
}

impl Default for Channel {
    fn default() -> Self {
        Self {
            channel_id: [0u8; 32],
            wallet_id: [0u8; 32],
            state: Arc::new(RwLock::new(PrivateChannelState::default())),
            state_history: Vec::new(),
            participants: Vec::new(),
            config: ChannelConfig::default(),
            spending_limit: 0,
            proof_system: Arc::new(PlonkySystemHandleWrapper::default()),
            boc_history: Vec::new(),
            proof: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub min_balance: u64,
    pub max_balance: u64,
    pub challenge_period: u64,
    pub max_state_size: usize,
    pub max_participants: usize,
    pub timeout: u64,
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self {
            min_balance: 0,
            max_balance: u64::MAX,
            challenge_period: 3600, // 1 hour default
            max_state_size: 1024,   // 1KB default
            max_participants: 2,    // Default to 2 participants
            timeout: 86400,         // 24 hours default
        }
    }
}

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub old_state: PrivateChannelState,
    pub new_state: PrivateChannelState,
    pub proof: ZkProof,
    pub timestamp: u64,
}

impl Default for StateTransition {
    fn default() -> Self {
        Self {
            old_state: PrivateChannelState::default(),
            new_state: PrivateChannelState::default(),
            proof: ZkProof::default(),
            timestamp: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivateChannelState {
    pub balance: u64,
    pub nonce: u64,
    pub sequence_number: u64,
    #[serde(with = "serde_arrays")]
    pub proof: [u8; 64],
    #[serde(with = "serde_arrays")]
    pub signature: [u8; 64],
    #[serde(with = "serde_arrays")]
    pub transaction: [u8; 32],
    pub witness: Vec<u8>,
    #[serde(with = "serde_arrays")]
    pub merkle_root: [u8; 32],
    pub merkle_proof: Vec<u8>,
    pub last_update: u64,
}

impl Default for PrivateChannelState {
    fn default() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            sequence_number: 0,
            proof: [0u8; 64],
            signature: [0u8; 64],
            transaction: [0u8; 32],
            witness: Vec::new(),
            merkle_root: [0u8; 32],
            merkle_proof: Vec::new(),
            last_update: 0,
        }
    }
}

impl Default for ZkProof {
    fn default() -> Self {
        Self {
            proof_data: vec![0u8; 64],
            public_inputs: Vec::new(),
            merkle_root: vec![0u8; 32],
            timestamp: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WalletBalanceTracker {
    pub wallet_balances: HashMap<[u8; 32], u64>,
    pub state_transitions: HashMap<[u8; 32], Vec<[u8; 32]>>,
    pub state_tree: Arc<RwLock<SparseMerkleTreeWasm>>,
    pub last_root: [u8; 32],
    pub pending_updates: Vec<StateTransition>,
}

impl WalletBalanceTracker {
    pub fn new() -> Self {
        Self {
            wallet_balances: HashMap::new(),
            state_transitions: HashMap::new(),
            state_tree: Arc::new(RwLock::new(SparseMerkleTreeWasm::new())),
            last_root: [0u8; 32],
            pending_updates: Vec::new(),
        }
    }

    pub fn get_current_balance(&self, channel_id: &[u8; 32]) -> Result<u64, Error> {
        self.wallet_balances
            .get(channel_id)
            .copied()
            .ok_or(Error::ChannelNotFound)
    }

    pub fn update_balance(
        &mut self,
        channel_id: [u8; 32],
        old_balance: u64,
        new_balance: u64,
        boc: &BOC,
    ) -> Result<(), Error> {
        // Update balance
        self.wallet_balances.insert(channel_id, new_balance);

        // Compute root cell hash for state transitions
        let mut hasher = sha2::Sha256::new();
        if let Some(root_cell) = boc.get_root_cell() {
            hasher.update(&root_cell.data);
            let mut root_hash = [0u8; 32];
            root_hash.copy_from_slice(&hasher.finalize());

            // Update state transitions
            let transitions = self
                .state_transitions
                .entry(channel_id)
                .or_insert_with(Vec::new);
            transitions.push(root_hash);

            // Update state tree
            let mut state_tree = self
                .state_tree
                .write()
                .map_err(|_| Error::StakeError("Lock acquisition failed".to_string()))?;

            // Create cell for state update
            let state_cell = Cell::new(
                root_cell.data.clone(),
                root_cell.references.clone(),
                CellType::Ordinary,
                root_hash,
            );

            let mut state_boc = BOC::new();
            state_boc.add_cell(state_cell);
            state_boc.add_root(0);

            let state_boc_bytes = state_boc
                .serialize()
                .map_err(|_| Error::StakeError("BOC serialization failed".to_string()))?;
            state_tree
                .update(&channel_id, &state_boc_bytes)
                .map_err(|_| Error::StakeError("State update failed".to_string()))?;
            self.last_root = root_hash;
            // Create state transition
            let transition = StateTransition {
                old_state: self.get_previous_state(&channel_id, old_balance)?,
                new_state: self.create_new_state(&channel_id, new_balance)?,
                proof: self.create_transition_proof(old_balance, new_balance)?,
                timestamp: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
            };

            self.pending_updates.push(transition);
            Ok(())
        } else {
            Err(Error::StakeError("Invalid BOC: no root cell".to_string()))
        }
    }
    fn get_previous_state(
        &self,
        _channel_id: &[u8; 32],
        balance: u64,
    ) -> Result<PrivateChannelState, Error> {
        Ok(PrivateChannelState {
            balance,
            nonce: 0,
            sequence_number: 0,
            proof: [0u8; 64],
            signature: [0u8; 64],
            transaction: [0u8; 32],
            witness: vec![],
            merkle_root: [0u8; 32],
            merkle_proof: vec![],
            last_update: 0,
        })
    }

    fn create_new_state(
        &self,
        _channel_id: &[u8; 32],
        balance: u64,
    ) -> Result<PrivateChannelState, Error> {
        Ok(PrivateChannelState {
            balance,
            nonce: 0,
            sequence_number: 0,
            proof: [0u8; 64],
            signature: [0u8; 64],
            transaction: [0u8; 32],
            witness: vec![],
            merkle_root: [0u8; 32],
            merkle_proof: vec![],
            last_update: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }

    fn create_transition_proof(
        &self,
        old_balance: u64,
        new_balance: u64,
    ) -> Result<ZkProof, Error> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Ok(ZkProof::new(
            vec![0u8; 64],                  // Proof data
            vec![old_balance, new_balance], // Public inputs
            vec![0u8; 32],                  // Merkle root
            timestamp,
        ))
    }
}

#[derive(Debug, Clone, Default)]
pub struct RootStateTracker {
    pub root_history: Vec<WalletRoot>,
}

#[derive(Debug, Clone, Default)]
pub struct WalletRoot {
    pub root_id: [u8; 32],
    pub wallet_merkle_proofs: Vec<[u8; 32]>,
    pub aggregated_balance: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RebalanceConfig {
    pub enabled: bool,
    pub min_imbalance_threshold: u64,
    pub max_rebalance_amount: u64,
    pub target_ratios: HashMap<[u8; 32], f64>,
    pub rebalance_interval: u64,
}

#[derive(Debug, Clone)]
pub struct WalletStorageManager {
    pub encrypted_states: HashMap<[u8; 32], EncryptedWalletState>,
    pub commitment_proofs: HashMap<[u8; 32], ZkProof>,
    pub encryption: EncryptionSystem,
}

impl Default for WalletStorageManager {
    fn default() -> Self {
        Self {
            encrypted_states: HashMap::new(),
            commitment_proofs: HashMap::new(),
            encryption: EncryptionSystem::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct EncryptedWalletState {
    pub encrypted_data: Vec<u8>,
    pub public_commitment: [u8; 32],
    pub proof_of_encryption: ZkProof,
}

#[derive(Debug, Clone, Default)]
pub struct EncryptionSystem {}

impl EncryptionSystem {
    pub fn new() -> Self {
        Self {}
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, Error> {
        // For now, return data unchanged - replace with real encryption
        Ok(data.to_vec())
    }
}