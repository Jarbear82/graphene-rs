use crate::app::DemoApp;
use crate::theme::Theme;
use gpui::prelude::FluentBuilder;
use gpui::{
    px, Context, EntityInputHandler, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::Input;
use graphene_style::{NodeShape, StylingTarget};

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
                    .child(if let Some(node_id) = self.state.selected.primary_node().or(self.selected_node) {
                        let label = self.state.get_node_label(node_id)
                            .map(|s| s.to_string())
                            .or_else(|| self.fixtures[self.selected_fixture_idx].node_labels.get(&node_id).cloned())
                            .unwrap_or_else(|| format!("N{}", self.state.node_keys[node_id]));
                        let uuid_str = self.state.get_node_uuid(node_id).unwrap_or("N/A").to_string();
                        let sec_node = self.state.selected.secondary_node();

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
                                let sec_label = self.state.get_node_label(sec_id)
                                    .map(|s| s.to_string())
                                    .or_else(|| self.fixtures[self.selected_fixture_idx].node_labels.get(&sec_id).cloned())
                                    .unwrap_or_else(|| format!("N{}", self.state.node_keys[sec_id]));
                                let sec_uuid = self.state.get_node_uuid(sec_id).unwrap_or("N/A").to_string();
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
                                                .child(format!("Secondary Selected: {}", sec_label)),
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
                                                    this.create_edge_between_nodes(node_id, sec_id);
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
                                            .map(
                                                |shape| {
                                                    let label = format!("{:?}", shape);
                                                    Button::new(SharedString::from(format!(
                                                        "shape-btn-{}",
                                                        label
                                                    )))
                                                    .label(label)
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        if let Some(id) = this.selected_node {
                                                            graphene_gpui::update_node_shape(&mut this.state, id, shape);
                                                            cx.notify();
                                                        }
                                                    }))
                                                },
                                            ),
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
                                    .flex()
                                    .flex_col()
                                    .gap_1()
                                    .child(
                                        gpui::div()
                                            .text_color(theme.text)
                                            .text_size(px(11.0))
                                            .child("Width"),
                                    )
                                    .child(gpui::div().flex().gap_1().children(
                                        vec![1.5, 3.0, 5.0].into_iter().map(|w| {
                                            Button::new(SharedString::from(format!(
                                                "width-btn-{}",
                                                w
                                            )))
                                            .label(format!("{}px", w))
                                            .on_click(cx.listener(move |this, _, _, _| {
                                                if let Some(edge_idx) = this.selected_edge {
                                                    let style = this
                                                        .state
                                                        .edge_computed_styles
                                                        .get_mut(edge_idx);
                                                    if let StylingTarget::Edge(ref mut edge_style) =
                                                        style.target
                                                    {
                                                        edge_style.line_width =
                                                            graphene_style::LengthValue::Pixels(w);
                                                    }
                                                }
                                            }))
                                        }),
                                    )),
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
                                                    this.undo_redo.record_state(&this.state);
                                                    let id = this.state.edge_index_to_id[edge_idx];
                                                    this.state.remove_edge(id);
                                                    this.selected_edge = None;
                                                    this.state.dirty_flags |=
                                                        graphene_core::DirtyFlags::TOPOLOGY_DIRTY;
                                                    this.interaction_state
                                                        .rebuild_grid(&this.state);
                                                }
                                            })),
                                    ),
                            )
                    } else {
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child("Select a node or edge to inspect.")
                    }),
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
                                                    let label = this.state.get_node_label(id)
                                                        .map(|s| s.to_string())
                                                        .or_else(|| this.fixtures[this.selected_fixture_idx].node_labels.get(&id).cloned())
                                                        .unwrap_or_else(|| format!("N{}", this.state.node_keys[id]));
                                                    this.edge_src_state.update(cx, |input, cx| {
                                                        let len = input.text().len();
                                                        input.replace_text_in_range(Some(0..len), &label, window, cx);
                                                    });
                                                }
                                            })),
                                    )
                                    .child(
                                        Button::new("set-tgt-btn")
                                            .label("USE SELECTED TARGET")
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                if let Some(id) = this.selected_node {
                                                    let label = this.state.get_node_label(id)
                                                        .map(|s| s.to_string())
                                                        .or_else(|| this.fixtures[this.selected_fixture_idx].node_labels.get(&id).cloned())
                                                        .unwrap_or_else(|| format!("N{}", this.state.node_keys[id]));
                                                    this.edge_tgt_state.update(cx, |input, cx| {
                                                        let len = input.text().len();
                                                        input.replace_text_in_range(Some(0..len), &label, window, cx);
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
                                    this.undo_redo.undo(&mut this.state);
                                    this.selected_node = None;
                                    this.selected_edge = None;
                                    this.interaction_state.rebuild_grid(&this.state);
                                },
                            )))
                            .child(Button::new("redo-btn").label("REDO").on_click(cx.listener(
                                |this, _, _, _| {
                                    this.undo_redo.redo(&mut this.state);
                                    this.selected_node = None;
                                    this.selected_edge = None;
                                    this.interaction_state.rebuild_grid(&this.state);
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
                            .child(Button::new("save-json-btn").label("SAVE JSON").on_click(
                                cx.listener(|this, _, _, _| {
                                    let json = this.state.to_json();
                                    if let Err(e) = std::fs::write("workspace_graph.json", json) {
                                        println!("Failed to save graph: {:?}", e);
                                    } else {
                                        println!("Saved graph to workspace_graph.json");
                                    }
                                }),
                            ))
                            .child(Button::new("load-json-btn").label("LOAD JSON").on_click(
                                cx.listener(|this, _, _, _| {
                                    if let Ok(json) =
                                        std::fs::read_to_string("workspace_graph.json")
                                    {
                                        if let Ok(new_state) =
                                            graphene_core::GraphState::from_json(&json)
                                        {
                                            this.undo_redo.record_state(&this.state);
                                            this.state = new_state;
                                            this.selected_node = None;
                                            this.selected_edge = None;
                                            this.interaction_state.rebuild_grid(&this.state);
                                            this.viewport.fit_to_graph(&this.state);
                                        }
                                    }
                                }),
                            ))
                            .child(Button::new("export-dot-btn").label("EXPORT DOT").on_click(
                                cx.listener(|this, _, _, _| {
                                    let dot = this.state.to_dot();
                                    if let Err(e) = std::fs::write("workspace_graph.dot", dot) {
                                        println!("Failed to export DOT: {:?}", e);
                                    } else {
                                        println!("Exported graph to workspace_graph.dot");
                                    }
                                }),
                            )),
                    ),
            )
    }
}
