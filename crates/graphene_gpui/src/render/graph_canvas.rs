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

        // Precompute edge paths for drawing
        let mut edge_paths = Vec::new();
        let mut edge_labels_to_render = Vec::new();
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

            edge_paths.push((src_screen, tgt_screen, curve_style));

            if let Some(lbl) = label_text {
                if !lbl.is_empty() {
                    edge_labels_to_render.push((i, src_screen, tgt_screen, curve_style, lbl));
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

        let render_node = |idx: usize| {
            let id = state.node_index_to_id[idx];
            let pos = *state.positions.get(idx);
            let size_val = *state.sizes.get(idx);

            let mut label = node_labels.get(&id)
                .cloned()
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

            let is_selected = selected_node == Some(id);
            if label.chars().count() > max_untruncated_len && !is_selected {
                label = label.chars().take(max_untruncated_len).collect::<String>() + "...";
            }

            let mut scale = 1.0f32;
            let score_opt = centrality_scores.as_ref().and_then(|m| m.get(&id).copied());

            let mut node_w = size_val.w * viewport.zoom;
            let mut node_h = size_val.h * viewport.zoom;

            if let Some(score) = score_opt {
                scale = 0.8 + 0.5 * score;
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
            } else if is_selected {
                accent_color
            } else if is_compound {
                let mut col = accent_color;
                col.a = cfg.compound_fill_alpha;
                col
            } else {
                node_fill_color
            };

            let mut border_color = if is_selected {
                accent_color
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

            if is_selected {
                border_color = accent_color;
            }

            gpui::div()
                .id(SharedString::from(format!("canvas-node-{}", idx)))
                .absolute()
                .left(px(screen_x))
                .top(px(screen_y))
                .w(px(node_w))
                .h(px(node_h))
                .border(px(cfg.node_border_width))
                .border_color(border_color)
                .bg(fill_color)
                .when(shape == NodeShape::Ellipse, |d| d.rounded_full())
                .when(shape == NodeShape::Rectangle, |d| d.rounded_none())
                .when(shape == NodeShape::Diamond, |d| d.rounded_md())
                .flex()
                .items_center()
                .justify_center()
                .child(
                    gpui::div()
                        .text_color(text_color)
                        .text_size(px(cfg.node_font_size * viewport.zoom * scale))
                        .child(label),
                )
        };

        let render_edge_label = |(i, src_p, tgt_p, curve_style, label): (usize, Point<f32>, Point<f32>, EdgeCurveStyle, String)| {
            let src_x = f32::from(src_p.x);
            let src_y = f32::from(src_p.y);
            let tgt_x = f32::from(tgt_p.x);
            let tgt_y = f32::from(tgt_p.y);

            let (mid_x, mid_y) = match curve_style {
                EdgeCurveStyle::Straight => {
                    ((src_x + tgt_x) / 2.0, (src_y + tgt_y) / 2.0)
                }
                _ => {
                    let mid_x = (src_x + tgt_x) / 2.0;
                    let mid_y = (src_y + tgt_y) / 2.0;
                    let dx = tgt_x - src_x;
                    let dy = tgt_y - src_y;
                    let len = (dx * dx + dy * dy).sqrt();
                    let curvature = cfg.edge_curvature;
                    let ctrl_x = if len > 0.0 {
                        mid_x - (dy / len) * curvature
                    } else {
                        mid_x
                    };
                    let ctrl_y = if len > 0.0 {
                        mid_y + (dx / len) * curvature
                    } else {
                        mid_y
                    };
                    (
                        0.25 * src_x + 0.5 * ctrl_x + 0.25 * tgt_x,
                        0.25 * src_y + 0.5 * ctrl_y + 0.25 * tgt_y,
                    )
                }
            };

            let font_size = match state.edge_computed_styles.get(i).target {
                StylingTarget::Edge(edge_style) => edge_style.label_font_size,
                _ => cfg.edge_label_font_size,
            };

            let label_w = cfg.edge_label_width * viewport.zoom;
            let label_h = cfg.edge_label_height * viewport.zoom;
            let screen_x = mid_x - (label_w / 2.0);
            let screen_y = mid_y - (label_h / 2.0);

            gpui::div()
                .id(SharedString::from(format!("canvas-edge-label-{}", i)))
                .absolute()
                .left(px(screen_x))
                .top(px(screen_y))
                .w(px(label_w))
                .h(px(label_h))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    gpui::div()
                        .text_color(text_color)
                        .text_size(px(font_size * viewport.zoom))
                        .child(label),
                )
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
                        for (src_p, tgt_p, curve_style) in &edge_paths {
                            let mut builder = PathBuilder::stroke(px(cfg.edge_stroke_width));
                            builder.move_to(gpui::point(px(src_p.x), px(src_p.y)));

                            match curve_style {
                                EdgeCurveStyle::Straight => {
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
                                window.paint_path(path, edge_color);
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
            .children(parent_indices.into_iter().map(render_node))
            .children(leaf_indices.into_iter().map(render_node))
            .children(edge_labels_to_render.into_iter().map(render_edge_label))
            .into_any_element()
    }
}
