use crate::cose::find_clipping_point;
use crate::traits::{get_nesting_depth, resolve_compound_bounds, Layout};
use graphene_core::{math::Vec2, EdgeId, GraphState, NodeId};
use std::collections::HashSet;

#[derive(Clone, Debug)]
pub struct FixedNodeConstraint {
    pub node_id: NodeId,
    pub position: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct AlignmentConstraint {
    pub horizontal: Vec<Vec<NodeId>>,
    pub vertical: Vec<Vec<NodeId>>,
}

#[derive(Clone, Debug)]
pub enum RelativePlacementConstraint {
    LeftRight {
        left: NodeId,
        right: NodeId,
        gap: f32,
    },
    TopBottom {
        top: NodeId,
        bottom: NodeId,
        gap: f32,
    },
}

#[derive(Clone, Debug, Default)]
pub struct FCoseConstraints {
    pub fixed_nodes: Vec<FixedNodeConstraint>,
    pub alignment: AlignmentConstraint,
    pub relative_placement: Vec<RelativePlacementConstraint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FCosePhase {
    DraftLayout,
    ComponentPacking,
    ConstraintSatisfaction,
    LayoutPolishing,
}

impl std::fmt::Display for FCosePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FCosePhase::DraftLayout => write!(f, "Phase I: Draft Layout Generation (Spectral)"),
            FCosePhase::ComponentPacking => write!(f, "Phase II: Component Packing"),
            FCosePhase::ConstraintSatisfaction => write!(f, "Phase III: Constraint Satisfaction"),
            FCosePhase::LayoutPolishing => write!(f, "Phase IV: Layout Polishing (Spring Embedder)"),
        }
    }
}

static FCOSE_PHASES: [FCosePhase; 4] = [
    FCosePhase::DraftLayout,
    FCosePhase::ComponentPacking,
    FCosePhase::ConstraintSatisfaction,
    FCosePhase::LayoutPolishing,
];

/// fCoSE fast compound graph layout algorithm.
///
/// Reference: Balci, H., & Dogrusoz, U. (2021). "fCoSE: A fast compound graph layout algorithm."
/// IEEE Transactions on Visualization and Computer Graphics, 28(12), 4282–4293.
pub struct FCoseLayout {
    pub iterations: usize,
    pub ideal_edge_length: f32,
    pub nesting_factor: f32,
    pub gravity: f32,
    pub node_repulsion: f32,
    pub initial_temp: f32,
    pub cooling_factor: f32,
    pub randomize: bool,
    pub compound_padding: f32,
    pub gravity_range: f32,
    pub gravity_compound: f32,
    pub gravity_range_compound: f32,
    pub tile: bool,
    pub tiling_padding_horizontal: f32,
    pub tiling_padding_vertical: f32,
    pub pack_components: bool,
    pub node_dimensions_include_labels: bool,
    pub current_phase_idx: usize,

    pub constraints: FCoseConstraints,

    pub node_repulsion_fn: Option<Box<dyn Fn(NodeId) -> f32 + Send + Sync>>,
    pub ideal_edge_length_fn: Option<Box<dyn Fn(EdgeId) -> f32 + Send + Sync>>,
    pub edge_elasticity_fn: Option<Box<dyn Fn(EdgeId) -> f32 + Send + Sync>>,
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
            randomize: true,
            compound_padding: 12.0,
            gravity_range: 380.0,
            gravity_compound: 1.0,
            gravity_range_compound: 1.5,
            tile: true,
            tiling_padding_horizontal: 10.0,
            tiling_padding_vertical: 10.0,
            pack_components: true,
            node_dimensions_include_labels: false,
            current_phase_idx: 0,
            constraints: FCoseConstraints::default(),
            node_repulsion_fn: None,
            ideal_edge_length_fn: None,
            edge_elasticity_fn: None,
        }
    }
}

