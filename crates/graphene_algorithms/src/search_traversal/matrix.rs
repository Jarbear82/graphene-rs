use crate::search_traversal::graph_state_search::EdgeTopology;
use graphene_core::GraphState;

#[derive(Debug, Clone)]
pub struct CsrMatrix {
    pub row_offsets: Vec<usize>,
    pub column_indices: Vec<usize>,
    pub values: Vec<f64>,
    pub shape: (usize, usize),
}

pub fn to_csr<S: Copy>(state: &GraphState<S>) -> CsrMatrix {
    to_csr_weighted(state, |_| 1.0)
}

pub fn to_csr_weighted<S: Copy>(
    state: &GraphState<S>,
    edge_weight: impl Fn(graphene_core::EdgeId) -> f64,
) -> CsrMatrix {
    let n = state.node_index_to_id.len();
    let topo = EdgeTopology::rebuild(state);
    let mut row_offsets = vec![0; n + 1];
    let mut column_indices = Vec::new();
    let mut values = Vec::new();

    for i in 0..n {
        row_offsets[i] = column_indices.len();
        for &edge_id in topo.outgoing_edges(i) {
            if let Some(&edge_idx) = state.edge_keys.get(edge_id) {
                let target_id = state.edge_targets[edge_idx];
                if let Some(&target_idx) = state.node_keys.get(target_id) {
                    column_indices.push(target_idx);
                    values.push(edge_weight(edge_id));
                }
            }
        }
    }
    row_offsets[n] = column_indices.len();

    CsrMatrix {
        row_offsets,
        column_indices,
        values,
        shape: (n, n),
    }
}

pub fn laplacian<S: Copy>(state: &GraphState<S>) -> CsrMatrix {
    laplacian_weighted(state, |_| 1.0)
}

pub fn laplacian_weighted<S: Copy>(
    state: &GraphState<S>,
    edge_weight: impl Fn(graphene_core::EdgeId) -> f64,
) -> CsrMatrix {
    let csr = to_csr_weighted(state, edge_weight);
    let n = csr.shape.0;

    let mut row_offsets = vec![0; n + 1];
    let mut column_indices = Vec::new();
    let mut values = Vec::new();

    for i in 0..n {
        row_offsets[i] = column_indices.len();
        let mut row_deg = 0.0;
        for ptr in csr.row_offsets[i]..csr.row_offsets[i + 1] {
            row_deg += csr.values[ptr];
        }
        column_indices.push(i);
        values.push(row_deg);

        for ptr in csr.row_offsets[i]..csr.row_offsets[i + 1] {
            let col = csr.column_indices[ptr];
            let val = csr.values[ptr];
            if col != i {
                column_indices.push(col);
                values.push(-val);
            }
        }
    }
    row_offsets[n] = column_indices.len();

    CsrMatrix {
        row_offsets,
        column_indices,
        values,
        shape: (n, n),
    }
}
