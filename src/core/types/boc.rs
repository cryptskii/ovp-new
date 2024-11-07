use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io;

// Error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemErrorType {
    InvalidTransaction,
    InvalidNonce,
    InvalidSequence,
    InvalidAmount,
    InsufficientBalance,
    SpendingLimitExceeded,
    NoRootCell,
    InvalidOperation,
}

impl fmt::Display for SystemErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransaction => write!(f, "Invalid transaction"),
            Self::InvalidNonce => write!(f, "Invalid nonce"),
            Self::InvalidSequence => write!(f, "Invalid sequence"),
            Self::InvalidAmount => write!(f, "Invalid amount"),
            Self::InsufficientBalance => write!(f, "Insufficient balance"),
            Self::SpendingLimitExceeded => write!(f, "Spending limit exceeded"),
            Self::NoRootCell => write!(f, "No root cell"),
            Self::InvalidOperation => write!(f, "Invalid operation"),
        }
    }
}

// root_cell struct
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RootCell {
    pub capacity: u64,
    pub lock_script: Script,
    pub type_script: Script,
    pub data: Vec<u8>,
}

// script struct
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Script {
    pub code_hash: H256,
    pub hash_type: ScriptHashType,
    pub args: Vec<u8>,
    // pub hash_type: ScriptHashType,
}

// script_hash_type enum
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ScriptHashType {
    Data,
    Type,
}

// h256 struct
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct H256(pub [u8; 32]);

pub fn repr_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}
// Hash

pub fn hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}

// hash_pair
pub fn hash_pair(left: &[u8], right: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(left);
    hasher.update(right);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}

// hash_value
pub fn hash_value(value: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(value);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}

// root_hash
pub fn root_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}

// root cell hash
pub fn root_cell_hash(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cell {
    pub data: Vec<u8>,
    pub references: Vec<usize>,
    pub cell_type: CellType,
    pub merkle_hash: [u8; 32],
    pub proof: Option<Vec<u8>>, // Added field for proof data
}

#[derive(Debug, Clone)]
pub struct SystemError {
    pub error_type: SystemErrorType,
    pub message: String,
}

impl SystemError {
    pub fn new(error_type: SystemErrorType, message: String) -> Self {
        Self {
            error_type,
            message,
        }
    }

    pub fn error_type(&self) -> SystemErrorType {
        self.error_type
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SystemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.error_type, self.message)
    }
}

impl std::error::Error for SystemError {}

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

impl std::error::Error for CellError {}

impl From<io::Error> for CellError {
    fn from(err: io::Error) -> Self {
        CellError::IoError(err)
    }
}

#[derive(Debug)]
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

impl std::error::Error for ZkProofError {}

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

impl std::error::Error for BocError {}

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
    ChargingTooFrequent,
    MaxChargingAttemptsExceeded,
    CellError(CellError),
    ZkProofError(ZkProofError),
    BocError(BocError),
    SystemError(SystemError),
}

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
            Error::CellError(err) => write!(f, "Cell error: {}", err),
            Error::ZkProofError(err) => write!(f, "ZK proof error: {}", err),
            Error::BocError(err) => write!(f, "BOC error: {}", err),
            Error::SystemError(err) => write!(f, "System error: {}", err),
        }
    }
}

impl std::error::Error for Error {}

// From implementations
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

impl From<BocError> for Error {
    fn from(err: BocError) -> Self {
        Error::BocError(err)
    }
}

