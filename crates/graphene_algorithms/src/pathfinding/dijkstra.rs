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
    pub fn dijkstra<F>(&self, mut config: DijkstraConfig<F>) -> DijkstraResult
    where
        F: Fn(&Edge) -> f64,
    {
        let mut dist = HashMap::new();
        let mut known_dist = HashMap::new();
        let mut prev = HashMap::new();

        // JS: `edges.unmergeBy( ele => ele.isLoop() )`
        let valid_edges: Vec<&Edge> = self.edges.iter().filter(|e| e.from != e.to).collect();

        let source_id = config.root.0;

        #[derive(Copy, Clone, PartialEq)]
        struct MinFloatNode(f64, usize);
        impl Eq for MinFloatNode {}
        impl Ord for MinFloatNode {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                other.0.partial_cmp(&self.0).unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| self.1.cmp(&other.1))
            }
        }
        impl PartialOrd for MinFloatNode {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                Some(self.cmp(other))
            }
        }

        let mut q = BinaryHeap::new();

        // Initialize all nodes
        for node in &self.nodes {
            dist.insert(
                node.0,
                if node.0 == source_id {
                    0.0
                } else {
                    f64::INFINITY
                },
            );
            q.push(MinFloatNode(dist[&node.0], node.0));
        }

        // Main loop
        while let Some(MinFloatNode(smallest_dist, u_id)) = q.pop() {
            // Lazy-deletion check (standard Rust replacement for `Q.updateItem`)
            if known_dist.get(&u_id).copied().unwrap_or(f64::MAX) < smallest_dist {
                continue;
            }

            known_dist.insert(u_id, smallest_dist);

            if smallest_dist == f64::INFINITY {
                break; // Remaining nodes are unreachable from source
            }

            // Relax neighbors
            for edge in &valid_edges {
                let v_id = if config.directed {
                    if edge.from.0 == u_id {
                        edge.to.0
                    } else {
                        continue;
                    }
                } else {
                    if edge.from.0 == u_id {
                        edge.to.0
                    } else if edge.to.0 == u_id {
                        edge.from.0
                    } else {
                        continue;
                    }
                };

                let current_v_dist = *dist.get(&v_id).unwrap_or(&f64::INFINITY);

                let weight = (config.weight_fn)(edge);
                let alt = smallest_dist + weight;

                if alt < current_v_dist {
                    dist.insert(v_id, alt);
                    q.push(MinFloatNode(alt, v_id));
                    prev.insert(v_id, (u_id, (*edge).clone()));
                }
            }
        }

        DijkstraResult {
            distances: known_dist,
            predecessors: prev,
        }
    }
}
