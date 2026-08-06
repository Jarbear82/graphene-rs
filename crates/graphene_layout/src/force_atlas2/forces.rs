use super::types::*;
use std::f64::consts::PI;

/// Barnes-Hut region (quadtree node)
pub struct Region {
    pub mass: f64,
    pub mass_center: Vec2,
    pub size: f64,
    pub nodes: Vec<usize>, // indices into the global nodes vec
    pub subregions: Vec<Region>,
}

impl Region {
    pub fn new(node_indices: Vec<usize>, nodes: &[Node]) -> Self {
        let mut r = Region {
            mass: 0.0,
            mass_center: Vec2::zero(),
            size: 0.0,
            nodes: node_indices,
            subregions: Vec::new(),
        };
        r.update_mass_and_geometry(nodes);
        r
    }

    pub fn update_mass_and_geometry(&mut self, nodes: &[Node]) {
        if self.nodes.is_empty() {
            return;
        }
        if self.nodes.len() == 1 {
            let n = &nodes[self.nodes[0]];
            self.mass = n.mass;
            self.mass_center = n.pos;
            self.size = 0.0;
            return;
        }

        let mut mass = 0.0;
        let mut sum = Vec2::zero();
        for &i in &self.nodes {
            let n = &nodes[i];
            mass += n.mass;
            sum = sum.add(n.pos.scale(n.mass));
        }
        self.mass = mass;
        if mass > 0.0 {
            self.mass_center = sum.scale(1.0 / mass);
        }

        let mut size: f64 = 0.0;
        for &i in &self.nodes {
            let d = nodes[i].pos.sub(self.mass_center).length();
            size = size.max(2.0 * d);
        }
        self.size = size;
    }

    pub fn build_subregions(&mut self, nodes: &[Node]) {
        if self.nodes.len() <= 1 {
            return;
        }

        let mut buckets: [Vec<usize>; 4] = Default::default();
        for &i in &self.nodes {
            let p = nodes[i].pos;
            let mut b = 0usize;
            if p.x >= self.mass_center.x {
                b |= 1;
            }
            if p.y >= self.mass_center.y {
                b |= 2;
            }
            buckets[b].push(i);
        }

        for bucket in buckets {
            if bucket.is_empty() {
                continue;
            }
            if bucket.len() < self.nodes.len() {
                let mut sub = Region::new(bucket, nodes);
                sub.build_subregions(nodes);
                self.subregions.push(sub);
            } else {
                for i in bucket {
                    let sub = Region::new(vec![i], nodes);
                    self.subregions.push(sub);
                }
            }
        }
    }

    pub fn apply_force(
        &self,
        n_idx: usize,
        nodes: &mut [Node],
        theta: f64,
        coefficient: f64,
        adjust_sizes: bool,
    ) {
        if self.nodes.is_empty() {
            return;
        }

        if self.nodes.len() == 1 {
            let other = self.nodes[0];
            if other != n_idx {
                lin_repulsion(n_idx, other, nodes, coefficient, adjust_sizes);
            }
            return;
        }

        let n = &nodes[n_idx];
        let dist = n.pos.sub(self.mass_center).length();
        if dist * theta > self.size {
            lin_repulsion_region(n_idx, self, nodes, coefficient, adjust_sizes);
        } else {
            for sub in &self.subregions {
                sub.apply_force(n_idx, nodes, theta, coefficient, adjust_sizes);
            }
        }
    }
}

// ── Force functions ──────────────────────────────────────────────────────────

pub fn effective_directional_radius(pos_diff: Vec2, euclidean: f64, node: &Node) -> f64 {
    if node.size_wh.x > 0.0 && node.size_wh.y > 0.0 {
        if euclidean < 1e-6 {
            return (node.size_wh.x.max(node.size_wh.y)) / 2.0;
        }
        let inv_w = pos_diff.x.abs() / node.size_wh.x;
        let inv_h = pos_diff.y.abs() / node.size_wh.y;
        let max_inv = inv_w.max(inv_h);
        if max_inv > 1e-6 {
            0.5 * euclidean / max_inv
        } else {
            (node.size_wh.x.max(node.size_wh.y)) / 2.0
        }
    } else {
        node.size
    }
}

