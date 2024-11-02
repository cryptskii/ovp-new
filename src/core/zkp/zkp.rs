use plonky2::{
    field::extension::Extendable,
    gates::{arithmetic_base::ArithmeticGate, constant::ConstantGate, poseidon::PoseidonGate},
    hash::hash_types::RichField,
    plonk::circuit_data::CircuitConfig,
};
use plonky2_field::types::{Field, Sample};
use std::ops::Add;
use wasm_bindgen::prelude::*;

#[derive(Clone, Debug)]
pub struct Column {
    index: usize,
}

impl Column {
    pub fn new(index: usize) -> Self {
        Self { index }
    }
}

#[derive(Clone, Debug)]
pub struct VirtualCell {
    column: Column,
    rotation: i32,
}

impl VirtualCell {
    pub fn new(column: Column, rotation: i32) -> Self {
        Self { column, rotation }
    }

    pub fn value(&self) -> u64 {
        // Implementation for value lookup
        0
    }
}

#[derive(Clone)]
pub struct PlonkConfig<F: Field> {
    pub degree: usize,
    pub num_variables: usize,
    pub _field: std::marker::PhantomData<F>,
}

impl<F: Field> PlonkConfig<F> {
    pub fn standard() -> Self {
        Self {
            degree: 1 << 12, // 4096
            num_variables: 100,
            _field: std::marker::PhantomData,
        }
    }
}

pub struct Circuit<F: Field> {
    config: CircuitConfig,
    gates: Vec<Box<dyn Gate<F>>>,
    public_inputs: Vec<Column>,
    witnesses: Vec<Column>,
}

impl<F: Field> Circuit<F> {
    pub fn new(
        config: CircuitConfig,
        gates: Vec<Box<dyn Gate<F>>>,
        public_inputs: Vec<Column>,
        witnesses: Vec<Column>,
    ) -> Result<Self, String> {
        if gates.is_empty() {
            return Err("Circuit must contain at least one gate".to_string());
        }
        Ok(Self {
            config,
            gates,
            public_inputs,
            witnesses,
        })
    }

    pub fn check_circuit(&self) -> Result<(), String> {
        if self.gates.is_empty() {
            return Err("Circuit must contain at least one gate".to_string());
        }
        Ok(())
    }
}

#[wasm_bindgen]
#[derive(Clone, Debug)]
pub enum GateType {
    Arithmetic,
    Constant,
    Poseidon,
}

pub trait Gate<F: Field>: Send + Sync {
    fn evaluate(&self, inputs: &[F]) -> F;
    fn gate_type(&self) -> GateType;
}

pub struct ZkCircuitBuilder<F: RichField + Extendable<D>, const D: usize> {
    config: CircuitConfig,
    gates: Vec<Box<dyn Gate<F>>>,
    public_inputs: Vec<Column>,
    witnesses: Vec<Column>,
}

impl<F: RichField + Extendable<D>, const D: usize> ZkCircuitBuilder<F, D> {
    pub fn new(config: CircuitConfig) -> Self {
        Self {
            config,
            gates: Vec::new(),
            public_inputs: Vec::new(),
            witnesses: Vec::new(),
        }
    }

    pub fn add_gate(&mut self, gate: Box<dyn Gate<F>>) {
        self.gates.push(gate);
    }

    pub fn add_public_input(&mut self) -> Column {
        let column = Column::new(self.public_inputs.len());
        self.public_inputs.push(column.clone());
        column
    }

    pub fn add_witness(&mut self) -> Column {
        let column = Column::new(self.witnesses.len());
        self.witnesses.push(column.clone());
        column
    }

    pub fn connect(&mut self, left: &VirtualCell, right: &VirtualCell) {
        let gate = ArithmeticGate::new_from_config(&self.config);
        self.add_gate(Box::new(gate));
    }

    pub fn assert_zero(&mut self, x: &VirtualCell) {
        let gate = ArithmeticGate::new_from_config(&self.config);
        self.add_gate(Box::new(gate));
    }

    pub fn assert_one(&mut self, x: &VirtualCell) {
        let gate = ArithmeticGate::new_from_config(&self.config);
        self.add_gate(Box::new(gate));
    }

    pub fn assert_equal(&mut self, left: &VirtualCell, right: &VirtualCell) {
        let gate = ArithmeticGate::new_from_config(&self.config);
        self.add_gate(Box::new(gate));
    }

    pub fn add(&mut self, a: &VirtualCell, b: &VirtualCell) -> VirtualCell {
        let c = self.add_witness();
        let gate = ArithmeticGate::new_from_config(&self.config);
        self.add_gate(Box::new(gate));
        VirtualCell::new(c, 0)
    }

    pub fn sub(&mut self, a: &VirtualCell, b: &VirtualCell) -> VirtualCell {
        let c = self.add_witness();
        let gate = ArithmeticGate::new_from_config(&self.config);
        self.add_gate(Box::new(gate));
        VirtualCell::new(c, 0)
    }

    pub fn mul(&mut self, a: &VirtualCell, b: &VirtualCell) -> VirtualCell {
        let c = self.add_witness();
        let gate = ArithmeticGate::new_from_config(&self.config);
        self.add_gate(Box::new(gate));
        VirtualCell::new(c, 0)
    }

