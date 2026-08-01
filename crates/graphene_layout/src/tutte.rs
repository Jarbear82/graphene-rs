// graphene_layout/src/tutte.rs
//
// Ported from Doc 2's `force_directed::tutte`, verified against
// Tutte (1963), "How to Draw a Graph". Useful for planar-safe placement
// of a compound node's boundary/outer face before running a spring layout
// on its interior.

use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState, NodeId};
use std::f32::consts::TAU;

/// Tutte's barycentric embedding algorithm for planar graphs.
///
/// Reference: Tutte, W. T. (1963). "How to Draw a Graph."
/// Proceedings of the London Mathematical Society, 3(13), 743–768.
///
/// Complexity: O(iterations * V * max_deg).
pub struct TutteBarycentricLayout {
    pub fixed_boundary: Vec<NodeId>, // must form the convex outer face, in order
    pub polygon_radius: f32,
    pub max_iterations: usize,
    pub tol: f32,
}

impl Default for TutteBarycentricLayout {
    fn default() -> Self {
        Self {
            fixed_boundary: Vec::new(),
            polygon_radius: 200.0,
            max_iterations: 200,
            tol: 1e-3,
        }
    }
}

impl TutteBarycentricLayout {
    pub fn with_fixed_boundary(mut self, boundary: Vec<NodeId>) -> Self {
        self.fixed_boundary = boundary;
        self
    }

    pub fn with_polygon_radius(mut self, radius: f32) -> Self {
        self.polygon_radius = radius;
        self
    }

    pub fn with_max_iterations(mut self, iterations: usize) -> Self {
        self.max_iterations = iterations;
        self
    }

    pub fn with_tol(mut self, tol: f32) -> Self {
        self.tol = tol;
        self
    }
}

impl<S: Copy> Layout<S> for TutteBarycentricLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        let k = self.fixed_boundary.len();
        if n == 0 || k < 3 {
            return;
        }

        let mut max_extent = 0.0f32;
        for &id in &self.fixed_boundary {
            if let Some(&idx) = state.node_keys.get(id) {
                let size = *state.sizes.get(idx);
                max_extent = max_extent.max(size.w.max(size.h));
            }
        }
        let req_circ = k as f32 * (max_extent + 10.0);
        let eff_radius = self.polygon_radius.max(req_circ / (2.0 * std::f32::consts::PI));

        let mut is_fixed = vec![false; n];
        for (i, &id) in self.fixed_boundary.iter().enumerate() {
            let Some(&idx) = state.node_keys.get(id) else {
                continue;
            };
            let angle = (i as f32 / k as f32) * TAU;
            let pos = Vec2::new(
                eff_radius * angle.cos(),
                eff_radius * angle.sin(),
            );
            state.positions.set(idx, pos);
            is_fixed[idx] = true;
        }

        // Build adjacency once.
        let mut adj = vec![Vec::new(); n];
        for i in 0..state.edges.len() {
            let (Some(&u), Some(&v)) = (
                state.node_keys.get(*state.edge_sources.get(i)),
                state.node_keys.get(*state.edge_targets.get(i)),
            ) else {
                continue;
            };
            adj[u].push(v);
            adj[v].push(u);
        }

        for _iter in 0..self.max_iterations {
            let mut max_change = 0.0f32;
            for v in 0..n {
                if is_fixed[v] || adj[v].is_empty() {
                    continue;
                }
                let mut sum = Vec2::default();
                for &u in &adj[v] {
                    sum += *state.positions.get(u);
                }
                let target = sum / adj[v].len() as f32;
                let old = *state.positions.get(v);
                max_change = max_change.max((target - old).len());
                state.positions.set(v, target);
            }
            if max_change < self.tol {
                break;
            }
        }
        let collapsed = std::collections::HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}
