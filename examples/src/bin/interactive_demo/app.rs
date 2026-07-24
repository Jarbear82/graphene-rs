use crate::theme::Theme;
use gpui::{AppContext, Context, Entity, EntityInputHandler, Window};
use gpui_component::input::InputState;
use graphene_analysis::GraphAnalysisReport;
use graphene_core::fixtures::{get_all_fixtures, GraphFixture};
use graphene_core::{EdgeData, GraphState, NodeId, Size2, UndoRedoManager, Vec2};
use graphene_gpui::interaction::state::InteractionState;
use graphene_gpui::render::draw_pipeline::Viewport;
use graphene_gpui::CanvasConfig;
use graphene_layout::{
    BipartiteLayout, CircleLayout, CollisionForceDirectedLayout, CompoundLayout,
    ConcentricHubLayout, DisconnectedPacker, FCoseLayout, ForceDirectedLayout, GridSortedLayout,
    KamadaKawaiLayout, Layout, MdsLayout, RegionalPartitionLayout, ReingoldTilfordLayout,
    SugiyamaLayout, WeightedForceDirectedLayout,
};
use graphene_style::{ColorValue, ComputedStyle, NodeShape, StylingTarget, ThemeRegistry};
use std::collections::HashMap;

gpui::actions!(
    graphene_demo,
    [ResetView, TogglePhysics, UndoAction, RedoAction]
);

pub struct DemoApp {
    pub state: GraphState<ComputedStyle>,
    pub fixtures: Vec<GraphFixture<ComputedStyle>>,
    pub selected_fixture_idx: usize,
    pub selected_layout: String,

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

    pub is_directed: bool,
    pub active_heatmap: Option<String>,
    pub analysis_report: Option<GraphAnalysisReport>,

    pub undo_redo: UndoRedoManager<ComputedStyle>,
    pub collapsed_parents: std::collections::HashSet<NodeId>,
    pub last_node_click: Option<(NodeId, std::time::Instant)>,
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
    pub physics_initial_temp: f32,
}

impl Default for DemoConfig {
    fn default() -> Self {
        Self {
            gravity: 1.0,
            k_rep: 30.0,
            k_att: 30.0,
            iterations: 100,
            circle_radius: 150.0,
            theta: 0.5,
            layer_spacing: 80.0,
            node_spacing: 60.0,
            mds_base_dist: 50.0,
            bipartite_col_spacing: 120.0,
            bipartite_vert_spacing: 60.0,
            packer_spacing: 80.0,
            compound_padding: 20.0,
            regional_columns: 2,
            regional_cell_size: 250.0,
            max_label_len: 10,
            physics_initial_temp: 10.0,
        }
    }
}

