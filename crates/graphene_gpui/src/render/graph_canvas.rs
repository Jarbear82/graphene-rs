use crate::interaction::state::InteractionState;
use crate::render::draw_pipeline::Viewport;
use crate::view::GraphView;
use gpui::prelude::*;
use gpui::{px, IntoElement, PathBuilder, Point, SharedString, Styled};
use graphene_core::NodeId;
use graphene_style::{ColorValue, ComputedStyle, EdgeCurveStyle, NodeShape, StylingTarget, Theme};
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

pub fn hex_to_rgba(hex: &str) -> Option<gpui::Rgba> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()? as u32;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()? as u32;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()? as u32;
        Some(gpui::rgba(r << 24 | g << 16 | b << 8 | 255))
    } else {
        None
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
    pub view: &'a GraphView<ComputedStyle>,
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
        view: &'a GraphView<ComputedStyle>,
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
            view,
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
        let view = self.view;
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
            while let Some(node) = view.nodes.get(&curr) {
                if let Some(parent_id) = node.parent {
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
            for (i, &edge_id) in view.edge_order.iter().enumerate() {
                if let Some(edge) = view.edges.get(&edge_id) {
                    let src_rep = get_visible_rep(edge.source);
                    let tgt_rep = get_visible_rep(edge.target);
                    if src_rep == sel_id || tgt_rep == sel_id {
                        connected_edge_indices.insert(i);
                        connected_nodes.insert(src_rep);
                        connected_nodes.insert(tgt_rep);
                    }
                }
            }
        }

        let mut edge_paths = Vec::new();
        let mut edge_labels_to_render = Vec::new();
        let is_panning_active = self.interaction_state.drag_session.is_some() || self.interaction_state.pan_origin.is_some();
        let skip_edges = cfg.hide_edges_during_pan && is_panning_active;

        if !skip_edges {
            for (i, &edge_id) in view.edge_order.iter().enumerate() {
                let Some(edge) = view.edges.get(&edge_id) else { continue };

                let src_rep = get_visible_rep(edge.source);
                let tgt_rep = get_visible_rep(edge.target);

                if src_rep == tgt_rep {
                    continue;
                }

                let (Some(src_node), Some(tgt_node)) = (view.nodes.get(&src_rep), view.nodes.get(&tgt_rep)) else {
                    continue;
                };

                let pos_src = src_node.pos;
                let pos_tgt = tgt_node.pos;
                let src_size = src_node.size;
                let tgt_size = tgt_node.size;

                if !viewport.is_visible(pos_src, src_size) && !viewport.is_visible(pos_tgt, tgt_size) {
                    continue;
                }

                let src_screen = viewport.model_to_screen(pos_src);

                let clipped_tgt = graphene_layout::find_clipping_point(
                    pos_tgt,
                    tgt_size,
                    pos_src.x - pos_tgt.x,
                    pos_src.y - pos_tgt.y,
                );
                let tgt_screen = viewport.model_to_screen(clipped_tgt);

                let curve_style = EdgeCurveStyle::Straight;
                let mut label_text = edge_labels.get(&i).cloned();
                if label_text.is_none() || label_text.as_deref() == Some("") {
                    let primary_lbl = edge.data.primary_label();
                    let mult = edge.data.multiplicity.as_deref();
                    if let Some(lbl) = primary_lbl {
                        if let Some(m) = mult {
                            label_text = Some(format!("{} {}", lbl, m));
                        } else {
                            label_text = Some(lbl.to_string());
                        }
                    } else if let Some(m) = mult {
                        label_text = Some(m.to_string());
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

        let nodes_count = view.node_order.len();

        let mut parent_indices = Vec::new();
        let mut leaf_indices = Vec::new();
        for (idx, &id) in view.node_order.iter().enumerate() {
            if get_visible_rep(id) != id {
                continue;
            }

            let is_parent = view.nodes.get(&id).map_or(false, |n| !n.children.is_empty());

            if is_parent {
                parent_indices.push(idx);
            } else {
                leaf_indices.push(idx);
            }
        }

        let render_node = |idx: usize| -> Option<gpui::AnyElement> {
            let id = view.node_order[idx];
            let node = view.nodes.get(&id)?;
            let pos = node.pos;
            let size_val = node.size;

            if !viewport.is_visible(pos, size_val) {
                return None;
            }

            let mut label = node_labels
                .get(&id)
                .cloned()
                .unwrap_or_else(|| {
                    if node.label.is_empty() {
                        format!("N{}", idx)
                    } else {
                        node.label.clone()
                    }
                });

            let is_compound = !node.children.is_empty();
            let is_collapsed = collapsed_parents.contains(&id);

            if is_compound {
                if is_collapsed {
                    label = format!("[+] {}", label);
                } else {
                    label = format!("[-] {}", label);
                }
            }

            let is_primary = selected_node == Some(id);
            let is_secondary = false;
            let is_selected = is_primary || is_secondary;
            let has_selection = selected_node.is_some();
            let is_neighbor = has_selection && connected_nodes.contains(&id);
            let is_faded = has_selection && !is_selected && !is_neighbor;

            match node.node_data.expansion_mode {
                graphene_core::DataExpansionMode::Compact => {}
                graphene_core::DataExpansionMode::Preview => {
                    let preview_items: Vec<String> = node
                        .node_data
                        .props
                        .iter()
                        .filter(|(k, _)| k.as_str() != "@display" && k.as_str() != "@background")
                        .take(2)
                        .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                        .collect();
                    if !preview_items.is_empty() {
                        label = format!("{}\n({})", label, preview_items.join(", "));
                    }
                }
                graphene_core::DataExpansionMode::Full => {
                    let full_items: Vec<String> = node
                        .node_data
                        .props
                        .iter()
                        .filter(|(k, _)| k.as_str() != "@display" && k.as_str() != "@background")
                        .map(|(k, v)| format!("{}: {}", k, v.to_display_string()))
                        .collect();
                    if !full_items.is_empty() {
                        label = format!("{}\n{}", label, full_items.join("\n"));
                    }
                }
            }

            let effective_font_size = cfg.node_font_size * viewport.zoom;
            if effective_font_size < cfg.min_visible_font_size {
                label = String::new();
            } else if label.chars().count() > max_untruncated_len && !is_selected && node.node_data.expansion_mode == graphene_core::DataExpansionMode::Compact {
                label = label.chars().take(max_untruncated_len).collect::<String>() + "...";
            }

            let mut scale = 1.0f32;
            let score_opt = centrality_scores.as_ref().and_then(|m| m.get(&id).copied());

            let mut node_w = size_val.w * viewport.zoom;
            let mut node_h = size_val.h * viewport.zoom;

            let line_count = label.lines().count().max(1);
            let max_line_len = label.lines().map(|l| l.chars().count()).max().unwrap_or(1);

            match node.node_data.expansion_mode {
                graphene_core::DataExpansionMode::Compact => {}
                graphene_core::DataExpansionMode::Preview => {
                    let min_w = (max_line_len as f32 * 8.5 + 28.0) * viewport.zoom;
                    let min_h = (line_count as f32 * 18.0 + 24.0) * viewport.zoom;
                    node_w = node_w.max(min_w);
                    node_h = node_h.max(min_h);
                }
                graphene_core::DataExpansionMode::Full => {
                    let min_w = (max_line_len as f32 * 9.0 + 36.0) * viewport.zoom;
                    let min_h = (line_count as f32 * 18.0 + 28.0) * viewport.zoom;
                    node_w = node_w.max(min_w);
                    node_h = node_h.max(min_h);
                }
            }

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

            // Parse @background property text from node_data.props if present
            if let Some(graphene_core::PropValue::Text(hex)) = node.node_data.props.get("@background") {
                if let Some(parsed) = hex_to_rgba(hex) {
                    fill_color = parsed;
                }
            }

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

            if score_opt.is_none() {
                if let StylingTarget::Node(ref node_style) = node.data.target {
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

            let font_size = cfg.edge_label_font_size;

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

                        // Draw Edges
                        for (src_screen_f, tgt_screen_f, curve_style, cur_edge_color, stroke_w) in edge_paths {
                            let src_p = gpui::point(px(src_screen_f.x), px(src_screen_f.y));
                            let tgt_p = gpui::point(px(tgt_screen_f.x), px(tgt_screen_f.y));

                            let mut builder = PathBuilder::stroke(px(stroke_w));
                            builder.move_to(src_p);

                            match curve_style {
                                EdgeCurveStyle::Straight => {
                                    builder.line_to(tgt_p);
                                }
                                EdgeCurveStyle::Bezier | EdgeCurveStyle::Segmented => {
                                    let mid_x = (src_screen_f.x + tgt_screen_f.x) / 2.0;
                                    let mid_y = (src_screen_f.y + tgt_screen_f.y) / 2.0 - cfg.edge_curvature * viewport.zoom;
                                    let control = gpui::point(px(mid_x), px(mid_y));
                                    builder.cubic_bezier_to(tgt_p, control, control);
                                }
                                EdgeCurveStyle::Taxi => {
                                    let mid_x = (src_screen_f.x + tgt_screen_f.x) / 2.0;
                                    builder.line_to(gpui::point(px(mid_x), px(src_screen_f.y)));
                                    builder.line_to(gpui::point(px(mid_x), px(tgt_screen_f.y)));
                                    builder.line_to(tgt_p);
                                }
                                EdgeCurveStyle::UnbundledBezier(cp1, cp2) => {
                                    let control1 = gpui::point(px(cp1.x), px(cp1.y));
                                    let control2 = gpui::point(px(cp2.x), px(cp2.y));
                                    builder.cubic_bezier_to(control1, control2, tgt_p);
                                }
                            }

                            if let Ok(path) = builder.build() {
                                window.paint_path(path, cur_edge_color);
                            }

                            if is_directed {
                                let dx = tgt_screen_f.x - src_screen_f.x;
                                let dy = tgt_screen_f.y - src_screen_f.y;
                                let len = (dx * dx + dy * dy).sqrt().max(0.001);
                                let dir_x = dx / len;
                                let dir_y = dy / len;
                                let perp_x = -dir_y;
                                let perp_y = dir_x;

                                let arrow_len = cfg.arrow_length * viewport.zoom;
                                let arrow_half_w = (cfg.arrow_width / 2.0) * viewport.zoom;

                                let p1 = gpui::point(
                                    px(tgt_screen_f.x - dir_x * arrow_len + perp_x * arrow_half_w),
                                    px(tgt_screen_f.y - dir_y * arrow_len + perp_y * arrow_half_w),
                                );
                                let p2 = gpui::point(
                                    px(tgt_screen_f.x - dir_x * arrow_len - perp_x * arrow_half_w),
                                    px(tgt_screen_f.y - dir_y * arrow_len - perp_y * arrow_half_w),
                                );

                                let mut arrow_builder = PathBuilder::fill();
                                arrow_builder.move_to(tgt_p);
                                arrow_builder.line_to(p1);
                                arrow_builder.line_to(p2);
                                arrow_builder.close();

                                if let Ok(arrow_path) = arrow_builder.build() {
                                    window.paint_path(arrow_path, cur_edge_color);
                                }
                            }
                        }
                    },
                )
                .absolute()
                .inset_0(),
            )
            .children(parent_indices.into_iter().filter_map(render_node))
            .children(leaf_indices.into_iter().filter_map(render_node))
            .children(edge_labels_to_render.into_iter().map(render_edge_label))
            .child(
                gpui::div()
                    .absolute()
                    .top(px(16.0))
                    .right(px(16.0))
                    .px_3()
                    .py_1()
                    .bg(gpui::rgba(0x1e1e2eff))
                    .border(px(1.0))
                    .border_color(gpui::rgba(0x313244ff))
                    .rounded_md()
                    .text_xs()
                    .font_family("Courier New")
                    .text_color(accent_color)
                    .child(graph_type_badge),
            )
            .into_any_element()
    }
}
