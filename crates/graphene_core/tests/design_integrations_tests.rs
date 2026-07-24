use graphene_core::{
    AllowMulti, Directed, EdgeData, GraphError, GraphState, GraphView, NodeKind, PropertyIndex,
    SimpleOnly, Size2, Undirected, UserDataValue, Vec2,
};
use graphene_algo::{laplacian, to_csr, BfsIter, DfsIter, HierarchyWalk};

#[test]
fn test_edge_type_and_insert_policy() {
    let mut state: GraphState<()> = GraphState::new();
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(10.0, 10.0));

    // Test SimpleOnly policy: reject self-loops
    let self_loop_res = state.add_edge_with_policy::<Directed, SimpleOnly>(n1, n1, EdgeData::default());
    assert_eq!(self_loop_res, Err(GraphError::SelfLoopNotAllowed));

    // Test SimpleOnly policy: allow first edge
    let e1 = state
        .add_edge_with_policy::<Directed, SimpleOnly>(n1, n2, EdgeData::default())
        .expect("First edge should succeed");
    assert!(state.edge_keys.contains_key(e1));

    // Test SimpleOnly policy: reject parallel edge in Directed mode
    let parallel_res = state.add_edge_with_policy::<Directed, SimpleOnly>(n1, n2, EdgeData::default());
    assert_eq!(parallel_res, Err(GraphError::ParallelEdgeNotAllowed));

    // Test AllowMulti policy: allow parallel edge
    let e2 = state
        .add_edge_with_policy::<Directed, AllowMulti>(n1, n2, EdgeData::default())
        .expect("Multi-graph policy should allow parallel edge");
    assert!(state.edge_keys.contains_key(e2));

    // Test SimpleOnly in Undirected mode: reverse edge rejected
    let reverse_res = state.add_edge_with_policy::<Undirected, SimpleOnly>(n2, n1, EdgeData::default());
    assert_eq!(reverse_res, Err(GraphError::ParallelEdgeNotAllowed));
}

#[test]
fn test_hyperedge_proxy() {
    let mut state: GraphState<()> = GraphState::new();
    let v1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(5.0, 5.0));
    let v2 = state.add_node(Vec2::new(10.0, 0.0), Size2::new(5.0, 5.0));
    let v3 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(5.0, 5.0));

    let proxy = state.add_hyperedge_proxy(Vec2::new(10.0, 10.0), Size2::new(8.0, 8.0), &[v1, v2, v3]);

    let proxy_idx = state.node_keys[proxy];
    assert_eq!(state.node_kinds[proxy_idx], NodeKind::HyperedgeProxy);
    let v1_idx = state.node_keys[v1];
    assert_eq!(state.node_kinds[v1_idx], NodeKind::Vertex);

    assert_eq!(state.edge_index_to_id.len(), 3);
}

#[test]
fn test_graph_view_induced_subgraph() {
    let mut state: GraphState<()> = GraphState::new();
    let a = state.add_node(Vec2::new(0.0, 0.0), Size2::new(1.0, 1.0));
    let b = state.add_node(Vec2::new(1.0, 0.0), Size2::new(1.0, 1.0));
    let c = state.add_node(Vec2::new(2.0, 0.0), Size2::new(1.0, 1.0));

    let e_ab = state.add_edge(a, b, EdgeData::default());
    let _e_bc = state.add_edge(b, c, EdgeData::default());

    let view = GraphView::induced(&state, &[a, b]);

    assert!(view.contains_node(a));
    assert!(view.contains_node(b));
    assert!(!view.contains_node(c));

    assert!(view.contains_edge(e_ab));

    let nodes: Vec<_> = view.nodes().collect();
    assert_eq!(nodes.len(), 2);
    assert!(nodes.contains(&a));
    assert!(nodes.contains(&b));
}

#[test]
fn test_property_index() {
    let mut state: GraphState<()> = GraphState::new();
    let k_type = state.string_arena.intern("type".into());

    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(1.0, 1.0));
    let n2 = state.add_node(Vec2::new(1.0, 0.0), Size2::new(1.0, 1.0));

    let idx1 = state.node_keys[n1];
    let idx2 = state.node_keys[n2];

    let val_person = UserDataValue::Integer(42);
    state.nodes.get_mut(idx1).user_data.insert(k_type, val_person);

    let index = PropertyIndex::rebuild(&state);
    let mask = index.query(k_type, val_person).expect("Query should find match");

    assert!(mask[idx1]);
    assert!(!mask[idx2]);
}

#[test]
fn test_traversal_iterators() {
    let mut state: GraphState<()> = GraphState::new();
    let r = state.add_node(Vec2::new(0.0, 0.0), Size2::new(1.0, 1.0));
    let c1 = state.add_node(Vec2::new(1.0, 0.0), Size2::new(1.0, 1.0));
    let c2 = state.add_node(Vec2::new(2.0, 0.0), Size2::new(1.0, 1.0));

    state.add_edge(r, c1, EdgeData::default());
    state.add_edge(c1, c2, EdgeData::default());

    state.reparent_node(c1, Some(r));
    state.reparent_node(c2, Some(c1));

    let bfs_order: Vec<_> = BfsIter::new(&state, r).collect();
    assert_eq!(bfs_order, vec![r, c1, c2]);

    let dfs_order: Vec<_> = DfsIter::new(&state, r).collect();
    assert_eq!(dfs_order, vec![r, c1, c2]);

    let hierarchy_order: Vec<_> = HierarchyWalk::new(&state, r).collect();
    assert_eq!(hierarchy_order, vec![r, c1, c2]);
}

#[test]
fn test_csr_and_laplacian_export() {
    let mut state: GraphState<()> = GraphState::new();
    let n0 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(1.0, 1.0));
    let n1 = state.add_node(Vec2::new(1.0, 0.0), Size2::new(1.0, 1.0));
    let n2 = state.add_node(Vec2::new(2.0, 0.0), Size2::new(1.0, 1.0));

    state.add_edge(n0, n1, EdgeData::default());
    state.add_edge(n0, n2, EdgeData::default());

    let csr = to_csr(&state);
    assert_eq!(csr.shape, (3, 3));
    assert_eq!(csr.row_offsets, vec![0, 2, 2, 2]);

    let lap = laplacian(&state);
    assert_eq!(lap.shape, (3, 3));
    // Row 0 has out-degree 2: L_00 = 2.0
    assert_eq!(lap.values[0], 2.0);
}
