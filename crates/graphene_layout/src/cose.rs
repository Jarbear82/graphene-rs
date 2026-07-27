use crate::traits::{get_nesting_depth, Layout};
use graphene_core::{math::Vec2, GraphState, NodeId};

pub struct CoseLayout {
    pub iterations: usize,
    pub ideal_edge_length: f32,
    pub edge_elasticity: f32,
    pub nesting_factor: f32,
    pub gravity: f32,
    pub node_repulsion: f32,
    pub node_overlap: f32,
    pub initial_temp: f32,
    pub cooling_factor: f32,
    pub min_temp: f32,
}

impl Default for CoseLayout {
    fn default() -> Self {
        Self {
            iterations: 1000,
            ideal_edge_length: 32.0,
            edge_elasticity: 32.0,
            nesting_factor: 1.2,
            gravity: 1.0,
            node_repulsion: 2048.0,
            node_overlap: 4.0,
            initial_temp: 1000.0,
            cooling_factor: 0.99,
            min_temp: 1.0,
        }
    }
}

impl CoseLayout {
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_ideal_edge_length(mut self, length: f32) -> Self {
        self.ideal_edge_length = length;
        self
    }

    pub fn with_edge_elasticity(mut self, elasticity: f32) -> Self {
        self.edge_elasticity = elasticity;
        self
    }

    pub fn with_nesting_factor(mut self, factor: f32) -> Self {
        self.nesting_factor = factor;
        self
    }

    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_node_repulsion(mut self, repulsion: f32) -> Self {
        self.node_repulsion = repulsion;
        self
    }

    pub fn with_node_overlap(mut self, overlap: f32) -> Self {
        self.node_overlap = overlap;
        self
    }

    pub fn with_initial_temp(mut self, temp: f32) -> Self {
        self.initial_temp = temp;
        self
    }

    pub fn with_cooling_factor(mut self, factor: f32) -> Self {
        self.cooling_factor = factor;
        self
    }

    pub fn with_min_temp(mut self, temp: f32) -> Self {
        self.min_temp = temp;
        self
    }
}

pub fn find_clipping_point(pos: Vec2, size: graphene_core::Size2, dx: f32, dy: f32) -> Vec2 {
    let w = size.w;
    let h = size.h;
    if dx == 0.0 && dy > 0.0 {
        return Vec2::new(pos.x, pos.y + h / 2.0);
    }
    if dx == 0.0 && dy < 0.0 {
        return Vec2::new(pos.x, pos.y - h / 2.0);
    }
    let dir_slope = dy / dx;
    let node_slope = h / w;

    if dx > 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(pos.x + w / 2.0, pos.y + (w * dy / (2.0 * dx)));
    }
    if dx < 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(pos.x - w / 2.0, pos.y - (w * dy / (2.0 * dx)));
    }
    if dy > 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(pos.x + (h * dx / (2.0 * dy)), pos.y + h / 2.0);
    }
    if dy < 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(pos.x - (h * dx / (2.0 * dy)), pos.y - h / 2.0);
    }

    pos
}



