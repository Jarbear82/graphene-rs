use gpui::{
    px, size, Background, Bounds, MouseDownEvent, PathBuilder, Pixels, SharedString, Styled,
    WindowOptions, Point, InteractiveElement, StatefulInteractiveElement,
};
use gpui::{App, Application, AppContext, Context, Element, IntoElement, ParentElement, Render, Window};
use graphene_core::fixtures::{get_all_fixtures, GraphFixture};
use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2};
use graphene_layout::{
    BipartiteLayout, CircleLayout, CollisionForceDirectedLayout, CompoundLayout, ConcentricHubLayout,
    DisconnectedPacker, ForceDirectedLayout, GridSortedLayout, KamadaKawaiLayout, Layout, MdsLayout,
    RegionalPartitionLayout, ReingoldTilfordLayout, SugiyamaLayout, WeightedForceDirectedLayout,
};
use graphene_style::{ColorValue, ComputedStyle, EdgeCurveStyle, NodeShape, StylingTarget};
use std::collections::HashMap;

// Layout methods helper list
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

// High-fidelity custom themes
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
    
    // Viewport Pan/Zoom
    offset: Vec2,
    zoom: f32,
    is_panning: bool,
    pan_start: Point<Pixels>,

    // Selected item for inspector Panel
    selected_node: Option<NodeId>,

    // Add node form state
    new_node_name: String,
    new_node_shape: NodeShape,
    new_node_color_hex: String,

    // Add edge form state
    new_edge_src_label: String,
    new_edge_tgt_label: String,
    new_edge_weight: String,

    current_theme: String,
}

impl DemoApp {
    fn new() -> Self {
        let fixtures = get_all_fixtures::<ComputedStyle>();
        let mut app = Self {
            state: GraphState::new(),
            fixtures,
            selected_fixture_idx: 0,
            selected_layout: "Circle".to_string(),
            offset: Vec2::default(),
            zoom: 1.0,
            is_panning: false,
            pan_start: Point::default(),
            selected_node: None,
            new_node_name: "NodeX".to_string(),
            new_node_shape: NodeShape::Ellipse,
            new_node_color_hex: "89b4fa".to_string(),
            new_edge_src_label: "".to_string(),
            new_edge_tgt_label: "".to_string(),
            new_edge_weight: "1.0".to_string(),
            current_theme: "Catppuccin Mocha".to_string(),
        };
        app.load_preset(0);
        app
    }

    fn get_theme(&self) -> Theme {
        match self.current_theme.as_str() {
            "Gruvbox Dark" => Theme::gruvbox_dark(),
            "One Dark" => Theme::one_dark(),
            _ => Theme::catppuccin_mocha(),
        }
    }

    fn load_preset(&mut self, idx: usize) {
        self.selected_fixture_idx = idx;
        let fixture = &self.fixtures[idx];
        self.state = fixture.state.clone();
        self.selected_node = None;

        // Auto style nodes and edges with default styles to render them beautifully
        for i in 0..self.state.node_index_to_id.len() {
            let _label_str = fixture.node_labels.get(&self.state.node_index_to_id[i])
                .cloned()
                .unwrap_or_else(|| format!("N{}", i));
            let mut style = ComputedStyle::default();
            if let StylingTarget::Node(ref mut node_style) = style.target {
                node_style.label = Some(i as u32);
                node_style.fill_color = ColorValue::Rgba(137.0 / 255.0, 180.0 / 255.0, 250.0 / 255.0, 1.0); // Default blue fill
                node_style.border_color = ColorValue::Rgba(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0, 1.0);
                node_style.border_width = graphene_style::LengthValue::Pixels(2.0);
            }
            self.state.computed_styles.set(i, style);
        }
        
        for i in 0..self.state.edges.len() {
            let label_str = fixture.edge_labels.get(&i)
                .cloned()
                .unwrap_or_default();
            let mut style = ComputedStyle::default();
            if let StylingTarget::Edge(ref mut edge_style) = style.target {
                edge_style.line_color = ColorValue::Rgba(166.0 / 255.0, 173.0 / 255.0, 200.0 / 255.0, 1.0);
                edge_style.line_width = graphene_style::LengthValue::Pixels(1.5);
                if !label_str.is_empty() {
                    edge_style.label = Some(i as u32);
                }
            }
            self.state.edge_computed_styles.set(i, style);
        }

        // Apply Circle Layout as baseline spacing
        let mut circle = CircleLayout { radius: 150.0, center: Vec2::default(), animate: false };
        circle.compute(&mut self.state);
        self.offset = Vec2::default();
        self.zoom = 1.0;
        self.state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY | graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
    }

