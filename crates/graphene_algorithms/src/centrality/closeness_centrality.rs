use std::collections::HashMap;

/// Enum dispatch provider for edge weight metrics in closeness centrality.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeWeightMetric {
    Uniform(f64),
    Scaled(f64),
}

impl EdgeWeightMetric {
    #[inline(always)]
    pub fn evaluate(&self, _edge: &Edge) -> f64 {
        match self {
            EdgeWeightMetric::Uniform(w) => *w,
            EdgeWeightMetric::Scaled(s) => *s,
        }
    }
}

impl Default for EdgeWeightMetric {
    fn default() -> Self {
        EdgeWeightMetric::Uniform(1.0)
    }
}

/// Configuration options for closeness centrality calculations.
#[derive(Clone, Debug, Default)]
pub struct ClosenessCentralityOptions {
    /// Use harmonic mean (1/d) instead of reciprocal of total distance.
    pub harmonic: bool,
    /// Weight metric applied to each edge when traversing.
    pub weight_metric: EdgeWeightMetric,
    /// Whether the graph is directed.
    pub directed: bool,
    /// Root node for single-source calculations (if any).
    pub root: Option<NodeId>,
}

/// Result of `closeness_centralities` for all nodes.
#[derive(Debug, Clone)]
pub struct ClosenessCentralityResult {
    /// Raw closeness values keyed by node ID.
    pub closenesses: HashMap<NodeId, f64>,
    /// Maximum closeness value used for normalization (0 if none).
    pub max_closeness: f64,
}

impl ClosenessCentralityResult {
    /// Normalized closeness for a given node.
    pub fn closeness(&self, node_id: NodeId) -> f64 {
        if self.max_closeness == 0.0 {
            return 0.0;
        }
        *self.closenesses.get(&node_id).unwrap_or(&0.0) / self.max_closeness
    }

    /// Returns the raw (unnormalized) closeness value for a node.
    pub fn raw_closeness(&self, node_id: NodeId) -> f64 {
        *self.closenesses.get(&node_id).unwrap_or(&0.0)
    }
}

/// Compute normalized closeness centrality for every node in the graph using Floyd–Warshall.
pub fn closeness_centralities(
    graph: &impl Graph,
    options: ClosenessCentralityOptions,
) -> ClosenessCentralityResult {
    let mut closenesses = HashMap::new();
    let mut max_closeness: f64 = 0.0;

    let fw = floyd_warshall(graph, &options);

    let node_ids: Vec<NodeId> = graph.node_ids();

    for i in 0..node_ids.len() {
        let curr_node = &node_ids[i];
        let mut curr_closeness = 0.0;

        for j in 0..node_ids.len() {
            if i != j {
                let target = &node_ids[j];
                // Skip unreachable nodes — distance would be infinite.
                if let Some(&d) = fw.get(&(*curr_node, *target)) {
                    if options.harmonic {
                        curr_closeness += 1.0 / d;
                    } else {
                        curr_closeness += d;
                    }
                }
            }
        }

        // Harmonic closeness already uses reciprocals; for non-harmonic take reciprocal of total.
        if !options.harmonic {
            if curr_closeness == 0.0 {
                curr_closeness = 0.0;
            } else {
                curr_closeness = 1.0 / curr_closeness;
            }
        }

        max_closeness = max_closeness.max(curr_closeness);
        closenesses.insert(*curr_node, curr_closeness);
    }

    ClosenessCentralityResult {
        closenesses,
        max_closeness,
    }
}

