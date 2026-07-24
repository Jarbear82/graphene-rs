use graphene_core::{EdgeId, GraphState, NodeId};
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone)]
pub struct EdgeTopology {
    pub out_offsets: Vec<usize>,
    pub out_edge_indices: Vec<EdgeId>,
}

impl EdgeTopology {
    pub fn rebuild<S: Copy>(state: &GraphState<S>) -> Self {
        let num_nodes = state.node_index_to_id.len();
        let mut out_counts = vec![0; num_nodes];

        for &src in state.edge_sources.iter() {
            if let Some(&src_idx) = state.node_keys.get(src) {
                out_counts[src_idx] += 1;
            }
        }

        let mut out_offsets = vec![0; num_nodes + 1];
        let mut accum = 0;
        for i in 0..num_nodes {
            out_offsets[i] = accum;
            accum += out_counts[i];
        }
        out_offsets[num_nodes] = accum;

        let mut out_edge_indices = vec![EdgeId::default(); accum];
        let mut current_offsets = out_offsets.clone();

        for (edge_idx, &src) in state.edge_sources.iter().enumerate() {
            let edge_id = state.edge_index_to_id[edge_idx];
            if let Some(&src_idx) = state.node_keys.get(src) {
                let dest_offset = current_offsets[src_idx];
                out_edge_indices[dest_offset] = edge_id;
                current_offsets[src_idx] += 1;
            }
        }

        Self {
            out_offsets,
            out_edge_indices,
        }
    }

    pub fn outgoing_edges(&self, node_idx: usize) -> &[EdgeId] {
        let start = self.out_offsets[node_idx];
        let end = self.out_offsets[node_idx + 1];
        &self.out_edge_indices[start..end]
    }
}

pub fn bfs<S: Copy>(state: &GraphState<S>, start_node: NodeId, mut visitor: impl FnMut(NodeId)) {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    if state.node_keys.contains_key(start_node) {
        queue.push_back(start_node);
        visited.insert(start_node);
    }

    let topo = EdgeTopology::rebuild(state);

    while let Some(current) = queue.pop_front() {
        visitor(current);

        if let Some(&curr_idx) = state.node_keys.get(current) {
            for &edge_id in topo.outgoing_edges(curr_idx) {
                if let Some(&edge_idx) = state.edge_keys.get(edge_id) {
                    let target = state.edge_targets[edge_idx];
                    if visited.insert(target) {
                        queue.push_back(target);
                    }
                }
            }
        }
    }
}

