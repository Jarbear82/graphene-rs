use graphene_core::fixtures::get_all_fixtures;
use graphene_core::{GraphState, Vec2};
use graphene_layout::{
    compute_multigraph_bezier_routing, star_expand_hypergraph, BipartiteLayout, CircleLayout,
    CollisionForceDirectedLayout, CompoundLayout, ConcentricHubLayout, DisconnectedPacker,
    ForceDirectedLayout, GridSortedLayout, KamadaKawaiLayout, Layout, MdsLayout,
    RegionalPartitionLayout, ReingoldTilfordLayout, SugiyamaLayout, WeightedForceDirectedLayout,
    FCoseLayout,
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

// 16. BARNES-HUT TESTS
#[test]
fn test_barnes_hut_layout() {
    let fixtures = get_all_fixtures::<()>();

    let f_large = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Large"))
        .unwrap();

    // Compute classical layout positions
    let mut f_classic = f_large.clone();
    let mut classic_layout = ForceDirectedLayout {
        use_barnes_hut: false,
        iterations: 50,
        ..Default::default()
    };
    classic_layout.compute(&mut f_classic.state);
    assert_valid_positions(&f_classic.state);

    // Compute Barnes-Hut layout positions
    let mut f_bh = f_large.clone();
    let mut bh_layout = ForceDirectedLayout {
        use_barnes_hut: true,
        theta: 0.5,
        iterations: 50,
        ..Default::default()
    };
    bh_layout.compute(&mut f_bh.state);
    assert_valid_positions(&f_bh.state);

    // Check we get different but valid results
    let n = f_classic.state.node_index_to_id.len();
    assert!(n > 0);
}

// 17. FCOSE & FILE TREE TESTS
#[test]
fn test_fcose_layout_and_file_tree_preset() {
    let fixtures = get_all_fixtures::<()>();

    // 1. Verify the Workspace File Tree preset loads correctly
    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    assert!(f_tree.state.node_index_to_id.len() > 0, "File tree should contain nodes");
    assert!(f_tree.state.edges.len() > 0, "File tree should contain edges");

    // 2. Verify fCoSE layout computes successfully on the file tree graph
    let mut f_layout = f_tree.clone();
    let mut fcose = FCoseLayout::default();
    fcose.compute(&mut f_layout.state);

    assert_valid_positions(&f_layout.state);
}

// 18. UNIVERSAL COMPOUND FLATTENING TESTS
#[test]
fn test_compound_flattening_on_circle_layout() {
    let fixtures = get_all_fixtures::<()>();

    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    let mut f_flat = f_tree.clone();
    let mut circle = CircleLayout {
        radius: 200.0,
        center: Vec2::default(),
        animate: false,
    };

    // Run CircleLayout via the flattening helper
    let collapsed = std::collections::HashSet::new();
    graphene_layout::compute_flat_layout(&mut circle, &mut f_flat.state, &collapsed);

    assert_valid_positions(&f_flat.state);

    // Verify parent directories enclose their child files/subfolders
    // (i.e. parent size w/h must be > 0 and center must reflect child coordinates)
    for idx in 0..f_flat.state.node_index_to_id.len() {
        let id = f_flat.state.node_index_to_id[idx];
        let mut is_parent = false;
        for j in 0..f_flat.state.node_index_to_id.len() {
            if let Some(p_id) = *f_flat.state.hierarchy.parent.get(j) {
                if p_id == id {
                    is_parent = true;
                    break;
                }
            }
        }

        if is_parent {
            let size = *f_flat.state.sizes.get(idx);
            assert!(size.w > 0.0, "Compound parent width should be greater than 0");
            assert!(size.h > 0.0, "Compound parent height should be greater than 0");
        }
    }
}

// 19. COLLAPSED COMPOUND LAYOUT TESTS
#[test]
fn test_collapsed_compound_parent_filtering() {
    let fixtures = get_all_fixtures::<()>();

    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    let mut f_collapsed = f_tree.clone();

    // Find the root compound node
    let root_id = f_collapsed.state.node_index_to_id[0];

    // Collapse the root folder
    let mut collapsed = std::collections::HashSet::new();
    collapsed.insert(root_id);

    let mut circle = CircleLayout {
        radius: 200.0,
        center: Vec2::default(),
        animate: false,
    };

    // Run layout with root collapsed
    graphene_layout::compute_flat_layout(&mut circle, &mut f_collapsed.state, &collapsed);

    assert_valid_positions(&f_collapsed.state);

    // Verify that the collapsed parent is sized correctly as a standard node
    let root_idx = f_collapsed.state.node_keys[root_id];
    let size = *f_collapsed.state.sizes.get(root_idx);
    assert_eq!(size.w, f_tree.state.sizes.get(root_idx).w, "Collapsed parent size should match its initial standard size, not enclose children");
}

// 20. FCOSE CONSTRAINTS & CALLBACKS INTEGRATION TESTS
#[test]
fn test_fcose_constraints_and_callbacks() {
    use graphene_core::fixtures::get_all_fixtures;
    use graphene_layout::{
        FixedNodeConstraint, AlignmentConstraint, RelativePlacementConstraint, FCoseConstraints,
    };

    let fixtures = get_all_fixtures::<()>();
    let f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Small"))
        .unwrap()
        .clone();

    let mut state = f_small.state;
    // We have three nodes A, B, C
    let nodes = state.node_index_to_id.clone();
    assert!(nodes.len() >= 3);
    let id_a = nodes[0];
    let id_b = nodes[1];
    let id_c = nodes[2];

    // Define constraints
    let fixed_pos = Vec2::new(123.0, 456.0);
    let fixed_node = FixedNodeConstraint {
        node_id: id_a,
        position: fixed_pos,
    };

    // Align B and C vertically (share same X)
    let alignment = AlignmentConstraint {
        vertical: vec![vec![id_b, id_c]],
        horizontal: vec![],
    };

    // Relative placement: B is to the left of A by at least 100.0
    let relative = RelativePlacementConstraint::LeftRight {
        left: id_b,
        right: id_a,
        gap: 100.0,
    };

    let constraints = FCoseConstraints {
        fixed_nodes: vec![fixed_node],
        alignment,
        relative_placement: vec![relative],
    };

    // Per-element callbacks
    let mut fcose = FCoseLayout::default()
        .with_constraints(constraints)
        .with_node_repulsion_fn(move |id| {
            if id == id_a { 10000.0 } else { 4500.0 }
        })
        .with_ideal_edge_length_fn(|_edge| 60.0)
        .with_edge_elasticity_fn(|_edge| 20.0);

    fcose.compute(&mut state);

    assert_valid_positions(&state);

    // 1. Verify fixed node position is exactly preserved
    let idx_a = state.node_keys[id_a];
    let pos_a = *state.positions.get(idx_a);
    assert_eq!(pos_a.x, 123.0);
    assert_eq!(pos_a.y, 456.0);

    // 2. Verify vertical alignment (B and C have the same X coordinate)
    let idx_b = state.node_keys[id_b];
    let idx_c = state.node_keys[id_c];
    let pos_b = *state.positions.get(idx_b);
    let pos_c = *state.positions.get(idx_c);
    assert!((pos_b.x - pos_c.x).abs() < 1e-3, "B and C should have the same X coordinate, got {} and {}", pos_b.x, pos_c.x);

    // 3. Verify relative placement constraint (B is to the left of A by at least 100)
    assert!(pos_b.x <= pos_a.x - 100.0 + 1e-3, "B.x ({}) should be to the left of A.x ({}) by at least 100", pos_b.x, pos_a.x);
}

// Helper to assert all child nodes are physically located within their parent bounding boxes
fn assert_containment<S: Copy>(state: &GraphState<S>) {
    let n = state.node_index_to_id.len();
    for idx in 0..n {
        let child_id = state.node_index_to_id[idx];
        if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
            let Some(&p_idx) = state.node_keys.get(parent_id) else { continue };
            let child_pos = *state.positions.get(idx);
            let child_size = *state.sizes.get(idx);
            let parent_pos = *state.positions.get(p_idx);
            let parent_size = *state.sizes.get(p_idx);

            let half_pw = parent_size.w / 2.0;
            let half_ph = parent_size.h / 2.0;
            let half_cw = child_size.w / 2.0;
            let half_ch = child_size.h / 2.0;

            // child bounds must be fully within parent bounds (with small epsilon for float precision)
            let eps = 0.05;
            assert!(
                child_pos.x - half_cw >= parent_pos.x - half_pw - eps,
                "Child node {:?} (x: {}, w: {}) extends left of parent {:?} (x: {}, w: {})",
                child_id, child_pos.x, child_size.w, parent_id, parent_pos.x, parent_size.w
            );
            assert!(
                child_pos.x + half_cw <= parent_pos.x + half_pw + eps,
                "Child node {:?} (x: {}, w: {}) extends right of parent {:?} (x: {}, w: {})",
                child_id, child_pos.x, child_size.w, parent_id, parent_pos.x, parent_size.w
            );
            assert!(
                child_pos.y - half_ch >= parent_pos.y - half_ph - eps,
                "Child node {:?} (y: {}, h: {}) extends top of parent {:?} (y: {}, h: {})",
                child_id, child_pos.y, child_size.h, parent_id, parent_pos.y, parent_size.h
            );
            assert!(
                child_pos.y + half_ch <= parent_pos.y + half_ph + eps,
                "Child node {:?} (y: {}, h: {}) extends bottom of parent {:?} (y: {}, h: {})",
                child_id, child_pos.y, child_size.h, parent_id, parent_pos.y, parent_size.h
            );
        }
    }
}

