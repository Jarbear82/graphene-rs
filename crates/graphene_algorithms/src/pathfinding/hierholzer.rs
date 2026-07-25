use std::collections::{HashMap, HashSet, VecDeque};

/// Result of Hierholzer's algorithm.
#[derive(Debug, Clone, PartialEq)]
pub struct EulerResult {
    /// Whether an Eulerian trail/circuit was found.
    pub found: bool,
    /// Ordered node IDs forming the trail or circuit.
    pub trail: Vec<String>,
}

/// Configuration for Hierholzer's algorithm.
#[derive(Debug, Clone)]
pub struct HierholzerConfig {
    /// Starting vertex ID (optional; auto-inferred if omitted).
    pub root: Option<String>,
    /// Whether the graph is directed.
    pub directed: bool,
}

impl Default for HierholzerConfig {
    fn default() -> Self {
        Self {
            root: None,
            directed: false,
        }
    }
}

/// A single edge with known source and target.
#[derive(Debug, Clone)]
pub struct EdgeInfo {
    pub source: String,
    pub target: String,
}

// ── Public API ─────────────────────────────────────────────────────

/// Run Hierholzer's algorithm to find an Eulerian trail or circuit.
///
/// # Arguments
/// * `nodes` — map from node ID → list of edge IDs incident to that node
/// * `edges` — map from edge ID → [`EdgeInfo`]
/// * `config` — configuration options (defaults: no root, undirected)
pub fn hierholzer(
    nodes: &HashMap<String, Vec<String>>,
    edges: &HashMap<String, EdgeInfo>,
    config: &HierholzerConfig,
) -> EulerResult {
    if nodes.is_empty() {
        return no_result();
    }

    // ── Determine start vertex ────────────────────────────────────────
    let start_vertex = match &config.root {
        Some(root) => {
            if nodes.contains_key(root) {
                root.clone()
            } else {
                return no_result();
            }
        }
        None => String::new(),
    };

    // ── Degree validation ─────────────────────────────────────────────
    let mut odd_in = None; // excess incoming / odd-degree node (start)
    let mut odd_out = None; // excess outgoing / second odd-degree node (end)
    let mut impossible = false;

    if config.directed {
        let mut in_deg: HashMap<&str, i32> = HashMap::new();
        let mut out_deg: HashMap<&str, i32> = HashMap::new();
        for node_id in nodes.keys() {
            in_deg.insert(node_id.as_str(), 0);
            out_deg.insert(node_id.as_str(), 0);
        }
        for e in edges.values() {
            *out_deg.entry(&e.source).or_default() += 1;
            *in_deg.entry(&e.target).or_default() += 1;
        }
        for node_id in nodes.keys() {
            let in_d = in_deg.get(node_id.as_str()).copied().unwrap_or(0);
            let out_d = out_deg.get(node_id.as_str()).copied().unwrap_or(0);
            let diff = in_d - out_d;

            if diff == 1 {
                if odd_in.is_some() {
                    impossible = true;
                } else {
                    odd_in = Some(node_id.clone());
                }
            } else if diff == -1 {
                if odd_out.is_some() {
                    impossible = true;
                } else {
                    odd_out = Some(node_id.clone());
                }
            } else if diff.abs() > 1 {
                impossible = true;
            }
        }

        // Valid directed Eulerian configurations:
        //   Circuit:  no odd-degree nodes (all in == out)
        //   Path:     exactly one node with (in-out)=1 (start of path)
        //               and exactly one with (out-in)=1 (end of path)
        if !impossible {
            let has_odd = |opt: &Option<String>| opt.is_some();
            let count = [has_odd(&odd_in), has_odd(&odd_out)]
                .iter()
                .filter(|&&b| b)
                .count();
            if count > 2 || (has_odd(&odd_in) != has_odd(&odd_out)) {
                impossible = true;
            }
        }
    } else {
        for (node_id, edge_ids) in nodes {
            let degree = edge_ids.len() as i32;
            if degree % 2 != 0 {
                if odd_in.is_none() {
                    odd_in = Some(node_id.clone());
                } else if odd_out.is_none() {
                    odd_out = Some(node_id.clone());
                } else {
                    impossible = true;
                }
            }
        }

        // For undirected Eulerian path, if two odd-degree nodes exist there must be exactly two.
        // If more than two have odd degree → impossible (already set above).
        // If exactly two → valid path starting at one of them.
    }

    if impossible {
        return no_result();
    }

    // ── Determine actual start vertex ─────────────────────────────────
    let mut start = start_vertex;

    match (&odd_in, &odd_out) {
        (Some(oi), Some(oo)) => {
            // Eulerian path exists (not circuit). Must start at node with excess outgoing edges (directed) or any odd node (undirected).
            if config.directed {
                if !start.is_empty() && &start != oo {
                    return no_result();
                }
                start = oo.clone();
            } else {
                if !start.is_empty() && &start != oi && &start != oo {
                    return no_result();
                }
                if start.is_empty() {
                    start = oi.clone();
                }
            }
        }
        _ => {
            // Eulerian circuit: any node can be the start.
            if start.is_empty() {
                if let Some(key) = nodes.keys().next() {
                    start = key.clone();
                } else {
                    return no_result();
                }
            }
        }
    }

    // ── Hierholzer's algorithm (iterative, stack-based) ───────────────
    // Build mutable adjacency: node → deque of edge IDs.
    let mut adj: HashMap<String, VecDeque<String>> = nodes
        .iter()
        .map(|(k, v)| (k.clone(), VecDeque::from(v.clone())))
        .collect();

    let mut used_edges: HashSet<String> = HashSet::new();

    // `tour_stack` holds the current walk path (nodes).
    let mut tour_stack: Vec<String> = vec![start];
    // `euler_path` will hold the final Eulerian trail in order.
    let mut euler_path: Vec<String> = Vec::new();

    while let Some(v) = tour_stack.last().cloned() {
        let found_edge = if let Some(edge_ids) = adj.get(&v) {
            // Find first unused edge incident to v.
            edge_ids.iter().find(|eid| !used_edges.contains(*eid))
        } else {
            None
        };

        match found_edge {
            Some(eid) => {
                let eid_str = eid.clone();

                // Remove from source node's adjacency (by index).
                if let Some(edge_ids) = adj.get_mut(&v) {
                    if let Some(idx) = edge_ids.iter().position(|e| e == &eid_str) {
                        edge_ids.remove(idx);
                    }
                }

                let e = &edges[&eid_str];
                let next_node: String;

                if config.directed {
                    // Directed: must traverse from source → target.
                    next_node = e.target.clone();
                    // Remove from the other endpoint's adjacency list too.
                    remove_edge_from_adj(&mut adj, &e.source, &eid_str);
                } else {
                    // Undirected: pick direction based on which endpoint is `v`.
                    if e.source == v {
                        next_node = e.target.clone();
                    } else {
                        next_node = e.source.clone();
                    }
                    // Remove from BOTH endpoints.
                    remove_edge_from_adj(&mut adj, &e.source, &eid_str);
                    remove_edge_from_adj(&mut adj, &e.target, &eid_str);
                }

                used_edges.insert(eid_str);
                tour_stack.push(next_node);
            }
            None => {
                // Backtrack: no more edges from this vertex.
                euler_path.push(tour_stack.pop().unwrap());
            }
        }
    }

    // Reverse because nodes are pushed in reverse order (backtracking).
    euler_path.reverse();

    // Verify all edges were consumed.
    let remaining_edges: usize = adj.values().map(|v| v.len()).sum();
    if remaining_edges > 0 {
        return no_result();
    }

    EulerResult {
        found: true,
        trail: euler_path,
    }
}

