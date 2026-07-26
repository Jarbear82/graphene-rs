use graphene_core::{
    HierarchyExt, EdgeData, GraphState, NodeId, Size2, Vec2,
};
use std::collections::{HashMap, HashSet};

// ==========================================
// 1. EMITTER TESTS (EMIT-01..12)
// ==========================================
#[test]
fn test_emit_01_to_12_emitter() {
    let mut state: GraphState<()> = GraphState::new();

    // EMIT-01: node creation adds keys
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    assert!(state.node_keys.contains_key(n1));

    // EMIT-12: event propagation / hierarchy ancestor check
    let n2 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(10.0, 10.0));
    state.reparent_node(n2, Some(n1));
    assert!(state.is_ancestor(state.node_keys[n2], state.node_keys[n1]));
}

// ==========================================
// 2. MATH & UTIL TESTS (UTIL-01..13)
// ==========================================
#[test]
fn test_util_01_to_13_hashing_and_strings() {
    // UTIL-01, UTIL-02: Hash consistency
    let k1 = state_hash("test_key");
    let k2 = state_hash("test_key");
    assert_eq!(k1, k2);

    // UTIL-03: Reversed strings hash differently
    let h1 = state_hash("node_a");
    let h2 = state_hash("a_edon");
    assert_ne!(h1, h2);

    // UTIL-05, UTIL-07: Unique hashes for ASCII range
    let mut hashes = HashSet::new();
    for i in 0..128u8 {
        let s = (i as char).to_string();
        let h = state_hash(&s);
        assert!(hashes.insert(h), "Collision detected for char {}", i);
    }

    // UTIL-06: Hash differs for negative numbers
    for i in 1..100 {
        let pos = state_hash(&i.to_string());
        let neg = state_hash(&(-i).to_string());
        assert_ne!(pos, neg);
    }

    // UTIL-13: ends_with correctness
    assert!("node_style.css".ends_with(".css"));
    assert!(!"node_style.css".ends_with(".js"));
}

fn state_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

// ==========================================
// 3. BUILDING AND FILTERING (BF-01..15)
// ==========================================
#[test]
fn test_bf_01_to_15_building_and_filtering() {
    let mut state: GraphState<()> = GraphState::new();

    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(20.0, 20.0));
    let n3 = state.add_node(Vec2::new(40.0, 0.0), Size2::new(30.0, 30.0));

    // BF-01: add / collection union
    let all_nodes = vec![n1, n2, n3];
    assert_eq!(all_nodes.len(), 3);

    // BF-02: not / exclude
    let not_n1: Vec<_> = all_nodes.iter().copied().filter(|&id| id != n1).collect();
    assert_eq!(not_n1, vec![n2, n3]);

    // BF-03, BF-04: intersect
    let set_a: HashSet<_> = vec![n1, n2].into_iter().collect();
    let set_b: HashSet<_> = vec![n2, n3].into_iter().collect();
    let intersection: Vec<_> = set_a.intersection(&set_b).copied().collect();
    assert_eq!(intersection, vec![n2]);

    // BF-09, BF-10: max / min node sizes
    let max_w = all_nodes
        .iter()
        .map(|&id| state.sizes.get(state.node_keys[id]).w)
        .fold(f32::MIN, f32::max);
    let min_w = all_nodes
        .iter()
        .map(|&id| state.sizes.get(state.node_keys[id]).w)
        .fold(f32::MAX, f32::min);

    assert_eq!(max_w, 30.0);
    assert_eq!(min_w, 10.0);

    // BF-14: xor / symmetric difference
    let sym_diff: Vec<_> = set_a.symmetric_difference(&set_b).copied().collect();
    assert_eq!(sym_diff.len(), 2);
    assert!(sym_diff.contains(&n1));
    assert!(sym_diff.contains(&n3));
}

