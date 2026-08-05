use gpui::prelude::*;
use gpui::{px, Application, Bounds, IntoElement, Render, Styled, WindowBounds, WindowOptions};
use gpui_component::StyledExt;
use graphene_core::{EdgeData, GraphState, NodeId, Size2, Vec2};
use graphene_gpui::render::draw_pipeline::Viewport;
use graphene_gpui::render::graph_canvas::CanvasConfig;
use graphene_layout::{
    BipartiteLayout, CircleLayout, CircularAdvancedLayout, CollisionForceDirectedLayout,
    ConcentricHubLayout, CoseLayout, ForceDirectedLayout, FA2Settings,
    GridSortedLayout, KamadaKawaiLayout, Layout, MdsLayout, ReingoldTilfordLayout,
    SugiyamaLayout, TutteBarycentricLayout, force_atlas2_step, FA2Node, FA2Edge
};
use graphene_style::ComputedStyle;
use std::collections::HashMap;
use std::time::Instant;

struct GuiPerformanceBenchmarkApp {
    state: GraphState<ComputedStyle>,
    scale: usize,
    selected_algo: String,
    layout_compute_ms: f64,
    render_build_ms: f64,
    visible_node_count: usize,
    viewport: Viewport,
    config: CanvasConfig,
}

impl GuiPerformanceBenchmarkApp {
    pub fn new(window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) -> Self {
        let screen_bounds: Bounds<f32> = Bounds {
            origin: gpui::point(0.0, 0.0),
            size: gpui::size(1280.0, 800.0),
        };
        let initial_scale = 1000;
        let mut app = Self {
            state: GraphState::new(),
            scale: initial_scale,
            selected_algo: "ForceAtlas2".to_string(),
            layout_compute_ms: 0.0,
            render_build_ms: 0.0,
            visible_node_count: 0,
            viewport: Viewport::new(screen_bounds),
            config: CanvasConfig::default(),
        };
        app.rebuild_and_run_layout();
        app
    }

    fn rebuild_and_run_layout(&mut self) {
        // 1. Generate Synthetic Graph
        self.state = GraphState::new();
        let mut nodes = Vec::with_capacity(self.scale);
        for i in 0..self.scale {
            let angle = (i as f32) * 0.1;
            let r = 50.0 + (i as f32).sqrt() * 10.0;
            let pos = Vec2::new(r * angle.cos(), r * angle.sin());
            let id = self.state.add_node(pos, Size2::new(40.0, 40.0));
            nodes.push(id);
        }
        for i in 0..self.scale {
            let hub1 = 0;
            let hub2 = i / 10;
            self.state.add_edge(nodes[i], nodes[hub1], EdgeData::default());
            if hub2 != hub1 && hub2 < self.scale {
                self.state.add_edge(nodes[i], nodes[hub2], EdgeData::default());
            }
            if i + 1 < self.scale {
                self.state.add_edge(nodes[i], nodes[i + 1], EdgeData::default());
            }
        }

        // 2. Run Selected Layout Algorithm & Time Execution
        let start = Instant::now();
        match self.selected_algo.as_str() {
            "CircleLayout" => {
                let mut layout = CircleLayout { radius: 500.0, center: Vec2::new(0.0, 0.0), animate: false };
                layout.compute(&mut self.state);
            }
            "GridSortedLayout" => {
                let mut layout = GridSortedLayout::default();
                layout.compute(&mut self.state);
            }
            "BipartiteLayout" => {
                let n_map: HashMap<NodeId, usize> = self.state.node_index_to_id.iter().enumerate().map(|(i, &id)| (id, i)).collect();
                let mut layout = BipartiteLayout {
                    partition_fn: move |id| if *n_map.get(&id).unwrap_or(&0) % 2 == 0 { 0 } else { 1 },
                    column_spacing: 120.0, vertical_spacing: 50.0
                };
                layout.compute(&mut self.state);
            }
            "ConcentricHubLayout" => {
                let mut layout = ConcentricHubLayout::default();
                layout.compute(&mut self.state);
            }
            "CircularAdvancedLayout" => {
                let layout = CircularAdvancedLayout::default();
                layout.apply(&mut self.state);
            }
            "ReingoldTilfordLayout" => {
                let mut layout = ReingoldTilfordLayout::default();
                layout.compute(&mut self.state);
            }
            "ForceAtlas2" => {
                let n_nodes = self.state.node_index_to_id.len();
                let mut fa2_nodes: Vec<FA2Node> = (0..n_nodes)
                    .map(|i| { let p = *self.state.positions.get(i); FA2Node::new(p.x as f64, p.y as f64, 1.0) })
                    .collect();
                let fa2_edges: Vec<FA2Edge> = (0..self.state.edges.len())
                    .map(|i| {
                        let src = *self.state.edge_sources.get(i);
                        let tgt = *self.state.edge_targets.get(i);
                        let u = self.state.node_keys.get(src).copied().unwrap_or(0);
                        let v = self.state.node_keys.get(tgt).copied().unwrap_or(0);
                        FA2Edge { source: u, target: v, weight: 1.0 }
                    })
                    .collect();
                let settings = FA2Settings::infer_settings(n_nodes, self.state.edges.len(), 20.0);
                let mut speed = 1.0;
                let mut speed_eff = 1.0;
                for _step in 0..50 {
                    force_atlas2_step(&mut fa2_nodes, &fa2_edges, &settings, &mut speed, &mut speed_eff);
                }
                for i in 0..n_nodes {
                    self.state.positions.set(i, Vec2::new(fa2_nodes[i].pos.x as f32, fa2_nodes[i].pos.y as f32));
                }
            }
            "CoseLayout" => {
                let mut layout = CoseLayout::default();
                layout.compute(&mut self.state);
            }
            "SugiyamaLayout" => {
                let mut layout = SugiyamaLayout::default();
                layout.compute(&mut self.state);
            }
            "ForceDirectedLayout" => {
                let mut layout = ForceDirectedLayout::default();
                layout.compute(&mut self.state);
            }
            "CollisionForceDirected" => {
                let mut layout = CollisionForceDirectedLayout::default();
                layout.compute(&mut self.state);
            }
            "KamadaKawaiLayout" => {
                let mut layout = KamadaKawaiLayout::default();
                layout.compute(&mut self.state);
            }
            "MdsLayout" => {
                let mut layout = MdsLayout::default();
                layout.compute(&mut self.state);
            }
            "TutteBarycentricLayout" => {
                let mut layout = TutteBarycentricLayout::default();
                layout.compute(&mut self.state);
            }
            _ => {}
        }
        self.layout_compute_ms = start.elapsed().as_secs_f64() * 1000.0;
        let view = graphene_gpui::GraphView::from_state(&self.state);
        self.viewport.fit_to_graph(&view);
    }
}

