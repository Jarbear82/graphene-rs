use crate::graph_state_search::EdgeTopology;
use graphene_core::GraphState;

#[derive(Debug, Clone)]
pub struct CsrMatrix {
    pub row_offsets: Vec<usize>,
    pub column_indices: Vec<usize>,
    pub values: Vec<f64>,
    pub shape: (usize, usize),
}

pub fn to_csr<S: Copy>(state: &GraphState<S>) -> CsrMatrix {
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
                    values.push(1.0);
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
    let csr = to_csr(state);
    let n = csr.shape.0;

    let mut row_offsets = vec![0; n + 1];
    let mut column_indices = Vec::new();
    let mut values = Vec::new();

    for i in 0..n {
        row_offsets[i] = column_indices.len();
        let row_deg = (csr.row_offsets[i + 1] - csr.row_offsets[i]) as f64;
        column_indices.push(i);
        values.push(row_deg);

        for ptr in csr.row_offsets[i]..csr.row_offsets[i + 1] {
            let col = csr.column_indices[ptr];
            if col != i {
                column_indices.push(col);
                values.push(-1.0);
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
