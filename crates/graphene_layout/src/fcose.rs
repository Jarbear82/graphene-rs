use crate::cose::find_clipping_point;
use crate::traits::{resolve_compound_bounds, Layout};
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

pub struct FCoseLayout {
    pub iterations: usize,
    pub ideal_edge_length: f32,
    pub nesting_factor: f32,
    pub gravity: f32,
    pub node_repulsion: f32,
    pub initial_temp: f32,
    pub cooling_factor: f32,
    pub randomize: bool,

    pub constraints: FCoseConstraints,

    pub node_repulsion_fn: Option<Box<dyn Fn(NodeId) -> f32>>,
    pub ideal_edge_length_fn: Option<Box<dyn Fn(EdgeId) -> f32>>,
    pub edge_elasticity_fn: Option<Box<dyn Fn(EdgeId) -> f32>>,
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
            constraints: FCoseConstraints::default(),
            node_repulsion_fn: None,
            ideal_edge_length_fn: None,
            edge_elasticity_fn: None,
        }
    }
}

impl FCoseLayout {
    pub fn with_constraints(mut self, constraints: FCoseConstraints) -> Self {
        self.constraints = constraints;
        self
    }

    pub fn with_node_repulsion_fn<F: Fn(NodeId) -> f32 + 'static>(mut self, f: F) -> Self {
        self.node_repulsion_fn = Some(Box::new(f));
        self
    }

    pub fn with_ideal_edge_length_fn<F: Fn(EdgeId) -> f32 + 'static>(mut self, f: F) -> Self {
        self.ideal_edge_length_fn = Some(Box::new(f));
        self
    }

    pub fn with_edge_elasticity_fn<F: Fn(EdgeId) -> f32 + 'static>(mut self, f: F) -> Self {
        self.edge_elasticity_fn = Some(Box::new(f));
        self
    }
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
            let mut components = Vec::new();
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

            let mut ext_adj = adj.clone();
            if components.len() > 1 {
                let dummy = n;
                ext_adj.push(Vec::new());
                for comp in &components {
                    let rep = comp[0];
                    ext_adj[dummy].push(rep);
                    ext_adj[rep].push(dummy);
                }
            }

            let total_nodes = ext_adj.len();
            let mut dists = vec![vec![f32::INFINITY; total_nodes]; total_nodes];
            for start in 0..total_nodes {
                dists[start][start] = 0.0;
                let mut q = std::collections::VecDeque::new();
                q.push_back(start);
                while let Some(curr) = q.pop_front() {
                    let d_curr = dists[start][curr];
                    for &next in &ext_adj[curr] {
                        if dists[start][next] == f32::INFINITY {
                            dists[start][next] = d_curr + 1.0;
                            q.push_back(next);
                        }
                    }
                }
            }

            let mut d_matrix = vec![vec![0.0f32; n]; n];
            for i in 0..n {
                for j in 0..n {
                    let val = dists[i][j];
                    d_matrix[i][j] = if val.is_finite() { val * val } else { 16.0 };
                }
            }

            let mut row_sums = vec![0.0f32; n];
            let mut total_sum = 0.0f32;
            for i in 0..n {
                let mut r_sum = 0.0f32;
                for j in 0..n {
                    r_sum += d_matrix[i][j];
                }
                row_sums[i] = r_sum;
                total_sum += r_sum;
            }

            let mut b_matrix = vec![vec![0.0f32; n]; n];
            let l_f = n as f32;
            for i in 0..n {
                for j in 0..n {
                    b_matrix[i][j] = -0.5 * (d_matrix[i][j] - row_sums[i] / l_f - row_sums[j] / l_f + total_sum / (l_f * l_f));
                }
            }

            let mut seed = 42u64;
            let mut lcg_rand = || {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                (seed >> 32) as f32 / u32::MAX as f32
            };

            let power_iteration = |matrix: &[Vec<f32>], rand_fn: &mut dyn FnMut() -> f32| -> (f32, Vec<f32>) {
                let sz = matrix.len();
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
                            w_vec[row] += matrix[row][col] * u_vec[col];
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
                        mu_vec[row] += matrix[row][col] * u_vec[col];
                    }
                    lamb += u_vec[row] * mu_vec[row];
                }

                (lamb, u_vec)
            };

            let (lambda_1, v_1) = power_iteration(&b_matrix, &mut lcg_rand);
            let mut b_deflated = b_matrix.clone();
            if lambda_1 > 0.0 {
                for i in 0..n {
                    for j in 0..n {
                        b_deflated[i][j] -= lambda_1 * v_1[i] * v_1[j];
                    }
                }
            }
            let (lambda_2, v_2) = power_iteration(&b_deflated, &mut lcg_rand);

            let l1 = lambda_1.max(0.0).sqrt();
            let l2 = lambda_2.max(0.0).sqrt();
            let mut draft_coords = vec![Vec2::default(); n];
            for i in 0..n {
                draft_coords[i] = Vec2::new(l1 * v_1[i], l2 * v_2[i]);
            }

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
            for c in &cons.fixed_nodes {
                if let Some(&idx) = h_state.node_keys.get(c.node_id) {
                    h_state.positions.set(idx, c.position);
                }
            }

            for group in &cons.alignment.vertical {
                let valid_idxs: Vec<usize> = group.iter()
                    .filter_map(|&id| h_state.node_keys.get(id).copied())
                    .collect();
                if !valid_idxs.is_empty() {
                    let sum_x: f32 = valid_idxs.iter().map(|&idx| h_state.positions.get(idx).x).sum();
                    let avg_x = sum_x / valid_idxs.len() as f32;
                    for &idx in &valid_idxs {
                        let mut p = *h_state.positions.get(idx);
                        p.x = avg_x;
                        h_state.positions.set(idx, p);
                    }
                }
            }
            for group in &cons.alignment.horizontal {
                let valid_idxs: Vec<usize> = group.iter()
                    .filter_map(|&id| h_state.node_keys.get(id).copied())
                    .collect();
                if !valid_idxs.is_empty() {
                    let sum_y: f32 = valid_idxs.iter().map(|&idx| h_state.positions.get(idx).y).sum();
                    let avg_y = sum_y / valid_idxs.len() as f32;
                    for &idx in &valid_idxs {
                        let mut p = *h_state.positions.get(idx);
                        p.y = avg_y;
                        h_state.positions.set(idx, p);
                    }
                }
            }

            for _ in 0..5 {
                for rel in &cons.relative_placement {
                    match rel {
                        &RelativePlacementConstraint::LeftRight { left, right, gap } => {
                            if let (Some(&l_idx), Some(&r_idx)) = (h_state.node_keys.get(left), h_state.node_keys.get(right)) {
                                let l_pos = *h_state.positions.get(l_idx);
                                let r_pos = *h_state.positions.get(r_idx);
                                if r_pos.x < l_pos.x + gap {
                                    let overlap = (l_pos.x + gap) - r_pos.x;
                                    let mut new_l = l_pos;
                                    let mut new_r = r_pos;
                                    let l_fixed = cons.fixed_nodes.iter().any(|f| f.node_id == left);
                                    let r_fixed = cons.fixed_nodes.iter().any(|f| f.node_id == right);
                                    if l_fixed && !r_fixed {
                                        new_r.x += overlap;
                                    } else if !l_fixed && r_fixed {
                                        new_l.x -= overlap;
                                    } else if !l_fixed && !r_fixed {
                                        new_l.x -= overlap * 0.5;
                                        new_r.x += overlap * 0.5;
                                    }
                                    h_state.positions.set(l_idx, new_l);
                                    h_state.positions.set(r_idx, new_r);
                                }
                            }
                        }
                        &RelativePlacementConstraint::TopBottom { top, bottom, gap } => {
                            if let (Some(&t_idx), Some(&b_idx)) = (h_state.node_keys.get(top), h_state.node_keys.get(bottom)) {
                                let t_pos = *h_state.positions.get(t_idx);
                                let b_pos = *h_state.positions.get(b_idx);
                                if b_pos.y < t_pos.y + gap {
                                    let overlap = (t_pos.y + gap) - b_pos.y;
                                    let mut new_t = t_pos;
                                    let mut new_b = b_pos;
                                    let t_fixed = cons.fixed_nodes.iter().any(|f| f.node_id == top);
                                    let b_fixed = cons.fixed_nodes.iter().any(|f| f.node_id == bottom);
                                    if t_fixed && !b_fixed {
                                        new_b.y += overlap;
                                    } else if !t_fixed && b_fixed {
                                        new_t.y -= overlap;
                                    } else if !t_fixed && !b_fixed {
                                        new_t.y -= overlap * 0.5;
                                        new_b.y += overlap * 0.5;
                                    }
                                    h_state.positions.set(t_idx, new_t);
                                    h_state.positions.set(b_idx, new_b);
                                }
                            }
                        }
                    }
                }
                for group in &cons.alignment.vertical {
                    let valid_idxs: Vec<usize> = group.iter()
                        .filter_map(|&id| h_state.node_keys.get(id).copied())
                        .collect();
                    if !valid_idxs.is_empty() {
                        let sum_x: f32 = valid_idxs.iter().map(|&idx| h_state.positions.get(idx).x).sum();
                        let avg_x = sum_x / valid_idxs.len() as f32;
                        for &idx in &valid_idxs {
                            let mut p = *h_state.positions.get(idx);
                            p.x = avg_x;
                            h_state.positions.set(idx, p);
                        }
                    }
                }
                for group in &cons.alignment.horizontal {
                    let valid_idxs: Vec<usize> = group.iter()
                        .filter_map(|&id| h_state.node_keys.get(id).copied())
                        .collect();
                    if !valid_idxs.is_empty() {
                        let sum_y: f32 = valid_idxs.iter().map(|&idx| h_state.positions.get(idx).y).sum();
                        let avg_y = sum_y / valid_idxs.len() as f32;
                        for &idx in &valid_idxs {
                            let mut p = *h_state.positions.get(idx);
                            p.y = avg_y;
                            h_state.positions.set(idx, p);
                        }
                    }
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
                let fx = self.gravity * dx / d;
                let fy = self.gravity * dy / d;

                displacements_x[idx] += fx;
                displacements_y[idx] += fy;
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
            resolve_compound_bounds(state, &HashSet::new(), 12.0);
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
