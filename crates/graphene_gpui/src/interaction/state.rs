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

        let get_nesting_depth = |node_id: NodeId, h_state: &GraphState<ComputedStyle>| -> usize {
            let mut depth = 0;
            let mut curr = node_id;
            while let Some(&idx) = h_state.node_keys.get(curr) {
                if let Some(parent_id) = *h_state.hierarchy.parent.get(idx) {
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

        let candidates: Vec<NodeId> = if physics_active {
            // During active physics simulation positions update continuously;
            // scan visible nodes to guarantee 100% accurate hit-testing without stale cell misses.
            state
                .node_index_to_id
                .iter()
                .copied()
                .filter(|&id| {
                    if let Some(&idx) = state.node_keys.get(id) {
                        viewport.is_visible(*state.positions.get(idx), *state.sizes.get(idx))
                    } else {
                        false
                    }
                })
                .collect()
        } else {
            self.spatial_grid.query(model_pos)
        };

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
                    let depth = get_nesting_depth(id, state);
                    if best_match.is_none() || depth > max_depth {
                        best_match = Some(id);
                        max_depth = depth;
                    }
                }
            }
        }

        best_match
    }

    pub fn hit_test_edge<S: Copy>(
        &self,
        screen_pos: gpui::Point<f32>,
        viewport: &Viewport,
        state: &GraphState<S>,
        threshold: f32,
    ) -> Option<usize> {
        for edge_idx in 0..state.edges.len() {
            let src = *state.edge_sources.get(edge_idx);
            let tgt = *state.edge_targets.get(edge_idx);
            let (Some(&src_idx), Some(&tgt_idx)) =
                (state.node_keys.get(src), state.node_keys.get(tgt))
            else {
                continue;
            };
            let pos_src = *state.positions.get(src_idx);
            let pos_tgt = *state.positions.get(tgt_idx);

            let src_screen = viewport.model_to_screen(pos_src);
            let tgt_screen = viewport.model_to_screen(pos_tgt);

            let dist = distance_to_segment(
                screen_pos,
                gpui::point(src_screen.x, src_screen.y),
                gpui::point(tgt_screen.x, tgt_screen.y),
            );
            if dist < threshold {
                return Some(edge_idx);
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
            let total_mouse_delta = gpui::point(
                position.x - start_mouse_pos.x,
                position.y - start_mouse_pos.y,
            );
            let target_parent_pos = start_node_pos + Vec2::new(total_mouse_delta.x / viewport.zoom, total_mouse_delta.y / viewport.zoom);
            if let Some(&idx) = state.node_keys.get(id) {
                let current_parent_pos = *state.positions.get(idx);
                let step_delta = target_parent_pos - current_parent_pos;
                state.translate_node_and_descendants(id, step_delta);
            }
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
    use graphene_core::math::Size2;

    #[test]
    fn test_hit_test_prioritizes_nested_children() {
        let mut state = GraphState::new();

        // 1. Create a parent node (cover -100 to 100 on both axes)
        let parent_id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(200.0, 200.0));

        // 2. Create a child node (nested inside, cover 30 to 70 on both axes)
        let child_id = state.add_node(Vec2::new(50.0, 50.0), Size2::new(40.0, 40.0));
        
        // Reparent child to parent
        state.reparent_node(child_id, Some(parent_id));

        // Create viewport (centered at 0, 0 in model space)
        let bounds = gpui::Bounds {
            origin: gpui::Point { x: 0.0, y: 0.0 },
            size: gpui::Size { width: 800.0, height: 600.0 },
        };
        let viewport = Viewport::new(bounds);

        // Rebuild spatial grid
        let mut interaction = InteractionState::new(60.0);
        interaction.rebuild_grid(&state);

        // Click directly on the child node at model coordinate (50, 50)
        // Screen position corresponding to (50, 50)
        let screen_pos = viewport.model_to_screen(Vec2::new(50.0, 50.0));

        // Hit test with active simulation (linear scan)
        let hit_active = interaction.hit_test(screen_pos, &viewport, &state, true);
        assert_eq!(hit_active, Some(child_id), "Active hit_test did not prioritize nested child node!");

        // Hit test with inactive simulation (spatial grid query)
        let hit_inactive = interaction.hit_test(screen_pos, &viewport, &state, false);
        assert_eq!(hit_inactive, Some(child_id), "Inactive hit_test did not prioritize nested child node!");

        // Click on the parent margin (e.g. at model coordinate (-50, -50), outside child)
        let screen_margin = viewport.model_to_screen(Vec2::new(-50.0, -50.0));
        
        let hit_margin_active = interaction.hit_test(screen_margin, &viewport, &state, true);
        assert_eq!(hit_margin_active, Some(parent_id), "Hit test at margin should match parent node!");

        let hit_margin_inactive = interaction.hit_test(screen_margin, &viewport, &state, false);
        assert_eq!(hit_margin_inactive, Some(parent_id), "Hit test at margin should match parent node!");
    }

    #[test]
    fn test_drag_compound_node_translates_children() {
        let mut state = GraphState::new();

        let parent_id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(200.0, 200.0));
        let child_id = state.add_node(Vec2::new(50.0, 50.0), Size2::new(40.0, 40.0));
        state.reparent_node(child_id, Some(parent_id));

        let bounds = gpui::Bounds {
            origin: gpui::Point { x: 0.0, y: 0.0 },
            size: gpui::Size { width: 800.0, height: 600.0 },
        };
        let mut viewport = Viewport::new(bounds);

        let mut interaction = InteractionState::new(60.0);
        let start_screen_pos = viewport.model_to_screen(Vec2::new(0.0, 0.0));

        // Mouse down on parent node
        interaction.on_mouse_down(start_screen_pos, Some(parent_id), &state);

        // Drag parent node by +100 in x, +50 in y
        let target_screen_pos = gpui::point(start_screen_pos.x + 100.0, start_screen_pos.y + 50.0);
        interaction.on_mouse_drag(target_screen_pos, &mut viewport, &mut state);

        let p_idx = state.node_keys[parent_id];
        let c_idx = state.node_keys[child_id];

        let parent_pos = *state.positions.get(p_idx);
        let child_pos = *state.positions.get(c_idx);

        assert_eq!(parent_pos, Vec2::new(100.0, 50.0));
        assert_eq!(child_pos, Vec2::new(150.0, 100.0));
    }
}
