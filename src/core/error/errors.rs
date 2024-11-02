// src/errors/errors.rs

use std::fmt;
use std::io;

#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    SerializationError(String),
    DeserializationError(String),
    InvalidProof,
    UnknownContract,
    InvalidTransaction,
    InvalidSignature,
    InvalidPublicKey,
    InvalidAddress,
    InvalidAmount,
    InvalidChannel,
    InvalidNonce,
    InvalidSequence,
    InvalidTimestamp,
    BatteryError,
    WalletError(String),
    InvalidProofData,
    InvalidProofDataLength,
    InvalidProofDataFormat,
    InvalidProofDataSignature,
    InvalidProofDataPublicKey,
    InvalidProofDataHash,
    ChannelNotFound,
    StorageError(String),
    StakeError(String),
    NetworkError(String),
    ChargingTooFrequent,         // Add this variant
    MaxChargingAttemptsExceeded, // Add this variant
}

#[derive(Debug)]
pub enum CellError {
    DataTooLarge,
    TooManyReferences,
    InvalidData,
    IoError(io::Error),
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CellError::DataTooLarge => write!(f, "Cell data is too large"),
            CellError::TooManyReferences => write!(f, "Too many references in cell"),
            CellError::InvalidData => write!(f, "Invalid cell data"),
            CellError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

impl From<io::Error> for CellError {
    fn from(err: io::Error) -> Self {
        CellError::IoError(err)
    }
}

pub enum ZkProofError {
    InvalidProof,
    InvalidProofData,
    InvalidProofDataLength,
    InvalidProofDataFormat,
    InvalidProofDataSignature,
    InvalidProofDataPublicKey,
    InvalidProofDataHash,
}
impl fmt::Display for ZkProofError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZkProofError::InvalidProof => write!(f, "Invalid proof"),
            ZkProofError::InvalidProofData => write!(f, "Invalid proof data"),
            ZkProofError::InvalidProofDataLength => write!(f, "Invalid proof data length"),
            ZkProofError::InvalidProofDataFormat => write!(f, "Invalid proof data format"),
            ZkProofError::InvalidProofDataSignature => write!(f, "Invalid proof data signature"),
            ZkProofError::InvalidProofDataPublicKey => write!(f, "Invalid proof data public key"),
            ZkProofError::InvalidProofDataHash => write!(f, "Invalid proof data hash"),
        }
    }
}

impl std::error::Error for CellError {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::IoError(err) => write!(f, "IO error: {}", err),
            Error::SerializationError(err) => write!(f, "Serialization error: {}", err),
            Error::DeserializationError(err) => write!(f, "Deserialization error: {}", err),
            Error::InvalidProof => write!(f, "Invalid proof"),
            Error::UnknownContract => write!(f, "Unknown contract"),
            Error::InvalidTransaction => write!(f, "Invalid transaction"),
            Error::InvalidSignature => write!(f, "Invalid signature"),
            Error::InvalidPublicKey => write!(f, "Invalid public key"),
            Error::InvalidAddress => write!(f, "Invalid address"),
            Error::InvalidAmount => write!(f, "Invalid amount"),
            Error::InvalidChannel => write!(f, "Invalid channel"),
            Error::InvalidNonce => write!(f, "Invalid nonce"),
            Error::InvalidSequence => write!(f, "Invalid sequence"),
            Error::InvalidTimestamp => write!(f, "Invalid timestamp"),
            Error::BatteryError => write!(f, "Battery error"),
            Error::WalletError(err) => write!(f, "Wallet error: {}", err),
            Error::InvalidProofData => write!(f, "Invalid proof data"),
            Error::InvalidProofDataLength => write!(f, "Invalid proof data length"),
            Error::InvalidProofDataFormat => write!(f, "Invalid proof data format"),
            Error::InvalidProofDataSignature => write!(f, "Invalid proof data signature"),
            Error::InvalidProofDataPublicKey => write!(f, "Invalid proof data public key"),
            Error::InvalidProofDataHash => write!(f, "Invalid proof data hash"),
            Error::ChannelNotFound => write!(f, "Channel not found"),
            Error::StorageError(err) => write!(f, "Storage error: {}", err),
            Error::StakeError(err) => write!(f, "Stake error: {}", err),
            Error::NetworkError(err) => write!(f, "Network error: {}", err),
            Error::ChargingTooFrequent => write!(f, "Charging too frequent"),
            Error::MaxChargingAttemptsExceeded => write!(f, "Max charging attempts exceeded"),
        }
    }
}

impl std::error::Error for Error {}

impl Error {
    pub fn from_io_error(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Self {
        Error::IoError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::SerializationError(err.to_string())
    }
}

impl From<CellError> for Error {
    fn from(err: CellError) -> Self {
        Error::CellError(err)
    }
}

impl From<ZkProofError> for Error {
    fn from(err: ZkProofError) -> Self {
        Error::ZkProofError(err)
    }
}

#[derive(Debug)]
pub enum BocError {
    TooManyCells,
    NoRoots,
    TotalSizeTooLarge,
    CellDataTooLarge,
    TooManyReferences,
    InvalidReference { from: usize, to: usize },
    InvalidRoot(usize),
    InvalidMerkleProof,
    InvalidPrunedBranch,
    CycleDetected,
    MaxDepthExceeded,
}

impl fmt::Display for BocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BocError::TooManyCells => write!(f, "Too many cells"),
            BocError::NoRoots => write!(f, "No roots"),
            BocError::TotalSizeTooLarge => write!(f, "Total size too large"),
            BocError::CellDataTooLarge => write!(f, "Cell data too large"),
            BocError::TooManyReferences => write!(f, "Too many references"),
            BocError::InvalidReference { from, to } => {
                write!(f, "Invalid reference from {} to {}", from, to)
            }
            BocError::InvalidRoot(index) => write!(f, "Invalid root at index {}", index),
            BocError::InvalidMerkleProof => write!(f, "Invalid Merkle proof"),
            BocError::InvalidPrunedBranch => write!(f, "Invalid pruned branch"),
            BocError::CycleDetected => write!(f, "Cycle detected"),
            BocError::MaxDepthExceeded => write!(f, "Max depth exceeded"),
        }
    }
}

impl From<BocError> for Error {
    fn from(err: BocError) -> Self {
        Error::BocError(err)
    }
}
