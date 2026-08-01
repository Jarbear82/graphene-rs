use std::collections::HashMap;

/// The betweenness centrality algorithm implemented using Brandes' approach.
///
/// # Arguments
/// * `nodes` - All node IDs in the graph.
/// * `get_neighbors` - A closure that, given a node ID, returns an iterator over its neighbors.
///   For undirected graphs this should return all adjacent nodes; for directed graphs,
///   typically outgoing edges' targets.
/// * `edge_weight` - An optional closure that takes the source and target node IDs and
///   returns the edge weight as a `f64`. If `None`, the graph is treated as unweighted.
/// * `directed` - Whether the graph is directed. When `false`, neighbors are assumed to be
///   symmetric (the algorithm traverses each neighbor once, which is correct for undirected).
///
/// # Returns
/// A `HashMap` mapping each node ID to its betweenness centrality score.
pub fn betweenness_centrality<NodeId, I>(
    nodes: &[NodeId],
    get_neighbors: impl Fn(&NodeId) -> I,
    _edge_weight: Option<impl Fn(&NodeId, &NodeId) -> f64>,
    _directed: bool,
) -> HashMap<NodeId, f64>
where
    NodeId: Eq + Ord + std::hash::Hash + Clone + Copy,
    I: IntoIterator<Item = NodeId>,
{
    let mut state = graphene_core::GraphState::<()>::new();
    let mut node_to_id = HashMap::new();
    let mut id_to_node = HashMap::new();

    for n in nodes {
        let id = state.add_node(
            graphene_core::math::Vec2::default(),
            graphene_core::math::Size2::default(),
        );
        node_to_id.insert(*n, id);
        id_to_node.insert(id, *n);
    }

    for n in nodes {
        if let Some(&src_id) = node_to_id.get(n) {
            for neighbor in get_neighbors(n) {
                if let Some(&tgt_id) = node_to_id.get(&neighbor) {
                    state.add_edge(src_id, tgt_id, graphene_core::EdgeData::default());
                }
            }
        }
    }

    let scores = crate::search_traversal::graph_state_metrics::betweenness_centrality(&state);

    let mut betweenness = HashMap::new();
    for n in nodes {
        if let Some(&id) = node_to_id.get(n) {
            let score = *scores.get(&id).unwrap_or(&0.0) as f64;
            betweenness.insert(*n, score);
        }
    }

    betweenness
}

// ---------------------------------------------------------------------------
// Convenience wrapper: returns a normalized score map where each value is
// betweenness / max_betweenness (0.0 if all scores are 0).
// ---------------------------------------------------------------------------
pub fn betweenness_centrality_normalized<NodeId, I>(
    nodes: &[NodeId],
    get_neighbors: impl Fn(&NodeId) -> I,
    edge_weight: Option<impl Fn(&NodeId, &NodeId) -> f64>,
    directed: bool,
) -> (HashMap<NodeId, f64>, HashMap<NodeId, f64>)
where
    NodeId: Eq + Ord + std::hash::Hash + Clone + Copy,
    I: IntoIterator<Item = NodeId>,
{
    let raw = betweenness_centrality(nodes, get_neighbors, edge_weight, directed);

    let max = raw
        .values()
        .copied()
        .fold(0.0f64, |acc, val| acc.max(val));

    let normalized: HashMap<NodeId, f64> = raw
        .iter()
        .map(|(&n, &s)| (n, if max == 0.0 { 0.0 } else { s / max }))
        .collect();

    (raw, normalized)
}
