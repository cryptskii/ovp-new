use plonky2::{
    field::extension::Extendable,
    hash::hash_types::RichField,
    iop::target::Target,
    plonk::{circuit_builder::CircuitBuilder, circuit_data::CircuitConfig},
};
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

#[derive(Clone, Copy, Debug)]
pub struct VirtualCell {
    target: Target,
    rotation: i32,
}

impl VirtualCell {
    pub fn new(row: usize, column: usize) -> Self {
        Self {
            target: Target::wire(row, column),
            rotation: 0,
        }
    }

    pub fn value(&self) -> u64 {
        match self.target {
            Target::Wire(row, col) => col.try_into().unwrap(),
            _ => panic!("Expected wire target"),
        }
    }

    pub fn target(&self) -> Target {
        self.target
    }
}

pub struct Circuit<F: RichField + Extendable<D>, const D: usize> {
    builder: CircuitBuilder<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> Circuit<F, D> {
    pub fn new(config: CircuitConfig) -> Result<Self, String> {
        Ok(Self {
            builder: CircuitBuilder::new(config),
        })
    }

    pub fn check_circuit(&self) -> Result<(), String> {
        if self.builder.num_gates() == 0 {
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

pub struct ZkCircuitBuilder<F: RichField + Extendable<D>, const D: usize> {
    builder: CircuitBuilder<F, D>,
}

impl<F: RichField + Extendable<D>, const D: usize> ZkCircuitBuilder<F, D> {
    pub fn new(config: CircuitConfig) -> Self {
        Self {
            builder: CircuitBuilder::new(config),
        }
    }

    pub fn add_public_input(&mut self) -> VirtualCell {
        let row = self.builder.get_next_row();
        VirtualCell::new(row, 0)
    }

    pub fn add_witness(&mut self) -> VirtualCell {
        let row = self.builder.get_next_row();
        VirtualCell::new(row, 1)
    }

    pub fn connect(&mut self, left: VirtualCell, right: VirtualCell) {
        self.builder.connect(left.target(), right.target());
    }

    pub fn assert_zero(&mut self, x: VirtualCell) {
        let zero = self.builder.zero();
        self.builder.connect(x.target(), zero);
    }

    pub fn assert_one(&mut self, x: VirtualCell) {
        let one = self.builder.one();
        self.builder.connect(x.target(), one);
    }

    pub fn assert_equal(&mut self, left: VirtualCell, right: VirtualCell) {
        self.builder.connect(left.target(), right.target());
    }

    pub fn add(&mut self, a: VirtualCell, b: VirtualCell) -> VirtualCell {
        let row = self.builder.get_next_row();
        let target = self.builder.add(a.target(), b.target());
        VirtualCell::new(row, 2)
    }

    pub fn sub(&mut self, a: VirtualCell, b: VirtualCell) -> VirtualCell {
        let row = self.builder.get_next_row();
        let target = self.builder.sub(a.target(), b.target());
        VirtualCell::new(row, 2)
    }

    pub fn mul(&mut self, a: VirtualCell, b: VirtualCell) -> VirtualCell {
        let row = self.builder.get_next_row();
        let target = self.builder.mul(a.target(), b.target());
        VirtualCell::new(row, 2)
    }

    pub fn constant(&mut self, value: u64) -> VirtualCell {
        let row = self.builder.get_next_row();
        let target = self.builder.constant(F::from_canonical_u64(value));
        VirtualCell::new(row, 3)
    }

    pub fn poseidon(&mut self, inputs: &[VirtualCell]) -> VirtualCell {
        let row = self.builder.get_next_row();
        let input_targets: Vec<Target> = inputs.iter().map(|x| x.target()).collect();
        let hash = self
            .builder
            .hash_n_to_hash_no_pad::<PoseidonFunction<F, D>>(&input_targets);
        VirtualCell::new(row, 4)
    }

    pub fn build_channel_state_circuit(
        &mut self,
        old_balance: VirtualCell,
        new_balance: VirtualCell,
        amount: VirtualCell,
    ) -> Result<(), JsValue> {
        // Enforce balance conservation
        let computed_new = self.sub(old_balance, amount);
        self.assert_equal(computed_new, new_balance);

        // Enforce 50% spending limit using division by 2
        let half = self.constant(2);
        let half_balance = self.mul(old_balance, half);
        let amount_ok = self.sub(half_balance, amount);
        self.assert_zero(amount_ok);

        Ok(())
    }

    pub fn build_merkle_proof_circuit(
        &mut self,
        leaf: VirtualCell,
        path: &[VirtualCell],
        root: VirtualCell,
    ) -> Result<(), JsValue> {
        let mut current = leaf;

        for sibling in path {
            // Order nodes lexicographically before hashing
            let (l, r) = if current.value() <= sibling.value() {
                (current, *sibling)
            } else {
                (*sibling, current)
            };

            // Hash pair to get next level
            current = self.poseidon(&[l, r]);
        }

        // Check final root
        self.assert_equal(current, root);

        Ok(())
    }

    pub fn build_transaction_circuit(
        &mut self,
        sender_balance: VirtualCell,
        recipient_balance: VirtualCell,
        amount: VirtualCell,
        sender_nonce: VirtualCell,
    ) -> Result<(), JsValue> {
        // Update balances
        let _new_sender = self.sub(sender_balance, amount);
        let _new_recipient = self.add(recipient_balance, amount);

        // Spending limit check
        let half = self.constant(2);
        let half_balance = self.mul(sender_balance, half);
        let amount_ok = self.sub(half_balance, amount);
        self.assert_zero(amount_ok);

        // Increment nonce
        let one = self.constant(1);
        let new_nonce = self.add(sender_nonce, one);
        self.assert_equal(sender_nonce, new_nonce);

        Ok(())
    }

    pub fn build(self) -> Result<Circuit<F, D>, JsValue> {
        Circuit::new(self.builder.config.clone()).map_err(|e| JsValue::from_str(&e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plonky2::plonk::config::{GenericConfig, PoseidonGoldilocksConfig};

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
            .build_channel_state_circuit(old_balance, new_balance, amount)
            .unwrap();

        let circuit = builder.build().unwrap();
        assert!(circuit.check_circuit().is_ok());
    }

    #[test]
    fn test_merkle_proof_circuit() {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = ZkCircuitBuilder::<F, D>::new(config);

        let leaf = builder.add_public_input();
        let root = builder.add_public_input();

        let path_inputs: Vec<_> = (0..32).map(|_| builder.add_public_input()).collect();

        builder
            .build_merkle_proof_circuit(leaf, &path_inputs, root)
            .unwrap();

        let circuit = builder.build().unwrap();
        assert!(circuit.check_circuit().is_ok());
    }

    #[test]
    fn test_transaction_circuit() {
        let config = CircuitConfig::standard_recursion_config();
        let mut builder = ZkCircuitBuilder::<F, D>::new(config);

        let sender = builder.add_public_input();
        let recipient = builder.add_public_input();
        let amount = builder.add_public_input();
        let nonce = builder.add_public_input();

        builder
            .build_transaction_circuit(sender, recipient, amount, nonce)
            .unwrap();

        let circuit = builder.build().unwrap();
        assert!(circuit.check_circuit().is_ok());
    }
}