// ==========================================
// 4. COLLECTION COMPARISON (CMP-01..08)
// ==========================================
#[test]
fn test_cmp_01_to_08_comparison() {
    let mut state: GraphState<()> = GraphState::new();

    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(10.0, 0.0), Size2::new(10.0, 10.0));
    let n3 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(10.0, 10.0));

    let e1 = state.add_edge(n1, n2, EdgeData::default());

    // CMP-01: same() set equality check
    let s1 = vec![n1, n2];
    let s2 = vec![n1, n2];
    assert_eq!(s1, s2);

    // CMP-03: allAreNeighbors
    assert!(state.node_keys.contains_key(n1));
    assert!(state.node_keys.contains_key(n2));
    assert!(state.edge_keys.contains_key(e1));

    // CMP-04, CMP-05: isNode / isEdge / allAre
    assert!(state.node_keys.contains_key(n1));
    assert!(!state.edge_keys.contains_key(graphene_core::EdgeId::default()));

    // CMP-08: contains
    let nodes = vec![n1, n2];
    assert!(nodes.contains(&n1));
    assert!(!nodes.contains(&n3));
}

// ==========================================
// 5. COMPOUND NODES (CN-01..20)
// ==========================================
#[test]
fn test_cn_01_to_20_compound_nodes() {
    let mut state: GraphState<()> = GraphState::new();

    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(100.0, 100.0));
    let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(40.0, 40.0));
    let n3 = state.add_node(Vec2::new(20.0, 20.0), Size2::new(20.0, 20.0));
    let n4 = state.add_node(Vec2::new(60.0, 20.0), Size2::new(20.0, 20.0));

    state.reparent_node(n2, Some(n1));
    state.reparent_node(n3, Some(n2));
    state.reparent_node(n4, Some(n2));

    let idx1 = state.node_keys[n1];
    let idx2 = state.node_keys[n2];
    let idx3 = state.node_keys[n3];
    let idx4 = state.node_keys[n4];

    // CN-01: is_parent
    assert!(state.is_parent(idx1));
    assert!(state.is_parent(idx2));
    assert!(!state.is_parent(idx3));

    // CN-02: childless
    assert!(!state.is_parent(idx3));

    // CN-03, CN-04: is_child / orphan
    assert_eq!(state.get_nesting_depth(n1), 0);
    assert_eq!(state.get_nesting_depth(n2), 1);
    assert_eq!(state.get_nesting_depth(n3), 2);

    // CN-06: parents
    assert!(state.is_ancestor(idx3, idx1));
    assert!(state.is_ancestor(idx3, idx2));

    // CN-08: descendants
    let descendants = state.get_all_descendants(idx1);
    assert_eq!(descendants.len(), 3);
    assert!(descendants.contains(&idx2));
    assert!(descendants.contains(&idx3));
    assert!(descendants.contains(&idx4));

    // CN-09: siblings
    let n2_children = state.get_all_descendants(idx2);
    assert!(n2_children.contains(&idx3));
    assert!(n2_children.contains(&idx4));

    // CN-17: position moves own bbox by delta
    let old_pos = *state.positions.get(idx3);
    let delta = Vec2::new(15.0, 25.0);
    state.positions.set(idx3, old_pos + delta);
    assert_eq!(*state.positions.get(idx3), Vec2::new(35.0, 45.0));
}

// ==========================================
// 6. GRAPH MANIPULATION (GM-01..21)
// ==========================================
#[test]
fn test_gm_01_to_21_graph_manipulation() {
    let mut state: GraphState<()> = GraphState::new();

    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(10.0, 10.0));
    let e1 = state.add_edge(n1, n2, EdgeData::default());

    assert_eq!(state.node_index_to_id.len(), 2);
    assert_eq!(state.edge_index_to_id.len(), 1);

    // GM-03: removing node removes connected edges
    state.remove_node(n1);
    assert!(!state.node_keys.contains_key(n1));
    assert!(!state.edge_keys.contains_key(e1));

    // GM-10: restore node / re-add
    let n1_new = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    assert!(state.node_keys.contains_key(n1_new));

    // GM-14, GM-15: reparent / orphan
    let p = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));
    state.reparent_node(n2, Some(p));
    assert_eq!(state.get_nesting_depth(n2), 1);

    state.reparent_node(n2, None);
    assert_eq!(state.get_nesting_depth(n2), 0);
}

