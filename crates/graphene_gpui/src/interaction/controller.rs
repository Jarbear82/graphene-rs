use crate::interaction::expansion::ExpansionState;
use crate::interaction::state::InteractionState;
use crate::render::draw_pipeline::{Viewport, MAX_ZOOM, MIN_ZOOM};
use crate::view::GraphView;
use gpui::Point;
use graphene_core::{math::Vec2, EdgeId, NodeId};
use graphene_layout::engine::DragPhase;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ControllerPolicy {
    pub edge_hit_threshold: f32,
    pub double_click_interval_ms: u128,
    pub canvas_double_click_dist: f32,
    pub min_zoom: f32,
    pub max_zoom: f32,
    pub zoom_step: f32,
}

impl Default for ControllerPolicy {
    fn default() -> Self {
        Self {
            edge_hit_threshold: 8.0,
            double_click_interval_ms: 300,
            canvas_double_click_dist: 10.0,
            min_zoom: MIN_ZOOM,
            max_zoom: MAX_ZOOM,
            zoom_step: 1.15,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CanvasAction {
    CreateEdge { source: NodeId, target: NodeId },
    ToggleParentCollapse { parent_id: NodeId },
    AddNewNode { screen_pos: Point<f32> },
}

#[derive(Debug, Clone, Default)]
pub struct InteractionResult {
    pub selected_node: Option<Option<NodeId>>,
    pub selected_edge: Option<Option<usize>>,
    pub selected_edge_id: Option<Option<EdgeId>>,
    pub drag_update: Option<(NodeId, Vec2, DragPhase)>,
    pub action: Option<CanvasAction>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphCanvasController {
    pub last_node_click: Option<(NodeId, Instant)>,
    pub last_canvas_click: Option<(Point<f32>, Instant)>,
    pub policy: ControllerPolicy,
}

impl GraphCanvasController {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: ControllerPolicy) -> Self {
        Self {
            policy,
            ..Self::default()
        }
    }

    pub fn handle_mouse_down<S: Copy + Send + 'static>(
        &mut self,
        click_pos: Point<f32>,
        is_shift: bool,
        currently_selected_node: Option<NodeId>,
        viewport: &Viewport,
        view: &GraphView<S>,
        interaction_state: &mut InteractionState,
        expansion_state: &mut ExpansionState,
        physics_enabled: bool,
    ) -> InteractionResult {
        let mut result = InteractionResult::default();
        let hit_node = interaction_state.hit_test(click_pos, viewport, view, physics_enabled);
        let now = Instant::now();

        if let Some(node_id) = hit_node {
            if is_shift {
                if let Some(prev_selected) = currently_selected_node {
                    if prev_selected != node_id {
                        result.action = Some(CanvasAction::CreateEdge {
                            source: prev_selected,
                            target: node_id,
                        });
                        result.selected_node = Some(Some(node_id));
                        let drag = interaction_state.on_mouse_down(click_pos, hit_node, view);
                        result.drag_update = drag;
                        return result;
                    }
                }
            }

            if let Some((prev_id, prev_time)) = self.last_node_click {
                if prev_id == node_id
                    && now.duration_since(prev_time).as_millis() < self.policy.double_click_interval_ms
                {
                    let is_parent = view
                        .nodes
                        .get(&node_id)
                        .map_or(false, |n| !n.children.is_empty());
                    if is_parent {
                        expansion_state.toggle(node_id);
                        interaction_state.rebuild_grid(view);
                        self.last_node_click = None;
                        result.action = Some(CanvasAction::ToggleParentCollapse { parent_id: node_id });
                        let drag = interaction_state.on_mouse_down(click_pos, hit_node, view);
                        result.drag_update = drag;
                        return result;
                    }
                }
            }

            self.last_node_click = Some((node_id, now));
            result.selected_node = Some(Some(node_id));
            result.selected_edge = Some(None);
            result.selected_edge_id = Some(None);
        } else {
            self.last_node_click = None;
            let is_double_click = if let Some((prev_pos, prev_time)) = self.last_canvas_click {
                now.duration_since(prev_time).as_millis() < 350
                    && (prev_pos.x - click_pos.x).abs() < self.policy.canvas_double_click_dist
                    && (prev_pos.y - click_pos.y).abs() < self.policy.canvas_double_click_dist
            } else {
                false
            };

            if is_double_click {
                self.last_canvas_click = None;
                result.action = Some(CanvasAction::AddNewNode { screen_pos: click_pos });
                let drag = interaction_state.on_mouse_down(click_pos, hit_node, view);
                result.drag_update = drag;
                return result;
            } else {
                self.last_canvas_click = Some((click_pos, now));
            }

            let hit_edge = interaction_state.hit_test_edge(
                click_pos,
                viewport,
                view,
                self.policy.edge_hit_threshold,
            );

            if let Some(edge_id) = hit_edge {
                let pos = view.edge_order.iter().position(|&e| e == edge_id);
                result.selected_edge = Some(pos);
                result.selected_edge_id = Some(Some(edge_id));
                result.selected_node = Some(None);
            } else {
                result.selected_node = Some(None);
                result.selected_edge = Some(None);
                result.selected_edge_id = Some(None);
            }
        }

        let drag = interaction_state.on_mouse_down(click_pos, hit_node, view);
        result.drag_update = drag;
        result
    }

    pub fn handle_mouse_move<S: Copy + Send + 'static>(
        &mut self,
        mouse_pos: Point<f32>,
        viewport: &mut Viewport,
        view: &GraphView<S>,
        interaction_state: &mut InteractionState,
    ) -> Option<(NodeId, Vec2, DragPhase)> {
        interaction_state.on_mouse_drag(mouse_pos, viewport, view)
    }

    pub fn handle_mouse_up<S: Copy + Send + 'static>(
        &mut self,
        interaction_state: &mut InteractionState,
        view: &GraphView<S>,
    ) -> Option<(NodeId, Vec2, DragPhase)> {
        let drag_update = interaction_state.on_mouse_up();
        interaction_state.rebuild_grid(view);
        drag_update
    }

