use gpui::prelude::*;
use gpui::{px, IntoElement, PathBuilder, Point, SharedString, Styled};
use graphene_core::{GraphState, NodeId};
use graphene_style::{ColorValue, ComputedStyle, EdgeCurveStyle, NodeShape, StylingTarget, Theme};
use crate::render::draw_pipeline::Viewport;
use crate::interaction::state::InteractionState;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CanvasConfig {
    pub grid_spacing: f32,
    pub edge_stroke_width: f32,
    pub arrow_length: f32,
    pub arrow_width: f32,
    pub edge_curvature: f32,
    pub node_border_width: f32,
    pub node_font_size: f32,
    pub edge_label_font_size: f32,
    pub edge_label_width: f32,
    pub edge_label_height: f32,
    pub compound_fill_alpha: f32,
    pub compound_border_alpha: f32,
    pub hide_edges_during_pan: bool,
    pub min_visible_font_size: f32,
}

impl Default for CanvasConfig {
    fn default() -> Self {
        Self {
            grid_spacing: 45.0,
            edge_stroke_width: 2.0,
            arrow_length: 10.0,
            arrow_width: 8.0,
            edge_curvature: 35.0,
            node_border_width: 2.0,
            node_font_size: 10.0,
            edge_label_font_size: 12.0,
            edge_label_width: 60.0,
            edge_label_height: 16.0,
            compound_fill_alpha: 0.08,
            compound_border_alpha: 0.4,
            hide_edges_during_pan: false,
            min_visible_font_size: 4.0,
        }
    }
}

pub fn color_to_gpui(val: ColorValue) -> gpui::Rgba {
    match val {
        ColorValue::Rgba(r, g, b, a) => gpui::rgba(
            ((r * 255.0) as u32) << 24
                | ((g * 255.0) as u32) << 16
                | ((b * 255.0) as u32) << 8
                | (a * 255.0) as u32,
        ),
    }
}

pub fn heatmap_color(val: f32) -> gpui::Rgba {
    let clamped = val.clamp(0.0, 1.0);
    let r = (clamped * 255.0) as u32;
    let g = ((1.0 - (clamped - 0.5).abs() * 2.0) * 200.0) as u32;
    let b = ((1.0 - clamped) * 255.0) as u32;
    gpui::rgba((r << 24) | (g << 16) | (b << 8) | 255)
}

#[derive(IntoElement)]
pub struct GraphNodeElement {
    pub id: SharedString,
    pub screen_x: f32,
    pub screen_y: f32,
    pub width: f32,
    pub height: f32,
    pub border_width: f32,
    pub border_color: gpui::Rgba,
    pub fill_color: gpui::Rgba,
    pub shape: NodeShape,
    pub text_color: gpui::Rgba,
    pub font_size: f32,
    pub label: String,
}

impl RenderOnce for GraphNodeElement {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        gpui::div()
            .id(self.id)
            .absolute()
            .left(px(self.screen_x))
            .top(px(self.screen_y))
            .w(px(self.width))
            .h(px(self.height))
            .border(px(self.border_width))
            .border_color(self.border_color)
            .bg(self.fill_color)
            .cursor_pointer()
            .when(self.shape == NodeShape::Ellipse, |d| d.rounded_full())
            .when(self.shape == NodeShape::Rectangle, |d| d.rounded_none())
            .when(self.shape == NodeShape::Square, |d| d.rounded_sm())
            .when(self.shape == NodeShape::Diamond, |d| d.rounded_tl_full().rounded_br_full())
            .when(self.shape == NodeShape::Triangle, |d| d.rounded_t_full().rounded_b_none())
            .when(self.shape == NodeShape::Pentagon, |d| d.rounded_t_xl().rounded_b_sm())
            .when(self.shape == NodeShape::Hexagon, |d| d.rounded_t_lg().rounded_b_lg())
            .when(self.shape == NodeShape::Octagon, |d| d.rounded_2xl())
            .when(self.shape == NodeShape::Star, |d| d.rounded_tr_full().rounded_bl_full())
            .when(self.shape == NodeShape::Ribbon, |d| d.rounded_b_full().rounded_t_none())
            .flex()
            .items_center()
            .justify_center()
            .child(
                gpui::div()
                    .text_color(self.text_color)
                    .text_size(px(self.font_size))
                    .child(self.label),
            )
    }
}

