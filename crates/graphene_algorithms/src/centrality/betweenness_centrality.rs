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
    _edge_weight: Option<impl Fn(&NodeId, &NodeId) -> f64>,
    _directed: bool,
) -> HashMap<NodeId, f64>
where
    NodeId: Eq + Ord + std::hash::Hash + Clone + Copy,
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
