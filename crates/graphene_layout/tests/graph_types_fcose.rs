use graphene_core::{GraphState, Vec2};
use graphene_fixtures::get_all_fixtures;
use graphene_layout::{CircleLayout, FCoseLayout, Layout};

fn assert_valid_positions<S: Copy>(state: &GraphState<S>) {
    for i in 0..state.node_index_to_id.len() {
        let pos = *state.positions.get(i);
        assert!(pos.x.is_finite(), "Position X is not finite");
        assert!(pos.y.is_finite(), "Position Y is not finite");
    }
}

fn assert_containment<S: Copy>(state: &GraphState<S>) {
    let n = state.node_index_to_id.len();
    for idx in 0..n {
        let child_id = state.node_index_to_id[idx];
        if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
            let Some(&p_idx) = state.node_keys.get(parent_id) else {
                continue;
            };
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
                "Child node {:?} extends left of parent {:?}",
                child_id,
                parent_id
            );
            assert!(
                child_pos.x + half_cw <= parent_pos.x + half_pw + eps,
                "Child node {:?} extends right of parent {:?}",
                child_id,
                parent_id
            );
            assert!(
                child_pos.y - half_ch >= parent_pos.y - half_ph - eps,
                "Child node {:?} extends top of parent {:?}",
                child_id,
                parent_id
            );
            assert!(
                child_pos.y + half_ch <= parent_pos.y + half_ph + eps,
                "Child node {:?} extends bottom of parent {:?}",
                child_id,
                parent_id
            );
        }
    }
}

// 17. FCOSE & FILE TREE TESTS
#[test]
fn test_fcose_layout_and_file_tree_preset() {
    let fixtures = get_all_fixtures::<()>();

    let f_tree = fixtures
        .iter()
        .find(|f| f.name.contains("Workspace File Tree"))
        .expect("Workspace File Tree preset should exist");

    assert!(
        f_tree.state.node_index_to_id.len() > 0,
        "File tree should contain nodes"
    );
    assert!(
        f_tree.state.edges.len() > 0,
        "File tree should contain edges"
    );

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

    let collapsed = std::collections::HashSet::new();
    graphene_layout::compute_flat_layout(&mut circle, &mut f_flat.state, &collapsed);

    assert_valid_positions(&f_flat.state);

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
            assert!(
                size.w > 0.0,
                "Compound parent width should be greater than 0"
            );
            assert!(
                size.h > 0.0,
                "Compound parent height should be greater than 0"
            );
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
    let root_id = f_collapsed.state.node_index_to_id[0];

    let mut collapsed = std::collections::HashSet::new();
    collapsed.insert(root_id);

    let mut circle = CircleLayout {
        radius: 200.0,
        center: Vec2::default(),
        animate: false,
    };

    graphene_layout::compute_flat_layout(&mut circle, &mut f_collapsed.state, &collapsed);

    assert_valid_positions(&f_collapsed.state);

    let root_idx = f_collapsed.state.node_keys[root_id];
    let size = *f_collapsed.state.sizes.get(root_idx);
    assert_eq!(
        size.w,
        f_tree.state.sizes.get(root_idx).w,
        "Collapsed parent size should match its initial standard size, not enclose children"
    );
}

