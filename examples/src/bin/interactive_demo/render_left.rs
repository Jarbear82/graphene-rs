use crate::app::DemoApp;
use crate::theme::{Theme, LAYOUT_NAMES};
use gpui::prelude::FluentBuilder;
use gpui::{
    px, Context, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::Input;
use gpui_component::scroll::ScrollableElement;
use gpui_component::Disableable;

impl DemoApp {
    pub fn render_sidebar_left(&self, theme: &Theme, cx: &mut Context<Self>) -> impl IntoElement {
        gpui::div()
            .id("sidebar-left-scroll")
            .w(px(280.0))
            .h_full()
            .overflow_y_scroll()
            .bg(theme.panel_bg)
            .border_r(px(1.0))
            .border_color(theme.border)
            .p_4()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("1. SELECT GRAPH FIXTURE"),
                    )
                    .child(
                        gpui::div()
                            .id("preset-scroll-container")
                            .flex()
                            .flex_col()
                            .h(px(150.0))
                            .overflow_y_scroll()
                            .on_scroll_wheel(cx.listener(|_, _, _, cx| {
                                cx.stop_propagation();
                            }))
                            .border(px(1.0))
                            .border_color(theme.border)
                            .bg(theme.bg)
                            .rounded_md()
                            .children(self.fixtures.iter().enumerate().map(|(idx, f)| {
                                let is_selected = self.selected_fixture_idx == idx;
                                gpui::div()
                                    .id(SharedString::from(format!("preset-{}", idx)))
                                    .p_2()
                                    .border_b(px(1.0))
                                    .border_color(theme.border)
                                    .bg(if is_selected {
                                        theme.accent
                                    } else {
                                        gpui::rgba(0)
                                    })
                                    .text_color(if is_selected {
                                        theme.panel_bg
                                    } else {
                                        theme.text
                                    })
                                    .text_size(px(11.0))
                                    .font_weight(if is_selected { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                                    .cursor_pointer()
                                    .hover(|s| if is_selected { s } else { s.bg(theme.border) })
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.load_preset(idx, window, cx);
                                    }))
                                    .child(if is_selected {
                                        format!("✓ {}", f.name)
                                    } else {
                                        f.name.clone()
                                    })
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
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("2. LAYOUT ENGINE"),
                    )
                    .child(
                        gpui::div()
                            .id("layout-accordion-container")
                            .flex()
                            .flex_col()
                            .gap_2()
                            .h(px(280.0))
                            .overflow_y_scroll()
                            .on_scroll_wheel(cx.listener(|_, _, _, cx| {
                                cx.stop_propagation();
                            }))
                            .children(LAYOUT_NAMES.iter().map(|&name| {
                                let is_selected = self.selected_layout == name;
                                let is_expanded = self.expanded_layout.as_deref() == Some(name);

                                gpui::div()
                                    .id(SharedString::from(format!("layout-card-{}", name)))
                                    .flex()
                                    .flex_col()
                                    .border(px(1.0))
                                    .border_color(if is_selected { theme.accent } else { theme.border })
                                    .bg(theme.bg)
                                    .rounded_md()
                                    .child(
                                        gpui::div()
                                            .id(SharedString::from(format!("layout-header-{}", name)))
                                            .flex()
                                            .items_center()
                                            .justify_between()
                                            .p_2()
                                            .bg(if is_selected {
                                                theme.accent
                                            } else {
                                                gpui::rgba(0)
                                            })
                                            .text_color(if is_selected {
                                                theme.panel_bg
                                            } else {
                                                theme.text
                                            })
                                            .cursor_pointer()
                                            .hover(|s| if is_selected { s } else { s.bg(theme.border) })
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.selected_layout = name.to_string();
                                                if this.expanded_layout.as_deref() == Some(name) {
                                                    this.expanded_layout = None;
                                                } else {
                                                    this.expanded_layout = Some(name.to_string());
                                                }
                                                cx.notify();
                                            }))
                                            .child(
                                                gpui::div()
                                                    .text_size(px(11.0))
                                                    .font_weight(if is_selected { gpui::FontWeight::BOLD } else { gpui::FontWeight::NORMAL })
                                                    .child(format!("{} {}", if is_expanded { "▼" } else { "▶" }, name)),
                                            ),
                                    )
                                    .when(is_expanded, |card| {
                                        let fields = self.render_layout_form_fields(name, theme);
                                        let phase_names = self.get_layout_phases(name);
                                        card.child(
                                            gpui::div()
                                                .p_2()
                                                .border_t(px(1.0))
                                                .border_color(theme.border)
                                                .bg(theme.panel_bg)
                                                .flex()
                                                .flex_col()
                                                .gap_2()
                                                .children(fields)
                                                .child(
                                                    Button::new(SharedString::from(format!("run-btn-{}", name)))
                                                        .primary()
                                                        .label(if self.is_layout_running && self.selected_layout == name { "COMPUTING..." } else { "RUN FULL LAYOUT" })
                                                        .disabled(self.is_layout_running)
                                                        .on_click(cx.listener(move |this, _, _, cx| {
                                                            if !this.is_layout_running {
                                                                this.selected_layout = name.to_string();
                                                                this.trigger_layout(cx);
                                                            }
                                                        })),
                                                )
                                                .when(!phase_names.is_empty(), |b| {
                                                    b.child(
                                                        gpui::div()
                                                            .flex()
                                                            .flex_col()
                                                            .gap_1()
                                                            .mt_1()
                                                            .child(
                                                                gpui::div()
                                                                    .text_color(theme.text)
                                                                    .text_size(px(10.0))
                                                                    .font_weight(gpui::FontWeight::BOLD)
                                                                    .child("STEP BY ALGORITHMIC PHASE:"),
                                                            )
                                                            .children(phase_names.into_iter().enumerate().map(|(phase_idx, phase_name)| {
                                                                let btn_id = format!("step-phase-btn-{}-{}", name, phase_idx);
                                                                Button::new(SharedString::from(btn_id))
                                                                    .label(SharedString::from(format!("▶ {}", phase_name)))
                                                                    .disabled(self.is_layout_running)
                                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                                        if !this.is_layout_running {
                                                                            this.selected_layout = name.to_string();
                                                                            this.trigger_step_specific_phase(phase_idx, cx);
                                                                        }
                                                                    }))
                                                            })),
                                                    )
                                                }),
                                        )
                                    })
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
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_size(px(12.0))
                            .child("3. LIVE PHYSICS (FORCEATLAS2)"),
                    )
                    .child(
                        Button::new("run-physics-toggle-btn")
                            .primary()
                            .label(if self.is_layout_running {
                                "LAYOUT RUNNING..."
                            } else if self.physics_enabled {
                                "STOP PHYSICS"
                            } else {
                                "RUN PHYSICS"
                            })
                            .disabled(self.is_layout_running)
                            .on_click(cx.listener(|this, _, _, cx| {
                                if !this.is_layout_running {
                                    this.physics_enabled = !this.physics_enabled;
                                    if this.physics_enabled {
                                        this.physics_temperature = 10.0;
                                        this.reset_physics();
                                    } else {
                                        this.engine.send_command(graphene_layout::GraphCommand::StopLiveSim).ok();
                                    }
                                    cx.notify();
                                }
                            })),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Stop Condition"),
                            )
                            .child(
                                Button::new("stop-condition-mode-btn")
                                    .label(match self.fa2_stop_mode {
                                        0 => "Auto Stop",
                                        1 => "Temp-Cooled",
                                        2 => "Iterations",
                                        _ => "Auto Stop",
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.fa2_stop_mode = (this.fa2_stop_mode + 1) % 3;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Scaling Ratio"),
                            )
                            .child(
                                gpui::div()
                                    .w(px(50.0))
                                    .child(Input::new(&self.input_fa2_scaling)),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("LinLog Mode"),
                            )
                            .child(
                                Button::new("toggle-linlog-btn")
                                    .label(if self.fa2_lin_log { "ON" } else { "OFF" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.fa2_lin_log = !this.fa2_lin_log;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Dissuade Hubs"),
                            )
                            .child(
                                Button::new("toggle-outbound-btn")
                                    .label(if self.fa2_outbound { "ON" } else { "OFF" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.fa2_outbound = !this.fa2_outbound;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Strong Gravity"),
                            )
                            .child(
                                Button::new("toggle-strong-gravity-btn")
                                    .label(if self.fa2_strong_gravity { "ON" } else { "OFF" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.fa2_strong_gravity = !this.fa2_strong_gravity;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Telemetry HUD"),
                            )
                            .child(
                                Button::new("toggle-hud-btn")
                                    .label(if self.show_performance_hud { "ON" } else { "OFF" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.show_performance_hud = !this.show_performance_hud;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Adjust Sizes"),
                            )
                            .child(
                                Button::new("toggle-adjust-sizes-btn")
                                    .label(if self.fa2_adjust_sizes { "ON" } else { "OFF" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.fa2_adjust_sizes = !this.fa2_adjust_sizes;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Barnes-Hut"),
                            )
                            .child(
                                Button::new("toggle-barnes-hut-btn")
                                    .label(if self.use_barnes_hut { "ON" } else { "OFF" })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.use_barnes_hut = !this.use_barnes_hut;
                                        cx.notify();
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
                            .text_size(px(12.0))
                            .child("4. FONT & TEXT CONFIG"),
                    )
                    .child(
                        gpui::div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .p_2()
                            .bg(theme.bg)
                            .rounded_md()
                            .border(px(1.0))
                            .border_color(theme.border)
                            .child(
                                gpui::div()
                                    .text_color(theme.text_dim)
                                    .text_size(px(11.0))
                                    .child("Max Label Length"),
                            )
                            .child(
                                gpui::div()
                                    .w(px(50.0))
                                    .child(Input::new(&self.input_max_len)),
                            ),
                    ),
            )
            .child(
                Button::new("run-layout-btn")
                    .primary()
                    .label("RUN LAYOUT")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.trigger_layout(cx);
                    })),
            )
            .child(
                gpui::div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("fit-view-btn")
                            .label("FIT VIEW")
                            .on_click(cx.listener(|this, _, _, _| {
                                this.fit_view();
                            })),
                    )
                    .child(
                        Button::new("reset-zoom-btn")
                            .label("RESET")
                            .on_click(cx.listener(|this, _, _, _| {
                                 this.viewport.offset = graphene_core::Vec2::default();
                                this.viewport.zoom = 1.0;
                            })),
                    ),
            )
    }

    pub fn render_layout_form_fields(&self, layout: &str, theme: &Theme) -> Vec<impl IntoElement> {
        let mut fields = Vec::new();

        let make_field =
            |label: &'static str, input: &gpui::Entity<gpui_component::input::InputState>| {
                gpui::div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(10.0))
                            .child(label),
                    )
                    .child(Input::new(input))
            };

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
            fields.push(make_field("Gravity", &self.input_gravity));
            fields.push(make_field("Repulsion (k_rep)", &self.input_k_rep));
            fields.push(make_field("Attraction (k_att)", &self.input_k_att));
            fields.push(make_field("Iterations", &self.input_iterations));
            fields.push(make_field("Barnes-Hut Theta", &self.input_theta));
        }

        if layout == "fCoSE" {
            fields.push(make_field("Iterations", &self.input_iterations));
            fields.push(make_field("Gravity", &self.input_gravity));
            fields.push(make_field("Node Repulsion", &self.input_k_rep));
            fields.push(make_field("Compound Padding", &self.input_compound_padding));
        }

        if layout == "Circle" {
            fields.push(make_field("Circle Radius", &self.input_circle_radius));
        }

        if layout == "Sugiyama" {
            fields.push(make_field("Layer Spacing", &self.input_layer_spacing));
            fields.push(make_field("Node Spacing", &self.input_node_spacing));
        }

        if layout == "MDS" {
            fields.push(make_field("Iterations", &self.input_iterations));
            fields.push(make_field("Base Distance", &self.input_mds_base_dist));
        }

        if layout == "Bipartite" {
            fields.push(make_field(
                "Column Spacing",
                &self.input_bipartite_col_spacing,
            ));
            fields.push(make_field(
                "Vertical Spacing",
                &self.input_bipartite_vert_spacing,
            ));
        }

        if matches!(layout, "KamadaKawai" | "CollisionForce") {
            fields.push(make_field("Iterations", &self.input_iterations));
            fields.push(make_field("Gravity", &self.input_gravity));
        }

        if layout == "DisconnectedPack" {
            fields.push(make_field("Packer Spacing", &self.input_packer_spacing));
        }

        if matches!(layout, "CoSE" | "Compound") {
            fields.push(make_field("Compound Padding", &self.input_compound_padding));
        }

        if layout == "RegionalPartition" {
            fields.push(make_field("Regional Columns", &self.input_regional_columns));
            fields.push(make_field(
                "Regional Cell Size",
                &self.input_regional_cell_size,
            ));
        }

        if fields.is_empty() {
            fields.push(make_field("Iterations", &self.input_iterations));
        }

        fields
    }
}
