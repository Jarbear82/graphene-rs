use crate::state::GraphState;
use crate::types::*;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedNode {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedEdge {
    pub source_idx: usize,
    pub target_idx: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedGraph {
    pub nodes: Vec<SerializedNode>,
    pub edges: Vec<SerializedEdge>,
}

pub fn to_json<S: Copy>(state: &GraphState<S>) -> String {
    let serialized = SerializedGraph {
        nodes: (0..state.node_index_to_id.len()).map(|idx| {
            let pos = state.positions[idx];
            let size = state.sizes[idx];
            SerializedNode { x: pos.x, y: pos.y, w: size.w, h: size.h }
        }).collect(),
        edges: (0..state.edges.len()).filter_map(|idx| {
            let src = state.edge_sources[idx];
            let tgt = state.edge_targets[idx];
            let source_idx = *state.node_keys.get(src)?;
            let target_idx = *state.node_keys.get(tgt)?;
            Some(SerializedEdge {
                source_idx,
                target_idx,
            })
        }).collect(),
    };
    serde_json::to_string_pretty(&serialized).unwrap_or_default()
}

pub fn from_json<S: Copy + Default>(json: &str) -> Result<GraphState<S>, String> {
    let serialized: SerializedGraph = serde_json::from_str(json)
        .map_err(|e| e.to_string())?;
    
    let mut state = GraphState::new();
    let mut node_ids = Vec::new();
    for n in serialized.nodes {
        let id = state.add_node(Vec2::new(n.x, n.y), Size2::new(n.w, n.h));
        node_ids.push(id);
    }
    for e in serialized.edges {
        if e.source_idx < node_ids.len() && e.target_idx < node_ids.len() {
            state.add_edge(node_ids[e.source_idx], node_ids[e.target_idx], EdgeData::default());
        }
    }
    Ok(state)
}