// 20. FCOSE CONSTRAINTS & CALLBACKS INTEGRATION TESTS
#[test]
fn test_fcose_constraints_and_callbacks() {
    use graphene_layout::{
        AlignmentConstraint, FCoseConstraints, FixedNodeConstraint, RelativePlacementConstraint,
    };

    let fixtures = get_all_fixtures::<()>();
    let f_small = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Small"))
        .unwrap()
        .clone();

    let mut state = f_small.state;
    let nodes = state.node_index_to_id.clone();
    assert!(nodes.len() >= 3);
    let id_a = nodes[0];
    let id_b = nodes[1];
    let id_c = nodes[2];

    let fixed_pos = Vec2::new(123.0, 456.0);
    let fixed_node = FixedNodeConstraint {
        node_id: id_a,
        position: fixed_pos,
    };

    let alignment = AlignmentConstraint {
        vertical: vec![vec![id_b, id_c]],
        horizontal: vec![],
    };

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

    use graphene_layout::fcose::{EdgeMetric, NodeRepulsionMetric};

    let mut fcose = FCoseLayout::default()
        .with_constraints(constraints)
        .with_node_repulsion_metric(NodeRepulsionMetric::NodePinned {
            target_id: id_a,
            pinned_val: 10000.0,
            default_val: 4500.0,
        })
        .with_ideal_edge_length_metric(EdgeMetric::Constant(60.0))
        .with_edge_elasticity_metric(EdgeMetric::Constant(20.0));

    fcose.compute(&mut state);

    assert_valid_positions(&state);

    let idx_a = state.node_keys[id_a];
    let pos_a = *state.positions.get(idx_a);
    assert_eq!(pos_a.x, 123.0);
    assert_eq!(pos_a.y, 456.0);

    let idx_b = state.node_keys[id_b];
    let idx_c = state.node_keys[id_c];
    let pos_b = *state.positions.get(idx_b);
    let pos_c = *state.positions.get(idx_c);
    assert!(
        (pos_b.x - pos_c.x).abs() < 1e-3,
        "B and C should have the same X coordinate, got {} and {}",
        pos_b.x,
        pos_c.x
    );

    assert!(
        pos_b.x <= pos_a.x - 100.0 + 1e-3,
        "B.x ({}) should be to the left of A.x ({}) by at least 100",
        pos_b.x,
        pos_a.x
    );
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

    let mut fcose = FCoseLayout::default();
    fcose.compute(&mut state);

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

    let get_leaf_descendants =
        |node_idx: usize, h_state: &GraphState<()>, is_p: &[bool]| -> Vec<usize> {
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

    let mut temp = 10.0;
    while temp > 0.05 {
        let mut forces = vec![Vec2::default(); n];

        let positions_slice = &*state.positions;
        let quadtree = graphene_layout::Quadtree::build(positions_slice);
        for i in 0..n {
            if !is_parent[i] {
                let pos_i = positions_slice[i];
                forces[i] = quadtree.accumulate_repulsion(i, pos_i, positions_slice, k_rep, 0.5);
            }
        }

        let edges_count = state.edges.len();
        for i in 0..edges_count {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            if let (Some(&src_idx), Some(&tgt_idx)) =
                (state.node_keys.get(src), state.node_keys.get(tgt))
            {
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

                        let apply_push =
                            |node_idx: usize, push_x: f32, push_y: f32, s: &mut GraphState<()>| {
                                if !is_parent[node_idx] {
                                    let p = s.positions.get_mut(node_idx);
                                    p.x += push_x;
                                    p.y += push_y;
                                } else {
                                    let leaf_descendants =
                                        get_leaf_descendants(node_idx, s, &is_parent);
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

        graphene_layout::resolve_compound_bounds(
            &mut state,
            &std::collections::HashSet::new(),
            20.0,
        );

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

    let mut fcose = FCoseLayout::default();
    fcose.compute(&mut state);

    let child_idx = 0;
    let original_pos = *state.positions.get(child_idx);
    let dragged_pos = Vec2::new(original_pos.x + 300.0, original_pos.y - 150.0);
    state.positions.set(child_idx, dragged_pos);

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

    let mut fcose = FCoseLayout::default();
    graphene_layout::compute_flat_layout(&mut fcose, &mut state, &collapsed);

    assert_valid_positions(&state);

    for idx in 0..n {
        let child_id = state.node_index_to_id[idx];
        if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
            if collapsed.contains(&parent_id) {
                continue;
            }
            let Some(&p_idx) = state.node_keys.get(parent_id) else {
                continue;
            };
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
                "Child node {:?} extends left of parent {:?}",
                child_id,
                parent_id
            );
            assert!(
                child_pos.x + half_cw <= parent_pos.x + half_pw + eps,
                "Child node {:?} extends right of parent {:?}",
                child_id,
                parent_id
            );
            assert!(
                child_pos.y - half_ch >= parent_pos.y - half_ph - eps,
                "Child node {:?} extends top of parent {:?}",
                child_id,
                parent_id
            );
            assert!(
                child_pos.y + half_ch <= parent_pos.y + half_ph + eps,
                "Child node {:?} extends bottom of parent {:?}",
                child_id,
                parent_id
            );
        }
    }
}

#[test]
fn test_cose_and_fcose_barnes_hut_large_graph() {
    use graphene_core::{math::Vec2, GraphState, Size2};
    use graphene_layout::cose::CoseLayout;
    use graphene_layout::fcose::FCoseLayout;

    let mut state = GraphState::<()>::default();
    let n = 150;
    for i in 0..n {
        let pos = Vec2::new((i % 15) as f32 * 20.0, (i / 15) as f32 * 20.0);
        let size = Size2::new(10.0, 10.0);
        state.add_node(pos, size);
    }

    for i in 0..(n - 1) {
        let u = state.node_index_to_id[i];
        let v = state.node_index_to_id[i + 1];
        state.add_edge(u, v, graphene_core::EdgeData::default());
    }

    let mut cose = CoseLayout {
        iterations: 10,
        ..Default::default()
    };
    cose.compute(&mut state);
    assert_valid_positions(&state);

    let mut fcose = FCoseLayout {
        iterations: 10,
        ..Default::default()
    };
    fcose.compute(&mut state);
    assert_valid_positions(&state);
}

#[test]
fn test_async_live_simulation_handle_background_thread() {
    use graphene_core::{math::Vec2, GraphState, Size2};
    use graphene_layout::livesim::{AsyncLiveSimulationHandle, LiveForceSimulation};

    let mut state = GraphState::<()>::default();
    let n = 50;
    for i in 0..n {
        let pos = Vec2::new((i % 10) as f32 * 30.0, (i / 10) as f32 * 30.0);
        let size = Size2::new(10.0, 10.0);
        state.add_node(pos, size);
    }

    for i in 0..(n - 1) {
        let u = state.node_index_to_id[i];
        let v = state.node_index_to_id[i + 1];
        state.add_edge(u, v, graphene_core::EdgeData::default());
    }

    let sim = LiveForceSimulation::new();
    let handle = AsyncLiveSimulationHandle::spawn(sim, state.clone(), 20);

    std::thread::sleep(std::time::Duration::from_millis(50));

    let snap = handle.latest_snapshot();
    assert_eq!(snap.positions.len(), n);
    assert!(snap.version > 0);

    handle.apply_to_graph_state(&mut state);
    assert_valid_positions(&state);
}

#[test]
fn test_graph_engine_decoupled_thread_actor() {
    use graphene_core::{math::Vec2, GraphState, Size2};
    use graphene_layout::cose::CoseLayout;
    use graphene_layout::engine::{GraphCommand, GraphEngineHandle, LayoutCommand};

    let initial_state = GraphState::<()>::default();
    let engine = GraphEngineHandle::spawn(initial_state);

    engine
        .send_command(GraphCommand::AddNode {
            pos: Vec2::new(0.0, 0.0),
            size: Size2::new(10.0, 10.0),
            data: (),
            label: None,
        })
        .unwrap();

    engine
        .send_command(GraphCommand::AddNode {
            pos: Vec2::new(50.0, 50.0),
            size: Size2::new(10.0, 10.0),
            data: (),
            label: None,
        })
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(30));

    let snap = engine.latest_snapshot();
    assert_eq!(snap.positions.len(), 2);
    assert!(snap.version > 0);

    engine
        .send_command(GraphCommand::RunLayout(LayoutCommand::Cose(CoseLayout {
            iterations: 10,
            ..Default::default()
        })))
        .unwrap();

    std::thread::sleep(std::time::Duration::from_millis(30));

    engine.shutdown();
}

#[test]
fn test_graph_engine_live_sim_stepping_and_tuning() {
    use graphene_core::{math::Vec2, GraphState, Size2};
    use graphene_layout::engine::{GraphCommand, GraphEngineHandle};
    use graphene_layout::livesim::{LiveForceSimulation, LiveSimParam};

    let mut state = GraphState::<()>::default();
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(100.0, 100.0), Size2::new(10.0, 10.0));
    state.add_edge(n1, n2, graphene_core::EdgeData::default());

    let engine = GraphEngineHandle::spawn(state);

    let sim = LiveForceSimulation::new();
    engine
        .send_command(GraphCommand::StartLiveSim(sim))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(20));

    engine
        .send_command(GraphCommand::UpdateLiveSimParam(LiveSimParam::Repulsion(
            5000.0,
        )))
        .unwrap();
    engine
        .send_command(GraphCommand::StepLiveSimN(5))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));

    let snap = engine.latest_snapshot();
    assert_eq!(snap.positions.len(), 2);
    assert!(snap.version > 0);

    engine.send_command(GraphCommand::StopLiveSim).unwrap();
    engine.shutdown();
}

