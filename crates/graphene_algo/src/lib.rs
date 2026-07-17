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
    let mut visited = std::collections::HashSet::new();
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
    let mut visited = std::collections::HashSet::new();
    let mut stack = Vec::new();

    if state.node_keys.contains_key(start_node) {
        stack.push(start_node);
    }

    let topo = EdgeTopology::rebuild(state);

    while let Some(current) = stack.pop() {
        if visited.insert(current) {
            visitor(current);

            if let Some(&curr_idx) = state.node_keys.get(current) {
                // Push nodes onto the stack
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

pub fn betweenness_centrality<S: Copy>(state: &GraphState<S>) -> HashMap<NodeId, f32> {
    let mut centrality = HashMap::new();
    for &id in &state.node_index_to_id {
        centrality.insert(id, 0.0);
    }

    let topo = EdgeTopology::rebuild(state);
    let num_nodes = state.node_index_to_id.len();

    for &s in &state.node_index_to_id {
        let mut stack = Vec::new();
        let mut pred = vec![Vec::new(); num_nodes];
        let mut sigma = vec![0.0; num_nodes];
        if let Some(&s_idx) = state.node_keys.get(s) {
            sigma[s_idx] = 1.0;
        }
        let mut dist = vec![-1.0; num_nodes];
        if let Some(&s_idx) = state.node_keys.get(s) {
            dist[s_idx] = 0.0;
        }

        let mut queue = VecDeque::new();
        queue.push_back(s);

        while let Some(v) = queue.pop_front() {
            stack.push(v);
            let v_idx = state.node_keys[v];
            let v_dist = dist[v_idx];
            let v_sigma = sigma[v_idx];

            for &edge_id in topo.outgoing_edges(v_idx) {
                if let Some(&edge_idx) = state.edge_keys.get(edge_id) {
                    let w = state.edge_targets[edge_idx];
                    let w_idx = state.node_keys[w];

                    if dist[w_idx] < 0.0 {
                        dist[w_idx] = v_dist + 1.0;
                        queue.push_back(w);
                    }

                    if dist[w_idx] == v_dist + 1.0 {
                        sigma[w_idx] += v_sigma;
                        pred[w_idx].push(v);
                    }
                }
            }
        }

        let mut delta = vec![0.0; num_nodes];
        while let Some(w) = stack.pop() {
            let w_idx = state.node_keys[w];
            let w_sigma = sigma[w_idx];
            let w_delta = delta[w_idx];

            for &v in &pred[w_idx] {
                let v_idx = state.node_keys[v];
                let v_sigma = sigma[v_idx];
                let factor = (v_sigma / w_sigma) * (1.0 + w_delta);
                delta[v_idx] += factor;
            }

            if w != s {
                if let Some(val) = centrality.get_mut(&w) {
                    *val += delta[w_idx];
                }
            }
        }
    }

    centrality
}

pub fn connected_components<S: Copy>(state: &GraphState<S>) -> Vec<Vec<NodeId>> {
    let mut visited = std::collections::HashSet::new();
    let mut components = Vec::new();

    let mut adj = HashMap::new();
    for &id in &state.node_index_to_id {
        adj.insert(id, Vec::new());
    }
    for i in 0..state.edges.len() {
        let src = *state.edge_sources.get(i);
        let tgt = *state.edge_targets.get(i);
        adj.entry(src).or_default().push(tgt);
        adj.entry(tgt).or_default().push(src);
    }

    for &node in &state.node_index_to_id {
        if !visited.contains(&node) {
            let mut comp = Vec::new();
            let mut queue = VecDeque::new();
            queue.push_back(node);
            visited.insert(node);

            while let Some(curr) = queue.pop_front() {
                comp.push(curr);
                if let Some(neighbors) = adj.get(&curr) {
                    for &next in neighbors {
                        if visited.insert(next) {
                            queue.push_back(next);
                        }
                    }
                }
            }
            components.push(comp);
        }
    }

    components
}

pub fn floyd_warshall<S: Copy>(
    state: &GraphState<S>,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> Vec<Vec<f32>> {
    let n = state.node_index_to_id.len();
    let mut dist = vec![vec![f32::INFINITY; n]; n];

    for i in 0..n {
        dist[i][i] = 0.0;
    }

    for i in 0..state.edges.len() {
        let src = *state.edge_sources.get(i);
        let tgt = *state.edge_targets.get(i);
        let weight = edge_weight(state.edge_index_to_id[i]);
        if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
            if weight < dist[u][v] {
                dist[u][v] = weight;
            }
        }
    }

    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                let alt = dist[i][k] + dist[k][j];
                if alt < dist[i][j] {
                    dist[i][j] = alt;
                }
            }
        }
    }

    dist
}

