use crate::core::zkps::plonky2::Plonky2SystemHandle;
use crate::core::zkps::proof::{ProofMetadataJS, ProofType, ProofWithMetadataJS, ZkProof};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn generate_proof(
    old_balance: u64,
    old_nonce: u64,
    new_balance: u64,
    new_nonce: u64,
    transfer_amount: u64,
) -> Result<Uint8Array, JsValue> {
    let plonky2_system_handle = Plonky2SystemHandle::new()?;

    let proof_bytes = plonky2_system_handle.generate_proof_js(
        old_balance,
        old_nonce,
        new_balance,
        new_nonce,
        transfer_amount,
    )?;

    Ok(Uint8Array::from(&proof_bytes[..]))
}

#[wasm_bindgen]
pub fn verify_proof(proof_bytes: &Uint8Array) -> Result<bool, JsValue> {
    let plonky2_system_handle = Plonky2SystemHandle::new()?;
    let proof_vec = proof_bytes.to_vec();
    let result = plonky2_system_handle.verify_proof_js(&proof_vec)?;
    Ok(result)
}

#[wasm_bindgen]
pub fn create_proof_with_metadata(
    proof_bytes: &Uint8Array,
    merkle_root: &Uint8Array,
    public_inputs: &Uint8Array,
    timestamp: u64,
) -> Result<JsValue, JsValue> {
    let proof_vec = proof_bytes.to_vec();
    let merkle_root_array: [u8; 32] = merkle_root
        .to_vec()
        .try_into()
        .map_err(|_| JsValue::from_str("Invalid merkle root length"))?;
    let public_inputs_vec = public_inputs.to_vec();

    let merkle_root_vec = merkle_root_array.to_vec();

    let public_inputs_u64: Vec<u64> = public_inputs_vec
        .chunks_exact(8)
        .map(|chunk| {
            let array: [u8; 8] = chunk.try_into().unwrap();
            u64::from_le_bytes(array)
        })
        .collect();

    let zk_proof = ZkProof {
        proof_data: proof_vec,
        merkle_root: merkle_root_vec,
        public_inputs: public_inputs_u64,
        timestamp,
    };

    let proof_metadata =
        ProofMetadataJS::new(ProofType::StateTransition as i32, None, timestamp, None);

    let proof_with_metadata = ProofWithMetadataJS::new(
        serde_wasm_bindgen::to_value(&zk_proof)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize proof: {}", e)))?,
        proof_metadata,
    )?;

    proof_with_metadata.proof().and_then(|_proof_js| {
        serde_wasm_bindgen::to_value(&proof_with_metadata)
            .map_err(|e| JsValue::from_str(&format!("Failed to serialize metadata: {}", e)))
    })
}
