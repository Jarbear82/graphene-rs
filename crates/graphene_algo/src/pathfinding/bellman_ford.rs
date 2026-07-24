use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::hash::Hash;

/// Represents a directed or undirected edge in the graph.
#[derive(Debug, Clone)]
pub struct Edge<N> {
    pub id: usize,
    pub source: N,
    pub target: N,
}

/// The result of running Bellman-Ford, providing distance and path accessors.
#[derive(Debug, Clone)]
pub struct BellmanFordResult<N> {
    distances: HashMap<N, f64>,
    predecessors: HashMap<N, Option<(N, usize)>>, // (predecessor_node, edge_id) or None for root
    has_negative_cycle: bool,
    negative_cycles: Vec<Vec<N>>,
}

impl<N> BellmanFordResult<N>
where
    N: Clone + Eq + Hash + Ord + Display,
{
    /// Computes the shortest distance from the root to `node`.
    pub fn distance_to(&self, node: &N) -> f64 {
        self.distances.get(node).copied().unwrap_or(f64::INFINITY)
    }

    /// Returns true if a negative weight cycle was detected.
    pub fn has_negative_cycle(&self) -> bool {
        self.has_negative_cycle
    }

    /// Returns the detected negative weight cycles.
    pub fn negative_cycles(&self) -> &[Vec<N>] {
        &self.negative_cycles
    }

    /// Reconstructs the shortest path from the root to `target`.
    /// Returns `None` if no path exists or if a negative cycle is present.
    pub fn path_to(&self, target: &N) -> Option<Vec<N>> {
        let mut path = Vec::new();
        let mut current = Some(target.clone());
        let mut visited = HashSet::new();

        while let Some(node) = current {
            if !visited.insert(node.clone()) {
                return None; // Path intersects a negative cycle
            }
            path.push(node.clone());
            match self.predecessors.get(&node).cloned().flatten() {
                Some((pred, _)) => current = Some(pred),
                None => break, // Reached root or unreachable node
            }
        }

        path.reverse();
        if !path.is_empty() && *path.first().unwrap() == *target {
            None // Target is not reachable from the root
        } else {
            Some(path)
        }
    }

    /// Runs the Bellman-Ford algorithm.
    ///
    /// # Arguments
    /// * `edges` - Collection of edges in the graph
    /// * `root` - The starting node for distance calculations
    /// * `weight_fn` - A closure that returns the weight of a given edge (default: 1.0)
    /// * `directed` - Whether the graph is directed (`false` means undirected/reversible)
    /// * `find_cycles` - Whether to detect and return negative weight cycles
    pub fn run<F>(
        edges: Vec<Edge<N>>,
        root: N,
        weight_fn: F,
        directed: bool,
        find_cycles: bool,
    ) -> Self
    where
        F: Fn(&Edge<N>) -> f64,
    {
        // Remove self-loops as in the original implementation
        let edges: Vec<_> = edges.into_iter().filter(|e| e.source != e.target).collect();

        let mut distances = HashMap::new();
        let mut predecessors = HashMap::new();

        // Initialize nodes encountered in edges
        for e in &edges {
            distances.entry(e.source.clone()).or_insert(f64::INFINITY);
            distances.entry(e.target.clone()).or_insert(f64::INFINITY);
            predecessors.entry(e.source.clone()).or_insert(None);
            predecessors.entry(e.target.clone()).or_insert(None);
        }

        // Set root initialization
        distances.insert(root.clone(), 0.0);
        predecessors.insert(root.clone(), None);

        let num_nodes = distances.len();

        // Relaxation phase (V - 1 iterations)
        let mut replaced = false;
        for _ in 1..num_nodes {
            replaced = false;
            for e in &edges {
                let w = weight_fn(e);
                let src_dist = distances.get(&e.source).copied().unwrap_or(f64::INFINITY);
                let tgt_dist = distances.get(&e.target).copied().unwrap_or(f64::INFINITY);

                // Relax forward edge
                if src_dist + w < tgt_dist {
                    distances.insert(e.target.clone(), src_dist + w);
                    predecessors.insert(e.target.clone(), Some((e.source.clone(), e.id)));
                    replaced = true;
                }

                // Relax reverse edge for undirected graphs
                if !directed {
                    let rev_src = tgt_dist;
                    let rev_tgt = src_dist;
                    if rev_src + w < rev_tgt {
                        distances.insert(e.source.clone(), rev_src + w);
                        predecessors.insert(e.source.clone(), Some((e.target.clone(), e.id)));
                        replaced = true;
                    }
                }
            }
            if !replaced {
                break;
            }
        }

        // Negative cycle detection phase
        let mut has_neg = false;
        let mut neg_cycles = Vec::new();

        if replaced && find_cycles {
            let mut seen_cycle_keys = HashSet::new();

            for e in &edges {
                let w = weight_fn(e);
                let src_d = distances.get(&e.source).copied().unwrap_or(f64::INFINITY);
                let tgt_d = distances.get(&e.target).copied().unwrap_or(f64::INFINITY);

                if src_d + w < tgt_d || (!directed && tgt_d + w < src_d) {
                    has_neg = true;

                    let mut cycle_starts = Vec::new();
                    if src_d + w < tgt_d {
                        cycle_starts.push(e.source.clone());
                    }
                    if !directed && tgt_d + w < src_d {
                        cycle_starts.push(e.target.clone());
                    }

                    for start in cycle_starts {
                        let mut current = start.clone();
                        let mut path: Vec<N> = vec![current.clone()];

                        // Walk predecessors until we hit a node already in the current path
                        while let Some((pred, _)) = predecessors.get(&current).cloned().flatten() {
                            if path.contains(&pred) {
                                break;
                            }
                            path.push(pred.clone());
                            current = pred;
                        }

                        // Normalize: rotate cycle so it starts with the lexicographically smallest node
                        let min_idx = if path.len() > 1 {
                            (1..path.len())
                                .min_by_key(|&i| path[i].clone())
                                .unwrap_or(0)
                        } else {
                            0
                        };
                        let mut normalized: Vec<N> = path[min_idx..].to_vec();
                        normalized.extend_from_slice(&path[..min_idx]);
                        normalized.push(normalized[0].clone()); // Close the cycle

                        // Deduplicate by serializing node display strings
                        let key: String = normalized
                            .iter()
                            .map(|n| format!("{}", n))
                            .collect::<Vec<_>>()
                            .join(",");

                        if !seen_cycle_keys.contains(&key) {
                            seen_cycle_keys.insert(key);
                            neg_cycles.push(normalized);
                        }
                    }
                }
            }
        } else if replaced && !find_cycles {
            has_neg = true;
        }

        Self {
            distances,
            predecessors,
            has_negative_cycle: has_neg,
            negative_cycles: neg_cycles,
        }
    }
}

