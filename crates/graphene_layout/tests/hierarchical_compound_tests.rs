use graphene_core::{math::Size2, math::Vec2, GraphState};
use graphene_layout::{
    apply_compound_parent_gravitational_forces, CircleLayout, FCoseLayout, HierarchicalLayout,
    HybridCompoundLayout, Layout,
};

#[test]
fn test_hierarchical_layout_multi_level_nesting() {
    let mut state: GraphState<()> = GraphState::new();

    let root_a = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));
    let parent_b = state.add_node(Vec2::new(10.0, 10.0), Size2::new(40.0, 40.0));
    let child_c = state.add_node(Vec2::new(20.0, 20.0), Size2::new(20.0, 20.0));
    let child_d = state.add_node(Vec2::new(30.0, 30.0), Size2::new(20.0, 20.0));

    state.reparent_node(parent_b, Some(root_a));
    state.reparent_node(child_c, Some(parent_b));
    state.reparent_node(child_d, Some(parent_b));

    let mut layout = HierarchicalLayout::new(CircleLayout::default()).with_padding(15.0);
    layout.compute(&mut state);

    // Verify all positions are finite
    for &id in &state.node_index_to_id {
        let idx = state.node_keys[id];
        let pos = state.positions.get(idx);
        let size = state.sizes.get(idx);
        assert!(pos.x.is_finite());
        assert!(pos.y.is_finite());
        assert!(size.w.is_finite() && size.w > 0.0);
        assert!(size.h.is_finite() && size.h > 0.0);
    }

    // Verify parent B bounds enclose children C and D
    let b_idx = state.node_keys[parent_b];
    let b_pos = *state.positions.get(b_idx);
    let b_size = *state.sizes.get(b_idx);
    let b_min_x = b_pos.x - b_size.w / 2.0;
    let b_max_x = b_pos.x + b_size.w / 2.0;
    let b_min_y = b_pos.y - b_size.h / 2.0;
    let b_max_y = b_pos.y + b_size.h / 2.0;

    for &child in &[child_c, child_d] {
        let c_idx = state.node_keys[child];
        let c_pos = *state.positions.get(c_idx);
        let c_size = *state.sizes.get(c_idx);
        let c_min_x = c_pos.x - c_size.w / 2.0;
        let c_max_x = c_pos.x + c_size.w / 2.0;
        let c_min_y = c_pos.y - c_size.h / 2.0;
        let c_max_y = c_pos.y + c_size.h / 2.0;

        assert!(c_min_x >= b_min_x - 0.01);
        assert!(c_max_x <= b_max_x + 0.01);
        assert!(c_min_y >= b_min_y - 0.01);
        assert!(c_max_y <= b_max_y + 0.01);
    }
}

#[test]
fn test_hybrid_compound_layout_execution() {
    let mut state: GraphState<()> = GraphState::new();

    let parent1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(60.0, 60.0));
    let c1 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(20.0, 20.0));
    let c2 = state.add_node(Vec2::new(20.0, 20.0), Size2::new(20.0, 20.0));

    let parent2 = state.add_node(Vec2::new(100.0, 100.0), Size2::new(60.0, 60.0));
    let c3 = state.add_node(Vec2::new(110.0, 110.0), Size2::new(20.0, 20.0));

    state.reparent_node(c1, Some(parent1));
    state.reparent_node(c2, Some(parent1));
    state.reparent_node(c3, Some(parent2));

    state.add_edge(c1, c2, graphene_core::EdgeData::default());
    state.add_edge(c2, c3, graphene_core::EdgeData::default());

    let mut hybrid = HybridCompoundLayout::new(CircleLayout::default(), FCoseLayout::default())
        .with_padding(20.0);
    hybrid.compute(&mut state);

    for &id in &state.node_index_to_id {
        let idx = state.node_keys[id];
        let pos = state.positions.get(idx);
        assert!(pos.x.is_finite());
        assert!(pos.y.is_finite());
    }
}

#[test]
fn test_compound_parent_gravitational_forces() {
    let mut state: GraphState<()> = GraphState::new();

    let parent = state.add_node(Vec2::new(100.0, 100.0), Size2::new(80.0, 80.0));
    let child = state.add_node(Vec2::new(0.0, 0.0), Size2::new(20.0, 20.0));
    state.reparent_node(child, Some(parent));

    let parent_pos = Vec2::new(100.0, 100.0);
    let child_pos = Vec2::new(0.0, 0.0);
    let initial_dist = ((parent_pos.x - child_pos.x).powi(2) + (parent_pos.y - child_pos.y).powi(2)).sqrt();

    apply_compound_parent_gravitational_forces(&mut state, 0.5, 0.1);

    let child_idx = state.node_keys[child];
    let new_pos = *state.positions.get(child_idx);
    let new_dist = ((parent_pos.x - new_pos.x).powi(2) + (parent_pos.y - new_pos.y).powi(2)).sqrt();

    assert!(new_dist < initial_dist);
}
