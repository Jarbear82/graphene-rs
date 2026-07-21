use gpui::prelude::*;
use gpui::{px, IntoElement, PathBuilder, Point, SharedString, Styled};
use graphene_core::{GraphState, NodeId};
use graphene_style::{ColorValue, ComputedStyle, EdgeCurveStyle, NodeShape, StylingTarget, Theme};
use crate::render::draw_pipeline::Viewport;
use crate::interaction::state::InteractionState;

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
        }
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

            let src_screen = viewport.model_to_screen(pos_src);
            let tgt_screen = viewport.model_to_screen(pos_tgt);

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

            let mut is_parent = false;
            for j in 0..nodes_count {
                let child_id = state.node_index_to_id[j];
                if let Some(p_id) = *state.hierarchy.parent.get(j) {
                    if p_id == id {
                        if get_visible_rep(child_id) == child_id {
                            is_parent = true;
                            break;
                        }
                    }
                }
            }

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

            let node_w = size_val.w * viewport.zoom;
            let node_h = size_val.h * viewport.zoom;
            let screen_x = (pos.x + viewport.offset.x) * viewport.zoom + viewport.bounds.size.width / 2.0 - (node_w / 2.0);
            let screen_y = (pos.y + viewport.offset.y) * viewport.zoom + viewport.bounds.size.height / 2.0 - (node_h / 2.0);

            let mut shape = if is_compound {
                NodeShape::Rectangle
            } else {
                NodeShape::Ellipse
            };

            let mut fill_color = if is_selected {
                accent_color
            } else if is_compound {
                let mut col = accent_color;
                col.a = 0.08;
                col
            } else {
                node_fill_color
            };

            let mut border_color = if is_selected {
                accent_color
            } else if is_compound {
                let mut col = accent_color;
                col.a = 0.4;
                col
            } else {
                node_border_color
            };

            if idx < state.computed_styles.len() {
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
                .border(px(2.0))
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
                        .text_size(px(10.0 * viewport.zoom))
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
                    let curvature = 35.0;
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
                    // t = 0.5 on quadratic bezier
                    (
                        0.25 * src_x + 0.5 * ctrl_x + 0.25 * tgt_x,
                        0.25 * src_y + 0.5 * ctrl_y + 0.25 * tgt_y,
                    )
                }
            };

            let font_size = match state.edge_computed_styles.get(i).target {
                StylingTarget::Edge(edge_style) => edge_style.label_font_size,
                _ => 12.0,
            };

            // Position label box centered on the midpoint
            let label_w = 60.0 * viewport.zoom;
            let label_h = 16.0 * viewport.zoom;
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

                        // Draw Edges
                        for (src_p, tgt_p, curve_style) in &edge_paths {
                            let mut builder = PathBuilder::stroke(px(2.0));
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
                                    builder.curve_to(ctrl, gpui::point(px(tgt_p.x), px(tgt_p.y)));
                                }
                            }
                            if let Ok(path) = builder.build() {
                                window.paint_path(path, edge_color);
                            }
                        }
                    }
                )
                .size_full()
                .absolute()
            )
            .children(parent_indices.into_iter().map(render_node))
            .children(leaf_indices.into_iter().map(render_node))
            .children(edge_labels_to_render.into_iter().map(render_edge_label))
            .into_any_element()
    }
}
