use graphene_core::{math::{Size2, Vec2}, GraphState, NodeId};
use graphene_style::ComputedStyle;
use crate::render::draw_pipeline::Viewport;
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
}

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

    pub fn rebuild_grid(&mut self, state: &GraphState<ComputedStyle>) {
        self.spatial_grid.clear();
        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let pos = *state.positions.get(idx);
            let size = *state.sizes.get(idx);
            self.spatial_grid.insert(id, pos, size);
        }
    }

    pub fn hit_test(
        &self,
        screen_pos: gpui::Point<f32>,
        viewport: &Viewport,
        state: &GraphState<ComputedStyle>,
        physics_active: bool,
    ) -> Option<NodeId> {
        let model_pos = viewport.screen_to_model(screen_pos);

        if physics_active {
            // Linear scan over visible-only candidates during active simulation
            for (idx, &id) in state.node_index_to_id.iter().enumerate() {
                let pos = *state.positions.get(idx);
                let size = *state.sizes.get(idx);
                if viewport.is_visible(pos, size) {
                    let half_w = size.w / 2.0;
                    let half_h = size.h / 2.0;
                    if model_pos.x >= pos.x - half_w
                        && model_pos.x <= pos.x + half_w
                        && model_pos.y >= pos.y - half_h
                        && model_pos.y <= pos.y + half_h
                    {
                        return Some(id);
                    }
                }
            }
        } else {
            // Query hash grid
            let candidates = self.spatial_grid.query(model_pos);
            for id in candidates {
                if let Some(&idx) = state.node_keys.get(id) {
                    let pos = *state.positions.get(idx);
                    let size = *state.sizes.get(idx);
                    let half_w = size.w / 2.0;
                    let half_h = size.h / 2.0;
                    if model_pos.x >= pos.x - half_w
                        && model_pos.x <= pos.x + half_w
                        && model_pos.y >= pos.y - half_h
                        && model_pos.y <= pos.y + half_h
                    {
                        return Some(id);
                    }
                }
            }
        }

        None
    }

    pub fn on_mouse_down(
        &mut self,
        position: gpui::Point<f32>,
        hit_node: Option<NodeId>,
        state: &GraphState<ComputedStyle>,
    ) {
        if let Some(node_id) = hit_node {
            if let Some(&idx) = state.node_keys.get(node_id) {
                let node_pos = *state.positions.get(idx);
                self.drag_start = Some((node_id, position, node_pos));
            }
        } else {
            self.pan_origin = Some(position);
        }
    }

    pub fn on_mouse_drag(
        &mut self,
        position: gpui::Point<f32>,
        viewport: &mut Viewport,
        state: &mut GraphState<ComputedStyle>,
    ) {
        if let Some((id, start_mouse_pos, start_node_pos)) = self.drag_start {
            let delta = gpui::point(
                position.x - start_mouse_pos.x,
                position.y - start_mouse_pos.y,
            );
            // Convert screen delta → model delta (taking zoom into account)
            let model_delta = Vec2::new(delta.x / viewport.zoom, delta.y / viewport.zoom);
            state.set_node_position(id, start_node_pos + model_delta);
        } else if let Some(last_pos) = self.pan_origin {
            let delta = gpui::point(
                position.x - last_pos.x,
                position.y - last_pos.y,
            );
            // Adjust viewport offset
            viewport.offset.x += delta.x / viewport.zoom;
            viewport.offset.y += delta.y / viewport.zoom;
            self.pan_origin = Some(position);
        }
    }

    pub fn on_mouse_up(&mut self) {
        self.drag_start = None;
        self.pan_origin = None;
    }
}
