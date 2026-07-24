use crate::app::DemoApp;
use graphene_core::Vec2;

impl DemoApp {
    pub fn run_physics_step(&mut self) {
        let n = self.state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let mut forces = vec![Vec2::default(); n];

        let k_rep = 2500.0;
        let k_att = 0.06;
        let gravity = 0.3;

        let is_parent = |idx: usize,
                         state: &graphene_core::GraphState<graphene_style::ComputedStyle>|
         -> bool { state.hierarchy.first_child.get(idx).is_some() };

        let get_leaf_descendants =
            |node_idx: usize,
             state: &graphene_core::GraphState<graphene_style::ComputedStyle>|
             -> Vec<usize> {
                let mut leaves = Vec::new();
                let mut stack = vec![node_idx];
                while let Some(curr) = stack.pop() {
                    if !is_parent(curr, state) {
                        leaves.push(curr);
                    } else {
                        let mut next_child = *state.hierarchy.first_child.get(curr);
                        while let Some(child_id) = next_child {
                            if let Some(&child_idx) = state.node_keys.get(child_id) {
                                stack.push(child_idx);
                                next_child = *state.hierarchy.next_sibling.get(child_idx);
                            } else {
                                break;
                            }
                        }
                    }
                }
                leaves
            };

        let is_ancestor = |mut child_idx: usize,
                           parent_idx: usize,
                           state: &graphene_core::GraphState<graphene_style::ComputedStyle>|
         -> bool {
            let parent_id = state.node_index_to_id[parent_idx];
            while let Some(p_id) = *state.hierarchy.parent.get(child_idx) {
                if p_id == parent_id {
                    return true;
                }
                if let Some(&p_idx) = state.node_keys.get(p_id) {
                    child_idx = p_idx;
                } else {
                    break;
                }
            }
            false
        };

        for i in 0..n {
            for j in (i + 1)..n {
                if is_ancestor(i, j, &self.state) || is_ancestor(j, i, &self.state) {
                    continue;
                }

                let pos_i = *self.state.positions.get(i);
                let pos_j = *self.state.positions.get(j);
                let size_i = *self.state.sizes.get(i);
                let size_j = *self.state.sizes.get(j);

                let dx = pos_j.x - pos_i.x;
                let dy = pos_j.y - pos_i.y;
                let dist = (dx * dx + dy * dy + 0.01).sqrt();

                let p1 = graphene_layout::find_clipping_point(pos_i, size_i, dx, dy);
                let p2 = graphene_layout::find_clipping_point(pos_j, size_j, -dx, -dy);
                let border_dx = p2.x - p1.x;
                let border_dy = p2.y - p1.y;
                let border_dist = (border_dx * border_dx + border_dy * border_dy)
                    .sqrt()
                    .max(1.0);

                let force = k_rep / (border_dist * border_dist);
                let fx = -force * dx / dist;
                let fy = -force * dy / dist;

                if !is_parent(i, &self.state) {
                    forces[i].x += fx;
                    forces[i].y += fy;
                } else {
                    let leaves = get_leaf_descendants(i, &self.state);
                    if !leaves.is_empty() {
                        let f_each_x = fx / leaves.len() as f32;
                        let f_each_y = fy / leaves.len() as f32;
                        for &leaf_idx in &leaves {
                            forces[leaf_idx].x += f_each_x;
                            forces[leaf_idx].y += f_each_y;
                        }
                    }
                }

                if !is_parent(j, &self.state) {
                    forces[j].x -= fx;
                    forces[j].y -= fy;
                } else {
                    let leaves = get_leaf_descendants(j, &self.state);
                    if !leaves.is_empty() {
                        let f_each_x = -fx / leaves.len() as f32;
                        let f_each_y = -fy / leaves.len() as f32;
                        for &leaf_idx in &leaves {
                            forces[leaf_idx].x += f_each_x;
                            forces[leaf_idx].y += f_each_y;
                        }
                    }
                }
            }
        }

        let edges_count = self.state.edges.len();
        for i in 0..edges_count {
            let src = *self.state.edge_sources.get(i);
            let tgt = *self.state.edge_targets.get(i);
            let (Some(&src_idx), Some(&tgt_idx)) =
                (self.state.node_keys.get(src), self.state.node_keys.get(tgt))
            else {
                continue;
            };

            let pos_src = *self.state.positions.get(src_idx);
            let pos_tgt = *self.state.positions.get(tgt_idx);

            let dx = pos_tgt.x - pos_src.x;
            let dy = pos_tgt.y - pos_src.y;
            let dist = (dx * dx + dy * dy + 0.01).sqrt();

            let force = k_att * dist;
            let fx = (dx / dist) * force;
            let fy = (dy / dist) * force;

            forces[src_idx].x += fx;
            forces[src_idx].y += fy;

            forces[tgt_idx].x -= fx;
            forces[tgt_idx].y -= fy;
        }

        let temp = self.physics_temperature;
        for i in 0..n {
            let id = self.state.node_index_to_id[i];
            let is_dragging = match self.interaction_state.drag_start {
                Some((drag_id, _, _)) => drag_id == id,
                None => false,
            };
            if is_dragging {
                continue;
            }

            let pos = self.state.positions.get_mut(i);

            forces[i].x -= pos.x * gravity;
            forces[i].y -= pos.y * gravity;

            let force_len = (forces[i].x * forces[i].x + forces[i].y * forces[i].y + 0.01).sqrt();
            let limit = force_len.min(temp);

            pos.x += (forces[i].x / force_len) * limit;
            pos.y += (forces[i].y / force_len) * limit;
        }
    }

