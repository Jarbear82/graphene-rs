use graphene_core::math::{Size2, Vec2};
use graphene_core::GraphState;
use graphene_layout::{
    resolve_overlaps, CollisionForceDirectedLayout, KamadaKawaiLayout, Layout, MdsLayout,
};

fn assert_no_aabb_overlaps<S: Copy>(state: &GraphState<S>, padding: f32, layout_name: &str) {
    let n = state.node_index_to_id.len();
    for i in 0..n {
        let pos_i = *state.positions.get(i);
        let size_i = *state.sizes.get(i);
        let hw_i = size_i.w / 2.0;
        let hh_i = size_i.h / 2.0;

        for j in (i + 1)..n {
            let pos_j = *state.positions.get(j);
            let size_j = *state.sizes.get(j);
            let hw_j = size_j.w / 2.0;
            let hh_j = size_j.h / 2.0;

            let dx = (pos_i.x - pos_j.x).abs();
            let dy = (pos_i.y - pos_j.y).abs();
            let min_dx = hw_i + hw_j + padding;
            let min_dy = hh_i + hh_j + padding;

            assert!(
                dx >= min_dx || dy >= min_dy,
                "[{}] Physical body overlap detected between node index {} and {}: dx={}, min_dx={}, dy={}, min_dy={}",
                layout_name, i, j, dx, min_dx, dy, min_dy
            );
        }
    }
}

#[test]
fn test_mds_physical_body_overlap_resolution() {
    let mut state = GraphState::<()>::new();
    // Add nodes with heterogeneous graph dimensions
    let _n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(80.0, 40.0));
    let _n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(120.0, 60.0));
    let _n3 = state.add_node(Vec2::new(5.0, 5.0), Size2::new(50.0, 50.0));
    let _n4 = state.add_node(Vec2::new(0.0, 5.0), Size2::new(100.0, 80.0));

    let mut mds = MdsLayout::default();
    mds.compute(&mut state);

    assert_no_aabb_overlaps(&state, 0.0, "MdsLayout");
}

#[test]
fn test_kamada_kawai_physical_body_overlap_resolution() {
    let mut state = GraphState::<()>::new();
    let _n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(90.0, 50.0));
    let _n2 = state.add_node(Vec2::new(5.0, 5.0), Size2::new(110.0, 70.0));
    let _n3 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(60.0, 60.0));

    let mut kk = KamadaKawaiLayout::default();
    kk.compute(&mut state);

    assert_no_aabb_overlaps(&state, 0.0, "KamadaKawaiLayout");
}

#[test]
fn test_collision_force_directed_physical_body_resolution() {
    let mut state = GraphState::<()>::new();
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(100.0, 50.0));
    let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(100.0, 50.0));
    let n3 = state.add_node(Vec2::new(20.0, 20.0), Size2::new(80.0, 80.0));

    state.add_edge(n1, n2, graphene_core::EdgeData::default());
    state.add_edge(n2, n3, graphene_core::EdgeData::default());

    let mut cfd = CollisionForceDirectedLayout::default();
    cfd.compute(&mut state);

    assert_no_aabb_overlaps(&state, 0.0, "CollisionForceDirectedLayout");
}

#[test]
fn test_resolve_overlaps_standalone_padding() {
    let mut state = GraphState::<()>::new();
    let _n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));
    let _n2 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));

    resolve_overlaps(&mut state, 15.0);

    assert_no_aabb_overlaps(&state, 15.0, "resolve_overlaps_standalone");
}
