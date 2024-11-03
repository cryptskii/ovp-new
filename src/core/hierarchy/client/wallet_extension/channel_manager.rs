use crate::core::types::ovp_ops::ChannelManagerOpCode;
use crate::core::types::ovp_types::{
    Channel, ChannelConfig, ChannelState, RebalanceRequest, SystemError,
};
use crate::core::zkps::zkp::ZkProofSystem;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub struct ChannelManager<StateManager> {
    channels: HashMap<[u8; 32], Arc<RwLock<Channel>>>,
    state_manager: Arc<RwLock<StateManager>>,
    proof_system: Arc<ZkProofSystem>,
    wallet_id: [u8; 32],
    state_history: Vec<ChannelState>,
    spending_limit: u64,
}

impl<StateManager> ChannelManager<StateManager> {
    pub fn new(state_manager: Arc<RwLock<StateManager>>, proof_system: Arc<ZkProofSystem>) -> Self {
        Self {
            channels: HashMap::new(),
            state_manager,
            proof_system,
            wallet_id: [0; 32],
            state_history: Vec::new(),
            spending_limit: 0,
        }
    }

    pub fn new_with_wallet(
        state_manager: Arc<RwLock<StateManager>>,
        proof_system: Arc<ZkProofSystem>,
        wallet_id: [u8; 32],
        spending_limit: u64,
    ) -> Self {
        Self {
            channels: HashMap::new(),
            state_manager,
            proof_system,
            wallet_id,
            state_history: Vec::new(),
            spending_limit,
        }
    }

    pub async fn dispatch(
        &self,
        op_code: ChannelManagerOpCode,
        params: Vec<u8>,
    ) -> Result<Vec<u8>> {
        match op_code {
            ChannelManagerOpCode::GetChannel => {
                // Decode parameters as necessary (e.g., channel_id)
                let channel_id = decode_channel_id(&params)?;
                self.get_channel(channel_id)
                    .map(|channel| serialize_channel(channel))
            }
            ChannelManagerOpCode::CreateChannel => {
                // Extract parameters like sender, recipient, initial balance, config
                let (sender, recipient, initial_balance, config) = decode_create_params(&params)?;
                let channel_id = self.create_channel(sender, recipient, initial_balance, config)?;
                Ok(channel_id.to_vec())
            }
            ChannelManagerOpCode::ManageChannel => {
                // Extract parameters like channel_id and channel_state
                let (channel_id, channel_state) = decode_manage_params(&params)?;
                self.manage_channel(channel_id, channel_state)?;
                Ok(vec![])
            }
            ChannelManagerOpCode::CloseChannel => {
                let channel_id = decode_channel_id(&params)?;
                self.close_channel(channel_id)?;
                Ok(vec![])
            }
            ChannelManagerOpCode::GetChannelState => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(channel_id)?;
                let serialized_channel = serialize_channel(channel)?;
                Ok(serialized_channel)
            }
            ChannelManagerOpCode::RebalanceChannels => {
                let rebalance_requests = decode_rebalance_requests(&params)?;
                self.rebalance_channels(rebalance_requests)?;
                Ok(vec![])
            }
            ChannelManagerOpCode::GetChannelStateHistory => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(channel_id)?;
                let serialized_channel = serialize_channel(channel)?;
                Ok(serialized_channel)
            }
            ChannelManagerOpCode::ValidateChannelState => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(channel_id)?;
                let serialized_channel = serialize_channel(channel)?;
                Ok(serialized_channel)
            }
            ChannelManagerOpCode::ValidateChannelStateTransition => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(channel_id)?;
                let serialized_channel = serialize_channel(channel)?;
                Ok(serialized_channel)
            }
            ChannelManagerOpCode::VerifyProof => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(channel_id)?;
                let proof = decode_proof(&params)?;
                let valid = self.verify_proof(channel, proof)?;
                Ok(valid)
            }
            ChannelManagerOpCode::LockFunds => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(channel_id)?;
                let channel = channel.read().await;
                self.lock_funds(channel)?;
                Ok(vec![])
            }
            ChannelManagerOpCode::ReleaseFunds => {
                let channel_id = decode_channel_id(&params)?;
                let channel = self.get_channel(channel_id)?;
                let channel = channel.read().await;
                self.release_funds(channel)?;
                Ok(vec![])
            }
        }
    }
    fn get_channel(&self, channel_id: [u8; 32]) -> Result<Arc<RwLock<Channel>>, SystemError> {
        self.channels
            .get(&channel_id)
            .cloned()
            .ok_or(SystemError::ChannelNotFound)
    }

    fn create_channel(
        &mut self,
        sender: [u8; 32],
        recipient: [u8; 32],
        initial_balance: u64,
        config: ChannelConfig,
    ) -> Result<[u8; 32], SystemError> {
        let channel_id = self.proof_system.generate_channel_id(sender, recipient);
        let channel = Channel::new(sender, recipient, initial_balance, config);
        self.channels
            .insert(channel_id, Arc::new(RwLock::new(channel)));
        Ok(channel_id)
    }

    fn manage_channel(
        &self,
        channel_id: [u8; 32],
        channel_state: ChannelState,
    ) -> Result<(), SystemError> {
        let channel = self
            .channels
            .get(&channel_id)
            .ok_or(SystemError::ChannelNotFound)?;
        let channel_state = channel.read().unwrap().get_state();
        let state_manager = self.state_manager.read().unwrap();
        let proof = self.proof_system.generate_proof(channel_state);
        state_manager.manage_channel(channel_id, channel_state, proof)?;
        Ok(())
    }
}

