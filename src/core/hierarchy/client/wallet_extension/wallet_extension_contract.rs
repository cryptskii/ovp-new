// src/core/hierarchy/client/wallet_extension/wallet_extension.rs

use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::*;
use crate::core::types::ovp_ops::WalletOpCode;
use crate::core::zkps::circuit_builder::Circuit;
use js_sys::{Date, Promise, Uint8Array};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
interface ByteArray32 {
  toArray(): Uint8Array;
}
"#;

// 1. Fix ByteArray32 implementation
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
    proof_system: Arc<ZkProofSystem>,
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

        let rebalance_config: RebalanceConfig =
            serde_wasm_bindgen::from_value(rebalance_config_js)?;
        let proof_system: Arc<ZkProofSystem> =
            Arc::new(serde_wasm_bindgen::from_value(proof_system_js)?);

        Ok(WalletExtension {
            wallet_id: wallet_id_array,
            channels: HashMap::new(),
            total_locked_balance: 0,
            rebalance_config,
            proof_system,
            state_tree: SparseMerkleTreeWasm::new(),
            encrypted_states: WalletStorageManager::new(),
            balance_tracker: WalletBalanceTracker {
                wallet_balances: HashMap::new(),
                state_transitions: HashMap::new(),
            },
            root_tracker: RootStateTracker {
                root_history: Vec::new(),
            },
        })
    }

    #[wasm_bindgen]
    pub fn dispatch(&mut self, op_code: u8, params: Uint8Array) -> Promise {
        let params_vec = params.to_vec();
        let wallet_extension = self.clone();

        future_to_promise(async move {
            let op_code = WalletOpCode::try_from(op_code)
                .map_err(|_| JsValue::from_str("Invalid operation code"))?;

            match op_code {
                WalletOpCode::CreateChannel => {
                    let params = deserialize_create_channel_params(&params_vec)?;
                    let channel_id = wallet_extension.create_channel(params).await?;
                    Ok(Uint8Array::from(&channel_id[..]).into())
                }

                WalletOpCode::UpdateChannel => {
                    let (channel_id, update) = deserialize_update_params(&params_vec)?;
                    wallet_extension
                        .update_channel(&channel_id, update)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Channel update failed: {:?}", e))
                        })?;
                    Ok(JsValue::null())
                }

                WalletOpCode::GroupChannels => {
                    let group_config = deserialize_group_config(&params_vec)?;
                    wallet_extension
                        .group_channels(group_config)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Channel grouping failed: {:?}", e))
                        })?;
                    Ok(JsValue::null())
                }

                WalletOpCode::UpdateWalletState => {
                    let new_state = deserialize_wallet_state(&params_vec)?;
                    let state_boc = wallet_extension
                        .update_wallet_state(&new_state)
                        .await
                        .map_err(|e| JsValue::from_str(&format!("State update failed: {:?}", e)))?;
                }

                WalletOpCode::UpdateBalance => {
                    let (amount, proof) = deserialize_balance_update(&params_vec)?;
                    wallet_extension
                        .update_balance(amount, proof)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Balance update failed: {:?}", e))
                        })?;
                    Ok(JsValue::null())
                }

                WalletOpCode::CreateTransaction => {
                    let tx_request = deserialize_tx_request(&params_vec)?;
                    let tx_boc = wallet_extension
                        .create_transaction(tx_request)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Transaction creation failed: {:?}", e))
                        })?;
                }
                WalletOpCode::GenerateWalletProof => {
                    let request = deserialize_proof_request(&params_vec)?;
                    let (proof, proof_boc) = wallet_extension
                        .generate_wallet_proof(request)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Proof generation failed: {:?}", e))
                        })?;
                    Ok(Uint8Array::from(&proof.serialize()?[..]).into())
                }
                WalletOpCode::VerifyWalletProof => {
                    let request = deserialize_proof_request(&params_vec)?;
                    let is_valid = wallet_extension
                        .verify_wallet_proof(request)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Proof verification failed: {:?}", e))
                        })?;
                    Ok(JsValue::from_bool(is_valid))
                }
                WalletOpCode::GenerateChannelProof => {
                    let request = deserialize_proof_request(&params_vec)?;
                    let (proof, proof_boc) = wallet_extension
                        .generate_channel_proof(request)
                        .await
                        .map_err(|e| {
                            JsValue::from_str(&format!("Proof generation failed: {:?}", e))
                        })?;
                    Ok(Uint8Array::from(&proof.serialize()?[..]).into())
                }

                _ => Err(JsValue::from_str("Invalid operation")),
            }
        })
    }
    async fn create_channel(
        &mut self,
        params: CreateChannelParams,
    ) -> Result<[u8; 32], SystemError> {
        let channel_id = generate_channel_id(&self.wallet_id.0, &params.counterparty.0);

        let initial_state = PrivateChannelState {
            balance: params.initial_balance,
            nonce: 0,
            sequence_number: 0,
            proof: [0u8; 64],
            signature: [0u8; 64],
            transaction: [0u8; 32],
            witness: Vec::new(),
            merkle_root: [0u8; 32],
            merkle_proof: Vec::new(),
            last_update: current_timestamp(),
        };

        let mut state_boc = BOC::default();

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
            .update(&channel_id, &state_boc.serialize(self.proof_system)?)?;

        self.channels
            .insert(channel_id, Arc::new(RwLock::new(channel)));
        self.total_locked_balance += params.initial_balance;
        self.balance_tracker
            .wallet_balances
            .insert(channel_id, params.initial_balance);
        self.balance_tracker
            .state_transitions
            .insert(channel_id, vec![]);

        Ok(channel_id)
    }

    async fn update_channel(
        &mut self,
        channel_id: &[u8; 32],
        update: ChannelUpdate,
    ) -> Result<(), SystemError> {
        let channel = self.get_channel(channel_id)?;
        let mut channel = channel.write().map_err(|_| SystemError {
            error_type: SystemErrorType::ChannelError,
            id: [0u8; 32],
            data: vec![],
            error_data: SystemErrorData {
                id: [0u8; 32],
                data: vec![],
            },
            error_data_id: [0u8; 32],
        })?;

        let mut state_boc = BOC::default();

        *channel.state.write().map_err(|_| SystemError {
            error_type: SystemErrorType::ChannelError,
            id: [0u8; 32],
            data: vec![],
            error_data: SystemErrorData {
                id: [0u8; 32],
                data: vec![],
            },
            error_data_id: [0u8; 32],
        })? = update.new_state;

        self.balance_tracker
            .wallet_balances
            .insert(*channel_id, update.balance);
        if let Some(transitions) = self.balance_tracker.state_transitions.get_mut(channel_id) {
            transitions.push(state_boc.merkle_root.unwrap());
        }

        Ok(())
    }

    async fn update_wallet_state(
        &mut self,
        new_state: WalletStateUpdate,
    ) -> Result<BOC, SystemError> {
        let mut state_boc = BOC::default();
        state_boc.state_data = Some(new_state.serialize()?);
        state_boc.merkle_root = Some(new_state.merkle_root);

        let proof = self
            .proof_system
            .generate_proof(Circuit::StateTransition, new_state.serialize()?)?;
        state_boc.state_proof = Some(proof.serialize()?);

        self.state_tree
            .update(&self.wallet_id.0, &state_boc.serialize(self.proof_system)?)?;

        self.encrypted_states.encrypted_states.insert(
            self.wallet_id.0,
            EncryptedWalletState {
                encrypted_data: self
                    .encrypted_states
                    .encryption
                    .encrypt(&new_state.serialize()?)?,
                public_commitment: new_state.merkle_root,
                proof_of_encryption: proof,
            },
        );

        self.root_tracker.root_history.push(WalletRoot {
            root_id: new_state.merkle_root,
            wallet_merkle_proofs: vec![],
            aggregated_balance: new_state.new_balance,
        });

        Ok(state_boc)
    }
    pub async fn create_transaction(
        &mut self,
        request: TransactionRequest,
    ) -> Result<(Transaction, ZkProof), SystemError> {
        let channel = self.get_channel(&request.channel_id.0)?;

        let tx = Transaction {
            id: generate_tx_id(),
            channel_id: request.channel_id.0,
            sender: self.wallet_id.0,
            recipient: request.recipient.0,
            amount: request.amount,
            nonce: channel
                .read()
                .map_err(|_| SystemError {
                    error_type: SystemErrorType::ChannelError,
                    id: [0u8; 32],
                    data: vec![],
                    error_data: SystemErrorData {
                        id: [0u8; 32],
                        data: vec![],
                    },
                    error_data_id: [0u8; 32],
                })?
                .get_next_nonce()?,
            sequence_number: channel
                .read()
                .map_err(|_| SystemError {
                    error_type: SystemErrorType::ChannelError,
                    id: [0u8; 32],
                    data: vec![],
                    error_data: SystemErrorData {
                        id: [0u8; 32],
                        data: vec![],
                    },
                    error_data_id: [0u8; 32],
                })?
                .get_next_sequence()?,
            timestamp: current_timestamp(),
            status: TransactionStatus::Pending,
            signature: [0u8; 64],
            zk_proof: vec![],
            merkle_proof: vec![],
            previous_state: vec![],
            new_state: vec![],
            fee: request.fee,
        };

        let signature = generate_signature(&tx, &self.wallet_id.0)?;
        let mut tx = tx;
        tx.signature = signature;

        let mut channel = channel.write().map_err(|_| SystemError {
            error_type: SystemErrorType::ChannelError,
            id: [0u8; 32],
            data: vec![],
            error_data: SystemErrorData {
                id: [0u8; 32],
                data: vec![],
            },
            error_data_id: [0u8; 32],
        })?;

        channel.process_transaction(tx.clone())?;
    }

    fn get_channel(&self, channel_id: &[u8; 32]) -> Result<Arc<RwLock<Channel>>, SystemError> {
        self.channels.get(channel_id).cloned().ok_or(SystemError {
            error_type: SystemErrorType::ChannelNotFound,
            id: [0u8; 32],
            data: vec![],
            error_data: SystemErrorData {
                id: [0u8; 32],
                data: vec![],
            },
            error_data_id: [0u8; 32],
        })
    }

    async fn group_channels(&mut self, config: GroupConfig) -> Result<(), SystemError> {
        for channel_id in config.channel_ids.iter() {
            if !self.channels.contains_key(&channel_id.0) {
                return Err(SystemError {
                    error_type: SystemErrorType::ChannelNotFound,
                    id: [0u8; 32],
                    data: vec![],
                    error_data: SystemErrorData {
                        id: [0u8; 32],
                        data: vec![],
                    },
                    error_data_id: [0u8; 32],
                });
            }
        }

        // Implementation for grouping channels
        Ok(())
    }

    async fn update_balance(&mut self, amount: u64, proof: ZkProof) -> Result<(), SystemError> {
        {
            let proof_system = self.proof_system.as_ref();
            proof_system.verify_proof(&proof)?;
        }
        self.total_locked_balance = amount;
        Ok(())
    }

    async fn generate_wallet_proof(&self, request: ProofRequest) -> Result<ZkProof, SystemError> {
        let proof_system = self.proof_system.as_ref();
        proof_system.generate_proof(request.proof_type, request.data)
    }
}

