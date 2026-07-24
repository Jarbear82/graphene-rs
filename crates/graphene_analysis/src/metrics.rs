use graphene_core::GraphState;

pub fn compute_density<S: Copy>(state: &GraphState<S>, directed: bool) -> f32 {
    let n = state.node_index_to_id.len();
    if n <= 1 {
        return 0.0;
    }
    let e = state.edges.len() as f32;
    let max_edges = if directed {
        (n * (n - 1)) as f32
    } else {
        (n * (n - 1)) as f32 / 2.0
    };
    if max_edges == 0.0 {
        0.0
    } else {
        (e / max_edges).min(1.0)
    }
}

pub fn compute_average_degree<S: Copy>(state: &GraphState<S>) -> f32 {
    let n = state.node_index_to_id.len();
    if n == 0 {
        return 0.0;
    }
    (2.0 * state.edges.len() as f32) / n as f32
}

pub fn compute_reciprocity<S: Copy>(state: &GraphState<S>) -> f32 {
    if state.edges.is_empty() {
        return 0.0;
    }
    let mut edge_set = std::collections::HashSet::new();
    for i in 0..state.edges.len() {
        let src = *state.edge_sources.get(i);
        let tgt = *state.edge_targets.get(i);
        edge_set.insert((src, tgt));
    }

    let mut reciprocal_count = 0;
    for &(src, tgt) in &edge_set {
        if edge_set.contains(&(tgt, src)) && src != tgt {
            reciprocal_count += 1;
        }
    }

    reciprocal_count as f32 / edge_set.len() as f32
}

pub fn compute_clustering_coefficient<S: Copy>(state: &GraphState<S>) -> f32 {
    let n = state.node_index_to_id.len();
    if n == 0 {
        return 0.0;
    }

    let mut adj = std::collections::HashMap::new();
    for &id in &state.node_index_to_id {
        adj.insert(id, std::collections::HashSet::new());
    }

    for i in 0..state.edges.len() {
        let src = *state.edge_sources.get(i);
        let tgt = *state.edge_targets.get(i);
        if src != tgt {
            if let Some(set) = adj.get_mut(&src) {
                set.insert(tgt);
            }
            if let Some(set) = adj.get_mut(&tgt) {
                set.insert(src);
            }
        }
    }

    let mut total_cc = 0.0f32;
    for &id in &state.node_index_to_id {
        let neighbors = &adj[&id];
        let k = neighbors.len();
        if k < 2 {
            continue;
        }

        let mut links = 0;
        let neighbor_vec: Vec<_> = neighbors.iter().copied().collect();
        for i in 0..neighbor_vec.len() {
            for j in (i + 1)..neighbor_vec.len() {
                let u = neighbor_vec[i];
                let v = neighbor_vec[j];
                if adj[&u].contains(&v) {
                    links += 1;
                }
            }
        }

        let possible_links = (k * (k - 1)) / 2;
        if possible_links > 0 {
            total_cc += links as f32 / possible_links as f32;
        }
    }

    total_cc / n as f32
}