pub fn bellman_ford<S: Copy>(
    state: &GraphState<S>,
    start_node: NodeId,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> Option<HashMap<NodeId, f32>> {
    let mut distances = HashMap::new();
    for &id in &state.node_index_to_id {
        distances.insert(id, f32::INFINITY);
    }

    if !state.node_keys.contains_key(start_node) {
        return Some(distances);
    }

    distances.insert(start_node, 0.0);
    let n = state.node_index_to_id.len();

    for _ in 0..(n - 1) {
        let mut relaxed = false;
        for i in 0..state.edges.len() {
            let u = *state.edge_sources.get(i);
            let v = *state.edge_targets.get(i);
            let edge_id = state.edge_index_to_id[i];
            let weight = edge_weight(edge_id);

            let dist_u = distances[&u];
            if dist_u != f32::INFINITY {
                let dist_v = distances[&v];
                if dist_u + weight < dist_v {
                    distances.insert(v, dist_u + weight);
                    relaxed = true;
                }
            }
        }
        if !relaxed {
            break;
        }
    }

    for i in 0..state.edges.len() {
        let u = *state.edge_sources.get(i);
        let v = *state.edge_targets.get(i);
        let edge_id = state.edge_index_to_id[i];
        let weight = edge_weight(edge_id);

        let dist_u = distances[&u];
        if dist_u != f32::INFINITY {
            let dist_v = distances[&v];
            if dist_u + weight < dist_v {
                return None;
            }
        }
    }

    Some(distances)
}

// === NEIGHBOR ACCESS HELPER ===

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

// === A* SEARCH ALGORITHM ===

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

// === PAGERANK ALGORITHM ===

pub fn page_rank<S: Copy>(
    state: &GraphState<S>,
    damping_factor: f32,
    precision: f32,
    iterations: usize,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> HashMap<NodeId, f32> {
    let num_nodes = state.node_index_to_id.len();
    let mut ranks = HashMap::new();
    if num_nodes == 0 {
        return ranks;
    }

    let init_rank = 1.0 / num_nodes as f32;
    for &id in &state.node_index_to_id {
        ranks.insert(id, init_rank);
    }

    let mut out_weight_sum = HashMap::new();
    for &id in &state.node_index_to_id {
        out_weight_sum.insert(id, 0.0f32);
    }

    for idx in 0..state.edges.len() {
        let src = *state.edge_sources.get(idx);
        let tgt = *state.edge_targets.get(idx);
        if src == tgt {
            continue;
        }
        let weight = edge_weight(state.edge_index_to_id[idx]);
        if let Some(sum) = out_weight_sum.get_mut(&src) {
            *sum += weight;
        }
    }

    let mut incoming_edges: HashMap<NodeId, Vec<(NodeId, EdgeId)>> = HashMap::new();
    for &id in &state.node_index_to_id {
        incoming_edges.insert(id, Vec::new());
    }
    for idx in 0..state.edges.len() {
        let src = *state.edge_sources.get(idx);
        let tgt = *state.edge_targets.get(idx);
        if src == tgt {
            continue;
        }
        let edge_id = state.edge_index_to_id[idx];
        incoming_edges.entry(tgt).or_default().push((src, edge_id));
    }

    let mut dangling_nodes = Vec::new();
    for &id in &state.node_index_to_id {
        if out_weight_sum[&id] == 0.0 {
            dangling_nodes.push(id);
        }
    }

    let additional_prob = (1.0 - damping_factor) / num_nodes as f32;

    for _iter in 0..iterations {
        let mut next_ranks = HashMap::new();
        let mut dangling_sum = 0.0;
        for &id in &dangling_nodes {
            dangling_sum += ranks[&id];
        }
        let dangling_contrib = (damping_factor * dangling_sum) / num_nodes as f32;

        let mut diff = 0.0;

        for &id in &state.node_index_to_id {
            let mut rank_sum = 0.0;
            if let Some(in_edges) = incoming_edges.get(&id) {
                for &(src, edge_id) in in_edges {
                    let src_out_sum = out_weight_sum[&src];
                    if src_out_sum > 0.0 {
                        let weight = edge_weight(edge_id);
                        rank_sum += ranks[&src] * (weight / src_out_sum);
                    }
                }
            }

            let next_rank = additional_prob + dangling_contrib + damping_factor * rank_sum;
            next_ranks.insert(id, next_rank);

            let delta = next_rank - ranks[&id];
            diff += delta * delta;
        }

        let total_rank_sum: f32 = next_ranks.values().sum();
        if total_rank_sum > 0.0 {
            for val in next_ranks.values_mut() {
                *val /= total_rank_sum;
            }
        }

        ranks = next_ranks;

        if diff.sqrt() < precision {
            break;
        }
    }

    ranks
}

// === KRUSKAL'S MST ALGORITHM ===

struct DisjointSet {
    parent: HashMap<NodeId, NodeId>,
}

impl DisjointSet {
    fn new(nodes: &[NodeId]) -> Self {
        let mut parent = HashMap::new();
        for &id in nodes {
            parent.insert(id, id);
        }
        Self { parent }
    }

    fn find(&mut self, i: NodeId) -> NodeId {
        let mut root = i;
        while root != self.parent[&root] {
            root = self.parent[&root];
        }
        let mut curr = i;
        while curr != root {
            let nxt = self.parent[&curr];
            self.parent.insert(curr, root);
            curr = nxt;
        }
        root
    }

    fn union(&mut self, i: NodeId, j: NodeId) -> bool {
        let root_i = self.find(i);
        let root_j = self.find(j);
        if root_i != root_j {
            self.parent.insert(root_i, root_j);
            true
        } else {
            false
        }
    }
}

pub fn kruskal<S: Copy>(state: &GraphState<S>, edge_weight: impl Fn(EdgeId) -> f32) -> Vec<EdgeId> {
    let mut mst = Vec::new();
    let mut edges: Vec<EdgeId> = (0..state.edges.len())
        .map(|idx| state.edge_index_to_id[idx])
        .collect();

    edges.sort_by(|&a, &b| {
        edge_weight(a)
            .partial_cmp(&edge_weight(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut ds = DisjointSet::new(&state.node_index_to_id);

    for edge_id in edges {
        if let Some(&idx) = state.edge_keys.get(edge_id) {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            if ds.union(src, tgt) {
                mst.push(edge_id);
            }
        }
    }

    mst
}

// === TARJAN'S STRONGLY CONNECTED COMPONENTS (SCC) ===

struct TarjanSCC {
    adj: AdjacencyList,
    indices: HashMap<NodeId, usize>,
    lowlink: HashMap<NodeId, usize>,
    on_stack: HashSet<NodeId>,
    stack: Vec<NodeId>,
    index: usize,
    components: Vec<Vec<NodeId>>,
}

impl TarjanSCC {
    fn run<S: Copy>(state: &GraphState<S>) -> Vec<Vec<NodeId>> {
        let adj = AdjacencyList::build(state, true);
        let mut runner = Self {
            adj,
            indices: HashMap::new(),
            lowlink: HashMap::new(),
            on_stack: HashSet::new(),
            stack: Vec::new(),
            index: 0,
            components: Vec::new(),
        };

        for &node_id in &state.node_index_to_id {
            if !runner.indices.contains_key(&node_id) {
                runner.strongconnect(node_id);
            }
        }

        runner.components
    }

    fn strongconnect(&mut self, v: NodeId) {
        self.indices.insert(v, self.index);
        self.lowlink.insert(v, self.index);
        self.index += 1;
        self.stack.push(v);
        self.on_stack.insert(v);

        let neighbors = self.adj.neighbors(v).to_vec();
        for &(neighbor, _) in &neighbors {
            if !self.indices.contains_key(&neighbor) {
                self.strongconnect(neighbor);
                let v_low = self.lowlink[&v];
                let w_low = self.lowlink[&neighbor];
                self.lowlink.insert(v, v_low.min(w_low));
            } else if self.on_stack.contains(&neighbor) {
                let v_low = self.lowlink[&v];
                let w_idx = self.indices[&neighbor];
                self.lowlink.insert(v, v_low.min(w_idx));
            }
        }

        if self.lowlink[&v] == self.indices[&v] {
            let mut component = Vec::new();
            loop {
                let w = self.stack.pop().unwrap();
                self.on_stack.remove(&w);
                component.push(w);
                if w == v {
                    break;
                }
            }
            self.components.push(component);
        }
    }
}

pub fn tarjan_scc<S: Copy>(state: &GraphState<S>) -> Vec<Vec<NodeId>> {
    TarjanSCC::run(state)
}

// === CLOSENESS CENTRALITY ===

pub fn closeness_centrality<S: Copy>(
    state: &GraphState<S>,
    root: NodeId,
    harmonic: bool,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> f32 {
    if !state.node_keys.contains_key(root) {
        return 0.0;
    }

    let distances = dijkstra(state, root, &edge_weight);

    let mut total = 0.0;
    for &node_id in &state.node_index_to_id {
        if node_id == root {
            continue;
        }
        let d = distances.get(&node_id).copied().unwrap_or(f32::INFINITY);
        if d != f32::INFINITY && d > 0.0 {
            if harmonic {
                total += 1.0 / d;
            } else {
                total += d;
            }
        }
    }

    if harmonic {
        total
    } else if total > 0.0 {
        1.0 / total
    } else {
        0.0
    }
}

pub fn closeness_centrality_normalized<S: Copy>(
    state: &GraphState<S>,
    harmonic: bool,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> HashMap<NodeId, f32> {
    let mut closenesses = HashMap::new();
    let mut max_closeness = 0.0f32;

    for &node_id in &state.node_index_to_id {
        let c = closeness_centrality(state, node_id, harmonic, &edge_weight);
        closenesses.insert(node_id, c);
        if c > max_closeness {
            max_closeness = c;
        }
    }

    for val in closenesses.values_mut() {
        if max_closeness > 0.0 {
            *val /= max_closeness;
        } else {
            *val = 0.0;
        }
    }

    closenesses
}

// === DEGREE CENTRALITY ===

#[derive(Debug, Clone, Copy)]
pub struct DegreeCentralityResult {
    pub degree: f32,
    pub indegree: f32,
    pub outdegree: f32,
}

pub fn degree_centrality<S: Copy>(
    state: &GraphState<S>,
    root: NodeId,
    directed: bool,
    alpha: f32,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> DegreeCentralityResult {
    if !state.node_keys.contains_key(root) {
        return DegreeCentralityResult {
            degree: 0.0,
            indegree: 0.0,
            outdegree: 0.0,
        };
    }

    if !directed {
        let mut k = 0.0f32;
        let mut s = 0.0f32;
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            if src == root || tgt == root {
                let edge_id = state.edge_index_to_id[idx];
                k += 1.0;
                s += edge_weight(edge_id);
            }
        }
        let degree = k.powf(1.0 - alpha) * s.powf(alpha);
        DegreeCentralityResult {
            degree,
            indegree: degree,
            outdegree: degree,
        }
    } else {
        let mut k_in = 0.0f32;
        let mut s_in = 0.0f32;
        let mut k_out = 0.0f32;
        let mut s_out = 0.0f32;
        for idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(idx);
            let tgt = *state.edge_targets.get(idx);
            let edge_id = state.edge_index_to_id[idx];
            if tgt == root {
                k_in += 1.0;
                s_in += edge_weight(edge_id);
            }
            if src == root {
                k_out += 1.0;
                s_out += edge_weight(edge_id);
            }
        }
        let indegree = k_in.powf(1.0 - alpha) * s_in.powf(alpha);
        let outdegree = k_out.powf(1.0 - alpha) * s_out.powf(alpha);
        let degree = indegree + outdegree;
        DegreeCentralityResult {
            degree,
            indegree,
            outdegree,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DegreeCentralityNormalizedResult {
    pub degrees: HashMap<NodeId, f32>,
    pub indegrees: HashMap<NodeId, f32>,
    pub outdegrees: HashMap<NodeId, f32>,
}

pub fn degree_centrality_normalized<S: Copy>(
    state: &GraphState<S>,
    directed: bool,
    alpha: f32,
    edge_weight: impl Fn(EdgeId) -> f32,
) -> DegreeCentralityNormalizedResult {
    let mut degrees = HashMap::new();
    let mut indegrees = HashMap::new();
    let mut outdegrees = HashMap::new();

    let mut max_degree = 0.0f32;
    let mut max_indegree = 0.0f32;
    let mut max_outdegree = 0.0f32;

    for &node_id in &state.node_index_to_id {
        let res = degree_centrality(state, node_id, directed, alpha, &edge_weight);
        degrees.insert(node_id, res.degree);
        indegrees.insert(node_id, res.indegree);
        outdegrees.insert(node_id, res.outdegree);

        if res.degree > max_degree {
            max_degree = res.degree;
        }
        if res.indegree > max_indegree {
            max_indegree = res.indegree;
        }
        if res.outdegree > max_outdegree {
            max_outdegree = res.outdegree;
        }
    }

    for val in degrees.values_mut() {
        if max_degree > 0.0 {
            *val /= max_degree;
        } else {
            *val = 0.0;
        }
    }
    for val in indegrees.values_mut() {
        if max_indegree > 0.0 {
            *val /= max_indegree;
        } else {
            *val = 0.0;
        }
    }
    for val in outdegrees.values_mut() {
        if max_outdegree > 0.0 {
            *val /= max_outdegree;
        } else {
            *val = 0.0;
        }
    }

    DegreeCentralityNormalizedResult {
        degrees,
        indegrees,
        outdegrees,
    }
}
