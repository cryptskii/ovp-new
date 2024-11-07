// ./src/core/tokens/token_types.rs

use crate::core::errors::SystemError;
use crate::core::utils::{ByteArray, ByteArray32};
use crate::core::wallet::{Curve, HashType, NetworkType, TokenType, WalletType};

// This is a struct that represents a token.
pub struct Token {
    pub token_type: TokenType,         // The type of the token.
    pub wallet_type: WalletType,       // The type of the wallet.
    pub network_type: NetworkType,     // The type of the network.
    pub curve: Curve,                  // The curve of the token.
    pub hash_type: HashType,           // The hash type of the token.
    pub public_key: ByteArray<32>,     // The public key of the token.
    pub private_key: ByteArray<32>,    // The private key of the token.
    pub signature: ByteArray<64>,      // The signature of the token.
    pub message: ByteArray<32>,        // The message of the token.
    pub nullifier: ByteArray<32>,      // The nullifier of the token.
    pub input_owner: ByteArray<32>,    // The input owner of the token.
    pub output_owner: ByteArray<32>,   // The output owner of the token.
    pub amount: u64,                   // The amount of the token.
    pub fee: u64,                      // The fee of the token.
    pub nonce: u64,                    // The nonce of the token.
    pub memo: ByteArray<32>,           // The memo of the token.
    pub root: ByteArray<32>,           // The root of the token.
    pub nullifier_hash: ByteArray<32>, // The nullifier hash of the token.
    pub recipient: ByteArray<32>,      // The recipient of the token.
}

impl Token {
    // This function generates a token.
    pub fn generate_token(
        token_type: TokenType,
        wallet_type: WalletType,
        network_type: NetworkType,
        curve: Curve,
        hash_type: HashType,
        public_key: ByteArray<32>,
        private_key: ByteArray<32>,
        signature: ByteArray<64>,
        message: ByteArray<32>,
        nullifier: ByteArray<32>,
        input_owner: ByteArray<32>,
        output_owner: ByteArray<32>,
        amount: u64,
        fee: u64,
        nonce: u64,
        memo: ByteArray<32>,
        root: ByteArray<32>,
        nullifier_hash: ByteArray<32>,
        recipient: ByteArray<32>,
    ) -> Result<Token, SystemError> {
        // Generate the token.
        Ok(Token {
            token_type,
            wallet_type,
            network_type,
            curve,
            hash_type,
            public_key,
            private_key,
            signature,
            message,
            nullifier,
            input_owner,
            output_owner,
            amount,
            fee,
            nonce,
            memo,
            root,
            nullifier_hash,
            recipient,
        })
    }
}
