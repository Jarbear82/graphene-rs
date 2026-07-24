use crate::types::*;

#[derive(Debug, Clone)]
pub struct GraphVisuals<S: Copy = ()> {
    pub positions: DenseStorage<Vec2>,
    pub sizes: DenseStorage<Size2>,
    pub selected: SelectionStore,
    pub computed_styles: DenseStorage<S>,
    pub edge_computed_styles: DenseStorage<S>,
}

impl<S: Copy + Default> Default for GraphVisuals<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Copy + Default> GraphVisuals<S> {
    pub fn new() -> Self {
        Self {
            positions: DenseStorage::new(),
            sizes: DenseStorage::new(),
            selected: SelectionStore::new(),
            computed_styles: DenseStorage::new(),
            edge_computed_styles: DenseStorage::new(),
        }
    }
}
