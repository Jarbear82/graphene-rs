use graphene_core::fixtures::get_all_fixtures;
use graphene_core::{GraphState, Vec2};
use graphene_layout::{
    compute_multigraph_bezier_routing, star_expand_hypergraph, BipartiteLayout, CircleLayout,
    CollisionForceDirectedLayout, CompoundLayout, ConcentricHubLayout, DisconnectedPacker,
    ForceDirectedLayout, GridSortedLayout, KamadaKawaiLayout, Layout, MdsLayout,
    RegionalPartitionLayout, ReingoldTilfordLayout, SugiyamaLayout, WeightedForceDirectedLayout,
};
use std::collections::HashMap;

// Helper to assert all positions are valid finite numbers
fn assert_valid_positions<S: Copy>(state: &GraphState<S>) {
    for i in 0..state.node_index_to_id.len() {
        let pos = *state.positions.get(i);
        assert!(pos.x.is_finite(), "Position X is not finite");
        assert!(pos.y.is_finite(), "Position Y is not finite");
    }
}

// 1. UNDIRECTED TESTS
#[test]
fn test_undirected_layouts() {
    let fixtures = get_all_fixtures::<()>();

    // Small: Undirected Small (Cycle)
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Small"))
        .unwrap()
        .clone();
    let mut circle = CircleLayout {
        radius: 50.0,
        center: Vec2::default(),
        animate: false,
    };
    circle.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);

    // Medium: Undirected Medium (Petersen)
    let mut f_med = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Medium"))
        .unwrap()
        .clone();
    let mut kk = KamadaKawaiLayout::default();
    kk.compute(&mut f_med.state);
    assert_valid_positions(&f_med.state);

    // Large: Undirected Large (Grid)
    let mut f_large = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Large"))
        .unwrap()
        .clone();
    let mut force = ForceDirectedLayout::default();
    force.compute(&mut f_large.state);
    assert_valid_positions(&f_large.state);
}

// 2. DIRECTED TESTS
#[test]
fn test_directed_layouts() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Directed Small"))
        .unwrap()
        .clone();
    let mut sugi = SugiyamaLayout::default();
    sugi.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);

    // Medium
    let mut f_med = fixtures
        .iter()
        .find(|f| f.name.contains("Directed Medium"))
        .unwrap()
        .clone();
    let mut sugi_med = SugiyamaLayout::default();
    sugi_med.compute(&mut f_med.state);
    assert_valid_positions(&f_med.state);

    // Large
    let mut f_large = fixtures
        .iter()
        .find(|f| f.name.contains("Directed Large"))
        .unwrap()
        .clone();
    let mut sugi_large = SugiyamaLayout::default();
    sugi_large.compute(&mut f_large.state);
    assert_valid_positions(&f_large.state);
}

// 3. WEIGHTED TESTS
#[test]
fn test_weighted_layouts() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Weighted Small"))
        .unwrap()
        .clone();
    let weights = f_small.weights.clone();
    let edge_keys = f_small.state.edge_keys.clone();
    let mut weighted = WeightedForceDirectedLayout {
        iterations: 100,
        gravity: 1.0,
        k_rep: 30.0,
        k_att: 30.0,
        weight_fn: move |edge| {
            if let Some(&idx) = edge_keys.get(edge) {
                *weights.get(&idx).unwrap_or(&1.0)
            } else {
                1.0
            }
        },
    };
    weighted.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);

    // Medium
    let mut f_med = fixtures
        .iter()
        .find(|f| f.name.contains("Weighted Medium"))
        .unwrap()
        .clone();
    let weights_med = f_med.weights.clone();
    let edge_keys_med = f_med.state.edge_keys.clone();
    let mut weighted_med = WeightedForceDirectedLayout {
        iterations: 100,
        gravity: 1.0,
        k_rep: 30.0,
        k_att: 30.0,
        weight_fn: move |edge| {
            if let Some(&idx) = edge_keys_med.get(edge) {
                *weights_med.get(&idx).unwrap_or(&1.0)
            } else {
                1.0
            }
        },
    };
    weighted_med.compute(&mut f_med.state);
    assert_valid_positions(&f_med.state);
}

