use crate::core::error::errors::Error;
use crate::core::hierarchy::client::channel::channel_contract::ChannelContract;
use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::hierarchy::client::wallet_extension::wallet_extension_contract::ByteArray32;
use crate::core::storage_node::storage_node_contract::StorageNode;
use crate::core::types::boc::{Cell, CellType, BOC};
use crate::core::types::WalletExtensionStateChangeOp;
use crate::core::zkps::plonky2::Plonky2SystemHandle;
use crate::core::zkps::proof::ZkProof;
use aes_gcm::aead::{Aead, Payload};
use aes_gcm::KeyInit;
use ed25519_dalek::Signature;
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

#[derive(Serialize, Deserialize, Clone)]
pub struct CreateChannelParams {
    pub counterparty: ByteArray32,
    pub initial_balance: u64,
    pub config: ChannelConfig,
    pub spending_limit: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChannelUpdate {
    pub new_state: PrivateChannelState,
    pub balance: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WalletStateUpdate {
    pub old_balance: u64,
    pub old_nonce: u64,
    pub new_balance: u64,
    pub new_nonce: u64,
    pub transfer_amount: u64,
    pub merkle_root: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PrivateChannelState {
    pub balance: u64,
    pub nonce: u64,
    pub sequence_number: u64,
    pub merkle_root: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct RebalanceConfig {
    pub min_balance: u64,
    pub max_balance: u64,
    pub rebalance_threshold: u64,
    pub auto_rebalance: bool,
    pub rebalance_interval: u64,
    pub last_rebalance_timestamp: u64,
    pub target_balance: u64,
    pub allowed_deviation: u64,
    pub emergency_threshold: u64,
    pub max_rebalance_attempts: u32,
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct ChannelConfig {
    pub channel_id: [u8; 32],
    pub capacity: u64,
    pub min_deposit: u64,
    pub max_deposit: u64,
    pub timeout_period: u64,
    pub fee_rate: u64,
    pub is_active: bool,
    pub participants: Vec<[u8; 32]>,
    pub creation_timestamp: u64,
    pub last_update_timestamp: u64,
    pub settlement_delay: u64,
    pub dispute_window: u64,
    pub max_participants: u32,
    pub channel_type: u8,
    pub security_deposit: u64,
    pub auto_close_threshold: u64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TransactionRequest {
    pub channel_id: ByteArray32,
    pub recipient: ByteArray32,
    pub amount: u64,
    pub fee: u64,
}

#[derive(Serialize, Deserialize, Clone)]
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
    pub signature: Signature,
    pub zk_proof: Vec<u8>,
    pub merkle_proof: Vec<u8>,
    pub previous_state: Vec<u8>,
    pub new_state: Vec<u8>,
    pub fee: u64,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

impl Transaction {
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        bincode::serialize(self).map_err(|e| {
            Error::SerializationError(format!("Failed to serialize transaction: {:?}", e))
        })
    }
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
    pub encrypted_states: StorageNode<ByteArray32, Vec<u8>>,
    pub balance_tracker: WalletBalanceTracker,
    pub root_tracker: RootStateTracker<[u8; 32]>,
}

impl fmt::Debug for WalletExtension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalletExtension")
            .field("wallet_id", &self.wallet_id)
            .field("total_locked_balance", &self.total_locked_balance)
            .field("rebalance_config", &self.rebalance_config)
            .field("state_tree", &self.state_tree)
            .field("balance_tracker", &self.balance_tracker)
            .field("root_tracker", &self.root_tracker)
            .finish()
    }
}

pub struct WalletExtensionStateChange {
    pub op: WalletExtensionStateChangeOp,
    pub channel_id: [u8; 32],
    pub wallet_id: [u8; 32],
    pub state: WalletExtension,
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
            proof: ZkProof::new(Vec::new(), Vec::new(), Vec::new(), 0),
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
        self.wallet_balances.insert(channel_id, new_balance);

        let mut hasher = sha2::Sha256::new();
        if let Some(root_cell) = boc.get_root_cell() {
            hasher.update(&root_cell.data);
            let mut root_hash = [0u8; 32];
            root_hash.copy_from_slice(&hasher.finalize());

            let transitions = self
                .state_transitions
                .entry(channel_id)
                .or_insert_with(Vec::new);
            transitions.push(root_hash);

            let mut state_tree = self
                .state_tree
                .write()
                .map_err(|_| Error::StakeError("Lock acquisition failed".to_string()))?;

            let state_cell = Cell::new(
                root_cell.data.clone(),
                root_cell.references.clone(),
                CellType::Ordinary,
                root_hash,
                None,
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

            let transition = StateTransition {
                old_state: self.get_previous_state(&channel_id, old_balance)?,
                new_state: PrivateChannelState::default(),
                proof: ZkProof::new(Vec::new(), Vec::new(), Vec::new(), 0),
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
        channel_id: &[u8; 32],
        balance: u64,
    ) -> Result<PrivateChannelState, Error> {
        let state_transitions = self
            .state_transitions
            .get(channel_id)
            .ok_or(Error::StakeError("No state transitions".to_string()))?;
        let state_tree = self
            .state_tree
            .read()
            .map_err(|_| Error::StakeError("Lock acquisition failed".to_string()))?;
        let state_boc = state_tree
            .get(channel_id)
            .map_err(|_| Error::StakeError("State tree lookup failed".to_string()))?;
        let state_boc = BOC::deserialize(&state_boc)
            .map_err(|_| Error::StakeError("State tree deserialization failed".to_string()))?;
        let state_cell = state_boc
            .get_cell(0)
            .ok_or(Error::StakeError("No state cell".to_string()))?;
        let state = PrivateChannelState::deserialize(state_cell.data.as_slice())
            .map_err(|_| Error::StakeError("State deserialization failed".to_string()))?;

        Ok(state)
    }
}

#[derive(Debug, Clone, Default)]
pub struct RootStateTracker<WalletRoot> {
    pub root_history: Vec<WalletRoot>,
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
        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key)
            .map_err(|_| Error::StakeError("Failed to generate random key".to_string()))?;

        let cipher = aes_gcm::Aes256Gcm::new_from_slice(&key)
            .map_err(|_| Error::StakeError("Failed to create cipher".to_string()))?;
        let nonce = aes_gcm::Nonce::from_slice(&[0u8; 12]);

        let ciphertext = cipher
            .encrypt(nonce, Payload::from(data))
            .map_err(|_| Error::StakeError("Encryption failed".to_string()))?;

        let mut encrypted = Vec::with_capacity(key.len() + ciphertext.len());
        encrypted.extend_from_slice(&key);
        encrypted.extend(ciphertext);

        Ok(encrypted)
    }
}
