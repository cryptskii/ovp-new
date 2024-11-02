// src/core/types/ovp_ops.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpCode {
    Root(RootOpCode),
    Intermediate(IntermediateOpCode),
    Wallet(WalletOpCode),
    Channel(ChannelOpCode),
    Storage(StorageOpCode),
}

impl OpCode {
    pub fn to_u8(&self) -> u8 {
        match self {
            OpCode::Root(op) => *op as u8,
            OpCode::Intermediate(op) => *op as u8,
            OpCode::Wallet(op) => *op as u8,
            OpCode::Channel(op) => *op as u8,
            OpCode::Storage(op) => *op as u8,
        }
    }

    pub fn from_u8(value: u8) -> Option<OpCode> {
        match value {
            0x01..=0x22 => Some(OpCode::Root(unsafe { std::mem::transmute(value) })),
            0x30..=0x61 => Some(OpCode::Intermediate(unsafe { std::mem::transmute(value) })),
            0x70..=0x91 => Some(OpCode::Wallet(unsafe { std::mem::transmute(value) })),
            0xA0..=0xB1 => Some(OpCode::Channel(unsafe { std::mem::transmute(value) })),
            0xC0..=0xE1 => Some(OpCode::Storage(unsafe { std::mem::transmute(value) })),
            _ => None,
        }
    }
}

// Common operation traits
pub trait Operation {
    fn op_code(&self) -> OpCode;
    fn validate(&self) -> bool;
    fn execute(&self) -> Result<(), String>;
}

// Operation result type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationResult {
    pub success: bool,
    pub op_code: OpCode,
    pub message: Option<String>,
    pub data: Option<Vec<u8>>,
}

/// Channel operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelOpCode {
    // State Operations
    CreatePayment = 0xA0,
    UpdateState = 0xA1,
    FinalizeState = 0xA2,
    DisputeState = 0xA3,

    // Verification
    ValidatePayment = 0xB0,
    ValidateState = 0xB1,
    ValidateFinalState = 0xB2,
    ValidateDispute = 0xB3,
    ValidateSettlement = 0xB4,
    // State Management
    InitChannel = 0xC0,

    // Payment Operations
    ProcessPayment = 0xD1,

    // Settlement
    InitiateSettlement = 0xE0,
    ProcessSettlement = 0xE1,
    FinalizeSettlement = 0xE2,

    // Validation
    ValidateTransition = 0xF0,
    VerifyProof = 0xF1,

    // Channel Management
    LockFunds = 0x10,
    ReleaseFunds = 0x11,
}
/// Root level operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RootOpCode {
    // Epoch Operations
    SubmitEpoch = 0x01,
    ValidateEpoch = 0x02,
    FinalizeEpoch = 0x03,

    // State Operations
    UpdateGlobalRoot = 0x10,
    ValidateGlobalState = 0x11,

    // Intermediate Management
    RegisterIntermediate = 0x20,
    RemoveIntermediate = 0x21,
    ValidateIntermediate = 0x22,
}

/// Storage node operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageOpCode {
    // Battery Operations
    ChargeNode = 0xC0,
    DischargeNode = 0xC1,
    ValidateBattery = 0xC2,

    // Epidemic Protocol
    PropagateState = 0xD0,
    SyncState = 0xD1,
    ValidateSync = 0xD2,

    // Replication
    ReplicateState = 0xE0,
    ValidateReplica = 0xE1,
}

/// Wallet extension operation codes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WalletOpCode {
    // Channel Management
    CreateChannel = 0x70,
    UpdateChannel = 0x71,
    CloseChannel = 0x72,
    ValidateChannel = 0x73,

    // Tree Operations
    UpdateWalletTree = 0x80,
    ValidateWalletTree = 0x81,

    // State Management
    UpdateWalletState = 0x90,
    ValidateWalletState = 0x91,

    ManageChannel = 0xA0,
    GroupChannels = 0xA1,

    // Balance Management
    UpdateBalance = 0xB0,
    ValidateBalance = 0xB1,

    // Transaction Management
    CreateTransaction = 0xC0,
    ValidateTransaction = 0xC1,
    ProcessTransaction = 0xC2,

    // Tree Operations
    GenerateWalletProof = 0xD0,
    VerifyWalletProof = 0xD1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChannelManagerOpCode {
    GetChannel = 0x01,
    CreateChannel = 0x02,
    ManageChannel = 0x03,
    CloseChannel = 0x04,
    GetChannelState = 0x05,
    RebalanceChannels = 0x06,
    GetChannelStateHistory = 0x07,
    ValidateChannelState = 0x08,
    ValidateChannelStateTransition = 0x09,
    VerifyProof = 0x0A,
    LockFunds = 0x0B,
    ReleaseFunds = 0x0C,
}

pub enum IntermediateOpCode {
    // Previous operations...

    // Channel Lifecycle Operations
    RequestChannelOpen,  // Request to open new channel
    ApproveChannelOpen,  // Approve channel opening
    RequestChannelClose, // Request to close channel
    ProcessChannelClose, // Process channel closure

    // Manual Operations (auth required)
    RequestManualRebalance, // Manual rebalance request

    // Rebalancing Operations
    CheckChannelBalances, // Monitor channel balance distribution
    ValidateRebalance,    // Validate rebalancing request/operation
    ExecuteRebalance,     // Execute rebalancing operation

    // Wallet Root State Management
    ReceiveWalletUpdate, // Receive wallet state update
    ProcessWalletRoot,   // Process wallet root state
    ValidateStateUpdate, // Validate wallet state update

    // Storage Management
    StoreWalletState,   // Store state to storage nodes
    VerifyStorageState, // Verify state across storage nodes
    ReplicateState,     // Trigger state replication

    // Root State Submission
    PrepareRootSubmission, // Prepare state for root contract
    SubmitToRoot,          // Submit state to root contract

    // Wallet Management
    RegisterWallet = 0x30,
    UpdateWalletRoot = 0x31,
    ValidateWalletRoot = 0x32,

    // Storage Node Management
    AssignStorageNode = 0x40,
    UpdateStorageNode = 0x41,
    ValidateStorageNode = 0x42,

    // State Management
    UpdateIntermediateState = 0x50,
    ValidateIntermediateState = 0x51,

    // Tree Operations
    UpdateTree = 0x60,
    ValidateTree = 0x61,
}
