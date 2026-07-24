use crate::types::*;
use slotmap::SlotMap;

#[derive(Debug, Clone)]
pub struct GraphTopology {
    pub node_keys: SlotMap<NodeId, usize>,
    pub node_index_to_id: Vec<NodeId>,

    pub edge_keys: SlotMap<EdgeId, usize>,
    pub edge_index_to_id: Vec<EdgeId>,

    pub nodes: DenseStorage<NodeData>,
    pub node_kinds: DenseStorage<NodeKind>,
    pub edges: DenseStorage<EdgeData>,

    pub hierarchy: Hierarchy,
    pub edge_sources: DenseStorage<NodeId>,
    pub edge_targets: DenseStorage<NodeId>,
}

impl Default for GraphTopology {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphTopology {
    pub fn new() -> Self {
        Self {
            node_keys: SlotMap::with_key(),
            node_index_to_id: Vec::new(),
            edge_keys: SlotMap::with_key(),
            edge_index_to_id: Vec::new(),
            nodes: DenseStorage::new(),
            node_kinds: DenseStorage::new(),
            edges: DenseStorage::new(),
            hierarchy: Hierarchy::new(),
            edge_sources: DenseStorage::new(),
            edge_targets: DenseStorage::new(),
        }
    }

    pub fn node_count(&self) -> usize {
        self.node_index_to_id.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_index_to_id.len()
    }

    pub fn unlink_from_hierarchy(&mut self, _id: NodeId, idx: usize) {
        let parent = *self.hierarchy.parent.get(idx);
        let prev = *self.hierarchy.prev_sibling.get(idx);
        let next = *self.hierarchy.next_sibling.get(idx);

        if let Some(prev_sib_id) = prev {
            if let Some(&prev_idx) = self.node_keys.get(prev_sib_id) {
                self.hierarchy.next_sibling.set(prev_idx, next);
            }
        } else if let Some(p_id) = parent {
            if let Some(&p_idx) = self.node_keys.get(p_id) {
                self.hierarchy.first_child.set(p_idx, next);
            }
        }

        if let Some(next_sib_id) = next {
            if let Some(&next_idx) = self.node_keys.get(next_sib_id) {
                self.hierarchy.prev_sibling.set(next_idx, prev);
            }
        }

        self.hierarchy.parent.set(idx, None);
        self.hierarchy.next_sibling.set(idx, None);
        self.hierarchy.prev_sibling.set(idx, None);
    }
}
