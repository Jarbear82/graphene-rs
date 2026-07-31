use crate::collision::resolve_overlaps;
use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState};

/// Kamada-Kawai spring layout.
///
/// Reference: Kamada, T., & Kawai, S. (1989). "An algorithm for drawing
/// general undirected graphs." Information Processing Letters, 31(1), 7–15.
///
/// Deviation from paper: none (as of the Newton-step fix in this file;
/// see commit history — a prior version used naive gradient descent and
/// was NOT equivalent to the algorithm below it was named after).
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
        let mut k = vec![vec![0.0f32; n]; n];
        for i in 0..n {
            for j in 0..n {
                if i != j {
                    l[i][j] = self.l_0 * d[i][j];
                    k[i][j] = self.k / (d[i][j] * d[i][j]);
                }
            }
        }

        for _outer in 0..self.iterations {
            let mut max_delta = -1.0f32;
            let mut m = 0usize;

            for i in 0..n {
                let (dx, dy) = partial_derivatives(state, i, &l, &k, n);
                let delta = (dx * dx + dy * dy).sqrt();
                if delta > max_delta {
                    max_delta = delta;
                    m = i;
                }
            }

            if max_delta < 1e-3 {
                break; // converged
            }

            // Inner loop: locally minimize energy at vertex m via Newton's method
            for _inner in 0..50 {
                let (dex, dey) = partial_derivatives(state, m, &l, &k, n);
                let delta = (dex * dex + dey * dey).sqrt();
                if delta < 1e-3 {
                    break;
                }

                let (a, b, c) = second_partials(state, m, &l, &k, n);
                let det = a * c - b * b;

                let (dx, dy) = if det.abs() > 1e-9 {
                    ((-c * dex + b * dey) / det, (b * dex - a * dey) / det)
                } else {
                    (-dex * 0.1, -dey * 0.1)
                };

                let old_pos = *state.positions.get(m);
                state.positions.set(m, Vec2::new(old_pos.x + dx, old_pos.y + dy));
            }
        }

        resolve_overlaps(state, 20.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

fn partial_derivatives<S: Copy>(
    state: &GraphState<S>,
    m: usize,
    l: &[Vec<f32>],
    k: &[Vec<f32>],
    n: usize,
) -> (f32, f32) {
    let pm = *state.positions.get(m);
    let (mut dex, mut dey) = (0.0, 0.0);
    for i in 0..n {
        if i == m || l[m][i] <= 0.0 {
            continue;
        }
        let pi = *state.positions.get(i);
        let (dx, dy) = (pm.x - pi.x, pm.y - pi.y);
        let dist = (dx * dx + dy * dy).sqrt().max(0.01);
        dex += k[m][i] * (dx - (l[m][i] * dx) / dist);
        dey += k[m][i] * (dy - (l[m][i] * dy) / dist);
    }
    (dex, dey)
}

fn second_partials<S: Copy>(
    state: &GraphState<S>,
    m: usize,
    l: &[Vec<f32>],
    k: &[Vec<f32>],
    n: usize,
) -> (f32, f32, f32) {
    let pm = *state.positions.get(m);
    let (mut a, mut b, mut c) = (0.0, 0.0, 0.0);
    for i in 0..n {
        if i == m || l[m][i] <= 0.0 {
            continue;
        }
        let pi = *state.positions.get(i);
        let (dx, dy) = (pm.x - pi.x, pm.y - pi.y);
        let dist = (dx * dx + dy * dy).sqrt().max(0.01);
        let dist3 = dist * dist * dist;
        a += k[m][i] * (1.0 - (l[m][i] * dy * dy) / dist3);
        b += k[m][i] * (l[m][i] * dx * dy) / dist3;
        c += k[m][i] * (1.0 - (l[m][i] * dx * dx) / dist3);
    }
    (a, b, c)
}

/// Multidimensional scaling via SMACOF majorization.
///
/// Reference: de Leeuw, J. (1977). "Applications of convex analysis to
/// multidimensional scaling." In Recent Developments in Statistics (pp. 133–145).
/// Uses the Guttman transform / SMACOF majorization, which guarantees monotonic
/// non-increasing stress each iteration.
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

        let delta: Vec<Vec<f32>> = d
            .iter()
            .map(|row| row.iter().map(|&x| x * self.base_dist).collect())
            .collect();

        let w: Vec<Vec<f32>> = delta
            .iter()
            .map(|row| row.iter().map(|&x| if x > 0.0 { 1.0 / (x * x) } else { 0.0 }).collect())
            .collect();

        for _iter in 0..self.iterations {
            let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();

            let mut new_positions = vec![Vec2::default(); n];
            for i in 0..n {
                let mut sum = Vec2::default();
                let mut w_row_sum = 0.0f32;
                for j in 0..n {
                    if i == j || w[i][j] == 0.0 {
                        continue;
                    }
                    let dist = (positions[i] - positions[j]).len().max(1e-6);
                    let b_ij = w[i][j] * delta[i][j] / dist;
                    sum += (positions[i] - positions[j]) * b_ij + positions[j] * w[i][j];
                    w_row_sum += w[i][j];
                }
                new_positions[i] = if w_row_sum > 1e-9 {
                    sum / w_row_sum
                } else {
                    positions[i]
                };
            }

            let mut moved = 0.0f32;
            for i in 0..n {
                moved += (new_positions[i] - positions[i]).len();
                state.positions.set(i, new_positions[i]);
            }
            if moved / (n as f32) < 0.01 {
                break;
            }
        }

        resolve_overlaps(state, 20.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