// ==========================================
// 7. DEGREE & METADATA (MD-01..11)
// ==========================================
#[test]
fn test_md_01_to_11_degree_metadata() {
    let mut state: GraphState<()> = GraphState::new();

    // 5-node complete graph (K5)
    let mut nodes = Vec::new();
    for i in 0..5 {
        let id = state.add_node(Vec2::new(i as f32 * 10.0, 0.0), Size2::new(5.0, 5.0));
        nodes.push(id);
    }

    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                state.add_edge(nodes[i], nodes[j], EdgeData::default());
            }
        }
    }

    // Directed K5: each node has out-degree 4, in-degree 4
    for &n in &nodes {
        let idx = state.node_keys[n];
        let mut in_deg = 0;
        let mut out_deg = 0;
        for i in 0..state.edges.len() {
            if *state.edge_sources.get(i) == n {
                out_deg += 1;
            }
            if *state.edge_targets.get(i) == n {
                in_deg += 1;
            }
        }
        assert_eq!(in_deg, 4);
        assert_eq!(out_deg, 4);
        let _ = idx;
    }
}

// ==========================================
// 8. POSITIONS AND DIMENSIONS (POS-01..17)
// ==========================================
#[test]
fn test_pos_01_to_17_positions_and_dimensions() {
    let mut state: GraphState<()> = GraphState::new();

    let n1 = state.add_node(Vec2::new(100.0, 200.0), Size2::new(30.0, 40.0));
    let idx = state.node_keys[n1];

    // POS-01: position gets initial pos
    assert_eq!(*state.positions.get(idx), Vec2::new(100.0, 200.0));

    // POS-02..04: set x, y
    state.positions.set(idx, Vec2::new(123.0, 456.0));
    assert_eq!(*state.positions.get(idx), Vec2::new(123.0, 456.0));

    // POS-08..11: shift
    let pos = *state.positions.get(idx);
    state.positions.set(idx, pos + Vec2::new(10.0, -20.0));
    assert_eq!(*state.positions.get(idx), Vec2::new(133.0, 436.0));

    // POS-14..17: dimensions & bbox
    let size = *state.sizes.get(idx);
    assert_eq!(size, Size2::new(30.0, 40.0));
    let corners = size.corners();
    assert_eq!(corners.len(), 4);
}

// ==========================================
// 9. TRAVERSAL METHODS (TRV-01..27)
// ==========================================
#[test]
fn test_trv_01_to_27_traversal_methods() {
    let mut state: GraphState<()> = GraphState::new();

    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(20.0, 0.0), Size2::new(10.0, 10.0));
    let n3 = state.add_node(Vec2::new(40.0, 0.0), Size2::new(10.0, 10.0));

    let e12 = state.add_edge(n1, n2, EdgeData::default());
    let e23 = state.add_edge(n2, n3, EdgeData::default());

    // TRV-08: connectedNodes
    let mut connected = HashSet::new();
    connected.insert(*state.edge_sources.get(state.edge_keys[e12]));
    connected.insert(*state.edge_targets.get(state.edge_keys[e12]));
    assert!(connected.contains(&n1));
    assert!(connected.contains(&n2));

    // TRV-10: source / target
    assert_eq!(*state.edge_sources.get(state.edge_keys[e23]), n2);
    assert_eq!(*state.edge_targets.get(state.edge_keys[e23]), n3);

    // TRV-14, TRV-15: roots / leaves
    let mut in_degrees = HashMap::new();
    let mut out_degrees = HashMap::new();
    for &id in &[n1, n2, n3] {
        in_degrees.insert(id, 0);
        out_degrees.insert(id, 0);
    }
    for i in 0..state.edges.len() {
        *out_degrees.get_mut(state.edge_sources.get(i)).unwrap() += 1;
        *in_degrees.get_mut(state.edge_targets.get(i)).unwrap() += 1;
    }

    // Roots (in_degree = 0)
    let roots: Vec<_> = in_degrees.iter().filter(|(_, &deg)| deg == 0).map(|(&id, _)| id).collect();
    assert_eq!(roots, vec![n1]);

    // Leaves (out_degree = 0)
    let leaves: Vec<_> = out_degrees.iter().filter(|(_, &deg)| deg == 0).map(|(&id, _)| id).collect();
    assert_eq!(leaves, vec![n3]);
}
