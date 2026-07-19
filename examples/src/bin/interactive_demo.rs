use gpui::{
    px, Bounds, EntityInputHandler, InteractiveElement, MouseDownEvent,
    Pixels, Point, SharedString, StatefulInteractiveElement, Styled, WindowOptions,
};
use gpui::{AppContext, Application, Context, Entity, IntoElement, ParentElement, Render, Window};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::Root;
use graphene_core::fixtures::{get_all_fixtures, GraphFixture};
use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2, UndoRedoManager};
use graphene_layout::{
    BipartiteLayout, CircleLayout, CollisionForceDirectedLayout, CompoundLayout,
    ConcentricHubLayout, DisconnectedPacker, ForceDirectedLayout, GridSortedLayout,
    KamadaKawaiLayout, Layout, MdsLayout, RegionalPartitionLayout, ReingoldTilfordLayout,
    SugiyamaLayout, WeightedForceDirectedLayout,
};
use graphene_style::{ColorValue, ComputedStyle, NodeShape, StylingTarget, ThemeRegistry};
use graphene_gpui::render::draw_pipeline::Viewport;
use graphene_gpui::interaction::state::InteractionState;
use graphene_gpui::render::graph_canvas::GraphCanvas;
use std::collections::HashMap;

// Layout names list
const LAYOUT_NAMES: &[&str] = &[
    "Circle",
    "ForceDirected",
    "CoSE",
    "KamadaKawai",
    "Sugiyama",
    "ReingoldTilford",
    "MDS",
    "Grid",
    "Concentric",
    "Bipartite",
    "WeightedForce",
    "CollisionForce",
    "DisconnectedPack",
    "Compound",
    "RegionalPartition",
];

// Helper to compute point-to-segment distance on screen
fn distance_to_segment(p: Point<Pixels>, a: Point<Pixels>, b: Point<Pixels>) -> f32 {
    let px_val = f32::from(p.x);
    let py_val = f32::from(p.y);
    let ax = f32::from(a.x);
    let ay = f32::from(a.y);
    let bx = f32::from(b.x);
    let by = f32::from(b.y);

    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        let rx = px_val - ax;
        let ry = py_val - ay;
        return (rx * rx + ry * ry).sqrt();
    }

    let t = ((px_val - ax) * dx + (py_val - ay) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let proj_x = ax + t * dx;
    let proj_y = ay + t * dy;

    let rx = px_val - proj_x;
    let ry = py_val - proj_y;
    (rx * rx + ry * ry).sqrt()
}

// Convert ColorValue from Graphene Style to gpui Color
fn color_value_to_gpui_color(color_val: ColorValue) -> gpui::Rgba {
    match color_val {
        ColorValue::Rgba(r, g, b, a) => gpui::rgba(
            ((r * 255.0) as u32) << 24
                | ((g * 255.0) as u32) << 16
                | ((b * 255.0) as u32) << 8
                | (a * 255.0) as u32,
        ),
    }
}

// Themes
struct Theme {
    bg: gpui::Rgba,
    panel_bg: gpui::Rgba,
    border: gpui::Rgba,
    accent: gpui::Rgba,
    text: gpui::Rgba,
    text_dim: gpui::Rgba,
}

impl Theme {
    fn from_style(theme: &graphene_style::Theme) -> Self {
        Self {
            bg: color_value_to_gpui_color(theme.bg),
            panel_bg: color_value_to_gpui_color(theme.panel_bg),
            border: color_value_to_gpui_color(theme.border),
            accent: color_value_to_gpui_color(theme.accent),
            text: color_value_to_gpui_color(theme.text),
            text_dim: color_value_to_gpui_color(theme.text_dim),
        }
    }
}

struct DemoApp {
    state: GraphState<ComputedStyle>,
    fixtures: Vec<GraphFixture<ComputedStyle>>,
    selected_fixture_idx: usize,
    selected_layout: String,

    // Viewport and Interaction states
    viewport: Viewport,
    interaction_state: InteractionState,

    selected_node: Option<NodeId>,
    selected_edge: Option<usize>,

    // Layout parameters input states
    input_gravity: Entity<InputState>,
    input_k_rep: Entity<InputState>,
    input_k_att: Entity<InputState>,
    input_iterations: Entity<InputState>,
    input_circle_radius: Entity<InputState>,
    input_theta: Entity<InputState>,
    input_layer_spacing: Entity<InputState>,
    input_node_spacing: Entity<InputState>,
    input_mds_base_dist: Entity<InputState>,
    input_bipartite_col_spacing: Entity<InputState>,
    input_bipartite_vert_spacing: Entity<InputState>,
    input_packer_spacing: Entity<InputState>,
    input_compound_padding: Entity<InputState>,
    input_regional_columns: Entity<InputState>,
    input_regional_cell_size: Entity<InputState>,

    // CRUD input states
    node_name_state: Entity<InputState>,
    edge_src_state: Entity<InputState>,
    edge_tgt_state: Entity<InputState>,
    edge_weight_state: Entity<InputState>,

    themes: ThemeRegistry,
    current_theme_idx: usize,



    // Live physics simulation fields
    physics_enabled: bool,
    physics_temperature: f32,
    use_barnes_hut: bool,

    // Undo/Redo
    undo_redo: UndoRedoManager<ComputedStyle>,
}

impl DemoApp {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let fixtures = get_all_fixtures::<ComputedStyle>();

