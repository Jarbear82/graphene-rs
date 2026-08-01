// graphene_layout/src/multilevel.rs

use crate::traits::Layout;
use graphene_core::{math::Vec2, GraphState, NodeId};

struct CoarsenLevel {
    merges: Vec<(NodeId, NodeId, Vec2)>,
}

/// Multilevel force-directed layout wrapper using maximal-matching coarsening.
///
/// Reference: Walshaw, C. (2003). "A Multilevel Algorithm for Force-Directed Graph Drawing."
/// Journal of Graph Algorithms and Applications, 7(3), 253–285.
///
/// Builds a coarsening hierarchy via greedy maximal matching, runs the inner layout
/// on the coarsest graph, and interpolates positions back through each level with local relaxation.
pub struct MultilevelLayout<L> {
    pub sub_layout: L,
    pub min_graph_size: usize,
}

impl<L> MultilevelLayout<L> {
    pub fn new(sub_layout: L) -> Self {
        Self {
            sub_layout,
            min_graph_size: 10,
        }
    }

    pub fn with_min_graph_size(mut self, min_size: usize) -> Self {
        self.min_graph_size = min_size;
        self
    }
}

impl<S: Copy + Default, L: Layout<S>> Layout<S> for MultilevelLayout<L> {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let mut levels: Vec<CoarsenLevel> = Vec::new();
        let mut current: GraphState<S> = state.clone();

        loop {
            let n = current.node_index_to_id.len();
            if n <= self.min_graph_size {
                break;
            }

            let matching = maximal_matching(&current);
            if matching.is_empty() {
                break;
            }

            let mut merges = Vec::new();
            let mut matched_away = std::collections::HashSet::new();

            for &(survivor, merged) in &matching {
                if matched_away.contains(&survivor) || matched_away.contains(&merged) {
                    continue;
                }
                let Some(&s_idx) = current.node_keys.get(survivor) else {
                    continue;
                };
                let Some(&m_idx) = current.node_keys.get(merged) else {
                    continue;
                };
                let offset = *current.positions.get(m_idx) - *current.positions.get(s_idx);
                merges.push((survivor, merged, offset));
                matched_away.insert(merged);
            }

            for &(_, merged, _) in &merges {
                current.remove_node(merged);
            }

            if merges.is_empty() {
                break;
            }
            levels.push(CoarsenLevel { merges });
        }

        self.sub_layout.compute(&mut current);

        for &id in &current.node_index_to_id {
            if let (Some(&idx), Some(&orig_idx)) =
                (current.node_keys.get(id), state.node_keys.get(id))
            {
                state.positions.set(orig_idx, *current.positions.get(idx));
            }
        }

        for level in levels.iter().rev() {
            for &(survivor, merged, offset) in &level.merges {
                if let (Some(&s_idx), Some(&m_idx)) =
                    (state.node_keys.get(survivor), state.node_keys.get(merged))
                {
                    let survivor_pos = *state.positions.get(s_idx);
                    state.positions.set(m_idx, survivor_pos + offset);
                }
            }
        }

        self.sub_layout.compute(state);
        let collapsed = std::collections::HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}

fn maximal_matching<S: Copy>(state: &GraphState<S>) -> Vec<(NodeId, NodeId)> {
    let mut matched = std::collections::HashSet::new();
    let mut pairs = Vec::new();

    for &id in &state.node_index_to_id {
        if matched.contains(&id) {
            continue;
        }

        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            let neighbor = if src == id {
                Some(tgt)
            } else if tgt == id {
                Some(src)
            } else {
                None
            };
            if let Some(n_id) = neighbor {
                if n_id != id && !matched.contains(&n_id) {
                    matched.insert(id);
                    matched.insert(n_id);
                    pairs.push((id, n_id));
                    break;
                }
            }
        }
    }
    pairs
}
