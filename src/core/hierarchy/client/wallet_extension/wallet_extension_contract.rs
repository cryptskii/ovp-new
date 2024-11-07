use crate::core::error::errors::Error;
use crate::core::error::SystemError;
use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::types::boc::BOC;
use crate::core::types::ovp_ops::WalletOpCode;
use crate::core:: // Added for proof verification
use crate::core::zkps::plonky2::Plonky2SystemHandle;
use crate::core::zkps::proof::ZkProof;
use js_sys::{Date, Promise, Uint8Array};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
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
    channels: HashMap<[u8; 32], Arc<RwLock<Channel>>>,
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
    pub fn create(
        wallet_id: Uint8Array,
        rebalance_config_js: JsValue,
        _proof_system_js: JsValue,
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
                WalletOpCode::CreateChannel => {
                    let params: CreateChannelParams =
                        bincode::deserialize(&params_vec).map_err(|e| {
                            JsValue::from_str(&format!("Failed to deserialize params: {:?}", e))
                        })?;

                    let channel_id =
                        wallet_extension.create_channel(params).await.map_err(|e| {
                            JsValue::from_str(&format!("Failed to create channel: {:?}", e))
                        })?;
                    Ok(Uint8Array::from(&channel_id[..]).into())
                }

                WalletOpCode::UpdateChannel => {
                    let (channel_id_bytes, update): ([u8; 32], ChannelUpdate) =
                        bincode::deserialize(&params_vec).map_err(|e| {
                            JsValue::from_str(&format!("Failed to deserialize params: {:?}", e))
                        })?;

                    let channel_id = ByteArray32(channel_id_bytes);

                    wallet_extension
                        .update_channel(&channel_id.0, update)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Channel update failed: {:?}", e))
                        })?;

                    Ok(JsValue::null())
                }

                WalletOpCode::UpdateWalletState => {
                    let new_state: WalletStateUpdate =
                        bincode::deserialize(&params_vec).map_err(|e| {
                            JsValue::from_str(&format!("Failed to deserialize new state: {:?}", e))
                        })?;

                    let state_boc = wallet_extension
                        .update_wallet_state(new_state)
                        .await
                        .map_err(|e| JsValue::from_str(&format!("State update failed: {:?}", e)))?;

                    Ok(JsValue::from(Uint8Array::from(
                        &state_boc.serialize().unwrap()[..],
                    )))
                }

                WalletOpCode::UpdateBalance => {
                    let (amount, proof_bytes): (u64, Vec<u8>) = bincode::deserialize(&params_vec)
                        .map_err(|e| {
                        JsValue::from_str(&format!("Failed to deserialize balance update: {:?}", e))
                    })?;
                    let proof = ZkProof::new(proof_bytes, Vec::new(), Vec::new(), 0);
                    let new_state = WalletStateUpdate {
                        old_balance: 0,
                        old_nonce: 0,
                        new_balance: 0,
                        new_nonce: 0,
                        transfer_amount: 0,
                        merkle_root: [0; 32],
                    };
                    wallet_extension
                        .update_balance(amount, proof)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Balance update failed: {:?}", e))
                        })?;

                    Ok(JsValue::null())
                }

                WalletOpCode::CreateTransaction => {
                    let tx_request: TransactionRequest = bincode::deserialize(&params_vec)
                        .map_err(|e| {
                            JsValue::from_str(&format!(
                                "Failed to deserialize transaction request: {:?}",
                                e
                            ))
                        })?;

                    let (tx, proof_bytes) = wallet_extension
                        .create_transaction(tx_request)
                        .await
                        .map_err(|e| {
                        JsValue::from_str(&format!("Transaction creation failed: {:?}", e))
                    })?;
                    let proof = ZkProof::new(proof_bytes, Vec::new(), Vec::new(), 0);
                    let new_state = WalletStateUpdate {
                        old_balance: 0,
                        old_nonce: 0,
                        new_balance: 0,
                        new_nonce: 0,
                        transfer_amount: 0,
                        merkle_root: [0; 32],
                    };

                    wallet_extension
                        .update_wallet_state(new_state)
                        .await
                        .map_err(|e| JsValue::from_str(&format!("State update failed: {:?}", e)))?;

                    Ok(JsValue::from(Uint8Array::from(
                        &tx.serialize().unwrap()[..],
                    )))
                }

                WalletOpCode::GenerateWalletProof => {
                    let proof_request: ProofRequest =
                        bincode::deserialize(&params_vec).map_err(|e| {
                            JsValue::from_str(&format!(
                                "Failed to deserialize proof request: {:?}",
                                e
                            ))
                        })?;
                    // Remove the call to generate_wallet_proof as it's not implemented
                    Ok(JsValue::null())
                }

                WalletOpCode::VerifyWalletProof => {
                    let (proof, statement): (ZkProof, ProofStatement) =
                        bincode::deserialize(&params_vec).map_err(|e| {
                            JsValue::from_str(&format!(
                                "Failed to deserialize proof verification request: {:?}",
                                e
                            ))
                        })?;
                    let is_valid = self.verify_proof(proof, statement).await.map_err(|e| {
                        JsValue::from_str(&format!("Proof verification failed: {:?}", e))
                    })?;

                    Ok(JsValue::from_bool(is_valid))
                }
            }
        })
    }

    async fn create_channel(&mut self, params: CreateChannelParams) -> Result<[u8; 32], Error> {
        let channel_id = generate_channel_id(&self.wallet_id.0, &params.counterparty.0)?;

        let initial_state = PrivateChannelState {
            balance: params.initial_balance,
            nonce: 0,
            sequence_number: 0,
            ..Default::default()
        };

        let state_boc = BOC::default();

        let channel = Channel::new(
            channel_id,
            self.wallet_id.0,
            initial_state,
            vec![self.wallet_id.0, params.counterparty.0],
            params.config,
            params.spending_limit,
            self.proof_system.clone(),
        );

        self.state_tree
            .update(&channel_id, &state_boc.serialize().unwrap())
            .map_err(|_| Error::InvalidTransaction)?;

        self.channels
            .insert(channel_id, Arc::new(RwLock::new(channel)));

        Ok(channel_id)
    }

    async fn update_channel(
        &mut self,
        channel_id: &[u8; 32],
        update: ChannelUpdate,
    ) -> Result<(), Error> {
        let channel = self.get_channel(channel_id)?;
        let mut channel = channel
            .write()
            .map_err(|_| Error::WalletError("Failed to acquire channel write lock".to_string()))?;

        channel.state = update.new_state.clone();

        if let Some(transitions) = self.balance_tracker.state_transitions.get_mut(channel_id) {
            transitions.push(channel.state.merkle_root.to_vec());
        }

        Ok(())
    }

    async fn update_wallet_state(&mut self, new_state: WalletStateUpdate) -> Result<BOC, Error> {
        let mut state_boc = BOC::default();
        let serialized_state = bincode::serialize(&new_state).map_err(|e| {
            Error::SerializationError(format!("Failed to serialize new state: {:?}", e))
        })?;
        state_boc
            .set_data(&serialized_state)
            .map_err(|e| Error::SerializationError(format!("Failed to set data: {:?}", e)))?;
        state_boc.set_merkle_root(new_state.merkle_root);

        let old_root = self
            .state_tree
            .root()
            .map_err(|e| Error::StorageError(format!("Failed to get state root: {:?}", e)))?;
        let old_root_cell = self
            .state_tree
            .get(&old_root)
            .map_err(|e| Error::StorageError(format!("Failed to get state root cell: {:?}", e)))?;
        // Prepare the inputs for the proof
        let old_balance = new_state.old_balance;
        let old_nonce = new_state.old_nonce;
        let new_balance = new_state.new_balance;
        let new_nonce = new_state.new_nonce;
        let transfer_amount = new_state.transfer_amount;
        // Generate the proof using Plonky2SystemHandle
        let proof_bytes = self
            .proof_system
            .generate_proof_js(
                old_balance,
                old_nonce,
                new_balance,
                new_nonce,
                transfer_amount,
            )
            .map_err(|_| {
                Error::ZkProofError(crate::core::error::errors::ZkProofError::InvalidProof)
            })?;
        let proof = ZkProof::new(
            proof_bytes.clone(),
            vec![
                old_balance,
                old_nonce,
                new_balance,
                new_nonce,
                transfer_amount,
            ],
            new_state.merkle_root.to_vec(),
            current_timestamp(),
        );

        state_boc
            .set_proof(&proof_bytes)
            .map_err(|e| Error::SerializationError(format!("Failed to set proof: {:?}", e)))?;

        self.state_tree
            .update(&self.wallet_id.0, &state_boc.serialize().unwrap())
            .map_err(|_| Error::InvalidTransaction)?;

        self.encrypted_states.insert_state(
            self.wallet_id.0,
            &serialized_state,
            new_state.merkle_root,
            proof.clone(),
        )?;

        self.root_tracker.add_root(WalletRoot {
            root_id: new_state.merkle_root,
            wallet_merkle_proofs: vec![],
            aggregated_balance: new_state.new_balance,
        });

        Ok(state_boc)
    }
    async fn create_transaction(
        &mut self,
        request: TransactionRequest,
    ) -> Result<(Transaction, Vec<u8>), Error> {
        let channel = self.get_channel(&request.channel_id.0)?;
        let mut channel_guard = channel
            .write()
            .map_err(|_| Error::WalletError("Failed to acquire channel write lock".to_string()))?;

        let tx = Transaction {
            id: generate_tx_id(),
            channel_id: request.channel_id.0,
            sender: self.wallet_id.0,
            recipient: request.recipient.0,
            amount: request.amount,
            nonce: channel_guard.get_next_nonce(),
            sequence_number: channel_guard.get_next_sequence(),
            timestamp: current_timestamp(),
            status: TransactionStatus::Pending,
            signature: Signature::default(),
            zk_proof: vec![],
            merkle_proof: vec![],
            previous_state: vec![],
            new_state: vec![],
            fee: request.fee,
        };

        let signature = generate_signature(&tx, &self.wallet_id.0)?;
        let mut tx = tx;
        tx.signature = Signature(signature);

        // Generate the proof using Plonky2SystemHandle
        let proof_bytes = self
            .proof_system
            .generate_proof_js(
                channel_guard.state.balance,
                channel_guard.state.nonce,
                channel_guard.state.balance - tx.amount,
                channel_guard.state.nonce + 1,
                tx.amount,
            )
            .map_err(|e| {
                Error::ZkProofError(crate::core::error::errors::ZkProofError::InvalidProof)
            })?;

        // Process the transaction
        channel_guard.process_transaction(tx.clone())?;

        Ok((tx, proof_bytes))
    }

    fn get_channel(&self, channel_id: &[u8; 32]) -> Result<Arc<RwLock<Channel>>, Error> {
        self.channels
            .get(channel_id)
            .cloned()
            .ok_or_else(|| Error::ChannelNotFound)
    }

    async fn update_balance(&mut self, amount: u64, proof: ZkProof) -> Result<(), Error> {
        // Verify the proof using Plonky2SystemHandle
        let is_valid = self
            .proof_system
            .verify_proof_js(&proof.proof_data)
            .map_err(|_| Error::InvalidProof)?;

        if !is_valid {
            return Err(Error::InvalidProof);
        }

        self.total_locked_balance = amount;
        Ok(())
    }
}

// Helper functions
fn generate_channel_id(wallet_id: &[u8; 32], counterparty: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(wallet_id);
    hasher.update(counterparty);
    hasher.update(&current_timestamp().to_le_bytes());

    let result = hasher.finalize();
    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(&result[..32]);
    channel_id
}

fn generate_tx_id() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&current_timestamp().to_le_bytes());
    let result = hasher.finalize();
    let mut tx_id = [0u8; 32];
    tx_id.copy_from_slice(&result[..32]);
    tx_id
}

fn current_timestamp() -> u64 {
    (Date::now() / 1000.0) as u64
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
