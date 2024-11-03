#[derive(Debug, Clone)]
pub struct Cell {
    pub data: Vec<u8>,
    pub references: Vec<usize>,
    pub cell_type: CellType,
    pub merkle_hash: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellType {
    Ordinary,
    PrunedBranch,
    LibraryReference,
    MerkleProof,
    MerkleUpdate,
}

impl Cell {
    pub fn new(
        data: Vec<u8>,
        references: Vec<usize>,
        cell_type: CellType,
        merkle_hash: [u8; 32],
    ) -> Self {
        Self {
            data,
            references,
            cell_type,
            merkle_hash,
        }
    }

    pub fn with_data(data: Vec<u8>) -> Self {
        Self {
            data,
            references: Vec::new(),
            cell_type: CellType::Ordinary,
            merkle_hash: [0u8; 32],
        }
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            data: Vec::new(),
            references: Vec::new(),
            cell_type: CellType::Ordinary,
            merkle_hash: [0u8; 32],
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BOC {
    pub cells: Vec<Cell>,
    pub roots: Vec<usize>,
}

impl BOC {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_cell(&mut self, cell: Cell) {
        self.cells.push(cell);
    }

    pub fn add_root(&mut self, index: usize) {
        self.roots.push(index);
    }

    pub fn get_cell(&self, index: usize) -> Option<&Cell> {
        self.cells.get(index)
    }

    pub fn get_cell_mut(&mut self, index: usize) -> Option<&mut Cell> {
        self.cells.get_mut(index)
    }

    pub fn get_root_cell(&self) -> Option<&Cell> {
        self.roots
            .first()
            .and_then(|&root_idx| self.get_cell(root_idx))
    }

    pub fn get_root_cell_mut(&mut self) -> Option<&mut Cell> {
        if let Some(&root_idx) = self.roots.first() {
            self.get_cell_mut(root_idx)
        } else {
            None
        }
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    pub fn root_count(&self) -> usize {
        self.roots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.roots.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_creation() {
        let data = vec![1, 2, 3];
        let refs = vec![0, 1];
        let hash = [0u8; 32];

        let cell = Cell::new(data.clone(), refs.clone(), CellType::Ordinary, hash);

        assert_eq!(cell.data, data);
        assert_eq!(cell.references, refs);
        assert!(matches!(cell.cell_type, CellType::Ordinary));
        assert_eq!(cell.merkle_hash, hash);
    }

    #[test]
    fn test_cell_with_data() {
        let data = vec![1, 2, 3];
        let cell = Cell::with_data(data.clone());

        assert_eq!(cell.data, data);
        assert!(cell.references.is_empty());
        assert!(matches!(cell.cell_type, CellType::Ordinary));
    }

    #[test]
    fn test_boc_operations() {
        let mut boc = BOC::new();
        assert!(boc.is_empty());

        let cell1 = Cell::with_data(vec![1, 2, 3]);
        let cell2 = Cell::with_data(vec![4, 5, 6]);

        boc.add_cell(cell1);
        boc.add_cell(cell2);
        boc.add_root(0);

        assert_eq!(boc.cell_count(), 2);
        assert_eq!(boc.root_count(), 1);

        let root_cell = boc.get_root_cell().unwrap();
        assert_eq!(root_cell.data, vec![1, 2, 3]);
    }

    #[test]
    fn test_boc_clear() {
        let mut boc = BOC::new();
        boc.add_cell(Cell::with_data(vec![1, 2, 3]));
        boc.add_root(0);

        assert!(!boc.is_empty());
        boc.clear();
        assert!(boc.is_empty());
        assert_eq!(boc.root_count(), 0);
    }
}
