use bitvec::vec::BitVec;
use crate::state::GraphState;
use crate::types::*;

/// Non-owning view for filtered and induced subgraphs
#[derive(Debug, Clone)]
pub struct GraphView<'a, S: Copy = ()> {
    pub state: &'a GraphState<S>,
    pub node_mask: BitVec,
    pub edge_mask: BitVec,
}

impl<'a, S: Copy> GraphView<'a, S> {
    pub fn new(state: &'a GraphState<S>) -> Self {
        let n_len = state.node_index_to_id.len();
        let e_len = state.edge_index_to_id.len();
        let node_mask = BitVec::repeat(true, n_len);
        let edge_mask = BitVec::repeat(true, e_len);

        Self {
            state,
            node_mask,
            edge_mask,
        }
    }

    pub fn induced(state: &'a GraphState<S>, node_subset: &[NodeId]) -> Self {
        let n_len = state.node_index_to_id.len();
        let e_len = state.edge_index_to_id.len();
        let mut node_mask = BitVec::repeat(false, n_len);
        for &id in node_subset {
            if let Some(&idx) = state.node_keys.get(id) {
                node_mask.set(idx, true);
            }
        }

        let mut edge_mask = BitVec::repeat(false, e_len);
        for i in 0..e_len {
            let src = state.edge_sources[i];
            let tgt = state.edge_targets[i];
            let src_contained = state.node_keys.get(src).map_or(false, |&idx| node_mask[idx]);
            let tgt_contained = state.node_keys.get(tgt).map_or(false, |&idx| node_mask[idx]);
            if src_contained && tgt_contained {
                edge_mask.set(i, true);
            }
        }

        Self {
            state,
            node_mask,
            edge_mask,
        }
    }

    pub fn contains_node(&self, id: NodeId) -> bool {
        self.state.node_keys.get(id).map_or(false, |&idx| self.node_mask[idx])
    }

    pub fn contains_edge(&self, id: EdgeId) -> bool {
        self.state.edge_keys.get(id).map_or(false, |&idx| self.edge_mask[idx])
    }

    pub fn nodes(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.state.node_index_to_id.iter().enumerate().filter_map(move |(idx, &id)| {
            if self.node_mask[idx] { Some(id) } else { None }
        })
    }
}

/// Secondary index over PropValue key-value properties using bit vectors
#[derive(Debug, Clone, Default)]
pub struct PropertyIndex {
    pub by_key_val: std::collections::HashMap<(CompactString, PropValue), BitVec>,
}

impl PropertyIndex {
    pub fn new() -> Self {
        Self {
            by_key_val: std::collections::HashMap::new(),
        }
    }

    pub fn rebuild<S: Copy>(state: &GraphState<S>) -> Self {
        let mut index = Self::new();
        let n = state.node_index_to_id.len();
        for idx in 0..n {
            let props = &state.nodes.get(idx).props;
            for (k, v) in props {
                let mask = index
                    .by_key_val
                    .entry((k.clone(), v.clone()))
                    .or_insert_with(|| BitVec::repeat(false, n));
                if mask.len() < n {
                    mask.resize(n, false);
                }
                mask.set(idx, true);
            }
        }
        index
    }

    pub fn query(&self, key: &str, val: &PropValue) -> Option<&BitVec> {
        let key_cs = CompactString::from(key);
        self.by_key_val.get(&(key_cs, val.clone()))
    }
}