pub fn lin_repulsion(i: usize, j: usize, nodes: &mut [Node], coefficient: f64, adjust_sizes: bool) {
    let (n1, n2) = if i < j {
        let (a, b) = nodes.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = nodes.split_at_mut(i);
        (&mut b[0], &mut a[j])
    };

    let mut dist_vec = n1.pos.sub(n2.pos);
    let mut euclidean = dist_vec.length();

    if euclidean < 1e-4 {
        let angle = ((i * 37 + j * 101) % 360) as f64 * PI / 180.0;
        dist_vec = Vec2::new(angle.cos() * 1e-2, angle.sin() * 1e-2);
        euclidean = 1e-2;
    }

    if adjust_sizes {
        let r1 = effective_directional_radius(dist_vec, euclidean, n1);
        let r2 = effective_directional_radius(dist_vec.scale(-1.0), euclidean, n2);
        let distance = euclidean - r1 - r2;
        if distance <= 0.0 {
            let padding = 0.5 * (r1 + r2);
            let target_dist = r1 + r2 + padding;
            let overlap = target_dist - euclidean;
            if euclidean > 1e-6 {
                let shift = dist_vec.scale(0.5 * overlap / euclidean);
                n1.pos = n1.pos.add(shift);
                n2.pos = n2.pos.sub(shift);
            }
            return;
        }

        let factor = coefficient * n1.mass * n2.mass / (distance * distance);
        let force = dist_vec.scale(factor);
        n1.force = n1.force.add(force);
        n2.force = n2.force.sub(force);
    } else {
        let factor = coefficient * n1.mass * n2.mass / (euclidean * euclidean);
        let force = dist_vec.scale(factor);
        n1.force = n1.force.add(force);
        n2.force = n2.force.sub(force);
    }
}

pub fn lin_repulsion_region(
    i: usize,
    region: &Region,
    nodes: &mut [Node],
    coefficient: f64,
    adjust_sizes: bool,
) {
    let n = &mut nodes[i];
    let mut dist_vec = n.pos.sub(region.mass_center);
    let mut euclidean = dist_vec.length();

    if euclidean < 1e-4 {
        let angle = (i * 37 % 360) as f64 * PI / 180.0;
        dist_vec = Vec2::new(angle.cos() * 1e-2, angle.sin() * 1e-2);
        euclidean = 1e-2;
    }

    if adjust_sizes {
        let r = effective_directional_radius(dist_vec, euclidean, n);
        let distance = euclidean - r;
        if distance <= 0.0 {
            let padding = 0.5 * r;
            let target_dist = r + padding;
            let overlap = target_dist - euclidean;
            if euclidean > 1e-6 {
                let shift = dist_vec.scale(overlap / euclidean);
                n.pos = n.pos.add(shift);
            }
            return;
        }

        let factor = coefficient * n.mass * region.mass / (distance * distance);
        n.force = n.force.add(dist_vec.scale(factor));
    } else {
        let factor = coefficient * n.mass * region.mass / (euclidean * euclidean);
        n.force = n.force.add(dist_vec.scale(factor));
    }
}

pub fn lin_gravity(n: &mut Node, g: f64) {
    let d = n.pos.length();
    if d > 1e-4 {
        let factor = n.mass * g / d;
        n.force = n.force.sub(n.pos.scale(factor));
    }
}

pub fn strong_gravity(n: &mut Node, g: f64) {
    if n.pos.x != 0.0 || n.pos.y != 0.0 {
        let factor = n.mass * g;
        n.force = n.force.sub(n.pos.scale(factor));
    }
}

pub fn lin_attraction(
    i: usize,
    j: usize,
    weight: f64,
    nodes: &mut [Node],
    distributed: bool,
    coefficient: f64,
    adjust_sizes: bool,
) {
    if i == j || i >= nodes.len() || j >= nodes.len() {
        return;
    }

    let (n1, n2) = if i < j {
        let (a, b) = nodes.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = nodes.split_at_mut(i);
        (&mut b[0], &mut a[j])
    };

    let dist_vec = n1.pos.sub(n2.pos);
    let euclidean = dist_vec.length();

    if euclidean < 1e-6 {
        return;
    }

    let (factor, eff_dist) = if adjust_sizes {
        let r1 = effective_directional_radius(dist_vec, euclidean, n1);
        let r2 = effective_directional_radius(dist_vec.scale(-1.0), euclidean, n2);
        let distance = euclidean - r1 - r2;
        if distance <= 0.0 {
            return;
        }
        let base_factor = if distributed {
            -coefficient * weight / n1.mass
        } else {
            -coefficient * weight
        };
        (base_factor, distance)
    } else {
        let base_factor = if distributed {
            -coefficient * weight / n1.mass
        } else {
            -coefficient * weight
        };
        (base_factor, euclidean)
    };

    let force = dist_vec.scale(factor * eff_dist / euclidean);
    n1.force = n1.force.add(force);
    n2.force = n2.force.sub(force);
}

