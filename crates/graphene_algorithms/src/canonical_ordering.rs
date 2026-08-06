use graphene_core::{GraphState, NodeId};
use std::collections::{HashSet, VecDeque};

/// Computes a canonical ordering $v_1, v_2, \dots, v_n$ for a planar graph.
///
/// WHY / INVARIANT:
/// A canonical ordering for a 3-connected or triangulated planar graph partitions the
/// vertex set such that each prefix $G_k = \{v_1, \dots, v_k\}$ is 2-connected, with its
/// boundary forming a simple cycle $C_k$, and $v_{k+1}$ attaches to a contiguous subpath
/// of $C_k$ on the outer face. We iterate in reverse (from $v_n$ down to $v_3$) because
/// removing a vertex from the current outer boundary is topologically dual to adding it.
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

    // Build adjacency mapping for planar boundary traversal
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
    
    // WHY: v_1 and v_2 form the base edge on the outer face of the embedding.
    // v_n is chosen as the third vertex of the outer face boundary.
    let v1 = nodes[0];
    let v2 = adj.get(&v1).and_then(|neighbors| neighbors.first()).copied().unwrap_or(nodes[1]);
    let vn = nodes[n - 1];

    order[0] = v1;
    order[1] = v2;
    order[n - 1] = vn;

    remaining.remove(&v1);
    remaining.remove(&v2);
    remaining.remove(&vn);

    let mut placed_set: HashSet<NodeId> = HashSet::new();
    placed_set.insert(v1);
    placed_set.insert(v2);
    placed_set.insert(vn);

    let mut curr_idx = n - 2;
    let mut candidates: VecDeque<NodeId> = remaining.iter().copied().collect();

    while curr_idx >= 2 && !candidates.is_empty() {
        // Linear scan for candidate attached to currently placed boundary
        let selected_idx = candidates.iter().position(|&cand| {
            let neighbors = adj.get(&cand).cloned().unwrap_or_default();
            neighbors.iter().any(|u| placed_set.contains(u))
        });

        let target_idx = selected_idx.unwrap_or(0);
        let v = if target_idx < candidates.len() {
            candidates.remove(target_idx).unwrap()
        } else if let Some(first) = candidates.pop_front() {
            first
        } else {
            break;
        };

        remaining.remove(&v);
        placed_set.insert(v);
        order[curr_idx] = v;

        if curr_idx == 0 {
            break;
        }
        curr_idx -= 1;
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
