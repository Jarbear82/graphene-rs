use crate::collision::resolve_overlaps;
use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState, NodeId};
use std::collections::HashMap;

/// Bipartite graph layout.
///
/// Reference: Two-column bipartite partition placement.
pub struct BipartiteLayout<F = fn(NodeId) -> usize> {
    pub partition_fn: F,
    pub column_spacing: f32,
    pub vertical_spacing: f32,
}

impl Default for BipartiteLayout<fn(NodeId) -> usize> {
    fn default() -> Self {
        Self {
            partition_fn: |_id: NodeId| 0,
            column_spacing: 200.0,
            vertical_spacing: 100.0,
        }
    }
}

impl<F> BipartiteLayout<F> {
    pub fn with_partition_fn<F2: Fn(NodeId) -> usize>(self, partition_fn: F2) -> BipartiteLayout<F2> {
        BipartiteLayout {
            partition_fn,
            column_spacing: self.column_spacing,
            vertical_spacing: self.vertical_spacing,
        }
    }

    pub fn with_column_spacing(mut self, spacing: f32) -> Self {
        self.column_spacing = spacing;
        self
    }

    pub fn with_vertical_spacing(mut self, spacing: f32) -> Self {
        self.vertical_spacing = spacing;
        self
    }
}

impl<S: Copy, F: Fn(NodeId) -> usize> Layout<S> for BipartiteLayout<F> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let mut sets: HashMap<usize, Vec<NodeId>> = HashMap::new();
        for &id in &state.node_index_to_id {
            let part = (self.partition_fn)(id);
            sets.entry(part).or_default().push(id);
        }

        for (&col, nodes) in &sets {
            let x = (col as f32) * self.column_spacing;
            let mut curr_y = 0.0f32;

            for (idx, &id) in nodes.iter().enumerate() {
                if let Some(&node_idx) = state.node_keys.get(id) {
                    let size = *state.sizes.get(node_idx);
                    if idx > 0 {
                        let prev_idx = state.node_keys.get(nodes[idx - 1]).copied().unwrap();
                        let prev_h = state.sizes.get(prev_idx).h;
                        curr_y += (prev_h + size.h) * 0.5 + self.vertical_spacing;
                    }
                    state.positions.set(node_idx, Vec2::new(x, curr_y));
                }
            }
        }

        let collapsed = std::collections::HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}
