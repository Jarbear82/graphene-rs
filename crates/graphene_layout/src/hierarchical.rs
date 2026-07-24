use crate::traits::Layout;
use graphene_core::{math::Vec2, EdgeId, GraphState, NodeId};
use std::collections::{HashMap, HashSet};

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
                if nodes_in_layer.is_empty() {
                    continue;
                }
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
