use crate::traits::Layout;
use graphene_core::{math::Vec2, EdgeId, GraphState};

pub struct CollisionForceDirectedLayout {
    pub iterations: usize,
    pub gravity: f32,
    pub ideal_length: f32,
}

impl Default for CollisionForceDirectedLayout {
    fn default() -> Self {
        Self {
            iterations: 200,
            gravity: 1.0,
            ideal_length: 50.0,
        }
    }
}

impl<S: Copy> Layout<S> for CollisionForceDirectedLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut temp = 100.0f32;
        let mut state_lcg = 12345u64;
        let mut next_rand = || {
            state_lcg = state_lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let r = (state_lcg >> 32) as f32 / u32::MAX as f32;
            -1.0 + 2.0 * r
        };

        for _step in 0..self.iterations {
            let mut displacements = vec![Vec2::default(); n];

            for i in 0..n {
                let pos_i = *state.positions.get(i);
                let size_i = *state.sizes.get(i);
                let r_i = size_i.w.max(size_i.h) / 2.0;

                for j in (i + 1)..n {
                    let pos_j = *state.positions.get(j);
                    let size_j = *state.sizes.get(j);
                    let r_j = size_j.w.max(size_j.h) / 2.0;

                    let mut delta = pos_i - pos_j;
                    if delta.x == 0.0 && delta.y == 0.0 {
                        delta = Vec2::new(next_rand(), next_rand());
                    }
                    let dist = delta.len().max(0.01);

                    let min_dist = r_i + r_j;
                    let force = if dist < min_dist {
                        10.0 * (min_dist - dist)
                    } else {
                        (self.ideal_length * self.ideal_length) / dist
                    };

                    let disp = delta.normalize() * force;
                    displacements[i] += disp;
                    displacements[j] -= disp;
                }
            }

            for idx in 0..state.edges.len() {
                let src_node = *state.edge_sources.get(idx);
                let tgt_node = *state.edge_targets.get(idx);
                let Some(&src_idx) = state.node_keys.get(src_node) else { continue };
                let Some(&tgt_idx) = state.node_keys.get(tgt_node) else { continue };

                if src_idx == tgt_idx { continue; }

                let pos_src = *state.positions.get(src_idx);
                let pos_tgt = *state.positions.get(tgt_idx);
                let delta = pos_tgt - pos_src;
                let dist = delta.len().max(0.01);

                let force = (dist * dist) / self.ideal_length;
                let disp = delta.normalize() * force;

                displacements[src_idx] += disp;
                displacements[tgt_idx] -= disp;
            }

            let mut center = Vec2::default();
            for i in 0..n {
                center += *state.positions.get(i);
            }
            center = center / n as f32;
            for i in 0..n {
                let pos = *state.positions.get(i);
                let delta = center - pos;
                displacements[i] += delta * self.gravity;
            }

            for i in 0..n {
                let disp = displacements[i];
                let disp_len = disp.len();
                if disp_len > 0.01 {
                    let cap = disp.normalize() * disp_len.min(temp);
                    let old_pos = *state.positions.get(i);
                    state.positions.set(i, old_pos + cap);
                }
            }

            temp *= 0.95;
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct WeightedForceDirectedLayout<W> {
    pub iterations: usize,
    pub gravity: f32,
    pub k_rep: f32,
    pub k_att: f32,
    pub weight_fn: W,
}

impl<S: Copy, W: Fn(EdgeId) -> f32> Layout<S> for WeightedForceDirectedLayout<W> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut temp = 100.0f32;
        let mut state_lcg = 42u64;
        let mut next_random = || {
            state_lcg = state_lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let r = (state_lcg >> 32) as f32 / u32::MAX as f32;
            -1.0 + 2.0 * r
        };

        for _step in 0..self.iterations {
            let mut displacements = vec![Vec2::default(); n];

            for i in 0..n {
                let pos_i = *state.positions.get(i);
                for j in (i + 1)..n {
                    let pos_j = *state.positions.get(j);
                    let mut delta = pos_i - pos_j;
                    if delta.x == 0.0 && delta.y == 0.0 {
                        delta = Vec2::new(next_random(), next_random());
                    }
                    let dist = delta.len().max(0.01);
                    let force = (self.k_rep * self.k_rep) / dist;
                    let disp = delta.normalize() * force;

                    displacements[i] += disp;
                    displacements[j] -= disp;
                }
            }

            for idx in 0..state.edges.len() {
                let src_node = *state.edge_sources.get(idx);
                let tgt_node = *state.edge_targets.get(idx);
                let edge_id = state.edge_index_to_id[idx];

                let Some(&src_idx) = state.node_keys.get(src_node) else { continue };
                let Some(&tgt_idx) = state.node_keys.get(tgt_node) else { continue };

                if src_idx == tgt_idx { continue; }

                let pos_src = *state.positions.get(src_idx);
                let pos_tgt = *state.positions.get(tgt_idx);
                let delta = pos_tgt - pos_src;
                let dist = delta.len().max(0.01);

                let weight = (self.weight_fn)(edge_id);
                let force = (dist * dist) / self.k_att * weight;
                let disp = delta.normalize() * force;

                displacements[src_idx] += disp;
                displacements[tgt_idx] -= disp;
            }

            let mut center = Vec2::default();
            for i in 0..n {
                center += *state.positions.get(i);
            }
            center = center / n as f32;
            for i in 0..n {
                let pos = *state.positions.get(i);
                let delta = center - pos;
                displacements[i] += delta * self.gravity;
            }

            for i in 0..n {
                let disp = displacements[i];
                let disp_len = disp.len();
                if disp_len > 0.01 {
                    let cap = disp.normalize() * disp_len.min(temp);
                    let old_pos = *state.positions.get(i);
                    state.positions.set(i, old_pos + cap);
                }
            }

            temp *= 0.95;
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
