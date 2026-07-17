use graphene_core::{EdgeData, GraphState, Size2, Vec2};
use graphene_layout::{
    CircleLayout, ForceDirectedLayout, Layout,
    KamadaKawaiLayout, SugiyamaLayout, ReingoldTilfordLayout, MdsLayout,
    GridSortedLayout, ConcentricHubLayout, BipartiteLayout,
    WeightedForceDirectedLayout, CollisionForceDirectedLayout,
    compute_multigraph_bezier_routing,
    star_expand_hypergraph, DisconnectedPacker, CompoundLayout,
    RegionalPartitionLayout
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
    let mut state = GraphState::<()>::new();

    // Small: A - B, B - C, C - A
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let c = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    state.add_edge(a, b, EdgeData::default());
    state.add_edge(b, c, EdgeData::default());
    state.add_edge(c, a, EdgeData::default());

    let mut circle = CircleLayout { radius: 50.0, center: Vec2::default(), animate: false };
    circle.compute(&mut state);
    assert_valid_positions(&state);

    // Medium (Petersen Graph structure): 10 nodes, 15 edges
    let mut state_med = GraphState::<()>::new();
    let mut nodes = Vec::new();
    for _ in 0..10 {
        nodes.push(state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0)));
    }
    let petersen_edges = vec![
        (0, 1), (1, 2), (2, 3), (3, 4), (4, 0),
        (0, 5), (1, 6), (2, 7), (3, 8), (4, 9),
        (5, 7), (7, 9), (9, 6), (6, 8), (8, 5)
    ];
    for (u, v) in petersen_edges {
        state_med.add_edge(nodes[u], nodes[v], EdgeData::default());
    }

    let mut kk = KamadaKawaiLayout::default();
    kk.compute(&mut state_med);
    assert_valid_positions(&state_med);

    // Large (Grid Mesh): 25 nodes, 40 edges
    let mut state_large = GraphState::<()>::new();
    let mut grid_nodes = Vec::new();
    for _ in 0..25 {
        grid_nodes.push(state_large.add_node(Vec2::default(), Size2::new(10.0, 10.0)));
    }
    // Connect grid row/cols
    for r in 0..5 {
        for c in 0..5 {
            let idx = r * 5 + c;
            if c < 4 { state_large.add_edge(grid_nodes[idx], grid_nodes[idx + 1], EdgeData::default()); }
            if r < 4 { state_large.add_edge(grid_nodes[idx], grid_nodes[idx + 5], EdgeData::default()); }
        }
    }
    let mut force = ForceDirectedLayout::default();
    force.compute(&mut state_large);
    assert_valid_positions(&state_large);
}

// 2. DIRECTED TESTS
#[test]
fn test_directed_layouts() {
    // Small (Feed-forward loop): A -> B, A -> C, B -> C
    let mut state = GraphState::<()>::new();
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let c = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    state.add_edge(a, b, EdgeData::default());
    state.add_edge(a, c, EdgeData::default());
    state.add_edge(b, c, EdgeData::default());

    let mut sugi = SugiyamaLayout::default();
    sugi.compute(&mut state);
    assert_valid_positions(&state);

    // Medium (Process Flow): 8 nodes, 8 edges
    let mut state_med = GraphState::<()>::new();
    let start = state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let s1 = state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let s2a = state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let s2b = state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let s3 = state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let app = state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let end = state_med.add_node(Vec2::default(), Size2::new(10.0, 10.0));

    state_med.add_edge(start, s1, EdgeData::default());
    state_med.add_edge(s1, s2a, EdgeData::default());
    state_med.add_edge(s1, s2b, EdgeData::default());
    state_med.add_edge(s2a, s3, EdgeData::default());
    state_med.add_edge(s2b, s3, EdgeData::default());
    state_med.add_edge(s3, app, EdgeData::default());
    state_med.add_edge(app, end, EdgeData::default());
    state_med.add_edge(app, s1, EdgeData::default());

    let mut sugi_med = SugiyamaLayout::default();
    sugi_med.compute(&mut state_med);
    assert_valid_positions(&state_med);

    // Large (Deep Cascade): 32 nodes
    let mut state_large = GraphState::<()>::new();
    let mut cascade_nodes = Vec::new();
    for _ in 0..32 {
        cascade_nodes.push(state_large.add_node(Vec2::default(), Size2::new(10.0, 10.0)));
    }
    // Connect binary tree-like cascades
    for i in 0..15 {
        state_large.add_edge(cascade_nodes[i], cascade_nodes[2 * i + 1], EdgeData::default());
        state_large.add_edge(cascade_nodes[i], cascade_nodes[2 * i + 2], EdgeData::default());
    }
    let mut sugi_large = SugiyamaLayout::default();
    sugi_large.compute(&mut state_large);
    assert_valid_positions(&state_large);
}