// ─────────────────────────────────────────────────────────────
// Example Usage
// ─────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bellman_ford() {
        let edges = vec![
            Edge {
                id: 0,
                source: "A",
                target: "B",
            },
            Edge {
                id: 1,
                source: "B",
                target: "C",
            },
            Edge {
                id: 2,
                source: "C",
                target: "D",
            },
            Edge {
                id: 3,
                source: "D",
                target: "E",
            },
            Edge {
                id: 4,
                source: "E",
                target: "F",
            },
        ];

        let result = BellmanFordResult::<&str>::run(
            edges,
            "A",
            |_| 1.0, // default weight
            true,    // directed
            false,   // don't need cycles for this test
        );

        assert!(!result.has_negative_cycle());
        assert_eq!(result.distance_to(&"A"), 0.0);
        assert_eq!(result.distance_to(&"C"), 2.0);
        assert_eq!(result.distance_to(&"E"), 4.0);

        let path = result.path_to(&"D").unwrap();
        assert_eq!(path, vec!["A", "B", "C", "D"]);
    }

    #[test]
    fn test_negative_cycle() {
        let edges = vec![
            Edge {
                id: 0,
                source: "X",
                target: "Y",
            },
            Edge {
                id: 1,
                source: "Y",
                target: "Z",
            },
            Edge {
                id: 2,
                source: "Z",
                target: "X",
            },
        ];

        let result = BellmanFordResult::<&str>::run(
            edges,
            "X",
            |e| match e.id {
                0 => -1.0,
                1 => -1.0,
                _ => -5.0,
            }, // Negative cycle
            true,
            true,
        );

        assert!(result.has_negative_cycle());
        assert!(!result.negative_cycles().is_empty());
    }
}