    pub fn constant(&mut self, value: u64) -> VirtualCell {
        let c = self.add_witness();
        let gate = ConstantGate::new(value);
        self.add_gate(Box::new(gate));
        VirtualCell::new(c, 0)
    }

    pub fn poseidon(&mut self, inputs: &[VirtualCell]) -> VirtualCell {
        let output = self.add_witness();
        let gate = PoseidonGate::<F, D>::new();
        self.add_gate(Box::new(gate));
        VirtualCell::new(output, 0)
    }

    pub fn build_channel_state_circuit(
        &mut self,
        old_balance: &VirtualCell,
        new_balance: &VirtualCell,
        amount: &VirtualCell,
    ) -> Result<(), JsValue> {
        // Calculate new balance
        let computed_new = self.sub(old_balance, amount);
        self.assert_equal(&computed_new, new_balance);

        // Enforce 50% spending limit
        let half = self.constant(2).mul(&amount);
        let amount_ok = self.sub(&half, old_balance);
        self.assert_zero(&amount_ok);

        Ok(())
    }

    pub fn build_merkle_proof_circuit(
        &mut self,
        leaf: &VirtualCell,
        path: &[VirtualCell],
        root: &VirtualCell,
    ) -> Result<(), JsValue> {
        let mut current = leaf.clone();

        for sibling in path {
            // Order nodes lexicographically before hashing
            let (l, r) = if current.value() <= sibling.value() {
                (current.clone(), sibling.clone())
            } else {
                (sibling.clone(), current)
            };

            // Poseidon hash the pair to get the next level's node
            current = self.poseidon(&[l, r]);
        }

        // Ensure the calculated root matches the provided root
        self.assert_equal(&current, root);

        Ok(())
    }

    pub fn build_transaction_circuit(
        &mut self,
        sender_balance: &VirtualCell,
        recipient_balance: &VirtualCell,
        amount: &VirtualCell,
        sender_nonce: &VirtualCell,
    ) -> Result<(), JsValue> {
        // Update balances
        let new_sender = self.sub(sender_balance, amount);
        let new_recipient = self.add(recipient_balance, amount);

        // Enforce spending limit
        let half = self.constant(2);
        let half_balance = self.mul(sender_balance, &half);
        let amount_ok = self.sub(&half_balance, amount);
        self.assert_zero(&amount_ok);

        // Increment nonce
        let new_nonce = self.add(sender_nonce, &self.constant(1));
        self.assert_equal(sender_nonce, &new_nonce);

        Ok(())
    }

    pub fn build_circuit(&self) -> Result<Circuit<F>, JsValue> {
        Circuit::new(
            self.config.clone(),
            self.gates.to_vec(),
            self.public_inputs.clone(),
            self.witnesses.clone(),
        )
        .map_err(|e| JsValue::from_str(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};
    use plonky2::{field::goldilocks_field::GoldilocksField, hash::hash_types::HashOut};

    const D: usize = 2;
    type C = PoseidonGoldilocksConfig;
    type F = <C as GenericConfig<D>>::F;

    #[test]
    fn test_channel_state_circuit() {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = ZkCircuitBuilder::<F, D>::new(config);

        let old_balance = builder.add_public_input();
        let new_balance = builder.add_public_input();
        let amount = builder.add_public_input();

        builder
            .build_channel_state_circuit(
                &VirtualCell::new(old_balance, 0),
                &VirtualCell::new(new_balance, 0),
                &VirtualCell::new(amount, 0),
            )
            .unwrap();

        let circuit = builder.build_circuit().unwrap();
        assert!(circuit.check_circuit().is_ok());
    }

    #[test]
    fn test_merkle_proof_circuit() {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = ZkCircuitBuilder::<F, D>::new(config);

        let leaf = builder.add_public_input();
        let root = builder.add_public_input();

        let path_inputs: Vec<_> = (0..32).map(|_| builder.add_public_input()).collect();
        let path: Vec<_> = path_inputs
            .iter()
            .map(|c| VirtualCell::new(c.clone(), 0))
            .collect();

        builder
            .build_merkle_proof_circuit(
                &VirtualCell::new(leaf, 0),
                &path,
                &VirtualCell::new(root, 0),
            )
            .unwrap();

        let circuit = builder.build_circuit().unwrap();
        assert!(circuit.check_circuit().is_ok());
    }

    #[test]
    fn test_transaction_circuit() {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = ZkCircuitBuilder::<F, D>::new(config);

        let sender_balance = builder.add_public_input();
        let recipient_balance = builder.add_public_input();
        let amount = builder.add_public_input();
        let sender_nonce = builder.add_public_input();

        builder
            .build_transaction_circuit(
                &VirtualCell::new(sender_balance, 0),
                &VirtualCell::new(recipient_balance, 0),
                &VirtualCell::new(amount, 0),
                &VirtualCell::new(sender_nonce, 0),
            )
            .unwrap();

        let circuit = builder.build_circuit().unwrap();
        assert!(circuit.check_circuit().is_ok());
    }
}