impl From<SystemError> for Error {
    fn from(err: SystemError) -> Self {
        Error::SystemError(err)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellType {
    Ordinary,
    PrunedBranch,
    LibraryReference,
    MerkleProof,
    MerkleUpdate,
}

impl Cell {
    /// Creates a new `Cell` with all fields specified.
    pub fn new(
        data: Vec<u8>,
        references: Vec<usize>,
        cell_type: CellType,
        merkle_hash: [u8; 32],
        proof: Option<Vec<u8>>,
    ) -> Self {
        Self {
            data,
            references,
            cell_type,
            merkle_hash,
            proof,
        }
    }

    /// Creates a new `Cell` with only data specified. Other fields are set to default values.
    pub fn with_data(data: Vec<u8>) -> Self {
        Self {
            data,
            references: Vec::new(),
            cell_type: CellType::Ordinary,
            merkle_hash: [0u8; 32],
            proof: None,
        }
    }

    /// Creates a new `Cell` from data.
    pub fn from_data(data: Vec<u8>) -> Self {
        Self::with_data(data)
    }

    /// Returns a reference to the cell's data.
    pub fn get_data(&self) -> &Vec<u8> {
        &self.data
    }

    /// Calculates and updates the Merkle hash of the cell.
    pub fn update_merkle_hash(&mut self) {
        let mut hasher = Sha256::new();
        hasher.update(&self.data);
        for &ref_idx in &self.references {
            hasher.update(ref_idx.to_le_bytes());
        }
        self.merkle_hash = hasher.finalize().into();
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            references: Vec::new(),
            cell_type: CellType::Ordinary,
            merkle_hash: [0u8; 32],
            proof: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BOC {
    pub cells: Vec<Cell>,
    pub roots: Vec<usize>,
}

impl BOC {
    /// Creates a new, empty `BOC`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a `Cell` to the `BOC` and returns its index.
    pub fn add_cell(&mut self, cell: Cell) -> usize {
        let index = self.cells.len();
        self.cells.push(cell);
        index
    }

    /// Adds a root index to the `BOC`.
    pub fn add_root(&mut self, index: usize) {
        self.roots.push(index);
    }

    /// Retrieves a reference to a `Cell` by index.
    pub fn get_cell(&self, index: usize) -> Option<&Cell> {
        self.cells.get(index)
    }

    /// Retrieves a mutable reference to a `Cell` by index.
    pub fn get_cell_mut(&mut self, index: usize) -> Option<&mut Cell> {
        self.cells.get_mut(index)
    }

    /// Retrieves a reference to the root `Cell`, if any.
    pub fn get_root_cell(&self) -> Option<&Cell> {
        self.roots
            .first()
            .and_then(|&root_idx| self.get_cell(root_idx))
    }

    /// Retrieves a mutable reference to the root `Cell`, if any.
    pub fn get_root_cell_mut(&mut self) -> Option<&mut Cell> {
        if let Some(&root_idx) = self.roots.first() {
            self.get_cell_mut(root_idx)
        } else {
            None
        }
    }

    /// Returns the number of cells in the `BOC`.
    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// Returns the number of roots in the `BOC`.
    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    /// Checks if the `BOC` is empty.
    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    /// Clears all cells and roots from the `BOC`.
    pub fn clear(&mut self) {
        self.cells.clear();
        self.roots.clear();
    }

    /// Sets the data for the root cell.
    pub fn set_data(&mut self, data: &[u8]) -> Result<(), SystemError> {
        if let Some(root_cell) = self.get_root_cell_mut() {
            root_cell.data = data.to_vec();
            root_cell.update_merkle_hash();
            Ok(())
        } else {
            Err(SystemError::new(
                SystemErrorType::NoRootCell,
                "No root cell defined".to_string(),
            ))
        }
    }

    /// Sets the Merkle root hash for the root cell.
    pub fn set_merkle_root(&mut self, merkle_root: [u8; 32]) -> Result<(), SystemError> {
        if let Some(root_cell) = self.get_root_cell_mut() {
            root_cell.merkle_hash = merkle_root;
            Ok(())
        } else {
            Err(SystemError::new(
                SystemErrorType::InvalidOperation,
                "No root cell defined".to_string(),
            ))
        }
    }

    /// Sets the proof data for the root cell.
    pub fn set_proof(&mut self, proof: &[u8]) -> Result<(), SystemError> {
        if let Some(root_cell) = self.get_root_cell_mut() {
            root_cell.proof = Some(proof.to_vec());
            Ok(())
        } else {
            Err(SystemError::new(
                SystemErrorType::NoRootCell,
                "No root cell defined".to_string(),
            ))
        }
    }

    /// Retrieves the Merkle root hash of the `BOC`.
    pub fn root_hash(&self) -> Result<[u8; 32], SystemError> {
        if let Some(root_cell) = self.get_root_cell() {
            Ok(root_cell.merkle_hash)
        } else {
            Err(SystemError::new(
                SystemErrorType::NoRootCell,
                "No root cell defined".to_string(),
            ))
        }
    }

    /// Serializes the `BOC` into a JSON byte vector.
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self).map_err(|e| Error::SerializationError(e.to_string()))
    }

    /// Deserializes a `BOC` from a JSON byte slice.
    pub fn deserialize(data: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(data).map_err(|e| Error::DeserializationError(e.to_string()))
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_creation() {
        let data = vec![1, 2, 3];
        let refs = vec![0, 1];
        let hash = [0u8; 32];

        let cell = Cell::new(data.clone(), refs.clone(), CellType::Ordinary, hash, None);

        assert_eq!(cell.data, data);
        assert_eq!(cell.references, refs);
        assert!(matches!(cell.cell_type, CellType::Ordinary));
        assert_eq!(cell.merkle_hash, hash);
        assert!(cell.proof.is_none());
    }

    #[test]
    fn test_cell_with_data() {
        let data = vec![1, 2, 3];
        let mut cell = Cell::with_data(data.clone());

        assert_eq!(cell.data, data);
        assert!(cell.references.is_empty());
        assert!(matches!(cell.cell_type, CellType::Ordinary));

        cell.update_merkle_hash();
        assert_ne!(cell.merkle_hash, [0u8; 32]);
    }

    #[test]
    fn test_boc_operations() {
        let mut boc = BOC::new();
        assert!(boc.is_empty());

        let cell1 = Cell::with_data(vec![1, 2, 3]);

        let idx1 = boc.add_cell(cell1);
        boc.add_root(idx1);

        assert_eq!(boc.cell_count(), 2);
        assert_eq!(boc.root_count(), 1);

        let root_cell = boc.get_root_cell().unwrap();
        assert_eq!(root_cell.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_boc_clear() {
        let mut boc = BOC::new();
        let cell = Cell::with_data(vec![1, 2, 3]);
        let idx = boc.add_cell(cell);
        boc.add_root(idx);

        assert!(!boc.is_empty());
        boc.clear();
        assert!(boc.is_empty());
        assert_eq!(boc.root_count(), 0);
    }

    #[test]
    fn test_boc_set_data() {
        let mut boc = BOC::new();
        let cell = Cell::with_data(vec![1, 2, 3]);
        let idx = boc.add_cell(cell);
        boc.add_root(idx);

        let new_data = vec![7, 8, 9];
        boc.set_data(&new_data).unwrap();

        let root_cell = boc.get_root_cell().unwrap();
        assert_eq!(root_cell.data, new_data);
    }

    #[test]
    fn test_boc_set_merkle_root() {
        let mut boc = BOC::new();
        let cell = Cell::with_data(vec![1, 2, 3]);
        let idx = boc.add_cell(cell);
        boc.add_root(idx);

        let new_merkle_root = [42u8; 32];
        boc.set_merkle_root(new_merkle_root).unwrap();

        let root_cell = boc.get_root_cell().unwrap();
        assert_eq!(root_cell.merkle_hash, new_merkle_root);
    }

    #[test]
    fn test_boc_set_proof() {
        let mut boc = BOC::new();
        let cell = Cell::with_data(vec![1, 2, 3]);
        let idx = boc.add_cell(cell);
        boc.add_root(idx);

        let proof_data = vec![10, 20, 30];
        boc.set_proof(&proof_data).unwrap();

        let root_cell = boc.get_root_cell().unwrap();
        assert_eq!(root_cell.proof.as_ref().unwrap(), &proof_data);
    }

    #[test]
    fn test_boc_root_hash() {
        let mut boc = BOC::new();
        let mut cell = Cell::with_data(vec![1, 2, 3]);
        cell.update_merkle_hash();
        let idx = boc.add_cell(cell);
        boc.add_root(idx);

        let root_hash = boc.root_hash().unwrap();
        assert_eq!(root_hash, boc.get_root_cell().unwrap().merkle_hash);
    }

    #[test]
    fn test_boc_serialize_deserialize() {
        let mut boc = BOC::new();
        let mut cell = Cell::with_data(vec![10, 20, 30]);
        cell.update_merkle_hash();
        let idx = boc.add_cell(cell);
        boc.add_root(idx);

        let serialized = boc.serialize().unwrap();
        let deserialized = BOC::deserialize(&serialized).unwrap();

        assert_eq!(boc.cells.len(), deserialized.cells.len());
        assert_eq!(boc.roots.len(), deserialized.roots.len());
        assert_eq!(boc.cells[0].data, deserialized.cells[0].data);
        assert_eq!(boc.cells[0].merkle_hash, deserialized.cells[0].merkle_hash);
    }

    #[test]
    fn test_boc_no_root_cell_error() {
        let mut boc = BOC::new();

        let result = boc.set_data(&[1, 2, 3]);
        assert!(matches!(
            result,
            Err(SystemError {
                error_type: SystemErrorType::NoRootCell,
                ..
            })
        ));
    }

    #[test]
    fn test_system_error_display() {
        let error = SystemError::new(
            SystemErrorType::InvalidTransaction,
            "Transaction validation failed".to_string(),
        );
        assert_eq!(
            error.to_string(),
            "Invalid transaction: Transaction validation failed"
        );
    }

    #[test]
    fn test_error_conversions() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let error: Error = io_err.into();
        assert!(matches!(error, Error::IoError(_)));

        let system_err = SystemError::new(
            SystemErrorType::InvalidNonce,
            "Invalid nonce value".to_string(),
        );
        let error: Error = system_err.into();
        assert!(matches!(error, Error::SystemError(_)));
    }

    #[test]
    fn test_error_display() {
        let error = Error::InvalidProof;
        assert_eq!(error.to_string(), "Invalid proof");

        let error = Error::WalletError("Balance too low".to_string());
        assert_eq!(error.to_string(), "Wallet error: Balance too low");
    }
}
