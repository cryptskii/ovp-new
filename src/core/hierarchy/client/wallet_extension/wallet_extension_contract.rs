use crate::core::error::errors::Error;
use crate::core::error::SystemError;
use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::types::boc::BOC;
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::{
    WalletExtension, WalletExtensionState, WalletExtensionStateChange, WalletExtensionStateChangeOp,
};
use crate::core::types::ovp_ops::WalletOpCode;
use crate::core::zkps::plonky2::Plonky2SystemHandle;
use crate::core::zkps::proof::ZkProof;
use js_sys::{Date, Promise, Uint8Array};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;



#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
interface ByteArray32 {
    toArray(): Uint8Array;
}
"#;

#[wasm_bindgen]
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ByteArray32(#[wasm_bindgen(skip)] pub [u8; 32]);

#[wasm_bindgen]
impl ByteArray32 {
    #[wasm_bindgen(constructor)]
    pub fn new(array: &[u8]) -> Result<ByteArray32, JsValue> {
        if array.len() != 32 {
            return Err(JsValue::from_str("Array must be 32 bytes long"));
        }
        let mut result = [0u8; 32];
        result.copy_from_slice(array);
        Ok(ByteArray32(result))
    }

    #[wasm_bindgen(js_name = fromWasmAbi)]
    pub fn from_wasm_abi(val: JsValue) -> Result<ByteArray32, JsValue> {
        let array = js_sys::Uint8Array::new(&val);
        Self::new(&array.to_vec())
    }

    #[wasm_bindgen(js_name = toWasmAbi)]
    pub fn to_wasm_abi(&self) -> JsValue {
        let array = js_sys::Uint8Array::new_with_length(32);
        array.copy_from(&self.0);
        array.into()
    }
}

#[wasm_bindgen]
pub struct WalletExtension {
    wallet_id: ByteArray32,
    channels: HashMap<
        [u8; 32],
        Arc<
            RwLock<
                crate::core::hierarchy::client::wallet_extension::wallet_extension_types::Channel,
            >,
        >,
    >,
    total_locked_balance: u64,
    rebalance_config: RebalanceConfig,
    proof_system: Arc<Plonky2SystemHandle>,
    state_tree: SparseMerkleTreeWasm,
    encrypted_states: WalletStorageManager,
    balance_tracker: WalletBalanceTracker,
    root_tracker: RootStateTracker,
}
#[wasm_bindgen]
impl WalletExtension {
    #[wasm_bindgen(constructor)]
    pub fn new(
        wallet_id: Uint8Array,
        rebalance_config_js: JsValue,
        proof_system_js: JsValue,
    ) -> Result<WalletExtension, JsValue> {
        console_error_panic_hook::set_once();

        let wallet_id_bytes: Vec<u8> = wallet_id.to_vec();
        let wallet_id_array = ByteArray32::new(&wallet_id_bytes)?;

        let rebalance_config: RebalanceConfig = serde_wasm_bindgen::from_value(rebalance_config_js)
            .map_err(|e| {
                JsValue::from_str(&format!("Failed to deserialize RebalanceConfig: {:?}", e))
            })?;

        let proof_system = Arc::new(Plonky2SystemHandle::new().map_err(|e| {
            JsValue::from_str(&format!(
                "Failed to initialize Plonky2SystemHandle: {:?}",
                e
            ))
        })?);

        Ok(WalletExtension {
            wallet_id: wallet_id_array,
            channels: HashMap::new(),
            total_locked_balance: 0,
            rebalance_config,
            proof_system,
            state_tree: SparseMerkleTreeWasm::new(),
            encrypted_states: WalletStorageManager::default(),
            balance_tracker: WalletBalanceTracker::new(),
            root_tracker: RootStateTracker::default(),
        })
    }

    #[wasm_bindgen(js_name = dispatch)]
    pub fn dispatch(&self, op_code: u8, params: Uint8Array) -> Promise {
        let params_vec = params.to_vec();
        let mut wallet_extension = self.clone();

        future_to_promise(async move {  
            let op_code = WalletOpCode::try_from(op_code)
            .map_err(|_| JsValue::from_str("Invalid operation code"))?;
        match op_code {

// Helper functions
fn generate_channel_id(wallet_id: &[u8; 32], counterparty: &[u8; 32]) -> Result<[u8; 32], Error> {
    let mut hasher = Sha256::new();
    hasher.update(wallet_id);
    hasher.update(counterparty);
    hasher.update(&current_timestamp().to_le_bytes());
    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(&hasher.finalize());
    Ok(channel_id)
}

fn current_timestamp() -> u64 {
    (Date::now() / 1000.0) as u64
}

fn generate_tx_id() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&current_timestamp().to_le_bytes());
    let result = hasher.finalize();
    let mut tx_id = [0u8; 32];
    tx_id.copy_from_slice(&result[..32]);
    tx_id
}

fn generate_signature(tx: &Transaction, private_key: &[u8; 32]) -> Result<[u8; 64], Error> {
    // Placeholder implementation for signature generation
    // In practice, use proper cryptographic signing
    let mut hasher = Sha256::new();
    hasher.update(&bincode::serialize(tx).map_err(|e| {
        Error::SerializationError(format!("Failed to serialize transaction: {:?}", e))
    })?);
    hasher.update(private_key);
    let result = hasher.finalize();
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(&result[..32]);
    signature[32..].copy_from_slice(&result[..32]); // Simplified for example
    Ok(signature)
}

// Type definitions and implementations
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
    // Add other necessary fields
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

fn get_next_nonce(state: &PrivateChannelState) -> u64 {
    state.nonce + 1
}

fn get_next_sequence(state: &PrivateChannelState) -> u64 {
    state.sequence_number + 1
}

fn process_transaction(state: &mut PrivateChannelState, tx: Transaction) -> Result<(), Error> {
    // Simplified transaction processing

    state.nonce = tx.nonce;
    state.sequence_number = tx.sequence_number;
    state.balance = state
        .balance
        .checked_sub(tx.amount)
        .ok_or_else(|| Error::InvalidAmount)?;
    Ok(())
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PrivateChannelState {
    pub balance: u64,
    pub nonce: u64,
    pub sequence_number: u64,
    pub merkle_root: [u8; 32],
    // Add other necessary fields
}

#[derive(Serialize, Deserialize, Clone, Default)]
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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WalletStorageManager {
    pub node_id: [u8; 32],
    pub stake: u64,
    pub stored_bocs: HashMap<[u8; 32], BOC>,
    pub stored_proofs: HashMap<[u8; 32], ZkProof>,
    pub config: StorageNodeConfig,
    pub intermediate_trees: HashMap<u64, RootTree>,
    pub root_trees: HashMap<u64, RootTree>,
    pub peers: HashSet<[u8; 32]>,
}

impl WalletStorageManager {
    pub fn default() -> Self {
        Self {
            node_id: [0u8; 32],
            stake: 0,
            stored_bocs: HashMap::new(),
            stored_proofs: HashMap::new(),
            config: StorageNodeConfig {
                battery_config: BatteryConfig,
                sync_config: SyncConfig,
                epidemic_protocol_config: EpidemicProtocolConfig {
                    redundancy_factor: 0,
                    propagation_probability: 0.0,
                },
                network_config: NetworkConfig,
            },
            intermediate_trees: HashMap::new(),
            root_trees: HashMap::new(),
            peers: HashSet::new(),
        }
    }

    pub fn insert_state(
        &mut self,
        _wallet_id: [u8; 32],
        _state_data: &[u8],
        _merkle_root: [u8; 32],
        _proof: ZkProof,
    ) -> Result<(), Error> {
        // Implement state insertion logic
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct WalletBalanceTracker {
    pub wallet_balances: HashMap<[u8; 32], u64>,
    pub state_transitions: HashMap<[u8; 32], Vec<Vec<u8>>>,
}

impl WalletBalanceTracker {
    pub fn new() -> Self {
        Self {
            wallet_balances: HashMap::new(),
            state_transitions: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct RootStateTracker {
    pub roots: HashMap<[u8; 32], WalletRoot>,
    pub state_updates: HashMap<[u8; 32], Vec<Vec<u8>>>,
    pub last_update_timestamp: u64,
    pub total_roots: u64,
}

impl RootStateTracker {
    pub fn default() -> Self {
        Self {
            roots: todo!(),
            state_updates: todo!(),
            last_update_timestamp: todo!(),
            total_roots: todo!(),
            // Initialize fields
        }
    }

    pub fn add_root(&mut self, root: WalletRoot) {
        self.roots.insert(root.root_id, root.clone());
        self.state_updates
            .entry(root.root_id)
            .or_insert_with(Vec::new)
            .push(root.wallet_merkle_proofs);
        self.last_update_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.total_roots += 1;
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WalletRoot {
    pub root_id: [u8; 32],
    pub wallet_merkle_proofs: Vec<u8>,
    pub aggregated_balance: u64,
}

impl Clone for WalletExtension {
    fn clone(&self) -> Self {
        WalletExtension {
            wallet_id: self.wallet_id,
            channels: self.channels.clone(),
            total_locked_balance: self.total_locked_balance,
            rebalance_config: self.rebalance_config.clone(),
            proof_system: self.proof_system.clone(),
            state_tree: self.state_tree.clone(),
            encrypted_states: self.encrypted_states.clone(),
            balance_tracker: self.balance_tracker.clone(),
            root_tracker: self.root_tracker.clone(),
        }
    }
}

// Add serde support for [u8; 64]
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Signature(#[serde(with = "serde_arrays")] pub [u8; 64]);

impl Default for Signature {
    fn default() -> Self {
        Signature([0u8; 64])
    }
}

impl AsRef<[u8]> for ByteArray32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// Add From<SystemError> for JsValue
impl From<SystemError> for JsValue {
    fn from(error: SystemError) -> Self {
        JsValue::from_str(&format!("System error: {:?}", error))
    }
}
