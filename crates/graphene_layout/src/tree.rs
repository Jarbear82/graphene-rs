use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState, NodeId};
use std::collections::HashMap;

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

impl ReingoldTilfordLayout {
    pub fn with_sibling_spacing(mut self, spacing: f32) -> Self {
        self.sibling_spacing = spacing;
        self
    }

    pub fn with_level_spacing(mut self, spacing: f32) -> Self {
        self.level_spacing = spacing;
        self
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

        let mut tree_x_offset = 0.0f32;
        for &root in &roots {
            if let Some(&root_tree_idx) = node_to_tree_idx.get(&root) {
                first_walk(root_tree_idx, 0, &mut nodes, self.sibling_spacing, self.level_spacing);
                second_walk(root_tree_idx, 0, tree_x_offset, &nodes, state, self.level_spacing);
                tree_x_offset += self.sibling_spacing * 3.0;
            }
        }

        crate::collision::resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
