use crate::graph_state_search::AdjacencyList;
use graphene_core::{EdgeId, GraphState, NodeId};
use std::collections::{HashMap, HashSet, VecDeque};

pub fn connected_components<S: Copy>(state: &GraphState<S>) -> Vec<Vec<NodeId>> {
    let mut visited = HashSet::new();
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