// 4. MULTIGRAPH TESTS
#[test]
fn test_multigraph_layouts() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Multigraph Small"))
        .unwrap()
        .clone();
    let routes = compute_multigraph_bezier_routing(&f_small.state, 20.0);
    assert!(routes.len() >= 2);
}

// 5. COMPOUND TESTS
#[test]
fn test_compound_layouts() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Compound Small"))
        .unwrap()
        .clone();
    let mut comp = CompoundLayout {
        sub_layout: ForceDirectedLayout::default(),
        padding: 10.0,
    };
    comp.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 6. HYPERGRAPH TESTS
#[test]
fn test_hypergraph_expansion() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Hypergraph Small"))
        .unwrap()
        .clone();
    let expanded = star_expand_hypergraph(&f_small.state, &f_small.hyperedges);
    assert!(expanded.node_index_to_id.len() > f_small.state.node_index_to_id.len());
}

// 7. ATTRIBUTE NETWORK TESTS
#[test]
fn test_attribute_regional_layouts() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Attribute Small"))
        .unwrap()
        .clone();
    let mut clusters = HashMap::new();
    for (idx, &id) in f_small.state.node_index_to_id.iter().enumerate() {
        clusters.insert(id, idx % 2);
    }
    let mut regional = RegionalPartitionLayout {
        cluster_fn: move |id| *clusters.get(&id).unwrap_or(&0),
        sub_layout: ForceDirectedLayout::default(),
        columns: 2,
        cell_size: 200.0,
    };
    regional.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 8. CHART NODES TESTS
#[test]
fn test_chart_nodes_collision() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Chart Nodes Small"))
        .unwrap()
        .clone();
    let mut collision = CollisionForceDirectedLayout::default();
    collision.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 9. SPARSE TESTS
#[test]
fn test_sparse_grid_sorting() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Sparse Small"))
        .unwrap()
        .clone();
    let mut grid = GridSortedLayout::default();
    grid.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 10. DENSE TESTS
#[test]
fn test_dense_clique_layouts() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Dense Small"))
        .unwrap()
        .clone();
    let mut mds = MdsLayout::default();
    mds.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 11. DISCONNECTED TESTS
#[test]
fn test_disconnected_packer() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Disconnected Small"))
        .unwrap()
        .clone();
    let mut packer = DisconnectedPacker {
        sub_layout: ForceDirectedLayout::default(),
        spacing: 50.0,
    };
    packer.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 12. ACYCLIC TESTS
#[test]
fn test_acyclic_reingold_tilford() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Acyclic Small"))
        .unwrap()
        .clone();
    let mut rt = ReingoldTilfordLayout::default();
    rt.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 13. CYCLIC TESTS
#[test]
fn test_cyclic_mds() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Cyclic Small"))
        .unwrap()
        .clone();
    let mut mds = MdsLayout::default();
    mds.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 14. SCALE-FREE TESTS
#[test]
fn test_scale_free_concentric() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Scale-Free Small"))
        .unwrap()
        .clone();
    let mut concentric = ConcentricHubLayout::default();
    concentric.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}

// 15. BIPARTITE TESTS
#[test]
fn test_bipartite_columns() {
    let fixtures = get_all_fixtures::<()>();

    // Small
    let mut f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Bipartite Small"))
        .unwrap()
        .clone();
    let node_partitions = vec![0, 0, 1, 1]; // matching small bipartite nodes
    let node_keys_map = f_small.state.node_keys.clone();
    let mut bipartite = BipartiteLayout {
        partition_fn: move |id| {
            let idx = *node_keys_map.get(id).unwrap_or(&0);
            node_partitions[idx % 4]
        },
        column_spacing: 100.0,
        vertical_spacing: 50.0,
    };
    bipartite.compute(&mut f_small.state);
    assert_valid_positions(&f_small.state);
}
