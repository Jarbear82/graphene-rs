use crate::app::DemoApp;
use crate::theme::{distance_to_segment, Theme};
use gpui::{
    px, Context, EntityInputHandler, InteractiveElement, IntoElement, MouseDownEvent,
    ParentElement, Render, Styled, Window,
};
use graphene_gpui::render::graph_canvas::GraphCanvas;

impl Render for DemoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.get_theme();

        let max_len = self.get_max_untruncated_len();
        let fixture = &self.fixtures[self.selected_fixture_idx];

        let nodes_count = self.state.node_index_to_id.len();
        let mut is_parent = vec![false; nodes_count];
        for idx in 0..nodes_count {
            let id = self.state.node_index_to_id[idx];
            for j in 0..nodes_count {
                if let Some(p_id) = *self.state.hierarchy.parent.get(j) {
                    if p_id == id {
                        is_parent[idx] = true;
                        break;
                    }
                }
            }
        }

        for (idx, &id) in self.state.node_index_to_id.iter().enumerate() {
            let is_collapsed = self.collapsed_parents.contains(&id);

            if is_parent[idx] && !is_collapsed {
                continue;
            }

            let mut label = fixture.node_labels.get(&id).cloned().unwrap_or_default();
            if is_parent[idx] && is_collapsed {
                label = format!("[+] {}", label);
            }
            let label_len = label.chars().count();

            let is_selected = self.selected_node == Some(id);
            let target_w;
            if label_len > max_len {
                if is_selected {
                    target_w = 40.0 + (label_len as f32) * 6.0;
                } else {
                    target_w = 40.0 + (max_len as f32) * 6.0;
                }
            } else {
                target_w = 40.0 + (label_len as f32) * 6.0;
            }

            let size = self.state.sizes.get_mut(idx);
            size.w = target_w;
        }

        let is_animating = !self.state.animations.tracks.is_empty();
        let needs_physics = self.physics_enabled
            && (self.physics_temperature > 0.05 || self.interaction_state.drag_start.is_some());
        let needs_tick = is_animating || needs_physics;

        if needs_tick {
            if is_animating {
                self.state
                    .tick_animations(std::time::Duration::from_millis(16));
                if self.state.animations.tracks.is_empty() {
                    self.interaction_state.rebuild_grid(&self.state);
                }
            } else if needs_physics {
                self.run_physics_step();
                if self.physics_temperature <= 0.05 {
                    self.interaction_state.rebuild_grid(&self.state);
                }
            }

            self.resolve_collisions();
            graphene_layout::resolve_compound_bounds(
                &mut self.state,
                &self.collapsed_parents,
                20.0,
            );

            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(16))
                    .await;
                this.update(cx, |this, cx| {
                    if this.physics_enabled && !is_animating {
                        if this.interaction_state.drag_start.is_some() {
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
                    .child(self.render_canvas_view(&theme, window, cx))
                    .child(self.render_sidebar_right(&theme, window, cx)),
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
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(12.0))
                            .child(format!("Zoom: {:.0}%", self.viewport.zoom * 100.0)),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(12.0))
                            .child("Status: Live (Animated)"),
                    ),
            )
    }

    fn render_canvas_view(
        &self,
        theme: &Theme,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let weak_entity = cx.weak_entity();
        let fixture = &self.fixtures[self.selected_fixture_idx];

        gpui::div()
            .id("canvas-container")
            .flex_1()
            .h_full()
            .relative()
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
                GraphCanvas::new(
                    &self.state,
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
                    let hit_node = this.interaction_state.hit_test(
                        gpui::point(f32::from(ev.position.x), f32::from(ev.position.y)),
                        &this.viewport,
                        &this.state,
                        this.physics_enabled,
                    );
                    let now = std::time::Instant::now();
                    if let Some(node_id) = hit_node {
                        if let Some((prev_id, prev_time)) = this.last_node_click {
                            if prev_id == node_id && now.duration_since(prev_time).as_millis() < 300
                            {
                                let mut is_parent = false;
                                for j in 0..this.state.node_index_to_id.len() {
                                    if let Some(p_id) = *this.state.hierarchy.parent.get(j) {
                                        if p_id == node_id {
                                            is_parent = true;
                                            break;
                                        }
                                    }
                                }
                                if is_parent {
                                    this.undo_redo.record_state(&this.state);
                                    if this.collapsed_parents.contains(&node_id) {
                                        this.collapsed_parents.remove(&node_id);
                                    } else {
                                        this.collapsed_parents.insert(node_id);
                                    }
                                    graphene_layout::resolve_compound_bounds(
                                        &mut this.state,
                                        &this.collapsed_parents,
                                        20.0,
                                    );
                                    this.physics_temperature = 5.0;
                                    this.interaction_state.rebuild_grid(&this.state);
                                    this.last_node_click = None;
                                    cx.notify();
                                    return;
                                }
                            }
                        }
                        this.last_node_click = Some((node_id, now));

                        this.undo_redo.record_state(&this.state);
                        this.selected_node = Some(node_id);
                        this.selected_edge = None;
                        if this.physics_enabled {
                            this.physics_temperature = 5.0;
                        }

                        let label = this.fixtures[this.selected_fixture_idx]
                            .node_labels
                            .get(&node_id)
                            .cloned()
                            .unwrap_or_else(|| format!("N{}", this.state.node_keys[node_id]));
                        this.node_name_state.update(cx, |input, cx| {
                            input.replace_text_in_range(None, &label, window, cx);
                        });
                    } else {
                        this.last_node_click = None;
                        let mut hit_edge = None;
                        for edge_idx in 0..this.state.edges.len() {
                            let src = *this.state.edge_sources.get(edge_idx);
                            let tgt = *this.state.edge_targets.get(edge_idx);
                            let (Some(&src_idx), Some(&tgt_idx)) =
                                (this.state.node_keys.get(src), this.state.node_keys.get(tgt))
                            else {
                                continue;
                            };
                            let pos_src = *this.state.positions.get(src_idx);
                            let pos_tgt = *this.state.positions.get(tgt_idx);

                            let src_screen = this.viewport.model_to_screen(pos_src);
                            let tgt_screen = this.viewport.model_to_screen(pos_tgt);

                            let dist = distance_to_segment(
                                ev.position,
                                gpui::point(px(src_screen.x), px(src_screen.y)),
                                gpui::point(px(tgt_screen.x), px(tgt_screen.y)),
                            );
                            if dist < 8.0 {
                                hit_edge = Some(edge_idx);
                                break;
                            }
                        }

                        if let Some(edge_idx) = hit_edge {
                            this.selected_edge = Some(edge_idx);
                            this.selected_node = None;
                        } else {
                            this.selected_node = None;
                            this.selected_edge = None;
                        }
                    }

                    let mut is_mut = this.interaction_state.clone();
                    is_mut.on_mouse_down(
                        gpui::point(f32::from(ev.position.x), f32::from(ev.position.y)),
                        hit_node,
                        &this.state,
                    );
                    this.interaction_state = is_mut;
                    cx.notify();
                }),
            )
            .on_mouse_move(cx.listener(|this, ev: &gpui::MouseMoveEvent, _, cx| {
                let mut is_mut = this.interaction_state.clone();
                let mut vp_mut = this.viewport.clone();
                let mut st_mut = this.state.clone();

                is_mut.on_mouse_drag(
                    gpui::point(f32::from(ev.position.x), f32::from(ev.position.y)),
                    &mut vp_mut,
                    &mut st_mut,
                );

                this.interaction_state = is_mut;
                this.viewport = vp_mut;
                this.state = st_mut;

                if this.interaction_state.drag_start.is_some() {
                    this.resolve_collisions();
                    graphene_layout::resolve_compound_bounds(
                        &mut this.state,
                        &this.collapsed_parents,
                        20.0,
                    );
                    this.state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
                }
                cx.notify();
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    let mut is_mut = this.interaction_state.clone();
                    is_mut.on_mouse_up();
                    this.interaction_state = is_mut;
                    this.interaction_state.rebuild_grid(&this.state);
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev: &gpui::ScrollWheelEvent, _, cx| {
                let amount = match ev.delta {
                    gpui::ScrollDelta::Pixels(p) => f32::from(p.y),
                    gpui::ScrollDelta::Lines(p) => p.y * 20.0,
                };
                let zoom_factor = if amount > 0.0 { 1.05 } else { 0.95 };
                this.viewport.zoom *= zoom_factor;
                this.viewport.zoom = this.viewport.zoom.clamp(0.15, 8.0);
                cx.notify();
            }))
    }

    fn render_bottom_bar(&self, theme: &Theme) -> impl IntoElement {
        let nodes_count = self.state.node_index_to_id.len();
        let edges_count = self.state.edges.len();

        let selection_status = if let Some(node_id) = self.selected_node {
            let label = self.fixtures[self.selected_fixture_idx]
                .node_labels
                .get(&node_id)
                .cloned()
                .unwrap_or_else(|| format!("N{}", self.state.node_keys[node_id]));
            format!("Selected: Node {}", label)
        } else if let Some(edge_idx) = self.selected_edge {
            format!("Selected: Edge #{}", edge_idx)
        } else {
            "Selected: None".to_string()
        };

        let physics_status = if self.physics_enabled {
            format!("Physics: Active (T={:.2})", self.physics_temperature)
        } else {
            "Physics: Disabled".to_string()
        };

        gpui::div()
            .flex()
            .items_center()
            .justify_between()
            .h(px(26.0))
            .px(px(12.0))
            .bg(theme.panel_bg)
            .border_t(px(1.0))
            .border_color(theme.border)
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(format!("Nodes: {}  •  Edges: {}", nodes_count, edges_count)),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.border)
                            .text_size(px(11.0))
                            .child("|"),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.accent)
                            .text_size(px(11.0))
                            .child(selection_status),
                    ),
            )
            .child(
                gpui::div()
                    .text_color(theme.text_dim)
                    .text_size(px(11.0))
                    .italic()
                    .child("Tips: [Left-drag] nodes to move • [Drag bg] to pan • [Scroll] to zoom"),
            )
            .child(
                gpui::div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(physics_status),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.border)
                            .text_size(px(11.0))
                            .child("|"),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(format!("Layout: {}", self.selected_layout)),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.border)
                            .text_size(px(11.0))
                            .child("|"),
                    )
                    .child(
                        gpui::div()
                            .text_color(theme.text_dim)
                            .text_size(px(11.0))
                            .child(format!(
                                "Theme: {}",
                                self.themes.themes[self.current_theme_idx].name
                            )),
                    ),
            )
    }
}
