use crate::traits::Layout;
use graphene_core::{math::Vec2, AnimationTrack, GraphState, NodeId};
use std::time::Duration;

pub struct RandomLayout {
    pub width: f32,
    pub height: f32,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for RandomLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let mut state_lcg = 12345u64;
        let mut next_float = || {
            state_lcg = state_lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            (state_lcg >> 32) as f32 / u32::MAX as f32
        };

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let target = Vec2::new(next_float() * self.width, next_float() * self.height);
            if self.animate {
                let from = *state.positions.get(idx);
                state.animations.tracks.insert(
                    id,
                    AnimationTrack::Position {
                        from,
                        to: target,
                        duration: Duration::from_millis(500),
                        elapsed: Duration::ZERO,
                    },
                );
            } else {
                state.positions.set(idx, target);
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct GridLayout {
    pub columns: usize,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for GridLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }
        let cols = self.columns.max(1);

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let r = idx / cols;
            let c = idx % cols;
            let target = Vec2::new(c as f32 * self.spacing_x, r as f32 * self.spacing_y);

            if self.animate {
                let from = *state.positions.get(idx);
                state.animations.tracks.insert(
                    id,
                    AnimationTrack::Position {
                        from,
                        to: target,
                        duration: Duration::from_millis(500),
                        elapsed: Duration::ZERO,
                    },
                );
            } else {
                state.positions.set(idx, target);
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct CircleLayout {
    pub radius: f32,
    pub center: Vec2,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for CircleLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let angle = (idx as f32 / num_nodes as f32) * 2.0 * std::f32::consts::PI;
            let target = Vec2::new(
                self.center.x + self.radius * angle.cos(),
                self.center.y + self.radius * angle.sin(),
            );

            if self.animate {
                let from = *state.positions.get(idx);
                state.animations.tracks.insert(
                    id,
                    AnimationTrack::Position {
                        from,
                        to: target,
                        duration: Duration::from_millis(500),
                        elapsed: Duration::ZERO,
                    },
                );
            } else {
                state.positions.set(idx, target);
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct ConcentricLayout {
    pub level_radius_step: f32,
    pub center: Vec2,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for ConcentricLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }

        let mut _level = 0;
        let mut max_in_level = 5;
        let mut level_count = 0;
        let mut level_radius = self.level_radius_step;

        let mut level_nodes = Vec::new();
        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            level_nodes.push((idx, id));
            level_count += 1;
            if level_count >= max_in_level || idx == num_nodes - 1 {
                let count = level_nodes.len();
                for (j, &(n_idx, n_id)) in level_nodes.iter().enumerate() {
                    let angle = (j as f32 / count as f32) * 2.0 * std::f32::consts::PI;
                    let target = Vec2::new(
                        self.center.x + level_radius * angle.cos(),
                        self.center.y + level_radius * angle.sin(),
                    );
                    if self.animate {
                        let from = *state.positions.get(n_idx);
                        state.animations.tracks.insert(
                            n_id,
                            AnimationTrack::Position {
                                from,
                                to: target,
                                duration: Duration::from_millis(500),
                                elapsed: Duration::ZERO,
                            },
                        );
                    } else {
                        state.positions.set(n_idx, target);
                    }
                }
                level_nodes.clear();
                level_count = 0;
                _level += 1;
                max_in_level *= 2;
                level_radius += self.level_radius_step;
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

pub struct BreadthFirstLayout {
    pub root: NodeId,
    pub sibling_spacing: f32,
    pub level_spacing: f32,
    pub animate: bool,
}

impl<S: Copy + Default> Layout<S> for BreadthFirstLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        if !state.node_keys.contains_key(self.root) {
            return;
        }

        let mut levels = std::collections::HashMap::new();
        let mut queue = std::collections::VecDeque::new();
        let mut visited = std::collections::HashSet::new();

        queue.push_back((self.root, 0));
        visited.insert(self.root);

        let mut adj: std::collections::HashMap<NodeId, Vec<NodeId>> = std::collections::HashMap::new();
        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            adj.entry(src).or_default().push(tgt);
        }

        while let Some((curr, lvl)) = queue.pop_front() {
            levels.entry(lvl).or_insert_with(Vec::new).push(curr);
            if let Some(children) = adj.get(&curr) {
                for &child in children {
                    if visited.insert(child) {
                        queue.push_back((child, lvl + 1));
                    }
                }
            }
        }

        for (&lvl, level_nodes) in &levels {
            let count = level_nodes.len();
            let total_width = (count - 1) as f32 * self.sibling_spacing;
            let start_x = -total_width / 2.0;

            for (i, &id) in level_nodes.iter().enumerate() {
                if let Some(&idx) = state.node_keys.get(id) {
                    let target = Vec2::new(
                        start_x + i as f32 * self.sibling_spacing,
                        lvl as f32 * self.level_spacing,
                    );
                    if self.animate {
                        let from = *state.positions.get(idx);
                        state.animations.tracks.insert(
                            id,
                            AnimationTrack::Position {
                                from,
                                to: target,
                                duration: Duration::from_millis(500),
                                elapsed: Duration::ZERO,
                            },
                        );
                    } else {
                        state.positions.set(idx, target);
                    }
                }
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
