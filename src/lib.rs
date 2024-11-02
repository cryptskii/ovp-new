// ./src/lib.rs

use crate::core::types::{ChannelConfig, ZkProofSystem};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::Arc;
use wasm_bindgen::prelude::*;

pub mod core;
pub mod logging;
pub mod metrics;
pub mod network;
pub mod wasm;

#[wasm_bindgen]

pub struct OverpassWasm {
    zkp_system: Arc<RwLock<ZkProofSystem>>,
}

#[wasm_bindgen]
impl OverpassWasm {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<OverpassWasm, JsValue> {
        let zkp_system = Arc::new(RwLock::new(ZkProofSystem::new()));
        Ok(OverpassWasm { zkp_system })
    }
    #[wasm_bindgen]
    pub async fn create_channel(
        &self,
        wallet_id: Vec<u8>,
        sender: Vec<u8>,
        recipient: Vec<u8>,
        initial_balance: u64,
    ) -> Result<Vec<u8>, JsValue> {
        let mut zkp_system = self.zkp_system.write();

        let channel_id = zkp_system
            .create_channel(
                wallet_id
                    .try_into()
                    .map_err(|_| JsValue::from_str("Invalid wallet ID"))?,
                sender
                    .try_into()
                    .map_err(|_| JsValue::from_str("Invalid sender"))?,
                recipient
                    .try_into()
                    .map_err(|_| JsValue::from_str("Invalid recipient"))?,
                initial_balance,
                ChannelConfig::default(),
            )
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(channel_id.to_vec())
    }
}

// Logging and panic hook initialization for WASM
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
    tracing_wasm::set_as_global_default();
}

#[derive(Serialize, Deserialize)]
pub struct SparseMerkleTree {
    nodes: HashMap<Vec<u8>, Vec<u8>>,
    root: Vec<u8>,
}

#[wasm_bindgen]
pub struct WSSparseMerkleTree(SparseMerkleTree);

#[wasm_bindgen]
impl WSSparseMerkleTree {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        let smt = SparseMerkleTree {
            nodes: HashMap::new(),
            root: vec![0; 32],
        };
        WSSparseMerkleTree(smt)
    }

    pub fn insert(&mut self, key: &[u8], value: &[u8]) -> Result<(), JsValue> {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(value);
        let hash = hasher.finalize().to_vec();

        self.0.nodes.insert(key.to_vec(), hash.clone());
        self.0.root = hash;
        Ok(())
    }

    pub fn get(&self, key: &[u8]) -> Result<JsValue, JsValue> {
        match self.0.nodes.get(key) {
            Some(value) => Ok(serde_wasm_bindgen::to_value(&value)?),
            None => Ok(JsValue::NULL),
        }
    }

    pub fn get_root(&self) -> Result<JsValue, JsValue> {
        Ok(serde_wasm_bindgen::to_value(&self.0.root)?)
    }
}
