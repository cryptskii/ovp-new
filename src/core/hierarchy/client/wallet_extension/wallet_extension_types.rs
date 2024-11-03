// src/core/hierarchy/client/wallet_extension/wallet_extension_types.rs

use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::types::boc::BOC;
use crate::core::zkps::plonky2::Plonky2System;
use crate::core::zkps::proof::{ProofMetadataJS, ProofType, ZkProof};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
/// Imports the `BuilderData` and `CircuitBuilder` types from the `crate::core::zkps::circuit_builder` module.
///
/// These types are likely used for constructing and managing ZK circuits as part of the wallet extension functionality.
use ton_types::BuilderData;

// Define the Transaction struct

#[derive(Debug, Clone)]
pub struct Transaction {
    pub sender: [u8; 32],
    pub nonce: u64,
    pub sequence_number: u64,
    pub amount: u64,
}

// Define the ChannelClosureRequest struct

#[derive(Debug, Clone)]
pub struct ChannelClosureRequest {
    pub channel_id: [u8; 32],
    pub final_balance: u64,
    pub boc: Vec<u8>,
    pub proof: ZkProof,
    pub signature: [u8; 64],
}

// Define the WalletExtension struct

#[derive(Debug)]
pub struct WalletExtension {
    pub wallet_id: [u8; 32],
    pub channels: HashMap<[u8; 32], Arc<RwLock<Channel>>>,
    pub total_locked_balance: u64,
    pub rebalance_config: RebalanceConfig,
    pub proof_system: Arc<Plonky2System>,
    pub state_tree: SparseMerkleTreeWasm,
    pub encrypted_states: WalletStorageManager,
    pub balance_tracker: WalletBalanceTracker,
    pub root_tracker: RootStateTracker,
}
#[derive(Debug, Clone)]
pub struct Channel {
    pub channel_id: [u8; 32],
    pub wallet_id: [u8; 32],
    pub state: Arc<RwLock<PrivateChannelState>>,
    pub state_history: Vec<StateTransition>,
    pub participants: Vec<[u8; 32]>,
    pub config: ChannelConfig,
    pub spending_limit: u64,
    pub proof_system: Arc<ZkProofSystem>,
    pub(crate) boc_history: Vec<BOC>,
    pub(crate) proof_history: Vec<ZkProof>,
}

// Define the ChannelConfig struct

#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub min_balance: u64,
    pub max_balance: u64,
    pub challenge_period: u64,
    pub max_state_size: usize,
    pub max_participants: usize,
}

// Define the PrivateChannelState struct

#[derive(Debug, Clone)]
pub struct PrivateChannelState {
    pub balance: u64,
    pub sequence_number: u64,
    pub proof: [u8; 64],
    pub signature: [u8; 64],
    pub transaction: [u8; 32],
    pub witness: Vec<[u8; 32]>,
    pub merkle_root: [u8; 32],
    pub merkle_proof: Vec<[u8; 32]>,
    pub last_update: u64,
}

// Define the StateTransition struct

#[derive(Debug, Clone)]
pub struct StateTransition {
    pub old_state: PrivateChannelState,
    pub new_state: PrivateChannelState,
    pub proof: ZkProof,
    pub timestamp: u64,
}

// Define the WalletBalanceTracker struct

#[derive(Debug, Clone)]
pub struct WalletBalanceTracker {
    pub wallet_balances: HashMap<[u8; 32], u64>, // channel_id -> balance
    pub state_transitions: HashMap<[u8; 32], Vec<[u8; 32]>>, // channel_id -> state BOCs
    pub state_tree: Arc<RwLock<SparseMerkleTreeWasm>>, // SMT for state tracking
    last_root: [u8; 32],
    pending_updates: Vec<StateTransition>,
}

// Define the RootStateTracker struct

#[derive(Debug, Clone)]
pub struct RootStateTracker {
    pub root_history: Vec<WalletRoot>,
}

#[derive(Debug, Clone)]
pub struct WalletRoot {
    pub root_id: [u8; 32],
    pub wallet_merkle_proofs: Vec<[u8; 32]>,
    pub aggregated_balance: u64,
}

// Define the RebalanceConfig struct

#[derive(Debug, Clone)]
pub struct RebalanceConfig {
    pub enabled: bool,
    pub min_imbalance_threshold: u64,
    pub max_rebalance_amount: u64,
    pub target_ratios: HashMap<[u8; 32], f64>,
    pub rebalance_interval: u64,
}

// Define the WalletStorageManager struct

#[derive(Debug, Clone)]
pub struct WalletStorageManager {
    pub encrypted_states: HashMap<[u8; 32], EncryptedWalletState>,
    pub commitment_proofs: HashMap<[u8; 32], ZkProof>,
}

#[derive(Debug, Clone)]
pub struct EncryptedWalletState {
    pub encrypted_data: Vec<u8>,
    pub public_commitment: [u8; 32],
    pub proof_of_encryption: ZkProof,
}
