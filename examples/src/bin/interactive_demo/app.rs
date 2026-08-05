use std::collections::{HashMap, HashSet};
use std::time::Instant;

use gpui::{AppContext, Context, Entity, EntityInputHandler, Window};
use gpui_component::input::InputState;

use crate::theme::Theme;
use graphene_analysis::GraphAnalysisReport;
use graphene_core::{math::Vec2, EdgeData, NodeId, Size2};
use graphene_fixtures::{get_all_fixtures, GraphFixture};
use graphene_gpui::{
    interaction::state::InteractionState, render::draw_pipeline::Viewport, CanvasConfig, GraphView,
};
use graphene_layout::{
    CircleLayout, CoseLayout, FCoseLayout, GraphEngineHandle, LayoutCommand, LiveForceSimulation,
    SugiyamaLayout,
};
use graphene_style::{ColorValue, ComputedStyle, NodeShape, StylingTarget, ThemeRegistry};

pub struct DemoApp {
    pub gravity: f32,
    pub k_rep: f32,
    pub k_att: f32,
    pub iterations: usize,
    pub circle_radius: f32,
    pub theta: f32,
    pub layer_spacing: f32,
    pub node_spacing: f32,
    pub mds_base_dist: f32,
    pub bipartite_col_spacing: f32,
    pub bipartite_vert_spacing: f32,
    pub packer_spacing: f32,
    pub compound_padding: f32,
    pub regional_columns: usize,
    pub regional_cell_size: f32,
    pub max_label_len: usize,

    pub grid_spacing: f32,
    pub arrow_length: f32,
    pub arrow_width: f32,
    pub edge_stroke_width: f32,
    pub edge_curvature: f32,

    pub engine: GraphEngineHandle<ComputedStyle>,
    pub view: GraphView<ComputedStyle>,
    pub fixtures: Vec<GraphFixture<ComputedStyle>>,
    pub selected_fixture_idx: usize,
    pub selected_layout: String,
    pub expanded_layout: Option<String>,

    pub viewport: Viewport,
    pub interaction_state: InteractionState,

    pub selected_node: Option<NodeId>,
    pub selected_edge: Option<usize>,

    pub input_gravity: Entity<InputState>,
    pub input_k_rep: Entity<InputState>,
    pub input_k_att: Entity<InputState>,
    pub input_iterations: Entity<InputState>,
    pub input_circle_radius: Entity<InputState>,
    pub input_theta: Entity<InputState>,
    pub input_layer_spacing: Entity<InputState>,
    pub input_node_spacing: Entity<InputState>,
    pub input_mds_base_dist: Entity<InputState>,
    pub input_bipartite_col_spacing: Entity<InputState>,
    pub input_bipartite_vert_spacing: Entity<InputState>,
    pub input_packer_spacing: Entity<InputState>,
    pub input_compound_padding: Entity<InputState>,
    pub input_regional_columns: Entity<InputState>,
    pub input_regional_cell_size: Entity<InputState>,

    pub input_grid_spacing: Entity<InputState>,
    pub input_arrow_length: Entity<InputState>,
    pub input_arrow_width: Entity<InputState>,
    pub input_edge_stroke: Entity<InputState>,
    pub input_edge_curvature: Entity<InputState>,

    pub node_name_state: Entity<InputState>,
    pub edge_src_state: Entity<InputState>,
    pub edge_tgt_state: Entity<InputState>,
    pub edge_weight_state: Entity<InputState>,

    pub themes: ThemeRegistry,
    pub current_theme_idx: usize,
    pub input_max_len: Entity<InputState>,

    pub physics_enabled: bool,
    pub physics_temperature: f32,
    pub use_barnes_hut: bool,
    pub fa2_lin_log: bool,
    pub fa2_outbound: bool,
    pub fa2_strong_gravity: bool,
    pub fa2_adjust_sizes: bool,
    pub fa2_scaling_ratio: f64,
    pub fa2_stop_mode: usize,
    pub fa2_max_iterations: usize,
    pub input_fa2_scaling: Entity<InputState>,
    pub input_fa2_iterations: Entity<InputState>,
    pub live_sim: LiveForceSimulation,
    pub snapshot_version: u64,

