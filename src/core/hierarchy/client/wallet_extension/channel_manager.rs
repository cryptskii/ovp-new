use crate::core::error::{Error, SystemError, SystemErrorType};
use crate::core::hierarchy::client::channel::channel_contract::ChannelContract;
use crate::core::types::ovp_ops::ChannelOpCode;
use crate::core::zkps::plonky2::Plonky2SystemHandle;
use crate::core::zkps::proof::ZkProof;
use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

type ChannelStore = Arc<RwLock<HashMap<[u8; 32], Arc<RwLock<ChannelContract>>>>>;

#[derive(Debug, Clone)]
pub struct ChannelConfig {
    pub timeout: u64,
    pub min_balance: u64,
    pub max_balance: u64,
}

pub struct ChannelManager {
    channels: ChannelStore,
    proof_system: Arc<Plonky2SystemHandle>,
    wallet_id: [u8; 32],
    spending_limit: u64,
}

impl ChannelManager {
    pub fn new(proof_system: Arc<Plonky2SystemHandle>) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            proof_system,
            wallet_id: [0; 32],
            spending_limit: 0,
        }
    }

    pub fn new_with_wallet(
        proof_system: Arc<Plonky2SystemHandle>,
        wallet_id: [u8; 32],
        spending_limit: u64,
    ) -> Self {
        Self {
            channels: Arc::new(RwLock::new(HashMap::new())),
            proof_system,
            wallet_id,
            spending_limit,
        }
    }

    pub async fn dispatch(&self, op_code: ChannelOpCode, params: Vec<u8>) -> Result<Vec<u8>> {
        match op_code {
            ChannelOpCode::GetChannel => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(&channel_id)?;
                Ok(serialize_channel(&channel))
            }
            ChannelOpCode::InitChannel => {
                let (sender, recipient, initial_balance, config) = decode_create_params(&params)?;
                let channel_id =
                    self.create_channel(sender, recipient, initial_balance, &config)?;
                Ok(channel_id.to_vec())
            }
            ChannelOpCode::UpdateState => {
                let (channel_id, new_state) = decode_update_params(&params)?;
                self.update_channel_state(&channel_id, new_state)?;
                Ok(vec![])
            }
            ChannelOpCode::VerifyProof => {
                let (channel_id, proof, old_balance, new_balance) = decode_verify_params(&params)?;
                let channel = self.get_channel(&channel_id)?;
                let channel = channel.read().map_err(|_| Error::InvalidTransaction)?;
                let is_valid = self.verify_proof(&channel, &proof, old_balance, new_balance)?;
                Ok(vec![is_valid as u8])
            }
            _ => Err(anyhow!("Unsupported operation code: {:?}", op_code)),
        }
    }

    fn get_channel(&self, channel_id: &[u8; 32]) -> Result<Arc<RwLock<ChannelContract>>, Error> {
        self.channels
            .read()
            .map_err(|_| Error::InvalidTransaction)?
            .get(channel_id)
            .cloned()
            .ok_or(Error::ChannelNotFound)
    }

    fn create_channel(
        &self,
        sender: [u8; 32],
        recipient: [u8; 32],
        initial_balance: u64,
        _config: &ChannelConfig,
    ) -> Result<[u8; 32], Error> {
        // Generate channel ID using SHA256
        let mut hasher = Sha256::new();
        hasher.update(sender);
        hasher.update(recipient);
        hasher.update(initial_balance.to_le_bytes());
        let mut channel_id = [0u8; 32];
        channel_id.copy_from_slice(&hasher.finalize());

        // Check if channel already exists
        let mut channels = self
            .channels
            .write()
            .map_err(|_| Error::InvalidTransaction)?;
        if channels.contains_key(&channel_id) {
            return Err(Error::InvalidChannel);
        }

        // Create new channel
        let channel = ChannelContract::new(hex::encode(channel_id));
        let channel = Arc::new(RwLock::new(channel));

        // Initialize channel state
        {
            let mut channel_lock = channel.write().map_err(|_| Error::InvalidTransaction)?;
            channel_lock
                .update_balance(initial_balance)
                .map_err(|_| Error::InvalidAmount)?;
        }

        // Store channel
        channels.insert(channel_id, channel);
        Ok(channel_id)
    }

    fn update_channel_state(&self, channel_id: &[u8; 32], new_state: Vec<u8>) -> Result<(), Error> {
        let channel = self.get_channel(channel_id)?;
        let mut channel = channel.write().map_err(|_| Error::InvalidTransaction)?;

        // Verify channel state transition
        if !self.validate_channel(&channel)? {
            return Err(Error::InvalidChannel);
        }

        // Update channel balance from state data
        if new_state.len() < 8 {
            return Err(Error::InvalidTransaction);
        }

        let mut balance_bytes = [0u8; 8];
        balance_bytes.copy_from_slice(&new_state[0..8]);
        let balance = u64::from_le_bytes(balance_bytes);

        channel
            .update_balance(balance)
            .map_err(|_| Error::InvalidAmount)?;

        Ok(())
    }

    fn validate_channel(&self, channel: &ChannelContract) -> Result<bool, Error> {
        if channel.balance > self.spending_limit {
            return Err(Error::SystemError(SystemError::new(
                SystemErrorType::SpendingLimitExceeded,
                "Channel balance exceeds spending limit".to_string(),
            )));
        }
        Ok(true)
    }

    fn verify_proof(
        &self,
        channel: &ChannelContract,
        _proof: &ZkProof,
        old_balance: u64,
        new_balance: u64,
    ) -> Result<bool, Error> {
        // Validate channel state
        self.validate_channel(channel)?;

        // Convert parameters to proof format
        let mut proof_data = Vec::new();
        proof_data.extend_from_slice(&old_balance.to_le_bytes());
        proof_data.extend_from_slice(&channel.nonce.to_le_bytes());
        proof_data.extend_from_slice(&new_balance.to_le_bytes());
        proof_data.extend_from_slice(&(channel.nonce + 1).to_le_bytes());
        proof_data.extend_from_slice(&new_balance.saturating_sub(old_balance).to_le_bytes());

        // Verify proof
        self.proof_system
            .verify_proof_js(&proof_data)
            .map_err(|_| Error::InvalidProof)
    }
}

