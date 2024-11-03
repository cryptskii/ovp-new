use crate::core::error::errors::{SystemError, SystemErrorType};
use crate::core::types::boc::{Cell, CellType, BOC};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::convert::From;

// Move to types/channel.rs or similar
mod types {
    use super::*;
    pub type Channel = String;
    pub type ChannelId = String;
    pub type ChannelState = String;
    pub type ChannelBalance = u64;
    pub type ChannelNonce = u64;
    pub type ChannelSeqNo = u64;
    pub type ChannelSignature = String;
    pub type PrivateChannelState = HashMap<String, String>;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    #[repr(u8)]
    pub enum ContractOpCode {
        CreatePayment = 0xA0,
        UpdateState = 0xA1,
        FinalizeState = 0xA2,
        DisputeState = 0xA3,
        InitChannel = 0xA4,
    }

    impl From<ContractOpCode> for u8 {
        fn from(code: ContractOpCode) -> Self {
            code as u8
        }
    }

    #[derive(Debug, Clone)]
    pub enum ChannelStatus {
        Active,
        TransactionPending {
            timeout: u64,
            reciepent_acceptance: ChannelSignature,
        },
        DisputeOpen {
            timeout: u64,
            challenger: ChannelId,
        },
        Closing {
            initiated_at: u64,
            final_state: Box<PrivateChannelState>,
        },
        Closed,
    }

    #[derive(Debug, Clone)]
    pub struct Transaction {
        pub sender: ChannelId,
        pub nonce: ChannelNonce,
        pub sequence_number: ChannelSeqNo,
        pub amount: ChannelBalance,
    }

    impl Transaction {
        pub fn new(
            sender: ChannelId,
            nonce: ChannelNonce,
            sequence_number: ChannelSeqNo,
            amount: ChannelBalance,
        ) -> Self {
            Self {
                sender,
                nonce,
                sequence_number,
                amount,
            }
        }
    }
}

use types::*;

#[derive(Debug, Clone)]
pub struct ChannelContract {
    pub id: ChannelId,
    pub state: ChannelState,
    pub balance: ChannelBalance,
    pub nonce: ChannelNonce,
    pub seqno: ChannelSeqNo,
    pub op_code: ContractOpCode,
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
            op_code: ContractOpCode::InitChannel,
            status: ChannelStatus::Active,
        }
    }

    pub fn update_balance(&mut self, amount: ChannelBalance) -> Result<(), SystemError> {
        let new_balance = self.balance.checked_add(amount).ok_or_else(|| {
            SystemError::new(
                SystemErrorType::InvalidAmount,
                "Balance overflow".to_string(),
            )
        })?;
        self.balance = new_balance;
        Ok(())
    }

    pub fn create_state_boc(&self) -> Result<BOC, SystemError> {
        let mut boc = BOC::new();
        let state_cell = Cell::new(
            self.serialize_state()?,
            vec![],
            CellType::Ordinary,
            self.calculate_state_hash()?,
        );
        boc.add_cell(state_cell);
        boc.add_root(0);
        Ok(boc)
    }

    fn serialize_state(&self) -> Result<Vec<u8>, SystemError> {
        let mut data = Vec::new();
        data.extend_from_slice(self.id.as_bytes());
        data.extend_from_slice(&self.balance.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.seqno.to_le_bytes());
        data.push(u8::from(self.op_code));

        let state_bytes = self.state.as_bytes();
        data.extend_from_slice(&(state_bytes.len() as u32).to_le_bytes());
        data.extend_from_slice(state_bytes);

        Ok(data)
    }

    fn calculate_state_hash(&self) -> Result<[u8; 32], SystemError> {
        let mut hasher = Sha256::new();
        hasher.update(&self.serialize_state()?);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize());
        Ok(hash)
    }

    pub fn process_transaction(&mut self, tx: Transaction) -> Result<BOC, SystemError> {
        self.validate_transaction(&tx)?;
        self.apply_transaction(&tx)?;
        self.create_state_boc()
    }

    fn validate_transaction(&self, tx: &Transaction) -> Result<(), SystemError> {
        if tx.sender != self.id {
            return Err(SystemError::new(
                SystemErrorType::InvalidTransaction,
                "Invalid transaction sender".to_string(),
            ));
        }

        if tx.nonce != self.nonce + 1 {
            return Err(SystemError::new(
                SystemErrorType::InvalidNonce,
                "Invalid nonce".to_string(),
            ));
        }

        if tx.sequence_number != self.seqno + 1 {
            return Err(SystemError::new(
                SystemErrorType::InvalidSequence,
                "Invalid sequence number".to_string(),
            ));
        }

        if tx.amount > self.balance / 2 {
            return Err(SystemError::new(
                SystemErrorType::SpendingLimitExceeded,
                "Spending limit exceeded".to_string(),
            ));
        }

        Ok(())
    }

    fn apply_transaction(&mut self, tx: &Transaction) -> Result<(), SystemError> {
        self.balance = self.balance.checked_sub(tx.amount).ok_or_else(|| {
            SystemError::new(
                SystemErrorType::InsufficientBalance,
                "Insufficient balance".to_string(),
            )
        })?;

        self.nonce += 1;
        self.seqno += 1;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_transaction(
        sender: &str,
        nonce: u64,
        sequence_number: u64,
        amount: u64,
    ) -> Transaction {
        Transaction::new(sender.to_string(), nonce, sequence_number, amount)
    }

    #[test]
    fn test_new_channel_contract() {
        let id = "test_channel".to_string();
        let contract = ChannelContract::new(id.clone());

        assert_eq!(contract.id, id);
        assert_eq!(contract.balance, 0);
        assert_eq!(contract.nonce, 0);
        assert_eq!(contract.seqno, 0);
        assert_eq!(contract.op_code, ContractOpCode::InitChannel);
        assert!(matches!(contract.status, ChannelStatus::Active));
    }

    #[test]
    fn test_process_valid_transaction() {
        let mut contract = ChannelContract::new("test_channel".to_string());
        contract.update_balance(1000).unwrap();

        let tx = create_test_transaction("test_channel", 1, 1, 400);

        let result = contract.process_transaction(tx);
        assert!(result.is_ok());
        assert_eq!(contract.balance, 600);
        assert_eq!(contract.nonce, 1);
        assert_eq!(contract.seqno, 1);
    }

    #[test]
    fn test_validate_transaction_failures() {
        let contract = ChannelContract::new("test_channel".to_string());

        // Invalid sender
        let tx = create_test_transaction("wrong_sender", 1, 1, 100);
        assert!(matches!(
            contract.validate_transaction(&tx),
            Err(SystemError {
                error_type: SystemErrorType::InvalidTransaction,
                ..
            })
        ));

        // Invalid nonce
        let tx = create_test_transaction("test_channel", 2, 1, 100);
        assert!(matches!(
            contract.validate_transaction(&tx),
            Err(SystemError {
                error_type: SystemErrorType::InvalidNonce,
                ..
            })
        ));
    }

    #[test]
    fn test_spending_limit() {
        let mut contract = ChannelContract::new("test_channel".to_string());
        contract.update_balance(1000).unwrap();

        let tx = create_test_transaction("test_channel", 1, 1, 501);

        assert!(matches!(
            contract.process_transaction(tx),
            Err(SystemError {
                error_type: SystemErrorType::SpendingLimitExceeded,
                ..
            })
        ));
    }
}
