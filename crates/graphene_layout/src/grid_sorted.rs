use crate::collision::resolve_overlaps;
use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState};
use std::collections::HashMap;

/// Grid layout with nodes sorted by degree or rank.
///
/// Reference: Grid layout with nodes sorted by degree or rank.
pub struct GridSortedLayout {
    pub columns: usize,
    pub node_spacing: f32,
    pub sort_by_degree: bool,
}

impl Default for GridSortedLayout {
    fn default() -> Self {
        Self {
            columns: 5,
            node_spacing: 80.0,
            sort_by_degree: true,
        }
    }
}

impl GridSortedLayout {
    pub fn with_columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }

    pub fn with_node_spacing(mut self, spacing: f32) -> Self {
        self.node_spacing = spacing;
        self
    }

    pub fn with_sort_by_degree(mut self, sort: bool) -> Self {
        self.sort_by_degree = sort;
        self
    }
}

impl<S: Copy> Layout<S> for GridSortedLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut sorted_nodes = state.node_index_to_id.clone();
        if self.sort_by_degree {
            let mut degrees = HashMap::new();
            for &id in &state.node_index_to_id {
                degrees.insert(id, 0);
            }
            for idx in 0..state.edges.len() {
                let src = *state.edge_sources.get(idx);
                let tgt = *state.edge_targets.get(idx);
                if let Some(deg) = degrees.get_mut(&src) { *deg += 1; }
                if let Some(deg) = degrees.get_mut(&tgt) { *deg += 1; }
            }
            sorted_nodes.sort_by(|a, b| degrees[b].cmp(&degrees[a]));
        } else {
            sorted_nodes.sort();
        }

        let mut max_w = 0.0f32;
        let mut max_h = 0.0f32;
        for i in 0..n {
            let size = *state.sizes.get(i);
            max_w = max_w.max(size.w);
            max_h = max_h.max(size.h);
        }

        let col_step = self.node_spacing.max(max_w + 10.0);
        let row_step = self.node_spacing.max(max_h + 10.0);

        let cols = self.columns.max(1);
        for (idx, id) in sorted_nodes.into_iter().enumerate() {
            if let Some(&node_idx) = state.node_keys.get(id) {
                let r = idx / cols;
                let c = idx % cols;
                let x = (c as f32) * col_step;
                let y = (r as f32) * row_step;
                state.positions.set(node_idx, Vec2::new(x, y));
            }
        }

        resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
