use graphene_core::{math::Size2, math::Vec2, AnimationTrack, EdgeId, GraphState, NodeId};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub trait Layout<S: Copy = ()> {
    fn compute(&mut self, state: &mut GraphState<S>);
}

pub fn resolve_compound_bounds<S: Copy>(
    state: &mut GraphState<S>,
    collapsed_parents: &HashSet<NodeId>,
    padding: f32,
) {
    let n = state.node_index_to_id.len();
    if n == 0 { return; }

    let mut parent_to_children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
    let mut all_parents = HashSet::new();

    for idx in 0..n {
        let id = state.node_index_to_id[idx];
        if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
            parent_to_children.entry(parent_id).or_default().push(id);
            all_parents.insert(parent_id);
        }
    }

    let mut resolved_parents = HashSet::new();
    let mut attempts = 0;
    while resolved_parents.len() < all_parents.len() && attempts < 100 {
        attempts += 1;
        for &parent_id in &all_parents {
            if resolved_parents.contains(&parent_id) { continue; }

            // If the parent itself is collapsed, skip resolving bounds from children
            if collapsed_parents.contains(&parent_id) {
                resolved_parents.insert(parent_id);
                continue;
            }

            let children = &parent_to_children[&parent_id];
            let mut can_resolve = true;
            for &child_id in children {
                if all_parents.contains(&child_id) && !resolved_parents.contains(&child_id) {
                    can_resolve = false;
                    break;
                }
            }

            if can_resolve {
                let mut min_x = f32::INFINITY;
                let mut max_x = -f32::INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_y = -f32::INFINITY;

                for &child_id in children {
                    let Some(&idx) = state.node_keys.get(child_id) else { continue };
                    let pos = *state.positions.get(idx);
                    let size = *state.sizes.get(idx);
                    min_x = min_x.min(pos.x - size.w / 2.0);
                    max_x = max_x.max(pos.x + size.w / 2.0);
                    min_y = min_y.min(pos.y - size.h / 2.0);
                    max_y = max_y.max(pos.y + size.h / 2.0);
                }

                if min_x.is_finite() {
                    let center_x = (min_x + max_x) / 2.0;
                    let center_y = (min_y + max_y) / 2.0;
                    let w = (max_x - min_x) + 2.0 * padding;
                    let h = (max_y - min_y) + 2.0 * padding;

                    if let Some(&p_idx) = state.node_keys.get(parent_id) {
                        state.positions.set(p_idx, Vec2::new(center_x, center_y));
                        state.sizes.set(p_idx, Size2::new(w, h));
                    }
                }
                resolved_parents.insert(parent_id);
            }
        }
    }
}

pub fn compute_flat_layout<S: Copy + Default, L: Layout<S>>(
    layout: &mut L,
    state: &mut GraphState<S>,
    collapsed_parents: &HashSet<NodeId>,
) {
    let n = state.node_index_to_id.len();
    let mut visible_indices = Vec::new();
    let mut node_map = HashMap::new();

    let get_visible_rep = |mut curr: NodeId| -> NodeId {
        let mut rep = curr;
        while let Some(&idx) = state.node_keys.get(curr) {
            if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
                if collapsed_parents.contains(&parent_id) {
                    rep = parent_id;
                }
                curr = parent_id;
            } else {
                break;
            }
        }
        rep
    };

    let mut flat_state = GraphState::new();
    for idx in 0..n {
        let id = state.node_index_to_id[idx];
        if get_visible_rep(id) == id {
            visible_indices.push(idx);
            let pos = *state.positions.get(idx);
            let size = *state.sizes.get(idx);
            let new_id = flat_state.add_node(pos, size);
            node_map.insert(id, new_id);
        }
    }

    for idx in 0..n {
        let id = state.node_index_to_id[idx];
        if get_visible_rep(id) == id {
            if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
                let parent_rep = get_visible_rep(parent_id);
                if parent_rep == parent_id && !collapsed_parents.contains(&parent_id) {
                    if let (Some(&new_child_id), Some(&new_parent_id)) = (node_map.get(&id), node_map.get(&parent_id)) {
                        let new_child_idx = flat_state.node_keys[new_child_id];
                        flat_state.hierarchy.parent.set(new_child_idx, Some(new_parent_id));
                    }
                }
            }
        }
    }

    for i in 0..state.edges.len() {
        let src = *state.edge_sources.get(i);
        let tgt = *state.edge_targets.get(i);
        let src_rep = get_visible_rep(src);
        let tgt_rep = get_visible_rep(tgt);

        if src_rep != tgt_rep {
            if let (Some(&new_src), Some(&new_tgt)) = (node_map.get(&src_rep), node_map.get(&tgt_rep)) {
                let mut edge_exists = false;
                for e_idx in 0..flat_state.edges.len() {
                    if flat_state.edge_sources[e_idx] == new_src && flat_state.edge_targets[e_idx] == new_tgt {
                        edge_exists = true;
                        break;
                    }
                }
                if !edge_exists {
                    flat_state.add_edge(new_src, new_tgt, graphene_core::EdgeData::default());
                }
            }
        }
    }

    layout.compute(&mut flat_state);

    for (&id, &new_id) in &node_map {
        if let (Some(&idx), Some(&flat_idx)) = (state.node_keys.get(id), flat_state.node_keys.get(new_id)) {
            state.positions.set(idx, *flat_state.positions.get(flat_idx));
        }
    }

    resolve_compound_bounds(state, collapsed_parents, 20.0);
    state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
}

