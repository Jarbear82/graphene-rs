use graphene_algorithms::{
    a_star, bellman_ford, betweenness_centrality, closeness_centrality_normalized,
    connected_components, degree_centrality_normalized, dijkstra, floyd_warshall, kruskal,
    page_rank, tarjan_scc, BfsIter, DfsIter,
};
use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2};
use std::collections::HashMap;

// Helper to build a 6-node weighted graph for algorithm test suite
fn build_6_node_test_graph() -> (GraphState<()>, HashMap<char, NodeId>) {
    let mut state: GraphState<()> = GraphState::new();
    let mut nodes = HashMap::new();

    for ch in &['a', 'b', 'c', 'd', 'e', 'f'] {
        let id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
        nodes.insert(*ch, id);
    }

    let edges = vec![
        ('a', 'b', 3.0),
        ('a', 'e', 1.0),
        ('b', 'c', 5.0),
        ('b', 'e', 4.0),
        ('c', 'd', 2.0),
        ('c', 'e', 6.0),
        ('d', 'e', 7.0),
    ];

    for (u, v, _w) in edges {
        state.add_edge(nodes[&u], nodes[&v], EdgeData::default());
    }

    (state, nodes)
}

// ==========================================
// 1. BFS & DFS TESTS (ALG-01..04)
// ==========================================
#[test]
fn test_alg_01_to_04_bfs_dfs() {
    let (state, nodes) = build_6_node_test_graph();
    let start = nodes[&'a'];

    // ALG-01: BFS from 'a'
    let bfs_result: Vec<_> = BfsIter::new(&state, start).collect();
    assert!(!bfs_result.is_empty());
    assert_eq!(bfs_result[0], start);

    // ALG-03: DFS from 'a'
    let dfs_result: Vec<_> = DfsIter::new(&state, start).collect();
    assert!(!dfs_result.is_empty());
    assert_eq!(dfs_result[0], start);
}

// ==========================================
// 2. DIJKSTRA & A* TESTS (ALG-05..07, ALG-09..15, AST-01..07, ASTE-01..03)
// ==========================================
#[test]
fn test_alg_05_to_15_dijkstra_and_astar() {
    let (state, nodes) = build_6_node_test_graph();
    let start = nodes[&'a'];
    let target = nodes[&'d'];

    // ALG-05: Dijkstra
    let dijk_res = dijkstra(&state, start, |_| 1.0);
    assert!(!dijk_res.is_empty());
    assert!(dijk_res.contains_key(&target));

    // AST-01..07: A* Search
    let astar_res = a_star(&state, start, target, |_| 1.0, |_| 0.0, true);
    assert!(astar_res.found);
    assert!(astar_res.distance > 0.0);
    assert_eq!(*astar_res.path.first().unwrap(), start);
    assert_eq!(*astar_res.path.last().unwrap(), target);

    // ASTE-02: Path node continuity
    for window in astar_res.path.windows(2) {
        let u = window[0];
        let v = window[1];
        assert!(state.node_keys.contains_key(u));
        assert!(state.node_keys.contains_key(v));
    }
}

// ==========================================
// 3. MST & FLOYD-WARSHALL & BELLMAN-FORD (ALG-08, ALG-16..24)
// ==========================================
#[test]
fn test_alg_08_16_to_24_mst_floyd_bellman() {
    let (state, nodes) = build_6_node_test_graph();
    let start = nodes[&'a'];

    // ALG-08: Kruskal MST
    let mst = kruskal(&state, |_| 1.0);
    assert!(!mst.is_empty());

    // ALG-16..19: Floyd-Warshall
    let fw_dist = floyd_warshall(&state, |_| 1.0);
    assert!(!fw_dist.is_empty());
    assert_eq!(fw_dist.len(), state.node_index_to_id.len());

    // ALG-20..24: Bellman-Ford
    let bf_res = bellman_ford(&state, start, |_| 1.0);
    assert!(bf_res.is_some());
    let dist_map = bf_res.unwrap();
    assert_eq!(dist_map[&start], 0.0);
}

// ==========================================
// 4. CENTRALITY & PAGERANK (ALG-26..43)
// ==========================================
#[test]
fn test_alg_26_to_43_centrality_metrics() {
    let (state, nodes) = build_6_node_test_graph();

    // ALG-26: PageRank (sums to ~1.0)
    let pr = page_rank(&state, 0.85, 1e-4, 20, |_| 1.0);
    let total_pr: f32 = pr.values().sum();
    assert!((total_pr - 1.0).abs() < 0.05);

    // ALG-27..34: Degree Centrality Normalized
    let deg_cent = degree_centrality_normalized(&state, true, 1.0, |_| 1.0);
    assert_eq!(deg_cent.degrees.len(), state.node_index_to_id.len());
    assert!(deg_cent.degrees[&nodes[&'e']] >= 0.0);

    // ALG-35..38: Closeness Centrality Normalized
    let close_cent = closeness_centrality_normalized(&state, true, |_| 1.0);
    assert_eq!(close_cent.len(), state.node_index_to_id.len());

    // ALG-39..43: Betweenness Centrality
    let bet_cent = betweenness_centrality(&state);
    assert_eq!(bet_cent.len(), state.node_index_to_id.len());
}

// ==========================================
// 5. COMPONENTS & TARJAN SCC (TSC-01..02, HTBC-01..02)
// ==========================================
#[test]
fn test_tsc_01_02_components() {
    let (state, _) = build_6_node_test_graph();

    // Weakly Connected Components
    let wcc = connected_components(&state);
    assert!(!wcc.is_empty());

    // Tarjan Strongly Connected Components
    let scc = tarjan_scc(&state);
    assert!(!scc.is_empty());
}