// 3. WEIGHTED TESTS
#[test]
fn test_weighted_layouts() {
    let mut state = GraphState::<()>::new();

    // Small: A - B [w=10], B - C [w=0.5], C - A [w=100]
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let c = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    
    let e1 = state.add_edge(a, b, EdgeData::default());
    let e2 = state.add_edge(b, c, EdgeData::default());
    let e3 = state.add_edge(c, a, EdgeData::default());

    let weights = vec![(e1, 10.0), (e2, 0.5), (e3, 100.0)]
        .into_iter()
        .collect::<HashMap<_, _>>();

    let mut weighted = WeightedForceDirectedLayout {
        iterations: 100,
        gravity: 1.0,
        k_rep: 30.0,
        k_att: 30.0,
        weight_fn: |edge| *weights.get(&edge).unwrap_or(&1.0),
    };
    weighted.compute(&mut state);
    assert_valid_positions(&state);
}

// 4. MULTIGRAPH TESTS
#[test]
fn test_multigraph_layouts() {
    let mut state = GraphState::<()>::new();

    // Small: A -> B (e1), A -> B (e2), B -> A (e3)
    let a = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::new(100.0, 0.0), Size2::new(10.0, 10.0));
    let _e1 = state.add_edge(a, b, EdgeData::default());
    let _e2 = state.add_edge(a, b, EdgeData::default());
    let _e3 = state.add_edge(b, a, EdgeData::default());

    let routes = compute_multigraph_bezier_routing(&state, 20.0);
    assert_eq!(routes.len(), 3);
}

// 5. COMPOUND TESTS
#[test]
fn test_compound_layouts() {
    let mut state = GraphState::<()>::new();

    // Small: Group1 { A, B }
    let parent = state.add_node(Vec2::default(), Size2::new(50.0, 50.0));
    let a = state.add_node(Vec2::new(-10.0, 0.0), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::new(10.0, 0.0), Size2::new(10.0, 10.0));

    // Link a & b to parent hierarchy
    let _parent_idx = state.node_keys[parent];
    let a_idx = state.node_keys[a];
    let b_idx = state.node_keys[b];
    state.hierarchy.parent.set(a_idx, Some(parent));
    state.hierarchy.parent.set(b_idx, Some(parent));

    let mut comp = CompoundLayout {
        sub_layout: ForceDirectedLayout::default(),
        padding: 10.0,
    };
    comp.compute(&mut state);
    assert_valid_positions(&state);
}

// 6. HYPERGRAPH TESTS
#[test]
fn test_hypergraph_expansion() {
    let mut state = GraphState::<()>::new();
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let c = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let d = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));

    // E1: {A, B, C}, E2: {C, D}
    let hyperedges = vec![
        vec![a, b, c],
        vec![c, d]
    ];

    let expanded = star_expand_hypergraph(&state, &hyperedges);
    assert_eq!(expanded.node_index_to_id.len(), 6); // 4 nodes + 2 virtual hypernodes
    assert_eq!(expanded.edges.len(), 5); // 3 edges (from E1) + 2 edges (from E2)
}

// 7. ATTRIBUTE NETWORK TESTS
#[test]
fn test_attribute_regional_layouts() {
    let mut state = GraphState::<()>::new();
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));

    // Region clusters
    let clusters = vec![(a, 0), (b, 1)].into_iter().collect::<HashMap<_, _>>();

    let mut regional = RegionalPartitionLayout {
        cluster_fn: |id| *clusters.get(&id).unwrap_or(&0),
        sub_layout: ForceDirectedLayout::default(),
        columns: 2,
        cell_size: 200.0,
    };
    regional.compute(&mut state);
    assert_valid_positions(&state);
}

// 8. CHART NODES TESTS
#[test]
fn test_chart_nodes_collision() {
    let mut state = GraphState::<()>::new();
    // N1 [chart] -> N2 [chart]
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(30.0, 30.0));
    let n2 = state.add_node(Vec2::new(5.0, 5.0), Size2::new(40.0, 40.0));
    state.add_edge(n1, n2, EdgeData::default());

    let mut collision = CollisionForceDirectedLayout::default();
    collision.compute(&mut state);
    assert_valid_positions(&state);
}

