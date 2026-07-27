use graphene_core::{math::Size2, math::Vec2, GraphState, HierarchyExt, NodeId};
use std::collections::{HashMap, HashSet};

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
                        flat_state.reparent_node(new_child_id, Some(new_parent_id));
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
    crate::collision::resolve_overlaps(state, 10.0);
    resolve_compound_bounds(state, collapsed_parents, 20.0);
    state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
}

pub fn get_nesting_depth<S: Copy>(state: &GraphState<S>, u: NodeId, v: NodeId) -> usize {
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