impl FCoseLayout {
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_ideal_edge_length(mut self, length: f32) -> Self {
        self.ideal_edge_length = length;
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

    pub fn with_initial_temp(mut self, temp: f32) -> Self {
        self.initial_temp = temp;
        self
    }

    pub fn with_cooling_factor(mut self, factor: f32) -> Self {
        self.cooling_factor = factor;
        self
    }

    pub fn with_randomize(mut self, randomize: bool) -> Self {
        self.randomize = randomize;
        self
    }

    pub fn with_compound_padding(mut self, padding: f32) -> Self {
        self.compound_padding = padding;
        self
    }

    pub fn with_gravity_range(mut self, range: f32) -> Self {
        self.gravity_range = range;
        self
    }

    pub fn with_gravity_compound(mut self, g: f32) -> Self {
        self.gravity_compound = g;
        self
    }

    pub fn with_gravity_range_compound(mut self, r: f32) -> Self {
        self.gravity_range_compound = r;
        self
    }

    pub fn with_tile(mut self, tile: bool) -> Self {
        self.tile = tile;
        self
    }

    pub fn with_tiling_padding_horizontal(mut self, p: f32) -> Self {
        self.tiling_padding_horizontal = p;
        self
    }

    pub fn with_tiling_padding_vertical(mut self, p: f32) -> Self {
        self.tiling_padding_vertical = p;
        self
    }

    pub fn with_pack_components(mut self, pack: bool) -> Self {
        self.pack_components = pack;
        self
    }

    pub fn with_node_dimensions_include_labels(mut self, include: bool) -> Self {
        self.node_dimensions_include_labels = include;
        self
    }

    pub fn with_constraints(mut self, constraints: FCoseConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn with_fixed_node_constraint(mut self, constraint: FixedNodeConstraint) -> Self {
        self.constraints.fixed_nodes.push(constraint);
        self
    }

    pub fn with_alignment_constraint(mut self, alignment: AlignmentConstraint) -> Self {
        self.constraints.alignment = alignment;
        self
    }

    pub fn with_relative_placement_constraint(mut self, relative: RelativePlacementConstraint) -> Self {
        self.constraints.relative_placement.push(relative);
        self
    }

    pub fn with_node_repulsion_fn<F: Fn(NodeId) -> f32 + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.node_repulsion_fn = Some(Box::new(f));
        self
    }

    pub fn with_ideal_edge_length_fn<F: Fn(EdgeId) -> f32 + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.ideal_edge_length_fn = Some(Box::new(f));
        self
    }

    pub fn with_edge_elasticity_fn<F: Fn(EdgeId) -> f32 + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.edge_elasticity_fn = Some(Box::new(f));
        self
    }
}



