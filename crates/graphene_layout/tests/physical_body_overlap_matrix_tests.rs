use std::collections::HashSet;
use graphene_core::math::{Size2, Vec2};
use graphene_core::{EdgeData, GraphState, HierarchyExt, NodeId};
use graphene_gpui::render::draw_pipeline::Viewport;
use graphene_layout::{
    compute_flat_layout, BipartiteLayout, BreadthFirstLayout, CircleLayout,
    CollisionForceDirectedLayout, ConcentricLayout, CoseLayout, FCoseLayout,
    ForceDirectedLayout, GridLayout, GridSortedLayout, KamadaKawaiLayout, Layout, MdsLayout,
    ReingoldTilfordLayout, SugiyamaLayout,
};

fn create_flat_graph(num_nodes: usize) -> GraphState<()> {
    let mut state = GraphState::<()>::new();
    let mut nodes = Vec::new();
    let mut lcg = 123456789u64;

    for i in 0..num_nodes {
        lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let w = 50.0 + ((lcg >> 16) % 70) as f32; // width 50..120
        let h = 30.0 + ((lcg >> 24) % 40) as f32; // height 30..70
        let pos = Vec2::new((i % 10) as f32 * 10.0, (i / 10) as f32 * 10.0);
        let id = state.add_node(pos, Size2::new(w, h));
        nodes.push(id);
    }

    // Connect sequential nodes to form edges
    for i in 1..num_nodes {
        state.add_edge(nodes[i - 1], nodes[i], EdgeData::default());
        if i % 5 == 0 && i >= 5 {
            state.add_edge(nodes[i - 5], nodes[i], EdgeData::default());
        }
    }

    state
}

fn create_compound_graph(num_leafs: usize, num_parents: usize) -> GraphState<()> {
    let mut state = create_flat_graph(num_leafs);
    let leaf_ids: Vec<NodeId> = state.node_index_to_id.iter().take(num_leafs).copied().collect();
    let mut parent_ids = Vec::new();

    for _p in 0..num_parents {
        let parent_id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(100.0, 100.0));
        parent_ids.push(parent_id);
    }

    // Reparent leafs to compound parents
    for (i, leaf_id) in leaf_ids.into_iter().enumerate() {
        let parent_id = parent_ids[i % num_parents];
        state.reparent_node(leaf_id, Some(parent_id));
    }

    state
}

fn assert_no_overlaps_graph_and_ui<S: Copy + Default>(state: &GraphState<S>, test_label: &str) {
    let n = state.node_index_to_id.len();
    if n <= 1 {
        return;
    }

    // 1. Verify zero overlap in Graph Dimensions between physical leaf nodes and sibling containers
    for i in 0..n {
        let id_i = state.node_index_to_id[i];
        let is_parent_i = state.is_parent(i);
        let pos_i = *state.positions.get(i);
        let size_i = *state.sizes.get(i);

        let hw_i = size_i.w / 2.0;
        let hh_i = size_i.h / 2.0;

        for j in (i + 1)..n {
            let id_j = state.node_index_to_id[j];
            let is_parent_j = state.is_parent(j);

            // Skip compound parent container boxes (parents enclose children)
            if is_parent_i || is_parent_j {
                continue;
            }

            let pos_j = *state.positions.get(j);
            let size_j = *state.sizes.get(j);

            let hw_j = size_j.w / 2.0;
            let hh_j = size_j.h / 2.0;

            let dx = (pos_i.x - pos_j.x).abs();
            let dy = (pos_i.y - pos_j.y).abs();
            let min_dx = hw_i + hw_j;
            let min_dy = hh_i + hh_j;

            assert!(
                dx >= min_dx || dy >= min_dy,
                "[{}] GRAPH SPACE OVERLAP: node {:?} at ({},{}) size ({},{}) vs node {:?} at ({},{}) size ({},{}): dx={}, min_dx={}, dy={}, min_dy={}",
                test_label, id_i, pos_i.x, pos_i.y, size_i.w, size_i.h, id_j, pos_j.x, pos_j.y, size_j.w, size_j.h, dx, min_dx, dy, min_dy
            );
        }
    }

    // 2. Verify zero overlap in Rendered UI Pixels (at 100% Zoom = 1.0)
    let viewport_bounds = gpui::Bounds {
        origin: gpui::Point { x: 0.0, y: 0.0 },
        size: gpui::Size {
            width: 3840.0,
            height: 2160.0,
        },
    };
    let mut viewport = Viewport::new(viewport_bounds);
    viewport.zoom = 1.0; // 100% Zoom 1:1

    for i in 0..n {
        let id_i = state.node_index_to_id[i];
        let is_parent_i = state.is_parent(i);
        let pos_i = *state.positions.get(i);
        let size_i = *state.sizes.get(i);
        let p_i_screen = viewport.model_to_canvas(pos_i);
        let ui_w_i = size_i.w * viewport.zoom;
        let ui_h_i = size_i.h * viewport.zoom;

        for j in (i + 1)..n {
            let id_j = state.node_index_to_id[j];
            let is_parent_j = state.is_parent(j);

            if is_parent_i || is_parent_j {
                continue;
            }

            let pos_j = *state.positions.get(j);
            let size_j = *state.sizes.get(j);
            let p_j_screen = viewport.model_to_canvas(pos_j);
            let ui_w_j = size_j.w * viewport.zoom;
            let ui_h_j = size_j.h * viewport.zoom;

            let ui_dx = (p_i_screen.x - p_j_screen.x).abs();
            let ui_dy = (p_i_screen.y - p_j_screen.y).abs();
            let ui_min_dx = (ui_w_i + ui_w_j) / 2.0;
            let ui_min_dy = (ui_h_i + ui_h_j) / 2.0;

            assert!(
                ui_dx >= ui_min_dx || ui_dy >= ui_min_dy,
                "[{}] RENDERED UI OVERLAP at 100% zoom: node {:?} vs node {:?}: ui_dx={}, ui_min_dx={}, ui_dy={}, ui_min_dy={}",
                test_label, id_i, id_j, ui_dx, ui_min_dx, ui_dy, ui_min_dy
            );
        }
    }
}