impl DemoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let cfg = DemoConfig::default();
        let default_canvas = CanvasConfig::default();
        let fixtures = get_all_fixtures::<ComputedStyle>();

        let input_gravity = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.gravity), window, cx);
            s
        });
        let input_k_rep = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.k_rep), window, cx);
            s
        });
        let input_k_att = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.k_att), window, cx);
            s
        });
        let input_iterations = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{}", cfg.iterations), window, cx);
            s
        });
        let input_circle_radius = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.circle_radius), window, cx);
            s
        });
        let input_theta = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.theta), window, cx);
            s
        });
        let input_layer_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.layer_spacing), window, cx);
            s
        });
        let input_node_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.node_spacing), window, cx);
            s
        });
        let input_mds_base_dist = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.mds_base_dist), window, cx);
            s
        });
        let input_bipartite_col_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.bipartite_col_spacing), window, cx);
            s
        });
        let input_bipartite_vert_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.bipartite_vert_spacing), window, cx);
            s
        });
        let input_packer_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.packer_spacing), window, cx);
            s
        });
        let input_compound_padding = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.compound_padding), window, cx);
            s
        });
        let input_regional_columns = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{}", cfg.regional_columns), window, cx);
            s
        });
        let input_regional_cell_size = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", cfg.regional_cell_size), window, cx);
            s
        });

        let input_grid_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", default_canvas.grid_spacing), window, cx);
            s
        });
        let input_arrow_length = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", default_canvas.arrow_length), window, cx);
            s
        });
        let input_arrow_width = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", default_canvas.arrow_width), window, cx);
            s
        });
        let input_edge_stroke = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", default_canvas.edge_stroke_width), window, cx);
            s
        });
        let input_edge_curvature = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{:.1}", default_canvas.edge_curvature), window, cx);
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
        let input_max_len = cx.new(|cx| {
            let mut s = InputState::new(window, cx);
            s.replace_text_in_range(None, &format!("{}", cfg.max_label_len), window, cx);
            s
        });

        let mut app = Self {
            state: GraphState::new(),
            fixtures,
            selected_fixture_idx: 0,
            selected_layout: "Circle".to_string(),
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

            physics_enabled: true,
            physics_temperature: 10.0,
            use_barnes_hut: false,

            is_directed: true,
            active_heatmap: None,
            analysis_report: None,

            undo_redo: UndoRedoManager::new(),
            collapsed_parents: std::collections::HashSet::new(),
            last_node_click: None,
        };
        app.load_preset(0, window, cx);
        app
    }

    pub fn get_canvas_config(&self, cx: &Context<Self>) -> CanvasConfig {
        let mut cfg = CanvasConfig::default();
        cfg.grid_spacing = self.input_grid_spacing.read(cx).text().to_string().parse().unwrap_or(45.0);
        cfg.arrow_length = self.input_arrow_length.read(cx).text().to_string().parse().unwrap_or(10.0);
        cfg.arrow_width = self.input_arrow_width.read(cx).text().to_string().parse().unwrap_or(8.0);
        cfg.edge_stroke_width = self.input_edge_stroke.read(cx).text().to_string().parse().unwrap_or(2.0);
        cfg.edge_curvature = self.input_edge_curvature.read(cx).text().to_string().parse().unwrap_or(35.0);
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
        }
    }

    pub fn run_analysis(&mut self) {
        self.analysis_report = Some(GraphAnalysisReport::analyze(&self.state, self.is_directed));
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
        self.state = fixture.state.clone();
        self.is_directed = fixture.is_directed;
        self.selected_node = None;
        self.selected_edge = None;
        self.collapsed_parents.clear();
        self.last_node_click = None;
        self.active_heatmap = None;
        self.analysis_report = None;

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
        self.run_analysis();
    }

    pub fn fit_view(&mut self) {
        self.viewport.fit_to_graph(&self.state);
        self.interaction_state.rebuild_grid(&self.state);
    }

    pub fn trigger_layout(&mut self, cx: &mut Context<Self>) {
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
                self.state.animations.tracks.insert(
                    node_id,
                    graphene_core::AnimationTrack::Position {
                        from: start_pos[idx],
                        to: target_pos[idx],
                        duration,
                        elapsed: std::time::Duration::ZERO,
                    },
                );
            }
        }
        cx.notify();
    }

    pub fn run_layout_internal(&mut self, cx: &mut Context<Self>) {
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
                graphene_layout::compute_flat_layout(
                    &mut circle,
                    &mut self.state,
                    &self.collapsed_parents,
                );
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
                graphene_layout::compute_flat_layout(
                    &mut force,
                    &mut self.state,
                    &self.collapsed_parents,
                );
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
                graphene_layout::compute_flat_layout(
                    &mut cose,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "KamadaKawai" => {
                let mut kk = KamadaKawaiLayout {
                    iterations,
                    k: 1.0,
                    l_0: 50.0,
                };
                graphene_layout::compute_flat_layout(
                    &mut kk,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "Sugiyama" => {
                let mut sugi = SugiyamaLayout {
                    layer_spacing,
                    node_spacing,
                };
                graphene_layout::compute_flat_layout(
                    &mut sugi,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "ReingoldTilford" => {
                let mut rt = ReingoldTilfordLayout::default();
                graphene_layout::compute_flat_layout(
                    &mut rt,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "MDS" => {
                let mut mds = MdsLayout {
                    iterations,
                    base_dist: mds_base_dist,
                };
                graphene_layout::compute_flat_layout(
                    &mut mds,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "Grid" => {
                let mut grid = GridSortedLayout::default();
                graphene_layout::compute_flat_layout(
                    &mut grid,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "Concentric" => {
                let mut concentric = ConcentricHubLayout::default();
                graphene_layout::compute_flat_layout(
                    &mut concentric,
                    &mut self.state,
                    &self.collapsed_parents,
                );
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
                graphene_layout::compute_flat_layout(
                    &mut bipartite,
                    &mut self.state,
                    &self.collapsed_parents,
                );
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
                graphene_layout::compute_flat_layout(
                    &mut weighted,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "CollisionForce" => {
                let mut collision = CollisionForceDirectedLayout {
                    iterations,
                    gravity,
                    ideal_length: 50.0,
                };
                graphene_layout::compute_flat_layout(
                    &mut collision,
                    &mut self.state,
                    &self.collapsed_parents,
                );
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
                graphene_layout::compute_flat_layout(
                    &mut packer,
                    &mut self.state,
                    &self.collapsed_parents,
                );
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
                graphene_layout::compute_flat_layout(
                    &mut comp,
                    &mut self.state,
                    &self.collapsed_parents,
                );
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
                graphene_layout::compute_flat_layout(
                    &mut regional,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            "fCoSE" => {
                let mut fcose = FCoseLayout::default();
                graphene_layout::compute_flat_layout(
                    &mut fcose,
                    &mut self.state,
                    &self.collapsed_parents,
                );
            }
            _ => {}
        }
        self.state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }

    pub fn add_new_node(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
        self.run_analysis();

        self.node_name_state.update(cx, |input, cx| {
            input.replace_text_in_range(None, "", window, cx);
        });
    }

    pub fn delete_selected_node(&mut self) {
        if let Some(id) = self.selected_node {
            self.undo_redo.record_state(&self.state);
            self.state.remove_node(id);
            self.selected_node = None;
            self.state.dirty_flags |= graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
            self.interaction_state.rebuild_grid(&self.state);
            self.run_analysis();
        }
    }

    pub fn add_new_edge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
            self.run_analysis();

            self.edge_src_state.update(cx, |input, cx| {
                input.replace_text_in_range(None, "", window, cx);
            });
            self.edge_tgt_state.update(cx, |input, cx| {
                input.replace_text_in_range(None, "", window, cx);
            });
        }
    }

    pub fn get_max_untruncated_len(&self, cx: &Context<Self>) -> usize {
        self.input_max_len
            .read(cx)
            .text()
            .to_string()
            .parse::<usize>()
            .unwrap_or(10)
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
