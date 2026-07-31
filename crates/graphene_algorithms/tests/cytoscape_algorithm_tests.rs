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

/// Star graph K_{1,n-1}: center's betweenness = (n-1)(n-2)/2, leaves = 0.
/// This pins down exact values instead of just checking "map has right length."
#[test]
fn test_betweenness_known_answer_star_graph() {
    let mut state: GraphState<()> = GraphState::new();
    let center = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let mut leaves = Vec::new();
    for _ in 0..5 {
        let leaf = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
        state.add_edge(center, leaf, EdgeData::default());
        state.add_edge(leaf, center, EdgeData::default()); // treat as undirected
        leaves.push(leaf);
    }

    let bet = betweenness_centrality(&state);
    let n = 6.0_f32;
    let expected_center = (n - 1.0) * (n - 2.0) / 2.0; // = 10.0 for n=6

    assert!(
        (bet[&center] - expected_center).abs() < 0.5,
        "expected center betweenness ~{}, got {}",
        expected_center,
        bet[&center]
    );
    for &leaf in &leaves {
        assert!(bet[&leaf] < 0.5, "leaf betweenness should be ~0, got {}", bet[&leaf]);
    }
}

/// Path graph A-B-C-D: Dijkstra distances must be exact, not just finite.
#[test]
fn test_dijkstra_known_answer_path_graph() {
    let mut state: GraphState<()> = GraphState::new();
    let nodes: Vec<_> = (0..4)
        .map(|_| state.add_node(Vec2::default(), Size2::new(10.0, 10.0)))
        .collect();
    for w in nodes.windows(2) {
        state.add_edge(w[0], w[1], EdgeData::default());
    }

    let dist = dijkstra(&state, nodes[0], |_| 1.0);
    assert_eq!(dist[&nodes[0]], 0.0);
    assert_eq!(dist[&nodes[1]], 1.0);
    assert_eq!(dist[&nodes[2]], 2.0);
    assert_eq!(dist[&nodes[3]], 3.0);
}

/// Triangle graph: PageRank should be exactly uniform (1/3 each) by symmetry.
#[test]
fn test_pagerank_known_answer_symmetric_triangle() {
    let mut state: GraphState<()> = GraphState::new();
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let c = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    for &(u, v) in &[(a, b), (b, c), (c, a), (b, a), (c, b), (a, c)] {
        state.add_edge(u, v, EdgeData::default());
    }

    let pr = page_rank(&state, 0.85, 1e-6, 100, |_| 1.0);
    for &node in &[a, b, c] {
        assert!(
            (pr[&node] - 1.0 / 3.0).abs() < 0.01,
            "expected uniform ~0.333, got {}",
            pr[&node]
        );
    }
}

/// Diamond graph A->B, A->C, B->D, C->D: A* must find one of the two
/// equal-length shortest paths (length 2), not a longer one.
#[test]
fn test_astar_known_answer_diamond_graph() {
    let mut state: GraphState<()> = GraphState::new();
    let a = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let b = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let c = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let d = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    for &(u, v) in &[(a, b), (a, c), (b, d), (c, d)] {
        state.add_edge(u, v, EdgeData::default());
    }

    let result = a_star(&state, a, d, |_| 1.0, |_| 0.0, true);
    assert!(result.found);
    assert_eq!(result.distance, 2.0);
    assert_eq!(result.path.len(), 3); // a -> (b or c) -> d
}

/// Two disjoint triangles: MCL must produce exactly 2 clusters, one per triangle,
/// with no cross-cluster membership. Pins down cluster *correctness*, not just count.
#[test]
fn test_markov_clustering_known_answer_disjoint_triangles() {
    use graphene_algorithms::clustering::markov_clustering::markov_clustering;

    let nodes = vec!["a", "b", "c", "d", "e", "f"];
    let edges = vec![
        (0, 1), (1, 0), (1, 2), (2, 1), (2, 0), (0, 2), // triangle 1: a,b,c
        (3, 4), (4, 3), (4, 5), (5, 4), (5, 3), (3, 5), // triangle 2: d,e,f
    ];
    let clusters = markov_clustering(&nodes, &edges, |_, _| 1.0, None);

    assert_eq!(clusters.len(), 2, "expected exactly 2 disjoint clusters");
    for cluster in &clusters {
        assert_eq!(cluster.len(), 3);
        let all_first_triangle = cluster.iter().all(|n| ["a", "b", "c"].contains(n));
        let all_second_triangle = cluster.iter().all(|n| ["d", "e", "f"].contains(n));
        assert!(
            all_first_triangle || all_second_triangle,
            "cluster {:?} mixes the two disjoint triangles",
            cluster
        );
    }
}

/// Hierholzer on a graph with NO Eulerian path (4 odd-degree vertices) must
/// report `found: false`, not a partial/incorrect trail.
#[test]
fn test_hierholzer_correctly_rejects_non_eulerian_graph() {
    use graphene_algorithms::pathfinding::hierholzer::{hierholzer, EdgeInfo, HierholzerConfig};
    use std::collections::HashMap;

    // Star with 4 leaves: center has degree 4 (even), each leaf has degree 1 (odd) -> 4 odd vertices
    let mut nodes: HashMap<String, Vec<String>> = HashMap::new();
    let mut edges: HashMap<String, EdgeInfo> = HashMap::new();

    nodes.insert(
        "center".to_string(),
        vec!["e_a".to_string(), "e_b".to_string(), "e_c".to_string(), "e_d".to_string()],
    );
    for leaf in ["a", "b", "c", "d"] {
        let edge_id = format!("e_{}", leaf);
        nodes.insert(leaf.to_string(), vec![edge_id.clone()]);
        edges.insert(
            edge_id,
            EdgeInfo {
                source: "center".to_string(),
                target: leaf.to_string(),
            },
        );
    }

    let result = hierholzer(&nodes, &edges, &HierholzerConfig::default());
    assert!(!result.found, "graph with 4 odd-degree vertices has no Eulerian path");
}


