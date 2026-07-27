use crate::collision::resolve_overlaps;
use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState};

pub struct KamadaKawaiLayout {
    pub iterations: usize,
    pub k: f32,
    pub l_0: f32,
}

impl Default for KamadaKawaiLayout {
    fn default() -> Self {
        Self {
            iterations: 200,
            k: 1.0,
            l_0: 50.0,
        }
    }
}

impl KamadaKawaiLayout {
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_k(mut self, k: f32) -> Self {
        self.k = k;
        self
    }

    pub fn with_ideal_length(mut self, length: f32) -> Self {
        self.l_0 = length;
        self
    }
}

impl<S: Copy> Layout<S> for KamadaKawaiLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n <= 1 {
            return;
        }

        let mut d = vec![vec![f32::INFINITY; n]; n];
        for i in 0..n {
            d[i][i] = 0.0;
        }

        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                d[u][v] = 1.0;
                d[v][u] = 1.0;
            }
        }

        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if d[i][k] != f32::INFINITY && d[k][j] != f32::INFINITY {
                        let new_d = d[i][k] + d[k][j];
                        if new_d < d[i][j] {
                            d[i][j] = new_d;
                        }
                    }
                }
            }
        }

        let max_finite_dist = d
            .iter()
            .flatten()
            .filter(|&&x| x != f32::INFINITY)
            .copied()
            .fold(0.0f32, |m, x| m.max(x));
        let disconnect_dist = if max_finite_dist > 0.0 {
            max_finite_dist * 2.0
        } else {
            4.0
        };
        for i in 0..n {
            for j in 0..n {
                if d[i][j] == f32::INFINITY {
                    d[i][j] = disconnect_dist;
                }
            }
        }

        let mut l = vec![vec![0.0f32; n]; n];
        let mut k_matrix = vec![vec![0.0f32; n]; n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    l[i][j] = self.l_0 * d[i][j];
                    k_matrix[i][j] = self.k / (d[i][j] * d[i][j]);
                }
            }
        }

        for _step in 0..self.iterations {
            let mut grads_x = vec![0.0f32; n];
            let mut grads_y = vec![0.0f32; n];

            for i in 0..n {
                let pos_i = *state.positions.get(i);
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let pos_j = *state.positions.get(j);
                    let dx = pos_i.x - pos_j.x;
                    let dy = pos_i.y - pos_j.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.01);

                    let factor = k_matrix[i][j] * (1.0 - l[i][j] / dist);
                    grads_x[i] += factor * dx;
                    grads_y[i] += factor * dy;
                }
            }

            let learning_rate = 0.5f32;
            for i in 0..n {
                let old_pos = *state.positions.get(i);
                let new_x = old_pos.x - learning_rate * grads_x[i].clamp(-10.0, 10.0);
                let new_y = old_pos.y - learning_rate * grads_y[i].clamp(-10.0, 10.0);
                state.positions.set(i, Vec2::new(new_x, new_y));
            }
        }

        resolve_overlaps(state, 20.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct MdsLayout {
    pub iterations: usize,
    pub base_dist: f32,
}

impl Default for MdsLayout {
    fn default() -> Self {
        Self {
            iterations: 150,
            base_dist: 50.0,
        }
    }
}

impl MdsLayout {
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_base_dist(mut self, dist: f32) -> Self {
        self.base_dist = dist;
        self
    }
}

impl<S: Copy> Layout<S> for MdsLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n <= 1 {
            return;
        }

        let mut d = vec![vec![f32::INFINITY; n]; n];
        for i in 0..n {
            d[i][i] = 0.0;
        }
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                d[u][v] = 1.0;
                d[v][u] = 1.0;
            }
        }

        for k in 0..n {
            for i in 0..n {
                for j in 0..n {
                    if d[i][k] != f32::INFINITY && d[k][j] != f32::INFINITY {
                        let new_d = d[i][k] + d[k][j];
                        if new_d < d[i][j] {
                            d[i][j] = new_d;
                        }
                    }
                }
            }
        }

        let max_finite_dist = d
            .iter()
            .flatten()
            .filter(|&&x| x != f32::INFINITY)
            .copied()
            .fold(0.0f32, |m, x| m.max(x));
        let disconnect_dist = if max_finite_dist > 0.0 {
            max_finite_dist * 2.0
        } else {
            4.0
        };
        for i in 0..n {
            for j in 0..n {
                if d[i][j] == f32::INFINITY {
                    d[i][j] = disconnect_dist;
                }
            }
        }

        let mut delta = vec![vec![0.0f32; n]; n];
        for i in 0..n {
            for j in 0..n {
                delta[i][j] = d[i][j] * self.base_dist;
            }
        }

        let learning_rate = 0.1f32;
        for _step in 0..self.iterations {
            let mut grads_x = vec![0.0f32; n];
            let mut grads_y = vec![0.0f32; n];

            for i in 0..n {
                let pos_i = *state.positions.get(i);
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let pos_j = *state.positions.get(j);
                    let dx = pos_i.x - pos_j.x;
                    let dy = pos_i.y - pos_j.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(0.1);

                    let factor = 2.0 * (dist - delta[i][j]);
                    grads_x[i] += factor * (dx / dist);
                    grads_y[i] += factor * (dy / dist);
                }
            }

            for i in 0..n {
                let old_pos = *state.positions.get(i);
                let new_x = old_pos.x - learning_rate * grads_x[i].clamp(-10.0, 10.0);
                let new_y = old_pos.y - learning_rate * grads_y[i].clamp(-10.0, 10.0);
                state.positions.set(i, Vec2::new(new_x, new_y));
            }
        }

        resolve_overlaps(state, 20.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
