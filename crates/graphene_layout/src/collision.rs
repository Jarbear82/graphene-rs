use crate::traits::Layout;
use graphene_core::{math::Vec2, EdgeId, GraphState, HierarchyExt};

/// Force-directed layout with AABB collision resolution.
///
/// Reference: Force-directed layout with AABB node bounds collision resolution.
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

impl CollisionForceDirectedLayout {
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_ideal_length(mut self, length: f32) -> Self {
        self.ideal_length = length;
        self
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

        resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

/// Force-directed layout with edge-weight scaled spring constants.
///
/// Reference: Force-directed layout with edge-weight scaled spring constants.
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

/// Separates overlapping node AABBs in graph space so that no two non-hierarchical nodes overlap,
/// enforcing a minimum margin of `padding` between physical node bounding boxes.
pub fn resolve_overlaps<S: Copy>(state: &mut GraphState<S>, padding: f32) {
    let n = state.node_index_to_id.len();
    if n <= 1 {
        return;
    }

    let is_hierarchical_pair = |state: &GraphState<S>, i: usize, j: usize| -> bool {
        state.is_ancestor(i, j) || state.is_ancestor(j, i)
    };

    let mut state_lcg = 987654321u64;
    let mut next_rand = || {
        state_lcg = state_lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = (state_lcg >> 32) as f32 / u32::MAX as f32;
        -0.5 + r
    };

    // Phase 1: Adaptive Uniform Expansion for tightly packed graphs
    let mut max_scale = 1.0f32;
    if n > 50 {
        let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();
        let quadtree = crate::quadtree::Quadtree::build(&positions);
        let mut candidates = Vec::with_capacity(32);

        for i in 0..n {
            let pos_i = positions[i];
            let size_i = *state.sizes.get(i);
            let hw_i = size_i.w / 2.0;
            let hh_i = size_i.h / 2.0;

            candidates.clear();
            quadtree.query_overlapping_candidates(pos_i, hw_i, hh_i, padding, &mut candidates);

            for &j in &candidates {
                if j <= i || is_hierarchical_pair(state, i, j) { continue; }

                let pos_j = positions[j];
                let size_j = *state.sizes.get(j);
                let hw_j = size_j.w / 2.0;
                let hh_j = size_j.h / 2.0;

                let min_dist_x = hw_i + hw_j + padding;
                let min_dist_y = hh_i + hh_j + padding;

                let dx = (pos_i.x - pos_j.x).abs();
                let dy = (pos_i.y - pos_j.y).abs();

                if dx < min_dist_x && dy < min_dist_y {
                    let sx = if dx > 0.1 { min_dist_x / dx } else { 2.0 };
                    let sy = if dy > 0.1 { min_dist_y / dy } else { 2.0 };
                    let s = sx.min(sy);
                    if s > max_scale && s < 10.0 {
                        max_scale = s;
                    }
                }
            }
        }
    } else {
        for i in 0..n {
            let pos_i = *state.positions.get(i);
            let size_i = *state.sizes.get(i);
            let hw_i = size_i.w / 2.0;
            let hh_i = size_i.h / 2.0;

            for j in (i + 1)..n {
                if is_hierarchical_pair(state, i, j) { continue; }

                let pos_j = *state.positions.get(j);
                let size_j = *state.sizes.get(j);
                let hw_j = size_j.w / 2.0;
                let hh_j = size_j.h / 2.0;

                let min_dist_x = hw_i + hw_j + padding;
                let min_dist_y = hh_i + hh_j + padding;

                let dx = (pos_i.x - pos_j.x).abs();
                let dy = (pos_i.y - pos_j.y).abs();

                if dx < min_dist_x && dy < min_dist_y {
                    let sx = if dx > 0.1 { min_dist_x / dx } else { 2.0 };
                    let sy = if dy > 0.1 { min_dist_y / dy } else { 2.0 };
                    let s = sx.min(sy);
                    if s > max_scale && s < 10.0 {
                        max_scale = s;
                    }
                }
            }
        }
    }

    if max_scale > 1.05 {
        for i in 0..n {
            let pos = *state.positions.get(i);
            state.positions.set(i, pos * max_scale);
        }
    }

    // Phase 2: Minimum Translation Vector (MTV) Overlap Resolution with Quadtree acceleration
    let max_iterations = if n > 500 { 50 } else { 200 };
    let mut candidates = Vec::with_capacity(32);
    let mut pos_buf = if n > 50 {
        Some(vec![Vec2::default(); n])
    } else {
        None
    };

    for _iter in 0..max_iterations {
        let mut overlap_found = false;

        let quadtree = if let Some(ref mut pos) = pos_buf {
            for i in 0..n {
                pos[i] = *state.positions.get(i);
            }
            Some(crate::quadtree::Quadtree::build(pos))
        } else {
            None
        };

        for i in 0..n {
            let pos_i = *state.positions.get(i);
            let size_i = *state.sizes.get(i);
            let hw_i = size_i.w / 2.0;
            let hh_i = size_i.h / 2.0;

            candidates.clear();
            let use_quadtree = quadtree.is_some() && _iter < max_iterations - 5;
            if use_quadtree {
                if let Some(ref qt) = quadtree {
                    qt.query_overlapping_candidates(pos_i, hw_i, hh_i, padding + 30.0, &mut candidates);
                }
            }

            let mut process_pair = |state: &mut GraphState<S>, j: usize, overlap_found: &mut bool| {
                if j <= i || is_hierarchical_pair(state, i, j) { return; }

                let pos_i = *state.positions.get(i);
                let pos_j = *state.positions.get(j);
                let size_j = *state.sizes.get(j);
                let hw_j = size_j.w / 2.0;
                let hh_j = size_j.h / 2.0;

                let min_dist_x = hw_i + hw_j + padding;
                let min_dist_y = hh_i + hh_j + padding;

                let mut dx = pos_i.x - pos_j.x;
                let mut dy = pos_i.y - pos_j.y;

                if dx.abs() < 0.001 && dy.abs() < 0.001 {
                    dx = next_rand() * 0.1;
                    dy = next_rand() * 0.1;
                }

                let overlap_x = min_dist_x - dx.abs();
                let overlap_y = min_dist_y - dy.abs();

                if overlap_x > 0.0 && overlap_y > 0.0 {
                    *overlap_found = true;

                    let (shift_x, shift_y) = if overlap_x < overlap_y {
                        let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                        (sign_x * (overlap_x / 2.0 + 0.01), 0.0)
                    } else {
                        let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                        (0.0, sign_y * (overlap_y / 2.0 + 0.01))
                    };

                    let new_pos_i = Vec2::new(pos_i.x + shift_x, pos_i.y + shift_y);
                    let new_pos_j = Vec2::new(pos_j.x - shift_x, pos_j.y - shift_y);

                    state.positions.set(i, new_pos_i);
                    state.positions.set(j, new_pos_j);
                }
            };

            if use_quadtree {
                for &j in &candidates {
                    process_pair(state, j, &mut overlap_found);
                }
            } else {
                for j in (i + 1)..n {
                    process_pair(state, j, &mut overlap_found);
                }
            }
        }

        if !overlap_found {
            break;
        }
    }

    state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
}

/// Translates the layout centroid of all nodes to origin (0, 0).
pub fn center_layout_at_origin<S: Copy>(state: &mut GraphState<S>) {
    let n = state.node_index_to_id.len();
    if n == 0 {
        return;
    }
    let mut sum_x = 0.0f32;
    let mut sum_y = 0.0f32;
    for i in 0..n {
        let pos = state.positions.get(i);
        sum_x += pos.x;
        sum_y += pos.y;
    }
    let cx = sum_x / n as f32;
    let cy = sum_y / n as f32;
    for i in 0..n {
        let pos = state.positions.get_mut(i);
        pos.x -= cx;
        pos.y -= cy;
    }
}

/// Universal size-aware post-processing epilogue for all layout algorithms.
/// Centers layout at origin, enforces AABB overlap resolution, and updates compound parent bounds.
pub fn finish_layout_epilogue<S: Copy>(
    state: &mut GraphState<S>,
    collapsed_parents: &std::collections::HashSet<graphene_core::NodeId>,
    overlap_padding: f32,
    compound_padding: f32,
) {
    center_layout_at_origin(state);
    resolve_overlaps(state, overlap_padding);
    crate::traits::resolve_compound_bounds(state, collapsed_parents, compound_padding);
    state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::math::{Size2, Vec2};

    #[test]
    fn test_resolve_overlaps_eliminates_aabb_collision() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));
        let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(50.0, 50.0));

        resolve_overlaps(&mut state, 10.0);

        let idx1 = state.node_keys.get(n1).copied().unwrap();
        let idx2 = state.node_keys.get(n2).copied().unwrap();
        let pos1 = *state.positions.get(idx1);
        let pos2 = *state.positions.get(idx2);

        let dx = (pos1.x - pos2.x).abs();
        let dy = (pos1.y - pos2.y).abs();
        let min_x = 50.0 + 10.0;
        let min_y = 50.0 + 10.0;

        assert!(
            dx >= min_x || dy >= min_y,
            "Node bounding boxes should not overlap after resolution: dx={}, dy={}",
            dx,
            dy
        );
    }
}

