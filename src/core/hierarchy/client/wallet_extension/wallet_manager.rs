// ./src/core/hierarchy/client/wallet_extension/wallet_manager.rs
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::*;
use crate::core::types::ovp_ops::*;
use crate::core::zkps::plonky2::Plonky2System;
use crate::core::zkps::proof::{ProofMetadataJS, ProofType, ZkProof};
use crate::core::zkps::zkp_interface::*;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct WalletManager<StateManager> {
    wallets: HashMap<[u8; 32], Arc<RwLock<WalletExtension>>>,
    state_manager: Arc<RwLock<StateManager>>,
    proof_system: Arc<Plonky2System>,
    wallet_id: [u8; 32],
    state_history: Vec<WalletExtension>,
    spending_limit: u64,
}
impl<StateManager> WalletManager<StateManager> {
    pub fn new(state_manager: Arc<RwLock<StateManager>>, proof_system: Arc<Plonky2System>) -> Self {
        Self {
            wallet: HashMap::new(),
            state_manager,
            proof_system,
            wallet_id: [0; 32],
            state_history: Vec::new(),
            spending_limit: 0,
        }
    }

    pub async fn dispatch(&self, op_code: WalletOpCode, params: Vec<u8>) -> Result<Vec<u8>> {
        match op_code {
            WalletOpCode::CreateChannel => {
                // Decode parameters as necessary (e.g., channel_id)
                let channel_id = decode_channel_id(&params)?;
                self.create_channel(channel_id)?;
                Ok(vec![])
            }