    fn run_layout(&mut self) {
        match self.selected_layout.as_str() {
            "Circle" => {
                let mut circle = CircleLayout { radius: 150.0, center: Vec2::default(), animate: false };
                circle.compute(&mut self.state);
            }
            "ForceDirected" => {
                let mut force = ForceDirectedLayout::default();
                force.compute(&mut self.state);
            }
            "CoSE" => {
                let mut cose = graphene_layout::CoseLayout::default();
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
                let node_partitions = vec![0, 0, 1, 1]; // baseline column partition
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
                weighted.compute(&mut self.state);
            }
            "CollisionForce" => {
                let mut collision = CollisionForceDirectedLayout::default();
                collision.compute(&mut self.state);
            }
            "DisconnectedPack" => {
                let mut packer = DisconnectedPacker {
                    sub_layout: ForceDirectedLayout::default(),
                    spacing: 80.0,
                };
                packer.compute(&mut self.state);
            }
            "Compound" => {
                let mut comp = CompoundLayout {
                    sub_layout: ForceDirectedLayout::default(),
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
                    sub_layout: ForceDirectedLayout::default(),
                    columns: 2,
                    cell_size: 250.0,
                };
                regional.compute(&mut self.state);
            }
            _ => {}
        }
        self.state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }

    fn add_new_node(&mut self) {
        if self.new_node_name.trim().is_empty() {
            return;
        }
        let pos = Vec2::new(0.0, 0.0);
        let id = self.state.add_node(pos, Size2::new(40.0, 40.0));

        let idx = self.state.node_keys[id];
        let mut style = ComputedStyle::default();
        if let StylingTarget::Node(ref mut node_style) = style.target {
            node_style.label = Some(idx as u32);
            node_style.shape = self.new_node_shape;
            
            // parse color
            let hex_val = u32::from_str_radix(&self.new_node_color_hex, 16).unwrap_or(0x89b4fa);
            let r = ((hex_val >> 16) & 0xFF) as f32 / 255.0;
            let g = ((hex_val >> 8) & 0xFF) as f32 / 255.0;
            let b = (hex_val & 0xFF) as f32 / 255.0;
            node_style.fill_color = ColorValue::Rgba(r, g, b, 1.0);
            node_style.border_color = ColorValue::Rgba(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0, 1.0);
            node_style.border_width = graphene_style::LengthValue::Pixels(2.0);
        }
        self.state.computed_styles.set(idx, style);

        // Also add to fixture node_labels
        self.fixtures[self.selected_fixture_idx].node_labels.insert(id, self.new_node_name.clone());
        self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
    }

    fn delete_selected_node(&mut self) {
        if let Some(id) = self.selected_node {
            self.state.remove_node(id);
            self.selected_node = None;
            self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
        }
    }

    fn add_new_edge(&mut self) {
        let fixture = &self.fixtures[self.selected_fixture_idx];
        let mut src_node = None;
        let mut tgt_node = None;

        for &id in &self.state.node_index_to_id {
            let label = fixture.node_labels.get(&id).cloned().unwrap_or_default();
            if label == self.new_edge_src_label {
                src_node = Some(id);
            }
            if label == self.new_edge_tgt_label {
                tgt_node = Some(id);
            }
        }

        if let (Some(src), Some(tgt)) = (src_node, tgt_node) {
            let edge_idx = self.state.edges.len();
            self.state.add_edge(src, tgt, EdgeData::default());

            // parse weight
            let w = self.new_edge_weight.parse::<f32>().unwrap_or(1.0);
            self.fixtures[self.selected_fixture_idx].weights.insert(edge_idx, w);

            let mut style = ComputedStyle::default();
            if let StylingTarget::Edge(ref mut edge_style) = style.target {
                edge_style.line_color = ColorValue::Rgba(166.0 / 255.0, 173.0 / 255.0, 200.0 / 255.0, 1.0);
                edge_style.line_width = graphene_style::LengthValue::Pixels(1.5);
            }
            self.state.edge_computed_styles.set(edge_idx, style);
            self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
        }
    }
}

