use crate::app::DemoApp;
use crate::theme::Theme;
use gpui::{
    px, Context, EntityInputHandler, InteractiveElement, IntoElement, MouseDownEvent,
    ParentElement, Render, Styled, Window,
};
use graphene_core::HierarchyExt;
use graphene_gpui::render::graph_canvas::GraphCanvas;

impl Render for DemoApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.get_theme();

        let max_len = self.get_max_untruncated_len();
        let fixture = &self.fixtures[self.selected_fixture_idx];

        for (idx, &id) in self.state.node_index_to_id.iter().enumerate() {
            let is_parent_node = self.state.is_parent(idx);
            let is_collapsed = self.collapsed_parents.contains(&id);

            if is_parent_node && !is_collapsed {
                continue;
            }

            let mut label = fixture.node_labels.get(&id).cloned().unwrap_or_default();
            if is_parent_node && is_collapsed {
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

        let snap = self.engine.latest_snapshot();
        if snap.version > 0 && snap.positions.len() == self.state.node_index_to_id.len() {
            let drag_node_id = self.interaction_state.drag_start.map(|(id, _, _)| id);
            for (i, &pos) in snap.positions.iter().enumerate() {
                let id = self.state.node_index_to_id[i];
                if Some(id) != drag_node_id {
                    self.state.positions.set(i, pos);
                }
            }
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
                self.engine.send_command(graphene_layout::GraphCommand::StepLiveSim).ok();
            }

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
                        if ev.modifiers.shift {
                            if let Some(prev_selected) = this.selected_node {
                                if prev_selected != node_id {
                                    this.create_edge_between_nodes(prev_selected, node_id);
                                    this.selected_node = Some(node_id);
                                    cx.notify();
                                    return;
                                }
                            }
                        }

                        if let Some((prev_id, prev_time)) = this.last_node_click {
                            if prev_id == node_id && now.duration_since(prev_time).as_millis() < 300 {
                                let is_parent = this
                                    .state
                                    .node_keys
                                    .get(node_id)
                                    .map(|&idx| this.state.is_parent(idx))
                                    .unwrap_or(false);
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
                        this.state.selected.select_node(node_id, &this.state.node_keys);
                        this.selected_node = this.state.selected.primary_node();
                        this.selected_edge = None;
                        if this.physics_enabled {
                            this.physics_temperature = 5.0;
                        }

                        if let (Some(p_id), Some(s_id)) = (this.state.selected.primary_node(), this.state.selected.secondary_node()) {
                            let p_label = this.state.get_node_label(p_id)
                                .map(|s| s.to_string())
                                .or_else(|| this.fixtures[this.selected_fixture_idx].node_labels.get(&p_id).cloned())
                                .unwrap_or_else(|| format!("N{}", this.state.node_keys[p_id]));

                            let s_label = this.state.get_node_label(s_id)
                                .map(|s| s.to_string())
                                .or_else(|| this.fixtures[this.selected_fixture_idx].node_labels.get(&s_id).cloned())
                                .unwrap_or_else(|| format!("N{}", this.state.node_keys[s_id]));

                            this.edge_src_state.update(cx, |input, cx| {
                                let len = input.text().len();
                                input.replace_text_in_range(Some(0..len), &p_label, window, cx);
                            });
                            this.edge_tgt_state.update(cx, |input, cx| {
                                let len = input.text().len();
                                input.replace_text_in_range(Some(0..len), &s_label, window, cx);
                            });
                        } else if let Some(p_id) = this.state.selected.primary_node() {
                            let label = this.state.get_node_label(p_id)
                                .map(|s| s.to_string())
                                .or_else(|| this.fixtures[this.selected_fixture_idx].node_labels.get(&p_id).cloned())
                                .unwrap_or_else(|| format!("N{}", this.state.node_keys[p_id]));
                            this.node_name_state.update(cx, |input, cx| {
                                let len = input.text().len();
                                input.replace_text_in_range(Some(0..len), &label, window, cx);
                            });
                        }
                    } else {
                        this.last_node_click = None;
                        let now = std::time::Instant::now();
                        let click_pos = gpui::point(f32::from(ev.position.x), f32::from(ev.position.y));
                        let is_double_click = if let Some((prev_pos, prev_time)) = this.last_canvas_click {
                            now.duration_since(prev_time).as_millis() < 350
                                && (prev_pos.x - click_pos.x).abs() < 10.0
                                && (prev_pos.y - click_pos.y).abs() < 10.0
                        } else {
                            false
                        };

                        if is_double_click {
                            this.last_canvas_click = None;
                            this.undo_redo.record_state(&this.state);
                            let label = format!("Node {}", this.state.node_count() + 1);
                            let new_id = this.interaction_state.on_double_click(
                                click_pos,
                                &this.viewport,
                                &mut this.state,
                                &label,
                            );
                            this.fixtures[this.selected_fixture_idx]
                                .node_labels
                                .insert(new_id, label.clone());
                            this.state.selected.select_node(new_id, &this.state.node_keys);
                            this.selected_node = Some(new_id);
                            this.selected_edge = None;
                            this.node_name_state.update(cx, |input, cx| {
                                let len = input.text().len();
                                input.replace_text_in_range(Some(0..len), &label, window, cx);
                            });
                            this.run_analysis();
                            this.engine.load_preset(this.state.clone());
                            cx.notify();
                            return;
                        } else {
                            this.last_canvas_click = Some((click_pos, now));
                        }

                        let hit_edge = this.interaction_state.hit_test_edge(
                            click_pos,
                            &this.viewport,
                            &this.state,
                            8.0,
                        );

                        if let Some(edge_idx) = hit_edge {
                            this.selected_edge = Some(edge_idx);
                            this.state.selected.select_edge(edge_idx);
                            this.selected_node = None;
                        } else {
                            this.selected_node = None;
                            this.selected_edge = None;
                            this.state.selected.clear();
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

                if let Some((drag_id, _, _)) = this.interaction_state.drag_start {
                    if let Some(&drag_idx) = this.state.node_keys.get(drag_id) {
                        let dragged_pos = *this.state.positions.get(drag_idx);
                        this.engine
                            .send_command(graphene_layout::GraphCommand::SetPosition {
                                id: drag_id,
                                pos: dragged_pos,
                            })
                            .ok();
                    }
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
                    if let Some((drag_id, _, _)) = this.interaction_state.drag_start {
                        if let Some(&drag_idx) = this.state.node_keys.get(drag_id) {
                            let final_pos = *this.state.positions.get(drag_idx);
                            this.engine
                                .send_command(graphene_layout::GraphCommand::SetPosition {
                                    id: drag_id,
                                    pos: final_pos,
                                })
                                .ok();
                        }
                    }
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
                let zoom_factor = if amount > 0.0 { 1.15 } else { 1.0 / 1.15 };
                this.viewport.zoom = (this.viewport.zoom * zoom_factor).clamp(
                    graphene_gpui::render::draw_pipeline::MIN_ZOOM,
                    graphene_gpui::render::draw_pipeline::MAX_ZOOM,
                );
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
