use graphene_core::math::Vec2;
use graphene_core::NodeId;
use graphene_layout::{
    BipartiteLayout, BreadthFirstLayout, CircleLayout, CollisionForceDirectedLayout,
    ConcentricLayout, CoseLayout, FCoseLayout, ForceDirectedLayout, GridLayout, GridSortedLayout,
    KamadaKawaiLayout, MdsLayout, RandomLayout, ReingoldTilfordLayout, SugiyamaLayout,
};

#[test]
fn test_force_directed_layout_configurability() {
    let layout = ForceDirectedLayout::default()
        .with_iterations(300)
        .with_ideal_length(75.0)
        .with_gravity(0.2)
        .with_k_rep(3000.0)
        .with_k_att(0.08)
        .with_initial_temp(20.0)
        .with_use_barnes_hut(false)
        .with_theta(0.8);

    assert_eq!(layout.iterations, 300);
    assert_eq!(layout.ideal_length, 75.0);
    assert_eq!(layout.gravity, 0.2);
    assert_eq!(layout.k_rep, 3000.0);
    assert_eq!(layout.k_att, 0.08);
    assert_eq!(layout.initial_temp, 20.0);
    assert_eq!(layout.use_barnes_hut, false);
    assert_eq!(layout.theta, 0.8);
}

#[test]
fn test_collision_force_directed_layout_configurability() {
    let layout = CollisionForceDirectedLayout::default()
        .with_iterations(400)
        .with_gravity(2.5)
        .with_ideal_length(65.0);

    assert_eq!(layout.iterations, 400);
    assert_eq!(layout.gravity, 2.5);
    assert_eq!(layout.ideal_length, 65.0);
}

#[test]
fn test_random_layout_configurability() {
    let layout = RandomLayout::default()
        .with_width(1200.0)
        .with_height(900.0)
        .with_animate(true);

    assert_eq!(layout.width, 1200.0);
    assert_eq!(layout.height, 900.0);
    assert_eq!(layout.animate, true);
}

#[test]
fn test_grid_layout_configurability() {
    let layout = GridLayout::default()
        .with_columns(8)
        .with_spacing_x(150.0)
        .with_spacing_y(130.0)
        .with_animate(true);

    assert_eq!(layout.columns, 8);
    assert_eq!(layout.spacing_x, 150.0);
    assert_eq!(layout.spacing_y, 130.0);
    assert_eq!(layout.animate, true);
}

#[test]
fn test_circle_layout_configurability() {
    let layout = CircleLayout::default()
        .with_radius(450.0)
        .with_center(Vec2::new(10.0, 20.0))
        .with_animate(true);

    assert_eq!(layout.radius, 450.0);
    assert_eq!(layout.center, Vec2::new(10.0, 20.0));
    assert_eq!(layout.animate, true);
}

#[test]
fn test_concentric_layout_configurability() {
    let layout = ConcentricLayout::default()
        .with_level_radius_step(200.0)
        .with_center(Vec2::new(5.0, 5.0))
        .with_animate(true);

    assert_eq!(layout.level_radius_step, 200.0);
    assert_eq!(layout.center, Vec2::new(5.0, 5.0));
    assert_eq!(layout.animate, true);
}

#[test]
fn test_breadth_first_layout_configurability() {
    let layout = BreadthFirstLayout::default()
        .with_sibling_spacing(140.0)
        .with_level_spacing(160.0)
        .with_animate(true);

    assert_eq!(layout.sibling_spacing, 140.0);
    assert_eq!(layout.level_spacing, 160.0);
    assert_eq!(layout.animate, true);
}

#[test]
fn test_grid_sorted_layout_configurability() {
    let layout = GridSortedLayout::default()
        .with_columns(10)
        .with_node_spacing(100.0)
        .with_sort_by_degree(false);

    assert_eq!(layout.columns, 10);
    assert_eq!(layout.node_spacing, 100.0);
    assert_eq!(layout.sort_by_degree, false);
}

#[test]
fn test_bipartite_layout_configurability() {
    let layout = BipartiteLayout::default()
        .with_partition_fn(|_id: NodeId| 1)
        .with_column_spacing(250.0)
        .with_vertical_spacing(120.0);

    assert_eq!((layout.partition_fn)(NodeId::default()), 1);
    assert_eq!(layout.column_spacing, 250.0);
    assert_eq!(layout.vertical_spacing, 120.0);
}

#[test]
fn test_sugiyama_layout_configurability() {
    let layout = SugiyamaLayout::default()
        .with_layer_spacing(110.0)
        .with_node_spacing(90.0);

    assert_eq!(layout.layer_spacing, 110.0);
    assert_eq!(layout.node_spacing, 90.0);
}

#[test]
fn test_reingold_tilford_layout_configurability() {
    let layout = ReingoldTilfordLayout::default()
        .with_sibling_spacing(120.0)
        .with_level_spacing(140.0);

    assert_eq!(layout.sibling_spacing, 120.0);
    assert_eq!(layout.level_spacing, 140.0);
}

#[test]
fn test_kamada_kawai_layout_configurability() {
    let layout = KamadaKawaiLayout::default()
        .with_iterations(350)
        .with_k(2.0)
        .with_ideal_length(70.0);

    assert_eq!(layout.iterations, 350);
    assert_eq!(layout.k, 2.0);
    assert_eq!(layout.l_0, 70.0);
}

#[test]
fn test_mds_layout_configurability() {
    let layout = MdsLayout::default()
        .with_iterations(250)
        .with_base_dist(80.0);

    assert_eq!(layout.iterations, 250);
    assert_eq!(layout.base_dist, 80.0);
}

#[test]
fn test_cose_layout_configurability() {
    let layout = CoseLayout::default()
        .with_iterations(500)
        .with_ideal_edge_length(40.0)
        .with_edge_elasticity(45.0)
        .with_nesting_factor(1.5)
        .with_gravity(2.0)
        .with_node_repulsion(3000.0)
        .with_node_overlap(8.0)
        .with_initial_temp(500.0)
        .with_cooling_factor(0.95)
        .with_min_temp(0.5);

    assert_eq!(layout.iterations, 500);
    assert_eq!(layout.ideal_edge_length, 40.0);
    assert_eq!(layout.edge_elasticity, 45.0);
    assert_eq!(layout.nesting_factor, 1.5);
    assert_eq!(layout.gravity, 2.0);
    assert_eq!(layout.node_repulsion, 3000.0);
    assert_eq!(layout.node_overlap, 8.0);
    assert_eq!(layout.initial_temp, 500.0);
    assert_eq!(layout.cooling_factor, 0.95);
    assert_eq!(layout.min_temp, 0.5);
}

#[test]
fn test_fcose_layout_configurability() {
    let layout = FCoseLayout::default()
        .with_iterations(400)
        .with_ideal_edge_length(60.0)
        .with_nesting_factor(1.3)
        .with_gravity(1.8)
        .with_node_repulsion(4000.0)
        .with_initial_temp(60.0)
        .with_cooling_factor(0.92)
        .with_randomize(false)
        .with_compound_padding(15.0);

    assert_eq!(layout.iterations, 400);
    assert_eq!(layout.ideal_edge_length, 60.0);
    assert_eq!(layout.nesting_factor, 1.3);
    assert_eq!(layout.gravity, 1.8);
    assert_eq!(layout.node_repulsion, 4000.0);
    assert_eq!(layout.initial_temp, 60.0);
    assert_eq!(layout.cooling_factor, 0.92);
    assert_eq!(layout.randomize, false);
    assert_eq!(layout.compound_padding, 15.0);
}
