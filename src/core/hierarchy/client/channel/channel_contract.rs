// src/core/hierarchy/client/channel/channel_contract.rs

use crate::core::types::ovp_ops::ChannelOpCode;
use crate::core::types::ovp_types::*;
use sha2::{Digest, Sha256};

pub struct ChannelContract {
    pub id: ChannelId,
    pub state: ChannelState,
    pub balance: ChannelBalance,
    pub nonce: ChannelNonce,
    pub seqno: ChannelSeqNo,
    pub op_code: ChannelOpCode,
    pub status: ChannelStatus,
}

impl ChannelContract {
    pub fn new(id: ChannelId) -> Self {
        Self {
            id,
            state: String::new(),
            balance: 0,
            nonce: 0,
            seqno: 0,
            op_code: ChannelOpCode::InitChannel,
            status: ChannelStatus::Active,
        }
    }

    pub fn create_state_boc(&self) -> Result<BOC, SystemError> {
        // Create BOC for state representation
        let mut boc = BOC {
            cells: vec![],
            roots: vec![],
        };

        // Create state cell
        let state_cell = Cell {
            data: self.serialize_state()?,
            references: vec![],
            cell_type: CellType::Ordinary,
            merkle_hash: self.calculate_state_hash()?,
        };

        // Add state cell to BOC
        boc.cells.push(state_cell);
        boc.roots.push(0); // State cell is the root

        Ok(boc)
    }

    fn serialize_state(&self) -> Result<Vec<u8>, SystemError> {
        let mut data = Vec::new();

        // Serialize channel id
        data.extend_from_slice(&self.id);

        // Serialize balance
        data.extend_from_slice(&self.balance.to_le_bytes());

        // Serialize nonce
        data.extend_from_slice(&self.nonce.to_le_bytes());

        // Serialize seqno
        data.extend_from_slice(&self.seqno.to_le_bytes());

        // Serialize opcode
        data.push(self.op_code.to_u8());

        // Serialize state string
        let state_bytes = self.state.as_bytes();
        data.extend_from_slice(&(state_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(state_bytes);

        Ok(data)
    }

    fn calculate_state_hash(&self) -> Result<[u8; 32], SystemError> {
        let mut hasher = Sha256::new();

        // Hash serialized state
        hasher.update(&self.serialize_state()?);

        // Convert to fixed size array
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize());

        Ok(hash)
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<BOC, SystemError> {
        // Validate transaction
        self.validate_transaction(&tx)?;

        // Update state based on transaction
        self.apply_transaction(&tx)?;

        // Create state BOC
        let boc = self.create_state_boc()?;

        Ok(boc)
    }

    fn validate_transaction(&self, tx: &Transaction) -> Result<(), SystemError> {
        // Check sender
        if tx.sender != self.id {
            return Err(SystemError {
                error_type: SystemErrorType::InvalidTransaction,
                id: [0u8; 32],
                data: vec![],
                error_data: SystemErrorData {
                    id: [0u8; 32],
                    data: vec![],
                },
                error_data_id: [0u8; 32],
            });
        }

        // Verify nonce
        if tx.nonce != self.nonce + 1 {
            return Err(SystemError {
                error_type: SystemErrorType::InvalidNonce,
                id: [0u8; 32],
                data: vec![],
                error_data: SystemErrorData {
                    id: [0u8; 32],
                    data: vec![],
                },
                error_data_id: [0u8; 32],
            });
        }

        // Check sequence number
        if tx.sequence_number != self.seqno + 1 {
            return Err(SystemError {
                error_type: SystemErrorType::InvalidSequence,
                id: [0u8; 32],
                data: vec![],
                error_data: SystemErrorData {
                    id: [0u8; 32],
                    data: vec![],
                },
                error_data_id: [0u8; 32],
            });
        }

        // Verify 50% spending rule
        if tx.amount > self.balance / 2 {
            return Err(SystemError {
                error_type: SystemErrorType::SpendingLimitExceeded,
                id: [0u8; 32],
                data: vec![],
                error_data: SystemErrorData {
                    id: [0u8; 32],
                    data: vec![],
                },
                error_data_id: [0u8; 32],
            });
        }

        Ok(())
    }

    fn apply_transaction(&mut self, tx: &Transaction) -> Result<(), SystemError> {
        // Update balance
        self.balance = self.balance.checked_sub(tx.amount).ok_or(SystemError {
            error_type: SystemErrorType::InsufficientBalance,
            id: [0u8; 32],
            data: vec![],
            error_data: SystemErrorData {
                id: [0u8; 32],
                data: vec![],
            },
            error_data_id: [0u8; 32],
        })?;

        // Update nonce
        self.nonce += 1;

        // Update seqno
        self.seqno += 1;

        Ok(())
    }
}