pub struct RandomLayout {
    pub width: f32,
    pub height: f32,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for RandomLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let mut state_lcg = 12345u64;
        let mut next_float = || {
            state_lcg = state_lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (state_lcg >> 32) as f32 / u32::MAX as f32
        };

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let target = Vec2::new(next_float() * self.width, next_float() * self.height);
            if self.animate {
                let from = *state.positions.get(idx);
                state.animations.tracks.insert(
                    id,
                    AnimationTrack::Position {
                        from,
                        to: target,
                        duration: Duration::from_millis(500),
                        elapsed: Duration::ZERO,
                    },
                );
            } else {
                state.positions.set(idx, target);
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct GridLayout {
    pub columns: usize,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for GridLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }
        let cols = self.columns.max(1);

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let r = idx / cols;
            let c = idx % cols;
            let target = Vec2::new(c as f32 * self.spacing_x, r as f32 * self.spacing_y);

            if self.animate {
                let from = *state.positions.get(idx);
                state.animations.tracks.insert(
                    id,
                    AnimationTrack::Position {
                        from,
                        to: target,
                        duration: Duration::from_millis(500),
                        elapsed: Duration::ZERO,
                    },
                );
            } else {
                state.positions.set(idx, target);
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct CircleLayout {
    pub radius: f32,
    pub center: Vec2,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for CircleLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let angle = (idx as f32 / num_nodes as f32) * 2.0 * std::f32::consts::PI;
            let target = Vec2::new(
                self.center.x + self.radius * angle.cos(),
                self.center.y + self.radius * angle.sin(),
            );

            if self.animate {
                let from = *state.positions.get(idx);
                state.animations.tracks.insert(
                    id,
                    AnimationTrack::Position {
                        from,
                        to: target,
                        duration: Duration::from_millis(500),
                        elapsed: Duration::ZERO,
                    },
                );
            } else {
                state.positions.set(idx, target);
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct ConcentricLayout {
    pub level_radius_step: f32,
    pub center: Vec2,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for ConcentricLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }

        // Put up to 5 nodes in first circle, 10 in second, 20 in third, etc.
        let mut _level = 0;
        let mut max_in_level = 5;
        let mut level_count = 0;
        let mut level_radius = self.level_radius_step;

        let mut level_nodes = Vec::new();
        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            level_nodes.push((idx, id));
            level_count += 1;
            if level_count >= max_in_level || idx == num_nodes - 1 {
                // Position all nodes in current level
                let count = level_nodes.len();
                for (j, &(n_idx, n_id)) in level_nodes.iter().enumerate() {
                    let angle = (j as f32 / count as f32) * 2.0 * std::f32::consts::PI;
                    let target = Vec2::new(
                        self.center.x + level_radius * angle.cos(),
                        self.center.y + level_radius * angle.sin(),
                    );
                    if self.animate {
                        let from = *state.positions.get(n_idx);
                        state.animations.tracks.insert(
                            n_id,
                            AnimationTrack::Position {
                                from,
                                to: target,
                                duration: Duration::from_millis(500),
                                elapsed: Duration::ZERO,
                            },
                        );
                    } else {
                        state.positions.set(n_idx, target);
                    }
                }
                level_nodes.clear();
                level_count = 0;
                _level += 1;
                max_in_level *= 2;
                level_radius += self.level_radius_step;
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct BreadthFirstLayout {
    pub root: NodeId,
    pub sibling_spacing: f32,
    pub level_spacing: f32,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for BreadthFirstLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        if !state.node_keys.contains_key(self.root) {
            return;
        }

        // Simple BFS to calculate levels
        let mut levels = std::collections::HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        let mut visited = std::collections::HashSet::new();

        queue.push_back((self.root, 0));
        visited.insert(self.root);

        // Build parent/child adjacency for tree layout
        let mut adj: std::collections::HashMap<NodeId, Vec<NodeId>> = std::collections::HashMap::new();
        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            adj.entry(src).or_default().push(tgt);
        }

        while let Some((curr, lvl)) = queue.pop_front() {
            levels.entry(lvl).or_insert_with(Vec::new).push(curr);
            if let Some(children) = adj.get(&curr) {
                for &child in children {
                    if visited.insert(child) {
                        queue.push_back((child, lvl + 1));
                    }
                }
            }
        }

        // Position nodes by level
        for (&lvl, level_nodes) in &levels {
            let count = level_nodes.len();
            let total_width = (count - 1) as f32 * self.sibling_spacing;
            let start_x = -total_width / 2.0;

            for (i, &id) in level_nodes.iter().enumerate() {
                if let Some(&idx) = state.node_keys.get(id) {
                    let target = Vec2::new(
                        start_x + i as f32 * self.sibling_spacing,
                        lvl as f32 * self.level_spacing,
                    );
                    if self.animate {
                        let from = *state.positions.get(idx);
                        state.animations.tracks.insert(
                            id,
                            AnimationTrack::Position {
                                from,
                                to: target,
                                duration: Duration::from_millis(500),
                                elapsed: Duration::ZERO,
                            },
                        );
                    } else {
                        state.positions.set(idx, target);
                    }
                }
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === QUAD TREE FOR BARNES-HUT SIMULATION ===

pub struct Quadtree {
    pub center_of_mass: Vec2,
    pub total_mass: f32,
    pub bounds_min: Vec2,
    pub bounds_max: Vec2,
    pub children: Option<Box<[Quadtree; 4]>>,
    pub node_indices: Vec<usize>,
}

impl Quadtree {
    pub fn new(bounds_min: Vec2, bounds_max: Vec2) -> Self {
        Self {
            center_of_mass: Vec2::default(),
            total_mass: 0.0,
            bounds_min,
            bounds_max,
            children: None,
            node_indices: Vec::new(),
        }
    }

    pub fn build(positions: &[Vec2]) -> Self {
        let n = positions.len();
        if n == 0 {
            return Self::new(Vec2::default(), Vec2::default());
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = -f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = -f32::INFINITY;

        for &pos in positions {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
        }

        let size_x = max_x - min_x;
        let size_y = max_y - min_y;
        let max_size = size_x.max(size_y).max(1.0);
        let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);

        let bounds_min = Vec2::new(center.x - max_size * 0.5 - 1.0, center.y - max_size * 0.5 - 1.0);
        let bounds_max = Vec2::new(center.x + max_size * 0.5 + 1.0, center.y + max_size * 0.5 + 1.0);

        let mut root = Self::new(bounds_min, bounds_max);
        for i in 0..n {
            root.insert(i, positions, 0);
        }
        root
    }

    pub fn insert(&mut self, idx: usize, positions: &[Vec2], depth: usize) {
        let pos = positions[idx];
        self.center_of_mass = (self.center_of_mass * self.total_mass + pos) / (self.total_mass + 1.0);
        self.total_mass += 1.0;

        if self.children.is_none() && self.node_indices.is_empty() {
            self.node_indices.push(idx);
            return;
        }

        if self.children.is_none() {
            if depth >= 15 {
                self.node_indices.push(idx);
                return;
            }

            let mid = (self.bounds_min + self.bounds_max) * 0.5;
            let sub_nodes = [
                Self::new(self.bounds_min, mid),
                Self::new(Vec2::new(mid.x, self.bounds_min.y), Vec2::new(self.bounds_max.x, mid.y)),
                Self::new(Vec2::new(self.bounds_min.x, mid.y), Vec2::new(mid.x, self.bounds_max.y)),
                Self::new(mid, self.bounds_max),
            ];

            let old_indices = std::mem::take(&mut self.node_indices);
            self.children = Some(Box::new(sub_nodes));

            let children_ref = self.children.as_mut().unwrap();
            for old_idx in old_indices {
                let old_pos = positions[old_idx];
                let c_idx = if old_pos.y < mid.y {
                    if old_pos.x < mid.x { 0 } else { 1 }
                } else {
                    if old_pos.x < mid.x { 2 } else { 3 }
                };
                children_ref[c_idx].insert(old_idx, positions, depth + 1);
            }
        }

        if let Some(ref mut children) = self.children {
            let mid = (self.bounds_min + self.bounds_max) * 0.5;
            let child_idx = if pos.y < mid.y {
                if pos.x < mid.x { 0 } else { 1 }
            } else {
                if pos.x < mid.x { 2 } else { 3 }
            };
            children[child_idx].insert(idx, positions, depth + 1);
        }
    }

    pub fn accumulate_repulsion(&self, i: usize, pos_i: Vec2, positions: &[Vec2], k_rep: f32, theta: f32) -> Vec2 {
        if self.total_mass == 0.0 {
            return Vec2::default();
        }

        let delta = pos_i - self.center_of_mass;
        let dist = delta.len();

        if let Some(ref children) = self.children {
            let s = (self.bounds_max.x - self.bounds_min.x).max(self.bounds_max.y - self.bounds_min.y);
            if dist > 0.1 && (s / dist) < theta {
                let force_magnitude = (k_rep * self.total_mass) / (dist * dist);
                let dir = delta.normalize();
                return dir * force_magnitude;
            }

            let mut force = Vec2::default();
            for child in children.iter() {
                force += child.accumulate_repulsion(i, pos_i, positions, k_rep, theta);
            }
            force
        } else {
            let mut force = Vec2::default();
            for &j in &self.node_indices {
                if i == j {
                    continue;
                }
                let pos_j = positions[j];
                let d_delta = pos_i - pos_j;
                let d_dist = d_delta.len();
                if d_dist > 0.1 {
                    let force_magnitude = k_rep / (d_dist * d_dist);
                    let dir = d_delta.normalize();
                    force += dir * force_magnitude;
                } else {
                    let force_magnitude = k_rep / 0.01;
                    let dir = Vec2::new(1.0, 0.0);
                    force += dir * force_magnitude;
                }
            }
            force
        }
    }
}

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

            // 1. Repulsive forces (classical O(N^2) or Barnes-Hut O(N log N))
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

            // 2. Attractive forces along edges
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

            // 3. Gravity towards center
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

            // 4. Limit displacement by temperature and update positions
            for i in 0..n {
                let disp = displacements[i];
                let disp_len = disp.len();
                if disp_len > 0.01 {
                    let capped_disp = disp.normalize() * disp_len.min(temp);
                    let old_pos = *state.positions.get(i);
                    state.positions.set(i, old_pos + capped_disp);
                }
            }

            // Cool temperature
            temp *= 0.95;
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === COSE LAYOUT ===

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

fn find_clipping_point(pos: Vec2, size: graphene_core::Size2, dx: f32, dy: f32) -> Vec2 {
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

    // Right border
    if dx > 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(pos.x + w / 2.0, pos.y + (w * dy / (2.0 * dx)));
    }
    // Left border
    if dx < 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(pos.x - w / 2.0, pos.y - (w * dy / (2.0 * dx)));
    }
    // Top border
    if dy > 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(pos.x + (h * dx / (2.0 * dy)), pos.y + h / 2.0);
    }
    // Bottom border
    if dy < 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(pos.x - (h * dx / (2.0 * dy)), pos.y - h / 2.0);
    }

    pos
}

fn get_nesting_depth<S: Copy>(state: &GraphState<S>, u: NodeId, v: NodeId) -> usize {
    let Some(&u_idx) = state.node_keys.get(u) else { return 0 };
    let Some(&v_idx) = state.node_keys.get(v) else { return 0 };

    let mut u_path = Vec::new();
    let mut curr_u = *state.hierarchy.parent.get(u_idx);
    while let Some(parent_id) = curr_u {
        u_path.push(parent_id);
        if let Some(&p_idx) = state.node_keys.get(parent_id) {
            curr_u = *state.hierarchy.parent.get(p_idx);
        } else {
            break;
        }
    }

    let mut v_path = Vec::new();
    let mut curr_v = *state.hierarchy.parent.get(v_idx);
    while let Some(parent_id) = curr_v {
        v_path.push(parent_id);
        if let Some(&p_idx) = state.node_keys.get(parent_id) {
            curr_v = *state.hierarchy.parent.get(p_idx);
        } else {
            break;
        }
    }

    let u_depth = u_path.len();
    let v_depth = v_path.len();

    for (i, &p_u) in u_path.iter().enumerate() {
        if let Some(j) = v_path.iter().position(|&p_v| p_v == p_u) {
            return i + j;
        }
    }

    u_depth + v_depth
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

            // 1. Calculate node repulsions
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

                    // Check overlap
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
                        // Clipping points
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

            // 2. Calculate edge forces (attraction)
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

                // ideal length with nesting depth scaling
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

            // 3. Gravity towards center
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

            // 4. Update positions with temperature cap
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

            // Cool temperature
            temp *= self.cooling_factor;
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === KAMADA-KAWAI LAYOUT ===

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

        let max_finite_dist = d.iter()
            .flatten()
            .filter(|&&x| x != f32::INFINITY)
            .copied()
            .fold(0.0f32, |m, x| m.max(x));
        let disconnect_dist = if max_finite_dist > 0.0 { max_finite_dist * 2.0 } else { 4.0 };
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
                    if i == j { continue; }
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

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === SUGIYAMA LAYOUT ===

pub struct SugiyamaLayout {
    pub layer_spacing: f32,
    pub node_spacing: f32,
}

impl Default for SugiyamaLayout {
    fn default() -> Self {
        Self {
            layer_spacing: 80.0,
            node_spacing: 60.0,
        }
    }
}

impl<S: Copy> Layout<S> for SugiyamaLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for &id in &state.node_index_to_id {
            adj.insert(id, Vec::new());
        }
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            adj.entry(src).or_default().push(tgt);
        }

        // DFS to find feedback edges (cycles)
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        let mut feedback_edges = HashSet::new();

        fn dfs_find_cycles(
            u: NodeId,
            adj: &HashMap<NodeId, Vec<NodeId>>,
            visited: &mut HashSet<NodeId>,
            stack: &mut HashSet<NodeId>,
            feedback_edges: &mut HashSet<(NodeId, NodeId)>,
        ) {
            visited.insert(u);
            stack.insert(u);

            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if stack.contains(&v) {
                        feedback_edges.insert((u, v));
                    } else if !visited.contains(&v) {
                        dfs_find_cycles(v, adj, visited, stack, feedback_edges);
                    }
                }
            }

            stack.remove(&u);
        }

        for &node_id in &state.node_index_to_id {
            if !visited.contains(&node_id) {
                dfs_find_cycles(node_id, &adj, &mut visited, &mut stack, &mut feedback_edges);
            }
        }

        // Compute in-degrees for the DAG
        let mut in_degrees = HashMap::new();
        for &id in &state.node_index_to_id {
            in_degrees.insert(id, 0);
        }
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            if feedback_edges.contains(&(src, tgt)) {
                continue;
            }
            if let Some(deg) = in_degrees.get_mut(&tgt) {
                *deg += 1;
            }
        }

