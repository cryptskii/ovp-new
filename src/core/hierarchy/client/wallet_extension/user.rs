// ./src/core/hierarchy/client/wallet_extension/user.rs
use crate::core::types::boc::BOC;
use std::collections::HashSet;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use web_sys::{Blob, Url};

// This is a struct that represents a user (private)
#[wasm_bindgen]
#[derive(Clone, Debug)]
pub struct User {
    name: String,
    channels: HashSet<[u8; 32]>,
}
// This is a function that returns the name of the user
