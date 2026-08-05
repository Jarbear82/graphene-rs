use graphene_core::{math::Vec2, GraphState};

/// Computes nesting-depth scaled ideal edge length.
pub fn compute_nesting_edge_length(ideal_len: f32, factor: f32, depth: usize) -> f32 {
    ideal_len * factor.powf(depth as f32)
}

/// Updates AABB bounding box and center coordinates for compound parent nodes
/// based on their current children's positions and dimensions plus padding.
pub fn update_compound_cart_bounds<S: Copy>(state: &mut GraphState<S>, padding: f32) {
    crate::traits::resolve_compound_bounds(state, &std::collections::HashSet::new(), padding);
}

/// Calculates gravitational pull towards compound parent center for nested child nodes.
pub fn apply_compound_parent_gravitational_forces<S: Copy>(
    state: &mut GraphState<S>,
    gravity: f32,
    delta_time: f32,
) {
    let n = state.node_index_to_id.len();
    if n == 0 || gravity.abs() < 1e-6 {
        return;
    }

    let mut position_deltas = vec![Vec2::default(); n];

    for idx in 0..n {
        if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
            if let Some(&p_idx) = state.node_keys.get(parent_id) {
                let p_pos = *state.positions.get(p_idx);
                let c_pos = *state.positions.get(idx);
                let dir = p_pos - c_pos;
                let dist = (dir.x * dir.x + dir.y * dir.y).sqrt();
                if dist > 0.001 {
                    let force = (dir / dist) * (gravity * dist * delta_time);
                    position_deltas[idx] += force;
                }
            }
        }
    }

    for (idx, delta) in position_deltas.into_iter().enumerate() {
        if delta != Vec2::default() {
            let pos = state.positions.get_mut(idx);
            *pos += delta;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_nesting_edge_length() {
        let base = 50.0;
        let factor = 1.5;
        assert_eq!(compute_nesting_edge_length(base, factor, 0), 50.0);
        assert_eq!(compute_nesting_edge_length(base, factor, 1), 75.0);
        assert_eq!(compute_nesting_edge_length(base, factor, 2), 112.5);
    }
}
