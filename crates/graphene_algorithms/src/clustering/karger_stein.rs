// src/lib.rs
//! Karger-Stein minimum cut algorithm implementation.
//!
//! Finds an approximate minimum cut of an undirected graph by randomized
//! edge contraction, repeating the process to improve probability of success.

use rand::{seq::SliceRandom, Rng};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Public API types
// ---------------------------------------------------------------------------

/// A weighted edge in the graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
    pub weight: f64,
}

/// The result of computing a minimum cut.
#[derive(Debug, Clone)]
pub struct MinCutResult {
    /// Edges whose removal produces the minimum cut.
    pub cut_edges: Vec<Edge>,
    /// Weight of the cut (sum of weights of removed edges).
    pub cut_weight: f64,
    /// Nodes in partition 1.
    pub partition_1: Vec<usize>,
    /// Nodes in partition 2.
    pub partition_2: Vec<usize>,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Contract two meta-nodes (identified by their current partition ids) into one.
///
/// * `edge` — the edge that causes the contraction.
/// * `meta_map` — maps original node indices to their current partition id.
/// * `edges` — mutable list of edges; will be mutated in-place.
fn collapse(
    edge_index: usize,
    meta_map: &mut Vec<usize>,
    edges: &mut Vec<(usize, usize, f64)>, // (src_idx, tgt_idx, weight)
) {
    let (src_node, tgt_node, _) = edges[edge_index];

    let partition1 = meta_map[src_node];
    let partition2 = meta_map[tgt_node];

    // Remove all edges between partition1 and partition2.
    edges.retain(|&edge| {
        let s = edge.0;
        let t = edge.1;
        let p_s = meta_map[s];
        let p_t = meta_map[t];

        // Keep the edge iff it does NOT connect the two partitions being merged.
        !((p_s == partition1 && p_t == partition2) || (p_s == partition2 && p_t == partition1))
    });



    // Move every node from partition2 to partition1 in the map.
    for id in meta_map.iter_mut() {
        if *id == partition2 {
            *id = partition1;
        }
    }
}

/// Contract edges randomly until at most `size_limit` meta-nodes remain.
fn contract_until(
    meta_map: &mut Vec<usize>,
    mut edges: Vec<(usize, usize, f64)>,
    size: usize,
    size_limit: usize,
) -> (Vec<(usize, usize, f64)>, usize) {
    let mut current_size = size;

    while current_size > size_limit && !edges.is_empty() {
        let edge_index = rand::thread_rng().gen_range(0..edges.len());

        collapse(edge_index, meta_map, &mut edges);

        // After a contraction the number of distinct partition ids decreases.
        // We track it by the count of unique values in meta_map (or equivalently
        // by decrementing our own counter — safer since meta_map may still hold
        // stale values).
        current_size -= 1;
    }

    (edges, current_size)
}

// ---------------------------------------------------------------------------
// Main algorithm
// ---------------------------------------------------------------------------

/// Compute an approximate minimum cut of a graph using the Karger-Stein algorithm.
///
/// WHY / INVARIANT:
/// Karger's algorithm contracts random edges because any edge in a specific minimum cut $C$
/// is contracted with probability at most $2/|V|$. By contracting until $n/\sqrt{2}$ meta-nodes
/// remain and recursively splitting into two independent trials, Karger-Stein elevates the
/// success probability from $O(1/n^2)$ to $O(1/\log n)$ per trial, running in $O(n^2 \log^3 n)$ time.
///
/// # Arguments
/// * `nodes` — list of node identifiers.
/// * `edges` — list of undirected edges connecting the nodes.
pub fn karger_stein(nodes: &[usize], edges: &[Edge]) -> Option<MinCutResult> {
    let num_nodes = nodes.len();

    if num_nodes < 2 {
        return None;
    }

    // Filter out self-loops and build the compacted edge list (src, tgt, weight).
    let mut edge_list: Vec<(usize, usize, f64)> = edges
        .iter()
        .filter_map(|e| {
            let s = e.source;
            let t = e.target;
            if s == t {
                None // skip self-loops
            } else {
                Some((s, t, e.weight))
            }
        })
        .collect();

    // Number of outer iterations ≈ ln²(n)  (constant-factor choice).
    let num_iter = (num_nodes as f64).ln().powi(2) as usize + 1;

    // Stop contracting when only n / √2 meta-nodes remain.
    let stop_size = (num_nodes as f64 / 2_f64.sqrt()) as usize;

    // Best cut found so far.
    let mut best_cut_weight = f64::INFINITY;
    let mut best_cut_edges: Vec<Edge> = Vec::new();
    let mut best_partition_map: Vec<usize> = vec![0; num_nodes];

    // Helper to evaluate cut weight and collect cut edges from partition map
    let cut_from_map = |map: &[usize]| -> (f64, Vec<Edge>) {
        let mut cut_weight = 0.0;
        let mut cut_edges = Vec::new();
        for e in edges {
            if e.source < num_nodes && e.target < num_nodes && map[e.source] != map[e.target] {
                cut_weight += e.weight;
                cut_edges.push(e.clone());
            }
        }
        (cut_weight, cut_edges)
    };

    for _ in 0..=num_iter {
        // Reset every node to its own partition.
        let mut meta_map: Vec<usize> = (0..num_nodes).collect();

        // Phase 1 — contract down to `stop_size`.
        let (edges_after_p1, _) = contract_until(
            &mut meta_map,
            edge_list.clone(),
            num_nodes,
            stop_size,
        );
        let mut edges_state = edges_after_p1.clone();

        // Copy of partition map for the second recursive call.
        let mut meta_map2 = meta_map.clone();
        let mut edges_state2 = edges_state.clone();

        // Phase 2 — two independent recursions from the stop point.
        let (_, _) = contract_until(
            &mut meta_map,
            std::mem::take(&mut edges_state),
            stop_size,
            2,
        );
        let (_, _) = contract_until(
            &mut meta_map2,
            std::mem::take(&mut edges_state2),
            stop_size,
            2,
        );

        let (w1, edges1) = cut_from_map(&meta_map);
        if w1 < best_cut_weight {
            best_cut_weight = w1;
            best_cut_edges = edges1;
            best_partition_map = meta_map.clone();
        }

        let (w2, edges2) = cut_from_map(&meta_map2);
        if w2 < best_cut_weight {
            best_cut_weight = w2;
            best_cut_edges = edges2;
            best_partition_map = meta_map2;
        }
    }

    // At this point best_cut_weight holds the minimum found.  Construct the
    // two partitions by scanning the best partition map again.
    let mut partition_1 = Vec::new();
    let mut partition_2 = Vec::new();
    if !best_partition_map.is_empty() {
        let witness = best_partition_map[0];
        for (i, &pid) in best_partition_map.iter().enumerate() {
            if pid == witness {
                partition_1.push(nodes[i]);
            } else {
                partition_2.push(nodes[i]);
            }
        }
    }

    Some(MinCutResult {
        cut_edges: best_cut_edges,
        cut_weight: best_cut_weight,
        partition_1,
        partition_2,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_karger_stein_on_simple_graph() {
        // A graph shaped like a "dumbbell": two cliques of 4 nodes each
        // connected by a single bridge edge. The min cut should be 1.
        let nodes = vec![0, 1, 2, 3, 4, 5, 6, 7];

        // First clique: 0-1-2-3 fully connected (complete graph K4)
        let mut edges = vec![
            Edge {
                source: 0,
                target: 1,
                weight: 1.0,
            },
            Edge {
                source: 0,
                target: 2,
                weight: 1.0,
            },
            Edge {
                source: 0,
                target: 3,
                weight: 1.0,
            },
            Edge {
                source: 1,
                target: 2,
                weight: 1.0,
            },
            Edge {
                source: 1,
                target: 3,
                weight: 1.0,
            },
            Edge {
                source: 2,
                target: 3,
                weight: 1.0,
            },
        ];

        // Bridge edge
        edges.push(Edge {
            source: 3,
            target: 4,
            weight: 1.0,
        });

        // Second clique: 4-5-6-7 fully connected
        for i in 4..=7 {
            for j in (i + 1)..=7 {
                edges.push(Edge {
                    source: i,
                    target: j,
                    weight: 1.0,
                });
            }
        }

        let result = karger_stein(&nodes, &edges).expect("should produce a result");
        // The minimum cut weight should be 1 (the bridge edge).
        assert_eq!(result.cut_weight, 1.0);
        assert!(!result.partition_1.is_empty());
        assert!(!result.partition_2.is_empty());
    }
}
