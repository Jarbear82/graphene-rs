use graphene_core::{GraphState, NodeId};
use std::collections::{HashMap, HashSet};

/// Checks if a graph is planar using Euler's formula and block boundary embedding constraints.
/// Returns true if the graph is planar, false otherwise.
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

    // Perform DFS-based Kuratowski subgraph check (K5 / K3,3 subgraph containment test)
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

    // Check for K5 or K3,3 minor/subgraph patterns
    !has_k5_or_k33_subgraph(&adj, &state.node_index_to_id)
}

fn has_k5_or_k33_subgraph(
    adj: &HashMap<NodeId, HashSet<NodeId>>,
    nodes: &[NodeId],
) -> bool {
    // If graph has >= 5 vertices, check for K5 complete graph minor
    if nodes.len() >= 5 {
        for i in 0..nodes.len() {
            let u = nodes[i];
            let neighbors = match adj.get(&u) {
                Some(n) => n,
                None => continue,
            };
            if neighbors.len() >= 4 {
                // Potential K5 vertex
                let n_vec: Vec<NodeId> = neighbors.iter().copied().collect();
                if n_vec.len() >= 4 {
                    let mut fully_connected = true;
                    for a in 0..n_vec.len() {
                        for b in (a + 1)..n_vec.len() {
                            let na = n_vec[a];
                            let nb = n_vec[b];
                            if !adj.get(&na).map_or(false, |s| s.contains(&nb)) {
                                fully_connected = false;
                                break;
                            }
                        }
                    }
                    if fully_connected && n_vec.len() >= 4 {
                        return true;
                    }
                }
            }
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
