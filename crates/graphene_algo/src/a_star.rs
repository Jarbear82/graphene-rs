use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

/// Represents an element in the reconstructed path (alternating nodes and edges).
#[derive(Debug, Clone)]
pub enum PathElement<N, E> {
    Node(N),
    Edge(E),
}

#[derive(Debug)]
pub struct AStarResult<N, E> {
    pub found: bool,
    pub distance: Option<f64>,
    pub path: Vec<PathElement<N, E>>,
    pub steps: usize,
}

/// A* shortest path search.
///
/// `neighbors_fn` should return a list of `(neighbor_id, edge_id)` for a given node.
/// Directionality (`directed`) and source validation are expected to be handled by the caller
/// or pre-filtered in `neighbors_fn`, as they depend on your specific graph library's API.
pub fn astar<N, E, W, H, Nb>(
    start_id: N,
    goal_id: N,
    directed: bool,
    weight_fn: W,
    heuristic_fn: H,
    neighbors_fn: Nb,
) -> AStarResult<N, E>
where
    N: Clone + PartialEq + Eq + std::hash::Hash + Copy,
    E: Clone,
    W: Fn(E) -> f64,
    H: Fn(N) -> f64,
    Nb: Fn(N) -> Vec<(N, E)>,
{
    let mut g_score = HashMap::new();
    let mut f_score = HashMap::new();
    let mut closed_set = HashSet::new();
    let mut open_set = BinaryHeap::new();
    let mut open_set_ids = HashSet::new();
    let mut came_from: HashMap<N, N> = HashMap::new();
    let mut came_from_edge: HashMap<N, E> = HashMap::new();

    // Priority queue item. We use `Reverse` because Rust's BinaryHeap is a max-heap.
    #[derive(Debug, Clone, PartialEq)]
    struct PItem<N>(N, f64);

    impl<N: Eq> Eq for PItem<N> {}

    impl<N: Eq> PartialOrd for PItem<N> {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl<N: Eq> Ord for PItem<N> {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            // Compare f64 distances directly
            self.1
                .partial_cmp(&other.1)
                .unwrap_or(std::cmp::Ordering::Equal)
        }
    }

    g_score.insert(start_id.clone(), 0.0);
    f_score.insert(start_id.clone(), heuristic_fn(start_id));
    open_set.push(Reverse(PItem(start_id, f_score[&start_id])));
    open_set_ids.insert(start_id.clone());

    let mut steps = 0;

    while let Some(Reverse(current_item)) = open_set.pop() {
        let current_id = current_item.0;
        open_set_ids.remove(&current_id);
        steps += 1;

        // Skip stale entries (lazy deletion pattern)
        if let Some(&best_f) = f_score.get(&current_id) {
            if current_item.1 > best_f {
                continue;
            }
        }

        // Goal reached
        if current_id == goal_id {
            let mut path = Vec::new();
            let mut node = goal_id.clone();
            let mut edge_opt = came_from_edge.get(&node).cloned();

            // Reconstruct path backwards (equivalent to JS unshift loop)
            loop {
                path.push(PathElement::Node(node.clone()));
                if let Some(e) = edge_opt {
                    path.push(PathElement::Edge(e));
                }
                match came_from.get(&node) {
                    Some(prev_node) => {
                        node = prev_node.clone();
                        edge_opt = came_from_edge.get(&node).cloned();
                    }
                    None => break,
                }
            }
            path.reverse(); // Convert backwards reconstruction to forward order

            return AStarResult {
                found: true,
                distance: Some(g_score[&current_id]),
                path,
                steps,
            };
        }

        closed_set.insert(current_id);

        for (neighbor_id, edge_id) in neighbors_fn(current_id) {
            if closed_set.contains(&neighbor_id) {
                continue;
            }

            let temp_g = g_score[&current_id] + weight_fn(edge_id.clone());

            // Equivalent to JS: !isInOpenSet(wid) || tempScore < gScore[wid]
            let should_update = !open_set_ids.contains(&neighbor_id)
                || temp_g < *g_score.get(&neighbor_id).unwrap_or(&f64::INFINITY);

            if should_update {
                g_score.insert(neighbor_id.clone(), temp_g);
                let f = temp_g + heuristic_fn(neighbor_id);
                f_score.insert(neighbor_id.clone(), f);

                open_set.push(Reverse(PItem(neighbor_id, f)));
                open_set_ids.insert(neighbor_id.clone());

                came_from.insert(neighbor_id.clone(), current_id);
                came_from_edge.insert(neighbor_id.clone(), edge_id);
            }
        }
    }

    // Goal not reachable
    AStarResult {
        found: false,
        distance: None,
        path: Vec::new(),
        steps,
    }
}