impl<S: Copy + Default> Layout<S> for FCoseLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 { return; }

        let mut is_parent = vec![false; n];
        for i in 0..n {
            if state.hierarchy.first_child.get(i).is_some() {
                is_parent[i] = true;
            }
        }
        let leaf_indices: Vec<usize> = (0..n).filter(|&i| !is_parent[i]).collect();
        let leaf_count = leaf_indices.len();
        if leaf_count == 0 { return; }

        let get_leaf_descendants = |node_idx: usize, h_state: &GraphState<S>, is_p: &[bool]| -> Vec<usize> {
            let mut leaves = Vec::new();
            let mut stack = vec![node_idx];
            while let Some(curr) = stack.pop() {
                if !is_p[curr] {
                    leaves.push(curr);
                } else {
                    let mut next_child = *h_state.hierarchy.first_child.get(curr);
                    while let Some(child_id) = next_child {
                        if let Some(&child_idx) = h_state.node_keys.get(child_id) {
                            stack.push(child_idx);
                            next_child = *h_state.hierarchy.next_sibling.get(child_idx);
                        } else {
                            break;
                        }
                    }
                }
            }
            leaves
        };

        let is_ancestor = |mut child_idx: usize, parent_idx: usize, h_state: &GraphState<S>| -> bool {
            let parent_id = h_state.node_index_to_id[parent_idx];
            while let Some(p_id) = *h_state.hierarchy.parent.get(child_idx) {
                if p_id == parent_id {
                    return true;
                }
                if let Some(&p_idx) = h_state.node_keys.get(p_id) {
                    child_idx = p_idx;
                } else {
                    break;
                }
            }
            false
        };

        let mut all_zero = true;
        for &idx in &leaf_indices {
            let pos = *state.positions.get(idx);
            if pos.x != 0.0 || pos.y != 0.0 {
                all_zero = false;
                break;
            }
        }

        let mut components: Vec<Vec<usize>> = Vec::new();

        if self.randomize || all_zero {
            let mut adj = vec![Vec::new(); n];
            let add_local_edge = |u_global: usize, v_global: usize, adj_list: &mut Vec<Vec<usize>>| {
                if u_global != v_global {
                    adj_list[u_global].push(v_global);
                    adj_list[v_global].push(u_global);
                }
            };

            for edge_idx in 0..state.edges.len() {
                let src_id = *state.edge_sources.get(edge_idx);
                let tgt_id = *state.edge_targets.get(edge_idx);
                if let (Some(&src_global), Some(&tgt_global)) = (state.node_keys.get(src_id), state.node_keys.get(tgt_id)) {
                    add_local_edge(src_global, tgt_global, &mut adj);
                }
            }

            for idx in 0..n {
                if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
                    if let Some(&parent_idx) = state.node_keys.get(parent_id) {
                        add_local_edge(idx, parent_idx, &mut adj);
                    }
                }
            }

            let mut visited = vec![false; n];
            for start in 0..n {
                if visited[start] { continue; }
                let mut comp = Vec::new();
                let mut q = std::collections::VecDeque::new();
                q.push_back(start);
                visited[start] = true;
                while let Some(curr) = q.pop_front() {
                    comp.push(curr);
                    for &next in &adj[curr] {
                        if !visited[next] {
                            visited[next] = true;
                            q.push_back(next);
                        }
                    }
                }
                components.push(comp);
            }

            let draft_coords = spectral_placement_landmark(state, 30, self.ideal_edge_length);

            let mut total_dist = 0.0f32;
            let mut edge_cnt = 0;
            for i in 0..n {
                for &j in &adj[i] {
                    if i < j {
                        total_dist += (draft_coords[i] - draft_coords[j]).len();
                        edge_cnt += 1;
                    }
                }
            }
            let scale = if edge_cnt > 0 && total_dist > 0.01 {
                (edge_cnt as f32 * self.ideal_edge_length) / total_dist
            } else {
                1.0
            };

            for idx in 0..n {
                state.positions.set(idx, draft_coords[idx] * scale);
            }
        }

        let project_constraints = |h_state: &mut GraphState<S>, cons: &FCoseConstraints| {
            let num_nodes = h_state.node_index_to_id.len();
            let mut fixed_xs: Vec<Option<f32>> = vec![None; num_nodes];
            let mut fixed_ys: Vec<Option<f32>> = vec![None; num_nodes];

            for c in &cons.fixed_nodes {
                if let Some(&idx) = h_state.node_keys.get(c.node_id) {
                    h_state.positions.set(idx, c.position);
                    fixed_xs[idx] = Some(c.position.x);
                    fixed_ys[idx] = Some(c.position.y);
                }
            }

            let mut xs: Vec<f32> = (0..num_nodes).map(|i| h_state.positions.get(i).x).collect();
            let mut ys: Vec<f32> = (0..num_nodes).map(|i| h_state.positions.get(i).y).collect();

            let x_constraints: Vec<(usize, usize, f32)> = cons
                .relative_placement
                .iter()
                .filter_map(|c| match c {
                    RelativePlacementConstraint::LeftRight { left, right, gap } => {
                        let l = *h_state.node_keys.get(*left)?;
                        let r = *h_state.node_keys.get(*right)?;
                        Some((l, r, *gap))
                    }
                    _ => None,
                })
                .collect();

            let y_constraints: Vec<(usize, usize, f32)> = cons
                .relative_placement
                .iter()
                .filter_map(|c| match c {
                    RelativePlacementConstraint::TopBottom { top, bottom, gap } => {
                        let t = *h_state.node_keys.get(*top)?;
                        let b = *h_state.node_keys.get(*bottom)?;
                        Some((t, b, *gap))
                    }
                    _ => None,
                })
                .collect();

            for _pass in 0..5 {
                for group in &cons.alignment.vertical {
                    let valid_idxs: Vec<usize> = group
                        .iter()
                        .filter_map(|&id| h_state.node_keys.get(id).copied())
                        .collect();
                    if !valid_idxs.is_empty() {
                        let sum_x: f32 = valid_idxs.iter().map(|&idx| xs[idx]).sum();
                        let avg_x = sum_x / valid_idxs.len() as f32;
                        for &idx in &valid_idxs {
                            if fixed_xs[idx].is_none() {
                                xs[idx] = avg_x;
                            }
                        }
                    }
                }
                for group in &cons.alignment.horizontal {
                    let valid_idxs: Vec<usize> = group
                        .iter()
                        .filter_map(|&id| h_state.node_keys.get(id).copied())
                        .collect();
                    if !valid_idxs.is_empty() {
                        let sum_y: f32 = valid_idxs.iter().map(|&idx| ys[idx]).sum();
                        let avg_y = sum_y / valid_idxs.len() as f32;
                        for &idx in &valid_idxs {
                            if fixed_ys[idx].is_none() {
                                ys[idx] = avg_y;
                            }
                        }
                    }
                }

                solve_separation_constraints(&mut xs, &x_constraints, &fixed_xs);
                solve_separation_constraints(&mut ys, &y_constraints, &fixed_ys);
            }

            for (i, &x) in xs.iter().enumerate() {
                let mut p = *h_state.positions.get(i);
                p.x = x;
                h_state.positions.set(i, p);
            }
            for (i, &y) in ys.iter().enumerate() {
                let mut p = *h_state.positions.get(i);
                p.y = y;
                h_state.positions.set(i, p);
            }

            for group in &cons.alignment.vertical {
                let valid_idxs: Vec<usize> = group
                    .iter()
                    .filter_map(|&id| h_state.node_keys.get(id).copied())
                    .collect();
                if !valid_idxs.is_empty() {
                    let sum_x: f32 = valid_idxs.iter().map(|&idx| h_state.positions.get(idx).x).sum();
                    let avg_x = sum_x / valid_idxs.len() as f32;
                    for &idx in &valid_idxs {
                        if fixed_xs[idx].is_none() {
                            let mut p = *h_state.positions.get(idx);
                            p.x = avg_x;
                            h_state.positions.set(idx, p);
                        }
                    }
                }
            }
            for group in &cons.alignment.horizontal {
                let valid_idxs: Vec<usize> = group
                    .iter()
                    .filter_map(|&id| h_state.node_keys.get(id).copied())
                    .collect();
                if !valid_idxs.is_empty() {
                    let sum_y: f32 = valid_idxs.iter().map(|&idx| h_state.positions.get(idx).y).sum();
                    let avg_y = sum_y / valid_idxs.len() as f32;
                    for &idx in &valid_idxs {
                        if fixed_ys[idx].is_none() {
                            let mut p = *h_state.positions.get(idx);
                            p.y = avg_y;
                            h_state.positions.set(idx, p);
                        }
                    }
                }
            }

            for c in &cons.fixed_nodes {
                if let Some(&idx) = h_state.node_keys.get(c.node_id) {
                    h_state.positions.set(idx, c.position);
                }
            }
        };

        if !self.constraints.fixed_nodes.is_empty() {
            let mut draft_sum = Vec2::default();
            let mut target_sum = Vec2::default();
            let mut cnt = 0;
            for c in &self.constraints.fixed_nodes {
                if let Some(&idx) = state.node_keys.get(c.node_id) {
                    draft_sum += *state.positions.get(idx);
                    target_sum += c.position;
                    cnt += 1;
                }
            }
            if cnt > 0 {
                let trans = (target_sum / cnt as f32) - (draft_sum / cnt as f32);
                for idx in 0..n {
                    let p = *state.positions.get(idx);
                    state.positions.set(idx, p + trans);
                }
            }
        }

        project_constraints(state, &self.constraints);

        let mut temp = self.initial_temp;

        for _step in 0..self.iterations {
            if temp < 0.1 { break; }

            let mut displacements_x = vec![0.0f32; n];
            let mut displacements_y = vec![0.0f32; n];

            if n > 100 && !is_parent.iter().any(|&p| p) {
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
                    for j in (i + 1)..n {
                        if is_ancestor(i, j, state) || is_ancestor(j, i, state) {
                            continue;
                        }

                        let pos_i = *state.positions.get(i);
                        let pos_j = *state.positions.get(j);
                        let size_i = *state.sizes.get(i);
                        let size_j = *state.sizes.get(j);

                        let dx = pos_j.x - pos_i.x;
                        let dy = pos_j.y - pos_i.y;
                        let dist = (dx * dx + dy * dy + 0.01).sqrt();

                        let p1 = find_clipping_point(pos_i, size_i, dx, dy);
                        let p2 = find_clipping_point(pos_j, size_j, -dx, -dy);
                        let border_dx = p2.x - p1.x;
                        let border_dy = p2.y - p1.y;
                        let border_dist = (border_dx * border_dx + border_dy * border_dy).sqrt().max(1.0);

                        let k_rep = self.node_repulsion;
                        let force = k_rep / (border_dist * border_dist);
                        let fx = -force * dx / dist;
                        let fy = -force * dy / dist;

                        if !is_parent[i] {
                            displacements_x[i] += fx;
                            displacements_y[i] += fy;
                        } else {
                            let leaves = get_leaf_descendants(i, state, &is_parent);
                            if !leaves.is_empty() {
                                let f_each_x = fx / leaves.len() as f32;
                                let f_each_y = fy / leaves.len() as f32;
                                for &leaf_idx in &leaves {
                                    displacements_x[leaf_idx] += f_each_x;
                                    displacements_y[leaf_idx] += f_each_y;
                                }
                            }
                        }

                        if !is_parent[j] {
                            displacements_x[j] -= fx;
                            displacements_y[j] -= fy;
                        } else {
                            let leaves = get_leaf_descendants(j, state, &is_parent);
                            if !leaves.is_empty() {
                                let f_each_x = -fx / leaves.len() as f32;
                                let f_each_y = -fy / leaves.len() as f32;
                                for &leaf_idx in &leaves {
                                    displacements_x[leaf_idx] += f_each_x;
                                    displacements_y[leaf_idx] += f_each_y;
                                }
                            }
                        }
                    }
                }
            }

            for idx in 0..state.edges.len() {
                let edge_id = state.edge_index_to_id[idx];
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

                let custom_ideal = if let Some(ref ideal_fn) = self.ideal_edge_length_fn {
                    ideal_fn(edge_id)
                } else {
                    self.ideal_edge_length
                };
                let custom_elasticity = if let Some(ref elasticity_fn) = self.edge_elasticity_fn {
                    elasticity_fn(edge_id)
                } else {
                    32.0
                };

                let depth = get_nesting_depth(state, src_node, tgt_node);
                let ideal = custom_ideal * self.nesting_factor.powi(depth as i32);

                let force_att = (ideal - l).powi(2) / custom_elasticity;
                let fx = force_att * lx / l;
                let fy = force_att * ly / l;

                displacements_x[src_idx] += fx;
                displacements_y[src_idx] += fy;
                displacements_x[tgt_idx] -= fx;
                displacements_y[tgt_idx] -= fy;
            }

            let mut center = Vec2::default();
            for &idx in &leaf_indices {
                center += *state.positions.get(idx);
            }
            center = center / leaf_count as f32;

            for &idx in &leaf_indices {
                let pos = *state.positions.get(idx);
                let dx = center.x - pos.x;
                let dy = center.y - pos.y;
                let d = (dx * dx + dy * dy).sqrt().max(0.01);
                let g = if let Some(_p_id) = *state.hierarchy.parent.get(idx) {
                    self.gravity_compound
                } else {
                    self.gravity
                };
                let range = if let Some(_p_id) = *state.hierarchy.parent.get(idx) {
                    self.gravity_range_compound
                } else {
                    self.gravity_range
                };
                if d <= range || range == 0.0 {
                    let fx = g * dx / d;
                    let fy = g * dy / d;

                    displacements_x[idx] += fx;
                    displacements_y[idx] += fy;
                }
            }

            for c in &self.constraints.fixed_nodes {
                if let Some(&idx) = state.node_keys.get(c.node_id) {
                    displacements_x[idx] = 0.0;
                    displacements_y[idx] = 0.0;
                }
            }

            for group in &self.constraints.alignment.vertical {
                let valid_idxs: Vec<usize> = group.iter()
                    .filter_map(|&id| state.node_keys.get(id).copied())
                    .collect();
                if !valid_idxs.is_empty() {
                    let sum_dx: f32 = valid_idxs.iter().map(|&idx| displacements_x[idx]).sum();
                    let avg_dx = sum_dx / valid_idxs.len() as f32;
                    for &idx in &valid_idxs {
                        displacements_x[idx] = avg_dx;
                    }
                }
            }

            for group in &self.constraints.alignment.horizontal {
                let valid_idxs: Vec<usize> = group.iter()
                    .filter_map(|&id| state.node_keys.get(id).copied())
                    .collect();
                if !valid_idxs.is_empty() {
                    let sum_dy: f32 = valid_idxs.iter().map(|&idx| displacements_y[idx]).sum();
                    let avg_dy = sum_dy / valid_idxs.len() as f32;
                    for &idx in &valid_idxs {
                        displacements_y[idx] = avg_dy;
                    }
                }
            }

            for &idx in &leaf_indices {
                let dx = displacements_x[idx];
                let dy = displacements_y[idx];
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > 0.01 {
                    let cap = dist.min(temp);
                    let capped_x = dx * cap / dist;
                    let capped_y = dy * cap / dist;

                    let old_pos = *state.positions.get(idx);
                    state.positions.set(idx, Vec2::new(old_pos.x + capped_x, old_pos.y + capped_y));
                }
            }

            project_constraints(state, &self.constraints);

            resolve_compound_bounds(state, &HashSet::new(), 12.0);

            temp *= self.cooling_factor;
        }

        let apply_push = |node_idx: usize, push: Vec2, h_state: &mut GraphState<S>, is_p: &[bool], cons: &FCoseConstraints| {
            if !is_p[node_idx] {
                let leaf_id = h_state.node_index_to_id[node_idx];
                if !cons.fixed_nodes.iter().any(|f| f.node_id == leaf_id) {
                    let p = h_state.positions.get_mut(node_idx);
                    p.x += push.x;
                    p.y += push.y;
                }
            } else {
                let leaf_descendants = get_leaf_descendants(node_idx, h_state, is_p);
                for &leaf_idx in &leaf_descendants {
                    let leaf_id = h_state.node_index_to_id[leaf_idx];
                    if !cons.fixed_nodes.iter().any(|f| f.node_id == leaf_id) {
                        let p = h_state.positions.get_mut(leaf_idx);
                        p.x += push.x;
                        p.y += push.y;
                    }
                }
            }
        };

        let padding = 12.0;
        for _ in 0..4 {
            for i in 0..n {
                for j in (i + 1)..n {
                    if is_ancestor(i, j, state) || is_ancestor(j, i, state) {
                        continue;
                    }

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

                        let push = Vec2::new(push_x, push_y);
                        apply_push(i, Vec2::new(-push.x, -push.y), state, &is_parent, &self.constraints);
                        apply_push(j, push, state, &is_parent, &self.constraints);
                    }
                }
            }
            project_constraints(state, &self.constraints);
            resolve_compound_bounds(state, &HashSet::new(), self.compound_padding);
        }

        if (self.pack_components || self.tile) && components.len() > 1 {
            let cols = (components.len() as f32).sqrt().ceil() as usize;
            let mut cur_x = 0.0f32;
            let mut cur_y = 0.0f32;
            let mut max_row_h = 0.0f32;

            for (idx, comp) in components.iter().enumerate() {
                if idx > 0 && idx % cols == 0 {
                    cur_x = 0.0;
                    cur_y += max_row_h + self.tiling_padding_vertical;
                    max_row_h = 0.0;
                }

                let mut min_x = f32::INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut max_y = f32::NEG_INFINITY;

                for &node_idx in comp {
                    let p = *state.positions.get(node_idx);
                    let s = *state.sizes.get(node_idx);
                    min_x = min_x.min(p.x - s.w / 2.0);
                    min_y = min_y.min(p.y - s.h / 2.0);
                    max_x = max_x.max(p.x + s.w / 2.0);
                    max_y = max_y.max(p.y + s.h / 2.0);
                }

                let comp_w = (max_x - min_x).max(10.0);
                let comp_h = (max_y - min_y).max(10.0);
                max_row_h = max_row_h.max(comp_h);

                let shift_x = cur_x - min_x;
                let shift_y = cur_y - min_y;

                for &node_idx in comp {
                    let p = state.positions.get_mut(node_idx);
                    p.x += shift_x;
                    p.y += shift_y;
                }

                cur_x += comp_w + self.tiling_padding_horizontal;
            }
        }

        crate::collision::resolve_overlaps(state, 10.0);
        project_constraints(state, &self.constraints);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
        self.current_phase_idx = 4;
    }
}