/// Remove a single edge ID from a node's adjacency list.
fn remove_edge_from_adj(adj: &mut HashMap<String, VecDeque<String>>, node_id: &str, eid: &str) {
    if let Some(edge_ids) = adj.get_mut(node_id) {
        edge_ids.retain(|e| e != eid);
    }
}

// ── Helpers ────────────────────────────────────────────────────────

fn no_result() -> EulerResult {
    EulerResult {
        found: false,
        trail: Vec::new(),
    }
}

// ── Example usage ──────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undirected_circuit() {
        // Triangle graph: A-B, B-C, C-A (every node has degree 2 → Eulerian circuit exists)
        let mut nodes = HashMap::new();
        nodes.insert("A".to_string(), vec!["e1".to_string(), "e3".to_string()]);
        nodes.insert("B".to_string(), vec!["e1".to_string(), "e2".to_string()]);
        nodes.insert("C".to_string(), vec!["e2".to_string(), "e3".to_string()]);

        let mut edges = HashMap::new();
        edges.insert(
            "e1".to_string(),
            EdgeInfo {
                source: "A".to_string(),
                target: "B".to_string(),
            },
        );
        edges.insert(
            "e2".to_string(),
            EdgeInfo {
                source: "B".to_string(),
                target: "C".to_string(),
            },
        );
        edges.insert(
            "e3".to_string(),
            EdgeInfo {
                source: "C".to_string(),
                target: "A".to_string(),
            },
        );

        let config = HierholzerConfig::default(); // undirected, no root
        let result = hierholzer(&nodes, &edges, &config);

        assert!(result.found);
        assert_eq!(result.trail.len(), 4); // circuit: A→B→C→A (4 node visits for 3 edges)
    }

    #[test]
    fn test_undirected_path() {
        // Path graph: A-B-C where A and C have odd degree (1 each) → Eulerian path exists
        let mut nodes = HashMap::new();
        nodes.insert("A".to_string(), vec!["e1".to_string()]);
        nodes.insert("B".to_string(), vec!["e1".to_string(), "e2".to_string()]);
        nodes.insert("C".to_string(), vec!["e2".to_string()]);

        let mut edges = HashMap::new();
        edges.insert(
            "e1".to_string(),
            EdgeInfo {
                source: "A".to_string(),
                target: "B".to_string(),
            },
        );
        edges.insert(
            "e2".to_string(),
            EdgeInfo {
                source: "B".to_string(),
                target: "C".to_string(),
            },
        );

        let config = HierholzerConfig::default();
        let result = hierholzer(&nodes, &edges, &config);

        assert!(result.found);
        // Trail should be A → B → C (or C → B → A)
        assert_eq!(result.trail.len(), 3);
    }

    #[test]
    fn test_no_eulerian_path() {
        // All four nodes have odd degree → impossible
        let mut nodes = HashMap::new();
        nodes.insert("A".to_string(), vec!["e1".to_string(), "e2".to_string(), "e3".to_string()]);
        nodes.insert("B".to_string(), vec!["e1".to_string()]);
        nodes.insert("C".to_string(), vec!["e2".to_string()]);
        nodes.insert("D".to_string(), vec!["e3".to_string()]);

        let mut edges = HashMap::new();
        edges.insert(
            "e1".to_string(),
            EdgeInfo {
                source: "A".to_string(),
                target: "B".to_string(),
            },
        );
        edges.insert(
            "e2".to_string(),
            EdgeInfo {
                source: "A".to_string(),
                target: "C".to_string(),
            },
        );
        edges.insert(
            "e3".to_string(),
            EdgeInfo {
                source: "A".to_string(),
                target: "D".to_string(),
            },
        );

        let config = HierholzerConfig {
            directed: false,
            ..Default::default()
        };
        let result = hierholzer(&nodes, &edges, &config);

        assert!(!result.found);
    }

    #[test]
    fn test_directed_circuit() {
        // Directed cycle: A→B→C→A
        let mut nodes = HashMap::new();
        nodes.insert("A".to_string(), vec!["e1".to_string()]);
        nodes.insert("B".to_string(), vec!["e2".to_string()]);
        nodes.insert("C".to_string(), vec!["e3".to_string()]);

        let mut edges = HashMap::new();
        edges.insert(
            "e1".to_string(),
            EdgeInfo {
                source: "A".to_string(),
                target: "B".to_string(),
            },
        );
        edges.insert(
            "e2".to_string(),
            EdgeInfo {
                source: "B".to_string(),
                target: "C".to_string(),
            },
        );
        edges.insert(
            "e3".to_string(),
            EdgeInfo {
                source: "C".to_string(),
                target: "A".to_string(),
            },
        );

        let config = HierholzerConfig {
            directed: true,
            ..Default::default()
        };
        let result = hierholzer(&nodes, &edges, &config);

        assert!(result.found);
        assert_eq!(result.trail.len(), 4);
    }
}