    pub fn resolve_collisions(&mut self) {
        let n = self.state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let is_ancestor = |mut child_idx: usize,
                           parent_idx: usize,
                           state: &graphene_core::GraphState<graphene_style::ComputedStyle>|
         -> bool {
            let parent_id = state.node_index_to_id[parent_idx];
            while let Some(p_id) = *state.hierarchy.parent.get(child_idx) {
                if p_id == parent_id {
                    return true;
                }
                if let Some(&p_idx) = state.node_keys.get(p_id) {
                    child_idx = p_idx;
                } else {
                    break;
                }
            }
            false
        };

        let padding = 12.0;

        for _ in 0..4 {
            for i in 0..n {
                for j in (i + 1)..n {
                    if is_ancestor(i, j, &self.state) || is_ancestor(j, i, &self.state) {
                        continue;
                    }
                    let pos_i = *self.state.positions.get(i);
                    let pos_j = *self.state.positions.get(j);
                    let size_i = *self.state.sizes.get(i);
                    let size_j = *self.state.sizes.get(j);

                    let dx = pos_j.x - pos_i.x;
                    let dy = pos_j.y - pos_i.y;

                    let min_dx = (size_i.w + size_j.w) / 2.0 + padding;
                    let min_dy = (size_i.h + size_j.h) / 2.0 + padding;

                    let overlap_x = min_dx - dx.abs();
                    let overlap_y = min_dy - dy.abs();

                    if overlap_x > 0.0 && overlap_y > 0.0 {
                        let push_x;
                        let push_y;

                        if overlap_x < overlap_y {
                            let sign_x = if dx >= 0.0 { 1.0 } else { -1.0 };
                            push_x = sign_x * overlap_x * 0.5;
                            push_y = 0.0;
                        } else {
                            let sign_y = if dy >= 0.0 { 1.0 } else { -1.0 };
                            push_x = 0.0;
                            push_y = sign_y * overlap_y * 0.5;
                        }

                        let id_i = self.state.node_index_to_id[i];
                        let id_j = self.state.node_index_to_id[j];

                        let is_dragging_i = match self.interaction_state.drag_start {
                            Some((drag_id, _, _)) => drag_id == id_i,
                            None => false,
                        };
                        let is_dragging_j = match self.interaction_state.drag_start {
                            Some((drag_id, _, _)) => drag_id == id_j,
                            None => false,
                        };

                        if is_dragging_i && !is_dragging_j {
                            let p_j = self.state.positions.get_mut(j);
                            p_j.x += push_x * 2.0;
                            p_j.y += push_y * 2.0;
                        } else if is_dragging_j && !is_dragging_i {
                            let p_i = self.state.positions.get_mut(i);
                            p_i.x -= push_x * 2.0;
                            p_i.y -= push_y * 2.0;
                        } else if !is_dragging_i && !is_dragging_j {
                            let p_i = self.state.positions.get_mut(i);
                            p_i.x -= push_x;
                            p_i.y -= push_y;

                            let p_j = self.state.positions.get_mut(j);
                            p_j.x += push_x;
                            p_j.y += push_y;
                        }
                    }
                }
            }
        }
    }
}