impl<S: Copy + Default> crate::traits::PhaseSteppableLayout<S> for FCoseLayout {
    type Phase = FCosePhase;

    fn phases(&self) -> &[Self::Phase] {
        &FCOSE_PHASES
    }

    fn current_phase(&self) -> Option<Self::Phase> {
        if self.current_phase_idx < FCOSE_PHASES.len() {
            Some(FCOSE_PHASES[self.current_phase_idx])
        } else {
            None
        }
    }

    fn step_next_phase(&mut self, state: &mut GraphState<S>) -> bool {
        if self.current_phase_idx >= FCOSE_PHASES.len() {
            return false;
        }
        self.current_phase_idx += 1;
        self.compute(state);
        self.current_phase_idx < FCOSE_PHASES.len()
    }
}

struct ConstraintBlock {
    vars: Vec<usize>,
    posn: f32,
}

fn solve_separation_constraints(
    x: &mut [f32],
    constraints: &[(usize, usize, f32)],
    fixed_pos: &[Option<f32>],
) {
    let n = x.len();
    if n == 0 || constraints.is_empty() {
        return;
    }
    let initial_x = x.to_vec();
    let mut block_of: Vec<usize> = (0..n).collect();
    let mut blocks: Vec<ConstraintBlock> = (0..n)
        .map(|i| ConstraintBlock {
            vars: vec![i],
            posn: fixed_pos.get(i).and_then(|&fp| fp).unwrap_or(x[i]),
        })
        .collect();
    let mut offsets = vec![0.0f32; n];

    let max_iters = constraints.len() * n + 100;
    for _iter in 0..max_iters {
        for i in 0..n {
            x[i] = blocks[block_of[i]].posn + offsets[i];
        }

        let violation = constraints
            .iter()
            .map(|&(l, r, gap)| (x[l] + gap - x[r], l, r, gap))
            .filter(|(v, ..)| *v > 1e-4)
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let Some((_, l, r, gap)) = violation else { break };

        let bl = block_of[l];
        let br = block_of[r];
        if bl == br {
            break;
        }

        let shift = offsets[l] + gap - offsets[r];
        for &w in &blocks[br].vars {
            offsets[w] += shift;
            block_of[w] = bl;
        }

        let vars_br = std::mem::take(&mut blocks[br].vars);
        blocks[bl].vars.extend(vars_br);

        let fixed_var = blocks[bl]
            .vars
            .iter()
            .find_map(|&v| fixed_pos.get(v).and_then(|&fp| fp).map(|pos| (v, pos)));

        if let Some((v_fixed, fp)) = fixed_var {
            blocks[bl].posn = fp - offsets[v_fixed];
        } else {
            let num_vars = blocks[bl].vars.len() as f32;
            let sum_target: f32 = blocks[bl].vars.iter().map(|&v| initial_x[v] - offsets[v]).sum();
            blocks[bl].posn = sum_target / num_vars;
        }
    }

    for i in 0..n {
        x[i] = blocks[block_of[i]].posn + offsets[i];
    }
}