// Helper functions
fn generate_channel_id(wallet_id: &[u8; 32], counterparty: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(wallet_id);
    hasher.update(counterparty);
    hasher.update(&current_timestamp().to_le_bytes());

    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(&hasher.finalize());
    channel_id
}

fn generate_tx_id() -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(&current_timestamp().to_le_bytes());
    let mut tx_id = [0u8; 32];
    tx_id.copy_from_slice(&hasher.finalize());
    tx_id
}

fn current_timestamp() -> u64 {
    (Date::now() / 1000.0) as u64
}

fn generate_signature(tx: &Transaction, private_key: &[u8; 32]) -> Result<[u8; 64], SystemError> {
    // Implementation for generating transaction signature
    let mut signature = [0u8; 64];
    // Signature generation logic here
    Ok(signature)
}

// Serialization helper functions with WASM bindings
#[wasm_bindgen]
pub fn deserialize_create_channel_params(params: &[u8]) -> Result<CreateChannelParams, JsValue> {
    bincode::deserialize(params).map_err(|e| {
        JsValue::from_str(&format!(
            "Failed to deserialize create channel params: {:?}",
            e
        ))
    })
}

#[wasm_bindgen]
pub fn deserialize_update_params(params: &[u8]) -> Result<([u8; 32], ChannelUpdate), JsValue> {
    bincode::deserialize(params)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize update params: {:?}", e)))
}