    pub fn handle_scroll(&self, delta_y: f32, viewport: &mut Viewport) {
        let zoom_factor = if delta_y > 0.0 {
            self.policy.zoom_step
        } else {
            1.0 / self.policy.zoom_step
        };
        viewport.zoom = (viewport.zoom * zoom_factor).clamp(self.policy.min_zoom, self.policy.max_zoom);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{math::Size2, GraphState};
    use graphene_style::ComputedStyle;

    #[test]
    fn test_controller_mouse_down_selects_node() {
        let mut state = GraphState::<ComputedStyle>::new();
        let node_id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));
        let view = GraphView::from_state(&state);
        let bounds = gpui::Bounds {
            origin: gpui::Point { x: 0.0, y: 0.0 },
            size: gpui::Size {
                width: 800.0,
                height: 600.0,
            },
        };
        let viewport = Viewport::new(bounds);
        let mut interaction = InteractionState::new(60.0);
        interaction.rebuild_grid(&view);
        let mut expansion = ExpansionState::new();

        let mut controller = GraphCanvasController::new();
        let screen_pos = viewport.model_to_screen(Vec2::new(0.0, 0.0));

        let res = controller.handle_mouse_down(
            screen_pos,
            false,
            None,
            &viewport,
            &view,
            &mut interaction,
            &mut expansion,
            false,
        );

        assert_eq!(res.selected_node, Some(Some(node_id)));
    }

    #[test]
    fn test_scroll_zoom() {
        let mut viewport = Viewport::new(gpui::Bounds {
            origin: gpui::Point { x: 0.0, y: 0.0 },
            size: gpui::Size {
                width: 800.0,
                height: 600.0,
            },
        });
        let controller = GraphCanvasController::new();
        let initial_zoom = viewport.zoom;
        controller.handle_scroll(10.0, &mut viewport);
        assert!(viewport.zoom > initial_zoom);
    }
}
