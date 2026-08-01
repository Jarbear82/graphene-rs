// graphene_layout/src/fruchterman_reingold.rs
//
// Ported from Doc 2's `force_directed::fruchterman_reingold`, verified against
// Fruchterman & Reingold (1991), "Graph Drawing by Force-Directed Placement".

use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState};

/// Fruchterman-Reingold force-directed layout algorithm.
///
/// Reference: Fruchterman, T. M. J., & Reingold, E. M. (1991).
/// "Graph Drawing by Force-Directed Placement." Software: Practice and Experience, 21(11), 1129–1164.
///
/// Complexity: O(V² + E) per iteration.
pub struct FruchtermanReingoldLayout {
    pub width: f32,
    pub length: f32,
    pub iterations: usize,
    pub initial_temp: f32,
}

impl Default for FruchtermanReingoldLayout {
    fn default() -> Self {
        Self {
            width: 1000.0,
            length: 1000.0,
            iterations: 100,
            initial_temp: 100.0,
        }
    }
}

impl FruchtermanReingoldLayout {
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_length(mut self, length: f32) -> Self {
        self.length = length;
        self
    }

    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_initial_temp(mut self, temp: f32) -> Self {
        self.initial_temp = temp;
        self
    }
}

impl<S: Copy> Layout<S> for FruchtermanReingoldLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let area = self.width * self.length;
        let k = (area / n as f32).sqrt();
        let fa = |x: f32| (x * x) / k;
        let fr = |x: f32| (k * k) / x.max(1e-4);

        let mut temp = self.initial_temp;
        let temp_step = self.initial_temp / self.iterations.max(1) as f32;

        for _iter in 0..self.iterations {
            let mut disp = vec![Vec2::default(); n];

            for v in 0..n {
                let pv = *state.positions.get(v);
                let sv = *state.sizes.get(v);
                for u in 0..n {
                    if u == v {
                        continue;
                    }
                    let pu = *state.positions.get(u);
                    let su = *state.sizes.get(u);
                    let delta = pv - pu;
                    let dist = delta.len().max(1e-4);
                    let min_extent = (sv.w + su.w).max(sv.h + su.h) * 0.5;
                    let eff_dist = dist.max(min_extent * 0.5);
                    let f = fr(eff_dist);
                    disp[v] += delta.normalize() * f;
                }
            }

            for i in 0..state.edges.len() {
                let (Some(&v), Some(&u)) = (
                    state.node_keys.get(*state.edge_sources.get(i)),
                    state.node_keys.get(*state.edge_targets.get(i)),
                ) else {
                    continue;
                };
                let (pv, pu) = (*state.positions.get(v), *state.positions.get(u));
                let (sv, su) = (*state.sizes.get(v), *state.sizes.get(u));
                let delta = pv - pu;
                let dist = delta.len().max(1e-4);
                let ideal_k = crate::geometry::size_aware_ideal_length(k, sv, su, delta);
                let f = fa(dist) * (ideal_k / k);
                disp[v] -= delta.normalize() * f;
                disp[u] += delta.normalize() * f;
            }

            let (half_w, half_l) = (self.width / 2.0, self.length / 2.0);
            for v in 0..n {
                let len = disp[v].len().max(1e-4);
                let step = len.min(temp);
                let old = *state.positions.get(v);
                let mut new_pos = old + disp[v].normalize() * step;
                new_pos.x = new_pos.x.clamp(-half_w, half_w);
                new_pos.y = new_pos.y.clamp(-half_l, half_l);
                state.positions.set(v, new_pos);
            }

            temp = (temp - temp_step).max(0.01);
        }
        let collapsed = std::collections::HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}
