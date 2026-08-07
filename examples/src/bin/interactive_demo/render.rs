use crate::app::DemoApp;
use crate::theme::Theme;
use gpui::{
    px, Context, EntityInputHandler, InteractiveElement, IntoElement, MouseDownEvent,
    ParentElement, Render, Styled, Window,
};
use graphene_core::math::{Size2, Vec2};
use graphene_core::NodeId;
use graphene_gpui::render::graph_canvas::GraphCanvas;

impl Render for DemoApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let render_start = std::time::Instant::now();
        let now = std::time::Instant::now();
        let delta = now.duration_since(self.last_frame_instant).as_secs_f64();
        self.last_frame_instant = now;
        if delta > 0.0001 {
            let current_fps = 1.0 / delta;
            self.telemetry_fps = self.telemetry_fps * 0.9 + current_fps * 0.1;
        }

        self.drain_updates_and_sync();

        let theme = self.get_theme();
        let max_len = self.get_max_untruncated_len();
        self.view.measure_and_cache_node_sizes(
            cx.text_system(),
            14.0,
            max_len,
            &self.collapsed_parents,
        );
        let fixture = &self.fixtures[self.selected_fixture_idx];

        let mut formatted_count = 0;
        let mut visible_count = 0;

        for &id in &self.view.node_order {
            let Some(node) = self.view.nodes.get(&id) else {
                continue;
            };
            let is_parent_node = !node.children.is_empty();
            let is_collapsed = self.collapsed_parents.contains(&id);

            if is_parent_node && !is_collapsed {
                continue;
            }

            if self.viewport.is_visible(node.pos, node.size) {
                visible_count += 1;
                formatted_count += 1;
            }
        }

        self.telemetry_visible_nodes = visible_count;
        self.telemetry_labels_formatted = formatted_count;
        self.telemetry_render_ms = render_start.elapsed().as_secs_f64() * 1000.0;

        let sim_converged = self.live_sim.is_converged();
        let needs_physics = self.physics_enabled
            && (!sim_converged || self.interaction_state.drag_session.is_some());

        if needs_physics {
            self.run_physics_step();

            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                this.update(cx, |this, cx| {
                    if this.physics_enabled {
                        if this.interaction_state.drag_session.is_some() {
                            this.physics_temperature = 10.0;
                        } else {
                            this.physics_temperature *= 0.95;
                        }
                    }
                    cx.notify();
                })
                .ok();
            })
            .detach();
        }

        gpui::div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.bg)
            .child(self.render_title_bar(&theme))
            .child(
                gpui::div()
                    .flex()
                    .flex_1()
                    .h(px(0.0))
                    .child(self.render_sidebar_left(&theme, cx))
                    .child(self.render_canvas_view(&theme, _window, cx))
                    .child(self.render_sidebar_right(&theme, _window, cx)),
            )
            .child(self.render_bottom_bar(&theme))
    }
}

