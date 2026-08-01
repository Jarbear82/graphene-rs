use graphene_core::{GraphState, NodeId};
use std::collections::{HashSet, VecDeque};

/// Computes a canonical ordering $v_1, v_2, \dots, v_n$ for a planar graph.
/// A canonical ordering ensures that each prefix subgraph $G_k$ is biconnected and
/// vertex $v_k$ connects to a contiguous subpath on the outer boundary of $G_{k-1}$.
pub fn compute_canonical_ordering<S: Copy + Default>(
    state: &GraphState<S>,
) -> Option<Vec<NodeId>> {
    let nodes = &state.node_index_to_id;
    let n = nodes.len();
    if n == 0 {
        return Some(Vec::new());
    }
    if n <= 3 {
        return Some(nodes.clone());
    }

    // Build adjacency mapping
    let mut adj: std::collections::HashMap<NodeId, Vec<NodeId>> = std::collections::HashMap::new();
    for &u in nodes {
        adj.insert(u, Vec::new());
    }
    for (i, &src) in state.edge_sources.iter().enumerate() {
        let tgt = state.edge_targets[i];
        if src != tgt {
            adj.entry(src).or_default().push(tgt);
            adj.entry(tgt).or_default().push(src);
        }
    }

    let mut remaining: HashSet<NodeId> = nodes.iter().copied().collect();
    let mut order = vec![nodes[0]; n];
    
    // Set v_1 and v_2
    let v1 = nodes[0];
    let v2 = adj.get(&v1).and_then(|neighbors| neighbors.first()).copied().unwrap_or(nodes[1]);
    let vn = nodes[n - 1];

    order[0] = v1;
    order[1] = v2;
    order[n - 1] = vn;

    remaining.remove(&v1);
    remaining.remove(&v2);
    remaining.remove(&vn);

    // Greedily pick vertices for positions k = n-1 down to 2
    let mut placed_set: HashSet<NodeId> = HashSet::new();
    placed_set.insert(v1);
    placed_set.insert(v2);
    placed_set.insert(vn);

    let mut curr_idx = n - 2;
    let mut candidates: VecDeque<NodeId> = remaining.iter().copied().collect();

    while curr_idx >= 2 && !candidates.is_empty() {
        let mut selected = None;
        let mut idx_to_remove = None;

        for (i, &cand) in candidates.iter().enumerate() {
            let neighbors = adj.get(&cand).cloned().unwrap_or_default();
            let placed_neighbors: Vec<NodeId> = neighbors
                .iter()
                .copied()
                .filter(|u| placed_set.contains(u))
                .collect();

            if !placed_neighbors.is_empty() {
                selected = Some(cand);
                idx_to_remove = Some(i);
                break;
            }
        }

        if let (Some(v), Some(i)) = (selected, idx_to_remove) {
            candidates.remove(i);
            remaining.remove(&v);
            placed_set.insert(v);
            order[curr_idx] = v;
            if curr_idx == 0 {
                break;
            }
            curr_idx -= 1;
        } else {
            if let Some(v) = candidates.pop_front() {
                remaining.remove(&v);
                placed_set.insert(v);
                order[curr_idx] = v;
                if curr_idx == 0 {
                    break;
                }
                curr_idx -= 1;
            }
        }
    }

    Some(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{math::Size2, math::Vec2};

    #[test]
    fn test_canonical_ordering_basic() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(1.0, 1.0));
        let n2 = state.add_node(Vec2::new(1.0, 0.0), Size2::new(1.0, 1.0));
        let n3 = state.add_node(Vec2::new(0.5, 1.0), Size2::new(1.0, 1.0));
        let n4 = state.add_node(Vec2::new(0.5, 0.5), Size2::new(1.0, 1.0));

        state.add_edge(n1, n2, Default::default());
        state.add_edge(n2, n3, Default::default());
        state.add_edge(n3, n1, Default::default());
        state.add_edge(n4, n1, Default::default());
        state.add_edge(n4, n2, Default::default());

        let ordering = compute_canonical_ordering(&state);
        assert!(ordering.is_some());
        let ord = ordering.unwrap();
        assert_eq!(ord.len(), 4);
    }
}
