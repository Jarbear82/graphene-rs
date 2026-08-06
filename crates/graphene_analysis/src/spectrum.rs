use graphene_algorithms::laplacian;
use graphene_core::GraphState;

pub fn algebraic_connectivity<S: Copy>(state: &GraphState<S>) -> f64 {
    let lap = laplacian(state);
    let n = lap.shape.0;
    if n <= 1 {
        return 0.0;
    }

    let mut degrees = vec![0.0; n];
    for i in 0..n {
        let start = lap.row_offsets[i];
        if start < lap.values.len() {
            degrees[i] = lap.values[start];
        }
    }

    let min_deg = degrees
        .into_iter()
        .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap_or(0.0);

    (min_deg * 0.5).max(0.0)
}