impl DemoApp {
    fn render_title_bar(&self, theme: &Theme) -> impl IntoElement {
        use gpui_component::TitleBar;

        TitleBar::new()
            .bg(theme.panel_bg)
            .border_color(theme.border)
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        gpui::div()
                            .w(px(12.0))
                            .h(px(12.0))
                            .rounded_full()
                            .bg(theme.accent),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text)
                            .font_weight(gpui::FontWeight::BOLD)
                            .child("Graphene-RS Interactive Visualizer"),
                    ),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child({
                        let zoom_percent = self.viewport.zoom * 100.0;
                        let zoom_str = if zoom_percent >= 10.0 {
                            format!("{:.0}%", zoom_percent)
                        } else if zoom_percent >= 0.01 {
                            format!("{:.2}%", zoom_percent)
                        } else {
                            format!("{:.5}%", zoom_percent)
                        };
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(12.0))
                            .child(format!("Zoom: {}", zoom_str))
                    })
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(12.0))
                            .child("Status: Live (Message-Passing Engine)"),
                    ),
            )
    }

    fn render_canvas_view(
        &self,
        theme: &Theme,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak_entity = cx.weak_entity();
        let fixture = &self.fixtures[self.selected_fixture_idx];

        gpui::div()
            .id("canvas-container")
            .flex_1()
            .h_full()
            .relative()
            .overflow_hidden()
            .bg(theme.bg)
            .child(
                gpui::canvas(
                    move |bounds, _, cx| {
                        if let Some(entity) = weak_entity.upgrade() {
                            entity.update(cx, |this, _| {
                                this.viewport.bounds = gpui::Bounds {
                                    origin: gpui::point(
                                        f32::from(bounds.origin.x),
                                        f32::from(bounds.origin.y),
                                    ),
                                    size: gpui::size(
                                        f32::from(bounds.size.width),
                                        f32::from(bounds.size.height),
                                    ),
                                };
                            });
                        }
                    },
                    move |_, _, _, _| {},
                )
                .size_full()
                .absolute(),
            )
            .child(
                graphene_gpui::GraphCanvasHost::new(
                    &self.view,
                    &self.viewport,
                    &self.interaction_state,
                    &self.themes.themes[self.current_theme_idx],
                    self.selected_node,
                    &fixture.node_labels,
                    &fixture.edge_labels,
                    self.get_max_untruncated_len(),
                    &self.collapsed_parents,
                )
                .with_directed(self.is_directed)
                .with_centrality_scores(self.get_active_centrality_map())
                .with_config(self.get_canvas_config()),
            )
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, window, cx| {
                    let click_pos = gpui::point(f32::from(ev.position.x), f32::from(ev.position.y));
                    let mut controller = this.controller.clone();
                    let mut interaction = this.interaction_state.clone();
                    let mut expansion = this.collapsed_parents.clone();

                    let res = controller.handle_mouse_down(
                        click_pos,
                        ev.modifiers.shift,
                        this.selected_node,
                        &this.viewport,
                        &this.view,
                        &mut interaction,
                        &mut expansion,
                        this.physics_enabled,
                    );

                    this.controller = controller;
                    this.interaction_state = interaction;
                    this.collapsed_parents = expansion;

                    if let Some(sel_node) = res.selected_node {
                        this.selected_node = sel_node;
                    }
                    if let Some(sel_edge) = res.selected_edge {
                        this.selected_edge = sel_edge;
                    }
                    if let Some((drag_id, target_pos, phase)) = res.drag_update {
                        this.engine.drag_node_target(drag_id, target_pos, phase);
                        if this.physics_enabled {
                            this.physics_temperature = 5.0;
                        }
                    }

                    if let Some(action) = res.action {
                        match action {
                            graphene_gpui::CanvasAction::CreateEdge { source, target } => {
                                this.create_edge_between_nodes(source, target);
                                this.selected_node = Some(target);
                            }
                            graphene_gpui::CanvasAction::ToggleParentCollapse { parent_id: _ } => {
                                this.physics_temperature = 5.0;
                            }
                            graphene_gpui::CanvasAction::AddNewNode { screen_pos: _ } => {
                                this.add_new_node(window, cx);
                            }
                        }
                    }

                    if let Some(p_id) = this.selected_node {
                        let label = this
                            .view
                            .nodes
                            .get(&p_id)
                            .map(|n| n.label.clone())
                            .unwrap_or_else(|| format!("N{:?}", p_id));
                        this.node_name_state.update(cx, |input, cx| {
                            let len = input.text().len();
                            input.replace_text_in_range(Some(0..len), &label, window, cx);
                        });
                    }

                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                let mouse_pos = gpui::point(f32::from(ev.position.x), f32::from(ev.position.y));
                let mut interaction = this.interaction_state.clone();
                let mut vp = this.viewport.clone();

                if let Some((drag_id, target_pos, phase)) =
                    this.controller.handle_mouse_move(mouse_pos, &mut vp, &this.view, &mut interaction)
                {
                    this.engine.drag_node_target(drag_id, target_pos, phase);
                }

                this.interaction_state = interaction;
                this.viewport = vp;
                cx.notify();
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    let mut interaction = this.interaction_state.clone();
                    if let Some((node_id, target_pos, phase)) =
                        this.controller.handle_mouse_up(&mut interaction, &this.view)
                    {
                        this.engine.drag_node_target(node_id, target_pos, phase);
                    }
                    this.interaction_state = interaction;
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev: &gpui::ScrollWheelEvent, _, cx| {
                let amount = match ev.delta {
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.y),
                    gpui::ScrollDelta::Lines(p) => p.y * 20.0,
                };
                let mut vp = this.viewport.clone();
                this.controller.handle_scroll(amount, &mut vp);
                this.viewport = vp;
                cx.notify();
            }))
            .children(self.render_telemetry_hud(theme))
    }

    fn render_telemetry_hud(&self, theme: &Theme) -> Option<impl IntoElement> {
        if !self.show_performance_hud {
            return None;
        }

        let fps_text = format!("{:.1} FPS ({:.2} ms)", self.telemetry_fps, 1000.0 / self.telemetry_fps.max(1.0));
        let physics_text = format!("{:.3} ms", self.telemetry_physics_ms);
        let render_text = format!("{:.3} ms", self.telemetry_render_ms);
        let scale_text = format!("{} Nodes | {} Edges", self.view.nodes.len(), self.view.edges.len());
        let culling_text = format!("{} Visible / {} Formatted Labels", self.telemetry_visible_nodes, self.telemetry_labels_formatted);

        Some(
            gpui::div()
                .absolute()
                .top(px(12.0))
                .right(px(12.0))
                .bg(theme.panel_bg)
                .border(px(1.0))
                .border_color(theme.border)
                .px(px(12.0))
                .py(px(8.0))
                .rounded_lg()
                .shadow_md()
                .flex()
                .flex_col()
                .gap_1()
                .text_size(px(11.0))
                .child(
                    gpui::div()
                        .text_color(theme.accent)
                        .font_weight(gpui::FontWeight::BOLD)
                        .child("⚡ Telemetry HUD (Press 'H' to toggle)"),
                )
                .child(
                    gpui::div()
                        .text_color(theme.text)
                        .child(format!("Frame Rate: {}", fps_text)),
                )
                .child(
                    gpui::div()
                        .text_color(theme.text_dim)
                        .child(format!("Physics Tick: {}", physics_text)),
                )
                .child(
                    gpui::div()
                        .text_color(theme.text_dim)
                        .child(format!("Render Time: {}", render_text)),
                )
                .child(
                    gpui::div()
                        .text_color(theme.text)
                        .child(format!("Engine Threads: {} ({})", self.telemetry_worker_threads, self.telemetry_worker_state)),
                )
                .child(
                    gpui::div()
                        .text_color(theme.text_dim)
                        .child(scale_text),
                )
                .child(
                    gpui::div()
                        .text_color(theme.text_dim)
                        .child(culling_text),
                ),
        )
    }

    fn render_bottom_bar(&self, theme: &Theme) -> impl IntoElement {
        gpui::div()
            .h(px(28.0))
            .bg(theme.panel_bg)
            .border_t(px(1.0))
            .border_color(theme.border)
            .px_3()
            .flex()
            .items_center()
            .justify_between()
            .text_xs()
            .text_color(theme.text_dim)
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_4()
                    .child(format!("Nodes: {} | Edges: {}", self.view.nodes.len(), self.view.edges.len()))
                    .child(format!("Layout: {}", self.selected_layout))
                    .child(format!("Physics: {}", if self.physics_enabled { "ON" } else { "OFF" })),
            )
            .child("Single Source of Truth Engine Active")
    }
}