fn decode_channel_id(params: &[u8]) -> Result<[u8; 32], Error> {
    if params.len() != 32 {
        return Err(Error::InvalidTransaction);
    }
    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(params);
    Ok(channel_id)
}

fn decode_create_params(params: &[u8]) -> Result<([u8; 32], [u8; 32], u64, ChannelConfig), Error> {
    if params.len() < 80 {
        return Err(Error::InvalidTransaction);
    }

    let mut sender = [0u8; 32];
    sender.copy_from_slice(&params[0..32]);

    let mut recipient = [0u8; 32];
    recipient.copy_from_slice(&params[32..64]);

    let mut balance_bytes = [0u8; 8];
    balance_bytes.copy_from_slice(&params[64..72]);
    let initial_balance = u64::from_le_bytes(balance_bytes);

    let mut timeout_bytes = [0u8; 8];
    timeout_bytes.copy_from_slice(&params[72..80]);
    let timeout = u64::from_le_bytes(timeout_bytes);

    let config = ChannelConfig {
        timeout,
        min_balance: 0,
        max_balance: u64::MAX,
    };

    Ok((sender, recipient, initial_balance, config))
}

fn decode_update_params(params: &[u8]) -> Result<([u8; 32], Vec<u8>), Error> {
    if params.len() <= 32 {
        return Err(Error::InvalidTransaction);
    }

    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(&params[0..32]);
    let new_state = params[32..].to_vec();

    Ok((channel_id, new_state))
}

fn decode_verify_params(params: &[u8]) -> Result<([u8; 32], ZkProof, u64, u64), Error> {
    if params.len() < 32 + 64 + 16 {
        return Err(Error::InvalidProofDataLength);
    }

    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(&params[0..32]);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| Error::InvalidTimestamp)?
        .as_secs();

    let public_inputs = Vec::new();
    let merkle_root = vec![0u8; 32];

    let proof = ZkProof::new(
        params[32..96].to_vec(),
        public_inputs,
        merkle_root,
        timestamp,
    );

    let mut old_balance_bytes = [0u8; 8];
    old_balance_bytes.copy_from_slice(&params[96..104]);
    let old_balance = u64::from_le_bytes(old_balance_bytes);

    let mut new_balance_bytes = [0u8; 8];
    new_balance_bytes.copy_from_slice(&params[104..112]);
    let new_balance = u64::from_le_bytes(new_balance_bytes);

    Ok((channel_id, proof, old_balance, new_balance))
}

fn serialize_channel(channel: &Arc<RwLock<ChannelContract>>) -> Vec<u8> {
    let channel = channel.read().unwrap();
    let mut serialized = Vec::with_capacity(32 + 24);

    if let Ok(id_bytes) = hex::decode(&channel.id) {
        serialized.extend_from_slice(&id_bytes);
    }

    serialized.extend_from_slice(&channel.balance.to_le_bytes());
    serialized.extend_from_slice(&channel.nonce.to_le_bytes());
    serialized.extend_from_slice(&channel.seqno.to_le_bytes());

    serialized
}