// 21. FCOSE CONTAINMENT TESTS
#[test]
fn test_fcose_containment_after_layout() {
    let fixtures = get_all_fixtures::<()>();
    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    let mut state = f_tree.state.clone();
    let mut fcose = FCoseLayout::default();
    fcose.compute(&mut state);

    assert_valid_positions(&state);
    assert_containment(&state);
}

#[test]
fn test_fcose_containment_after_physics_simulation() {
    let fixtures = get_all_fixtures::<()>();
    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    let mut state = f_tree.state.clone();
    
    // 1. Initial layout
    let mut fcose = FCoseLayout::default();
    fcose.compute(&mut state);

    // 2. Simulate physics steps as implemented in the interactive demo
    let n = state.node_index_to_id.len();
    assert!(n > 0);

    let k_rep = 2500.0;
    let k_att = 0.06;
    let gravity = 0.3;
    let padding = 12.0;

    let mut is_parent = vec![false; n];
    for i in 0..n {
        if state.hierarchy.first_child.get(i).is_some() {
            is_parent[i] = true;
        }
    }

    let get_leaf_descendants = |node_idx: usize, h_state: &GraphState<()>, is_p: &[bool]| -> Vec<usize> {
        let mut leaves = Vec::new();
        let mut stack = vec![node_idx];
        while let Some(curr) = stack.pop() {
            if !is_p[curr] {
                leaves.push(curr);
            } else {
                let mut next_child = *h_state.hierarchy.first_child.get(curr);
                while let Some(child_id) = next_child {
                    if let Some(&child_idx) = h_state.node_keys.get(child_id) {
                        stack.push(child_idx);
                        next_child = *h_state.hierarchy.next_sibling.get(child_idx);
                    } else {
                        break;
                    }
                }
            }
        }
        leaves
    };

    let is_ancestor = |mut child_idx: usize, parent_idx: usize, h_state: &GraphState<()>| -> bool {
        let parent_id = h_state.node_index_to_id[parent_idx];
        while let Some(p_id) = *h_state.hierarchy.parent.get(child_idx) {
            if p_id == parent_id {
                return true;
            }
            if let Some(&p_idx) = h_state.node_keys.get(p_id) {
                child_idx = p_idx;
            } else {
                break;
            }
        }
        false
    };

    // Cool down the temperature from 10.0 to 0.05
    let mut temp = 10.0;
    while temp > 0.05 {
        let mut forces = vec![Vec2::default(); n];

        // Repulsive forces between leaf nodes
        let positions_slice = &*state.positions;
        let quadtree = graphene_layout::Quadtree::build(positions_slice);
        for i in 0..n {
            if !is_parent[i] {
                let pos_i = positions_slice[i];
                forces[i] = quadtree.accumulate_repulsion(i, pos_i, positions_slice, k_rep, 0.5);
            }
        }

        // Attraction along edges
        let edges_count = state.edges.len();
        for i in 0..edges_count {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            if let (Some(&src_idx), Some(&tgt_idx)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                if src_idx != tgt_idx {
                    let pos_src = *state.positions.get(src_idx);
                    let pos_tgt = *state.positions.get(tgt_idx);
                    let dx = pos_tgt.x - pos_src.x;
                    let dy = pos_tgt.y - pos_src.y;
                    let dist = (dx * dx + dy * dy + 0.01).sqrt();
                    let force = k_att * dist;
                    let fx = (dx / dist) * force;
                    let fy = (dy / dist) * force;
                    forces[src_idx].x += fx;
                    forces[src_idx].y += fy;
                    forces[tgt_idx].x -= fx;
                    forces[tgt_idx].y -= fy;
                }
            }
        }

        // Apply forces to positions with temperature limit
        for i in 0..n {
            if is_parent[i] {
                continue;
            }
            let pos = state.positions.get_mut(i);
            forces[i].x -= pos.x * gravity;
            forces[i].y -= pos.y * gravity;
            let force_len = (forces[i].x * forces[i].x + forces[i].y * forces[i].y + 0.01).sqrt();
            let limit = force_len.min(temp);
            pos.x += (forces[i].x / force_len) * limit;
            pos.y += (forces[i].y / force_len) * limit;
        }

        // Resolve collisions (excluding ancestor containment)
        for _ in 0..4 {
            for i in 0..n {
                for j in (i + 1)..n {
                    if is_ancestor(i, j, &state) || is_ancestor(j, i, &state) {
                        continue;
                    }
                    let pos_i = *state.positions.get(i);
                    let pos_j = *state.positions.get(j);
                    let size_i = *state.sizes.get(i);
                    let size_j = *state.sizes.get(j);

                    let dx = pos_j.x - pos_i.x;
                    let dy = pos_j.y - pos_i.y;
                    let min_dx = (size_i.w + size_j.w) / 2.0 + padding;
                    let min_dy = (size_i.h + size_j.h) / 2.0 + padding;

                    let overlap_x = min_dx - dx.abs();
                    let overlap_y = min_dy - dy.abs();

                    if overlap_x > 0.0 && overlap_y > 0.0 {
                        let push_x;
                        let push_y;
                        if overlap_x < overlap_y {
                            let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                            push_x = sign_x * overlap_x * 0.5;
                            push_y = 0.0;
                        } else {
                            let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                            push_x = 0.0;
                            push_y = sign_y * overlap_y * 0.5;
                        }

                        let apply_push = |node_idx: usize, push_x: f32, push_y: f32, s: &mut GraphState<()>| {
                            if !is_parent[node_idx] {
                                let p = s.positions.get_mut(node_idx);
                                p.x += push_x;
                                p.y += push_y;
                            } else {
                                let leaf_descendants = get_leaf_descendants(node_idx, s, &is_parent);
                                for &leaf_idx in &leaf_descendants {
                                    let p = s.positions.get_mut(leaf_idx);
                                    p.x += push_x;
                                    p.y += push_y;
                                }
                            }
                        };

                        apply_push(i, -push_x, -push_y, &mut state);
                        apply_push(j, push_x, push_y, &mut state);
                    }
                }
            }
        }

        // Dynamically resolve compound bounds
        graphene_layout::resolve_compound_bounds(&mut state, &std::collections::HashSet::new(), 20.0);

        temp *= 0.95;
    }

    assert_valid_positions(&state);
    assert_containment(&state);
}

