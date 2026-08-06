use graphene_core::{GraphState, NodeId};
use std::collections::{HashMap, HashSet};

/// Checks if a graph is planar using Euler's formula and Kuratowski subgraph constraints.
///
/// WHY / INVARIANT:
/// By Kuratowski's Theorem (1930) and Wagner's Theorem (1937), a finite graph is planar
/// if and only if it does not contain a subgraph that is a subdivision of $K_5$ (complete graph on 5 vertices)
/// or $K_{3,3}$ (complete bipartite graph on 3+3 vertices). Additionally, Euler's formula
/// establishes the necessary bound $E \le 3V - 6$ for any simple planar graph with $V \ge 3$.
pub fn is_planar<S: Copy + Default>(state: &GraphState<S>) -> bool {
    let v = state.node_count();
    let e = state.edge_count();

    if v <= 4 {
        return true;
    }

    // Euler's bound: for simple connected planar graphs with V >= 3, E <= 3V - 6
    if e > 3 * v - 6 {
        return false;
    }

    let mut adj: HashMap<NodeId, HashSet<NodeId>> = HashMap::new();
    for &node in &state.node_index_to_id {
        adj.insert(node, HashSet::new());
    }
    for (i, &src) in state.edge_sources.iter().enumerate() {
        let tgt = state.edge_targets[i];
        if src != tgt {
            adj.entry(src).or_default().insert(tgt);
            adj.entry(tgt).or_default().insert(src);
        }
    }

    !has_k5_or_k33_subgraph(&adj, &state.node_index_to_id)
}

fn has_k5_or_k33_subgraph(
    adj: &HashMap<NodeId, HashSet<NodeId>>,
    nodes: &[NodeId],
) -> bool {
    if nodes.len() < 5 {
        return false;
    }

    for &u in nodes {
        let Some(neighbors) = adj.get(&u) else { continue };
        if neighbors.len() < 4 {
            continue;
        }

        let n_vec: Vec<NodeId> = neighbors.iter().copied().collect();
        let is_clique = n_vec.iter().enumerate().all(|(i, &na)| {
            n_vec[i + 1..].iter().all(|&nb| {
                adj.get(&na).map_or(false, |s| s.contains(&nb))
            })
        });

        if is_clique && n_vec.len() >= 4 {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{math::Size2, math::Vec2};

    #[test]
    fn test_planarity_basic() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(1.0, 1.0));
        let n2 = state.add_node(Vec2::new(1.0, 0.0), Size2::new(1.0, 1.0));
        let n3 = state.add_node(Vec2::new(0.5, 1.0), Size2::new(1.0, 1.0));

        state.add_edge(n1, n2, Default::default());
        state.add_edge(n2, n3, Default::default());
        state.add_edge(n3, n1, Default::default());

        assert!(is_planar(&state));
    }
}
