use crate::app::DemoApp;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder;
use gpui::{
    px, Context, EntityInputHandler, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::Input;
use graphene_core::NodeId;
use graphene_style::{ComputedStyle, NodeShape, StylingTarget};

impl DemoApp {
    pub fn render_sidebar_right(
        &self,
        theme: &Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        gpui::div()
            .id("sidebar-right")
            .flex_col()
            .h_full()
            .bg(theme.panel_bg)
            .border_l(px(1.0))
            .border_color(theme.border)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .overflow_y_scroll()
            .child(self.render_analysis_panel(theme, cx))
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .p_3()
                    .bg(theme.bg)
                    .rounded_md()
                    .border(px(1.0))
                    .border_color(theme.border)
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("CANVAS & ARROW STYLING"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .gap_2()
                            .child(
                                gpui::div()
                                    .flex_1()
                                    .child(
                                        gpui::div()
                                            .text_color(theme.text_dim)
                                            .text_size(px(10.0))
                                            .child("Grid Spacing"),
                                    )
                                    .child(Input::new(&self.input_grid_spacing)),
                            )
                            .child(
                                gpui::div()
                                    .flex_1()
                                    .child(
                                        gpui::div()
                                            .text_color(theme.text_dim)
                                            .text_size(px(10.0))
                                            .child("Arrow Len"),
                                    )
                                    .child(Input::new(&self.input_arrow_length)),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .gap_2()
                            .child(
                                gpui::div()
                                    .flex_1()
                                    .child(
                                        gpui::div()
                                            .text_color(theme.text_dim)
                                            .text_size(px(10.0))
                                            .child("Arrow Width"),
                                    )
                                    .child(Input::new(&self.input_arrow_width)),
                            )
                            .child(
                                gpui::div()
                                    .flex_1()
                                    .child(
                                        gpui::div()
                                            .text_color(theme.text_dim)
                                            .text_size(px(10.0))
                                            .child("Edge Stroke"),
                                    )
                                    .child(Input::new(&self.input_edge_stroke)),
                            ),
                    )
                    .child(
                        gpui::div().flex().gap_2().child(
                            gpui::div()
                                .flex_1()
                                .child(
                                    gpui::div()
                                        .text_color(theme.text_dim)
                                        .text_size(px(10.0))
                                        .child("Edge Curvature"),
                                )
                                .child(Input::new(&self.input_edge_curvature)),
                        ),
                    ),
            )
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
                            .child("3. INSPECTOR"),
                    )
                    .child(
                        if let Some(node_id) = self.selected_node {
                            let label = self
                                .view
                                .nodes
                                .get(&node_id)
                                .map(|n| n.label.clone())
                                .or_else(|| {
                                    self.fixtures[self.selected_fixture_idx]
                                        .node_labels
                                        .get(&node_id)
                                        .cloned()
                                })
                                .unwrap_or_else(|| format!("N{:?}", node_id));
                            let uuid_str = format!("{:?}", node_id);
                            let sec_node: Option<NodeId> = None;

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
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .text_size(px(11.0))
                                        .child(format!("Primary Selected: {}", label)),
                                )
                                .child(
                                    gpui::div()
                                        .text_color(theme.text_dim)
                                        .text_size(px(10.0))
                                        .child(format!("Node UUID (Read-Only):\n{}", uuid_str)),
                                )
                                .when_some(sec_node, |parent, sec_id| {
                                    let sec_label = self
                                        .view
                                        .nodes
                                        .get(&sec_id)
                                        .map(|n| n.label.clone())
                                        .unwrap_or_else(|| {
                                            format!("N{:?}", sec_id)
                                        });
                                    let sec_uuid = format!("{:?}", sec_id);
                                    parent.child(
                                        gpui::div()
                                            .p_2()
                                            .bg(theme.panel_bg)
                                            .rounded_md()
                                            .border(px(1.0))
                                            .border_color(theme.accent)
                                            .flex()
                                            .flex_col()
                                            .gap_1()
                                            .child(
                                                gpui::div()
                                                    .text_color(theme.accent)
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_size(px(11.0))
                                                    .child(format!(
                                                        "Secondary Selected: {}",
                                                        sec_label
                                                    )),
                                            )
                                            .child(
                                                gpui::div()
                                                    .text_color(theme.text_dim)
                                                    .text_size(px(10.0))
                                                    .child(format!("UUID: {}", sec_uuid)),
                                            )
                                            .child(
                                                Button::new("connect-pri-sec-btn")
                                                    .primary()
                                                    .label("CONNECT PRIMARY ➔ SECONDARY")
                                                    .on_click(cx.listener(move |this, _, _, _| {
                                                        this.create_edge_between_nodes(
                                                            node_id, sec_id,
                                                        );
                                                    })),
                                            ),
                                    )
                                })
                                .child(
                                    gpui::div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            gpui::div()
                                                .text_color(theme.text_dim)
                                                .text_size(px(10.0))
                                                .child("Edit Primary Node Label"),
                                        )
                                        .child(Input::new(&self.node_name_state))
                                        .child(
                                            Button::new("update-node-label-btn")
                                                .primary()
                                                .label("UPDATE NODE LABEL")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.update_selected_node_label(window, cx);
                                                })),
                                        ),
                                )
                                .child(
                                    gpui::div()
                                        .flex()
                                        .flex_col()
                                        .gap_1()
                                        .child(
                                            gpui::div()
                                                .text_color(theme.text)
                                                .text_size(px(11.0))
                                                .child("Shape"),
                                        )
                                        .child(
                                            gpui::div().flex().flex_col().gap_1().children(
                                                vec![
                                                    NodeShape::Ellipse,
                                                    NodeShape::Rectangle,
                                                    NodeShape::Square,
                                                    NodeShape::Triangle,
                                                    NodeShape::Diamond,
                                                    NodeShape::Pentagon,
                                                    NodeShape::Hexagon,
                                                    NodeShape::Octagon,
                                                    NodeShape::Star,
                                                    NodeShape::Ribbon,
                                                ]
                                                .into_iter()
                                                .map(|shape| {
                                                    let label = format!("{:?}", shape);
                                                    Button::new(SharedString::from(format!(
                                                        "shape-btn-{}",
                                                        label
                                                    )))
                                                    .label(label)
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        if let Some(id) = this.selected_node {
                                                            let mut style = ComputedStyle::default();
                                                            if let StylingTarget::Node(ref mut node_style) = style.target {
                                                                node_style.shape = shape;
                                                            }
                                                            this.engine.send_command(graphene_layout::GraphCommand::SetNodeData {
                                                                id,
                                                                data: style,
                                                            }).ok();
                                                            cx.notify();
                                                        }
                                                    }))
                                                }),
                                            ),
                                        ),
                                )
                                .child(
                                    gpui::div()
                                        .id("delete-node-container")
                                        .p_1()
                                        .rounded_md()
                                        .child(
                                            Button::new("delete-node-btn")
                                                .danger()
                                                .label("DELETE NODE")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    this.delete_selected_node();
                                                })),
                                        ),
                                )
                        } else if let Some(edge_idx) = self.selected_edge {
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
                                        .child(format!("Selected Edge: idx={}", edge_idx)),
                                )
                                .child(
                                    gpui::div()
                                        .id("delete-edge-container")
                                        .p_1()
                                        .rounded_md()
                                        .child(
                                            Button::new("delete-edge-btn")
                                                .danger()
                                                .label("DELETE EDGE")
                                                .on_click(cx.listener(|this, _, _, _| {
                                                    if let Some(edge_idx) = this.selected_edge {
                                                        if let Some(&id) = this.view.edge_order.get(edge_idx) {
                                                            this.engine.send_command(graphene_layout::GraphCommand::RemoveEdge(id)).ok();
                                                        }
                                                        this.selected_edge = None;
                                                    }
                                                })),
                                        ),
                                )
                        } else {
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(11.0))
                                .child("Select a node or edge to inspect.")
                        },
                    ),
            )
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
                            .child("ADD NODE"),
                    )
                    .child(
                        gpui::div()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(Input::new(&self.node_name_state))
                            .child(
                                Button::new("add-node-btn")
                                    .primary()
                                    .label("ADD NODE")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_new_node(window, cx);
                                    })),
                            ),
                    ),
            )
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
                            .child("ADD EDGE"),
                    )
                    .child(
                        gpui::div()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Source Node Label"),
                            )
                            .child(Input::new(&self.edge_src_state))
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Target Node Label"),
                            )
                            .child(Input::new(&self.edge_tgt_state))
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(10.0))
                                    .child("Weight"),
                            )
                            .child(Input::new(&self.edge_weight_state))
                            .child(
                                gpui::div()
                                    .flex()
                                    .gap_1()
                                    .child(
                                        Button::new("set-src-btn")
                                            .label("USE SELECTED SOURCE")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                if let Some(id) = this.selected_node {
                                                    let label = this
                                                        .view
                                                        .nodes
                                                        .get(&id)
                                                        .map(|n| n.label.clone())
                                                        .or_else(|| {
                                                            this.fixtures[this.selected_fixture_idx]
                                                                .node_labels
                                                                .get(&id)
                                                                .cloned()
                                                        })
                                                        .unwrap_or_else(|| {
                                                            format!("N{:?}", id)
                                                        });
                                                    this.edge_src_state.update(cx, |input, cx| {
                                                        let len = input.text().len();
                                                        input.replace_text_in_range(
                                                            Some(0..len),
                                                            &label,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            })),
                                    )
                                    .child(
                                        Button::new("set-tgt-btn")
                                            .label("USE SELECTED TARGET")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                if let Some(id) = this.selected_node {
                                                    let label = this
                                                        .view
                                                        .nodes
                                                        .get(&id)
                                                        .map(|n| n.label.clone())
                                                        .or_else(|| {
                                                            this.fixtures[this.selected_fixture_idx]
                                                                .node_labels
                                                                .get(&id)
                                                                .cloned()
                                                        })
                                                        .unwrap_or_else(|| {
                                                            format!("N{:?}", id)
                                                        });
                                                    this.edge_tgt_state.update(cx, |input, cx| {
                                                        let len = input.text().len();
                                                        input.replace_text_in_range(
                                                            Some(0..len),
                                                            &label,
                                                            window,
                                                            cx,
                                                        );
                                                    });
                                                }
                                            })),
                                    ),
                            )
                            .child(
                                Button::new("add-edge-btn")
                                    .primary()
                                    .label("ADD EDGE")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.add_new_edge(window, cx);
                                    })),
                            ),
                    ),
            )
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
                            .child("THEME"),
                    )
                    .child(
                        gpui::div().flex_col().gap_1().children(
                            vec![
                                "Catppuccin Mocha",
                                "Gruvbox Dark",
                                "One Dark",
                                "GitHub Light",
                            ]
                            .into_iter()
                            .map(|t| {
                                let is_active =
                                    self.themes.themes[self.current_theme_idx].name == t;
                                gpui::div()
                                    .id(SharedString::from(format!("theme-{}", t)))
                                    .p_1()
                                    .bg(if is_active { theme.accent } else { theme.bg })
                                    .text_color(if is_active {
                                        theme.panel_bg
                                    } else {
                                        theme.text
                                    })
                                    .text_size(px(10.0))
                                    .rounded_md()
                                    .cursor_pointer()
                                    .on_click(cx.listener(move |this, _, _, _| {
                                        if let Some(pos) =
                                            this.themes.themes.iter().position(|x| x.name == t)
                                        {
                                            this.current_theme_idx = pos;
                                        }
                                    }))
                                    .child(t)
                            }),
                        ),
                    ),
            )
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
                            .child("HISTORY"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .gap_2()
                            .child(Button::new("undo-btn").label("UNDO").on_click(cx.listener(
                                |this, _, _, _| {
                                    this.engine.send_command(graphene_layout::GraphCommand::Undo).ok();
                                    this.selected_node = None;
                                    this.selected_edge = None;
                                },
                            )))
                            .child(Button::new("redo-btn").label("REDO").on_click(cx.listener(
                                |this, _, _, _| {
                                    this.engine.send_command(graphene_layout::GraphCommand::Redo).ok();
                                    this.selected_node = None;
                                    this.selected_edge = None;
                                },
                            ))),
                    ),
            )
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
                            .child("WORKSPACE IO"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(Button::new("load-json-btn").label("LOAD JSON").on_click(
                                cx.listener(|this, _, _, _| {
                                    if let Ok(json) =
                                        std::fs::read_to_string("workspace_graph.json")
                                    {
                                        if let Ok(new_state) =
                                            graphene_core::GraphState::from_json(&json)
                                        {
                                            this.engine.load_preset(new_state);
                                            this.selected_node = None;
                                            this.selected_edge = None;
                                        }
                                    }
                                }),
                            )),
                    ),
            )
    }
}
