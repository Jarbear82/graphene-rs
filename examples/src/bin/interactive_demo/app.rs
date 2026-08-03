use crate::theme::Theme;
use gpui::{App, AppContext, AsyncApp, Context, Entity, EntityInputHandler, Window};
use gpui_component::input::{InputEvent, InputState};
use graphene_analysis::GraphAnalysisReport;
use graphene_core::{EdgeData, GraphState, NodeId, Size2, UndoRedoManager, Vec2};
use graphene_fixtures::{get_all_fixtures, GraphFixture};
use graphene_gpui::{
    interaction::state::InteractionState, render::draw_pipeline::Viewport, CanvasConfig,
};
use graphene_layout::{
    CircleLayout, CoseLayout, FCoseLayout, GraphEngineHandle, LayoutCommand, LiveForceSimulation, SugiyamaLayout,
};
use graphene_style::{ColorValue, ComputedStyle, NodeShape, StylingTarget, ThemeRegistry};
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

gpui::actions!(
    graphene_demo,
    [ResetView, TogglePhysics, UndoAction, RedoAction]
);

pub struct DemoApp {
    /// cached, typed values (read by layout code, hot path)
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

    /// widget handles (rendering only, never read-and-parsed again)
    pub engine: GraphEngineHandle<ComputedStyle>,
    pub state: GraphState<ComputedStyle>,
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

    pub is_directed: bool,
    pub active_heatmap: Option<String>,
    pub analysis_report: Option<GraphAnalysisReport>,

