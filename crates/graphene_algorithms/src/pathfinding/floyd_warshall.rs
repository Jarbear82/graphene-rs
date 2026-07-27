#[derive(Debug, Clone)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
}

#[derive(Debug, Clone)]
pub enum PathStep {
    Node(usize),
    Edge(usize), // Index into the original edges slice provided to the algorithm
}

/// Result of the Floyd-Warshall computation. Stores matrices and edge references for path reconstruction.
#[derive(Debug)]
pub struct FloydWarshallResult {
    nodes_count: usize,
    dist: Vec<f64>,
    next: Vec<Option<usize>>,
    edge_next: Vec<Option<usize>>,
    _edges: Vec<Edge>, // Kept for reference during path reconstruction
}

impl FloydWarshallResult {
    /// Returns the shortest distance between two nodes.
    /// Returns `f64::INFINITY` if no path exists.
    pub fn distance(&self, from: usize, to: usize) -> f64 {
        self.dist[from * self.nodes_count + to]
    }

    /// Reconstructs the shortest path as a sequence of alternating nodes and edges.
    /// Returns `None` if no path exists or if `from == to` (handled separately in some use-cases).
    pub fn path(&self, from: usize, to: usize) -> Option<Vec<PathStep>> {
        if from == to {
            return Some(vec![PathStep::Node(from)]);
        }

        let ij = from * self.nodes_count + to;
        if self.next[ij].is_none() {
            return None; // Unreachable target
        }

        let mut current = from;
        let mut path = vec![PathStep::Node(from)];

        while current != to {
            let next_node = self.next[current * self.nodes_count + to].unwrap();
            let edge_idx = self.edge_next[current * self.nodes_count + next_node].unwrap();
            path.push(PathStep::Edge(edge_idx));
            path.push(PathStep::Node(next_node));
            current = next_node;
        }

        Some(path)
    }
}

/// Configuration with sensible defaults matching the original JS code.
#[derive(Debug)]
pub struct Config {
    pub weight: fn(&Edge) -> f64,
    pub directed: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            weight: |_edge| 1.0,
            directed: false,
        }
    }
}

/// Runs the Floyd-Warshall algorithm on a graph defined by nodes (count) and edges.
/// Nodes are expected to be indexed `0..nodes_count - 1`.
pub fn run_floyd_warshall(
    nodes_count: usize,
    edges: &[Edge],
    config: &Config,
) -> FloydWarshallResult {
    let n = nodes_count;
    let mut state = graphene_core::GraphState::<()>::new();
    let mut node_ids = Vec::with_capacity(n);

    for _ in 0..n {
        let id = state.add_node(
            graphene_core::math::Vec2::default(),
            graphene_core::math::Size2::default(),
        );
        node_ids.push(id);
    }

    let mut next = vec![None; n * n];
    let mut edge_next = vec![None; n * n];
    let mut edge_weight_map = std::collections::HashMap::new();

    for (edge_idx, edge) in edges.iter().enumerate() {
        let s = edge.source;
        let t = edge.target;
        if s == t || s >= n || t >= n {
            continue;
        }

        let st = s * n + t;
        let w = (config.weight)(edge);
        let e_id = state.add_edge(node_ids[s], node_ids[t], graphene_core::EdgeData::default());
        edge_weight_map.insert(e_id, w as f32);

        next[st] = Some(t);
        edge_next[st] = Some(edge_idx);

        if !config.directed {
            let ts = t * n + s;
            let e_rev = state.add_edge(node_ids[t], node_ids[s], graphene_core::EdgeData::default());
            edge_weight_map.insert(e_rev, w as f32);
            next[ts] = Some(s);
            edge_next[ts] = Some(edge_idx);
        }
    }

    let raw_dist = crate::pathfinding::graph_state_pathfinding::floyd_warshall(&state, |e| {
        *edge_weight_map.get(&e).unwrap_or(&1.0)
    });

    let mut dist = vec![f64::INFINITY; n * n];
    for i in 0..n {
        for j in 0..n {
            if i < raw_dist.len() && j < raw_dist[i].len() {
                dist[i * n + j] = raw_dist[i][j] as f64;
            }
        }
    }

    // Reconstruction matrices update
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                let ik = i * n + k;
                let kj = k * n + j;
                let ij = i * n + j;
                if dist[ik] != f64::INFINITY && dist[kj] != f64::INFINITY {
                    let alt = dist[ik] + dist[kj];
                    if alt < dist[ij] {
                        dist[ij] = alt;
                        next[ij] = next[ik];
                    }
                }
            }
        }
    }

    FloydWarshallResult {
        nodes_count,
        dist,
        next,
        edge_next,
        _edges: edges.to_vec(),
    }
}

// ======================================================
// Example Usage
// ======================================================
fn main() {
    // Domain types (simulating your graph's node/edge objects)
    #[derive(Debug)]
    struct Node(pub String);
    #[derive(Debug)]
    struct MyEdge {
        pub src: usize,
        pub tgt: usize,
        pub cost: f64,
    }

    let nodes = vec![
        Node("A".into()),
        Node("B".into()),
        Node("C".into()),
        Node("D".into()),
    ];
    let domain_edges = vec![
        MyEdge {
            src: 0,
            tgt: 1,
            cost: 4.0,
        },
        MyEdge {
            src: 0,
            tgt: 2,
            cost: 6.0,
        },
        MyEdge {
            src: 1,
            tgt: 3,
            cost: 3.0,
        },
        MyEdge {
            src: 2,
            tgt: 1,
            cost: 2.0,
        }, // Undirected logic will mirror this
    ];

    // Map domain edges to algorithm-friendly format
    let algo_edges: Vec<Edge> = domain_edges
        .iter()
        .map(|e| Edge {
            source: e.src,
            target: e.tgt,
        })
        .collect();

    // Run with defaults (weight = 1 per edge unless specified otherwise)
    let mut config = Config::default();
    config.weight = |edge: &Edge| {
        // Custom weight function if needed. Default is 1.0.
        1.0
    };
    config.directed = false;

    let result = run_floyd_warshall(nodes.len(), &algo_edges, &config);

    println!("Distance A -> D: {}", result.distance(0, 3));

    if let Some(path) = result.path(0, 3) {
        print!("Path A -> D: ");
        for step in path {
            match step {
                PathStep::Node(idx) => print!("{:?} ", nodes[idx].0),
                PathStep::Edge(idx) => print!("[{}] ", idx),
            }
        }
        println!();
    } else {
        println!("No path found");
    }
}