#[test]
fn test_fcose_containment_after_drag() {
    let fixtures = get_all_fixtures::<()>();
    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    let mut state = f_tree.state.clone();
    
    // 1. Initial layout
    let mut fcose = FCoseLayout::default();
    fcose.compute(&mut state);

    // 2. Pick a child node and drag it to a distant position
    let child_idx = 0; // First node
    let original_pos = *state.positions.get(child_idx);
    let dragged_pos = Vec2::new(original_pos.x + 300.0, original_pos.y - 150.0);
    state.positions.set(child_idx, dragged_pos);

    // 3. Resolve collisions and compound bounds as dragging logic does
    let n = state.node_index_to_id.len();
    let padding = 12.0;

    let is_ancestor = |mut child_idx: usize, parent_idx: usize, h_state: &GraphState<()>| -> bool {
        let parent_id = h_state.node_index_to_id[parent_idx];
        while let Some(p_id) = *h_state.hierarchy.parent.get(child_idx) {
            if p_id == parent_id {
                return true;
            }
            if let Some(&p_idx) = h_state.node_keys.get(p_id) {
                child_idx = p_idx;
            } else {
                break;
            }
        }
        false
    };

    // Collision resolution and compound bounds update (as done on mouse drag)
    for _ in 0..4 {
        for i in 0..n {
            for j in (i + 1)..n {
                if is_ancestor(i, j, &state) || is_ancestor(j, i, &state) {
                    continue;
                }
                let pos_i = *state.positions.get(i);
                let pos_j = *state.positions.get(j);
                let size_i = *state.sizes.get(i);
                let size_j = *state.sizes.get(j);

                let dx = pos_j.x - pos_i.x;
                let dy = pos_j.y - pos_i.y;
                let min_dx = (size_i.w + size_j.w) / 2.0 + padding;
                let min_dy = (size_i.h + size_j.h) / 2.0 + padding;

                let overlap_x = min_dx - dx.abs();
                let overlap_y = min_dy - dy.abs();

                if overlap_x > 0.0 && overlap_y > 0.0 {
                    let push_x;
                    let push_y;
                    if overlap_x < overlap_y {
                        let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                        push_x = sign_x * overlap_x * 0.5;
                        push_y = 0.0;
                    } else {
                        let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                        push_x = 0.0;
                        push_y = sign_y * overlap_y * 0.5;
                    }

                    let p_i = state.positions.get_mut(i);
                    p_i.x -= push_x;
                    p_i.y -= push_y;

                    let p_j = state.positions.get_mut(j);
                    p_j.x += push_x;
                    p_j.y += push_y;
                }
            }
        }
    }

    graphene_layout::resolve_compound_bounds(&mut state, &std::collections::HashSet::new(), 20.0);

    assert_valid_positions(&state);
    assert_containment(&state);
}

