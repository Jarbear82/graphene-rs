use crate::traits::Layout;
use graphene_core::{math::Size2, math::Vec2, GraphState, HierarchyExt, NodeId};
use std::collections::{HashMap, HashSet};

/// Compound node hierarchical layout.
///
/// Reference: Hierarchical layout for compound parent/child graphs.
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
        let collapsed = HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, self.padding);
    }
}

/// Generic multi-level recursive hierarchical post-order layout adapter.
///
/// Reference: Hierarchical Layout for Compound Graphs (classic post-order inclusion tree layout).
pub struct HierarchicalLayout<L> {
    pub sub_layout: L,
    pub padding: f32,
}

impl<L> HierarchicalLayout<L> {
    pub fn new(sub_layout: L) -> Self {
        Self {
            sub_layout,
            padding: 20.0,
        }
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.padding = padding;
        self
    }
}

impl<S: Copy + Default, L: Layout<S>> Layout<S> for HierarchicalLayout<L> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let mut depth_and_parent: Vec<(usize, NodeId)> = Vec::new();
        for idx in 0..n {
            let id = state.node_index_to_id[idx];
            if state.is_parent(idx) {
                let depth = state.get_nesting_depth(id);
                depth_and_parent.push((depth, id));
            }
        }

        depth_and_parent.sort_by(|a, b| b.0.cmp(&a.0));

        for (_, parent_id) in depth_and_parent {
            let Some(&p_idx) = state.node_keys.get(parent_id) else { continue };

            let children: Vec<NodeId> = state
                .node_index_to_id
                .iter()
                .enumerate()
                .filter_map(|(idx, &id)| {
                    if *state.hierarchy.parent.get(idx) == Some(parent_id) {
                        Some(id)
                    } else {
                        None
                    }
                })
                .collect();

            if children.is_empty() {
                continue;
            }

            let mut sub_state: GraphState<S> = GraphState::new();
            let mut mapping = HashMap::new();
            let mut reverse_mapping = HashMap::new();

            for &c_id in &children {
                let Some(&c_idx) = state.node_keys.get(c_id) else { continue };
                let pos = *state.positions.get(c_idx);
                let size = *state.sizes.get(c_idx);
                let new_id = sub_state.add_node(pos, size);
                mapping.insert(c_id, new_id);
                reverse_mapping.insert(new_id, c_id);
            }

            for e_idx in 0..state.edges.len() {
                let src = *state.edge_sources.get(e_idx);
                let tgt = *state.edge_targets.get(e_idx);
                if mapping.contains_key(&src) && mapping.contains_key(&tgt) {
                    let sub_src = mapping[&src];
                    let sub_tgt = mapping[&tgt];
                    sub_state.add_edge(sub_src, sub_tgt, state.edges.get(e_idx).clone());
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

            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;
            let w = (max_x - min_x) + 2.0 * self.padding;
            let h = (max_y - min_y) + 2.0 * self.padding;

            state.positions.set(p_idx, Vec2::new(center_x, center_y));
            state.sizes.set(p_idx, Size2::new(w, h));

            for i in 0..sub_state.node_index_to_id.len() {
                let sub_id = sub_state.node_index_to_id[i];
                if let Some(&orig_id) = reverse_mapping.get(&sub_id) {
                    if let Some(&orig_idx) = state.node_keys.get(orig_id) {
                        state.positions.set(orig_idx, *sub_state.positions.get(i));
                    }
                }
            }
        }

        self.sub_layout.compute(state);
        let collapsed = HashSet::new();
        crate::traits::resolve_compound_bounds(state, &collapsed, self.padding);
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, self.padding);
    }
}

/// Hybrid compound layout combining bottom-up hierarchical seeding with global polishing.
///
/// Reference: Hybrid Compound Layout with post-order seeding and global polishing.
pub struct HybridCompoundLayout<L, P> {
    pub seed_layout: HierarchicalLayout<L>,
    pub polish_layout: P,
}

impl<L, P> HybridCompoundLayout<L, P> {
    pub fn new(seed_layout: L, polish_layout: P) -> Self {
        Self {
            seed_layout: HierarchicalLayout::new(seed_layout),
            polish_layout,
        }
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.seed_layout = self.seed_layout.with_padding(padding);
        self
    }
}

impl<S: Copy + Default, L: Layout<S>, P: Layout<S>> Layout<S> for HybridCompoundLayout<L, P> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        self.seed_layout.compute(state);
        self.polish_layout.compute(state);
    }
}

/// Concentric hub graph layout.
///
/// Reference: Concentric layout centered on highest-degree hub nodes.
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

        let mut degrees = vec![0usize; n];
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                if u < n && v < n {
                    degrees[u] += 1;
                    degrees[v] += 1;
                }
            }
        }

        let mut hubs = Vec::new();
        let mut peers = Vec::new();
        for i in 0..n {
            let id = state.node_index_to_id[i];
            if degrees[i] >= self.hub_threshold {
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

        let collapsed = HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}

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

/// Regional partition layout.
///
/// Reference: Partitioned layout operating independently on sub-regions.
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

        let collapsed = HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}
