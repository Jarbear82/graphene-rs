use std::collections::HashMap;

/// Represents a directed edge in the graph.
#[derive(Debug, Clone)]
pub struct Edge {
    pub source: String,
    pub target: String,
}

/// Configuration options for the PageRank algorithm.
pub struct PageRankConfig {
    pub damping_factor: f64,
    pub precision: f64,
    pub max_iterations: usize,
    pub weight_fn: std::sync::Arc<dyn Fn(&Edge) -> f64 + Send + Sync>,
}

impl std::fmt::Debug for PageRankConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PageRankConfig")
            .field("damping_factor", &self.damping_factor)
            .field("precision", &self.precision)
            .field("max_iterations", &self.max_iterations)
            .finish()
    }
}

impl Clone for PageRankConfig {
    fn clone(&self) -> Self {
        Self {
            damping_factor: self.damping_factor,
            precision: self.precision,
            max_iterations: self.max_iterations,
            weight_fn: self.weight_fn.clone(),
        }
    }
}

impl Default for PageRankConfig {
    fn default() -> Self {
        PageRankConfig {
            damping_factor: 0.8,
            precision: 1e-6,
            max_iterations: 200,
            weight_fn: std::sync::Arc::new(|_edge| 1.0),
        }
    }
}

impl PageRankConfig {
    /// Override the default edge weight function.
    pub fn with_weight<F>(mut self, f: F) -> Self
    where
        F: Fn(&Edge) -> f64 + Send + Sync + 'static,
    {
        self.weight_fn = std::sync::Arc::new(f);
        self
    }

    /// Override the damping factor (default 0.8).
    pub fn with_damping_factor(mut self, factor: f64) -> Self {
        self.damping_factor = factor;
        self
    }

    /// Override the convergence precision (default 1e-6).
    pub fn with_precision(mut self, precision: f64) -> Self {
        self.precision = precision;
        self
    }

    /// Override the maximum iterations (default 200).
    pub fn with_max_iterations(mut self, n: usize) -> Self {
        self.max_iterations = n;
        self
    }

    fn weight(&self, edge: &Edge) -> f64 {
        (self.weight_fn)(edge)
    }
}

/// A directed graph ready for PageRank computation.
#[derive(Debug, Clone)]
pub struct Graph {
    nodes: Vec<String>,
    edges: Vec<Edge>,
    node_map: HashMap<String, usize>,
}

impl Graph {
    /// Create a new graph with an automatically-built index for O(1) lookups.
    pub fn new(nodes: Vec<String>, edges: Vec<Edge>) -> Self {
        let mut graph = Graph {
            nodes,
            edges,
            node_map: HashMap::new(),
        };
        graph.build_index();
        graph
    }

    /// Create a new graph without building the index (O(n) lookups).
    pub fn new_unindexed(nodes: Vec<String>, edges: Vec<Edge>) -> Self {
        Graph {
            nodes,
            edges,
            node_map: HashMap::new(),
        }
    }

    /// Compute PageRank scores for all nodes in the graph.
    pub fn page_rank(&self, config: &PageRankConfig) -> PageRankResult {
        let mut state = graphene_core::GraphState::<()>::new();
        let mut node_to_id = HashMap::new();
        for name in &self.nodes {
            let id = state.add_node(
                graphene_core::math::Vec2::default(),
                graphene_core::math::Size2::default(),
            );
            node_to_id.insert(name.clone(), id);
        }

        let mut edge_weight_map = HashMap::new();
        for edge in &self.edges {
            if let (Some(&src), Some(&tgt)) =
                (node_to_id.get(&edge.source), node_to_id.get(&edge.target))
            {
                let e_id = state.add_edge(src, tgt, graphene_core::EdgeData::default());
                edge_weight_map.insert(e_id, config.weight(edge) as f32);
            }
        }

        let scores = crate::search_traversal::graph_state_metrics::page_rank(
            &state,
            config.damping_factor as f32,
            config.precision as f32,
            config.max_iterations,
            |e| *edge_weight_map.get(&e).unwrap_or(&1.0),
        );

        let mut eigenvector = vec![0.0; self.nodes.len()];
        for (i, name) in self.nodes.iter().enumerate() {
            if let Some(&id) = node_to_id.get(name) {
                eigenvector[i] = *scores.get(&id).unwrap_or(&0.0) as f64;
            }
        }

        PageRankResult::new(self.nodes.clone(), eigenvector, true)
    }

    fn node_index(&self, id: &str) -> Option<usize> {
        self.node_map.get(id).copied()
    }

    fn build_index(&mut self) {
        for (i, node) in self.nodes.iter().enumerate() {
            self.node_map.insert(node.clone(), i);
        }
    }
}

/// Normalize a vector so that its elements sum to 1.0.
fn in_place_sum_normalize(vec: &mut Vec<f64>) {
    let sum: f64 = vec.iter().sum();
    if sum > 0.0 {
        for val in vec.iter_mut() {
            *val /= sum;
        }
    }
}

/// Result of a PageRank computation with a rank lookup function.
#[derive(Debug, Clone)]
pub struct PageRankResult {
    nodes: Vec<String>,
    scores: Vec<f64>,
    converged: bool,
}

impl PageRankResult {
    fn new(nodes: Vec<String>, scores: Vec<f64>, converged: bool) -> Self {
        PageRankResult {
            nodes,
            scores,
            converged,
        }
    }

    /// Get the PageRank score for a node by its string ID.
    pub fn rank(&self, node_id: &str) -> Option<f64> {
        self.nodes
            .iter()
            .position(|n| n == node_id)
            .map(|idx| self.scores[idx])
    }

    /// Get the PageRank score for a node by its index.
    pub fn rank_by_index(&self, idx: usize) -> Option<f64> {
        self.scores.get(idx).copied()
    }

    /// Iterate over all (node_id, score) pairs in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, f64)> + '_ {
        self.nodes
            .iter()
            .map(|n| n.as_str())
            .zip(self.scores.iter().copied())
    }

    /// Whether the algorithm converged within the iteration limit.
    pub fn converged(&self) -> bool {
        self.converged
    }

    /// Get all scores as a slice.
    pub fn scores(&self) -> &[f64] {
        &self.scores
    }

    /// Get the number of nodes ranked.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
