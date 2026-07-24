use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

/// A min-heap priority queue using `BinaryHeap` with a comparator function.
pub struct MinHeap<K, F>
where
    K: Eq + std::hash::Hash,
{
    heap: Vec<(K,)>,
    pos_map: HashMap<K, usize>, // current position of key in the binary heap
    cmp: F,
}

// We need a custom wrapper to store comparator along with elements.
// This is a simpler approach: we'll use BinaryHeap with a custom wrapper type.



// For simplicity, let's use a binary heap with update capability instead.
// Actually, the simplest correct approach is to use BinaryHeap + re-insert on distance decrease.
// Or just use `vec`-based min-heap for clarity and correctness (Dijkstra doesn't need updates if we allow duplicates).

pub struct MinPQ<K: Ord> {
    heap: Vec<K>,
}

impl<K: Ord> MinPQ<K> {
    pub fn new() -> Self {
        MinPQ { heap: Vec::new() }
    }

    pub fn push(&mut self, key: K) {
        self.heap.push(key);
        let mut i = self.heap.len() - 1;
        while i > 0 {
            let parent = (i - 1) / 2;
            if self.heap[parent] <= self.heap[i] {
                break;
            }
            self.heap.swap(parent, i);
            i = parent;
        }
    }

    pub fn pop(&mut self) -> Option<K> {
        if self.heap.is_empty() {
            return None;
        }
        let len = self.heap.len();
        self.heap.swap(0, len - 1);
        let min = self.heap.pop().unwrap();
        if !self.heap.is_empty() {
            let mut i = 0;
            loop {
                let left = 2 * i + 1;
                let right = 2 * i + 2;
                let mut smallest = i;
                if left < self.heap.len() && self.heap[left] < self.heap[smallest] {
                    smallest = left;
                }
                if right < self.heap.len() && self.heap[right] < self.heap[smallest] {
                    smallest = right;
                }
                if smallest == i {
                    break;
                }
                self.heap.swap(i, smallest);
                i = smallest;
            }
        }
        Some(min)
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }
}

/// Trait for getting neighbors of a node.
pub trait GetNeighbors<I> {
    /// Return the neighbor nodes of `node`.
    fn get_neighbors(node: &I) -> Box<dyn Iterator<Item = I> + '_>;
}

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
pub fn betweenness_centrality<NodeId>(
    nodes: &[NodeId],
    get_neighbors: impl Fn(&NodeId) -> Box<dyn Iterator<Item = NodeId>>,
    edge_weight: Option<impl Fn(&NodeId, &NodeId) -> f64>,
    directed: bool,
) -> HashMap<NodeId, f64>
where
    NodeId: Eq + Ord + std::hash::Hash + Clone + Copy,
{
    let mut betweenness: HashMap<NodeId, f64> = HashMap::new();

    // Initialize scores to 0.
    for n in nodes {
        betweenness.insert(n.clone(), 0.0);
    }

    for s in nodes {
        // Dijkstra's / Brandes' state
        let mut dist: HashMap<NodeId, f64> = HashMap::new();
        let mut sigma: HashMap<NodeId, f64> = HashMap::new(); // path count
        let mut pred: HashMap<NodeId, Vec<NodeId>> = HashMap::new();
        let mut delta: HashMap<NodeId, f64> = HashMap::new(); // accumulation

        for n in nodes {
            let nn = n.clone();
            dist.insert(nn, f64::INFINITY);
            sigma.insert(nn, 0.0);
            pred.insert(nn, Vec::new());
            delta.insert(nn, 0.0);
        }

        dist.insert(s.clone(), 0.0);
        sigma.insert(s.clone(), 1.0);

        let mut pq = MinPQ::<NodeId>::new();
        let mut visited: std::collections::HashSet<NodeId> = std::collections::HashSet::new();

        pq.push(s.clone());
        // We track "in PQ" via the visited set used to avoid re-pushing.
        // Actually, we need a separate "in_pq" tracking or just allow duplicates in the heap
        // and skip stale entries. Let's use a simpler approach: push always, check on pop.

        let mut stack: Vec<NodeId> = Vec::new();

        while let Some(v) = pq.pop() {
            if visited.contains(&v) {
                continue;
            }
            visited.insert(v.clone());
            stack.push(v.clone());

            let neighbors: Vec<_> = get_neighbors(&v).collect();

            for w in &neighbors {
                let edge_w = if let Some(ref ew) = edge_weight {
                    ew(&v, w)
                } else {
                    1.0
                };

                let new_dist = dist[&v] + edge_w;

                if dist[w] > new_dist {
                    // Found a shorter path to w — update and record predecessor
                    dist.insert(w.clone(), new_dist);
                    sigma.insert(w.clone(), sigma[&v]);
                    pred.get_mut(w).unwrap().clear();
                    pred.get_mut(w).unwrap().push(v.clone());

                    pq.push(w.clone()); // may create duplicates, handled by `visited` above
                } else if dist[w] == new_dist {
                    // Found an alternative shortest path to w
                    let prev_sigma = *sigma.get(w).unwrap_or(&0.0);
                    sigma.insert(w.clone(), prev_sigma + sigma[&v]);
                    pred.get_mut(w).unwrap().push(v.clone());

                    // Note: w may already be in PQ; for Dijkstra correctness with non-negative
                    // weights, this is fine (we skip stale entries).
                }
            }
        }

        // Back-propagation phase
        while let Some(w) = stack.pop() {
            if w != *s {
                // accumulate delta values onto predecessors
                if let Some(preds) = pred.get(&w) {
                    for v in preds {
                        let contrib = sigma[v] / sigma[&w] * (1.0 + delta[&w]);
                        let prev_delta = delta.get_mut(v).unwrap();
                        *prev_delta += contrib;
                    }
                }
                // Add delta to betweenness score of w
                let bc = betweenness.get_mut(&w).unwrap();
                *bc += delta[&w];
            }
        }
    }

    // For undirected graphs, each pair is counted twice. Divide by 2.
    if !directed {
        for bc in betweenness.values_mut() {
            *bc /= 2.0;
        }
    }

    betweenness
}

// ---------------------------------------------------------------------------
// Convenience wrapper: returns a normalized score map where each value is
// betweenness / max_betweenness (0.0 if all scores are 0).
// ---------------------------------------------------------------------------
pub fn betweenness_centrality_normalized<NodeId>(
    nodes: &[NodeId],
    get_neighbors: impl Fn(&NodeId) -> Box<dyn Iterator<Item = NodeId>>,
    edge_weight: Option<impl Fn(&NodeId, &NodeId) -> f64>,
    directed: bool,
) -> (HashMap<NodeId, f64>, HashMap<NodeId, f64>)
where
    NodeId: Eq + Ord + std::hash::Hash + Clone + Copy,
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
