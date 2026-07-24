use graphene_core::{GraphState, NodeId};
use std::collections::HashMap;

/// Builds an adjacency list mapping NodeId -> `Vec<NodeId>` for graph traversal and centrality algorithms.
pub fn build_adjacency_list<S: Copy>(
    state: &GraphState<S>,
    directed: bool,
) -> HashMap<NodeId, Vec<NodeId>> {
    let mut adj: HashMap<NodeId, Vec<NodeId>> = HashMap::new();

    for &id in &state.node_index_to_id {
        adj.entry(id).or_default();
    }

    for idx in 0..state.edges.len() {
        let src = state.edge_sources[idx];
        let tgt = state.edge_targets[idx];

        adj.entry(src).or_default().push(tgt);
        if !directed {
            adj.entry(tgt).or_default().push(src);
        }
    }

    adj
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{EdgeData, Size2, Vec2};

    #[test]
    fn test_build_adjacency_list_directed_and_undirected() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
        let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(10.0, 10.0));
        let n3 = state.add_node(Vec2::new(20.0, 20.0), Size2::new(10.0, 10.0));
        state.add_edge(n1, n2, EdgeData::default());

        let adj_directed = build_adjacency_list(&state, true);
        assert_eq!(adj_directed.get(&n1).unwrap(), &vec![n2]);
        assert!(adj_directed.get(&n2).unwrap().is_empty());
        assert!(adj_directed.get(&n3).unwrap().is_empty());

        let adj_undirected = build_adjacency_list(&state, false);
        assert_eq!(adj_undirected.get(&n1).unwrap(), &vec![n2]);
        assert_eq!(adj_undirected.get(&n2).unwrap(), &vec![n1]);
    }
}