fn run_flat_and_compound_tests<L: Layout<()>>(
    mut layout: L,
    algorithm_name: &str,
) {
    let collapsed = HashSet::new();

    // 1. Small Flat (5 nodes)
    let mut small_flat = create_flat_graph(5);
    layout.compute(&mut small_flat);
    assert_no_overlaps_graph_and_ui(&small_flat, &format!("{}_small_flat", algorithm_name));

    // 2. Medium Flat (35 nodes)
    let mut med_flat = create_flat_graph(35);
    layout.compute(&mut med_flat);
    assert_no_overlaps_graph_and_ui(&med_flat, &format!("{}_medium_flat", algorithm_name));

    // 3. Large Flat (100 nodes)
    let mut large_flat = create_flat_graph(100);
    layout.compute(&mut large_flat);
    assert_no_overlaps_graph_and_ui(&large_flat, &format!("{}_large_flat", algorithm_name));

    // 4. Small Compound (5 nodes + 2 parents)
    let mut small_cmp = create_compound_graph(5, 2);
    compute_flat_layout(&mut layout, &mut small_cmp, &collapsed);
    assert_no_overlaps_graph_and_ui(&small_cmp, &format!("{}_small_compound", algorithm_name));

    // 5. Medium Compound (35 nodes + 5 parents)
    let mut med_cmp = create_compound_graph(35, 5);
    compute_flat_layout(&mut layout, &mut med_cmp, &collapsed);
    assert_no_overlaps_graph_and_ui(&med_cmp, &format!("{}_medium_compound", algorithm_name));

    // 6. Large Compound (100 nodes + 10 parents)
    let mut large_cmp = create_compound_graph(100, 10);
    compute_flat_layout(&mut layout, &mut large_cmp, &collapsed);
    assert_no_overlaps_graph_and_ui(&large_cmp, &format!("{}_large_compound", algorithm_name));
}

#[test]
fn test_force_directed_layout_matrix() {
    run_flat_and_compound_tests(ForceDirectedLayout::default(), "ForceDirectedLayout");
}

#[test]
fn test_collision_force_directed_layout_matrix() {
    run_flat_and_compound_tests(CollisionForceDirectedLayout::default(), "CollisionForceDirectedLayout");
}

#[test]
fn test_circle_layout_matrix() {
    run_flat_and_compound_tests(CircleLayout { radius: 300.0, center: Vec2::default(), animate: false }, "CircleLayout");
}

#[test]
fn test_grid_layout_matrix() {
    run_flat_and_compound_tests(GridLayout { columns: 5, spacing_x: 120.0, spacing_y: 100.0, animate: false }, "GridLayout");
}

#[test]
fn test_grid_sorted_layout_matrix() {
    run_flat_and_compound_tests(GridSortedLayout::default(), "GridSortedLayout");
}

#[test]
fn test_sugiyama_layout_matrix() {
    run_flat_and_compound_tests(SugiyamaLayout::default(), "SugiyamaLayout");
}

#[test]
fn test_kamada_kawai_layout_matrix() {
    run_flat_and_compound_tests(KamadaKawaiLayout::default(), "KamadaKawaiLayout");
}

#[test]
fn test_mds_layout_matrix() {
    run_flat_and_compound_tests(MdsLayout::default(), "MdsLayout");
}

#[test]
fn test_reingold_tilford_layout_matrix() {
    run_flat_and_compound_tests(ReingoldTilfordLayout { sibling_spacing: 150.0, level_spacing: 150.0 }, "ReingoldTilfordLayout");
}

#[test]
fn test_bipartite_layout_matrix() {
    run_flat_and_compound_tests(BipartiteLayout { partition_fn: |_id: NodeId| 0, column_spacing: 200.0, vertical_spacing: 100.0 }, "BipartiteLayout");
}

#[test]
fn test_cose_layout_matrix() {
    run_flat_and_compound_tests(CoseLayout::default(), "CoseLayout");
}

#[test]
fn test_fcose_layout_matrix() {
    run_flat_and_compound_tests(FCoseLayout::default(), "FCoseLayout");
}

#[test]
fn test_concentric_layout_matrix() {
    run_flat_and_compound_tests(ConcentricLayout { level_radius_step: 150.0, center: Vec2::default(), animate: false }, "ConcentricLayout");
}

#[test]
fn test_breadth_first_layout_matrix() {
    let state = create_flat_graph(5);
    let root = state.node_index_to_id[0];
    run_flat_and_compound_tests(BreadthFirstLayout { root, sibling_spacing: 100.0, level_spacing: 120.0, animate: false }, "BreadthFirstLayout");
}