impl Render for GuiPerformanceBenchmarkApp {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        // Measure Element Tree Construction
        let render_start = Instant::now();
        let n = self.state.node_index_to_id.len();
        let mut node_elements = Vec::new();
        let mut visible_count = 0;

        for i in 0..n {
            let pos = *self.state.positions.get(i);
            let size = *self.state.sizes.get(i);
            if self.viewport.is_visible(pos, size) {
                visible_count += 1;
                // Render first 200 nodes in GUI tree for fast viewport presentation
                if visible_count <= 200 {
                    let screen_p = self.viewport.model_to_screen(pos);
                    node_elements.push(
                        gpui::div()
                            .absolute()
                            .left(px(screen_p.x))
                            .top(px(screen_p.y))
                            .w(px(size.w * self.viewport.zoom))
                            .h(px(size.h * self.viewport.zoom))
                            .bg(gpui::rgba(0x3b82f6ff))
                            .border_1()
                            .border_color(gpui::rgba(0x000000ff))
                            .rounded_sm()
                    );
                }
            }
        }
        self.visible_node_count = visible_count;
        self.render_build_ms = render_start.elapsed().as_secs_f64() * 1000.0;

        let scale_options = vec![10, 100, 1000, 10000];
        let algos = vec![
            "ForceAtlas2", "CircleLayout", "GridSortedLayout", "BipartiteLayout",
            "ConcentricHubLayout", "CircularAdvancedLayout", "ReingoldTilfordLayout",
            "CoseLayout", "SugiyamaLayout", "ForceDirectedLayout", "CollisionForceDirected",
            "KamadaKawaiLayout", "MdsLayout", "TutteBarycentricLayout"
        ];

        gpui::div()
            .size_full()
            .bg(gpui::rgba(0x0f172aff)) // Dark navy slate background
            .flex()
            .flex_col()
            .child(
                // Header & Telemetry Overlay HUD
                gpui::div()
                    .w_full()
                    .bg(gpui::rgba(0x1e293bff))
                    .p_4()
                    .border_b_1()
                    .border_color(gpui::rgba(0x334155ff))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                gpui::div()
                                    .text_color(gpui::rgba(0xf8fafcff))
                                    .text_xl()
                                    .font_bold()
                                    .child("🚀 Graphene-RS Performance Telemetry Benchmark HUD")
                            )
                    )
                    .child(
                        // Metrics Bar
                        gpui::div()
                            .flex()
                            .gap_4()
                            .child(
                                gpui::div()
                                    .bg(gpui::rgba(0x0f172aff))
                                    .p_3()
                                    .rounded_md()
                                    .child(format!("⏱️ Layout Compute: {:.3} ms", self.layout_compute_ms))
                                    .text_color(gpui::rgba(0x38bdf8ff))
                            )
                            .child(
                                gpui::div()
                                    .bg(gpui::rgba(0x0f172aff))
                                    .p_3()
                                    .rounded_md()
                                    .child(format!("🎨 Element Build: {:.3} ms", self.render_build_ms))
                                    .text_color(gpui::rgba(0x4ade80ff))
                            )
                            .child(
                                gpui::div()
                                    .bg(gpui::rgba(0x0f172aff))
                                    .p_3()
                                    .rounded_md()
                                    .child(format!("👁️ Visible / Total: {} / {}", self.visible_node_count, self.scale))
                                    .text_color(gpui::rgba(0xfacc15ff))
                            )
                    )
            )
            .child(
                // Interactive Canvas Area
                gpui::div()
                    .flex_1()
                    .relative()
                    .bg(gpui::rgba(0x020617ff))
                    .children(node_elements)
            )
    }
}

fn main() {
    println!("Launching Graphene-RS GUI Performance Benchmark App...");
    Application::new().run(|cx: &mut gpui::App| {
        gpui_component::init(cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(gpui::Bounds::centered(
                    None,
                    gpui::size(gpui::px(1280.0), gpui::px(800.0)),
                    cx,
                ))),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Graphene-RS GUI Performance Telemetry Benchmark".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::point(gpui::px(8.0), gpui::px(8.0))),
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| GuiPerformanceBenchmarkApp::new(window, cx));
                cx.new(|cx| gpui_component::Root::new(view, window, cx))
            },
        )
        .expect("Failed to start application");
    });
}