    pub show_performance_hud: bool,
    pub is_layout_running: bool,
    pub telemetry_fps: f64,
    pub telemetry_physics_ms: f64,
    pub telemetry_render_ms: f64,
    pub telemetry_visible_nodes: usize,
    pub telemetry_labels_formatted: usize,
    pub telemetry_is_worker_thread: bool,
    pub telemetry_worker_threads: usize,
    pub telemetry_worker_state: String,
    pub last_frame_instant: std::time::Instant,

    pub is_directed: bool,
    pub active_heatmap: Option<String>,
    pub analysis_report: Option<GraphAnalysisReport>,

    pub collapsed_parents: HashSet<NodeId>,
    pub last_node_click: Option<(NodeId, Instant)>,
    pub last_canvas_click: Option<(gpui::Point<f32>, Instant)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemoConfig {
    pub gravity: f32,
    pub k_rep: f32,
    pub k_att: f32,
    pub iterations: usize,
    pub circle_radius: f32,
    pub theta: f32,
    pub layer_spacing: f32,
    pub node_spacing: f32,
    pub mds_base_dist: f32,
    pub bipartite_col_spacing: f32,
    pub bipartite_vert_spacing: f32,
    pub packer_spacing: f32,
    pub compound_padding: f32,
    pub regional_columns: usize,
    pub regional_cell_size: f32,
    pub max_label_len: usize,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            gravity: 1.0,
            k_rep: 100.0,
            k_att: 0.1,
            iterations: 100,
            circle_radius: 200.0,
            theta: 0.5,
            layer_spacing: 100.0,
            node_spacing: 50.0,
            mds_base_dist: 100.0,
            bipartite_col_spacing: 200.0,
            bipartite_vert_spacing: 60.0,
            packer_spacing: 30.0,
            compound_padding: 20.0,
            regional_columns: 3,
            regional_cell_size: 150.0,
            max_label_len: 20,
        }
    }
}

