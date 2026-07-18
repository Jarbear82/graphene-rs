use gpui::{
    px, Background, Bounds, EntityInputHandler, InteractiveElement, MouseDownEvent, PathBuilder,
    Pixels, Point, SharedString, StatefulInteractiveElement, Styled, WindowOptions,
};
use gpui::{AppContext, Application, Context, Entity, IntoElement, ParentElement, Render, Window};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::Root;
use graphene_core::fixtures::{get_all_fixtures, GraphFixture};
use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2};
use graphene_layout::{
    BipartiteLayout, CircleLayout, CollisionForceDirectedLayout, CompoundLayout,
    ConcentricHubLayout, DisconnectedPacker, ForceDirectedLayout, GridSortedLayout,
    KamadaKawaiLayout, Layout, MdsLayout, RegionalPartitionLayout, ReingoldTilfordLayout,
    SugiyamaLayout, WeightedForceDirectedLayout,
};
use graphene_style::{ColorValue, ComputedStyle, EdgeCurveStyle, NodeShape, StylingTarget};
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
        _ => gpui::rgba(0x89b4faff),
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
    node_fill: gpui::Rgba,
    node_border: gpui::Rgba,
    edge_color: gpui::Rgba,
}

impl Theme {
    fn catppuccin_mocha() -> Self {
        Self {
            bg: gpui::rgb(0x1e1e2e),
            panel_bg: gpui::rgb(0x181825),
            border: gpui::rgb(0x313244),
            accent: gpui::rgb(0x89b4fa), // blue
            text: gpui::rgb(0xcdd6f4),
            text_dim: gpui::rgb(0xa6adc8),
            node_fill: gpui::rgb(0x313244),
            node_border: gpui::rgb(0xcdd6f4),
            edge_color: gpui::rgb(0x45475a),
        }
    }

    fn gruvbox_dark() -> Self {
        Self {
            bg: gpui::rgb(0x282828),
            panel_bg: gpui::rgb(0x1d2021),
            border: gpui::rgb(0x3c3836),
            accent: gpui::rgb(0xd65d0e), // orange
            text: gpui::rgb(0xfbf1c7),
            text_dim: gpui::rgb(0xa89984),
            node_fill: gpui::rgb(0x3c3836),
            node_border: gpui::rgb(0xfbf1c7),
            edge_color: gpui::rgb(0x504945),
        }
    }

    fn one_dark() -> Self {
        Self {
            bg: gpui::rgb(0x282c34),
            panel_bg: gpui::rgb(0x21252b),
            border: gpui::rgb(0x181a1f),
            accent: gpui::rgb(0x98c379), // green
            text: gpui::rgb(0xabb2bf),
            text_dim: gpui::rgb(0x5c6370),
            node_fill: gpui::rgb(0x3e4452),
            node_border: gpui::rgb(0xabb2bf),
            edge_color: gpui::rgb(0x2c313c),
        }
    }
}

struct DemoApp {
    state: GraphState<ComputedStyle>,
    fixtures: Vec<GraphFixture<ComputedStyle>>,
    selected_fixture_idx: usize,
    selected_layout: String,

    // Viewport transforms
    offset: Vec2,
    zoom: f32,

    // Mouse interaction states
    is_panning: bool,
    pan_start: Point<Pixels>,
    dragging_node: Option<NodeId>,
    selected_node: Option<NodeId>,
    selected_edge: Option<usize>,

    // Layout parameters input states
    input_gravity: Entity<InputState>,
    input_k_rep: Entity<InputState>,
    input_k_att: Entity<InputState>,
    input_iterations: Entity<InputState>,
    input_circle_radius: Entity<InputState>,

    // CRUD input states
    node_name_state: Entity<InputState>,
    edge_src_state: Entity<InputState>,
    edge_tgt_state: Entity<InputState>,
    edge_weight_state: Entity<InputState>,

    current_theme: String,

    // Canvas bounds recorded during paint
    canvas_bounds: Bounds<Pixels>,

