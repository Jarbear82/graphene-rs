use graphene_core::math::Vec2;

pub struct Quadtree {
    pub center_of_mass: Vec2,
    pub total_mass: f32,
    pub bounds_min: Vec2,
    pub bounds_max: Vec2,
    pub children: Option<Box<[Quadtree; 4]>>,
    pub node_indices: Vec<usize>,
}

impl Quadtree {
    pub fn new(bounds_min: Vec2, bounds_max: Vec2) -> Self {
        Self {
            center_of_mass: Vec2::default(),
            total_mass: 0.0,
            bounds_min,
            bounds_max,
            children: None,
            node_indices: Vec::new(),
        }
    }

    pub fn build(positions: &[Vec2]) -> Self {
        let n = positions.len();
        if n == 0 {
            return Self::new(Vec2::default(), Vec2::default());
        }

        let mut min_x = f32::INFINITY;
        let mut max_x = -f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_y = -f32::INFINITY;

        for &pos in positions {
            min_x = min_x.min(pos.x);
            max_x = max_x.max(pos.x);
            min_y = min_y.min(pos.y);
            max_y = max_y.max(pos.y);
        }

        let size_x = max_x - min_x;
        let size_y = max_y - min_y;
        let max_size = size_x.max(size_y).max(1.0);
        let center = Vec2::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5);

        let bounds_min = Vec2::new(center.x - max_size * 0.5 - 1.0, center.y - max_size * 0.5 - 1.0);
        let bounds_max = Vec2::new(center.x + max_size * 0.5 + 1.0, center.y + max_size * 0.5 + 1.0);

        let mut root = Self::new(bounds_min, bounds_max);
        for i in 0..n {
            root.insert(i, positions, 0);
        }
        root
    }

    pub fn insert(&mut self, idx: usize, positions: &[Vec2], depth: usize) {
        let pos = positions[idx];
        self.center_of_mass = (self.center_of_mass * self.total_mass + pos) / (self.total_mass + 1.0);
        self.total_mass += 1.0;

        if self.children.is_none() && self.node_indices.is_empty() {
            self.node_indices.push(idx);
            return;
        }

        if self.children.is_none() {
            if depth >= 15 {
                self.node_indices.push(idx);
                return;
            }

            let mid = (self.bounds_min + self.bounds_max) * 0.5;
            let sub_nodes = [
                Self::new(self.bounds_min, mid),
                Self::new(Vec2::new(mid.x, self.bounds_min.y), Vec2::new(self.bounds_max.x, mid.y)),
                Self::new(Vec2::new(self.bounds_min.x, mid.y), Vec2::new(mid.x, self.bounds_max.y)),
                Self::new(mid, self.bounds_max),
            ];

            let old_indices = std::mem::take(&mut self.node_indices);
            self.children = Some(Box::new(sub_nodes));

            let children_ref = self.children.as_mut().unwrap();
            for old_idx in old_indices {
                let old_pos = positions[old_idx];
                let c_idx = if old_pos.y < mid.y {
                    if old_pos.x < mid.x { 0 } else { 1 }
                } else {
                    if old_pos.x < mid.x { 2 } else { 3 }
                };
                children_ref[c_idx].insert(old_idx, positions, depth + 1);
            }
        }

        if let Some(ref mut children) = self.children {
            let mid = (self.bounds_min + self.bounds_max) * 0.5;
            let child_idx = if pos.y < mid.y {
                if pos.x < mid.x { 0 } else { 1 }
            } else {
                if pos.x < mid.x { 2 } else { 3 }
            };
            children[child_idx].insert(idx, positions, depth + 1);
        }
    }

    pub fn accumulate_repulsion(&self, i: usize, pos_i: Vec2, positions: &[Vec2], k_rep: f32, theta: f32) -> Vec2 {
        if self.total_mass == 0.0 {
            return Vec2::default();
        }

        let delta = pos_i - self.center_of_mass;
        let dist = delta.len();

        if let Some(ref children) = self.children {
            let s = (self.bounds_max.x - self.bounds_min.x).max(self.bounds_max.y - self.bounds_min.y);
            if dist > 0.1 && (s / dist) < theta {
                let force_magnitude = (k_rep * self.total_mass) / (dist * dist);
                let dir = delta.normalize();
                return dir * force_magnitude;
            }

            let mut force = Vec2::default();
            for child in children.iter() {
                force += child.accumulate_repulsion(i, pos_i, positions, k_rep, theta);
            }
            force
        } else {
            let mut force = Vec2::default();
            for &j in &self.node_indices {
                if i == j {
                    continue;
                }
                let pos_j = positions[j];
                let d_delta = pos_i - pos_j;
                let d_dist = d_delta.len();
                if d_dist > 0.1 {
                    let force_magnitude = k_rep / (d_dist * d_dist);
                    let dir = d_delta.normalize();
                    force += dir * force_magnitude;
                } else {
                    let force_magnitude = k_rep / 0.01;
                    let dir = Vec2::new(1.0, 0.0);
                    force += dir * force_magnitude;
                }
            }
            force
        }
    }

    /// Query all node indices whose bounding boxes might overlap with node `i`'s bounding box expanded by `padding`.
    pub fn query_overlapping_candidates(
        &self,
        pos_i: Vec2,
        half_w_i: f32,
        half_h_i: f32,
        padding: f32,
        out: &mut Vec<usize>,
    ) {
        if self.total_mass == 0.0 {
            return;
        }

        let query_min_x = pos_i.x - half_w_i - padding;
        let query_max_x = pos_i.x + half_w_i + padding;
        let query_min_y = pos_i.y - half_h_i - padding;
        let query_max_y = pos_i.y + half_h_i + padding;

        let intersects = query_max_x >= self.bounds_min.x
            && query_min_x <= self.bounds_max.x
            && query_max_y >= self.bounds_min.y
            && query_min_y <= self.bounds_max.y;

        if !intersects {
            return;
        }

        if let Some(ref children) = self.children {
            for child in children.iter() {
                child.query_overlapping_candidates(pos_i, half_w_i, half_h_i, padding, out);
            }
        } else {
            out.extend_from_slice(&self.node_indices);
        }
    }
}