/// Compute closeness centrality for a single root node using Dijkstra.
/// Returns harmonic sum (1/d) or reciprocal of total distance depending on `harmonic`.
pub fn closeness_centralty_one_node(
    graph: &impl Graph,
    options: ClosenessCentralityOptions,
) -> f64 {
    let Some(root) = options.root else {
        return 0.0;
    };

    let dists = dijkstra_shortest_paths(graph, root, &options);
    let mut total = 0.0;

    for n in graph.node_ids() {
        if n != root {
            // Skip unreachable nodes to avoid infinities.
            if let Some(d) = dists.get(&n) {
                if options.harmonic {
                    total += 1.0 / d;
                } else {
                    total += d;
                }
            }
        }
    }

    if options.harmonic {
        total
    } else if total == 0.0 {
        0.0
    } else {
        1.0 / total
    }
}

// ---------------------------------------------------------------------------
// Minimal graph trait to keep this standalone (replace with your Graph type)
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct NodeId(pub u64);

#[derive(Debug, Clone)]
pub struct Edge {
    pub source: NodeId,
    pub target: NodeId,
}

/// Minimal interface expected by these functions.
pub trait Graph {
    fn node_ids(&self) -> Vec<NodeId>;
    fn adjacency_list(&self) -> &HashMap<NodeId, Vec<(NodeId, f64)>>;
    fn is_directed(&self) -> bool;
}

// ---------------------------------------------------------------------------
// Minimal Floyd–Warshall implementation
// ---------------------------------------------------------------------------

/// All-pairs shortest distances. `None` means unreachable.
pub type DistanceMatrix = HashMap<(NodeId, NodeId), f64>;

pub fn floyd_warshall(graph: &impl Graph, options: &ClosenessCentralityOptions) -> DistanceMatrix {
    let mut dists = HashMap::new();

    // Initialize with direct edges.
    for (&n, adj) in graph.adjacency_list().iter() {
        for &(m, w) in adj.iter() {
            let key = (n, m);
            let weight = w;
            dists.entry(key).or_insert(weight);
        }
    }

    // Also add 0-diagonal entries.
    for n in graph.node_ids() {
        dists.entry((n, n)).or_insert(0.0);
    }

    let nodes: Vec<NodeId> = graph.node_ids();

    for &k in &nodes {
        for &i in &nodes {
            if let Some(&dik) = dists.get(&(i, k)) {
                for &j in &nodes {
                    if i == j || j == k {
                        continue;
                    }
                    if let Some(&dkj) = dists.get(&(k, j)) {
                        let new_dist = dik + dkj;
                        dists
                            .entry((i, j))
                            .and_modify(|d| *d = d.min(new_dist))
                            .or_insert(new_dist);
                    }
                }
            }
        }
    }

    dists
}



// ---------------------------------------------------------------------------
// Minimal Dijkstra implementation
// ---------------------------------------------------------------------------

pub type ShortestPaths = HashMap<NodeId, f64>;

pub fn dijkstra_shortest_paths(
    graph: &impl Graph,
    root: NodeId,
    _options: &ClosenessCentralityOptions,
) -> ShortestPaths {
    use std::cmp::Ordering;
    use std::collections::BinaryHeap;

    #[derive(PartialEq)]
    struct Entry(NodeId, f64);

    impl Eq for Entry {}

    impl Ord for Entry {
        fn cmp(&self, other: &Self) -> Ordering {
            other.1.partial_cmp(&self.1).unwrap_or(Ordering::Equal)
        }
    }

    impl PartialOrd for Entry {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    let mut dists = HashMap::new();
    dists.insert(root, 0.0);

    let mut heap = BinaryHeap::new();
    heap.push(Entry(root, 0.0));

    while let Some(Entry(u, _)) = heap.pop() {
        if let Some(&d_u) = dists.get(&u) {
            for &(v, w) in graph
                .adjacency_list()
                .get(&u)
                .map(|adj| adj.as_slice())
                .unwrap_or(&[])
            {
                let new_dist = d_u + w;
                if let Some(&d_v) = dists.get(&v) {
                    if new_dist >= d_v {
                        continue;
                    }
                }
                dists.insert(v, new_dist);
                heap.push(Entry(v, new_dist));
            }
        }
    }

    dists
}
