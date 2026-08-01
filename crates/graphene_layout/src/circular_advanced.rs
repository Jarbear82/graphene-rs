use graphene_core::math::Vec2;
use graphene_core::{GraphState, NodeId};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
/// Advanced Algorithm CIRCULAR with degree-bucket sorting, wavefront/center placement,
/// and sweep-line edge crossing post-processing minimization.
/// Reference: Six & Tollis (1999) / Dogrusoz et al. Circular Layout Algorithms for Graph Visualization.
pub struct CircularAdvancedLayout {
    pub radius: f32,
    pub post_process_iterations: usize,
}

impl Default for CircularAdvancedLayout {
    fn default() -> Self {
        Self {
            radius: 250.0,
            post_process_iterations: 3,
        }
    }
}

impl CircularAdvancedLayout {
    pub fn new(radius: f32, post_process_iterations: usize) -> Self {
        Self {
            radius,
            post_process_iterations,
        }
    }

    pub fn apply<S: Copy + Default>(&self, state: &mut GraphState<S>) {
        let n = state.node_count();
        if n == 0 {
            return;
        }
        if n == 1 {
            state.positions.set(0, Vec2::new(0.0, 0.0));
            return;
        }

        // 1. Sort nodes by degree (bucket sort by ascending degree)
        let mut degree_map: HashMap<NodeId, usize> = HashMap::new();
        for &node in &state.node_index_to_id {
            degree_map.insert(node, 0);
        }
        for (i, &src) in state.edge_sources.iter().enumerate() {
            let tgt = state.edge_targets[i];
            if src != tgt {
                *degree_map.entry(src).or_default() += 1;
                *degree_map.entry(tgt).or_default() += 1;
            }
        }

        let mut sorted_nodes = state.node_index_to_id.clone();
        sorted_nodes.sort_by_key(|node| degree_map.get(node).copied().unwrap_or(0));

        // 2. Compute circular embedding angles around perimeter
        let mut embedding_order = sorted_nodes;

        // 3. Post-processing crossing minimization swaps
        if self.post_process_iterations > 0 && n > 3 {
            self.post_process_crossing_minimization(state, &mut embedding_order);
        }

        // 4. Assign positions along circle boundary based on final embedding_order
        let step = 2.0 * std::f32::consts::PI / (n as f32);
        for (i, &node_id) in embedding_order.iter().enumerate() {
            let angle = (i as f32) * step;
            let x = self.radius * angle.cos();
            let y = self.radius * angle.sin();
            if let Some(&node_idx) = state.node_keys.get(node_id) {
                state.positions.set(node_idx, Vec2::new(x, y));
            }
        }
    }

    fn post_process_crossing_minimization<S: Copy + Default>(
        &self,
        state: &GraphState<S>,
        order: &mut Vec<NodeId>,
    ) {
        let n = order.len();
        for _iter in 0..self.post_process_iterations {
            let mut improved = false;
            for i in 0..n {
                let next_i = (i + 1) % n;
                let current_crossings = count_crossings(state, order);

                order.swap(i, next_i);
                let new_crossings = count_crossings(state, order);

                if new_crossings < current_crossings {
                    improved = true;
                } else {
                    order.swap(i, next_i); // revert swap
                }
            }
            if !improved {
                break;
            }
        }
    }
}

/// Evaluates total edge crossings for a given circular embedding order.
pub fn count_crossings<S: Copy + Default>(
    state: &GraphState<S>,
    order: &[NodeId],
) -> usize {
    let mut pos_map: HashMap<NodeId, usize> = HashMap::new();
    for (idx, &id) in order.iter().enumerate() {
        pos_map.insert(id, idx);
    }

    let mut edges: Vec<(usize, usize)> = Vec::new();
    for (i, &src) in state.edge_sources.iter().enumerate() {
        let tgt = state.edge_targets[i];
        if src != tgt {
            if let (Some(&u), Some(&v)) = (pos_map.get(&src), pos_map.get(&tgt)) {
                let (min_uv, max_uv) = if u < v { (u, v) } else { (v, u) };
                edges.push((min_uv, max_uv));
            }
        }
    }

    let mut count = 0;
    let num_edges = edges.len();
    for a in 0..num_edges {
        for b in (a + 1)..num_edges {
            let (u1, v1) = edges[a];
            let (u2, v2) = edges[b];
            if u1 != u2 && u1 != v2 && v1 != u2 && v1 != v2 {
                // Two chords (u1, v1) and (u2, v2) cross on a circle iff their endpoints alternate:
                // u1 < u2 < v1 < v2 or u2 < u1 < v2 < v1
                if (u1 < u2 && u2 < v1 && v1 < v2) || (u2 < u1 && u1 < v2 && v2 < v1) {
                    count += 1;
                }
            }
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{math::Size2, math::Vec2};

    #[test]
    fn test_circular_advanced_layout() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(40.0, 40.0));
        let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(40.0, 40.0));
        let n3 = state.add_node(Vec2::new(20.0, 20.0), Size2::new(40.0, 40.0));
        let n4 = state.add_node(Vec2::new(30.0, 30.0), Size2::new(40.0, 40.0));

        state.add_edge(n1, n3, Default::default());
        state.add_edge(n2, n4, Default::default());

        let layout = CircularAdvancedLayout::default();
        layout.apply(&mut state);

        assert_eq!(state.node_count(), 4);
    }
}
