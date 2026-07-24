use graphene_algo::{connected_components, tarjan_scc};
use graphene_core::{EdgeId, GraphState, NodeId};

pub fn find_articulation_points<S: Copy + Default>(state: &GraphState<S>) -> Vec<NodeId> {
    let mut ap = Vec::new();
    let base_components = connected_components(state).len();

    for &node_id in &state.node_index_to_id {
        let mut temp_state = state.clone();
        temp_state.remove_node(node_id);
        let new_components = connected_components(&temp_state).len();
        if new_components > base_components {
            ap.push(node_id);
        }
    }

    ap
}

pub fn find_bridges<S: Copy + Default>(state: &GraphState<S>) -> Vec<EdgeId> {
    let mut bridges = Vec::new();
    let base_components = connected_components(state).len();

    for i in 0..state.edges.len() {
        let edge_id = state.edge_index_to_id[i];
        let mut temp_state = state.clone();
        temp_state.remove_edge(edge_id);
        let new_components = connected_components(&temp_state).len();
        if new_components > base_components {
            bridges.push(edge_id);
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