        let input_gravity = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "1.0", window, cx);
            s
        });
        let input_k_rep = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "30.0", window, cx);
            s
        });
        let input_k_att = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "30.0", window, cx);
            s
        });
        let input_iterations = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "100", window, cx);
            s
        });
        let input_circle_radius = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "150.0", window, cx);
            s
        });
        let input_theta = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "0.5", window, cx);
            s
        });
        let input_layer_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "80.0", window, cx);
            s
        });
        let input_node_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "60.0", window, cx);
            s
        });
        let input_mds_base_dist = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "50.0", window, cx);
            s
        });
        let input_bipartite_col_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "120.0", window, cx);
            s
        });
        let input_bipartite_vert_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "60.0", window, cx);
            s
        });
        let input_packer_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "80.0", window, cx);
            s
        });
        let input_compound_padding = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "20.0", window, cx);
            s
        });
        let input_regional_columns = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "2", window, cx);
            s
        });
        let input_regional_cell_size = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "250.0", window, cx);
            s
        });

        let node_name_state = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "NodeX", window, cx);
            s
        });
        let edge_src_state = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "", window, cx);
            s
        });
        let edge_tgt_state = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "", window, cx);
            s
        });
        let edge_weight_state = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, "1.0", window, cx);
            s
        });

        let mut app = Self {
            state: GraphState::new(),
            fixtures,
            selected_fixture_idx: 0,
            selected_layout: "Circle".to_string(),
            viewport: Viewport::new(Bounds::default()),
            interaction_state: InteractionState::new(64.0),
            selected_node: None,
            selected_edge: None,
            input_gravity,
            input_k_rep,
            input_k_att,
            input_iterations,
            input_circle_radius,
            input_theta,
            input_layer_spacing,
            input_node_spacing,
            input_mds_base_dist,
            input_bipartite_col_spacing,
            input_bipartite_vert_spacing,
            input_packer_spacing,
            input_compound_padding,
            input_regional_columns,
            input_regional_cell_size,
            node_name_state,
            edge_src_state,
            edge_tgt_state,
            edge_weight_state,
            themes: ThemeRegistry::new(),
            current_theme_idx: 3, // GitHub Light index is 3

            physics_enabled: true,
            physics_temperature: 10.0,
            use_barnes_hut: false,
            undo_redo: UndoRedoManager::new(),
        };
        app.load_preset(0, window, cx);
        app
    }

    fn run_physics_step(&mut self) {
        let n = self.state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let mut forces = vec![Vec2::default(); n];

        let k_rep = 2500.0;
        let k_att = 0.06;
        let gravity = 0.3;

        // 1. Repulsive forces (classical O(N^2) or Barnes-Hut O(N log N))
        if self.use_barnes_hut {
            let positions_slice = &*self.state.positions;
            let quadtree = graphene_layout::Quadtree::build(positions_slice);
            for i in 0..n {
                let pos_i = positions_slice[i];
                forces[i] = quadtree.accumulate_repulsion(i, pos_i, positions_slice, k_rep, 0.5);
            }
        } else {
            for i in 0..n {
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let pos_i = *self.state.positions.get(i);
                    let pos_j = *self.state.positions.get(j);

                    let dx = pos_i.x - pos_j.x;
                    let dy = pos_i.y - pos_j.y;
                    let dist_sq = dx * dx + dy * dy + 0.01;
                    let dist = dist_sq.sqrt();

                    let force = k_rep / dist_sq;
                    forces[i].x += (dx / dist) * force;
                    forces[i].y += (dy / dist) * force;
                }
            }
        }

        let edges_count = self.state.edges.len();
        for i in 0..edges_count {
            let src = *self.state.edge_sources.get(i);
            let tgt = *self.state.edge_targets.get(i);
            let (Some(&src_idx), Some(&tgt_idx)) =
                (self.state.node_keys.get(src), self.state.node_keys.get(tgt))
            else {
                continue;
            };

            let pos_src = *self.state.positions.get(src_idx);
            let pos_tgt = *self.state.positions.get(tgt_idx);

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

        let temp = self.physics_temperature;
        for i in 0..n {
            let id = self.state.node_index_to_id[i];
            let is_dragging = match self.interaction_state.drag_start {
                Some((drag_id, _, _)) => drag_id == id,
                None => false,
            };
            if is_dragging {
                continue;
            }

            let pos = self.state.positions.get_mut(i);

            forces[i].x -= pos.x * gravity;
            forces[i].y -= pos.y * gravity;

            let force_len = (forces[i].x * forces[i].x + forces[i].y * forces[i].y + 0.01).sqrt();
            let limit = force_len.min(temp);

            pos.x += (forces[i].x / force_len) * limit;
            pos.y += (forces[i].y / force_len) * limit;
        }
    }

    fn resolve_collisions(&mut self) {
        let n = self.state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let min_distance = 65.0;

        for _ in 0..3 {
            for i in 0..n {
                for j in (i + 1)..n {
                    let pos_i = *self.state.positions.get(i);
                    let pos_j = *self.state.positions.get(j);

                    let dx = pos_j.x - pos_i.x;
                    let dy = pos_j.y - pos_i.y;
                    let dist_sq = dx * dx + dy * dy;
                    let dist = dist_sq.sqrt();

                    if dist < min_distance {
                        let overlap = min_distance - dist;
                        let push_x;
                        let push_y;
                        if dist > 0.001 {
                            push_x = (dx / dist) * overlap * 0.5;
                            push_y = (dy / dist) * overlap * 0.5;
                        } else {
                            push_x = overlap * 0.5;
                            push_y = 0.0;
                        }

                        let id_i = self.state.node_index_to_id[i];
                        let id_j = self.state.node_index_to_id[j];

                        let is_dragging_i = match self.interaction_state.drag_start {
                            Some((drag_id, _, _)) => drag_id == id_i,
                            None => false,
                        };
                        let is_dragging_j = match self.interaction_state.drag_start {
                            Some((drag_id, _, _)) => drag_id == id_j,
                            None => false,
                        };

                        if is_dragging_i && !is_dragging_j {
                            let p_j = self.state.positions.get_mut(j);
                            p_j.x += push_x * 2.0;
                            p_j.y += push_y * 2.0;
                        } else if is_dragging_j && !is_dragging_i {
                            let p_i = self.state.positions.get_mut(i);
                            p_i.x -= push_x * 2.0;
                            p_i.y -= push_y * 2.0;
                        } else if !is_dragging_i && !is_dragging_j {
                            let p_i = self.state.positions.get_mut(i);
                            p_i.x -= push_x;
                            p_i.y -= push_y;

                            let p_j = self.state.positions.get_mut(j);
                            p_j.x += push_x;
                            p_j.y += push_y;
                        }
                    }
                }
            }
        }
    }

    fn get_theme(&self) -> Theme {
        let style_theme = &self.themes.themes[self.current_theme_idx];
        Theme::from_style(style_theme)
    }

    fn load_preset(&mut self, idx: usize, _window: &mut Window, _cx: &mut Context<Self>) {
        self.selected_fixture_idx = idx;
        let fixture = &self.fixtures[idx];
        self.state = fixture.state.clone();
        self.selected_node = None;
        self.selected_edge = None;

        for i in 0..self.state.node_index_to_id.len() {
            let mut style = ComputedStyle::default();
            if let StylingTarget::Node(ref mut node_style) = style.target {
                node_style.label = Some(i as u32);
                node_style.fill_color =
                    ColorValue::Rgba(137.0 / 255.0, 180.0 / 255.0, 250.0 / 255.0, 1.0);
                node_style.border_color =
                    ColorValue::Rgba(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0, 1.0);
                node_style.border_width = graphene_style::LengthValue::Pixels(2.0);
            }
            self.state.computed_styles.set(i, style);
        }

        for i in 0..self.state.edges.len() {
            let label_str = fixture.edge_labels.get(&i).cloned().unwrap_or_default();
            let mut style = ComputedStyle::default();
            if let StylingTarget::Edge(ref mut edge_style) = style.target {
                edge_style.line_color =
                    ColorValue::Rgba(166.0 / 255.0, 173.0 / 255.0, 200.0 / 255.0, 1.0);
                edge_style.line_width = graphene_style::LengthValue::Pixels(1.5);
                if !label_str.is_empty() {
                    edge_style.label = Some(i as u32);
                }
            }
            self.state.edge_computed_styles.set(i, style);
        }

        let mut circle = CircleLayout {
            radius: 150.0,
            center: Vec2::default(),
            animate: false,
        };
        circle.compute(&mut self.state);
        self.viewport.offset = Vec2::default();
        self.viewport.zoom = 1.0;
        self.physics_temperature = 10.0;
        self.state.dirty_flags |=
            graphene_core::DirtyFlags::POSITION_DIRTY | graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
        self.interaction_state.rebuild_grid(&self.state);
    }

    fn fit_view(&mut self) {
        self.viewport.fit_to_graph(&self.state);
        self.interaction_state.rebuild_grid(&self.state);
    }

    fn trigger_layout(&mut self, cx: &mut Context<Self>) {
        if self.state.node_index_to_id.is_empty() {
            return;
        }

        self.undo_redo.record_state(&self.state);

        let start_pos: Vec<Vec2> = self.state.positions.iter().copied().collect();

        self.run_layout_internal(cx);
        let target_pos: Vec<Vec2> = self.state.positions.iter().copied().collect();

        for (idx, &pos) in start_pos.iter().enumerate() {
            self.state.positions.set(idx, pos);
        }

        let duration = std::time::Duration::from_millis(300);
        for (idx, &node_id) in self.state.node_index_to_id.iter().enumerate() {
            if idx < start_pos.len() && idx < target_pos.len() {
                self.state.animations.tracks.insert(node_id, graphene_core::AnimationTrack::Position {
                    from: start_pos[idx],
                    to: target_pos[idx],
                    duration,
                    elapsed: std::time::Duration::ZERO,
                });
            }
        }
        cx.notify();
    }

    fn run_layout_internal(&mut self, cx: &mut Context<Self>) {
        let gravity = self
            .input_gravity
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(1.0);
        let k_rep = self
            .input_k_rep
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(30.0);
        let k_att = self
            .input_k_att
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(30.0);
        let iterations = self
            .input_iterations
            .read(cx)
            .text()
            .to_string()
            .parse::<usize>()
            .unwrap_or(100);
        let radius = self
            .input_circle_radius
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(150.0);
        let theta = self
            .input_theta
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(0.5);
        let layer_spacing = self
            .input_layer_spacing
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(80.0);
        let node_spacing = self
            .input_node_spacing
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(60.0);
        let mds_base_dist = self
            .input_mds_base_dist
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(50.0);
        let bipartite_col_spacing = self
            .input_bipartite_col_spacing
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(120.0);
        let bipartite_vert_spacing = self
            .input_bipartite_vert_spacing
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(60.0);
        let packer_spacing = self
            .input_packer_spacing
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(80.0);
        let compound_padding = self
            .input_compound_padding
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(20.0);
        let regional_columns = self
            .input_regional_columns
            .read(cx)
            .text()
            .to_string()
            .parse::<usize>()
            .unwrap_or(2);
        let regional_cell_size = self
            .input_regional_cell_size
            .read(cx)
            .text()
            .to_string()
            .parse::<f32>()
            .unwrap_or(250.0);

        match self.selected_layout.as_str() {
            "Circle" => {
                let mut circle = CircleLayout {
                    radius,
                    center: Vec2::default(),
                    animate: false,
                };
                circle.compute(&mut self.state);
            }
            "ForceDirected" => {
                let mut force = ForceDirectedLayout {
                    iterations,
                    ideal_length: 50.0,
                    gravity,
                    k_rep,
                    k_att,
                    initial_temp: 10.0,
                    use_barnes_hut: self.use_barnes_hut,
                    theta,
                };
                force.compute(&mut self.state);
            }
            "CoSE" => {
                let mut cose = CompoundLayout {
                    sub_layout: ForceDirectedLayout {
                        iterations,
                        ideal_length: 50.0,
                        gravity,
                        k_rep,
                        k_att,
                        initial_temp: 10.0,
                        use_barnes_hut: self.use_barnes_hut,
                        theta,
                    },
                    padding: compound_padding,
                };
                cose.compute(&mut self.state);
            }
            "KamadaKawai" => {
                let mut kk = KamadaKawaiLayout {
                    iterations,
                    k: 1.0,
                    l_0: 50.0,
                };
                kk.compute(&mut self.state);
            }
            "Sugiyama" => {
                let mut sugi = SugiyamaLayout {
                    layer_spacing,
                    node_spacing,
                };
                sugi.compute(&mut self.state);
            }
            "ReingoldTilford" => {
                let mut rt = ReingoldTilfordLayout::default();
                rt.compute(&mut self.state);
            }
            "MDS" => {
                let mut mds = MdsLayout {
                    iterations,
                    base_dist: mds_base_dist,
                };
                mds.compute(&mut self.state);
            }
            "Grid" => {
                let mut grid = GridSortedLayout::default();
                grid.compute(&mut self.state);
            }
            "Concentric" => {
                let mut concentric = ConcentricHubLayout::default();
                concentric.compute(&mut self.state);
            }
            "Bipartite" => {
                let node_partitions = vec![0, 0, 1, 1];
                let node_keys_map = self.state.node_keys.clone();
                let mut bipartite = BipartiteLayout {
                    partition_fn: move |id| {
                        let idx = *node_keys_map.get(id).unwrap_or(&0);
                        node_partitions[idx % 4]
                    },
                    column_spacing: bipartite_col_spacing,
                    vertical_spacing: bipartite_vert_spacing,
                };
                bipartite.compute(&mut self.state);
            }
            "WeightedForce" => {
                let weights = self.fixtures[self.selected_fixture_idx].weights.clone();
                let edge_keys = self.state.edge_keys.clone();
                let mut weighted = WeightedForceDirectedLayout {
                    iterations,
                    gravity,
                    k_rep,
                    k_att,
                    weight_fn: move |edge| {
                        if let Some(&idx) = edge_keys.get(edge) {
                            *weights.get(&idx).unwrap_or(&1.0)
                        } else {
                            1.0
                        }
                    },
                };
                weighted.compute(&mut self.state);
            }
            "CollisionForce" => {
                let mut collision = CollisionForceDirectedLayout {
                    iterations,
                    gravity,
                    ideal_length: 50.0,
                };
                collision.compute(&mut self.state);
            }
            "DisconnectedPack" => {
                let mut packer = DisconnectedPacker {
                    sub_layout: ForceDirectedLayout {
                        iterations,
                        ideal_length: 50.0,
                        gravity,
                        k_rep,
                        k_att,
                        initial_temp: 10.0,
                        use_barnes_hut: self.use_barnes_hut,
                        theta,
                    },
                    spacing: packer_spacing,
                };
                packer.compute(&mut self.state);
            }
            "Compound" => {
                let mut comp = CompoundLayout {
                    sub_layout: ForceDirectedLayout {
                        iterations,
                        ideal_length: 50.0,
                        gravity,
                        k_rep,
                        k_att,
                        initial_temp: 10.0,
                        use_barnes_hut: self.use_barnes_hut,
                        theta,
                    },
                    padding: compound_padding,
                };
                comp.compute(&mut self.state);
            }
            "RegionalPartition" => {
                let mut clusters = HashMap::new();
                for (idx, &id) in self.state.node_index_to_id.iter().enumerate() {
                    clusters.insert(id, idx % 4);
                }
                let mut regional = RegionalPartitionLayout {
                    cluster_fn: move |id| *clusters.get(&id).unwrap_or(&0),
                    sub_layout: ForceDirectedLayout {
                        iterations,
                        ideal_length: 50.0,
                        gravity,
                        k_rep,
                        k_att,
                        initial_temp: 10.0,
                        use_barnes_hut: self.use_barnes_hut,
                        theta,
                    },
                    columns: regional_columns,
                    cell_size: regional_cell_size,
                };
                regional.compute(&mut self.state);
            }
            _ => {}
        }
        self.state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }

    fn add_new_node(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let label = self.node_name_state.read(cx).text().to_string();
        if label.trim().is_empty() {
            return;
        }
        self.undo_redo.record_state(&self.state);
        let pos = Vec2::new(0.0, 0.0);
        let id = self.state.add_node(pos, Size2::new(40.0, 40.0));

        let idx = self.state.node_keys[id];
        let mut style = ComputedStyle::default();
        if let StylingTarget::Node(ref mut node_style) = style.target {
            node_style.label = Some(idx as u32);
            node_style.shape = NodeShape::Ellipse;
            node_style.fill_color =
                ColorValue::Rgba(137.0 / 255.0, 180.0 / 255.0, 250.0 / 255.0, 1.0);
            node_style.border_color =
                ColorValue::Rgba(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0, 1.0);
            node_style.border_width = graphene_style::LengthValue::Pixels(2.0);
        }
        self.state.computed_styles.set(idx, style);

        self.fixtures[self.selected_fixture_idx]
            .node_labels
            .insert(id, label);
        self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
        self.interaction_state.rebuild_grid(&self.state);

        self.node_name_state.update(cx, |input, cx| {
            input.replace_text_in_range(None, "", window, cx);
        });
    }

    fn delete_selected_node(&mut self) {
        if let Some(id) = self.selected_node {
            self.undo_redo.record_state(&self.state);
            self.state.remove_node(id);
            self.selected_node = None;
            self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
            self.interaction_state.rebuild_grid(&self.state);
        }
    }

    fn add_new_edge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let src_label = self.edge_src_state.read(cx).text().to_string();
        let tgt_label = self.edge_tgt_state.read(cx).text().to_string();
        let weight_str = self.edge_weight_state.read(cx).text().to_string();

        let fixture = &self.fixtures[self.selected_fixture_idx];
        let mut src_node = None;
        let mut tgt_node = None;

        for &id in &self.state.node_index_to_id {
            let label = fixture.node_labels.get(&id).cloned().unwrap_or_default();
            if label == src_label {
                src_node = Some(id);
            }
            if label == tgt_label {
                tgt_node = Some(id);
            }
        }

        if let (Some(src), Some(tgt)) = (src_node, tgt_node) {
            let edge_idx = self.state.edges.len();
            self.undo_redo.record_state(&self.state);
            self.state.add_edge(src, tgt, EdgeData::default());

            let w = weight_str.parse::<f32>().unwrap_or(1.0);
            self.fixtures[self.selected_fixture_idx]
                .weights
                .insert(edge_idx, w);

            let mut style = ComputedStyle::default();
            if let StylingTarget::Edge(ref mut edge_style) = style.target {
                edge_style.line_color =
                    ColorValue::Rgba(166.0 / 255.0, 173.0 / 255.0, 200.0 / 255.0, 1.0);
                edge_style.line_width = graphene_style::LengthValue::Pixels(1.5);
            }
            self.state.edge_computed_styles.set(edge_idx, style);
            self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
            self.interaction_state.rebuild_grid(&self.state);

            self.edge_src_state.update(cx, |input, cx| {
                input.replace_text_in_range(None, "", window, cx);
            });
            self.edge_tgt_state.update(cx, |input, cx| {
                input.replace_text_in_range(None, "", window, cx);
            });
        }
    }
}

