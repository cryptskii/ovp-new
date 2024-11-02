// src/core/state/zkp/proof.rs

use plonky2::plonk::proof::Proof;
use plonky2_field::goldilocks_field::GoldilocksField;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZkProof {
    pub proof_data: Vec<u8>,
    pub public_inputs: Vec<u64>,
    pub merkle_root: [u8; 32],
    pub timestamp: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ProofType {
    StateTransition,
    BalanceTransfer,
    MerkleInclusion,
    Aggregate,
}

impl ZkProof {
    pub fn new(
        proof_data: Vec<u8>,
        public_inputs: Vec<u64>,
        merkle_root: [u8; 32],
        timestamp: u64,
    ) -> Self {
        Self {
            proof_data,
            public_inputs,
            merkle_root,
            timestamp,
        }
    }

    pub fn verify(&self, verifier: &ProofVerifier) -> Result<bool, JsValue> {
        verifier.verify_proof(self)
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, JsValue> {
        bincode::serialize(self)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<ZkProof, JsValue> {
        bincode::deserialize(bytes)
            .map_err(|e| JsValue::from_str(&format!("Deserialization error: {}", e)))
    }
}

#[wasm_bindgen]
pub struct ProofVerifier {
    verifier_data: Arc<Mutex<plonky2::plonk::config::PoseidonGoldilocksConfig>>,
}
#[wasm_bindgen]
impl ProofVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(verifier_data: Vec<u8>) -> Result<ProofVerifier, JsValue> {
        let verifier = bincode::deserialize(&verifier_data)
            .map_err(|e| JsValue::from_str(&format!("Invalid verifier data: {}", e)))?;

        Ok(ProofVerifier {
            verifier_data: Arc::new(Mutex::new(verifier)),
        })
    }

    pub fn verify_proof(&self, proof: &ZkProof) -> Result<bool, JsValue> {
        // Convert proof data back to PLONKY2 proof
        let plonky_proof = self
            .decode_proof(&proof.proof_data)
            .map_err(|e| JsValue::from_str(&format!("Invalid proof data: {}", e)))?;

        // Convert public inputs
        let inputs: Vec<GoldilocksField> = proof
            .public_inputs
            .iter()
            .map(|&x| GoldilocksField::from(x))
            .collect();

        // Verify the proof
        let verifier = self
            .verifier_data
            .lock()
            .map_err(|e| JsValue::from_str(&format!("Lock error: {}", e)))?;

        verifier
            .verify(&plonky_proof, &inputs)
            .map_err(|e| JsValue::from_str(&format!("Verification error: {}", e)))
    }

    fn decode_proof(&self, proof_data: &[u8]) -> Result<Proof<GoldilocksField>, ProofError> {
        bincode::deserialize(proof_data).map_err(|_| ProofError::InvalidProofData)
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofWithMetadata {
    proof: ZkProof,
    metadata: ProofMetadata,
}

#[wasm_bindgen]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofMetadata {
    proof_type: ProofType,
    channel_id: Option<[u8; 32]>,
    created_at: u64,
    verified_at: Option<u64>,
}

#[derive(Debug)]
pub enum ProofError {
    InvalidProofData,
    VerificationError(String),
    SerializationError(String),
}

#[wasm_bindgen]
pub struct BatchProofVerifier {
    verifier: Arc<Mutex<ProofVerifier>>,
    batch_size: usize,
}

#[wasm_bindgen]
impl BatchProofVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(verifier_data: Vec<u8>, batch_size: usize) -> Result<BatchProofVerifier, JsValue> {
        let verifier = ProofVerifier::new(verifier_data)?;
        Ok(BatchProofVerifier {
            verifier: Arc::new(Mutex::new(verifier)),
            batch_size,
        })
    }

    pub fn verify_batch(&self, proofs: Vec<ZkProof>) -> Result<Vec<bool>, JsValue> {
        let verifier = self
            .verifier
            .lock()
            .map_err(|e| JsValue::from_str(&format!("Lock error: {}", e)))?;

        // Split into batches
        let results = proofs
            .chunks(self.batch_size)
            .map(|batch| {
                batch
                    .iter()
                    .map(|proof| verifier.verify_proof(proof))
                    .collect::<Result<Vec<bool>, JsValue>>()
            })
            .collect::<Result<Vec<Vec<bool>>, JsValue>>()?;

        Ok(results.into_iter().flatten().collect())
    }
}

#[wasm_bindgen]
pub struct ProofAggregator {
    proofs: Vec<ZkProof>,
    metadata: Vec<ProofMetadata>,
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

    pub fn add_proof(&mut self, proof: ZkProof, metadata: ProofMetadata) {
        self.proofs.push(proof);
        self.metadata.push(metadata);
    }

    pub fn aggregate(&self) -> Result<ZkProof, JsValue> {
        // Combine proofs into single aggregate proof
        let aggregated_data = self.aggregate_proofs()?;

        // Generate new proof data combining all proofs
        let mut proof_data = Vec::new();
        for proof in &self.proofs {
            proof_data.extend(&proof.proof_data);
        }

        // Create aggregate proof
        Ok(ZkProof {
            proof_data,
            public_inputs: aggregated_data.public_inputs,
            merkle_root: aggregated_data.merkle_root,
            timestamp: current_timestamp(),
        })
    }

    fn aggregate_proofs(&self) -> Result<AggregatedData, JsValue> {
        // Actual proof aggregation logic
        todo!()
    }
}

struct AggregatedData {
    public_inputs: Vec<u64>,
    merkle_root: [u8; 32],
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_serialization() {
        let proof = ZkProof {
            proof_data: vec![1, 2, 3],
            public_inputs: vec![4, 5, 6],
            merkle_root: [0; 32],
            timestamp: current_timestamp(),
        };

        let bytes = proof.to_bytes().unwrap();
        let decoded = ZkProof::from_bytes(&bytes).unwrap();

        assert_eq!(proof.proof_data, decoded.proof_data);
        assert_eq!(proof.public_inputs, decoded.public_inputs);
        assert_eq!(proof.merkle_root, decoded.merkle_root);
        assert_eq!(proof.timestamp, decoded.timestamp);
    }

    #[test]
    fn test_batch_verification() {
        let verifier_data = vec![1, 2, 3]; // Mock verifier data
        let batch_verifier = BatchProofVerifier::new(verifier_data, 2).unwrap();

        let proofs = vec![
            ZkProof::new(vec![1], vec![2], [0; 32], current_timestamp()),
            ZkProof::new(vec![3], vec![4], [0; 32], current_timestamp()),
            ZkProof::new(vec![5], vec![6], [0; 32], current_timestamp()),
        ];

        let results = batch_verifier.verify_batch(proofs).unwrap();
        assert_eq!(results.len(), 3);
    }
}