    pub undo_redo: UndoRedoManager<ComputedStyle>,
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
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.gravity), window, cx);
            s
        });

        cx.subscribe_in(&input_gravity, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Ok(v) = state.read(cx).value().parse::<f32>() {
                    this.gravity = v;
                }
            }
        })
        .detach();

        let input_k_rep = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.k_rep), window, cx);
            s
        });

        cx.subscribe_in(&input_k_rep, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Ok(v) = state.read(cx).value().parse::<f32>() {
                    this.k_rep = v;
                }
            }
        })
        .detach();

        let input_k_att = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.k_att), window, cx);
            s
        });

        cx.subscribe_in(&input_k_att, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Ok(v) = state.read(cx).value().parse::<f32>() {
                    this.k_att = v;
                }
            }
        })
        .detach();

        let input_iterations = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<usize>().is_ok());
            s.replace_text_in_range(None, &format!("{}", cfg.iterations), window, cx);
            s
        });

        cx.subscribe_in(
            &input_iterations,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<usize>() {
                        this.iterations = v;
                    }
                }
            },
        )
        .detach();

        let input_circle_radius = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.circle_radius), window, cx);
            s
        });

        cx.subscribe_in(
            &input_circle_radius,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.circle_radius = v;
                    }
                }
            },
        )
        .detach();

        let input_theta = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.theta), window, cx);
            s
        });

        cx.subscribe_in(&input_theta, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Ok(v) = state.read(cx).value().parse::<f32>() {
                    this.theta = v;
                }
            }
        })
        .detach();

        let input_layer_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.layer_spacing), window, cx);
            s
        });

        cx.subscribe_in(
            &input_layer_spacing,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.layer_spacing = v;
                    }
                }
            },
        )
        .detach();

        let input_node_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.node_spacing), window, cx);
            s
        });

        cx.subscribe_in(
            &input_node_spacing,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.node_spacing = v;
                    }
                }
            },
        )
        .detach();

        let input_mds_base_dist = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.mds_base_dist), window, cx);
            s
        });

        cx.subscribe_in(
            &input_mds_base_dist,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.mds_base_dist = v;
                    }
                }
            },
        )
        .detach();

        let input_bipartite_col_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(
                None,
                &format!("{:.1}", cfg.bipartite_col_spacing),
                window,
                cx,
            );
            s
        });

        cx.subscribe_in(
            &input_bipartite_col_spacing,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.bipartite_col_spacing = v;
                    }
                }
            },
        )
        .detach();

        let input_bipartite_vert_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(
                None,
                &format!("{:.1}", cfg.bipartite_vert_spacing),
                window,
                cx,
            );
            s
        });

        cx.subscribe_in(
            &input_bipartite_vert_spacing,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.bipartite_vert_spacing = v;
                    }
                }
            },
        )
        .detach();

        let input_packer_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.packer_spacing), window, cx);
            s
        });

        cx.subscribe_in(
            &input_packer_spacing,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.packer_spacing = v;
                    }
                }
            },
        )
        .detach();

        let input_compound_padding = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.compound_padding), window, cx);
            s
        });

        cx.subscribe_in(
            &input_compound_padding,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.compound_padding = v;
                    }
                }
            },
        )
        .detach();

        let input_regional_columns = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<usize>().is_ok());
            s.replace_text_in_range(None, &format!("{}", cfg.regional_columns), window, cx);
            s
        });

        cx.subscribe_in(
            &input_regional_columns,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<usize>() {
                        this.regional_columns = v;
                    }
                }
            },
        )
        .detach();

        let input_regional_cell_size = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, &format!("{:.1}", cfg.regional_cell_size), window, cx);
            s
        });

        cx.subscribe_in(
            &input_regional_cell_size,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.regional_cell_size = v;
                    }
                }
            },
        )
        .detach();

        let input_grid_spacing = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(
                None,
                &format!("{:.1}", default_canvas.grid_spacing),
                window,
                cx,
            );
            s
        });

        cx.subscribe_in(
            &input_grid_spacing,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.grid_spacing = v;
                    }
                }
            },
        )
        .detach();

        let input_arrow_length = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(
                None,
                &format!("{:.1}", default_canvas.arrow_length),
                window,
                cx,
            );
            s
        });

        cx.subscribe_in(
            &input_arrow_length,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.arrow_length = v;
                    }
                }
            },
        )
        .detach();

        let input_arrow_width = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(
                None,
                &format!("{:.1}", default_canvas.arrow_width),
                window,
                cx,
            );
            s
        });

        cx.subscribe_in(
            &input_arrow_width,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.arrow_width = v;
                    }
                }
            },
        )
        .detach();

        let input_edge_stroke = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(
                None,
                &format!("{:.1}", default_canvas.edge_stroke_width),
                window,
                cx,
            );
            s
        });

        cx.subscribe_in(
            &input_edge_stroke,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.edge_stroke_width = v;
                    }
                }
            },
        )
        .detach();

        let input_edge_curvature = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(
                None,
                &format!("{:.1}", default_canvas.edge_curvature),
                window,
                cx,
            );
            s
        });

        cx.subscribe_in(
            &input_edge_curvature,
            window,
            |this, state, event, _window, cx| {
                if let InputEvent::Change = event {
                    if let Ok(v) = state.read(cx).value().parse::<f32>() {
                        this.edge_curvature = v;
                    }
                }
            },
        )
        .detach();

        let node_name_state = cx.new(|cx| {
            InputState::new(window, cx)
        });

        cx.subscribe_in(&node_name_state, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Some(id) = this.selected_node {
                    let label = state.read(cx).value().to_string();
                    if !label.trim().is_empty() {
                        this.state.set_node_label(id, &label);
                        this.fixtures[this.selected_fixture_idx]
                            .node_labels
                            .insert(id, label);
                        this.interaction_state.rebuild_grid(&this.state);
                        cx.notify();
                    }
                }
            }
        })
        .detach();

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
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f32>().is_ok());
            s.replace_text_in_range(None, "1.0", window, cx);
            s
        });

        let input_max_len = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<usize>().is_ok());
            s.replace_text_in_range(None, &format!("{}", cfg.max_label_len), window, cx);
            s
        });

        cx.subscribe_in(&input_max_len, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Ok(v) = state.read(cx).value().parse::<usize>() {
                    this.max_label_len = v;
                }
            }
        })
        .detach();

        let input_fa2_scaling = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<f64>().is_ok());
            s.replace_text_in_range(None, "0.02", window, cx);
            s
        });

        cx.subscribe_in(&input_fa2_scaling, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Ok(v) = state.read(cx).value().parse::<f64>() {
                    this.fa2_scaling_ratio = v;
                }
            }
        })
        .detach();

        let input_fa2_iterations = cx.new(|cx| {
            let mut s = InputState::new(window, cx).validate(|s, _| s.parse::<usize>().is_ok());
            s.replace_text_in_range(None, "100", window, cx);
            s
        });

        cx.subscribe_in(&input_fa2_iterations, window, |this, state, event, _window, cx| {
            if let InputEvent::Change = event {
                if let Ok(v) = state.read(cx).value().parse::<usize>() {
                    this.fa2_max_iterations = v;
                }
            }
        })
        .detach();

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

            engine: GraphEngineHandle::spawn(GraphState::new()),
            state: GraphState::new(),
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
            fa2_scaling_ratio: 0.02,
            fa2_stop_mode: 0,
            fa2_max_iterations: 100,
            input_fa2_scaling,
            input_fa2_iterations,
            live_sim: LiveForceSimulation::new(),
            snapshot_version: 0,

            is_directed: true,
            active_heatmap: None,
            analysis_report: None,

            undo_redo: UndoRedoManager::new(),
            collapsed_parents: std::collections::HashSet::new(),
            last_node_click: None,
            last_canvas_click: None,
        };
        app.load_preset(0, window, cx);
        app
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

        self.engine.load_preset(self.state.clone());
        if let Some(cmd) = graphene_layout::LayoutCommand::from_name("Circle", 100) {
            self.engine.run_layout(cmd);
        }
        self.viewport.offset = Vec2::default();
        self.viewport.zoom = 1.0;
        self.physics_temperature = 10.0;
        self.state.dirty_flags |=
            graphene_core::DirtyFlags::POSITION_DIRTY | graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
        self.interaction_state.rebuild_grid(&self.state);
    }

    pub fn fit_view(&mut self) {
        self.viewport.fit_to_graph(&self.state);
        self.interaction_state.rebuild_grid(&self.state);
    }

    /// Asynchronously runs a layout computation off the UI thread and triggers a smooth 300ms
    /// UI-thread animation transitioning from current to new positions once the background computation completes.
    fn animate_layout_transition<F: FnOnce(&mut Self)>(
        &mut self,
        cx: &mut Context<Self>,
        dispatch_layout: F,
    ) {
        if self.state.node_index_to_id.is_empty() {
            return;
        }

        self.undo_redo.record_state(&self.state);

        // Ensure the background engine has the latest state including any newly created nodes/edges
        self.engine.load_preset(self.state.clone());

        // Capture current positions to animate from
        let start_pos: Vec<Vec2> = self.state.positions.iter().copied().collect();

        // Send layout command to the engine
        dispatch_layout(self);

        // Queue a QueryState command. Because the background engine processes commands sequentially,
        // this guarantees we receive the state exactly after the layout computes.
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = self
            .engine
            .send_command(graphene_layout::GraphCommand::QueryState(tx));

        cx.spawn(async move |this, cx| {
            // Offload the blocking receive channel to the background executor
            let updated_state = cx
                .background_executor()
                .spawn(async move { rx.recv().ok() })
                .await;

            if let Some(target_state) = updated_state {
                // Apply the animation tracks to the UI state
                let _ = this.update(cx, |app, cx| {
                    let target_pos: Vec<Vec2> = target_state.positions.iter().copied().collect();
                    let duration = std::time::Duration::from_millis(300);

                    for (idx, &node_id) in app.state.node_index_to_id.iter().enumerate() {
                        if idx < target_pos.len() {
                            let from_pos = if idx < start_pos.len() {
                                start_pos[idx]
                            } else {
                                app.state.positions[idx]
                            };
                            let to_pos = target_pos[idx];
                            if from_pos != to_pos {
                                app.state.animations.tracks.insert(
                                    node_id,
                                    graphene_core::AnimationTrack::Position {
                                        from: from_pos,
                                        to: to_pos,
                                        duration,
                                        elapsed: std::time::Duration::ZERO,
                                    },
                                );
                            }
                            app.state.positions.set(idx, to_pos);
                        }
                    }
                    app.interaction_state.rebuild_grid(&app.state);
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub fn trigger_layout(&mut self, cx: &mut Context<Self>) {
        self.animate_layout_transition(cx, |app| {
            app.run_layout_internal();
        });
    }

    pub fn run_layout_internal(&mut self) {
        self.engine.load_preset(self.state.clone());
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

        self.animate_layout_transition(cx, move |app| {
            let _ = app
                .engine
                .send_command(graphene_layout::GraphCommand::StepLayoutPhase(layout_cmd));
        });
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

        self.animate_layout_transition(cx, move |app| {
            let _ = app
                .engine
                .send_command(graphene_layout::GraphCommand::StepLayoutPhase(layout_cmd));
        });
    }

    pub fn add_new_node(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let label = self.node_name_state.read(cx).text().to_string();
        let label = if label.trim().is_empty() {
            format!("Node {}", self.state.node_count() + 1)
        } else {
            label
        };
        self.undo_redo.record_state(&self.state);
        let center_screen = gpui::point(400.0, 300.0);
        let id = self.interaction_state.on_double_click(
            center_screen,
            &self.viewport,
            &mut self.state,
            &label,
        );

        self.fixtures[self.selected_fixture_idx]
            .node_labels
            .insert(id, label);
        self.selected_node = Some(id);
        self.selected_edge = None;
        self.run_analysis();

        self.node_name_state.update(cx, |input, cx| {
            input.replace_text_in_range(None, "", window, cx);
        });
    }

    pub fn update_selected_node_label(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.selected_node {
            let label = self.node_name_state.read(cx).text().to_string();
            if !label.trim().is_empty() {
                self.undo_redo.record_state(&self.state);
                self.state.set_node_label(id, &label);
                self.fixtures[self.selected_fixture_idx]
                    .node_labels
                    .insert(id, label);
                self.interaction_state.rebuild_grid(&self.state);
                self.run_analysis();
            }
        }
    }

    pub fn delete_selected_node(&mut self) {
        if let Some(id) = self.selected_node {
            self.undo_redo.record_state(&self.state);
            self.state.remove_node(id);
            self.fixtures[self.selected_fixture_idx].node_labels.remove(&id);
            self.selected_node = None;
            self.interaction_state.rebuild_grid(&self.state);
            self.run_analysis();
        }
    }

    pub fn find_node_by_label_or_id_or_index(&self, query: &str) -> Option<NodeId> {
        let q = query.trim();
        if q.is_empty() {
            return None;
        }

        if let Some(id) = self.state.find_node_by_uuid(q) {
            return Some(id);
        }

        if let Some(id) = self.state.find_node_by_label(q) {
            return Some(id);
        }

        let q_lower = q.to_lowercase();
        let fixture_labels = &self.fixtures[self.selected_fixture_idx].node_labels;

        for (&id, label) in fixture_labels.iter() {
            if label == q || label.to_lowercase() == q_lower {
                return Some(id);
            }
        }

        let digits = if q_lower.starts_with('n') {
            &q_lower[1..]
        } else if q_lower.starts_with("node") {
            q_lower[4..].trim()
        } else {
            &q_lower
        };

        if let Ok(idx) = digits.parse::<usize>() {
            if idx < self.state.node_index_to_id.len() {
                return Some(self.state.node_index_to_id[idx]);
            }
        }

        None
    }

    pub fn create_edge_between_nodes(&mut self, src: NodeId, tgt: NodeId) {
        if src == tgt {
            return;
        }
        let edge_idx = self.state.edges.len();
        self.undo_redo.record_state(&self.state);
        let _edge_id = self.state.add_edge(src, tgt, EdgeData::default());

        self.fixtures[self.selected_fixture_idx]
            .weights
            .insert(edge_idx, 1.0);

        let mut style = ComputedStyle::default();
        if let StylingTarget::Edge(ref mut edge_style) = style.target {
            edge_style.line_color =
                ColorValue::Rgba(166.0 / 255.0, 173.0 / 255.0, 200.0 / 255.0, 1.0);
            edge_style.line_width = graphene_style::LengthValue::Pixels(1.5);
        }
        self.state.edge_computed_styles.set(edge_idx, style);
        self.interaction_state.rebuild_grid(&self.state);
        self.run_analysis();
    }

    pub fn add_new_edge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let src_label = self.edge_src_state.read(cx).value().to_string();
        let tgt_label = self.edge_tgt_state.read(cx).value().to_string();
        let weight_str = self.edge_weight_state.read(cx).value().to_string();
        let w = weight_str.parse::<f32>().unwrap_or(1.0);

        let src_node = self.find_node_by_label_or_id_or_index(&src_label);
        let tgt_node = self.find_node_by_label_or_id_or_index(&tgt_label);

        if let (Some(src), Some(tgt)) = (src_node, tgt_node) {
            let edge_idx = self.state.edges.len();
            self.undo_redo.record_state(&self.state);
            let _edge_id = self.state.add_edge(src, tgt, EdgeData::default());

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
            self.interaction_state.rebuild_grid(&self.state);
            self.run_analysis();

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
