use plonky2::{
    field::goldilocks_field::GoldilocksField, hash::hash_types::RichField,
    plonk::proof::ProofWithPublicInputs,
};
use sha2::{Digest, Sha256};
use ton_types::{
    cells_serialization::{deserialize_tree_of_cells, serialize_tree_of_cells},
    BuilderData, Cell, CellBuilder, Result as TonResult, SliceData,
};

const BOC_HASH_SIZE: usize = 32;
const BRIDGE_PREFIX: &[u8] = b"ZKBRIDGE";

/// BOC container for complete ZK proof data
#[derive(Clone, Debug)]
pub struct ZkProofBoc {
    /// Full Plonky2 proof
    proof: ProofWithPublicInputs<GoldilocksField, 2>,
    /// Verification key
    vk_hash: [u8; 32],
    /// Public inputs
    public_inputs: Vec<GoldilocksField>,
    /// Auxiliary data (e.g., Merkle paths)
    auxiliary_data: Vec<u8>,
}

/// Slice of BOC data for OP_RETURN
#[derive(Clone, Debug)]
pub struct ZkProofSlice {
    /// Hash of the complete BOC
    boc_hash: [u8; BOC_HASH_SIZE],
    /// Proof verification metadata
    metadata: ProofMetadata,
}

#[derive(Clone, Copy, Debug)]
pub struct ProofMetadata {
    /// Proof protocol version
    version: u8,
    /// Proof type identifier
    proof_type: ProofType,
    /// Block height bounds
    height_bounds: HeightBounds,
}

#[derive(Clone, Copy, Debug)]
pub enum ProofType {
    Deposit = 1,
    Withdrawal = 2,
    Transfer = 3,
}

#[derive(Clone, Copy, Debug)]
pub struct HeightBounds {
    min_height: u32,
    max_height: u32,
}

impl ZkProofBoc {
    /// Create new ZK proof BOC
    pub fn new(
        proof: ProofWithPublicInputs<GoldilocksField, 2>,
        vk_hash: [u8; 32],
        public_inputs: Vec<GoldilocksField>,
        auxiliary_data: Vec<u8>,
    ) -> TonResult<Self> {
        Ok(Self {
            proof,
            vk_hash,
            public_inputs,
            auxiliary_data,
        })
    }

    /// Serialize BOC to bytes
    pub fn serialize(&self) -> TonResult<Vec<u8>> {
        let mut builder = CellBuilder::new();

        // Store proof data
        let proof_data = self.proof.to_bytes();
        builder.store_bytes(&proof_data)?;

        // Store verification key hash
        builder.store_bytes(&self.vk_hash)?;

        // Store public inputs
        for input in &self.public_inputs {
            builder.store_u256(input.to_canonical_u64() as u256)?;
        }

        // Store auxiliary data
        builder.store_bytes(&self.auxiliary_data)?;

        let cell = builder.build()?;
        serialize_tree_of_cells(&cell)
    }

    /// Calculate BOC hash for OP_RETURN
    pub fn calculate_hash(&self) -> TonResult<[u8; BOC_HASH_SIZE]> {
        let boc_data = self.serialize()?;
        let mut hasher = Sha256::new();
        hasher.update(&boc_data);
        Ok(hasher.finalize().into())
    }

    /// Create slice for OP_RETURN
    pub fn create_slice(&self, metadata: ProofMetadata) -> TonResult<ZkProofSlice> {
        Ok(ZkProofSlice {
            boc_hash: self.calculate_hash()?,
            metadata,
        })
    }
}

impl ZkProofSlice {
    /// Convert slice to OP_RETURN data
    pub fn to_op_return(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(80);

        // Add bridge identifier
        data.extend_from_slice(BRIDGE_PREFIX);

        // Add BOC hash
        data.extend_from_slice(&self.boc_hash);

        // Add compact metadata
        data.push(self.metadata.version);
        data.push(self.metadata.proof_type as u8);
        data.extend_from_slice(&self.metadata.height_bounds.min_height.to_le_bytes());
        data.extend_from_slice(&self.metadata.height_bounds.max_height.to_le_bytes());

        data
    }

