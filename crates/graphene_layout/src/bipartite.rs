use crate::collision::resolve_overlaps;
use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState, NodeId};
use std::collections::HashMap;

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
            let col_height = (nodes.len() - 1) as f32 * self.vertical_spacing;
            let start_y = -col_height / 2.0;
            let x = (col as f32) * self.column_spacing;

            for (idx, &id) in nodes.iter().enumerate() {
                if let Some(&node_idx) = state.node_keys.get(id) {
                    let y = start_y + (idx as f32) * self.vertical_spacing;
                    state.positions.set(node_idx, Vec2::new(x, y));
                }
            }
        }

        resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
