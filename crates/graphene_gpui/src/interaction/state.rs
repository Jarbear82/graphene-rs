use crate::render::draw_pipeline::Viewport;
use crate::view::GraphView;
use graphene_core::{
    math::{Size2, Vec2},
    EdgeId, NodeId,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SpatialHashGrid {
    pub cell_size: f32,
    pub cells: HashMap<(i32, i32), Vec<NodeId>>,
}

impl SpatialHashGrid {
    pub fn new(cell_size: f32) -> Self {
        Self {
            cell_size,
            cells: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
    }

    pub fn hash(&self, pos: Vec2) -> (i32, i32) {
        let cx = (pos.x / self.cell_size).floor() as i32;
        let cy = (pos.y / self.cell_size).floor() as i32;
        (cx, cy)
    }

    pub fn insert(&mut self, id: NodeId, pos: Vec2, size: Size2) {
        let half_w = size.w / 2.0;
        let half_h = size.h / 2.0;
        let min_x = pos.x - half_w;
        let max_x = pos.x + half_w;
        let min_y = pos.y - half_h;
        let max_y = pos.y + half_h;

        let start_cell = self.hash(Vec2::new(min_x, min_y));
        let end_cell = self.hash(Vec2::new(max_x, max_y));

        for cx in start_cell.0..=end_cell.0 {
            for cy in start_cell.1..=end_cell.1 {
                self.cells.entry((cx, cy)).or_default().push(id);
            }
        }
    }

    pub fn query(&self, pos: Vec2) -> Vec<NodeId> {
        let cell = self.hash(pos);
        self.cells.get(&cell).cloned().unwrap_or_default()
    }

    pub fn query_neighborhood(&self, pos: Vec2) -> Vec<NodeId> {
        let (cx, cy) = self.hash(pos);
        let mut candidates = Vec::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                if let Some(list) = self.cells.get(&(cx + dx, cy + dy)) {
                    candidates.extend_from_slice(list);
                }
            }
        }
        candidates
    }
}

#[derive(Debug, Clone)]
pub struct InteractionState {
    pub drag_start: Option<(NodeId, gpui::Point<f32>, Vec2)>, // grabbed node + mouse starting pos + node starting pos
    pub pan_origin: Option<gpui::Point<f32>>,                 // last pan start position
    pub spatial_grid: SpatialHashGrid,
    pub is_box_selecting: bool,
    pub box_select_rect: Option<gpui::Bounds<f32>>,
}

impl InteractionState {
    pub fn new(cell_size: f32) -> Self {
        Self {
            drag_start: None,
            pan_origin: None,
            spatial_grid: SpatialHashGrid::new(cell_size),
            is_box_selecting: false,
            box_select_rect: None,
        }
    }