#[test]
fn test_graph_engine_phase_stepped_layout() {
    use graphene_core::{math::Vec2, GraphState, Size2};
    use graphene_layout::engine::{GraphCommand, GraphEngineHandle, LayoutCommand};
    use graphene_layout::hierarchical::{SugiyamaLayout, SugiyamaPhase};
    use graphene_layout::traits::PhaseSteppableLayout;

    let sugiyama = SugiyamaLayout::default();
    assert_eq!(PhaseSteppableLayout::<()>::phases(&sugiyama).len(), 4);
    assert_eq!(
        PhaseSteppableLayout::<()>::current_phase(&sugiyama),
        Some(SugiyamaPhase::CycleBreaking)
    );

    let mut state = GraphState::<()>::default();
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::new(50.0, 50.0), Size2::new(10.0, 10.0));
    state.add_edge(n1, n2, graphene_core::EdgeData::default());

    let engine = GraphEngineHandle::spawn(state);

    engine
        .send_command(GraphCommand::StepLayoutPhase(LayoutCommand::Sugiyama(
            sugiyama,
        )))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(30));

    let snap = engine.latest_snapshot();
    assert_eq!(snap.positions.len(), 2);
    assert!(snap.version > 0);

    engine.shutdown();
}

#[test]
fn test_fcose_and_cose_phase_stepped_layouts() {
    use graphene_layout::cose::{CoseLayout, CosePhase};
    use graphene_layout::fcose::{FCoseLayout, FCosePhase};
    use graphene_layout::traits::PhaseSteppableLayout;

    let fcose = FCoseLayout::default();
    assert_eq!(PhaseSteppableLayout::<()>::phases(&fcose).len(), 4);
    assert_eq!(
        PhaseSteppableLayout::<()>::current_phase(&fcose),
        Some(FCosePhase::DraftLayout)
    );

    let cose = CoseLayout::default();
    assert_eq!(PhaseSteppableLayout::<()>::phases(&cose).len(), 4);
    assert_eq!(
        PhaseSteppableLayout::<()>::current_phase(&cose),
        Some(CosePhase::Initialization)
    );
}

