use graphene_core::{math::Vec2, EdgeId, GraphState, NodeId};
use std::collections::HashMap;

pub fn compute_multigraph_bezier_routing<S: Copy>(
    state: &GraphState<S>,
    base_offset: f32,
) -> HashMap<EdgeId, Option<Vec2>> {
    let mut edge_control_points = HashMap::new();
    let mut edge_counts: HashMap<(NodeId, NodeId), Vec<EdgeId>> = HashMap::new();

    for idx in 0..state.edges.len() {
        let edge_id = state.edge_index_to_id[idx];
        let src = *state.edge_sources.get(idx);
        let tgt = *state.edge_targets.get(idx);
        let key = if src < tgt { (src, tgt) } else { (tgt, src) };
        edge_counts.entry(key).or_default().push(edge_id);
    }

    for ((src, tgt), edges) in edge_counts {
        let num_edges = edges.len();
        if num_edges <= 1 {
            for edge_id in edges {
                edge_control_points.insert(edge_id, None);
            }
            continue;
        }

        let Some(&src_idx) = state.node_keys.get(src) else { continue };
        let Some(&tgt_idx) = state.node_keys.get(tgt) else { continue };
        let p_src = *state.positions.get(src_idx);
        let p_tgt = *state.positions.get(tgt_idx);

        let mid = (p_src + p_tgt) / 2.0;
        let diff = p_tgt - p_src;
        let length = diff.len().max(0.01);
        let perp = Vec2::new(-diff.y / length, diff.x / length);

        for (i, edge_id) in edges.into_iter().enumerate() {
            let offset_factor = (i as f32 - (num_edges - 1) as f32 / 2.0) * base_offset;
            if offset_factor == 0.0 {
                edge_control_points.insert(edge_id, None);
            } else {
                let cp = mid + perp * offset_factor;
                edge_control_points.insert(edge_id, Some(cp));
            }
        }
    }

    edge_control_points
}