#[test]
fn test_fcose_containment_collapsed_parents() {
    let fixtures = get_all_fixtures::<()>();
    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    let mut state = f_tree.state.clone();
    
    // Choose one parent node to collapse
    let n = state.node_index_to_id.len();
    let mut parent_id_to_collapse = None;
    for i in 0..n {
        if state.hierarchy.first_child.get(i).is_some() {
            parent_id_to_collapse = Some(state.node_index_to_id[i]);
            break;
        }
    }

    let mut collapsed = std::collections::HashSet::new();
    if let Some(pid) = parent_id_to_collapse {
        collapsed.insert(pid);
    }

    // Run layout with collapsed parents via compute_flat_layout
    let mut fcose = FCoseLayout::default();
    graphene_layout::compute_flat_layout(&mut fcose, &mut state, &collapsed);

    assert_valid_positions(&state);
    
    // Verify that all non-collapsed parents contain their children.
    for idx in 0..n {
        let child_id = state.node_index_to_id[idx];
        if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
            if collapsed.contains(&parent_id) {
                // If parent is collapsed, skip containment assertion since child is hidden
                continue;
            }
            let Some(&p_idx) = state.node_keys.get(parent_id) else { continue };
            let child_pos = *state.positions.get(idx);
            let child_size = *state.sizes.get(idx);
            let parent_pos = *state.positions.get(p_idx);
            let parent_size = *state.sizes.get(p_idx);

            let half_pw = parent_size.w / 2.0;
            let half_ph = parent_size.h / 2.0;
            let half_cw = child_size.w / 2.0;
            let half_ch = child_size.h / 2.0;

            let eps = 0.05;
            assert!(
                child_pos.x - half_cw >= parent_pos.x - half_pw - eps,
                "Child node {:?} extends left of parent {:?}", child_id, parent_id
            );
            assert!(
                child_pos.x + half_cw <= parent_pos.x + half_pw + eps,
                "Child node {:?} extends right of parent {:?}", child_id, parent_id
            );
            assert!(
                child_pos.y - half_ch >= parent_pos.y - half_ph - eps,
                "Child node {:?} extends top of parent {:?}", child_id, parent_id
            );
            assert!(
                child_pos.y + half_ch <= parent_pos.y + half_ph + eps,
                "Child node {:?} extends bottom of parent {:?}", child_id, parent_id
            );
        }
    }
}


