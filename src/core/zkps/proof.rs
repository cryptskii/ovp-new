use plonky2::field::goldilocks_field::GoldilocksField;
use plonky2::plonk::config::PoseidonGoldilocksConfig;
use plonky2::plonk::{
    circuit_builder::CircuitBuilder, circuit_data::CircuitConfig, proof::ProofWithPublicInputs,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

const D: usize = 2;
type F = GoldilocksField;
type C = PoseidonGoldilocksConfig;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkProof {
    pub proof_data: Vec<u8>,
    pub public_inputs: Vec<u64>,
    pub merkle_root: Vec<u8>,
    pub timestamp: u64,
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum ProofType {
    StateTransition = 0,
    BalanceTransfer = 1,
    MerkleInclusion = 2,
    Aggregate = 3,
}

impl ZkProof {
    pub fn new(
        proof_data: Vec<u8>,
        public_inputs: Vec<u64>,
        merkle_root: Vec<u8>,
        timestamp: u64,
    ) -> Self {
        Self {
            proof_data,
            public_inputs,
            merkle_root,
            timestamp,
        }
    }
}

#[wasm_bindgen]
pub struct ProofVerifier {
    circuit_data: plonky2::plonk::circuit_data::CircuitData<F, C, D>,
}

#[wasm_bindgen]
impl ProofVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new() -> ProofVerifier {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = CircuitBuilder::<F, D>::new(config);

        let x = builder.add_virtual_public_input();
        let y = builder.add_virtual_public_input();
        let sum = builder.add(x, y);
        builder.register_public_input(sum);

        let circuit_data = builder.build::<C>();

        ProofVerifier { circuit_data }
    }

    pub fn verify_proof(&self, proof_js: &JsValue) -> Result<bool, JsValue> {
        let zk_proof: ZkProof = serde_wasm_bindgen::from_value(proof_js.clone())
            .map_err(|e| JsValue::from_str(&format!("Invalid proof data: {}", e)))?;

        let proof: ProofWithPublicInputs<F, C, D> = bincode::deserialize(&zk_proof.proof_data)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))?;

        self.circuit_data
            .verify(proof)
            .map(|_| true)
            .map_err(|e| JsValue::from_str(&format!("Verification error: {}", e)))
    }
}

impl Default for ProofVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
pub struct ProofAggregator {
    proofs: Vec<ZkProof>,
    metadata: Vec<ProofMetadataInternal>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ProofMetadataInternal {
    proof_type: ProofType,
    channel_id: Option<Vec<u8>>,
    created_at: u64,
    verified_at: Option<u64>,
}

#[wasm_bindgen]
impl ProofAggregator {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        ProofAggregator {
            proofs: Vec::new(),
            metadata: Vec::new(),
        }
    }

    pub fn add_proof(&mut self, proof_js: &JsValue, metadata_js: &JsValue) -> Result<(), JsValue> {
        let proof: ZkProof = serde_wasm_bindgen::from_value(proof_js.clone())
            .map_err(|e| JsValue::from_str(&format!("Invalid proof data: {}", e)))?;

        let metadata: ProofMetadataInternal =
            serde_wasm_bindgen::from_value(metadata_js.clone())
                .map_err(|e| JsValue::from_str(&format!("Invalid metadata: {}", e)))?;

        self.proofs.push(proof);
        self.metadata.push(metadata);
        Ok(())
    }

    pub fn aggregate(&self) -> Result<JsValue, JsValue> {
        let aggregated_proof = self.aggregate_proofs()?;
        serde_wasm_bindgen::to_value(&aggregated_proof)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    fn aggregate_proofs(&self) -> Result<ZkProof, JsValue> {
        if self.proofs.is_empty() {
            return Err(JsValue::from_str("No proofs to aggregate"));
        }

        let mut combined_data = Vec::new();
        let mut combined_inputs = Vec::new();
        let timestamp = current_timestamp();

        for proof in &self.proofs {
            combined_data.extend_from_slice(&proof.proof_data);
            combined_inputs.extend_from_slice(&proof.public_inputs);
        }

        let merkle_root = self
            .proofs
            .last()
            .ok_or_else(|| JsValue::from_str("No proofs available"))?
            .merkle_root
            .clone();

        Ok(ZkProof {
            proof_data: combined_data,
            public_inputs: combined_inputs,
            merkle_root,
            timestamp,
        })
    }
}

impl Default for ProofAggregator {
    fn default() -> Self {
        Self::new()
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofMetadataJS {
    proof_type: i32,
    channel_id: Option<Vec<u8>>,
    created_at: u64,
    verified_at: Option<u64>,
}

#[wasm_bindgen]
impl ProofMetadataJS {
    #[wasm_bindgen(constructor)]
    pub fn new(
        proof_type: i32,
        channel_id: Option<Vec<u8>>,
        created_at: u64,
        verified_at: Option<u64>,
    ) -> Self {
        Self {
            proof_type,
            channel_id,
            created_at,
            verified_at,
        }
    }

    #[wasm_bindgen(getter)]
    pub fn proof_type(&self) -> i32 {
        self.proof_type
    }

    #[wasm_bindgen(getter)]
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    #[wasm_bindgen(getter)]
    pub fn verified_at(&self) -> Option<u64> {
        self.verified_at
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofWithMetadataJS {
    proof: ZkProof,
    metadata: ProofMetadataJS,
}

#[wasm_bindgen]
impl ProofWithMetadataJS {
    #[wasm_bindgen(constructor)]
    pub fn new(proof: JsValue, metadata: ProofMetadataJS) -> Result<ProofWithMetadataJS, JsValue> {
        let proof: ZkProof = serde_wasm_bindgen::from_value(proof)
            .map_err(|e| JsValue::from_str(&format!("Invalid proof data: {}", e)))?;

        Ok(ProofWithMetadataJS { proof, metadata })
    }

    #[wasm_bindgen(getter)]
    pub fn proof(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.proof)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    #[wasm_bindgen(getter)]
    pub fn metadata(&self) -> ProofMetadataJS {
        self.metadata.clone()
    }
}

pub fn current_timestamp() -> u64 {
    use js_sys::Date;
    (Date::now() / 1000.0) as u64
}

#[derive(Debug)]
pub enum ProofError {
    InvalidProofData,
    VerificationError(String),
    SerializationError(String),
}

impl std::fmt::Display for ProofError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofError::InvalidProofData => write!(f, "Invalid proof data"),
            ProofError::VerificationError(msg) => write!(f, "Verification error: {}", msg),
            ProofError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for ProofError {}
