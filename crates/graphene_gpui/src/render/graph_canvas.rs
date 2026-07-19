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
}

impl<'a> GraphCanvas<'a> {
    pub fn new(
        state: &'a GraphState<ComputedStyle>,
        viewport: &'a Viewport,
        interaction_state: &'a InteractionState,
        theme: &'a Theme,
        selected_node: Option<NodeId>,
        node_labels: &'a std::collections::HashMap<NodeId, String>,
    ) -> Self {
        Self {
            state,
            viewport,
            interaction_state,
            theme,
            selected_node,
            node_labels,
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

        let edge_color = color_to_gpui(theme.edge_color);
        let text_color = color_to_gpui(theme.text);
        let accent_color = color_to_gpui(theme.accent);
        let node_fill_color = color_to_gpui(theme.node_fill);
        let node_border_color = color_to_gpui(theme.node_border);

        // Precompute edge paths for drawing
        let mut edge_paths = Vec::new();
        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            let (Some(&src_idx), Some(&tgt_idx)) = (state.node_keys.get(src), state.node_keys.get(tgt)) else {
                continue;
            };
            let pos_src = *state.positions.get(src_idx);
            let pos_tgt = *state.positions.get(tgt_idx);

            let src_screen = viewport.model_to_screen(pos_src);
            let tgt_screen = viewport.model_to_screen(pos_tgt);

            let curve_style = match state.edge_computed_styles.get(i).target {
                StylingTarget::Edge(edge_style) => edge_style.curve_style,
                _ => EdgeCurveStyle::Straight,
            };

            edge_paths.push((src_screen, tgt_screen, curve_style));
        }

        let nodes_count = state.node_index_to_id.len();

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
            .children((0..nodes_count).map(|idx| {
                let id = state.node_index_to_id[idx];
                let pos = *state.positions.get(idx);
                let size_val = *state.sizes.get(idx);

                let label = node_labels.get(&id)
                    .cloned()
                    .unwrap_or_else(|| format!("N{}", idx));

                let node_w = size_val.w * viewport.zoom;
                let node_h = size_val.h * viewport.zoom;
                let screen_x = (pos.x + viewport.offset.x) * viewport.zoom + viewport.bounds.size.width / 2.0 - (node_w / 2.0);
                let screen_y = (pos.y + viewport.offset.y) * viewport.zoom + viewport.bounds.size.height / 2.0 - (node_h / 2.0);

                let is_selected = selected_node == Some(id);

                let mut fill_color = if is_selected {
                    accent_color
                } else {
                    node_fill_color
                };
                let mut border_color = if is_selected {
                    color_to_gpui(theme.panel_bg)
                } else {
                    node_border_color
                };

                let mut shape = NodeShape::Ellipse;
                if idx < state.computed_styles.len() {
                    if let StylingTarget::Node(node_style) = state.computed_styles.get(idx).target {
                        fill_color = color_to_gpui(node_style.fill_color);
                        border_color = color_to_gpui(node_style.border_color);
                        shape = node_style.shape;
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
            }))
            .into_any_element()
    }
}