pub fn log_attraction(
    i: usize,
    j: usize,
    weight: f64,
    nodes: &mut [Node],
    distributed: bool,
    coefficient: f64,
    adjust_sizes: bool,
) {
    if i == j || i >= nodes.len() || j >= nodes.len() {
        return;
    }

    let (n1, n2) = if i < j {
        let (a, b) = nodes.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = nodes.split_at_mut(i);
        (&mut b[0], &mut a[j])
    };

    let dist_vec = n1.pos.sub(n2.pos);
    let euclidean = dist_vec.length();

    if euclidean < 1e-6 {
        return;
    }

    let (factor, eff_dist) = if adjust_sizes {
        let r1 = effective_directional_radius(dist_vec, euclidean, n1);
        let r2 = effective_directional_radius(dist_vec.scale(-1.0), euclidean, n2);
        let distance = euclidean - r1 - r2;
        if distance <= 0.0 {
            return;
        }
        let log_factor = (1.0 + distance).ln();
        let base_factor = if distributed {
            -coefficient * weight * log_factor / n1.mass
        } else {
            -coefficient * weight * log_factor
        };
        (base_factor, 1.0)
    } else {
        let log_factor = (1.0 + euclidean).ln() / euclidean;
        let base_factor = if distributed {
            -coefficient * weight * log_factor / n1.mass
        } else {
            -coefficient * weight * log_factor
        };
        (base_factor, euclidean)
    };

    let force = dist_vec.scale(factor * eff_dist / euclidean);
    n1.force = n1.force.add(force);
    n2.force = n2.force.sub(force);
}

pub fn adjust_speed_and_apply_forces(
    nodes: &mut [Node],
    speed: &mut f64,
    speed_efficiency: &mut f64,
    jitter_tolerance: f64,
    adjust_sizes: bool,
    _scaling_ratio: f64,
    slow_down: f64,
) {
    let mut total_swinging = 0.0;
    let mut total_effective_traction = 0.0;

    for n in nodes.iter() {
        let swinging = n.old_force.sub(n.force).length();
        total_swinging += n.mass * swinging;
        total_effective_traction += 0.5 * n.mass * n.old_force.add(n.force).length();
    }

    let n = nodes.len() as f64;
    let estimated_optimal_jt = 0.05 * n.sqrt();
    let min_jt = estimated_optimal_jt.sqrt();
    let max_jt: f64 = 10.0;

    let jt = if n > 0.0 && total_effective_traction > 0.0 {
        jitter_tolerance
            * max_jt
                .min(estimated_optimal_jt * total_effective_traction / (n * n))
                .max(min_jt)
    } else {
        jitter_tolerance * min_jt
    };

    let min_speed_efficiency = 0.05;
    let max_speed_efficiency = 1.0;

    if total_effective_traction > 0.0 && total_swinging / total_effective_traction > 2.0 {
        if *speed_efficiency > min_speed_efficiency {
            *speed_efficiency *= 0.5;
        }
    }

    let target_speed = if total_swinging == 0.0 {
        1000.0
    } else {
        (jt * *speed_efficiency * total_effective_traction / total_swinging).min(100.0)
    };

    if total_swinging > jt * total_effective_traction {
        if *speed_efficiency > min_speed_efficiency {
            *speed_efficiency *= 0.7;
        }
    } else if *speed < 1000.0 {
        *speed_efficiency = (*speed_efficiency * 1.3).min(max_speed_efficiency);
    }

    let max_rise = 0.9;
    *speed += (target_speed - *speed).min(max_rise * *speed);

    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);

    for n in nodes.iter() {
        min_x = min_x.min(n.pos.x);
        max_x = max_x.max(n.pos.x);
        min_y = min_y.min(n.pos.y);
        max_y = max_y.max(n.pos.y);
    }

    let graph_extent = if min_x.is_finite() && max_x.is_finite() && max_x > min_x {
        (max_x - min_x).max(max_y - min_y)
    } else {
        200.0
    };

    let max_disp = (graph_extent * 0.05).clamp(10.0, 250.0);

    for n in nodes.iter_mut() {
        let swinging = n.old_force.sub(n.force).length();
        let mut factor = *speed / (2.0 + (*speed * swinging).sqrt());
        factor *= slow_down;

        if adjust_sizes {
            let max_dim = if n.size_wh.x > 0.0 || n.size_wh.y > 0.0 {
                (n.size_wh.x.max(n.size_wh.y)) / 2.0
            } else {
                n.size
            };
            if max_dim > 0.0 {
                factor = factor.min(10.0 / max_dim);
            }
        }

        let displacement = n.force.scale(factor);
        let disp_len = displacement.length();
        let final_disp = if disp_len > max_disp {
            displacement.scale(max_disp / disp_len)
        } else {
            displacement
        };

        n.pos = n.pos.add(final_disp);
    }
}