#[derive(IntoElement)]
pub struct GraphEdgeLabelElement {
    pub id: SharedString,
    pub screen_x: f32,
    pub screen_y: f32,
    pub width: f32,
    pub height: f32,
    pub text_color: gpui::Rgba,
    pub font_size: f32,
    pub label: String,
}

impl RenderOnce for GraphEdgeLabelElement {
    fn render(self, _window: &mut gpui::Window, _cx: &mut gpui::App) -> impl IntoElement {
        gpui::div()
            .id(self.id)
            .absolute()
            .left(px(self.screen_x))
            .top(px(self.screen_y))
            .w(px(self.width))
            .h(px(self.height))
            .flex()
            .items_center()
            .justify_center()
            .child(
                gpui::div()
                    .text_color(self.text_color)
                    .text_size(px(self.font_size))
                    .child(self.label),
            )
    }
}

pub struct GraphCanvas<'a> {
    pub state: &'a GraphState<ComputedStyle>,
    pub viewport: &'a Viewport,
    pub interaction_state: &'a InteractionState,
    pub theme: &'a Theme,
    pub selected_node: Option<NodeId>,
    pub node_labels: &'a std::collections::HashMap<NodeId, String>,
    pub edge_labels: &'a std::collections::HashMap<usize, String>,
    pub max_untruncated_len: usize,
    pub collapsed_parents: &'a std::collections::HashSet<NodeId>,
    pub is_directed: bool,
    pub centrality_scores: Option<&'a HashMap<NodeId, f32>>,
    pub config: CanvasConfig,
}

impl<'a> GraphCanvas<'a> {
    pub fn new(
        state: &'a GraphState<ComputedStyle>,
        viewport: &'a Viewport,
        interaction_state: &'a InteractionState,
        theme: &'a Theme,
        selected_node: Option<NodeId>,
        node_labels: &'a std::collections::HashMap<NodeId, String>,
        edge_labels: &'a std::collections::HashMap<usize, String>,
        max_untruncated_len: usize,
        collapsed_parents: &'a std::collections::HashSet<NodeId>,
    ) -> Self {
        Self {
            state,
            viewport,
            interaction_state,
            theme,
            selected_node,
            node_labels,
            edge_labels,
            max_untruncated_len,
            collapsed_parents,
            is_directed: true,
            centrality_scores: None,
            config: CanvasConfig::default(),
        }
    }

    pub fn with_directed(mut self, is_directed: bool) -> Self {
        self.is_directed = is_directed;
        self
    }

    pub fn with_centrality_scores(mut self, scores: Option<&'a HashMap<NodeId, f32>>) -> Self {
        self.centrality_scores = scores;
        self
    }

    pub fn with_config(mut self, config: CanvasConfig) -> Self {
        self.config = config;
        self
    }
}

impl<'a> IntoElement for GraphCanvas<'a> {
    type Element = gpui::AnyElement;

    fn into_element(self) -> Self::Element {
        let state = self.state;
        let viewport = self.viewport.clone();
        let theme = *self.theme;
        let selected_node = self.selected_node;
        let node_labels = self.node_labels.clone();
        let edge_labels = self.edge_labels.clone();
        let max_untruncated_len = self.max_untruncated_len;
        let collapsed_parents = self.collapsed_parents;
        let is_directed = self.is_directed;
        let centrality_scores = self.centrality_scores.cloned();
        let cfg = self.config;

        let edge_color = color_to_gpui(theme.edge_color);
        let text_color = color_to_gpui(theme.text);
        let accent_color = color_to_gpui(theme.accent);
        let node_fill_color = color_to_gpui(theme.node_fill);
        let node_border_color = color_to_gpui(theme.node_border);

        let get_visible_rep = |mut curr: NodeId| -> NodeId {
            let mut rep = curr;
            while let Some(&idx) = state.node_keys.get(curr) {
                if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
                    if collapsed_parents.contains(&parent_id) {
                        rep = parent_id;
                    }
                    curr = parent_id;
                } else {
                    break;
                }
            }
            rep
        };

