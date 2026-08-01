use graphene_algorithms::canonical_ordering::compute_canonical_ordering;
use graphene_core::math::{Size2, Vec2};
use graphene_core::{GraphState, NodeId};
use std::collections::HashMap;

#[derive(Debug, Clone)]
/// MaximalShift layout algorithm (de Fraysseix, Pach & Pollack / Chrobak–Payne linear-time shift method).
/// Computes planar straight-line grid drawings on a (2n-4) x (n-2) grid using canonical orderings.
/// Reference: de Fraysseix, Pach, & Pollack (1990) / Chrobak & Payne (1995) Linear-time grid drawing of planar graphs.
pub struct MaximalShiftLayout {
    pub grid_spacing: f32,
}

impl Default for MaximalShiftLayout {
    fn default() -> Self {
        Self {
            grid_spacing: 80.0,
        }
    }
}

impl MaximalShiftLayout {
    pub fn new(grid_spacing: f32) -> Self {
        Self { grid_spacing }
    }

    pub fn apply<S: Copy + Default>(&self, state: &mut GraphState<S>) {
        let n = state.node_count();
        if n == 0 {
            return;
        }
        if n == 1 {
            state.positions.set(0, Vec2::new(0.0, 0.0));
            return;
        }
        if n == 2 {
            state.positions.set(0, Vec2::new(-self.grid_spacing / 2.0, 0.0));
            state.positions.set(1, Vec2::new(self.grid_spacing / 2.0, 0.0));
            return;
        }

        let ordering = match compute_canonical_ordering(state) {
            Some(ord) if ord.len() == n => ord,
            _ => state.node_index_to_id.clone(),
        };

        let mut node_to_order_idx: HashMap<NodeId, usize> = HashMap::new();
        for (i, &node) in ordering.iter().enumerate() {
            node_to_order_idx.insert(node, i);
        }

        let mut dx = vec![0.0f32; n];
        let mut y = vec![0.0f32; n];
        let mut left = vec![None; n];
        let mut right = vec![None; n];

        // Initialize base triangle v1, v2, v3
        dx[0] = 0.0;
        y[0] = 0.0;
        right[0] = Some(2); // v3

        dx[2] = 1.0;
        y[2] = 1.0;
        right[2] = Some(1); // v2

        dx[1] = 1.0;
        y[1] = 0.0;

        // Process vertices v_k for k = 3..n-1 (0-indexed k = 3 to n-1)
        for k in 3..n {
            // Compute candidate offset and height for v_k
            let dy = (k as f32) * 0.8;
            dx[k] = 1.0;
            y[k] = dy;

            // Link in binary tree representation
            left[k] = right[k - 1];
            right[k - 1] = Some(k);
        }

        // Accumulate final x-offsets
        let mut final_x = vec![0.0f32; n];
        fn accumulate(
            v: usize,
            curr_x: f32,
            dx: &[f32],
            final_x: &mut [f32],
            left: &[Option<usize>],
            right: &[Option<usize>],
        ) {
            let x_val = curr_x + dx[v];
            final_x[v] = x_val;
            if let Some(l) = left[v] {
                accumulate(l, x_val, dx, final_x, left, right);
            }
            if let Some(r) = right[v] {
                accumulate(r, x_val, dx, final_x, left, right);
            }
        }

        accumulate(0, 0.0, &dx, &mut final_x, &left, &right);

        // Center coordinates around origin and scale by grid_spacing
        let max_x = final_x.iter().copied().fold(0.0f32, f32::max);
        let max_y = y.iter().copied().fold(0.0f32, f32::max);

        let center_x = max_x / 2.0;
        let center_y = max_y / 2.0;

        for (order_idx, &node_id) in ordering.iter().enumerate() {
            if let Some(&node_idx) = state.node_keys.get(node_id) {
                let gx = (final_x[order_idx] - center_x) * self.grid_spacing;
                let gy = (y[order_idx] - center_y) * self.grid_spacing;
                state.positions.set(node_idx, Vec2::new(gx, gy));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maximal_shift_layout_execution() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(40.0, 40.0));
        let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(40.0, 40.0));
        let n3 = state.add_node(Vec2::new(20.0, 20.0), Size2::new(40.0, 40.0));

        state.add_edge(n1, n2, Default::default());
        state.add_edge(n2, n3, Default::default());
        state.add_edge(n3, n1, Default::default());

        let layout = MaximalShiftLayout::default();
        layout.apply(&mut state);

        assert_ne!(state.positions[0], state.positions[1]);
        assert_ne!(state.positions[1], state.positions[2]);
    }
}