fn color_value_to_gpui_color(color_val: ColorValue) -> gpui::Rgba {
    match color_val {
        ColorValue::Rgba(r, g, b, a) => gpui::Rgba { r, g, b, a },
    }
}

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.get_theme();

        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .child(self.render_title_bar(&theme, cx))
            .child(
                gpui::div()
                    .flex()
                    .size_full()
                    .child(self.render_sidebar_left(&theme, cx))
                    .child(self.render_canvas_view(&theme, cx))
                    .child(self.render_sidebar_right(&theme, cx))
            )
    }
}

impl DemoApp {
    fn render_title_bar(&self, theme: &Theme, _cx: &mut Context<Self>) -> impl IntoElement {
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
                            .bg(theme.accent)
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Graphene-RS Interactive Visualizer")
                    )
            )
            .child(
                gpui::div()
                    .text_color(theme.text_dim)
                    .text_size(px(12.0))
                    .child("Status: Live (Hardware Accelerated)")
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
            // 1. Presets List
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
                            .child("1. SELECT GRAPH FIXTURE")
                    )
                    .child(
                        gpui::div()
                            .id("preset-scroll-container")
                            .flex()
                            .flex_col()
                            .h(px(200.0))
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
                                    .bg(if is_selected { theme.accent } else { gpui::rgba(0) })
                                    .text_color(if is_selected { theme.panel_bg } else { theme.text })
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        this.load_preset(idx);
                                    }))
                                    .child(f.name.clone())
                            }))
                    )
            )
            // 2. Layouts List
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
                            .child("2. LAYOUT ENGINE")
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
                                    .bg(if is_selected { theme.accent } else { gpui::rgba(0) })
                                    .text_color(if is_selected { theme.panel_bg } else { theme.text })
                                    .text_size(px(11.0))
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        this.selected_layout = name.to_string();
                                    }))
                                    .child(name)
                            }))
                    )
            )
            .child(
                gpui::div()
                    .id("run-layout-btn")
                    .p_2()
                    .bg(theme.accent)
                    .text_color(theme.panel_bg)
                    .font_weight(gpui::FontWeight::BOLD)
                    .rounded_md()
                    .flex()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, _, _| {
                        this.run_layout();
                    }))
                    .child("RUN LAYOUT")
            )
    }

    fn render_sidebar_right(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .w(px(260.0))
            .h_full()
            .bg(theme.panel_bg)
            .border_l(px(1.0))
            .border_color(theme.border)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            // Topology panel
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
                            .child("3. TOPOLOGY INSPECTOR & CRUD")
                    )
                    .child(
                        if let Some(node_id) = self.selected_node {
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
                                        .child(format!("Selected: {}", label))
                                )
                                .child(
                                    gpui::div()
                                        .id("delete-node-btn")
                                        .p_1()
                                        .bg(gpui::rgb(0xf38ba8))
                                        .text_color(theme.panel_bg)
                                        .rounded_md()
                                        .flex()
                                        .justify_center()
                                        .cursor_pointer()
                                        .on_click(cx.listener(|this, _, _, _| {
                                            this.delete_selected_node();
                                        }))
                                        .child("DELETE NODE")
                                )
                        } else {
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(11.0))
                                .child("Select a node on the canvas to inspect.")
                        }
                    )
            )
            // Add Node Form
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(11.0))
                            .child("ADD NODE")
                    )
                    .child(
                        gpui::div()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Name: NodeX")
                            )
                            .child(
                                gpui::div()
                                    .id("add-node-btn")
                                    .p_1()
                                    .bg(theme.accent)
                                    .text_color(theme.panel_bg)
                                    .text_size(px(10.0))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .on_click(cx.listener(|this, _, _, _| {
                                        this.add_new_node();
                                    }))
                                    .child("ADD")
                            )
                    )
            )
            // Theme selector
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
                            .child("THEME")
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .gap_1()
                            .children(vec!["Catppuccin Mocha", "Gruvbox Dark", "One Dark"].into_iter().map(|t| {
                                let is_active = self.current_theme == t;
                                gpui::div()
                                    .id(SharedString::from(format!("theme-{}", t)))
                                    .p_1()
                                    .bg(if is_active { theme.accent } else { theme.bg })
                                    .text_color(if is_active { theme.panel_bg } else { theme.text })
                                    .text_size(px(10.0))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        this.current_theme = t.to_string();
                                    }))
                                    .child(t)
                            }))
                    )
            )
    }

    fn render_canvas_view(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        let fixture = &self.fixtures[self.selected_fixture_idx];
        let nodes_count = self.state.node_index_to_id.len();
        let edges_count = self.state.edges.len();

        let mut edge_paths = Vec::new();
        // Collect edge endpoints in screen coordinates
        for i in 0..edges_count {
            let src = *self.state.edge_sources.get(i);
            let tgt = *self.state.edge_targets.get(i);
            let (Some(&src_idx), Some(&tgt_idx)) = (self.state.node_keys.get(src), self.state.node_keys.get(tgt)) else {
                continue;
            };

            let pos_src = *self.state.positions.get(src_idx);
            let pos_tgt = *self.state.positions.get(tgt_idx);

            // Convert model positions to canvas/screen coordinates (centered at canvas origin)
            let src_screen = Point {
                x: px(pos_src.x * self.zoom + self.offset.x + 300.0),
                y: px(pos_src.y * self.zoom + self.offset.y + 300.0)
            };
            let tgt_screen = Point {
                x: px(pos_tgt.x * self.zoom + self.offset.x + 300.0),
                y: px(pos_tgt.y * self.zoom + self.offset.y + 300.0)
            };
            edge_paths.push((src_screen, tgt_screen));
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
                    move |_, _, _| {},
                    move |_bounds, _, window, _| {
                        // Paint grid background/edges
                        for (src_p, tgt_p) in &edge_paths {
                            let mut builder = PathBuilder::stroke(px(1.5));
                            builder.move_to(*src_p);
                            builder.line_to(*tgt_p);
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, edge_color);
                            }
                        }
                    }
                )
                .size_full()
                .absolute()
            )
            // Render nodes as interactive divs placed absolutely on top of the canvas
            .children(
                (0..nodes_count).map(|idx| {
                    let id = self.state.node_index_to_id[idx];
                    let pos = *self.state.positions.get(idx);
                    let size_val = *self.state.sizes.get(idx);
                    let label = fixture.node_labels.get(&id).cloned().unwrap_or_else(|| format!("N{}", idx));

                    let screen_x = pos.x * self.zoom + self.offset.x + 300.0 - (size_val.w * self.zoom / 2.0);
                    let screen_y = pos.y * self.zoom + self.offset.y + 300.0 - (size_val.h * self.zoom / 2.0);
                    let node_w = size_val.w * self.zoom;
                    let node_h = size_val.h * self.zoom;

                    let is_selected = self.selected_node == Some(id);

                    // Fetch computed color
                    let fill_color = if is_selected { theme.accent } else { theme.node_fill };
                    let border_color = if is_selected { theme.panel_bg } else { theme.node_border };

                    gpui::div()
                        .id(SharedString::from(format!("node-{}", idx)))
                        .absolute()
                        .left(px(screen_x))
                        .top(px(screen_y))
                        .w(px(node_w))
                        .h(px(node_h))
                        .border(px(1.0))
                        .border_color(border_color)
                        .bg(fill_color)
                        .rounded_md()
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .on_click(cx.listener(move |this, _, _, _| {
                            this.selected_node = Some(id);
                        }))
                        // Drag node handler (simplistic)
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, ev: &MouseDownEvent, _, _| {
                            this.is_panning = false;
                            this.selected_node = Some(id);
                            this.pan_start = ev.position;
                        }))
                        .child(
                            gpui::div()
                                .text_color(if is_selected { theme.panel_bg } else { theme.text })
                                .text_size(px(10.0))
                                .child(label)
                        )
                })
            )
            // Global canvas panning/zooming mouse listeners
            .on_mouse_down(gpui::MouseButton::Right, cx.listener(|this, ev: &MouseDownEvent, _, _| {
                this.is_panning = true;
                this.pan_start = ev.position;
            }))
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                if this.is_panning {
                    let dx = f32::from(ev.position.x - this.pan_start.x);
                    let dy = f32::from(ev.position.y - this.pan_start.y);
                    this.offset.x += dx;
                    this.offset.y += dy;
                    this.pan_start = ev.position;
                    cx.notify();
                }
            }))
            .on_mouse_up(gpui::MouseButton::Right, cx.listener(|this, _, _, _| {
                this.is_panning = false;
            }))
    }
}

fn main() {
    Application::new().run(|cx| {
        cx.open_window(
            WindowOptions {
                focus: true,
                ..Default::default()
            },
            |window, cx| cx.new(|_cx| DemoApp::new()),
        )
        .unwrap();
        cx.on_window_closed(|cx| {
            cx.quit();
        })
        .detach();
        cx.activate(true);
    });
}
