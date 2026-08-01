use crate::collision::resolve_overlaps;
use crate::traits::Layout;
use graphene_core::{math::Vec2, AnimationTrack, GraphState, NodeId};
use std::time::Duration;

const LCG_MULTIPLIER: u64 = 6364136223846793005;
const LCG_INCREMENT: u64 = 1442695040888963407;

/// Random graph layout.
///
/// Reference: Uniform random coordinate distribution.
pub struct RandomLayout {
    pub width: f32,
    pub height: f32,
    pub animate: bool,
}

impl Default for RandomLayout {
    fn default() -> Self {
        Self {
            width: 800.0,
            height: 600.0,
            animate: false,
        }
    }
}

impl RandomLayout {
    pub fn with_width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    pub fn with_animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }
}

impl<S: Copy + Default> Layout<S> for RandomLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let mut state_lcg = 12345u64;
        let mut next_float = || {
            state_lcg = state_lcg.wrapping_mul(LCG_MULTIPLIER).wrapping_add(LCG_INCREMENT);
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
        resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

/// Grid graph layout.
///
/// Reference: Regular 2D grid matrix placement.
pub struct GridLayout {
    pub columns: usize,
    pub spacing_x: f32,
    pub spacing_y: f32,
    pub animate: bool,
}

impl Default for GridLayout {
    fn default() -> Self {
        Self {
            columns: 5,
            spacing_x: 120.0,
            spacing_y: 100.0,
            animate: false,
        }
    }
}

impl GridLayout {
    pub fn with_columns(mut self, columns: usize) -> Self {
        self.columns = columns;
        self
    }

    pub fn with_spacing_x(mut self, spacing_x: f32) -> Self {
        self.spacing_x = spacing_x;
        self
    }

    pub fn with_spacing_y(mut self, spacing_y: f32) -> Self {
        self.spacing_y = spacing_y;
        self
    }

    pub fn with_animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }
}

impl<S: Copy + Default> Layout<S> for GridLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }
        let cols = self.columns.max(1);

        let mut max_w = 0.0f32;
        let mut max_h = 0.0f32;
        for idx in 0..num_nodes {
            let size = *state.sizes.get(idx);
            max_w = max_w.max(size.w);
            max_h = max_h.max(size.h);
        }
        let col_step = self.spacing_x.max(max_w + 10.0);
        let row_step = self.spacing_y.max(max_h + 10.0);

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let r = idx / cols;
            let c = idx % cols;
            let target = Vec2::new(c as f32 * col_step, r as f32 * row_step);

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
        let collapsed = std::collections::HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}

/// Circular graph layout.
///
/// Reference: Circular layout placement algorithm.
pub struct CircleLayout {
    pub radius: f32,
    pub center: Vec2,
    pub animate: bool,
}

impl Default for CircleLayout {
    fn default() -> Self {
        Self {
            radius: 150.0,
            center: Vec2::default(),
            animate: false,
        }
    }
}

impl CircleLayout {
    pub fn with_radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    pub fn with_center(mut self, center: Vec2) -> Self {
        self.center = center;
        self
    }

    pub fn with_animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }
}

impl<S: Copy + Default> Layout<S> for CircleLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let num_nodes = state.node_index_to_id.len();
        if num_nodes == 0 {
            return;
        }

        let mut max_extent = 0.0f32;
        for idx in 0..num_nodes {
            let size = *state.sizes.get(idx);
            max_extent = max_extent.max(size.w.max(size.h));
        }
        let required_circumference = num_nodes as f32 * (max_extent + 10.0);
        let effective_radius = self.radius.max(required_circumference / (2.0 * std::f32::consts::PI));

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let angle = (idx as f32 / num_nodes as f32) * 2.0 * std::f32::consts::PI;
            let target = Vec2::new(
                self.center.x + effective_radius * angle.cos(),
                self.center.y + effective_radius * angle.sin(),
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
        let collapsed = std::collections::HashSet::new();
        crate::collision::finish_layout_epilogue(state, &collapsed, 10.0, 20.0);
    }
}

/// Concentric ring graph layout.
///
/// Reference: Concentric ring placement based on topological levels.
pub struct ConcentricLayout {
    pub level_radius_step: f32,
    pub center: Vec2,
    pub animate: bool,
}

impl Default for ConcentricLayout {
    fn default() -> Self {
        Self {
            level_radius_step: 150.0,
            center: Vec2::default(),
            animate: false,
        }
    }
}

impl ConcentricLayout {
    pub fn with_level_radius_step(mut self, step: f32) -> Self {
        self.level_radius_step = step;
        self
    }

    pub fn with_center(mut self, center: Vec2) -> Self {
        self.center = center;
        self
    }

    pub fn with_animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }
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
        resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

/// Breadth-first tree graph layout.
///
/// Reference: BFS tree-level placement algorithm.
pub struct BreadthFirstLayout {
    pub root: NodeId,
    pub sibling_spacing: f32,
    pub level_spacing: f32,
    pub animate: bool,
}

impl Default for BreadthFirstLayout {
    fn default() -> Self {
        Self {
            root: graphene_core::NodeId::default(),
            sibling_spacing: 100.0,
            level_spacing: 120.0,
            animate: false,
        }
    }
}

impl BreadthFirstLayout {
    pub fn with_root(mut self, root: NodeId) -> Self {
        self.root = root;
        self
    }

    pub fn with_sibling_spacing(mut self, spacing: f32) -> Self {
        self.sibling_spacing = spacing;
        self
    }

    pub fn with_level_spacing(mut self, spacing: f32) -> Self {
        self.level_spacing = spacing;
        self
    }

    pub fn with_animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }
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
            if count == 0 {
                continue;
            }

            let mut x_offsets = vec![0.0f32; count];
            let mut current_x = 0.0f32;
            for i in 0..count {
                let id = level_nodes[i];
                let idx = *state.node_keys.get(id).unwrap();
                let w = state.sizes.get(idx).w;
                if i == 0 {
                    x_offsets[i] = w / 2.0;
                    current_x = w;
                } else {
                    let prev_id = level_nodes[i - 1];
                    let prev_idx = *state.node_keys.get(prev_id).unwrap();
                    let prev_w = state.sizes.get(prev_idx).w;
                    let gap = self.sibling_spacing.max(10.0);
                    current_x += prev_w / 2.0 + gap + w / 2.0;
                    x_offsets[i] = current_x;
                }
            }

            let total_width = current_x;
            let start_x = -total_width / 2.0;

            for (i, &id) in level_nodes.iter().enumerate() {
                if let Some(&idx) = state.node_keys.get(id) {
                    let target = Vec2::new(
                        start_x + x_offsets[i],
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
        resolve_overlaps(state, 10.0);
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}