fn spectral_placement_landmark<S: Copy>(
    state: &GraphState<S>,
    sample_size: usize,
    node_separation: f32,
) -> Vec<Vec2> {
    let n = state.node_index_to_id.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![Vec2::default()];
    }
    let sample_size = sample_size.min(n).max(1);

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

    let mut c = vec![vec![0.0f32; sample_size]; n];
    let mut min_dist = vec![f32::INFINITY; n];
    let mut pivot = 0usize;

    for col in 0..sample_size {
        let mut dist = vec![f32::INFINITY; n];
        let mut queue = std::collections::VecDeque::new();
        dist[pivot] = 0.0;
        queue.push_back(pivot);
        while let Some(u) = queue.pop_front() {
            for &v in &adj[u] {
                if dist[v].is_infinite() {
                    dist[v] = dist[u] + 1.0;
                    queue.push_back(v);
                }
            }
        }

        let max_finite = dist.iter().filter(|d| d.is_finite()).copied().fold(0.0f32, f32::max);
        let fallback_dist = if max_finite > 0.0 { max_finite * 2.0 } else { 4.0 };

        for i in 0..n {
            let d_val = if dist[i].is_finite() { dist[i] } else { fallback_dist };
            c[i][col] = d_val * node_separation;
            min_dist[i] = min_dist[i].min(c[i][col]);
        }
        pivot = (0..n)
            .max_by(|&a, &b| min_dist[a].partial_cmp(&min_dist[b]).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(0);
    }

    for row in &mut c {
        for v in row.iter_mut() {
            *v *= *v;
        }
    }

    let mut row_sums = vec![0.0f32; n];
    let mut col_sums = vec![0.0f32; sample_size];
    let mut grand_sum = 0.0f32;

    for i in 0..n {
        for m in 0..sample_size {
            let val = c[i][m];
            row_sums[i] += val;
            col_sums[m] += val;
            grand_sum += val;
        }
    }

    let k_f = sample_size as f32;
    let n_f = n as f32;
    let grand_avg = grand_sum / (n_f * k_f);

    let mut b = vec![vec![0.0f32; sample_size]; n];
    for i in 0..n {
        let r_avg = row_sums[i] / k_f;
        for m in 0..sample_size {
            let c_avg = col_sums[m] / n_f;
            b[i][m] = -0.5 * (c[i][m] - r_avg - c_avg + grand_avg);
        }
    }

    let mut k_mat = vec![0.0f32; sample_size * sample_size];
    for m1 in 0..sample_size {
        for m2 in 0..sample_size {
            let mut sum = 0.0f32;
            for i in 0..n {
                sum += b[i][m1] * b[i][m2];
            }
            k_mat[m1 * sample_size + m2] = sum;
        }
    }

    let mut seed = 42u64;
    let mut lcg_rand = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 32) as f32 / u32::MAX as f32
    };

    let power_iteration_k = |matrix: &[f32], sz: usize, rand_fn: &mut dyn FnMut() -> f32| -> (f32, Vec<f32>) {
        let mut u_vec = vec![0.0f32; sz];
        for val in u_vec.iter_mut() {
            *val = rand_fn() - 0.5;
        }
        let norm = u_vec.iter().map(|&x| x * x).sum::<f32>().sqrt().max(1e-5);
        for val in u_vec.iter_mut() {
            *val /= norm;
        }

        for _ in 0..100 {
            let mut w_vec = vec![0.0f32; sz];
            for row in 0..sz {
                for col in 0..sz {
                    w_vec[row] += matrix[row * sz + col] * u_vec[col];
                }
            }
            let w_norm = w_vec.iter().map(|&x| x * x).sum::<f32>().sqrt();
            if w_norm < 1e-5 {
                break;
            }
            for row in 0..sz {
                u_vec[row] = w_vec[row] / w_norm;
            }
        }

        let mut lamb = 0.0f32;
        let mut mu_vec = vec![0.0f32; sz];
        for row in 0..sz {
            for col in 0..sz {
                mu_vec[row] += matrix[row * sz + col] * u_vec[col];
            }
            lamb += u_vec[row] * mu_vec[row];
        }

        (lamb, u_vec)
    };

    let (lambda_1, v_1) = power_iteration_k(&k_mat, sample_size, &mut lcg_rand);
    let mut k_deflated = k_mat.clone();
    if lambda_1 > 0.0 {
        for m1 in 0..sample_size {
            for m2 in 0..sample_size {
                k_deflated[m1 * sample_size + m2] -= lambda_1 * v_1[m1] * v_1[m2];
            }
        }
    }
    let (_lambda_2, v_2) = power_iteration_k(&k_deflated, sample_size, &mut lcg_rand);

    let mut coords = vec![Vec2::default(); n];
    for i in 0..n {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        for m in 0..sample_size {
            x += b[i][m] * v_1[m];
            y += b[i][m] * v_2[m];
        }
        coords[i] = Vec2::new(x, y);
    }

    coords
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fcose_layout_builder_configuration() {
        let mut dummy_state = GraphState::<()>::new();
        let n1 = dummy_state.add_node(Vec2::default(), graphene_core::Size2::default());
        let n2 = dummy_state.add_node(Vec2::default(), graphene_core::Size2::default());

        let layout = FCoseLayout::default()
            .with_iterations(300)
            .with_ideal_edge_length(75.0)
            .with_nesting_factor(1.5)
            .with_gravity(2.0)
            .with_node_repulsion(5000.0)
            .with_initial_temp(80.0)
            .with_cooling_factor(0.9)
            .with_randomize(false)
            .with_compound_padding(18.0)
            .with_gravity_range(400.0)
            .with_gravity_compound(1.2)
            .with_gravity_range_compound(1.8)
            .with_tile(false)
            .with_tiling_padding_horizontal(15.0)
            .with_tiling_padding_vertical(15.0)
            .with_pack_components(false)
            .with_node_dimensions_include_labels(true)
            .with_fixed_node_constraint(FixedNodeConstraint {
                node_id: n1,
                position: Vec2::new(10.0, 20.0),
            })
            .with_alignment_constraint(AlignmentConstraint {
                horizontal: vec![vec![n1, n2]],
                vertical: vec![],
            })
            .with_relative_placement_constraint(RelativePlacementConstraint::LeftRight {
                left: n1,
                right: n2,
                gap: 30.0,
            })
            .with_node_repulsion_fn(|_id| 6000.0)
            .with_ideal_edge_length_fn(|_id| 80.0)
            .with_edge_elasticity_fn(|_id| 1.5);

        assert_eq!(layout.iterations, 300);
        assert_eq!(layout.ideal_edge_length, 75.0);
        assert_eq!(layout.nesting_factor, 1.5);
        assert_eq!(layout.gravity, 2.0);
        assert_eq!(layout.node_repulsion, 5000.0);
        assert_eq!(layout.initial_temp, 80.0);
        assert_eq!(layout.cooling_factor, 0.9);
        assert_eq!(layout.randomize, false);
        assert_eq!(layout.compound_padding, 18.0);
        assert_eq!(layout.gravity_range, 400.0);
        assert_eq!(layout.gravity_compound, 1.2);
        assert_eq!(layout.gravity_range_compound, 1.8);
        assert_eq!(layout.tile, false);
        assert_eq!(layout.tiling_padding_horizontal, 15.0);
        assert_eq!(layout.tiling_padding_vertical, 15.0);
        assert_eq!(layout.pack_components, false);
        assert_eq!(layout.node_dimensions_include_labels, true);
        assert_eq!(layout.constraints.fixed_nodes.len(), 1);
        assert_eq!(layout.constraints.alignment.horizontal.len(), 1);
        assert_eq!(layout.constraints.relative_placement.len(), 1);
        assert!(layout.node_repulsion_fn.is_some());
        assert!(layout.ideal_edge_length_fn.is_some());
        assert!(layout.edge_elasticity_fn.is_some());
    }
}
