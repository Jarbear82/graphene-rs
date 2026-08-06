use graphene_algorithms::{connected_components, hopcroft_tarjan_biconnected, tarjan_scc};
use graphene_core::{EdgeId, GraphState, NodeId};
use std::collections::HashMap;

fn build_undirected_adj_map<S: Copy>(state: &GraphState<S>) -> HashMap<u32, Vec<(u32, u32)>> {
    let mut adj: HashMap<u32, Vec<(u32, u32)>> = HashMap::new();
    for idx in 0..state.edges.len() {
        let src = *state.edge_sources.get(idx);
        let tgt = *state.edge_targets.get(idx);
        if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
            adj.entry(u as u32).or_default().push((v as u32, idx as u32));
            adj.entry(v as u32).or_default().push((u as u32, idx as u32));
        }
    }
    adj
}

pub fn find_articulation_points<S: Copy + Default>(state: &GraphState<S>) -> Vec<NodeId> {
    if state.node_index_to_id.is_empty() || state.edges.is_empty() {
        return Vec::new();
    }

    let adj = build_undirected_adj_map(state);
    let result = hopcroft_tarjan_biconnected(&adj);
    let mut ap = Vec::new();
    for node_idx in result.cut_vertices {
        if let Some(&node_id) = state.node_index_to_id.get(node_idx as usize) {
            ap.push(node_id);
        }
    }
    ap
}

pub fn find_bridges<S: Copy + Default>(state: &GraphState<S>) -> Vec<EdgeId> {
    if state.edges.is_empty() {
        return Vec::new();
    }

    let adj = build_undirected_adj_map(state);
    let result = hopcroft_tarjan_biconnected(&adj);
    let mut bridges = Vec::new();
    for comp in result.components {
        if comp.len() == 1 {
            if let Some(&edge_idx_u32) = comp.iter().next() {
                let edge_idx = edge_idx_u32 as usize;
                if edge_idx < state.edge_index_to_id.len() {
                    bridges.push(state.edge_index_to_id[edge_idx]);
                }
            }
        }
    }
    bridges
}

pub fn get_components_summary<S: Copy>(
    state: &GraphState<S>,
) -> (Vec<Vec<NodeId>>, Vec<Vec<NodeId>>) {
    let wcc = connected_components(state);
    let scc = tarjan_scc(state);
    (wcc, scc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{EdgeData, Size2, Vec2};

    #[test]
    fn test_articulation_points_and_bridges_linear_graph() {
        let mut state = GraphState::<()>::new();
        let n0 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
        let n1 = state.add_node(Vec2::new(10.0, 0.0), Size2::new(10.0, 10.0));
        let n2 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(10.0, 10.0));

        let e0 = state.add_edge(n0, n1, EdgeData::default());
        let e1 = state.add_edge(n1, n2, EdgeData::default());

        let aps = find_articulation_points(&state);
        let bridges = find_bridges(&state);

        assert_eq!(aps, vec![n1]);
        assert_eq!(bridges.len(), 2);
        assert!(bridges.contains(&e0));
        assert!(bridges.contains(&e1));
    }
}