        let mut connected_nodes = std::collections::HashSet::new();
        let mut connected_edge_indices = std::collections::HashSet::new();

        if let Some(sel_id) = selected_node {
            connected_nodes.insert(sel_id);
            for i in 0..state.edges.len() {
                let src = *state.edge_sources.get(i);
                let tgt = *state.edge_targets.get(i);
                let src_rep = get_visible_rep(src);
                let tgt_rep = get_visible_rep(tgt);
                if src_rep == sel_id || tgt_rep == sel_id {
                    connected_edge_indices.insert(i);
                    connected_nodes.insert(src_rep);
                    connected_nodes.insert(tgt_rep);
                }
            }
        }

        // Precompute edge paths for drawing
        let mut edge_paths = Vec::new();
        let mut edge_labels_to_render = Vec::new();
        let is_panning_active = self.interaction_state.drag_start.is_some() || self.interaction_state.pan_origin.is_some();
        let skip_edges = cfg.hide_edges_during_pan && is_panning_active;

        if !skip_edges {
            for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);

            let src_rep = get_visible_rep(src);
            let tgt_rep = get_visible_rep(tgt);

            if src_rep == tgt_rep {
                continue; // Hidden internal edge
            }

            let (Some(&src_idx), Some(&tgt_idx)) = (state.node_keys.get(src_rep), state.node_keys.get(tgt_rep)) else {
                continue;
            };
            let pos_src = *state.positions.get(src_idx);
            let pos_tgt = *state.positions.get(tgt_idx);
            let tgt_size = *state.sizes.get(tgt_idx);

            let src_screen = viewport.model_to_screen(pos_src);

            let clipped_tgt = graphene_layout::find_clipping_point(
                pos_tgt,
                tgt_size,
                pos_src.x - pos_tgt.x,
                pos_src.y - pos_tgt.y,
            );
            let tgt_screen = viewport.model_to_screen(clipped_tgt);

            let mut curve_style = EdgeCurveStyle::Straight;
            let mut label_text = edge_labels.get(&i).cloned();

            let style = state.edge_computed_styles.get(i);
            if let StylingTarget::Edge(ref edge_style) = style.target {
                if !edge_style.visible {
                    continue;
                }
                curve_style = edge_style.curve_style;
                if label_text.is_none() {
                    if let Some(lbl_id) = edge_style.label {
                        label_text = state.string_arena.get(lbl_id).map(|s| s.to_string());
                    }
                }
            }

            let screen_curve_style = match curve_style {
                EdgeCurveStyle::UnbundledBezier(cp1, cp2) => {
                    let s1 = viewport.model_to_screen(cp1);
                    let s2 = viewport.model_to_screen(cp2);
                    EdgeCurveStyle::UnbundledBezier(
                        graphene_core::math::Vec2::new(s1.x, s1.y),
                        graphene_core::math::Vec2::new(s2.x, s2.y),
                    )
                }
                other => other,
            };

            let (cur_edge_color, stroke_width) = if let Some(_sel_id) = selected_node {
                if connected_edge_indices.contains(&i) {
                    (accent_color, cfg.edge_stroke_width * 1.5)
                } else {
                    let mut faded = edge_color;
                    faded.a = 0.12;
                    (faded, cfg.edge_stroke_width)
                }
            } else {
                (edge_color, cfg.edge_stroke_width)
            };