pub fn dfs<S: Copy>(state: &GraphState<S>, start_node: NodeId, mut visitor: impl FnMut(NodeId)) {
    let mut visited = HashSet::new();
    let mut stack = Vec::new();

    if state.node_keys.contains_key(start_node) {
        stack.push(start_node);
    }

    let topo = EdgeTopology::rebuild(state);

    while let Some(current) = stack.pop() {
        if visited.insert(current) {
            visitor(current);

            if let Some(&curr_idx) = state.node_keys.get(current) {
                for &edge_id in topo.outgoing_edges(curr_idx) {
                    if let Some(&edge_idx) = state.edge_keys.get(edge_id) {
                        let target = state.edge_targets[edge_idx];
                        if !visited.contains(&target) {
                            stack.push(target);
                        }
                    }
                }
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
struct DijkstraState {
    cost: f32,
    position: NodeId,
}

impl Eq for DijkstraState {}

impl Ord for DijkstraState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for DijkstraState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn dijkstra<S: Copy>(
    state: &GraphState<S>,
    start_node: NodeId,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> HashMap<NodeId, f32> {
    let mut distances = HashMap::new();
    let mut heap = BinaryHeap::new();

    if !state.node_keys.contains_key(start_node) {
        return distances;
    }

    distances.insert(start_node, 0.0);
    heap.push(DijkstraState {
        cost: 0.0,
        position: start_node,
    });

    let topo = EdgeTopology::rebuild(state);

    while let Some(DijkstraState { cost, position }) = heap.pop() {
        if let Some(&curr_dist) = distances.get(&position) {
            if cost > curr_dist {
                continue;
            }
        }

        if let Some(&curr_idx) = state.node_keys.get(position) {
            for &edge_id in topo.outgoing_edges(curr_idx) {
                if let Some(&edge_idx) = state.edge_keys.get(edge_id) {
                    let target = state.edge_targets[edge_idx];
                    let weight = edge_weight(edge_id);
                    let next_cost = cost + weight;

                    let prev_cost = distances.get(&target).copied().unwrap_or(f32::INFINITY);
                    if next_cost < prev_cost {
                        distances.insert(target, next_cost);
                        heap.push(DijkstraState {
                            cost: next_cost,
                            position: target,
                        });
                    }
                }
            }
        }
    }

    distances
}

pub struct AdjacencyList {
    pub adj: HashMap<NodeId, Vec<(NodeId, EdgeId)>>,
}

impl AdjacencyList {
    pub fn build<S: Copy>(state: &GraphState<S>, directed: bool) -> Self {
        let mut adj = HashMap::new();
        for &id in &state.node_index_to_id {
            adj.insert(id, Vec::new());
        }
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            let edge_id = state.edge_index_to_id[idx];
            adj.entry(src).or_default().push((tgt, edge_id));
            if !directed {
                adj.entry(tgt).or_default().push((src, edge_id));
            }
        }
        Self { adj }
    }

    pub fn neighbors(&self, node: NodeId) -> &[(NodeId, EdgeId)] {
        self.adj.get(&node).map(|v| v.as_slice()).unwrap_or(&[])
    }
}

#[derive(Debug, Clone)]
pub struct AStarResult {
    pub found: bool,
    pub distance: f32,
    pub path: Vec<NodeId>,
    pub edges: Vec<EdgeId>,
    pub steps: usize,
}

#[derive(Copy, Clone, PartialEq)]
struct AStarState {
    f_score: f32,
    g_score: f32,
    position: NodeId,
}

impl Eq for AStarState {}

impl Ord for AStarState {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other
            .f_score
            .partial_cmp(&self.f_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl PartialOrd for AStarState {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn a_star<S: Copy>(
    state: &GraphState<S>,
    start: NodeId,
    goal: NodeId,
    edge_weight: impl Fn(EdgeId) -> f32,
    heuristic: impl Fn(NodeId) -> f32,
    directed: bool,
) -> AStarResult {
    let mut open_set = BinaryHeap::new();
    let mut g_score = HashMap::new();
    let mut f_score = HashMap::new();
    let mut came_from = HashMap::new();
    let mut came_from_edge = HashMap::new();
    let mut closed_set = HashSet::new();
    let mut steps = 0;

    if !state.node_keys.contains_key(start) || !state.node_keys.contains_key(goal) {
        return AStarResult {
            found: false,
            distance: 0.0,
            path: Vec::new(),
            edges: Vec::new(),
            steps: 0,
        };
    }

    g_score.insert(start, 0.0);
    let start_f = heuristic(start);
    f_score.insert(start, start_f);
    open_set.push(AStarState {
        f_score: start_f,
        g_score: 0.0,
        position: start,
    });

    let adj = AdjacencyList::build(state, directed);

    while let Some(AStarState {
        g_score: curr_g,
        position: current,
        ..
    }) = open_set.pop()
    {
        steps += 1;

        if current == goal {
            let mut path = Vec::new();
            let mut path_edges = Vec::new();
            let mut curr_node = goal;
            path.push(curr_node);
            while let Some(&prev_node) = came_from.get(&curr_node) {
                if let Some(&edge) = came_from_edge.get(&curr_node) {
                    path_edges.push(edge);
                }
                path.push(prev_node);
                curr_node = prev_node;
            }
            path.reverse();
            path_edges.reverse();

            return AStarResult {
                found: true,
                distance: curr_g,
                path,
                edges: path_edges,
                steps,
            };
        }

        if !closed_set.insert(current) {
            continue;
        }

        for &(neighbor, edge_id) in adj.neighbors(current) {
            if closed_set.contains(&neighbor) {
                continue;
            }

            let tentative_g = curr_g + edge_weight(edge_id);
            let prev_g = g_score.get(&neighbor).copied().unwrap_or(f32::INFINITY);

            if tentative_g < prev_g {
                came_from.insert(neighbor, current);
                came_from_edge.insert(neighbor, edge_id);
                g_score.insert(neighbor, tentative_g);
                let neighbor_f = tentative_g + heuristic(neighbor);
                f_score.insert(neighbor, neighbor_f);
                open_set.push(AStarState {
                    f_score: neighbor_f,
                    g_score: tentative_g,
                    position: neighbor,
                });
            }
        }
    }

    AStarResult {
        found: false,
        distance: 0.0,
        path: Vec::new(),
        edges: Vec::new(),
        steps,
    }
}

pub struct BfsIter<'a, S: Copy = ()> {
    state: &'a GraphState<S>,
    topo: EdgeTopology,
    visited: HashSet<NodeId>,
    queue: VecDeque<NodeId>,
}

impl<'a, S: Copy> BfsIter<'a, S> {
    pub fn new(state: &'a GraphState<S>, start: NodeId) -> Self {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        if state.node_keys.contains_key(start) {
            queue.push_back(start);
            visited.insert(start);
        }
        let topo = EdgeTopology::rebuild(state);
        Self { state, topo, visited, queue }
    }
}

impl<'a, S: Copy> Iterator for BfsIter<'a, S> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.queue.pop_front()?;
        if let Some(&curr_idx) = self.state.node_keys.get(current) {
            for &edge_id in self.topo.outgoing_edges(curr_idx) {
                if let Some(&edge_idx) = self.state.edge_keys.get(edge_id) {
                    let target = self.state.edge_targets[edge_idx];
                    if self.visited.insert(target) {
                        self.queue.push_back(target);
                    }
                }
            }
        }
        Some(current)
    }
}

pub struct DfsIter<'a, S: Copy = ()> {
    state: &'a GraphState<S>,
    topo: EdgeTopology,
    visited: HashSet<NodeId>,
    stack: Vec<NodeId>,
}

impl<'a, S: Copy> DfsIter<'a, S> {
    pub fn new(state: &'a GraphState<S>, start: NodeId) -> Self {
        let visited = HashSet::new();
        let mut stack = Vec::new();
        if state.node_keys.contains_key(start) {
            stack.push(start);
        }
        let topo = EdgeTopology::rebuild(state);
        Self { state, topo, visited, stack }
    }
}

impl<'a, S: Copy> Iterator for DfsIter<'a, S> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(current) = self.stack.pop() {
            if self.visited.insert(current) {
                if let Some(&curr_idx) = self.state.node_keys.get(current) {
                    for &edge_id in self.topo.outgoing_edges(curr_idx) {
                        if let Some(&edge_idx) = self.state.edge_keys.get(edge_id) {
                            let target = self.state.edge_targets[edge_idx];
                            if !self.visited.contains(&target) {
                                self.stack.push(target);
                            }
                        }
                    }
                }
                return Some(current);
            }
        }
        None
    }
}

pub struct HierarchyWalk<'a, S: Copy = ()> {
    state: &'a GraphState<S>,
    stack: Vec<NodeId>,
}

impl<'a, S: Copy> HierarchyWalk<'a, S> {
    pub fn new(state: &'a GraphState<S>, root: NodeId) -> Self {
        let mut stack = Vec::new();
        if state.node_keys.contains_key(root) {
            stack.push(root);
        }
        Self { state, stack }
    }
}

impl<'a, S: Copy> Iterator for HierarchyWalk<'a, S> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.stack.pop()?;
        if let Some(&idx) = self.state.node_keys.get(current) {
            let mut child = *self.state.hierarchy.first_child.get(idx);
            let mut children_rev = Vec::new();
            while let Some(c) = child {
                children_rev.push(c);
                if let Some(&c_idx) = self.state.node_keys.get(c) {
                    child = *self.state.hierarchy.next_sibling.get(c_idx);
                } else {
                    break;
                }
            }
            for &c in children_rev.iter().rev() {
                self.stack.push(c);
            }
        }
        Some(current)
    }
}