impl Render for DemoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.get_theme();

        let is_animating = !self.state.animations.tracks.is_empty();
        let needs_physics = self.physics_enabled
            && (self.physics_temperature > 0.05 || self.interaction_state.drag_start.is_some());
        let needs_tick = is_animating || needs_physics;

        if needs_tick {
            if is_animating {
                self.state.tick_animations(std::time::Duration::from_millis(16));
                if self.state.animations.tracks.is_empty() {
                    self.interaction_state.rebuild_grid(&self.state);
                }
            } else if needs_physics {
                self.run_physics_step();
            }

            self.resolve_collisions();

            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                this.update(cx, |this, cx| {
                    if this.physics_enabled && !is_animating {
                        if this.interaction_state.drag_start.is_some() {
                            this.physics_temperature = 10.0;
                        } else {
                            this.physics_temperature *= 0.95;
                        }
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }

        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .child(self.render_title_bar(&theme))
            .child(
                gpui::div()
                    .flex()
                    .flex_1()
                    .h(px(0.0))
                    .child(self.render_sidebar_left(&theme, cx))
                    .child(self.render_canvas_view(&theme, window, cx))
                    .child(self.render_sidebar_right(&theme, window, cx)),
            )
            .child(self.render_bottom_bar(&theme))
    }
}

impl DemoApp {
    fn render_title_bar(&self, theme: &Theme) -> impl IntoElement {
        use gpui_component::TitleBar;

        TitleBar::new()
            .bg(theme.panel_bg)
            .border_color(theme.border)
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        gpui::div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .rounded_full()
                            .bg(theme.accent),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Graphene-RS Interactive Visualizer"),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(12.0))
                            .child(format!("Zoom: {:.0}%", self.viewport.zoom * 100.0)),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(12.0))
                            .child("Status: Live (Animated)"),
                    ),
            )
    }

    fn render_sidebar_left(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .flex_col()
            .h_full()
            .bg(theme.panel_bg)
            .border_r(px(1.0))
            .border_color(theme.border)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("1. SELECT GRAPH FIXTURE"),
                    )
                    .child(
                        gpui::div()
                            .id("preset-scroll-container")
                            .flex()
                            .flex_col()
                            .h(px(150.0))
                            .overflow_y_scroll()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .bg(theme.bg)
                            .rounded_md()
                            .children(self.fixtures.iter().enumerate().map(|(idx, f)| {
                                let is_selected = self.selected_fixture_idx == idx;
                                gpui::div()
                                    .id(SharedString::from(format!("preset-{}", idx)))
                                    .p_2()
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    .bg(if is_selected {
                                        theme.accent
                                    } else {
                                        gpui::rgba(0)
                                    })
                                    .text_color(if is_selected {
                                        theme.panel_bg
                                    } else {
                                        theme.text
                                    })
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.load_preset(idx, window, cx);
                                    }))
                                    .child(f.name.clone())
                            })),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("2. LAYOUT ENGINE"),
                    )
                    .child(
                        gpui::div()
                            .id("layout-scroll-container")
                            .flex()
                            .flex_col()
                            .h(px(150.0))
                            .overflow_y_scroll()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .bg(theme.bg)
                            .rounded_md()
                            .children(LAYOUT_NAMES.iter().map(|&name| {
                                let is_selected = self.selected_layout == name;
                                gpui::div()
                                    .id(SharedString::from(format!("layout-{}", name)))
                                    .p_2()
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    .bg(if is_selected {
                                        theme.accent
                                    } else {
                                        gpui::rgba(0)
                                    })
                                    .text_color(if is_selected {
                                        theme.panel_bg
                                    } else {
                                        theme.text
                                    })
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        this.selected_layout = name.to_string();
                                    }))
                                    .child(name)
                            })),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("3. LIVE PHYSICS ENGINE"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text)
                                    .text_size(px(11.0))
                                    .child(if self.physics_enabled {
                                        format!(
                                            "Status: Active (Temp: {:.2})",
                                            self.physics_temperature
                                        )
                                    } else {
                                        "Status: Disabled".to_string()
                                    }),
                            )
                            .child(
                                Button::new("toggle-physics-btn")
                                    .label(if self.physics_enabled {
                                        "DISABLE"
                                    } else {
                                        "ENABLE"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.physics_enabled = !this.physics_enabled;
                                        if this.physics_enabled {
                                            this.physics_temperature = 10.0;
                                        }
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text)
                                    .text_size(px(11.0))
                                    .child(if self.use_barnes_hut {
                                        "Barnes-Hut: ON"
                                    } else {
                                        "Barnes-Hut: OFF"
                                    }),
                            )
                            .child(
                                Button::new("toggle-barnes-hut-btn")
                                    .label(if self.use_barnes_hut {
                                        "CLASSIC"
                                    } else {
                                        "BARNES-HUT"
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.use_barnes_hut = !this.use_barnes_hut;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                Button::new("run-layout-btn")
                    .primary()
                    .label("RUN LAYOUT")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.trigger_layout(cx);
                    })),
            )
            .child(
                gpui::div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("fit-view-btn")
                            .label("FIT VIEW")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.fit_view();
                            })),
                    )
                    .child(
                        Button::new("reset-zoom-btn")
                            .label("RESET")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.viewport.offset = Vec2::default();
                                this.viewport.zoom = 1.0;
                            })),
                    ),
            )
    }

    fn render_sidebar_right(
        &self,
        theme: &Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let layout_params_children = {
            let mut children = Vec::new();
            let layout = self.selected_layout.as_str();

            let has_force_directed = matches!(
                layout,
                "ForceDirected" | "CoSE" | "WeightedForce" | "DisconnectedPack" | "Compound" | "RegionalPartition"
            );

            if has_force_directed {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Gravity"))
                        .child(Input::new(&self.input_gravity))
                );
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Repulsion"))
                        .child(Input::new(&self.input_k_rep))
                );
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Attraction"))
                        .child(Input::new(&self.input_k_att))
                );
            }

            if has_force_directed || matches!(layout, "KamadaKawai" | "MDS" | "CollisionForce") {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Iterations"))
                        .child(Input::new(&self.input_iterations))
                );
            }

            if layout == "Circle" {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Circle Radius"))
                        .child(Input::new(&self.input_circle_radius))
                );
            }

            if matches!(layout, "ForceDirected" | "CoSE" | "DisconnectedPack" | "Compound" | "RegionalPartition") {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Barnes-Hut Theta"))
                        .child(Input::new(&self.input_theta))
                );
            }

            if layout == "Sugiyama" {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Layer Spacing"))
                        .child(Input::new(&self.input_layer_spacing))
                );
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Node Spacing"))
                        .child(Input::new(&self.input_node_spacing))
                );
            }

            if layout == "MDS" {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Base Distance"))
                        .child(Input::new(&self.input_mds_base_dist))
                );
            }

            if layout == "Bipartite" {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Column Spacing"))
                        .child(Input::new(&self.input_bipartite_col_spacing))
                );
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Vertical Spacing"))
                        .child(Input::new(&self.input_bipartite_vert_spacing))
                );
            }

            if layout == "DisconnectedPack" {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Packer Spacing"))
                        .child(Input::new(&self.input_packer_spacing))
                );
            }

            if matches!(layout, "CoSE" | "Compound") {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Compound Padding"))
                        .child(Input::new(&self.input_compound_padding))
                );
            }

            if layout == "RegionalPartition" {
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Regional Columns"))
                        .child(Input::new(&self.input_regional_columns))
                );
                children.push(
                    gpui::div()
                        .child(gpui::div().text_color(theme.text_dim).text_size(px(10.0)).child("Regional Cell Size"))
                        .child(Input::new(&self.input_regional_cell_size))
                );
            }

            if children.is_empty() {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(11.0))
                                .child("No configurable options for this layout.")
                        )
                );
            }

            children
        };

        gpui::div()
            .id("sidebar-right")
            .flex_col()
            .h_full()
            .bg(theme.panel_bg)
            .border_l(px(1.0))
            .border_color(theme.border)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .overflow_y_scroll()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("3. INSPECTOR"),
                    )
                    .child(if let Some(node_id) = self.selected_node {
                        let label = self.fixtures[self.selected_fixture_idx]
                            .node_labels
                            .get(&node_id)
                            .cloned()
                            .unwrap_or_else(|| "No label".to_string());
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text)
                                    .text_size(px(11.0))
                                    .child(format!("Selected Node: {}", label)),
                            )
                            .child(
                                gpui::div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        gpui::div()
                                            .text_color(theme.text)
                                            .text_size(px(11.0))
                                            .child("Shape"),
                                    )
                                    .child(
                                        gpui::div().flex_auto().gap_1().children(
                                            vec![
                                                NodeShape::Ellipse,
                                                NodeShape::Rectangle,
                                                NodeShape::Diamond,
                                            ]
                                            .into_iter()
                                            .map(
                                                |shape| {
                                                    let label = format!("{:?}", shape);
                                                    Button::new(SharedString::from(format!(
                                                        "shape-btn-{}",
                                                        label
                                                    )))
                                                    .label(label)
                                                    .on_click(cx.listener(move |this, _, _, _| {
                                                        if let Some(id) = this.selected_node {
                                                            if let Some(&idx) =
                                                                this.state.node_keys.get(id)
                                                            {
                                                                let style = this
                                                                    .state
                                                                    .computed_styles
                                                                    .get_mut(idx);
                                                                if let StylingTarget::Node(
                                                                    ref mut node_style,
                                                                ) = style.target
                                                                {
                                                                    node_style.shape = shape;
                                                                }
                                                            }
                                                        }
                                                    }))
                                                },
                                            ),
                                        ),
                                    ),
                            )
                            .child(
                                gpui::div()
                                    .id("delete-node-container")
                                    .p_1()
                                    .rounded_md()
                                    .child(
                                        Button::new("delete-node-btn")
                                            .danger()
                                            .label("DELETE NODE")
                                            .on_click(cx.listener(|this, _, _, _| {
                                                this.delete_selected_node();
                                            })),
                                    ),
                            )
                    } else if let Some(edge_idx) = self.selected_edge {
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text)
                                    .text_size(px(11.0))
                                    .child(format!("Selected Edge: idx={}", edge_idx)),
                            )
                            .child(
                                gpui::div()
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        gpui::div()
                                            .text_color(theme.text)
                                            .text_size(px(11.0))
                                            .child("Width"),
                                    )
                                    .child(gpui::div().flex().gap_1().children(
                                        vec![1.5, 3.0, 5.0].into_iter().map(|w| {
                                            Button::new(SharedString::from(format!(
                                                "width-btn-{}",
                                                w
                                            )))
                                            .label(format!("{}px", w))
                                            .on_click(cx.listener(move |this, _, _, _| {
                                                if let Some(edge_idx) = this.selected_edge {
                                                    let style = this
                                                        .state
                                                        .edge_computed_styles
                                                        .get_mut(edge_idx);
                                                    if let StylingTarget::Edge(ref mut edge_style) =
                                                        style.target
                                                    {
                                                        edge_style.line_width =
                                                            graphene_style::LengthValue::Pixels(w);
                                                    }
                                                }
                                            }))
                                        }),
                                    )),
                            )
                            .child(
                                gpui::div()
                                    .id("delete-edge-container")
                                    .p_1()
                                    .rounded_md()
                                    .child(
                                        Button::new("delete-edge-btn")
                                            .danger()
                                            .label("DELETE EDGE")
                                            .on_click(cx.listener(|this, _, _, _| {
                                                if let Some(edge_idx) = this.selected_edge {
                                                    this.undo_redo.record_state(&this.state);
                                                    let id = this.state.edge_index_to_id[edge_idx];
                                                    this.state.remove_edge(id);
                                                    this.selected_edge = None;
                                                    this.state.dirty_flags |=
                                                        graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
                                                    this.interaction_state.rebuild_grid(&this.state);
                                                }
                                            })),
                                    ),
                            )
                    } else {
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child("Select a node or edge to inspect.")
                    }),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("LAYOUT PARAMETERS"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .children(layout_params_children),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("ADD NODE"),
                    )
                    .child(
                        gpui::div()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(Input::new(&self.node_name_state))
                            .child(
                                Button::new("add-node-btn")
                                    .primary()
                                    .label("ADD NODE")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_new_node(window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("ADD EDGE"),
                    )
                    .child(
                        gpui::div()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Source Node Label"),
                            )
                            .child(Input::new(&self.edge_src_state))
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Target Node Label"),
                            )
                            .child(Input::new(&self.edge_tgt_state))
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Weight"),
                            )
                            .child(Input::new(&self.edge_weight_state))
                            .child(
                                Button::new("add-edge-btn")
                                    .primary()
                                    .label("ADD EDGE")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_new_edge(window, cx);
                                    })),
                            ),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(11.0))
                            .child("THEME"),
                    )
                    .child(
                        gpui::div().flex_col().gap_1().children(
                            vec![
                                "Catppuccin Mocha",
                                "Gruvbox Dark",
                                "One Dark",
                                "GitHub Light",
                            ]
                            .into_iter()
                            .map(|t| {
                                let is_active = self.themes.themes[self.current_theme_idx].name == t;
                                gpui::div()
                                    .id(SharedString::from(format!("theme-{}", t)))
                                    .p_1()
                                    .bg(if is_active { theme.accent } else { theme.bg })
                                    .text_color(if is_active {
                                        theme.panel_bg
                                    } else {
                                        theme.text
                                    })
                                    .text_size(px(10.0))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        if let Some(pos) = this.themes.themes.iter().position(|x| x.name == t) {
                                            this.current_theme_idx = pos;
                                        }
                                    }))
                                    .child(t)
                            }),
                        ),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(11.0))
                            .child("HISTORY"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("undo-btn")
                                    .label("UNDO")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.undo_redo.undo(&mut this.state);
                                        this.selected_node = None;
                                        this.selected_edge = None;
                                        this.interaction_state.rebuild_grid(&this.state);
                                    })),
                            )
                            .child(
                                Button::new("redo-btn")
                                    .label("REDO")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.undo_redo.redo(&mut this.state);
                                        this.selected_node = None;
                                        this.selected_edge = None;
                                        this.interaction_state.rebuild_grid(&this.state);
                                    })),
                            )
                    )
            )
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(11.0))
                            .child("WORKSPACE IO"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                Button::new("save-json-btn")
                                    .label("SAVE JSON")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        let json = this.state.to_json();
                                        if let Err(e) = std::fs::write("workspace_graph.json", json) {
                                            println!("Failed to save graph: {:?}", e);
                                        } else {
                                            println!("Saved graph to workspace_graph.json");
                                        }
                                    })),
                            )
                            .child(
                                Button::new("load-json-btn")
                                    .label("LOAD JSON")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        if let Ok(json) = std::fs::read_to_string("workspace_graph.json") {
                                            if let Ok(new_state) = GraphState::from_json(&json) {
                                                this.undo_redo.record_state(&this.state);
                                                this.state = new_state;
                                                this.selected_node = None;
                                                this.selected_edge = None;
                                                this.interaction_state.rebuild_grid(&this.state);
                                                this.viewport.fit_to_graph(&this.state);
                                            }
                                        }
                                    })),
                            )
                            .child(
                                Button::new("export-dot-btn")
                                    .label("EXPORT DOT")
                                    .on_click(cx.listener(|this, _, _, _| {
                                        let dot = this.state.to_dot();
                                        if let Err(e) = std::fs::write("workspace_graph.dot", dot) {
                                            println!("Failed to export DOT: {:?}", e);
                                        } else {
                                            println!("Exported graph to workspace_graph.dot");
                                        }
                                    })),
                            )
                    )
            )
    }

    fn render_canvas_view(
        &self,
        theme: &Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak_entity = cx.weak_entity();
        let fixture = &self.fixtures[self.selected_fixture_idx];

        gpui::div()
            .id("canvas-container")
            .flex_1()
            .h_full()
            .relative()
            .bg(theme.bg)
            .child(
                gpui::canvas(
                    move |bounds, _, cx| {
                        if let Some(entity) = weak_entity.upgrade() {
                            entity.update(cx, |this, _| {
                                this.viewport.bounds = gpui::Bounds {
                                    origin: gpui::point(f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
                                    size: gpui::size(f32::from(bounds.size.width), f32::from(bounds.size.height)),
                                };
                            });
                        }
                    },
                    move |_, _, _, _| {}
                )
                .size_full()
                .absolute(),
            )
            .child(
                GraphCanvas::new(
                    &self.state,
                    &self.viewport,
                    &self.interaction_state,
                    &self.themes.themes[self.current_theme_idx],
                    self.selected_node,
                    &fixture.node_labels,
                )
            )
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    let hit_node = this.interaction_state.hit_test(
                        gpui::point(f32::from(ev.position.x), f32::from(ev.position.y)),
                        &this.viewport,
                        &this.state,
                        this.physics_enabled,
                    );
                    if let Some(node_id) = hit_node {
                        this.undo_redo.record_state(&this.state);
                        this.selected_node = Some(node_id);
                        this.selected_edge = None;

                        let label = this.fixtures[this.selected_fixture_idx]
                            .node_labels
                            .get(&node_id)
                            .cloned()
                            .unwrap_or_else(|| format!("N{}", this.state.node_keys[node_id]));
                        this.node_name_state.update(cx, |input, cx| {
                            input.replace_text_in_range(None, &label, window, cx);
                        });
                    } else {
                        let mut hit_edge = None;
                        for edge_idx in 0..this.state.edges.len() {
                            let src = *this.state.edge_sources.get(edge_idx);
                            let tgt = *this.state.edge_targets.get(edge_idx);
                            let (Some(&src_idx), Some(&tgt_idx)) =
                                (this.state.node_keys.get(src), this.state.node_keys.get(tgt))
                            else {
                                continue;
                            };
                            let pos_src = *this.state.positions.get(src_idx);
                            let pos_tgt = *this.state.positions.get(tgt_idx);

                            let src_screen = this.viewport.model_to_screen(pos_src);
                            let tgt_screen = this.viewport.model_to_screen(pos_tgt);

                            let dist = distance_to_segment(
                                ev.position,
                                gpui::point(px(src_screen.x), px(src_screen.y)),
                                gpui::point(px(tgt_screen.x), px(tgt_screen.y)),
                            );
                            if dist < 8.0 {
                                hit_edge = Some(edge_idx);
                                break;
                            }
                        }

                        if let Some(edge_idx) = hit_edge {
                            this.selected_edge = Some(edge_idx);
                            this.selected_node = None;
                        } else {
                            this.selected_node = None;
                            this.selected_edge = None;
                        }
                    }

                    let mut is_mut = this.interaction_state.clone();
                    is_mut.on_mouse_down(
                        gpui::point(f32::from(ev.position.x), f32::from(ev.position.y)),
                        hit_node,
                        &this.state,
                    );
                    this.interaction_state = is_mut;
                    cx.notify();
                })
            )
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                let mut is_mut = this.interaction_state.clone();
                let mut vp_mut = this.viewport.clone();
                let mut st_mut = this.state.clone();

                is_mut.on_mouse_drag(
                    gpui::point(f32::from(ev.position.x), f32::from(ev.position.y)),
                    &mut vp_mut,
                    &mut st_mut,
                );

                this.interaction_state = is_mut;
                this.viewport = vp_mut;
                this.state = st_mut;

                if this.interaction_state.drag_start.is_some() {
                    this.resolve_collisions();
                    this.state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
                }
                cx.notify();
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    let mut is_mut = this.interaction_state.clone();
                    is_mut.on_mouse_up();
                    this.interaction_state = is_mut;
                    this.interaction_state.rebuild_grid(&this.state);
                    cx.notify();
                })
            )
            .on_scroll_wheel(cx.listener(|this, ev: &gpui::ScrollWheelEvent, _, cx| {
                let amount = match ev.delta {
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.y),
                    gpui::ScrollDelta::Lines(p) => p.y * 20.0,
                };
                let zoom_factor = if amount > 0.0 { 1.05 } else { 0.95 };
                this.viewport.zoom *= zoom_factor;
                this.viewport.zoom = this.viewport.zoom.clamp(0.15, 8.0);
                cx.notify();
            }))
    }

    fn render_bottom_bar(&self, theme: &Theme) -> impl IntoElement {
        let nodes_count = self.state.node_index_to_id.len();
        let edges_count = self.state.edges.len();

        let selection_status = if let Some(node_id) = self.selected_node {
            let label = self.fixtures[self.selected_fixture_idx]
                .node_labels
                .get(&node_id)
                .cloned()
                .unwrap_or_else(|| format!("N{}", self.state.node_keys[node_id]));
            format!("Selected: Node {}", label)
        } else if let Some(edge_idx) = self.selected_edge {
            format!("Selected: Edge #{}", edge_idx)
        } else {
            "Selected: None".to_string()
        };

        let physics_status = if self.physics_enabled {
            format!("Physics: Active (T={:.2})", self.physics_temperature)
        } else {
            "Physics: Disabled".to_string()
        };

        gpui::div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(26.0))
            .px(px(12.0))
            .bg(theme.panel_bg)
            .border_t(px(1.0))
            .border_color(theme.border)
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(format!("Nodes: {}  •  Edges: {}", nodes_count, edges_count)),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.border)
                            .text_size(px(11.0))
                            .child("|"),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.accent)
                            .text_size(px(11.0))
                            .child(selection_status),
                    ),
            )
            .child(
                gpui::div()
                    .text_color(theme.text_dim)
                    .text_size(px(11.0))
                    .italic()
                    .child("Tips: [Left-drag] nodes to move • [Drag bg] to pan • [Scroll] to zoom"),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(physics_status),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.border)
                            .text_size(px(11.0))
                            .child("|"),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(format!("Layout: {}", self.selected_layout)),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.border)
                            .text_size(px(11.0))
                            .child("|"),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(format!("Theme: {}", self.themes.themes[self.current_theme_idx].name)),
                    ),
            )
    }
}

fn main() {
    Application::new().run(|cx| {
        gpui_component::init(cx);
        cx.open_window(
            WindowOptions {
                focus: true,
                titlebar: Some(gpui_component::TitleBar::title_bar_options()),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| DemoApp::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .unwrap();
        cx.on_window_closed(|cx| {
            cx.quit();
        })
        .detach();
        cx.activate(true);
    });
}