            edge_paths.push((src_screen, tgt_screen, screen_curve_style, cur_edge_color, stroke_width));

            if let Some(lbl) = label_text {
                if !lbl.is_empty() {
                    edge_labels_to_render.push((i, src_screen, tgt_screen, curve_style, lbl));
                }
            }
        }
        }

        let nodes_count = state.node_index_to_id.len();

        let mut parent_indices = Vec::new();
        let mut leaf_indices = Vec::new();
        for idx in 0..nodes_count {
            let id = state.node_index_to_id[idx];
            if get_visible_rep(id) != id {
                continue; // Hidden descendant
            }

            let is_parent = state.hierarchy.first_child.get(idx).is_some();

            if is_parent {
                parent_indices.push(idx);
            } else {
                leaf_indices.push(idx);
            }
        }

        let render_node = |idx: usize| -> Option<gpui::AnyElement> {
            let id = state.node_index_to_id[idx];
            let pos = *state.positions.get(idx);
            let size_val = *state.sizes.get(idx);

            // Frustum Culling: Skip building and rendering nodes outside active viewport
            if !viewport.is_visible(pos, size_val) {
                return None;
            }

            let mut label = node_labels.get(&id)
                .cloned()
                .or_else(|| state.get_node_label(id).map(|s| s.to_string()))
                .unwrap_or_else(|| format!("N{}", idx));

            let is_compound = {
                let id = state.node_index_to_id[idx];
                let mut found = false;
                for j in 0..nodes_count {
                    let child_id = state.node_index_to_id[j];
                    if let Some(p_id) = *state.hierarchy.parent.get(j) {
                        if p_id == id {
                            if get_visible_rep(child_id) == child_id {
                                found = true;
                                break;
                            }
                        }
                    }
                }
                found
            };
            let is_collapsed = collapsed_parents.contains(&id);

            if is_compound {
                if is_collapsed {
                    label = format!("[+] {}", label);
                } else {
                    label = format!("[-] {}", label);
                }
            }

            let is_primary = state.selected.is_primary(id) || selected_node == Some(id);
            let is_secondary = state.selected.is_secondary(id);
            let is_selected = is_primary || is_secondary;
            let has_selection = state.selected.primary_node().is_some() || selected_node.is_some();
            let is_neighbor = has_selection && connected_nodes.contains(&id);
            let is_faded = has_selection && !is_selected && !is_neighbor;

            let effective_font_size = cfg.node_font_size * viewport.zoom;
            if effective_font_size < cfg.min_visible_font_size {
                label = String::new();
            } else if label.chars().count() > max_untruncated_len && !is_selected {
                label = label.chars().take(max_untruncated_len).collect::<String>() + "...";
            }

            let mut scale = 1.0f32;
            let score_opt = centrality_scores.as_ref().and_then(|m| m.get(&id).copied());

            let mut node_w = size_val.w * viewport.zoom;
            let mut node_h = size_val.h * viewport.zoom;

            if let Some(score) = score_opt {
                scale = 0.8 + score * 0.8;
                node_w *= scale;
                node_h *= scale;
            }

            let screen_x = (pos.x + viewport.offset.x) * viewport.zoom + viewport.bounds.size.width / 2.0 - (node_w / 2.0);
            let screen_y = (pos.y + viewport.offset.y) * viewport.zoom + viewport.bounds.size.height / 2.0 - (node_h / 2.0);

            let mut shape = if is_compound {
                NodeShape::Rectangle
            } else {
                NodeShape::Ellipse
            };

            let mut fill_color = if let Some(score) = score_opt {
                heatmap_color(score)
            } else if is_primary {
                accent_color
            } else if is_secondary {
                gpui::rgba(0xf9e2af_ff)
            } else if is_compound {
                let mut col = accent_color;
                col.a = cfg.compound_fill_alpha;
                col
            } else {
                node_fill_color
            };

            let mut border_color = if is_primary || is_neighbor {
                accent_color
            } else if is_secondary {
                gpui::rgba(0xf9e2af_ff)
            } else if is_compound {
                let mut col = accent_color;
                col.a = cfg.compound_border_alpha;
                col
            } else {
                node_border_color
            };

            if score_opt.is_none() && idx < state.computed_styles.len() {
                if let StylingTarget::Node(node_style) = state.computed_styles.get(idx).target {
                    if !is_compound {
                        fill_color = color_to_gpui(node_style.fill_color);
                        border_color = color_to_gpui(node_style.border_color);
                        shape = node_style.shape;
                    }
                }
            }

            if is_selected || is_neighbor {
                border_color = accent_color;
            }

            let mut cur_text_color = text_color;

            if is_faded {
                fill_color.a *= 0.20;
                border_color.a *= 0.20;
                cur_text_color.a *= 0.20;
            }

            Some(GraphNodeElement {
                id: SharedString::from(format!("canvas-node-{}", idx)),
                screen_x,
                screen_y,
                width: node_w,
                height: node_h,
                border_width: cfg.node_border_width,
                border_color,
                fill_color,
                shape,
                text_color: cur_text_color,
                font_size: cfg.node_font_size * viewport.zoom * scale,
                label,
            }.into_any_element())
        };

        let render_edge_label = |(i, src_p, tgt_p, curve_style, label): (usize, Point<f32>, Point<f32>, EdgeCurveStyle, String)| {
            let src_x = f32::from(src_p.x);
            let src_y = f32::from(src_p.y);
            let tgt_x = f32::from(tgt_p.x);
            let tgt_y = f32::from(tgt_p.y);

            let screen_curve = match curve_style {
                EdgeCurveStyle::UnbundledBezier(cp1, cp2) => {
                    let p1 = viewport.model_to_screen(cp1);
                    let p2 = viewport.model_to_screen(cp2);
                    EdgeCurveStyle::UnbundledBezier(
                        graphene_core::Vec2::new(p1.x, p1.y),
                        graphene_core::Vec2::new(p2.x, p2.y),
                    )
                }
                other => other,
            };
            let mid = graphene_layout::compute_curve_midpoint(
                graphene_core::Vec2::new(src_x, src_y),
                graphene_core::Vec2::new(tgt_x, tgt_y),
                screen_curve,
                cfg.edge_curvature,
            );
            let (mid_x, mid_y) = (mid.x, mid.y);

            let font_size = match state.edge_computed_styles.get(i).target {
                StylingTarget::Edge(edge_style) => edge_style.label_font_size,
                _ => cfg.edge_label_font_size,
            };

            let label_w = cfg.edge_label_width * viewport.zoom;
            let label_h = cfg.edge_label_height * viewport.zoom;
            let screen_x = mid_x - (label_w / 2.0);
            let screen_y = mid_y - (label_h / 2.0);

            GraphEdgeLabelElement {
                id: SharedString::from(format!("canvas-edge-label-{}", i)),
                screen_x,
                screen_y,
                width: label_w,
                height: label_h,
                text_color,
                font_size: font_size * viewport.zoom,
                label,
            }
        };

        let graph_type_badge = if is_directed {
            "DIRECTED GRAPH"
        } else {
            "UNDIRECTED GRAPH"
        };

        gpui::div()
            .flex_1()
            .h_full()
            .relative()
            .overflow_hidden()
            .child(
                gpui::canvas(
                    move |_, _, _| {},
                    move |_bounds, _, window, _| {
                        let origin_x = f32::from(_bounds.origin.x);
                        let origin_y = f32::from(_bounds.origin.y);
                        let width = f32::from(_bounds.size.width);
                        let height = f32::from(_bounds.size.height);

                        // Draw Grid
                        let grid_spacing = cfg.grid_spacing;
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

                        // Draw Edges and Arrowheads
                        for (src_p, tgt_p, curve_style, cur_edge_color, stroke_width) in &edge_paths {
                            let mut builder = PathBuilder::stroke(px(*stroke_width));
                            builder.move_to(gpui::point(px(src_p.x), px(src_p.y)));

                            match curve_style {
                                EdgeCurveStyle::Straight => {
                                    builder.line_to(gpui::point(px(tgt_p.x), px(tgt_p.y)));
                                }
                                EdgeCurveStyle::Taxi => {
                                    let (wp1, wp2) = graphene_layout::compute_taxi_path(
                                        graphene_core::Vec2::new(src_p.x, src_p.y),
                                        graphene_core::Vec2::new(tgt_p.x, tgt_p.y),
                                    );
                                    builder.line_to(gpui::point(px(wp1.x), px(wp1.y)));
                                    builder.line_to(gpui::point(px(wp2.x), px(wp2.y)));
                                    builder.line_to(gpui::point(px(tgt_p.x), px(tgt_p.y)));
                                }
                                EdgeCurveStyle::UnbundledBezier(cp1, cp2) => {
                                    builder.curve_to(gpui::point(px(cp1.x), px(cp1.y)), gpui::point(px(cp2.x), px(cp2.y)));
                                    builder.line_to(gpui::point(px(tgt_p.x), px(tgt_p.y)));
                                }
                                _ => {
                                    let mid_x = (src_p.x + tgt_p.x) / 2.0;
                                    let mid_y = (src_p.y + tgt_p.y) / 2.0;
                                    let dx = tgt_p.x - src_p.x;
                                    let dy = tgt_p.y - src_p.y;
                                    let len = (dx * dx + dy * dy).sqrt();
                                    let curvature = cfg.edge_curvature;
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
                                    builder.curve_to(ctrl, gpui::point(px(tgt_p.x), px(tgt_p.y)));
                                }
                            }
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, *cur_edge_color);
                            }

                            // Render Directed Arrowhead
                            if is_directed {
                                let dx = tgt_p.x - src_p.x;
                                let dy = tgt_p.y - src_p.y;
                                let len = (dx * dx + dy * dy).sqrt();
                                if len > 0.1 {
                                    let u_x = dx / len;
                                    let u_y = dy / len;
                                    let v_x = -u_y;
                                    let v_y = u_x;

                                    let arrow_len = cfg.arrow_length * viewport.zoom;
                                    let arrow_width = cfg.arrow_width * viewport.zoom;

                                    let base_x = tgt_p.x - u_x * arrow_len;
                                    let base_y = tgt_p.y - u_y * arrow_len;

                                    let p1 = Point {
                                        x: px(base_x + v_x * arrow_width / 2.0),
                                        y: px(base_y + v_y * arrow_width / 2.0),
                                    };
                                    let p2 = Point {
                                        x: px(base_x - v_x * arrow_width / 2.0),
                                        y: px(base_y - v_y * arrow_width / 2.0),
                                    };

                                    let mut arr_builder = PathBuilder::fill();
                                    arr_builder.move_to(gpui::point(px(tgt_p.x), px(tgt_p.y)));
                                    arr_builder.line_to(p1);
                                    arr_builder.line_to(p2);
                                    arr_builder.line_to(gpui::point(px(tgt_p.x), px(tgt_p.y)));
                                    if let Ok(arr_path) = arr_builder.build() {
                                        window.paint_path(arr_path, edge_color);
                                    }
                                }
                            }
                        }
                    }
                )
                .size_full()
                .absolute()
            )
            .child(
                gpui::div()
                    .absolute()
                    .top_3()
                    .right_3()
                    .px_2()
                    .py_1()
                    .bg(color_to_gpui(theme.panel_bg))
                    .border(px(1.0))
                    .border_color(color_to_gpui(theme.accent))
                    .rounded_md()
                    .text_color(color_to_gpui(theme.accent))
                    .text_size(px(10.0))
                    .font_weight(gpui::FontWeight::BOLD)
                    .child(graph_type_badge),
            )
            .children(parent_indices.into_iter().filter_map(render_node))
            .children(leaf_indices.into_iter().filter_map(render_node))
            .children(edge_labels_to_render.into_iter().map(render_edge_label))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_node_element_construction() {
        let elem = GraphNodeElement {
            id: SharedString::from("test-node-1"),
            screen_x: 100.0,
            screen_y: 200.0,
            width: 50.0,
            height: 50.0,
            border_width: 2.0,
            border_color: gpui::rgba(0xff0000ff),
            fill_color: gpui::rgba(0x00ff00ff),
            shape: NodeShape::Ellipse,
            text_color: gpui::rgba(0xffffffff),
            font_size: 12.0,
            label: "Node 1".to_string(),
        };
        assert_eq!(elem.label, "Node 1");
        assert_eq!(elem.screen_x, 100.0);
        assert_eq!(elem.screen_y, 200.0);
        assert_eq!(elem.shape, NodeShape::Ellipse);
    }

    #[test]
    fn test_graph_edge_label_element_construction() {
        let elem = GraphEdgeLabelElement {
            id: SharedString::from("test-edge-1"),
            screen_x: 50.0,
            screen_y: 75.0,
            width: 60.0,
            height: 20.0,
            text_color: gpui::rgba(0x0000ffff),
            font_size: 10.0,
            label: "Edge A->B".to_string(),
        };
        assert_eq!(elem.label, "Edge A->B");
        assert_eq!(elem.width, 60.0);
        assert_eq!(elem.height, 20.0);
    }

    #[test]
    fn test_color_to_gpui_conversion() {
        let color_val = ColorValue::Rgba(1.0, 0.0, 0.0, 1.0);
        let gpui_color = color_to_gpui(color_val);
        assert_eq!(gpui_color, gpui::rgba(0xff0000ff));
    }

    #[test]
    fn test_heatmap_color_bounds() {
        let min_color = heatmap_color(0.0);
        let max_color = heatmap_color(1.0);
        let clamped_low = heatmap_color(-0.5);
        let clamped_high = heatmap_color(1.5);
        assert_eq!(clamped_low, min_color);
        assert_eq!(clamped_high, max_color);
    }

    #[test]
    fn test_all_node_shapes_element_construction() {
        let shapes = [
            NodeShape::Ellipse,
            NodeShape::Rectangle,
            NodeShape::Triangle,
            NodeShape::Square,
            NodeShape::Diamond,
            NodeShape::Pentagon,
            NodeShape::Hexagon,
            NodeShape::Octagon,
            NodeShape::Star,
            NodeShape::Ribbon,
        ];
        for shape in shapes {
            let elem = GraphNodeElement {
                id: SharedString::from(format!("node-{:?}", shape)),
                screen_x: 10.0,
                screen_y: 20.0,
                width: 30.0,
                height: 30.0,
                border_width: 1.0,
                border_color: gpui::rgba(0x000000ff),
                fill_color: gpui::rgba(0xffffffff),
                shape,
                text_color: gpui::rgba(0x000000ff),
                font_size: 10.0,
                label: "N".to_string(),
            };
            assert_eq!(elem.shape, shape);
        }
    }

    #[test]
    fn test_canvas_config_performance_mode_defaults() {
        let cfg = CanvasConfig::default();
        assert!(!cfg.hide_edges_during_pan);
        assert_eq!(cfg.min_visible_font_size, 4.0);
    }

    #[test]
    fn test_edge_curve_style_extended_variants() {
        let taxi = EdgeCurveStyle::Taxi;
        let bezier = EdgeCurveStyle::UnbundledBezier(
            graphene_core::math::Vec2::new(10.0, 20.0),
            graphene_core::math::Vec2::new(30.0, 40.0),
        );
        assert_eq!(taxi, EdgeCurveStyle::Taxi);
        assert_ne!(taxi, bezier);
    }
}