impl DemoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cfg = DemoConfig::default();
        let default_canvas = CanvasConfig::default();

        let input_gravity = cx.new(|cx| InputState::new(window, cx).default_value("1.0"));
        let input_k_rep = cx.new(|cx| InputState::new(window, cx).default_value("100.0"));
        let input_k_att = cx.new(|cx| InputState::new(window, cx).default_value("0.1"));
        let input_iterations = cx.new(|cx| InputState::new(window, cx).default_value("100"));
        let input_circle_radius = cx.new(|cx| InputState::new(window, cx).default_value("200.0"));
        let input_theta = cx.new(|cx| InputState::new(window, cx).default_value("0.5"));
        let input_layer_spacing = cx.new(|cx| InputState::new(window, cx).default_value("100.0"));
        let input_node_spacing = cx.new(|cx| InputState::new(window, cx).default_value("50.0"));
        let input_mds_base_dist = cx.new(|cx| InputState::new(window, cx).default_value("100.0"));
        let input_bipartite_col_spacing =
            cx.new(|cx| InputState::new(window, cx).default_value("200.0"));
        let input_bipartite_vert_spacing =
            cx.new(|cx| InputState::new(window, cx).default_value("60.0"));
        let input_packer_spacing = cx.new(|cx| InputState::new(window, cx).default_value("30.0"));
        let input_compound_padding = cx.new(|cx| InputState::new(window, cx).default_value("20.0"));
        let input_regional_columns = cx.new(|cx| InputState::new(window, cx).default_value("3"));
        let input_regional_cell_size =
            cx.new(|cx| InputState::new(window, cx).default_value("150.0"));
        let input_grid_spacing = cx.new(|cx| InputState::new(window, cx).default_value("45.0"));
        let input_arrow_length = cx.new(|cx| InputState::new(window, cx).default_value("10.0"));
        let input_arrow_width = cx.new(|cx| InputState::new(window, cx).default_value("8.0"));
        let input_edge_stroke = cx.new(|cx| InputState::new(window, cx).default_value("2.0"));
        let input_edge_curvature = cx.new(|cx| InputState::new(window, cx).default_value("35.0"));

        let input_fa2_scaling = cx.new(|cx| InputState::new(window, cx).default_value("100.0"));
        let input_fa2_iterations = cx.new(|cx| InputState::new(window, cx).default_value("100"));

        let node_name_state = cx.new(|cx| InputState::new(window, cx));
        let edge_src_state = cx.new(|cx| InputState::new(window, cx));
        let edge_tgt_state = cx.new(|cx| InputState::new(window, cx));
        let edge_weight_state = cx.new(|cx| InputState::new(window, cx).default_value("1.0"));
        let input_max_len = cx.new(|cx| InputState::new(window, cx).default_value("20"));

        let fixtures = get_all_fixtures();

        let initial_state = fixtures[0].state.clone();
        let engine = GraphEngineHandle::spawn(initial_state.clone());
        let view = GraphView::from_state(&initial_state);

        let mut app = Self {
            gravity: cfg.gravity,
            k_rep: cfg.k_rep,
            k_att: cfg.k_att,
            iterations: cfg.iterations,
            circle_radius: cfg.circle_radius,
            theta: cfg.theta,
            layer_spacing: cfg.layer_spacing,
            node_spacing: cfg.node_spacing,
            mds_base_dist: cfg.mds_base_dist,
            bipartite_col_spacing: cfg.bipartite_col_spacing,
            bipartite_vert_spacing: cfg.bipartite_vert_spacing,
            packer_spacing: cfg.packer_spacing,
            compound_padding: cfg.compound_padding,
            regional_columns: cfg.regional_columns,
            regional_cell_size: cfg.regional_cell_size,
            max_label_len: cfg.max_label_len,
            grid_spacing: default_canvas.grid_spacing,
            arrow_length: default_canvas.arrow_length,
            arrow_width: default_canvas.arrow_width,
            edge_stroke_width: default_canvas.edge_stroke_width,
            edge_curvature: default_canvas.edge_curvature,

            engine,
            view,
            fixtures,
            selected_fixture_idx: 0,
            selected_layout: "Circle".to_string(),
            expanded_layout: Some("Circle".to_string()),
            viewport: Viewport::new(gpui::Bounds::default()),
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
            input_grid_spacing,
            input_arrow_length,
            input_arrow_width,
            input_edge_stroke,
            input_edge_curvature,
            node_name_state,
            edge_src_state,
            edge_tgt_state,
            edge_weight_state,
            themes: ThemeRegistry::new(),
            current_theme_idx: 3,
            input_max_len,

            physics_enabled: false,
            physics_temperature: 10.0,
            use_barnes_hut: false,
            fa2_lin_log: false,
            fa2_outbound: false,
            fa2_strong_gravity: false,
            fa2_adjust_sizes: true,
            fa2_scaling_ratio: 100.0,
            fa2_stop_mode: 0,
            fa2_max_iterations: 100,
            input_fa2_scaling,
            input_fa2_iterations,
            live_sim: LiveForceSimulation::new(),
            snapshot_version: 0,

            show_performance_hud: true,
            is_layout_running: false,
            telemetry_fps: 60.0,
            telemetry_physics_ms: 0.0,
            telemetry_render_ms: 0.0,
            telemetry_visible_nodes: 0,
            telemetry_labels_formatted: 0,
            telemetry_is_worker_thread: false,
            telemetry_worker_threads: 1,
            telemetry_worker_state: "Idle (Thread Waiting)".to_string(),
            last_frame_instant: std::time::Instant::now(),

            is_directed: true,
            active_heatmap: None,
            analysis_report: None,

            collapsed_parents: std::collections::HashSet::new(),
            last_node_click: None,
            last_canvas_click: None,
        };
        app.load_preset(0, window, cx);
        app
    }

    pub fn drain_updates_and_sync(&mut self) {
        for update in self.engine.drain_updates() {
            if let graphene_layout::GraphUpdate::AnalysisReady(report) = update {
                self.analysis_report = Some(report);
            } else {
                self.view.apply_update(update);
            }
        }
        let snapshot = self.engine.latest_snapshot();
        self.view.sync_positions_from_snapshot(&snapshot);
        self.interaction_state.rebuild_grid(&self.view);
        self.telemetry_worker_threads = self.engine.active_worker_threads();
        self.telemetry_worker_state = self.engine.worker_state().as_str().to_string();
    }

    pub fn get_canvas_config(&self) -> CanvasConfig {
        let mut cfg = CanvasConfig::default();
        cfg.grid_spacing = self.grid_spacing;
        cfg.arrow_length = self.arrow_length;
        cfg.arrow_width = self.arrow_width;
        cfg.edge_stroke_width = self.edge_stroke_width;
        cfg.edge_curvature = self.edge_curvature;
        cfg
    }

    pub fn reset_view(&mut self) {
        self.viewport.offset = Vec2::default();
        self.viewport.zoom = 1.0;
    }

    pub fn toggle_physics(&mut self) {
        self.physics_enabled = !self.physics_enabled;
        if self.physics_enabled {
            self.physics_temperature = 10.0;
            self.reset_physics();
        } else {
            self.engine
                .send_command(graphene_layout::GraphCommand::StopLiveSim)
                .ok();
        }
    }

    pub fn run_analysis(&mut self) {
        self.engine.run_analysis(self.is_directed);
    }

    pub fn get_active_centrality_map(&self) -> Option<&HashMap<NodeId, f32>> {
        let report = self.analysis_report.as_ref()?;
        let metric = self.active_heatmap.as_deref()?;
        match metric {
            "PageRank" => Some(&report.centralities.page_rank),
            "Betweenness" => Some(&report.centralities.betweenness),
            "Degree" => Some(&report.centralities.degree),
            "Closeness" => Some(&report.centralities.closeness),
            _ => None,
        }
    }

    pub fn get_theme(&self) -> Theme {
        let style_theme = &self.themes.themes[self.current_theme_idx];
        Theme::from_style(style_theme)
    }

    pub fn load_preset(&mut self, idx: usize, _window: &mut Window, _cx: &mut Context<Self>) {
        self.selected_fixture_idx = idx;
        let fixture = &self.fixtures[idx];
        let mut preset_state = fixture.state.clone();
        self.is_directed = fixture.is_directed;
        self.selected_node = None;
        self.selected_edge = None;
        self.collapsed_parents.clear();
        self.last_node_click = None;
        self.active_heatmap = None;
        self.analysis_report = None;

        for i in 0..preset_state.node_index_to_id.len() {
            let mut style = ComputedStyle::default();
            if let StylingTarget::Node(ref mut node_style) = style.target {
                node_style.label = Some(i as u32);
                node_style.fill_color =
                    ColorValue::Rgba(137.0 / 255.0, 180.0 / 255.0, 250.0 / 255.0, 1.0);
                node_style.border_color =
                    ColorValue::Rgba(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0, 1.0);
                node_style.border_width = graphene_style::LengthValue::Pixels(2.0);
            }
            preset_state.computed_styles.set(i, style);
        }

        for i in 0..preset_state.edges.len() {
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
            preset_state.edge_computed_styles.set(i, style);
        }

        self.view.load_preset(&preset_state);
        self.live_sim.reset_simulation();
        self.engine.load_preset(preset_state);
        if let Some(cmd) = graphene_layout::LayoutCommand::from_name("Circle", 100) {
            self.engine.run_layout(cmd);
        }
        self.viewport.offset = Vec2::default();
        self.viewport.zoom = 1.0;
        self.physics_temperature = 10.0;
        self.interaction_state.rebuild_grid(&self.view);
    }

    pub fn fit_view(&mut self) {
        self.viewport.fit_to_graph(&self.view);
        self.interaction_state.rebuild_grid(&self.view);
    }

    pub fn trigger_layout(&mut self, _cx: &mut Context<Self>) {
        if self.is_layout_running {
            return;
        }

        if self.physics_enabled {
            self.physics_enabled = false;
            self.engine
                .send_command(graphene_layout::GraphCommand::StopLiveSim)
                .ok();
        }

        self.live_sim.reset_simulation();
        self.run_layout_internal();
    }

    pub fn run_layout_internal(&mut self) {
        self.live_sim.reset_simulation();
        if let Some(cmd) =
            graphene_layout::LayoutCommand::from_name(&self.selected_layout, self.iterations)
        {
            self.engine.run_layout(cmd);
        }
    }

    pub fn get_layout_phases(&self, name: &str) -> Vec<String> {
        use graphene_layout::traits::PhaseSteppableLayout;
        match name {
            "Sugiyama" => PhaseSteppableLayout::<()>::phases(&SugiyamaLayout::default())
                .iter()
                .map(|p| p.to_string())
                .collect(),
            "fCoSE" => PhaseSteppableLayout::<()>::phases(&FCoseLayout::default())
                .iter()
                .map(|p| p.to_string())
                .collect(),
            "CoSE" => PhaseSteppableLayout::<()>::phases(&CoseLayout::default())
                .iter()
                .map(|p| p.to_string())
                .collect(),
            _ => Vec::new(),
        }
    }

    pub fn trigger_step_phase(&mut self, cx: &mut Context<Self>) {
        let layout_cmd = match self.selected_layout.as_str() {
            "Sugiyama" => graphene_layout::LayoutCommand::Sugiyama(
                SugiyamaLayout::default()
                    .with_layer_spacing(self.layer_spacing)
                    .with_node_spacing(self.node_spacing),
            ),
            "fCoSE" => graphene_layout::LayoutCommand::FCose(
                FCoseLayout::default()
                    .with_iterations(self.iterations)
                    .with_gravity(self.gravity),
            ),
            "CoSE" => graphene_layout::LayoutCommand::Cose(
                CoseLayout::default()
                    .with_iterations(self.iterations)
                    .with_gravity(self.gravity),
            ),
            _ => {
                self.trigger_layout(cx);
                return;
            }
        };

        self.engine
            .send_command(graphene_layout::GraphCommand::StepLayoutPhase(layout_cmd))
            .ok();
    }

    pub fn trigger_step_specific_phase(&mut self, phase_idx: usize, cx: &mut Context<Self>) {
        let layout_cmd = match self.selected_layout.as_str() {
            "Sugiyama" => {
                let mut l = SugiyamaLayout::default()
                    .with_layer_spacing(self.layer_spacing)
                    .with_node_spacing(self.node_spacing);
                l.current_phase_idx = phase_idx;
                graphene_layout::LayoutCommand::Sugiyama(l)
            }
            "fCoSE" => {
                let mut l = FCoseLayout::default()
                    .with_iterations(self.iterations)
                    .with_gravity(self.gravity);
                l.current_phase_idx = phase_idx;
                graphene_layout::LayoutCommand::FCose(l)
            }
            "CoSE" => {
                let mut l = CoseLayout::default()
                    .with_iterations(self.iterations)
                    .with_gravity(self.gravity);
                l.current_phase_idx = phase_idx;
                graphene_layout::LayoutCommand::Cose(l)
            }
            _ => {
                self.trigger_layout(cx);
                return;
            }
        };

        self.engine
            .send_command(graphene_layout::GraphCommand::StepLayoutPhase(layout_cmd))
            .ok();
    }

    pub fn add_new_node(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let label = self.node_name_state.read(cx).text().to_string();
        let label = if label.trim().is_empty() {
            format!("Node {}", self.view.nodes.len() + 1)
        } else {
            label
        };
        let center_pos = self.viewport.screen_to_model(gpui::point(400.0, 300.0));
        let mut style = ComputedStyle::default();
        if let StylingTarget::Node(ref mut node_style) = style.target {
            node_style.shape = graphene_style::NodeShape::Ellipse;
            node_style.fill_color =
                ColorValue::Rgba(137.0 / 255.0, 180.0 / 255.0, 250.0 / 255.0, 1.0);
            node_style.border_color =
                ColorValue::Rgba(205.0 / 255.0, 214.0 / 255.0, 244.0 / 255.0, 1.0);
            node_style.border_width = graphene_style::LengthValue::Pixels(2.0);
        }
        self.engine
            .send_command(graphene_layout::GraphCommand::AddNode {
                pos: center_pos,
                size: Size2::new(40.0, 40.0),
                data: style,
                label: Some(label),
            })
            .ok();

        self.run_analysis();

        self.node_name_state.update(cx, |input, cx| {
            input.replace_text_in_range(None, "", window, cx);
        });
    }

    pub fn update_selected_node_label(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.selected_node {
            let label = self.node_name_state.read(cx).text().to_string();
            if !label.trim().is_empty() {
                self.engine
                    .send_command(graphene_layout::GraphCommand::SetNodeLabel {
                        id,
                        label: label.clone(),
                    })
                    .ok();
                self.fixtures[self.selected_fixture_idx]
                    .node_labels
                    .insert(id, label);
                self.run_analysis();
            }
        }
    }

    pub fn delete_selected_node(&mut self) {
        if let Some(id) = self.selected_node {
            self.engine
                .send_command(graphene_layout::GraphCommand::RemoveNode(id))
                .ok();
            self.fixtures[self.selected_fixture_idx]
                .node_labels
                .remove(&id);
            self.selected_node = None;
            self.run_analysis();
        }
    }

    pub fn find_node_by_label_or_id_or_index(&self, query: &str) -> Option<NodeId> {
        let q = query.trim();
        if q.is_empty() {
            return None;
        }

        for (&id, node) in &self.view.nodes {
            if node.label == q || node.label.to_lowercase() == q.to_lowercase() {
                return Some(id);
            }
        }

        let q_lower = q.to_lowercase();
        let digits = if q_lower.starts_with('n') {
            &q_lower[1..]
        } else if q_lower.starts_with("node") {
            q_lower[4..].trim()
        } else {
            &q_lower
        };

        if let Ok(idx) = digits.parse::<usize>() {
            if idx < self.view.node_order.len() {
                return Some(self.view.node_order[idx]);
            }
        }

        None
    }

    pub fn create_edge_between_nodes(&mut self, src: NodeId, tgt: NodeId) {
        if src == tgt {
            return;
        }
        self.engine
            .send_command(graphene_layout::GraphCommand::AddEdge {
                source: src,
                target: tgt,
                data: EdgeData::default(),
            })
            .ok();
        self.run_analysis();
    }

    pub fn add_new_edge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let src_label = self.edge_src_state.read(cx).value().to_string();
        let tgt_label = self.edge_tgt_state.read(cx).value().to_string();

        let src_node = self.find_node_by_label_or_id_or_index(&src_label);
        let tgt_node = self.find_node_by_label_or_id_or_index(&tgt_label);

        if let (Some(src), Some(tgt)) = (src_node, tgt_node) {
            self.create_edge_between_nodes(src, tgt);

            self.edge_src_state.update(cx, |input, cx| {
                let len = input.text().len();
                input.replace_text_in_range(Some(0..len), "", window, cx);
            });
            self.edge_tgt_state.update(cx, |input, cx| {
                let len = input.text().len();
                input.replace_text_in_range(Some(0..len), "", window, cx);
            });
        }
    }

    pub fn get_max_untruncated_len(&self) -> usize {
        self.max_label_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_demo_config_defaults() {
        let cfg = DemoConfig::default();
        assert_eq!(cfg.gravity, 1.0);
        assert_eq!(cfg.iterations, 100);
    }
}