// Helper functions for parameter encoding/decoding
fn decode_channel_id(params: &[u8]) -> Result<[u8; 32], SystemError> {
    if params.len() != 32 {
        return Err(SystemError::InvalidParameters);
    }
    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(params);
    Ok(channel_id)
}

fn decode_create_params(
    params: &[u8],
) -> Result<([u8; 32], [u8; 32], u64, ChannelConfig), SystemError> {
    if params.len() != 105 {
        // 32 bytes for sender, 32 bytes for recipient, 8 bytes for initial_balance, 32 bytes for config
        return Err(SystemError::InvalidParameters);
    }

    let mut sender = [0u8; 32];
    sender.copy_from_slice(&params[0..32]);

    let mut recipient = [0u8; 32];
    recipient.copy_from_slice(&params[32..64]);

    let mut initial_balance = [0u8; 8];
    initial_balance.copy_from_slice(&params[64..72]);

    let mut config = [0u8; 32];
    config.copy_from_slice(&params[72..104]);
    Ok((sender, recipient, initial_balance, config))
}
fn decode_manage_params(params: &[u8]) -> Result<([u8; 32], ChannelState), SystemError> {
    if params.len() != 33 {
        // 32 bytes for channel_id + 1 byte for state
        return Err(SystemError::InvalidParameters);
    }

    let mut channel_id = [0u8; 32];
    channel_id.copy_from_slice(&params[0..32]);
    let state = match params[32] {
        0 => ChannelState::Initialized,
        1 => ChannelState::Active,
        2 => ChannelState::Closing,
        3 => ChannelState::Closed,
        _ => return Err(SystemError::InvalidParameters),
    };

    Ok((channel_id, state))
}

fn serialize_channel(channel: Arc<RwLock<Channel>>) -> Vec<u8> {
    let channel = channel.read().unwrap();
    let mut serialized = Vec::new();

    // Serialize sender
    serialized.extend_from_slice(&channel.sender);

    // Serialize recipient
    serialized.extend_from_slice(&channel.recipient);

    // Serialize balance
    serialized.extend_from_slice(&channel.balance.to_be_bytes());

    // Serialize state
    serialized.push(match channel.state {
        ChannelState::Initialized => 0,
        ChannelState::Active => 1,
        ChannelState::Closing => 2,
        ChannelState::Closed => 3,
    });

    // Serialize config
    serialized.extend_from_slice(&channel.config.timeout.to_be_bytes());
    // Add other config serialization as needed

    serialized
}

fn decode_rebalance_requests(params: &[u8]) -> Result<Vec<RebalanceRequest>, SystemError> {
    let rebalance_requests = params
        .chunks(32)
        .map(|chunk| {
            let mut rebalance_request = [0u8; 32];
            rebalance_request.copy_from_slice(chunk);
            Ok(rebalance_request)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rebalance_requests)
}

fn decode_proof(params: &[u8]) -> Result<ZkProof, SystemError> {
    if params.len() != 64 {
        return Err(SystemError::InvalidParameters);
    }

    let mut proof = [0u8; 64];
    proof.copy_from_slice(params);
    Ok(proof)
}
