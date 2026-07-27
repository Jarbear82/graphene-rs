use std::collections::{BinaryHeap, HashMap};

// ---------------------------------------------------------------------------
// Minimal Graph Types
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
}

#[derive(Debug, Clone)]
pub struct Graph {
    pub nodes: Vec<NodeId>,
    pub edges: Vec<Edge>,
}

// ---------------------------------------------------------------------------
// Configuration & Defaults
// ---------------------------------------------------------------------------
pub struct DijkstraConfig<F> {
    pub root: NodeId,
    pub weight_fn: F,
    pub directed: bool,
}

impl<F> DijkstraConfig<F> {
    /// Creates a config with defaults (undirected, default weight of 1.0 handled by caller)
    pub fn new(root: NodeId, weight_fn: F) -> Self {
        Self {
            root,
            weight_fn,
            directed: false,
        }
    }

}



impl DijkstraConfig<fn(&Edge) -> f64> {
    /// Helper to create a config with the JS default weight (`edge => 1`)
    pub fn with_defaults(root: NodeId) -> Self {
        Self::new(root, default_weight)
    }
}

/// Matches the JS default: `edge => 1`
pub fn default_weight(_: &Edge) -> f64 {
    1.0
}

// ---------------------------------------------------------------------------
// Algorithm Result
// ---------------------------------------------------------------------------
pub struct DijkstraResult {
    distances: HashMap<usize, f64>,
    predecessors: HashMap<usize, (usize, Edge)>,
}

impl DijkstraResult {
    /// Returns the shortest distance to `target`. Returns `INFINITY` if unreachable.
    pub fn distance_to(&self, target: usize) -> f64 {
        *self.distances.get(&target).unwrap_or(&f64::INFINITY)
    }

    /// Reconstructs the path from the root node to `target`.
    /// Returns nodes in order `[root, ..., target]`.
    pub fn path_to(&self, target: usize) -> Vec<NodeId> {
        let mut path = vec![NodeId(target)];
        let mut current = target;

        while let Some((prev_id, _edge)) = self.predecessors.get(&current) {
            path.push(NodeId(*prev_id));
            current = *prev_id;
        }

        path.reverse();
        path
    }
}

// ---------------------------------------------------------------------------
// Core Algorithm
// ---------------------------------------------------------------------------
impl Graph {
    pub fn dijkstra<F>(&self, config: DijkstraConfig<F>) -> DijkstraResult
    where
        F: Fn(&Edge) -> f64,
    {
        let mut state = graphene_core::GraphState::<()>::new();
        let mut num_to_id = HashMap::new();
        let mut id_to_num = HashMap::new();

        for node in &self.nodes {
            let id = state.add_node(
                graphene_core::math::Vec2::default(),
                graphene_core::math::Size2::default(),
            );
            num_to_id.insert(node.0, id);
            id_to_num.insert(id, node.0);
        }

        let mut edge_map = HashMap::new();
        for edge in &self.edges {
            if edge.from != edge.to {
                if let (Some(&src), Some(&tgt)) = (num_to_id.get(&edge.from.0), num_to_id.get(&edge.to.0)) {
                    let e_id = state.add_edge(src, tgt, graphene_core::EdgeData::default());
                    edge_map.insert(e_id, edge.clone());
                    if !config.directed {
                        let e_rev = state.add_edge(tgt, src, graphene_core::EdgeData::default());
                        edge_map.insert(e_rev, edge.clone());
                    }
                }
            }
        }

        let root_id = num_to_id.get(&config.root.0).copied().unwrap_or_default();
        let dist_map = crate::search_traversal::graph_state_search::dijkstra(
            &state,
            root_id,
            |e| {
                if let Some(edge) = edge_map.get(&e) {
                    (config.weight_fn)(edge) as f32
                } else {
                    1.0
                }
            },
        );

        let mut distances = HashMap::new();
        for node in &self.nodes {
            if let Some(&id) = num_to_id.get(&node.0) {
                if let Some(&d) = dist_map.get(&id) {
                    distances.insert(node.0, d as f64);
                } else {
                    distances.insert(node.0, f64::INFINITY);
                }
            }
        }

        DijkstraResult {
            distances,
            predecessors: HashMap::new(),
        }
    }
}
