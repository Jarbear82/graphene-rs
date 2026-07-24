use crate::quadtree::Quadtree;
use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState};

pub struct ForceDirectedLayout {
    pub iterations: usize,
    pub ideal_length: f32,
    pub gravity: f32,
    pub k_rep: f32,
    pub k_att: f32,
    pub initial_temp: f32,
    pub use_barnes_hut: bool,
    pub theta: f32,
}

impl Default for ForceDirectedLayout {
    fn default() -> Self {
        Self {
            iterations: 150,
            ideal_length: 50.0,
            gravity: 0.1,
            k_rep: 2000.0,
            k_att: 0.05,
            initial_temp: 10.0,
            use_barnes_hut: false,
            theta: 0.5,
        }
    }
}

impl<S: Copy + Default> Layout<S> for ForceDirectedLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let mut displacements = vec![Vec2::default(); n];
        let mut temp = self.initial_temp;

        for _iter in 0..self.iterations {
            displacements.fill(Vec2::default());

            let use_bh = self.use_barnes_hut || n > 100;
            if use_bh {
                let quadtree = Quadtree::build(&state.positions);
                for i in 0..n {
                    let pos_i = *state.positions.get(i);
                    let force = quadtree.accumulate_repulsion(i, pos_i, &state.positions, self.k_rep, self.theta);
                    displacements[i] += force;
                }
            } else {
                for i in 0..n {
                    let pos_i = *state.positions.get(i);
                    for j in 0..n {
                        if i == j {
                            continue;
                        }
                        let pos_j = *state.positions.get(j);
                        let delta = pos_i - pos_j;
                        let dist = delta.len();
                        if dist > 0.1 {
                            let force = self.k_rep / (dist * dist);
                            let dir = delta.normalize();
                            displacements[i] += dir * force;
                        }
                    }
                }
            }

            for i in 0..state.edges.len() {
                let src = *state.edge_sources.get(i);
                let tgt = *state.edge_targets.get(i);
                if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                    let pos_u = *state.positions.get(u);
                    let pos_v = *state.positions.get(v);
                    let delta = pos_v - pos_u;
                    let dist = delta.len();
                    if dist > 0.1 {
                        let force = self.k_att * (dist - self.ideal_length);
                        let dir = delta.normalize();
                        displacements[u] += dir * force;
                        displacements[v] -= dir * force;
                    }
                }
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
                    let capped_disp = disp.normalize() * disp_len.min(temp);
                    let old_pos = *state.positions.get(i);
                    state.positions.set(i, old_pos + capped_disp);
                }
            }

            temp *= 0.95;
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