        let mut layers: HashMap<NodeId, usize> = HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        for &id in &state.node_index_to_id {
            layers.insert(id, 0);
            if in_degrees[&id] == 0 {
                queue.push_back(id);
            }
        }

        while let Some(u) = queue.pop_front() {
            let u_layer = layers[&u];
            if let Some(neighbors) = adj.get(&u) {
                for &v in neighbors {
                    if feedback_edges.contains(&(u, v)) {
                        continue;
                    }
                    let current_v_layer = layers[&v];
                    let target_layer = u_layer + 1;
                    if target_layer > current_v_layer {
                        layers.insert(v, target_layer);
                    }
                    if let Some(deg) = in_degrees.get_mut(&v) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(v);
                        }
                    }
                }
            }
        }

        for &id in &state.node_index_to_id {
            layers.entry(id).or_insert(0);
        }

        let mut layer_groups: HashMap<usize, Vec<NodeId>> = HashMap::new();
        for (&id, &layer) in &layers {
            layer_groups.entry(layer).or_default().push(id);
        }

        let num_layers = layer_groups.keys().copied().fold(0, |a, b| a.max(b)) + 1;

        for layer in 1..num_layers {
            if let Some(nodes_in_layer) = layer_groups.get_mut(&layer) {
                let mut barycenters = HashMap::new();
                for &v in nodes_in_layer.iter() {
                    let mut sum = 0.0;
                    let mut count = 0;
                    for idx in 0..state.edges.len() {
                        let src = *state.edge_sources.get(idx);
                        let tgt = *state.edge_targets.get(idx);
                        if tgt == v {
                            if let Some(&src_idx) = state.node_keys.get(src) {
                                sum += state.positions.get(src_idx).x;
                                count += 1;
                            }
                        }
                    }
                    let bc = if count > 0 { sum / count as f32 } else { 0.0 };
                    barycenters.insert(v, bc);
                }

                nodes_in_layer.sort_by(|a, b| {
                    barycenters[a].partial_cmp(&barycenters[b]).unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }

        for layer in 0..num_layers {
            if let Some(nodes_in_layer) = layer_groups.get(&layer) {
                let layer_width = (nodes_in_layer.len() - 1) as f32 * self.node_spacing;
                let start_x = -layer_width / 2.0;
                let y = (layer as f32) * self.layer_spacing;

                for (idx, &id) in nodes_in_layer.iter().enumerate() {
                    if let Some(&node_idx) = state.node_keys.get(id) {
                        let x = start_x + (idx as f32) * self.node_spacing;
                        state.positions.set(node_idx, Vec2::new(x, y));
                    }
                }
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === REINGOLD-TILFORD TREE LAYOUT ===

pub struct ReingoldTilfordLayout {
    pub sibling_spacing: f32,
    pub level_spacing: f32,
}

impl Default for ReingoldTilfordLayout {
    fn default() -> Self {
        Self {
            sibling_spacing: 50.0,
            level_spacing: 80.0,
        }
    }
}

struct TreeNode {
    id: NodeId,
    x: f32,
    mod_val: f32,
    children: Vec<usize>,
}

impl<S: Copy> Layout<S> for ReingoldTilfordLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut roots = Vec::new();
        for idx in 0..n {
            let parent = *state.hierarchy.parent.get(idx);
            if parent.is_none() {
                roots.push(state.node_index_to_id[idx]);
            }
        }

        if roots.is_empty() {
            roots.push(state.node_index_to_id[0]);
        }

        let mut nodes = Vec::new();
        let mut node_to_tree_idx = HashMap::new();

        for &id in &state.node_index_to_id {
            node_to_tree_idx.insert(id, nodes.len());
            nodes.push(TreeNode {
                id,
                x: 0.0,
                mod_val: 0.0,
                children: Vec::new(),
            });
        }

        for idx in 0..n {
            let id = state.node_index_to_id[idx];
            if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
                if let Some(&parent_tree_idx) = node_to_tree_idx.get(&parent_id) {
                    let tree_idx = node_to_tree_idx[&id];
                    nodes[parent_tree_idx].children.push(tree_idx);
                }
            }
        }

        fn first_walk(
            tree_idx: usize,
            depth: usize,
            nodes: &mut [TreeNode],
            sibling_spacing: f32,
            level_spacing: f32,
        ) {
            if nodes[tree_idx].children.is_empty() {
                nodes[tree_idx].x = 0.0;
            } else {
                let children = nodes[tree_idx].children.clone();
                for &child_idx in &children {
                    first_walk(child_idx, depth + 1, nodes, sibling_spacing, level_spacing);
                }

                let mid_x = if nodes[tree_idx].children.len() == 1 {
                    nodes[nodes[tree_idx].children[0]].x
                } else {
                    let first = nodes[nodes[tree_idx].children[0]].x;
                    let last = nodes[*nodes[tree_idx].children.last().unwrap()].x;
                    (first + last) / 2.0
                };

                nodes[tree_idx].x = mid_x;

                let mut max_shift = 0.0f32;
                for i in 0..nodes[tree_idx].children.len() {
                    for j in (i + 1)..nodes[tree_idx].children.len() {
                        let c1 = nodes[tree_idx].children[i];
                        let c2 = nodes[tree_idx].children[j];
                        let overlap = (nodes[c1].x + sibling_spacing) - nodes[c2].x;
                        if overlap > max_shift {
                            max_shift = overlap;
                        }
                    }
                }
                if max_shift > 0.0 {
                    let last_idx = *nodes[tree_idx].children.last().unwrap();
                    nodes[last_idx].x += max_shift;
                    nodes[tree_idx].x += max_shift / 2.0;
                }
            }
        }

        fn second_walk<S: Copy>(
            tree_idx: usize,
            depth: usize,
            acc_mod: f32,
            nodes: &[TreeNode],
            state: &mut GraphState<S>,
            level_spacing: f32,
        ) {
            let x = nodes[tree_idx].x + acc_mod;
            let y = (depth as f32) * level_spacing;
            if let Some(&node_idx) = state.node_keys.get(nodes[tree_idx].id) {
                state.positions.set(node_idx, Vec2::new(x, y));
            }

            for &child_idx in &nodes[tree_idx].children {
                second_walk(
                    child_idx,
                    depth + 1,
                    acc_mod + nodes[tree_idx].mod_val,
                    nodes,
                    state,
                    level_spacing,
                );
            }
        }

        for &root in &roots {
            if let Some(&root_tree_idx) = node_to_tree_idx.get(&root) {
                first_walk(root_tree_idx, 0, &mut nodes, self.sibling_spacing, self.level_spacing);
                second_walk(root_tree_idx, 0, 0.0, &nodes, state, self.level_spacing);
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === MDS LAYOUT ===

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

impl<S: Copy> Layout<S> for MdsLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n <= 1 { return; }

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

        let max_finite_dist = d.iter()
            .flatten()
            .filter(|&&x| x != f32::INFINITY)
            .copied()
            .fold(0.0f32, |m, x| m.max(x));
        let disconnect_dist = if max_finite_dist > 0.0 { max_finite_dist * 2.0 } else { 4.0 };
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
                    if i == j { continue; }
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

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === GRID PLACEMENT WITH LINEAR SORTING ===

pub struct GridSortedLayout {
    pub columns: usize,
    pub node_spacing: f32,
    pub sort_by_degree: bool,
}

impl Default for GridSortedLayout {
    fn default() -> Self {
        Self {
            columns: 5,
            node_spacing: 80.0,
            sort_by_degree: true,
        }
    }
}

impl<S: Copy> Layout<S> for GridSortedLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut sorted_nodes = state.node_index_to_id.clone();
        if self.sort_by_degree {
            let mut degrees = HashMap::new();
            for &id in &state.node_index_to_id {
                degrees.insert(id, 0);
            }
            for idx in 0..state.edges.len() {
                let src = *state.edge_sources.get(idx);
                let tgt = *state.edge_targets.get(idx);
                if let Some(deg) = degrees.get_mut(&src) { *deg += 1; }
                if let Some(deg) = degrees.get_mut(&tgt) { *deg += 1; }
            }
            sorted_nodes.sort_by(|a, b| degrees[b].cmp(&degrees[a]));
        } else {
            sorted_nodes.sort();
        }

        let cols = self.columns.max(1);
        for (idx, id) in sorted_nodes.into_iter().enumerate() {
            if let Some(&node_idx) = state.node_keys.get(id) {
                let r = idx / cols;
                let c = idx % cols;
                let x = (c as f32) * self.node_spacing;
                let y = (r as f32) * self.node_spacing;
                state.positions.set(node_idx, Vec2::new(x, y));
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === HIERARCHICAL EDGE BUNDLING ===

pub fn compute_hierarchical_edge_bundling<S: Copy>(
    state: &GraphState<S>,
    beta: f32,
) -> HashMap<EdgeId, Vec<Vec2>> {
    let mut bundled_edges = HashMap::new();
    for idx in 0..state.edges.len() {
        let edge_id = state.edge_index_to_id[idx];
        let src = *state.edge_sources.get(idx);
        let tgt = *state.edge_targets.get(idx);
        let Some(&src_idx) = state.node_keys.get(src) else { continue };
        let Some(&tgt_idx) = state.node_keys.get(tgt) else { continue };

        let p_start = *state.positions.get(src_idx);
        let p_end = *state.positions.get(tgt_idx);

        let mut src_path = Vec::new();
        let mut curr_src = src;
        while let Some(&curr_idx) = state.node_keys.get(curr_src) {
            src_path.push(curr_src);
            if let Some(p) = *state.hierarchy.parent.get(curr_idx) {
                curr_src = p;
            } else {
                break;
            }
        }

        let mut tgt_path = Vec::new();
        let mut curr_tgt = tgt;
        while let Some(&curr_idx) = state.node_keys.get(curr_tgt) {
            tgt_path.push(curr_tgt);
            if let Some(p) = *state.hierarchy.parent.get(curr_idx) {
                curr_tgt = p;
            } else {
                break;
            }
        }

        let mut lca = None;
        for &u in &src_path {
            if tgt_path.contains(&u) {
                lca = Some(u);
                break;
            }
        }

        let mut control_points = Vec::new();
        control_points.push(p_start);

        if let Some(lca_node) = lca {
            for &u in &src_path {
                if u == lca_node { break; }
                if let Some(&u_idx) = state.node_keys.get(u) {
                    control_points.push(*state.positions.get(u_idx));
                }
            }

            if let Some(&lca_idx) = state.node_keys.get(lca_node) {
                control_points.push(*state.positions.get(lca_idx));
            }

            let mut from_lca = Vec::new();
            for &v in &tgt_path {
                if v == lca_node { break; }
                if let Some(&v_idx) = state.node_keys.get(v) {
                    from_lca.push(*state.positions.get(v_idx));
                }
            }
            from_lca.reverse();
            control_points.extend(from_lca);
        }

        control_points.push(p_end);

        let mut bundled_points = Vec::new();
        let cp_len = control_points.len();
        for (i, &cp) in control_points.iter().enumerate() {
            let t = if cp_len > 1 { i as f32 / (cp_len - 1) as f32 } else { 0.0 };
            let straight_point = p_start + (p_end - p_start) * t;
            let bundled_point = cp * beta + straight_point * (1.0 - beta);
            bundled_points.push(bundled_point);
        }

        bundled_edges.insert(edge_id, bundled_points);
    }
    bundled_edges
}

// === DISCONNECTED PACKER ===

pub struct DisconnectedPacker<L> {
    pub sub_layout: L,
    pub spacing: f32,
}

impl<S: Copy + Default, L: Layout<S>> Layout<S> for DisconnectedPacker<L> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut visited = HashSet::new();
        let mut components = Vec::new();

        let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            adj.entry(src).or_default().push(tgt);
            adj.entry(tgt).or_default().push(src);
        }

        for &node_id in &state.node_index_to_id {
            if !visited.contains(&node_id) {
                let mut comp = Vec::new();
                let mut queue = std::collections::VecDeque::new();
                queue.push_back(node_id);
                visited.insert(node_id);

                while let Some(u) = queue.pop_front() {
                    comp.push(u);
                    if let Some(neighbors) = adj.get(&u) {
                        for &v in neighbors {
                            if !visited.contains(&v) {
                                visited.insert(v);
                                queue.push_back(v);
                            }
                        }
                    }
                }
                components.push(comp);
            }
        }

        if components.is_empty() { return; }

        let mut current_offset = Vec2::default();

        for component in components {
            let mut sub_state: GraphState<S> = GraphState::new();
            let mut node_mapping = HashMap::new();

            for &node_id in &component {
                let Some(&idx) = state.node_keys.get(node_id) else { continue };
                let pos = *state.positions.get(idx);
                let size = *state.sizes.get(idx);
                let new_id = sub_state.add_node(pos, size);
                node_mapping.insert(node_id, new_id);
            }

            for idx in 0..state.edges.len() {
                let src = *state.edge_sources.get(idx);
                let tgt = *state.edge_targets.get(idx);
                if component.contains(&src) && component.contains(&tgt) {
                    let data = state.edges[idx];
                    sub_state.add_edge(node_mapping[&src], node_mapping[&tgt], data);
                }
            }

            self.sub_layout.compute(&mut sub_state);

            let mut min_x = f32::INFINITY;
            let mut max_x = -f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = -f32::INFINITY;

            for i in 0..sub_state.node_index_to_id.len() {
                let pos = *sub_state.positions.get(i);
                let size = *sub_state.sizes.get(i);
                min_x = min_x.min(pos.x - size.w / 2.0);
                max_x = max_x.max(pos.x + size.w / 2.0);
                min_y = min_y.min(pos.y - size.h / 2.0);
                max_y = max_y.max(pos.y + size.h / 2.0);
            }

            let comp_w = max_x - min_x;
            let shift = current_offset - Vec2::new(min_x, min_y);
            for &node_id in &component {
                if let Some(&node_idx) = state.node_keys.get(node_id) {
                    if let Some(&sub_node_id) = node_mapping.get(&node_id) {
                        if let Some(&sub_node_idx) = sub_state.node_keys.get(sub_node_id) {
                            let local_pos = *sub_state.positions.get(sub_node_idx);
                            state.positions.set(node_idx, local_pos + shift);
                        }
                    }
                }
            }

            current_offset.x += comp_w + self.spacing;
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === CONCENTRIC CIRCULAR HUB LAYOUT ===

pub struct ConcentricHubLayout {
    pub hub_threshold: usize,
    pub inner_radius: f32,
    pub ring_spacing: f32,
}

impl Default for ConcentricHubLayout {
    fn default() -> Self {
        Self {
            hub_threshold: 3,
            inner_radius: 50.0,
            ring_spacing: 80.0,
        }
    }
}

impl<S: Copy> Layout<S> for ConcentricHubLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut degrees = HashMap::new();
        for &id in &state.node_index_to_id {
            degrees.insert(id, 0);
        }
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            if let Some(deg) = degrees.get_mut(&src) { *deg += 1; }
            if let Some(deg) = degrees.get_mut(&tgt) { *deg += 1; }
        }

        let mut hubs = Vec::new();
        let mut peers = Vec::new();
        for &id in &state.node_index_to_id {
            if degrees[&id] >= self.hub_threshold {
                hubs.push(id);
            } else {
                peers.push(id);
            }
        }

        if hubs.len() == 1 {
            if let Some(&idx) = state.node_keys.get(hubs[0]) {
                state.positions.set(idx, Vec2::new(0.0, 0.0));
            }
        } else {
            let angle_step = 2.0 * std::f32::consts::PI / hubs.len() as f32;
            for (i, &id) in hubs.iter().enumerate() {
                if let Some(&idx) = state.node_keys.get(id) {
                    let angle = (i as f32) * angle_step;
                    let x = self.inner_radius * angle.cos();
                    let y = self.inner_radius * angle.sin();
                    state.positions.set(idx, Vec2::new(x, y));
                }
            }
        }

        if !peers.is_empty() {
            let outer_radius = self.inner_radius + self.ring_spacing;
            let angle_step = 2.0 * std::f32::consts::PI / peers.len() as f32;
            for (i, &id) in peers.iter().enumerate() {
                if let Some(&idx) = state.node_keys.get(id) {
                    let angle = (i as f32) * angle_step;
                    let x = outer_radius * angle.cos();
                    let y = outer_radius * angle.sin();
                    state.positions.set(idx, Vec2::new(x, y));
                }
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === BIPARTITE/MULTIPARTITE LAYOUT ===

pub struct BipartiteLayout<F> {
    pub partition_fn: F,
    pub column_spacing: f32,
    pub vertical_spacing: f32,
}

impl<S: Copy, F: Fn(NodeId) -> usize> Layout<S> for BipartiteLayout<F> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let mut sets: HashMap<usize, Vec<NodeId>> = HashMap::new();
        for &id in &state.node_index_to_id {
            let part = (self.partition_fn)(id);
            sets.entry(part).or_default().push(id);
        }

        for (&col, nodes) in &sets {
            let col_height = (nodes.len() - 1) as f32 * self.vertical_spacing;
            let start_y = -col_height / 2.0;
            let x = (col as f32) * self.column_spacing;

            for (idx, &id) in nodes.iter().enumerate() {
                if let Some(&node_idx) = state.node_keys.get(id) {
                    let y = start_y + (idx as f32) * self.vertical_spacing;
                    state.positions.set(node_idx, Vec2::new(x, y));
                }
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === WEIGHTED SPRING EMBEDDER ===

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

// === MULTIGRAPH ROUTING ===

pub fn compute_multigraph_bezier_routing<S: Copy>(
    state: &GraphState<S>,
    base_offset: f32,
) -> HashMap<EdgeId, Option<Vec2>> {
    let mut edge_control_points = HashMap::new();
    let mut edge_counts: HashMap<(NodeId, NodeId), Vec<EdgeId>> = HashMap::new();

    for idx in 0..state.edges.len() {
        let edge_id = state.edge_index_to_id[idx];
        let src = *state.edge_sources.get(idx);
        let tgt = *state.edge_targets.get(idx);
        let key = if src < tgt { (src, tgt) } else { (tgt, src) };
        edge_counts.entry(key).or_default().push(edge_id);
    }

    for ((src, tgt), edges) in edge_counts {
        let num_edges = edges.len();
        if num_edges <= 1 {
            for edge_id in edges {
                edge_control_points.insert(edge_id, None);
            }
            continue;
        }

        let Some(&src_idx) = state.node_keys.get(src) else { continue };
        let Some(&tgt_idx) = state.node_keys.get(tgt) else { continue };
        let p_src = *state.positions.get(src_idx);
        let p_tgt = *state.positions.get(tgt_idx);

        let mid = (p_src + p_tgt) / 2.0;
        let diff = p_tgt - p_src;
        let length = diff.len().max(0.01);
        let perp = Vec2::new(-diff.y / length, diff.x / length);

        for (i, edge_id) in edges.into_iter().enumerate() {
            let offset_factor = (i as f32 - (num_edges - 1) as f32 / 2.0) * base_offset;
            if offset_factor == 0.0 {
                edge_control_points.insert(edge_id, None);
            } else {
                let cp = mid + perp * offset_factor;
                edge_control_points.insert(edge_id, Some(cp));
            }
        }
    }

    edge_control_points
}

// === COMPOUND RECURSIVE LAYOUT ===

pub struct CompoundLayout<L> {
    pub sub_layout: L,
    pub padding: f32,
}

impl<S: Copy + Default, L: Layout<S>> Layout<S> for CompoundLayout<L> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut parent_to_children: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut leaf_nodes = HashSet::new();

        for idx in 0..n {
            let id = state.node_index_to_id[idx];
            if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
                parent_to_children.entry(parent_id).or_default().push(id);
            } else {
                leaf_nodes.insert(id);
            }
        }

        for (&parent_id, children) in &parent_to_children {
            let mut sub_state: GraphState<S> = GraphState::new();
            let mut mapping = HashMap::new();

            for &child_id in children {
                let Some(&idx) = state.node_keys.get(child_id) else { continue };
                let pos = *state.positions.get(idx);
                let size = *state.sizes.get(idx);
                let new_id = sub_state.add_node(pos, size);
                mapping.insert(child_id, new_id);
            }

            self.sub_layout.compute(&mut sub_state);

            let mut min_x = f32::INFINITY;
            let mut max_x = -f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_y = -f32::INFINITY;

            for i in 0..sub_state.node_index_to_id.len() {
                let pos = *sub_state.positions.get(i);
                let size = *sub_state.sizes.get(i);
                min_x = min_x.min(pos.x - size.w / 2.0);
                max_x = max_x.max(pos.x + size.w / 2.0);
                min_y = min_y.min(pos.y - size.h / 2.0);
                max_y = max_y.max(pos.y + size.h / 2.0);
            }

            if let Some(&p_idx) = state.node_keys.get(parent_id) {
                let center_x = (min_x + max_x) / 2.0;
                let center_y = (min_y + max_y) / 2.0;
                let w = (max_x - min_x) + 2.0 * self.padding;
                let h = (max_y - min_y) + 2.0 * self.padding;

                state.positions.set(p_idx, Vec2::new(center_x, center_y));
                state.sizes.set(p_idx, Size2::new(w, h));
            }
        }

        self.sub_layout.compute(state);
    }
}

// === HYPERGRAPH STAR EXPANSION ===

pub fn star_expand_hypergraph<S: Copy + Default>(
    state: &GraphState<S>,
    hyperedges: &[Vec<NodeId>],
) -> GraphState<S> {
    let mut expanded = GraphState::new();
    let mut mapping = HashMap::new();

    for idx in 0..state.node_index_to_id.len() {
        let id = state.node_index_to_id[idx];
        let pos = *state.positions.get(idx);
        let size = *state.sizes.get(idx);
        let new_id = expanded.add_node(pos, size);
        mapping.insert(id, new_id);
    }

    for hedge in hyperedges {
        let mut center = Vec2::default();
        let mut count = 0;
        for &node_id in hedge {
            if let Some(&idx) = state.node_keys.get(node_id) {
                center += *state.positions.get(idx);
                count += 1;
            }
        }
        if count > 0 {
            center = center / count as f32;
        }

        let virtual_id = expanded.add_node(center, Size2::new(15.0, 15.0));

        for &node_id in hedge {
            if let Some(&mapped_id) = mapping.get(&node_id) {
                expanded.add_edge(virtual_id, mapped_id, graphene_core::EdgeData::default());
            }
        }
    }

    expanded
}

// === ATTRIBUTE NETWORK REGIONAL PARTITION ===

pub struct RegionalPartitionLayout<F, L> {
    pub cluster_fn: F,
    pub sub_layout: L,
    pub columns: usize,
    pub cell_size: f32,
}

impl<S: Copy + Default, F: Fn(NodeId) -> usize, L: Layout<S>> Layout<S> for RegionalPartitionLayout<F, L> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let mut clusters: HashMap<usize, Vec<NodeId>> = HashMap::new();
        for &id in &state.node_index_to_id {
            let c = (self.cluster_fn)(id);
            clusters.entry(c).or_default().push(id);
        }

        let cols = self.columns.max(1);

        for (&cluster_idx, nodes) in &clusters {
            let r = cluster_idx / cols;
            let c = cluster_idx % cols;

            let cell_center = Vec2::new(
                (c as f32) * self.cell_size,
                (r as f32) * self.cell_size,
            );

            let mut sub_state: GraphState<S> = GraphState::new();
            let mut mapping = HashMap::new();

            for &node_id in nodes {
                let Some(&idx) = state.node_keys.get(node_id) else { continue };
                let pos = *state.positions.get(idx);
                let size = *state.sizes.get(idx);
                let new_id = sub_state.add_node(pos, size);
                mapping.insert(node_id, new_id);
            }

            self.sub_layout.compute(&mut sub_state);

            let mut sub_center = Vec2::default();
            let sub_n = sub_state.node_index_to_id.len();
            if sub_n > 0 {
                for i in 0..sub_n {
                    sub_center += *sub_state.positions.get(i);
                }
                sub_center = sub_center / sub_n as f32;
            }

            let shift = cell_center - sub_center;

            for &node_id in nodes {
                if let Some(&node_idx) = state.node_keys.get(node_id) {
                    if let Some(&sub_node_id) = mapping.get(&node_id) {
                        if let Some(&sub_node_idx) = sub_state.node_keys.get(sub_node_id) {
                            let local_pos = *sub_state.positions.get(sub_node_idx);
                            state.positions.set(node_idx, local_pos + shift);
                        }
                    }
                }
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

// === COLLISION-FREE FORCE DIRECTED ===

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


// === FCOSE LAYOUT ===

pub struct FCoseLayout {
    pub iterations: usize,
    pub ideal_edge_length: f32,
    pub nesting_factor: f32,
    pub gravity: f32,
    pub node_repulsion: f32,
    pub initial_temp: f32,
    pub cooling_factor: f32,
}

impl Default for FCoseLayout {
    fn default() -> Self {
        Self {
            iterations: 150,
            ideal_edge_length: 50.0,
            nesting_factor: 1.2,
            gravity: 1.5,
            node_repulsion: 4500.0,
            initial_temp: 50.0,
            cooling_factor: 0.95,
        }
    }
}

impl<S: Copy + Default> Layout<S> for FCoseLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut all_zero = true;
        for i in 0..n {
            let pos = *state.positions.get(i);
            if pos.x != 0.0 || pos.y != 0.0 {
                all_zero = false;
                break;
            }
        }
        if all_zero {
            let mut circle = CircleLayout {
                radius: 150.0,
                center: Vec2::default(),
                animate: false,
            };
            circle.compute(state);
        }

        let mut temp = self.initial_temp;
        let mut _state_lcg = 42u64;

        for _step in 0..self.iterations {
            if temp < 0.1 { break; }

            let mut displacements_x = vec![0.0f32; n];
            let mut displacements_y = vec![0.0f32; n];

            let positions_slice: Vec<Vec2> = state.positions.iter().copied().collect();
            let quadtree = Quadtree::build(&positions_slice);
            for i in 0..n {
                let pos_i = positions_slice[i];
                let force_rep = quadtree.accumulate_repulsion(i, pos_i, &positions_slice, self.node_repulsion, 0.9);
                displacements_x[i] += force_rep.x;
                displacements_y[i] += force_rep.y;
            }

            for idx in 0..state.edges.len() {
                let src_node = *state.edge_sources.get(idx);
                let tgt_node = *state.edge_targets.get(idx);
                let Some(&src_idx) = state.node_keys.get(src_node) else { continue };
                let Some(&tgt_idx) = state.node_keys.get(tgt_node) else { continue };

                if src_idx == tgt_idx { continue; }

                let pos_src = *state.positions.get(src_idx);
                let pos_tgt = *state.positions.get(tgt_idx);
                let size_src = *state.sizes.get(src_idx);
                let size_tgt = *state.sizes.get(tgt_idx);

                let dir_x = pos_tgt.x - pos_src.x;
                let dir_y = pos_tgt.y - pos_src.y;

                if dir_x == 0.0 && dir_y == 0.0 { continue; }

                let p1 = find_clipping_point(pos_src, size_src, dir_x, dir_y);
                let p2 = find_clipping_point(pos_tgt, size_tgt, -dir_x, -dir_y);

                let lx = p2.x - p1.x;
                let ly = p2.y - p1.y;
                let l = (lx * lx + ly * ly).sqrt().max(0.01);

                let depth = get_nesting_depth(state, src_node, tgt_node);
                let ideal = self.ideal_edge_length * self.nesting_factor.powi(depth as i32);

                let force_att = (ideal - l).powi(2) / 32.0;
                let fx = force_att * lx / l;
                let fy = force_att * ly / l;

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
                    let pos = state.positions.get_mut(i);
                    pos.x += dx * cap / dist;
                    pos.y += dy * cap / dist;
                }
            }

            temp *= self.cooling_factor;
        }

        let padding = 12.0;
        for _ in 0..4 {
            for i in 0..n {
                for j in (i + 1)..n {
                    let pos_i = *state.positions.get(i);
                    let pos_j = *state.positions.get(j);
                    let size_i = *state.sizes.get(i);
                    let size_j = *state.sizes.get(j);

                    let dx = pos_j.x - pos_i.x;
                    let dy = pos_j.y - pos_i.y;

                    let min_dx = (size_i.w + size_j.w) / 2.0 + padding;
                    let min_dy = (size_i.h + size_j.h) / 2.0 + padding;

                    let overlap_x = min_dx - dx.abs();
                    let overlap_y = min_dy - dy.abs();

                    if overlap_x > 0.0 && overlap_y > 0.0 {
                        let push_x;
                        let push_y;
                        if overlap_x < overlap_y {
                            let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                            push_x = sign_x * overlap_x * 0.5;
                            push_y = 0.0;
                        } else {
                            let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                            push_x = 0.0;
                            push_y = sign_y * overlap_y * 0.5;
                        }

                        let p_i = state.positions.get_mut(i);
                        p_i.x -= push_x;
                        p_i.y -= push_y;

                        let p_j = state.positions.get_mut(j);
                        p_j.x += push_x;
                        p_j.y += push_y;
                    }
                }
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