impl<S: Copy> Layout<S> for CoseLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let mut temp = self.initial_temp;
        let mut state_lcg = 42u64;
        let mut random_distance = || {
            state_lcg = state_lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let r = (state_lcg >> 32) as f32 / u32::MAX as f32;
            -1.0 + 2.0 * r
        };

        for _step in 0..self.iterations {
            if temp < self.min_temp {
                break;
            }

            let mut displacements_x = vec![0.0f32; n];
            let mut displacements_y = vec![0.0f32; n];

            if n > 100 {
                let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();
                let quadtree = crate::quadtree::Quadtree::build(&positions);
                for i in 0..n {
                    let pos_i = positions[i];
                    let force = quadtree.accumulate_repulsion(i, pos_i, &positions, self.node_repulsion, 0.5);
                    displacements_x[i] += force.x;
                    displacements_y[i] += force.y;
                }
            } else {
                for i in 0..n {
                    let pos_i = *state.positions.get(i);
                    let size_i = *state.sizes.get(i);
                    let min_x_i = pos_i.x - size_i.w / 2.0;
                    let max_x_i = pos_i.x + size_i.w / 2.0;
                    let min_y_i = pos_i.y - size_i.h / 2.0;
                    let max_y_i = pos_i.y + size_i.h / 2.0;

                    for j in (i + 1)..n {
                        let pos_j = *state.positions.get(j);
                        let size_j = *state.sizes.get(j);
                        let min_x_j = pos_j.x - size_j.w / 2.0;
                        let max_x_j = pos_j.x + size_j.w / 2.0;
                        let min_y_j = pos_j.y - size_j.h / 2.0;
                        let max_y_j = pos_j.y + size_j.h / 2.0;

                        let mut dir_x = pos_j.x - pos_i.x;
                        let mut dir_y = pos_j.y - pos_i.y;

                        if dir_x == 0.0 && dir_y == 0.0 {
                            dir_x = random_distance();
                            dir_y = random_distance();
                        }

                        let overlap_x = if dir_x > 0.0 { max_x_i - min_x_j } else { max_x_j - min_x_i };
                        let overlap_y = if dir_y > 0.0 { max_y_i - min_y_j } else { max_y_j - min_y_i };

                        if overlap_x >= 0.0 && overlap_y >= 0.0 {
                            let overlap = (overlap_x * overlap_x + overlap_y * overlap_y).sqrt();
                            let force = self.node_overlap * overlap;
                            let dist = (dir_x * dir_x + dir_y * dir_y).sqrt().max(0.01);
                            let fx = force * dir_x / dist;
                            let fy = force * dir_y / dist;

                            displacements_x[i] -= fx;
                            displacements_y[i] -= fy;
                            displacements_x[j] += fx;
                            displacements_y[j] += fy;
                        } else {
                            let p1 = find_clipping_point(pos_i, size_i, dir_x, dir_y);
                            let p2 = find_clipping_point(pos_j, size_j, -dir_x, -dir_y);

                            let dx = p2.x - p1.x;
                            let dy = p2.y - p1.y;
                            let dist_sqr = (dx * dx + dy * dy).max(0.01);
                            let dist = dist_sqr.sqrt();

                            let force = (self.node_repulsion + self.node_repulsion) / dist_sqr;
                            let fx = force * dx / dist;
                            let fy = force * dy / dist;

                            displacements_x[i] -= fx;
                            displacements_y[i] -= fy;
                            displacements_x[j] += fx;
                            displacements_y[j] += fy;
                        }
                    }
                }
            }

            for idx in 0..state.edges.len() {
                let src_node = *state.edge_sources.get(idx);
                let tgt_node = *state.edge_targets.get(idx);
                let Some(&src_idx) = state.node_keys.get(src_node) else { continue };
                let Some(&tgt_idx) = state.node_keys.get(tgt_node) else { continue };

                if src_idx == tgt_idx {
                    continue;
                }

                let pos_src = *state.positions.get(src_idx);
                let pos_tgt = *state.positions.get(tgt_idx);
                let size_src = *state.sizes.get(src_idx);
                let size_tgt = *state.sizes.get(tgt_idx);

                let dir_x = pos_tgt.x - pos_src.x;
                let dir_y = pos_tgt.y - pos_src.y;

                if dir_x == 0.0 && dir_y == 0.0 {
                    continue;
                }

                let p1 = find_clipping_point(pos_src, size_src, dir_x, dir_y);
                let p2 = find_clipping_point(pos_tgt, size_tgt, -dir_x, -dir_y);

                let lx = p2.x - p1.x;
                let ly = p2.y - p1.y;
                let l = (lx * lx + ly * ly).sqrt().max(0.01);

                let depth = get_nesting_depth(state, src_node, tgt_node);
                let ideal = self.ideal_edge_length * self.nesting_factor.powi(depth as i32);

                let force = (ideal - l).powi(2) / self.edge_elasticity;
                let fx = force * lx / l;
                let fy = force * ly / l;

                displacements_x[src_idx] += fx;
                displacements_y[src_idx] += fy;
                displacements_x[tgt_idx] -= fx;
                displacements_y[tgt_idx] -= fy;
            }

            let mut center = Vec2::default();
            for i in 0..n {
                center += *state.positions.get(i);
            }
            center = center / n as f32;

            for i in 0..n {
                let pos = *state.positions.get(i);
                let dx = center.x - pos.x;
                let dy = center.y - pos.y;
                let d = (dx * dx + dy * dy).sqrt().max(0.01);
                let fx = self.gravity * dx / d;
                let fy = self.gravity * dy / d;

                displacements_x[i] += fx;
                displacements_y[i] += fy;
            }

            for i in 0..n {
                let dx = displacements_x[i];
                let dy = displacements_y[i];
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 0.01 {
                    let cap = dist.min(temp);
                    let capped_x = dx * cap / dist;
                    let capped_y = dy * cap / dist;

                    let old_pos = *state.positions.get(i);
                    state.positions.set(i, Vec2::new(old_pos.x + capped_x, old_pos.y + capped_y));
                }
            }

            temp *= self.cooling_factor;
        }

        crate::collision::resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
