use crate::core::error::errors::Error;
use crate::core::error::SystemError;
use crate::core::hierarchy::client::wallet_extension::sparse_merkle_tree_wasm::SparseMerkleTreeWasm;
use crate::core::hierarchy::client::wallet_extension::wallet_extension_types::{
    WalletExtension, WalletExtensionStateChange,
};
use crate::core::types::boc::BOC;
use crate::core::types::ovp_ops::WalletOpCode;
use crate::core::zkps::plonky2::Plonky2SystemHandle;
use crate::core::zkps::proof::ZkProof;
use js_sys::{Date, Promise, Uint8Array};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen(typescript_custom_section)]
const TS_APPEND_CONTENT: &'static str = r#"
interface ByteArray32 {
    toArray(): Uint8Array;
}
"#;

#[wasm_bindgen]
#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct ByteArray32(#[wasm_bindgen(skip)] pub [u8; 32]);

#[wasm_bindgen]
impl ByteArray32 {
    #[wasm_bindgen(constructor)]
    pub fn new(array: &[u8]) -> Result<ByteArray32, JsValue> {
        if array.len() != 32 {
            return Err(JsValue::from_str("Array must be 32 bytes long"));
        }
        let mut result = [0u8; 32];
        result.copy_from_slice(array);
        Ok(ByteArray32(result))
    }

    #[wasm_bindgen(js_name = fromWasmAbi)]
    pub fn from_wasm_abi(val: JsValue) -> Result<ByteArray32, JsValue> {
        let array = js_sys::Uint8Array::new(&val);
        Self::new(&array.to_vec())
    }

    #[wasm_bindgen(js_name = toWasmAbi)]
    pub fn to_wasm_abi(&self) -> JsValue {
        let array = js_sys::Uint8Array::new_with_length(32);
        array.copy_from(&self.0);
        array.into()
    }
}
