use crate::app::DemoApp;
use crate::theme::Theme;
use gpui::{
    px, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
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
        let layout_params_children = {
            let mut children = Vec::new();
            let layout = self.selected_layout.as_str();

            let has_force_directed = matches!(
                layout,
                "ForceDirected"
                    | "CoSE"
                    | "WeightedForce"
                    | "DisconnectedPack"
                    | "Compound"
                    | "RegionalPartition"
            );

            if has_force_directed {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Gravity"),
                        )
                        .child(Input::new(&self.input_gravity)),
                );
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Repulsion"),
                        )
                        .child(Input::new(&self.input_k_rep)),
                );
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Attraction"),
                        )
                        .child(Input::new(&self.input_k_att)),
                );
            }

            if has_force_directed || matches!(layout, "KamadaKawai" | "MDS" | "CollisionForce") {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Iterations"),
                        )
                        .child(Input::new(&self.input_iterations)),
                );
            }

            if layout == "Circle" {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Circle Radius"),
                        )
                        .child(Input::new(&self.input_circle_radius)),
                );
            }

            if matches!(
                layout,
                "ForceDirected" | "CoSE" | "DisconnectedPack" | "Compound" | "RegionalPartition"
            ) {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Barnes-Hut Theta"),
                        )
                        .child(Input::new(&self.input_theta)),
                );
            }

            if layout == "Sugiyama" {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Layer Spacing"),
                        )
                        .child(Input::new(&self.input_layer_spacing)),
                );
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Node Spacing"),
                        )
                        .child(Input::new(&self.input_node_spacing)),
                );
            }

            if layout == "MDS" {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Base Distance"),
                        )
                        .child(Input::new(&self.input_mds_base_dist)),
                );
            }

            if layout == "Bipartite" {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Column Spacing"),
                        )
                        .child(Input::new(&self.input_bipartite_col_spacing)),
                );
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Vertical Spacing"),
                        )
                        .child(Input::new(&self.input_bipartite_vert_spacing)),
                );
            }

            if layout == "DisconnectedPack" {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Packer Spacing"),
                        )
                        .child(Input::new(&self.input_packer_spacing)),
                );
            }

            if matches!(layout, "CoSE" | "Compound") {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Compound Padding"),
                        )
                        .child(Input::new(&self.input_compound_padding)),
                );
            }

            if layout == "RegionalPartition" {
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Regional Columns"),
                        )
                        .child(Input::new(&self.input_regional_columns)),
                );
                children.push(
                    gpui::div()
                        .child(
                            gpui::div()
                                .text_color(theme.text_dim)
                                .text_size(px(10.0))
                                .child("Regional Cell Size"),
                        )
                        .child(Input::new(&self.input_regional_cell_size)),
                );
            }

            if children.is_empty() {
                children.push(
                    gpui::div().child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child("No configurable options for this layout."),
                    ),
                );
            }

            children
        };

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
                    .child(if let Some(node_id) = self.selected_node {
                        let label = self.fixtures[self.selected_fixture_idx]
                            .node_labels
                            .get(&node_id)
                            .cloned()
                            .unwrap_or_else(|| "No label".to_string());
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
                                    .child(format!("Selected Node: {}", label)),
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
                                                    .on_click(cx.listener(move |this, _, _, _| {
                                                        if let Some(id) = this.selected_node {
                                                            if let Some(&idx) =
                                                                this.state.node_keys.get(id)
                                                            {
                                                                let style = this
                                                                    .state
                                                                    .computed_styles
                                                                    .get_mut(idx);
                                                                if let StylingTarget::Node(
                                                                    ref mut node_style,
                                                                ) = style.target
                                                                {
                                                                    node_style.shape = shape;
                                                                }
                                                            }
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
                            .child("LAYOUT PARAMETERS"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .flex_col()
                            .gap_2()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .children(layout_params_children),
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
