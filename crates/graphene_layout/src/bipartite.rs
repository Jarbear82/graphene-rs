use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState, NodeId};
use std::collections::HashMap;

pub struct BipartiteLayout<F> {
    pub partition_fn: F,
    pub column_spacing: f32,
    pub vertical_spacing: f32,
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

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