#[test]
fn test_fruchterman_reingold_layout_execution() {
    use graphene_layout::FruchtermanReingoldLayout;

    let fixtures = get_all_fixtures::<()>();
    let mut fixture = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Small"))
        .unwrap()
        .clone();

    let mut fr = FruchtermanReingoldLayout::default().with_iterations(50);
    fr.compute(&mut fixture.state);
    assert_valid_positions(&fixture.state);
}

#[test]
fn test_tutte_barycentric_layout_execution() {
    use graphene_core::Size2;
    use graphene_layout::TutteBarycentricLayout;

    let mut state = GraphState::<()>::new();
    let n1 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let n2 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let n3 = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));
    let interior = state.add_node(Vec2::default(), Size2::new(10.0, 10.0));

    state.add_edge(n1, n2, graphene_core::EdgeData::default());
    state.add_edge(n2, n3, graphene_core::EdgeData::default());
    state.add_edge(n3, n1, graphene_core::EdgeData::default());
    state.add_edge(interior, n1, graphene_core::EdgeData::default());
    state.add_edge(interior, n2, graphene_core::EdgeData::default());
    state.add_edge(interior, n3, graphene_core::EdgeData::default());

    let mut tutte = TutteBarycentricLayout::default()
        .with_fixed_boundary(vec![n1, n2, n3])
        .with_polygon_radius(100.0)
        .with_max_iterations(50);

    tutte.compute(&mut state);
    assert_valid_positions(&state);
}

#[test]
fn test_multilevel_layout_execution() {
    use graphene_layout::{ForceDirectedLayout, MultilevelLayout};

    let fixtures = get_all_fixtures::<()>();
    let mut fixture = fixtures
        .iter()
        .find(|f| f.name.contains("Undirected Medium"))
        .unwrap()
        .clone();

    let mut ml = MultilevelLayout::new(ForceDirectedLayout::default().with_iterations(20))
        .with_min_graph_size(3);
    ml.compute(&mut fixture.state);
    assert_valid_positions(&fixture.state);
}
