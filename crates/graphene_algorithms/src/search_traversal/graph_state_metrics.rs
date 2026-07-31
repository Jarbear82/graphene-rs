use crate::search_traversal::graph_state_search::{dijkstra, EdgeTopology};
use graphene_core::{EdgeId, GraphState, NodeId};
use std::collections::{HashMap, VecDeque};

/// Betweenness centrality via Brandes' algorithm.
///
/// Reference: Brandes, U. (2001). "A Faster Algorithm for Betweenness
/// Centrality." Journal of Mathematical Sociology, 25(2), 163–177.
///
/// Complexity: O(VE) for unweighted graphs (this implementation).
/// Verified against: known-answer test on star graph K_{1,n-1}
/// (see `cytoscape_algorithm_tests::test_betweenness_known_answer_star_graph`).
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

    for val in centrality.values_mut() {
        *val /= 2.0;
    }

    centrality
}

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