    /// Reconstruct slice from OP_RETURN data
    pub fn from_op_return(data: &[u8]) -> Result<Self, &'static str> {
        if data.len() < BRIDGE_PREFIX.len() + BOC_HASH_SIZE + 10 {
            return Err("Invalid OP_RETURN data length");
        }

        // Verify prefix
        if &data[..BRIDGE_PREFIX.len()] != BRIDGE_PREFIX {
            return Err("Invalid bridge prefix");
        }

        let mut offset = BRIDGE_PREFIX.len();

        // Extract BOC hash
        let mut boc_hash = [0u8; BOC_HASH_SIZE];
        boc_hash.copy_from_slice(&data[offset..offset + BOC_HASH_SIZE]);
        offset += BOC_HASH_SIZE;

        // Extract metadata
        let version = data[offset];
        offset += 1;

        let proof_type = match data[offset] {
            1 => ProofType::Deposit,
            2 => ProofType::Withdrawal,
            3 => ProofType::Transfer,
            _ => return Err("Invalid proof type"),
        };
        offset += 1;

        let min_height = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);
        offset += 4;

        let max_height = u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ]);

        Ok(Self {
            boc_hash,
            metadata: ProofMetadata {
                version,
                proof_type,
                height_bounds: HeightBounds {
                    min_height,
                    max_height,
                },
            },
        })
    }
}

/// BitcoinBridge implementation extensions
impl<T: BridgeConfig> BitcoinBridge<T> {
    /// Create OP_RETURN data with BOC hash
    fn create_op_return_data(&self, zk_boc: &ZkProofBoc) -> Result<Vec<u8>, DispatchError> {
        let metadata = ProofMetadata {
            version: 1,
            proof_type: ProofType::Deposit,
            height_bounds: HeightBounds {
                min_height: self.current_height,
                max_height: self.current_height + MAX_CONFIRMATION_DELAY,
            },
        };

        let slice = zk_boc
            .create_slice(metadata)
            .map_err(|_| DispatchError::Other("Failed to create BOC slice"))?;

        Ok(slice.to_op_return())
    }

    /// Store complete BOC and use hash in OP_RETURN
    pub fn process_proof(&mut self, zk_boc: ZkProofBoc) -> Result<Transaction, DispatchError> {
        // Store complete BOC
        self.store_proof_boc(&zk_boc)?;

        // Create Bitcoin transaction with BOC hash
        let op_return_data = self.create_op_return_data(&zk_boc)?;

        // Create funding transaction
        self.create_funding_transaction(
            self.config.amount,
            &self.create_multisig_script()?,
            &op_return_data,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boc_serialization() {
        let (proof, vk) = generate_test_proof();
        let zk_boc = ZkProofBoc::new(
            proof,
            vk,
            vec![GoldilocksField::from_canonical_u64(1)],
            vec![],
        )
        .unwrap();

        let serialized = zk_boc.serialize().unwrap();
        let hash = zk_boc.calculate_hash().unwrap();

        assert_eq!(hash.len(), BOC_HASH_SIZE);
    }

    #[test]
    fn test_op_return_slice() {
        let (proof, vk) = generate_test_proof();
        let zk_boc = ZkProofBoc::new(
            proof,
            vk,
            vec![GoldilocksField::from_canonical_u64(1)],
            vec![],
        )
        .unwrap();

        let metadata = ProofMetadata {
            version: 1,
            proof_type: ProofType::Deposit,
            height_bounds: HeightBounds {
                min_height: 100,
                max_height: 200,
            },
        };

        let slice = zk_boc.create_slice(metadata).unwrap();
        let op_return = slice.to_op_return();

        assert!(op_return.len() <= 80);

        // Test reconstruction
        let reconstructed = ZkProofSlice::from_op_return(&op_return).unwrap();
        assert_eq!(reconstructed.boc_hash, slice.boc_hash);
        assert_eq!(
            reconstructed.metadata.height_bounds.min_height,
            metadata.height_bounds.min_height
        );
    }
}
