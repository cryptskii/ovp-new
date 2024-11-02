use plonky2::{
    field::goldilocks_field::GoldilocksField,
    gadgets::arithmetic::ArithmeticGate,
    iop::{
        target::Target,
        witness::{PartialWitness, WitnessWrite},
    },
    plonk::{
        circuit_builder::CircuitBuilder,
        circuit_data::{CircuitConfig, CircuitData, CommonCircuitData},
        config::PoseidonGoldilocksConfig,
        proof::ProofWithPublicInputs,
    },
};
use wasm_bindgen::prelude::*;

const D: usize = 2; // Extension degree

#[wasm_bindgen]
pub struct Plonky2System {
    circuit_config: CircuitConfig,
    state_transition_circuit: StateTransitionCircuitData,
}

#[wasm_bindgen]
impl Plonky2System {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Result<Plonky2System, JsValue> {
        let circuit_config = CircuitConfig::standard_recursion_config();

        // Build state transition circuit
        let mut builder = CircuitBuilder::<GoldilocksField, D>::new(circuit_config.clone());
        let state_transition_circuit = build_state_transition_circuit(&mut builder)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(Plonky2System {
            circuit_config,
            state_transition_circuit,
        })
    }

    pub fn generate_proof(
        &self,
        old_balance: u64,
        old_nonce: u64,
        new_balance: u64,
        new_nonce: u64,
        transfer_amount: u64,
    ) -> Result<Vec<u8>, JsValue> {
        let circuit_data = &self.state_transition_circuit.circuit_data;
        let mut pw = PartialWitness::new();

        fill_state_transition_witness(
            &mut pw,
            &self.state_transition_circuit,
            old_balance,
            old_nonce,
            new_balance,
            new_nonce,
            transfer_amount,
        );

        let proof = circuit_data
            .prove(pw)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        Ok(proof.to_bytes())
    }

    pub fn verify_proof(&self, proof_bytes: &[u8]) -> Result<(), JsValue> {
        let circuit_data = &self.state_transition_circuit.circuit_data;
        let common_data = circuit_data.common.clone();

        let proof =
            ProofWithPublicInputs::<GoldilocksField, PoseidonGoldilocksConfig, D>::from_bytes(
                proof_bytes.to_vec(),
                &common_data,
            )
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        circuit_data
            .verify(proof)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

struct StateTransitionCircuitData {
    circuit_data: CircuitData<GoldilocksField, PoseidonGoldilocksConfig, D>,
    old_balance_target: Target,
    old_nonce_target: Target,
    new_balance_target: Target,
    new_nonce_target: Target,
    transfer_amount_target: Target,
}

fn build_state_transition_circuit(
    builder: &mut CircuitBuilder<GoldilocksField, D>,
) -> Result<StateTransitionCircuitData, PlonkyError> {
    // Define public inputs
    let old_balance_target = builder.add_virtual_public_input();
    let old_nonce_target = builder.add_virtual_public_input();
    let new_balance_target = builder.add_virtual_public_input();
    let new_nonce_target = builder.add_virtual_public_input();
    let transfer_amount_target = builder.add_virtual_public_input();

    // Constraints: new_nonce == old_nonce + 1
    let one = builder.one();
    let old_nonce_plus_one = builder.add(old_nonce_target, one);
    builder.connect(old_nonce_plus_one, new_nonce_target);

    // Constraints: new_balance == old_balance - transfer_amount
    let old_balance_minus_amount = builder.sub(old_balance_target, transfer_amount_target);
    builder.connect(old_balance_minus_amount, new_balance_target);

    // Constraint: transfer_amount <= old_balance / 2
    let half = builder.constant(GoldilocksField::from_canonical_u64(2).inverse());
    let half_balance = builder.mul(old_balance_target, half);
    builder.range_check(transfer_amount_target, 64);
    builder.range_check(half_balance, 64);
    builder.arithmetic(
        ArithmeticGate::new_from_config(&builder.config),
        &[transfer_amount_target, half_balance],
        &[GoldilocksField::ONE, GoldilocksField::NEG_ONE],
        GoldilocksField::ZERO,
    );

    // Build circuit data
    let circuit_data = builder.build::<PoseidonGoldilocksConfig>();

    Ok(StateTransitionCircuitData {
        circuit_data,
        old_balance_target,
        old_nonce_target,
        new_balance_target,
        new_nonce_target,
        transfer_amount_target,
    })
}

fn fill_state_transition_witness(
    pw: &mut PartialWitness<GoldilocksField>,
    circuit: &StateTransitionCircuitData,
    old_balance: u64,
    old_nonce: u64,
    new_balance: u64,
    new_nonce: u64,
    transfer_amount: u64,
) {
    let old_balance_f = GoldilocksField::from_canonical_u64(old_balance);
    let old_nonce_f = GoldilocksField::from_canonical_u64(old_nonce);
    let new_balance_f = GoldilocksField::from_canonical_u64(new_balance);
    let new_nonce_f = GoldilocksField::from_canonical_u64(new_nonce);
    let transfer_amount_f = GoldilocksField::from_canonical_u64(transfer_amount);

    pw.set_target(circuit.old_balance_target, old_balance_f);
    pw.set_target(circuit.old_nonce_target, old_nonce_f);
    pw.set_target(circuit.new_balance_target, new_balance_f);
    pw.set_target(circuit.new_nonce_target, new_nonce_f);
    pw.set_target(circuit.transfer_amount_target, transfer_amount_f);
}

#[derive(Debug)]
enum PlonkyError {
    InvalidInput(String),
    ProofGenerationError(String),
}

impl std::fmt::Display for PlonkyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match &*self {
            PlonkyError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            PlonkyError::ProofGenerationError(msg) => write!(f, "Proof generation error: {}", msg),
        }
    }
}

impl std::error::Error for PlonkyError {}