// 9. SPARSE TESTS
#[test]
fn test_sparse_grid_sorting() {
    let mut state = GraphState::<()>::new();
    // Nodes A, B, C, D, E
    for _ in 0..5 {
        state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    }
    let mut grid = GridSortedLayout::default();
    grid.compute(&mut state);
    assert_valid_positions(&state);
}

// 10. DENSE TESTS
#[test]
fn test_dense_clique_layouts() {
    let mut state = GraphState::<()>::new();
    let mut nodes = Vec::new();
    // Clique K4
    for _ in 0..4 {
        nodes.push(state.add_node(Vec2::default(), Size2::new(10.0, 10.0)));
    }
    for i in 0..4 {
        for j in (i + 1)..4 {
            state.add_edge(nodes[i], nodes[j], EdgeData::default());
        }
    }

    let mut mds = MdsLayout::default();
    mds.compute(&mut state);
    assert_valid_positions(&state);
}

// 11. DISCONNECTED TESTS
#[test]
fn test_disconnected_packer() {
    let mut state = GraphState::<()>::new();
    // Component 1: A - B
    let a = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::new(10.0, 0.0), Size2::new(10.0, 10.0));
    state.add_edge(a, b, EdgeData::default());

    // Component 2: C - D
    let c = state.add_node(Vec2::new(100.0, 0.0), Size2::new(10.0, 10.0));
    let d = state.add_node(Vec2::new(110.0, 0.0), Size2::new(10.0, 10.0));
    state.add_edge(c, d, EdgeData::default());

    let mut packer = DisconnectedPacker {
        sub_layout: ForceDirectedLayout::default(),
        spacing: 50.0,
    };
    packer.compute(&mut state);
    assert_valid_positions(&state);
}

// 12. ACYCLIC TESTS
#[test]
fn test_acyclic_reingold_tilford() {
    let mut state = GraphState::<()>::new();
    let root = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let l1 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let r1 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let l2 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));

    state.add_edge(root, l1, EdgeData::default());
    state.add_edge(root, r1, EdgeData::default());
    state.add_edge(l1, l2, EdgeData::default());

    // Setup hierarchy tree parent backlinks
    let l1_idx = state.node_keys[l1];
    let r1_idx = state.node_keys[r1];
    let l2_idx = state.node_keys[l2];
    state.hierarchy.parent.set(l1_idx, Some(root));
    state.hierarchy.parent.set(r1_idx, Some(root));
    state.hierarchy.parent.set(l2_idx, Some(l1));

    let mut rt = ReingoldTilfordLayout::default();
    rt.compute(&mut state);
    assert_valid_positions(&state);
}

// 13. CYCLIC TESTS
#[test]
fn test_cyclic_mds() {
    let mut state = GraphState::<()>::new();
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let c = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    state.add_edge(a, b, EdgeData::default());
    state.add_edge(b, c, EdgeData::default());
    state.add_edge(c, a, EdgeData::default());

    let mut mds = MdsLayout::default();
    mds.compute(&mut state);
    assert_valid_positions(&state);
}

// 14. SCALE-FREE TESTS
#[test]
fn test_scale_free_concentric() {
    let mut state = GraphState::<()>::new();
    let hub = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    for _ in 0..4 {
        let leaf = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
        state.add_edge(hub, leaf, EdgeData::default());
    }

    let mut concentric = ConcentricHubLayout::default();
    concentric.compute(&mut state);
    assert_valid_positions(&state);
}

// 15. BIPARTITE TESTS
#[test]
fn test_bipartite_columns() {
    let mut state = GraphState::<()>::new();
    let u1 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let u2 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let v1 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let v2 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));

    state.add_edge(u1, v1, EdgeData::default());
    state.add_edge(u1, v2, EdgeData::default());
    state.add_edge(u2, v1, EdgeData::default());

    let node_partitions = vec![(u1, 0), (u2, 0), (v1, 1), (v2, 1)]
        .into_iter()
        .collect::<HashMap<_, _>>();

    let mut bipartite = BipartiteLayout {
        partition_fn: |id| *node_partitions.get(&id).unwrap_or(&0),
        column_spacing: 100.0,
        vertical_spacing: 50.0,
    };
    bipartite.compute(&mut state);
    assert_valid_positions(&state);
}