    // Layout animation states
    start_positions: Vec<Vec2>,
    target_positions: Vec<Vec2>,
    animation_progress: f32,
    is_animating: bool,
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
            offset: Vec2::default(),
            zoom: 1.0,
            is_panning: false,
            pan_start: Point::default(),
            dragging_node: None,
            selected_node: None,
            selected_edge: None,
            input_gravity,
            input_k_rep,
            input_k_att,
            input_iterations,
            input_circle_radius,
            node_name_state,
            edge_src_state,
            edge_tgt_state,
            edge_weight_state,
            current_theme: "Catppuccin Mocha".to_string(),
            canvas_bounds: Bounds::default(),
            start_positions: Vec::new(),
            target_positions: Vec::new(),
            animation_progress: 0.0,
            is_animating: false,
        };
        app.load_preset(0, window, cx);
        app
    }

    fn get_theme(&self) -> Theme {
        match self.current_theme.as_str() {
            "Gruvbox Dark" => Theme::gruvbox_dark(),
            "One Dark" => Theme::one_dark(),
            _ => Theme::catppuccin_mocha(),
        }
    }

    fn load_preset(&mut self, idx: usize, window: &mut Window, cx: &mut Context<Self>) {
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
        self.offset = Vec2::default();
        self.zoom = 1.0;
        self.state.dirty_flags |=
            graphene_core::DirtyFlags::POSITION_DIRTY | graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
    }

    fn fit_view(&mut self) {
        if self.state.node_index_to_id.is_empty() {
            self.offset = Vec2::default();
            self.zoom = 1.0;
            return;
        }
        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        let mut y_min = f32::MAX;
        let mut y_max = f32::MIN;
        for &id in &self.state.node_index_to_id {
            if let Some(&idx) = self.state.node_keys.get(id) {
                let pos = *self.state.positions.get(idx);
                x_min = x_min.min(pos.x);
                x_max = x_max.max(pos.x);
                y_min = y_min.min(pos.y);
                y_max = y_max.max(pos.y);
            }
        }
        let cx_graph = (x_min + x_max) / 2.0;
        let cy_graph = (y_min + y_max) / 2.0;

        self.offset = Vec2::new(-cx_graph, -cy_graph);

        let w_graph = x_max - x_min + 100.0;
        let h_graph = y_max - y_min + 100.0;
        let w_canvas = f32::from(self.canvas_bounds.size.width);
        let h_canvas = f32::from(self.canvas_bounds.size.height);

        if w_canvas > 0.0 && h_canvas > 0.0 {
            let z_x = w_canvas / w_graph;
            let z_y = h_canvas / h_graph;
            self.zoom = z_x.min(z_y).clamp(0.2, 3.0);
        } else {
            self.zoom = 1.0;
        }
    }

    fn trigger_layout(&mut self, cx: &mut Context<Self>) {
        if self.state.node_index_to_id.is_empty() {
            return;
        }

        let start_pos: Vec<Vec2> = self.state.positions.iter().copied().collect();

        self.run_layout_internal(cx);
        let target_pos: Vec<Vec2> = self.state.positions.iter().copied().collect();

        for (idx, &pos) in start_pos.iter().enumerate() {
            self.state.positions.set(idx, pos);
        }

        self.start_positions = start_pos;
        self.target_positions = target_pos;
        self.animation_progress = 0.0;
        self.is_animating = true;
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
                    },
                    padding: 20.0,
                };
                cose.compute(&mut self.state);
            }
            "KamadaKawai" => {
                let mut kk = KamadaKawaiLayout::default();
                kk.compute(&mut self.state);
            }
            "Sugiyama" => {
                let mut sugi = SugiyamaLayout::default();
                sugi.compute(&mut self.state);
            }
            "ReingoldTilford" => {
                let mut rt = ReingoldTilfordLayout::default();
                rt.compute(&mut self.state);
            }
            "MDS" => {
                let mut mds = MdsLayout::default();
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
                    column_spacing: 120.0,
                    vertical_spacing: 60.0,
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
                let mut collision = CollisionForceDirectedLayout::default();
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
                    },
                    spacing: 80.0,
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
                    },
                    padding: 20.0,
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
                    },
                    columns: 2,
                    cell_size: 250.0,
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

        self.node_name_state.update(cx, |input, cx| {
            input.replace_text_in_range(None, "", window, cx);
        });
    }

    fn delete_selected_node(&mut self) {
        if let Some(id) = self.selected_node {
            self.state.remove_node(id);
            self.selected_node = None;
            self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
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
        let weak_entity = cx.weak_entity();

        if self.is_animating {
            let progress = self.animation_progress;

            for idx in 0..self.state.node_index_to_id.len() {
                if idx < self.start_positions.len() && idx < self.target_positions.len() {
                    let start = self.start_positions[idx];
                    let target = self.target_positions[idx];
                    let current = start * (1.0 - progress) + target * progress;
                    self.state.positions.set(idx, current);
                }
            }

            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                this.update(cx, |this, cx| {
                    this.animation_progress += 0.05;
                    if this.animation_progress >= 1.0 {
                        this.animation_progress = 1.0;
                        this.is_animating = false;
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
                    .size_full()
                    .child(self.render_sidebar_left(&theme, cx))
                    .child(self.render_canvas_view(&theme, window, cx))
                    .child(self.render_sidebar_right(&theme, window, cx)),
            )
    }
}

impl DemoApp {
    fn render_title_bar(&self, theme: &Theme) -> impl IntoElement {
        gpui::div()
            .flex()
            .justify_between()
            .items_center()
            .h(px(40.0))
            .px_4()
            .bg(theme.panel_bg)
            .border_b(px(1.0))
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
                            .child(format!("Zoom: {:.0}%", self.zoom * 100.0)),
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
            .w(px(250.0))
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
                                this.offset = Vec2::default();
                                this.zoom = 1.0;
                            })),
                    ),
            )
    }

    fn render_sidebar_right(
        &self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        gpui::div()
            .id("sidebar-right")
            .w(px(260.0))
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
                                        gpui::div().flex().gap_1().children(
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
                                                    let id = this.state.edge_index_to_id[edge_idx];
                                                    this.state.remove_edge(id);
                                                    this.selected_edge = None;
                                                    this.state.dirty_flags |=
                                                        graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
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
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Gravity"),
                            )
                            .child(Input::new(&self.input_gravity))
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Repulsion"),
                            )
                            .child(Input::new(&self.input_k_rep))
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Attraction"),
                            )
                            .child(Input::new(&self.input_k_att))
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Iterations"),
                            )
                            .child(Input::new(&self.input_iterations)),
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
                        gpui::div().flex().gap_1().children(
                            vec!["Catppuccin Mocha", "Gruvbox Dark", "One Dark"]
                                .into_iter()
                                .map(|t| {
                                    let is_active = self.current_theme == t;
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
                                            this.current_theme = t.to_string();
                                        }))
                                        .child(t)
                                }),
                        ),
                    ),
            )
    }

    fn render_canvas_view(
        &self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak_entity = cx.weak_entity();
        let fixture = &self.fixtures[self.selected_fixture_idx];
        let nodes_count = self.state.node_index_to_id.len();
        let edges_count = self.state.edges.len();

        let mut edge_paths = Vec::new();
        let half_w = f32::from(self.canvas_bounds.size.width) / 2.0;
        let half_h = f32::from(self.canvas_bounds.size.height) / 2.0;
        let cx_val = f32::from(self.canvas_bounds.origin.x) + half_w;
        let cy_val = f32::from(self.canvas_bounds.origin.y) + half_h;

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

            let src_screen = Point {
                x: px(pos_src.x * self.zoom + self.offset.x + cx_val),
                y: px(pos_src.y * self.zoom + self.offset.y + cy_val),
            };
            let tgt_screen = Point {
                x: px(pos_tgt.x * self.zoom + self.offset.x + cx_val),
                y: px(pos_tgt.y * self.zoom + self.offset.y + cy_val),
            };

            let curve_style = match self.state.edge_computed_styles.get(i).target {
                StylingTarget::Edge(edge_style) => edge_style.curve_style,
                _ => EdgeCurveStyle::Straight,
            };

            edge_paths.push((src_screen, tgt_screen, curve_style));
        }

        let edge_color = theme.edge_color;

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
                                this.canvas_bounds = bounds;
                            });
                        }
                    },
                    move |_bounds, _, window, _| {
                        let origin_x = f32::from(_bounds.origin.x);
                        let origin_y = f32::from(_bounds.origin.y);
                        let width = f32::from(_bounds.size.width);
                        let height = f32::from(_bounds.size.height);

                        let grid_spacing = 45.0;
                        let mut x = 0.0;
                        while x < width {
                            let mut builder = PathBuilder::stroke(px(1.0));
                            builder.move_to(gpui::point(px(origin_x + x), px(origin_y)));
                            builder.line_to(gpui::point(px(origin_x + x), px(origin_y + height)));
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, gpui::rgba(0x2d313c11));
                            }
                            x += grid_spacing;
                        }
                        let mut y = 0.0;
                        while y < height {
                            let mut builder = PathBuilder::stroke(px(1.0));
                            builder.move_to(gpui::point(px(origin_x), px(origin_y + y)));
                            builder.line_to(gpui::point(px(origin_x + width), px(origin_y + y)));
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, gpui::rgba(0x2d313c11));
                            }
                            y += grid_spacing;
                        }

                        for (src_p, tgt_p, curve_style) in &edge_paths {
                            let mut builder = PathBuilder::stroke(px(2.0));
                            builder.move_to(*src_p);

                            match curve_style {
                                EdgeCurveStyle::Straight => {
                                    builder.line_to(*tgt_p);
                                }
                                _ => {
                                    let mid_x = (f32::from(src_p.x) + f32::from(tgt_p.x)) / 2.0;
                                    let mid_y = (f32::from(src_p.y) + f32::from(tgt_p.y)) / 2.0;
                                    let dx = f32::from(tgt_p.x) - f32::from(src_p.x);
                                    let dy = f32::from(tgt_p.y) - f32::from(src_p.y);
                                    let len = (dx * dx + dy * dy).sqrt();
                                    let curvature = 35.0;
                                    let ctrl = if len > 0.0 {
                                        Point {
                                            x: px(mid_x - (dy / len) * curvature),
                                            y: px(mid_y + (dx / len) * curvature),
                                        }
                                    } else {
                                        Point {
                                            x: px(mid_x),
                                            y: px(mid_y),
                                        }
                                    };
                                    builder.curve_to(ctrl, *tgt_p);
                                }
                            }
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, edge_color);
                            }
                        }
                    },
                )
                .size_full()
                .absolute(),
            )
            .children((0..nodes_count).map(|idx| {
                let id = self.state.node_index_to_id[idx];
                let pos = *self.state.positions.get(idx);
                let size_val = *self.state.sizes.get(idx);
                let label = fixture
                    .node_labels
                    .get(&id)
                    .cloned()
                    .unwrap_or_else(|| format!("N{}", idx));

                let screen_x =
                    pos.x * self.zoom + self.offset.x + half_w - (size_val.w * self.zoom / 2.0);
                let screen_y =
                    pos.y * self.zoom + self.offset.y + half_h - (size_val.h * self.zoom / 2.0);
                let node_w = size_val.w * self.zoom;
                let node_h = size_val.h * self.zoom;

                let is_selected = self.selected_node == Some(id);

                let mut fill_color = if is_selected {
                    theme.accent
                } else {
                    theme.node_fill
                };
                let mut border_color = if is_selected {
                    theme.panel_bg
                } else {
                    theme.node_border
                };

                if idx < self.state.computed_styles.len() {
                    if let StylingTarget::Node(node_style) =
                        self.state.computed_styles.get(idx).target
                    {
                        fill_color = color_value_to_gpui_color(node_style.fill_color);
                        border_color = color_value_to_gpui_color(node_style.border_color);
                    }
                }

                if is_selected {
                    border_color = theme.accent;
                }

                gpui::div()
                    .id(SharedString::from(format!("node-{}", idx)))
                    .absolute()
                    .left(px(screen_x))
                    .top(px(screen_y))
                    .w(px(node_w))
                    .h(px(node_h))
                    .border(px(2.0))
                    .border_color(border_color)
                    .bg(fill_color)
                    .rounded_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, _| {
                        this.selected_node = Some(id);
                        this.selected_edge = None;
                    }))
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .text_size(px(10.0))
                            .child(label),
                    )
            }))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    let mut hit_node = None;
                    for idx in (0..this.state.node_index_to_id.len()).rev() {
                        let id = this.state.node_index_to_id[idx];
                        let pos = *this.state.positions.get(idx);
                        let size_val = *this.state.sizes.get(idx);

                        let cx_val = f32::from(this.canvas_bounds.origin.x)
                            + f32::from(this.canvas_bounds.size.width) / 2.0;
                        let cy_val = f32::from(this.canvas_bounds.origin.y)
                            + f32::from(this.canvas_bounds.size.height) / 2.0;

                        let node_center_x = pos.x * this.zoom + this.offset.x + cx_val;
                        let node_center_y = pos.y * this.zoom + this.offset.y + cy_val;
                        let half_w = (size_val.w * this.zoom) / 2.0;
                        let half_h = (size_val.h * this.zoom) / 2.0;

                        let px_val = f32::from(ev.position.x);
                        let py_val = f32::from(ev.position.y);

                        if px_val >= node_center_x - half_w
                            && px_val <= node_center_x + half_w
                            && py_val >= node_center_y - half_h
                            && py_val <= node_center_y + half_h
                        {
                            hit_node = Some(id);
                            break;
                        }
                    }

                    if let Some(node_id) = hit_node {
                        this.selected_node = Some(node_id);
                        this.selected_edge = None;
                        this.dragging_node = Some(node_id);
                        this.pan_start = ev.position;

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

                            let cx_val = f32::from(this.canvas_bounds.origin.x)
                                + f32::from(this.canvas_bounds.size.width) / 2.0;
                            let cy_val = f32::from(this.canvas_bounds.origin.y)
                                + f32::from(this.canvas_bounds.size.height) / 2.0;

                            let src_screen = Point {
                                x: px(pos_src.x * this.zoom + this.offset.x + cx_val),
                                y: px(pos_src.y * this.zoom + this.offset.y + cy_val),
                            };
                            let tgt_screen = Point {
                                x: px(pos_tgt.x * this.zoom + this.offset.x + cx_val),
                                y: px(pos_tgt.y * this.zoom + this.offset.y + cy_val),
                            };

                            let dist = distance_to_segment(ev.position, src_screen, tgt_screen);
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
                            this.is_panning = true;
                            this.pan_start = ev.position;
                        }
                    }
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                if let Some(node_id) = this.dragging_node {
                    let dx = f32::from(ev.position.x - this.pan_start.x) / this.zoom;
                    let dy = f32::from(ev.position.y - this.pan_start.y) / this.zoom;
                    if let Some(&idx) = this.state.node_keys.get(node_id) {
                        let pos = this.state.positions.get_mut(idx);
                        pos.x += dx;
                        pos.y += dy;
                    }
                    this.pan_start = ev.position;
                    this.state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
                    cx.notify();
                } else if this.is_panning {
                    let dx = f32::from(ev.position.x - this.pan_start.x);
                    let dy = f32::from(ev.position.y - this.pan_start.y);
                    this.offset.x += dx;
                    this.offset.y += dy;
                    this.pan_start = ev.position;
                    cx.notify();
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, _| {
                    this.dragging_node = None;
                    this.is_panning = false;
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev: &gpui::ScrollWheelEvent, _, cx| {
                let amount = match ev.delta {
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.y),
                    gpui::ScrollDelta::Lines(p) => p.y * 20.0,
                };
                let zoom_factor = if amount > 0.0 { 1.05 } else { 0.95 };
                this.zoom *= zoom_factor;
                this.zoom = this.zoom.clamp(0.15, 8.0);
                cx.notify();
            }))
    }
}

fn main() {
    Application::new().run(|cx| {
        gpui_component::init(cx);
        cx.open_window(
            WindowOptions {
                focus: true,
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
