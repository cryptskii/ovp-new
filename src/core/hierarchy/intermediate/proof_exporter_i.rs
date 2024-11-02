use crate::core::hierarchy::ZkProof;
use crate::core::state::boc::cell_serialization::BOC;
use crate::core::zkp::circuit_builder::ZkCircuitBuilder;
use crate::core::zkp::plonky2::Plonky2System;

pub struct ProofExporterI;

pub struct IntermediateProofExporter;

impl IntermediateProofExporter {
    pub fn generate_intermediate_proof(intermediate_root: [u8; 32]) -> ZkProof {
        let mut circuit = ZkCircuitBuilder::new();

        circuit.add_public_input(&intermediate_root);
        circuit.add_intermediate_root_constraints();

        let proving_system = Plonky2System::new();
        proving_system.generate_proof(&circuit)
    }

    pub fn package_intermediate_boc(intermediate_root: [u8; 32], proof: ZkProof) -> BOC {
        let mut boc = BOC::new();
        boc.add_root_cell(intermediate_root);
        boc.add_proof_cell(proof);

        boc.finalize()
    }
}
