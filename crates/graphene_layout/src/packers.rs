use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState, NodeId};
use std::collections::{HashMap, HashSet};

/// Disconnected component packing layout wrapper.
///
/// Reference: Disconnected component bounding box packing.
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
                    let data = state.edges[idx].clone();
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

        let collapsed = std::collections::HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}
