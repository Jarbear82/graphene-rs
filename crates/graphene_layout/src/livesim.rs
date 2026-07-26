use crate::quadtree::Quadtree;
use graphene_core::{math::Vec2, GraphState, HierarchyExt};
use std::cell::RefCell;
/// Live force-directed simulation that can be advanced frame-by-frame
///
/// This struct maintains internal state for the simulation and allows
/// incremental updates via the `tick()` method.
pub struct LiveForceSimulation {
    /// Simulation parameters
    pub k_rep: f32,
    pub k_att: f32,
    pub gravity: f32,
    pub ideal_length: f32,
    /// Temperature for simulated annealing
    pub temperature: f32,
    pub cooling_rate: f32,
    pub use_barnes_hut: bool,
    pub theta: f32,
}

impl LiveForceSimulation {
    /// Create a new live force simulation with default parameters
    pub fn new() -> Self {
        Self {
            k_rep: 2500.0,
            k_att: 0.06,
            gravity: 0.3,
            ideal_length: 50.0,
            temperature: 10.0,
            cooling_rate: 0.95,
            use_barnes_hut: true,
            theta: 0.5,
        }
    }

    /// Advance the simulation by one step
    pub fn tick(&mut self, state: &mut GraphState<impl Copy + Default>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        // Use RefCell pattern to get mutable references to positions
        // This is safe because we're not aliasing during the tick
        let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();

        let mut displacements = vec![Vec2::default(); n];

        // Barnes-Hut approximation for repulsion
        if self.use_barnes_hut && n > 100 {
            let quadtree = Quadtree::build(&positions);
            for i in 0..n {
                let pos_i = positions[i];
                let force =
                    quadtree.accumulate_repulsion(i, pos_i, &positions, self.k_rep, self.theta);
                displacements[i] += force;
            }
        } else {
            // Direct N-body calculation
            for i in 0..n {
                let pos_i = positions[i];
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let pos_j = positions[j];
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

        // Edge attractions
        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                let pos_u = positions[u];
                let pos_v = positions[v];
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

        // Gravity towards center
        let mut center = Vec2::default();
        for i in 0..n {
            center += positions[i];
        }
        center = center / n as f32;

        for i in 0..n {
            let pos = positions[i];
            let delta = center - pos;
            displacements[i] += delta * self.gravity;
        }

        // Apply displacements with temperature cap
        for i in 0..n {
            let disp = displacements[i];
            let disp_len = disp.len();
            if disp_len > 0.01 {
                let capped_disp = disp.normalize() * disp_len.min(self.temperature);
                let old_pos = *state.positions.get(i);
                state.positions.set(i, old_pos + capped_disp);
            }
        }

        // Apply collision resolution
        self.resolve_collisions(state, 2.0);

        // Cool down
        self.temperature *= self.cooling_rate;
    }

    /// Resolve node collisions to prevent overlap
    pub fn resolve_collisions(&self, state: &mut GraphState<impl Copy + Default>, padding: f32) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        // Get current positions and sizes
        let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();

        let sizes: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let size = *state.sizes.get(i);
                (size.w, size.h)
            })
            .collect();

        // Simple overlap resolution using hierarchy-aware collision detection
        for i in 0..n {
            let pos_i = positions[i];
            let (w_i, h_i) = sizes[i];

            for j in (i + 1)..n {
                let pos_j = positions[j];
                let (w_j, h_j) = sizes[j];

                // Check if nodes overlap
                let dx = pos_i.x - pos_j.x;
                let dy = pos_i.y - pos_j.y;
                let dist_sq = dx * dx + dy * dy;

                let min_dist_x = (w_i + w_j) / 2.0 + padding;
                let min_dist_y = (h_i + h_j) / 2.0 + padding;

                if dist_sq < min_dist_x * min_dist_x && dist_sq < min_dist_y * min_dist_y {
                    let dist = dist_sq.sqrt().max(0.1);
                    let overlap_x = min_dist_x - dist.abs();
                    let overlap_y = min_dist_y - dist.abs();

                    // Calculate repulsion direction
                    let fx = dx / dist * overlap_x;
                    let fy = dy / dist * overlap_y;

                    // Apply repulsion with hierarchy awareness
                    if let (Some(&idx_i), Some(&idx_j)) = (
                        state.node_keys.get(state.node_index_to_id[i]),
                        state.node_keys.get(state.node_index_to_id[j]),
                    ) {
                        // Check hierarchy relationship
                        let is_parent_child =
                            state.is_ancestor(idx_i, idx_j) || state.is_ancestor(idx_j, idx_i);

                        if is_parent_child {
                            // Parent-child: move child away from parent
                            if state.is_ancestor(idx_j, idx_i) {
                                // j is child of i - move j away
                                let curr_pos = *state.positions.get(idx_j);
                                state.positions.set(idx_j, curr_pos + Vec2::new(fx, fy));
                            } else if state.is_ancestor(idx_i, idx_j) {
                                // i is child of j - move i away
                                let curr_pos = *state.positions.get(idx_i);
                                state.positions.set(idx_i, curr_pos + Vec2::new(-fx, -fy));
                            }
                        } else {
                            // Independent nodes: repel both
                            let curr_pos_i = *state.positions.get(idx_i);
                            state.positions.set(idx_i, curr_pos_i + Vec2::new(fx, fy));

                            let curr_pos_j = *state.positions.get(idx_j);
                            state.positions.set(idx_j, curr_pos_j + Vec2::new(-fx, -fy));
                        }
                    } else {
                        // Fallback: move both nodes
                        let curr_pos_i = *state.positions.get(i);
                        state.positions.set(i, curr_pos_i + Vec2::new(fx, fy));

                        let curr_pos_j = *state.positions.get(j);
                        state.positions.set(j, curr_pos_j + Vec2::new(-fx, -fy));
                    }
                }
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

impl Default for LiveForceSimulation {
    fn default() -> Self {
        Self::new()
    }
}