    pub fn rebuild_grid<S: Copy + Send + 'static>(&mut self, view: &GraphView<S>) {
        self.spatial_grid.clear();
        for (&id, node) in &view.nodes {
            self.spatial_grid.insert(id, node.pos, node.size);
        }
    }

    pub fn hit_test<S: Copy + Send + 'static>(
        &self,
        screen_pos: gpui::Point<f32>,
        viewport: &Viewport,
        view: &GraphView<S>,
        physics_active: bool,
    ) -> Option<NodeId> {
        let model_pos = viewport.screen_to_model(screen_pos);

        let get_nesting_depth = |node_id: NodeId, v: &GraphView<S>| -> usize {
            let mut depth = 0;
            let mut curr = node_id;
            while let Some(node) = v.nodes.get(&curr) {
                if let Some(parent_id) = node.parent {
                    curr = parent_id;
                    depth += 1;
                } else {
                    break;
                }
            }
            depth
        };

        let mut best_match = None;
        let mut max_depth = 0;

        let candidates = if physics_active || self.spatial_grid.cells.is_empty() {
            let neighborhood = self.spatial_grid.query_neighborhood(model_pos);
            if neighborhood.is_empty() {
                view.node_order
                    .iter()
                    .copied()
                    .filter(|&id| {
                        if let Some(node) = view.nodes.get(&id) {
                            viewport.is_visible(node.pos, node.size)
                        } else {
                            false
                        }
                    })
                    .collect()
            } else {
                neighborhood
            }
        } else {
            self.spatial_grid.query(model_pos)
        };

        for id in candidates {
            if let Some(node) = view.nodes.get(&id) {
                let half_w = node.size.w / 2.0;
                let half_h = node.size.h / 2.0;
                if model_pos.x >= node.pos.x - half_w
                    && model_pos.x <= node.pos.x + half_w
                    && model_pos.y >= node.pos.y - half_h
                    && model_pos.y <= node.pos.y + half_h
                {
                    let depth = get_nesting_depth(id, view);
                    if best_match.is_none() || depth > max_depth {
                        best_match = Some(id);
                        max_depth = depth;
                    }
                }
            }
        }

        best_match
    }

    pub fn hit_test_edge<S: Copy + Send + 'static>(
        &self,
        screen_pos: gpui::Point<f32>,
        viewport: &Viewport,
        view: &GraphView<S>,
        threshold: f32,
    ) -> Option<EdgeId> {
        for &edge_id in &view.edge_order {
            if let Some(edge) = view.edges.get(&edge_id) {
                let (Some(src_node), Some(tgt_node)) = (view.nodes.get(&edge.source), view.nodes.get(&edge.target))
                else {
                    continue;
                };

                let src_screen = viewport.model_to_screen(src_node.pos);
                let tgt_screen = viewport.model_to_screen(tgt_node.pos);

                let dist = distance_to_segment(
                    screen_pos,
                    gpui::point(src_screen.x, src_screen.y),
                    gpui::point(tgt_screen.x, tgt_screen.y),
                );
                if dist < threshold {
                    return Some(edge_id);
                }
            }
        }
        None
    }

    pub fn on_mouse_down<S: Copy + Send + 'static>(
        &mut self,
        position: gpui::Point<f32>,
        hit_node: Option<NodeId>,
        view: &GraphView<S>,
    ) {
        if let Some(node_id) = hit_node {
            if let Some(node) = view.nodes.get(&node_id) {
                self.drag_start = Some((node_id, position, node.pos));
            }
        } else {
            self.pan_origin = Some(position);
        }
    }

    pub fn on_mouse_drag<S: Copy + Send + 'static>(
        &mut self,
        position: gpui::Point<f32>,
        viewport: &mut Viewport,
        view: &GraphView<S>,
    ) -> Option<(NodeId, Vec2)> {
        if let Some((id, start_mouse_pos, start_node_pos)) = self.drag_start {
            let total_mouse_delta = gpui::point(
                position.x - start_mouse_pos.x,
                position.y - start_mouse_pos.y,
            );
            let target_parent_pos = start_node_pos
                + Vec2::new(
                    total_mouse_delta.x / viewport.zoom,
                    total_mouse_delta.y / viewport.zoom,
                );
            if let Some(node) = view.nodes.get(&id) {
                let current_parent_pos = node.pos;
                let step_delta = target_parent_pos - current_parent_pos;
                return Some((id, step_delta));
            }
        } else if let Some(last_pos) = self.pan_origin {
            let delta = gpui::point(position.x - last_pos.x, position.y - last_pos.y);
            viewport.offset.x += delta.x / viewport.zoom;
            viewport.offset.y += delta.y / viewport.zoom;
            self.pan_origin = Some(position);
        }
        None
    }

    pub fn on_mouse_up(&mut self) {
        self.drag_start = None;
        self.pan_origin = None;
    }
}

pub fn update_node_shape() {
    // Styling stays on ComputedStyle / commands
}

pub fn update_edge_width() {
    // Styling stays on ComputedStyle / commands
}

pub fn distance_to_segment(p: gpui::Point<f32>, a: gpui::Point<f32>, b: gpui::Point<f32>) -> f32 {
    let ab_x = b.x - a.x;
    let ab_y = b.y - a.y;
    let ap_x = p.x - a.x;
    let ap_y = p.y - a.y;
    let ab_len_sq = ab_x * ab_x + ab_y * ab_y;
    if ab_len_sq == 0.0 {
        return ((p.x - a.x) * (p.x - a.x) + (p.y - a.y) * (p.y - a.y)).sqrt();
    }
    let t = ((ap_x * ab_x + ap_y * ab_y) / ab_len_sq).clamp(0.0, 1.0);
    let proj_x = a.x + t * ab_x;
    let proj_y = a.y + t * ab_y;
    ((p.x - proj_x) * (p.x - proj_x) + (p.y - proj_y) * (p.y - proj_y)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::GraphState;
    use graphene_style::ComputedStyle;

    #[test]
    fn test_hit_test_prioritizes_nested_children() {
        let mut state = GraphState::<ComputedStyle>::new();

        let parent_id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(200.0, 200.0));
        let child_id = state.add_node(Vec2::new(50.0, 50.0), Size2::new(40.0, 40.0));
        state.reparent_node(child_id, Some(parent_id));

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

        let screen_pos = viewport.model_to_screen(Vec2::new(50.0, 50.0));

        let hit_active = interaction.hit_test(screen_pos, &viewport, &view, true);
        assert_eq!(
            hit_active,
            Some(child_id),
            "Active hit_test did not prioritize nested child node!"
        );

        let hit_inactive = interaction.hit_test(screen_pos, &viewport, &view, false);
        assert_eq!(
            hit_inactive,
            Some(child_id),
            "Inactive hit_test did not prioritize nested child node!"
        );

        let screen_margin = viewport.model_to_screen(Vec2::new(-50.0, -50.0));

        let hit_margin_active = interaction.hit_test(screen_margin, &viewport, &view, true);
        assert_eq!(
            hit_margin_active,
            Some(parent_id),
            "Hit test at margin should match parent node!"
        );

        let hit_margin_inactive = interaction.hit_test(screen_margin, &viewport, &view, false);
        assert_eq!(
            hit_margin_inactive,
            Some(parent_id),
            "Hit test at margin should match parent node!"
        );
    }
}