#[wasm_bindgen]
pub fn deserialize_group_config(params: &[u8]) -> Result<GroupConfig, JsValue> {
    bincode::deserialize(params)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize group config: {:?}", e)))
}

#[wasm_bindgen]
pub fn deserialize_wallet_state(params: &[u8]) -> Result<WalletStateUpdate, JsValue> {
    bincode::deserialize(params)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize wallet state: {:?}", e)))
}

#[wasm_bindgen]
pub fn deserialize_balance_update(params: &[u8]) -> Result<(u64, ZkProof), JsValue> {
    bincode::deserialize(params)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize balance update: {:?}", e)))
}

#[wasm_bindgen]
pub fn deserialize_tx_request(params: &[u8]) -> Result<TransactionRequest, JsValue> {
    bincode::deserialize(params).map_err(|e| {
        JsValue::from_str(&format!(
            "Failed to deserialize transaction request: {:?}",
            e
        ))
    })
}

#[wasm_bindgen]
pub fn deserialize_proof_request(params: &[u8]) -> Result<ProofRequest, JsValue> {
    bincode::deserialize(params)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize proof request: {:?}", e)))
}

// Modified structs for proper WASM binding
#[derive(Serialize, Deserialize)]
pub struct CreateChannelParams {
    pub counterparty: ByteArray32,
    pub initial_balance: u64,
    pub config: ChannelConfig,
    pub spending_limit: u64,
}

#[derive(Serialize, Deserialize)]
pub struct TransactionRequest {
    pub channel_id: ByteArray32,
    pub recipient: ByteArray32,
    pub amount: u64,
    pub fee: u64,
}

#[derive(Serialize, Deserialize)]
pub struct GroupConfig {
    pub group_name: String,
    pub channel_ids: Vec<ByteArray32>,
}

#[derive(Serialize, Deserialize)]
pub struct ProofRequest {
    pub proof_type: ProofType,
    pub data: Vec<u8>,
}

// Remove wasm_bindgen from trait implementation
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

impl From<SystemError> for JsValue {
    fn from(error: SystemError) -> Self {
        JsValue::from_str(&format!("System error: {:?}", error))
    }
}

// Helper functions for ByteArray32
impl AsRef<[u8]> for ByteArray32 {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}
