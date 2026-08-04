//! ForceAtlas2 layout algorithm in pure Rust.
//! Based on Jacomy et al. 2014 (PLOS ONE) and the Gephi reference implementation.

use std::f64::consts::PI;

/// 2-D vector helper for double-precision physics calculations
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
    pub fn length_sq(self) -> f64 {
        self.x * self.x + self.y * self.y
    }
    pub fn length(self) -> f64 {
        self.length_sq().sqrt()
    }
    pub fn sub(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x - other.x, self.y - other.y)
    }
    pub fn add(self, other: Vec2) -> Vec2 {
        Vec2::new(self.x + other.x, self.y + other.y)
    }
    pub fn scale(self, s: f64) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}

/// A graph node for ForceAtlas2
#[derive(Clone, Debug)]
pub struct Node {
    pub pos: Vec2,
    pub force: Vec2,     // current force (dx, dy)
    pub old_force: Vec2, // previous iteration force
    pub mass: f64,       // degree + 1
    pub size: f64,       // for adjust_sizes (radius fallback)
    pub size_wh: Vec2, // rectangular node dimensions (width, height) for exact AABB size awareness
}

impl Node {
    pub fn new(x: f64, y: f64, mass: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            force: Vec2::zero(),
            old_force: Vec2::zero(),
            mass,
            size: 0.0,
            size_wh: Vec2::zero(),
        }
    }
}

/// An undirected edge (indices into the nodes vector)
#[derive(Clone, Debug)]
pub struct Edge {
    pub source: usize,
    pub target: usize,
    pub weight: f64,
}

/// ForceAtlas2 algorithm settings
#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub lin_log_mode: bool,
    pub outbound_attraction_distribution: bool, // dissuade hubs
    pub adjust_sizes: bool,
    pub edge_weight_influence: f64, // 0 = ignore weights, 1 = normal
    pub jitter_tolerance: f64,      // ~1.0 recommended
    pub barnes_hut_optimize: bool,
    pub barnes_hut_theta: f64, // ~1.2
    pub scaling_ratio: f64,    // repulsion strength (higher = more spread)
    pub strong_gravity_mode: bool,
    pub gravity: f64,
    pub slow_down: f64,                // multiplies final speed (usually 1.0)
    pub fixed_node_idx: Option<usize>, // Pin specific node (e.g. node 0) at origin (0,0)
}

impl Settings {
    /// Infer sensible baseline ForceAtlas2 parameters based on graph size (|V|),
    /// edge count (|E|), and average physical node radius in pixel space.
    /// Modeled after Graphology ForceAtlas2 inferSettings().
    pub fn infer_settings(nodes_count: usize, edges_count: usize, avg_node_radius: f64) -> Self {
        let mut settings = Self::default();

        if nodes_count == 0 {
            return settings;
        }

        // Calculate network density ρ = 2|E| / (|V|(|V|-1))
        let density = if nodes_count > 1 {
            ((2.0 * edges_count as f64) / (nodes_count * (nodes_count - 1)) as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Enable Barnes-Hut optimization for larger graphs (|V| >= 50)
        settings.barnes_hut_optimize = nodes_count >= 50;
        settings.barnes_hut_theta = if nodes_count >= 50 { 0.5 } else { 1.2 };

        // Analytical Repulsion Scaling based on node physical size and scale
        let v_scale = (nodes_count as f64).ln().max(1.0);
        let base_scaling = if avg_node_radius > 0.0 {
            (10.0 + 0.5 * avg_node_radius * v_scale).clamp(10.0, 100.0)
        } else {
            (10.0 * v_scale).clamp(10.0, 100.0)
        };

        settings.scaling_ratio = base_scaling;
        settings.adjust_sizes = true;

        // Density-calibrated gravity: Relax gravity for dense networks (rho -> 1), tighten for sparse networks (rho -> 0)
        let g_base = 1.0;
        settings.gravity = (g_base * (1.0 - 0.8 * density)).clamp(0.1, 2.0);

        settings
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            lin_log_mode: false,
            outbound_attraction_distribution: false,
            adjust_sizes: true,
            edge_weight_influence: 1.0,
            jitter_tolerance: 1.0,
            barnes_hut_optimize: true,
            barnes_hut_theta: 1.2,
            scaling_ratio: 25.0,
            strong_gravity_mode: false,
            gravity: 1.0,
            slow_down: 1.0,
            fixed_node_idx: None,
        }
    }
}

/// Barnes-Hut region (quadtree node)
struct Region {
    mass: f64,
    mass_center: Vec2,
    size: f64,
    nodes: Vec<usize>, // indices into the global nodes vec
    subregions: Vec<Region>,
}

impl Region {
    fn new(node_indices: Vec<usize>, nodes: &[Node]) -> Self {
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

    fn update_mass_and_geometry(&mut self, nodes: &[Node]) {
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

    fn build_subregions(&mut self, nodes: &[Node]) {
        if self.nodes.len() <= 1 {
            return;
        }

        // Partition into 4 quadrants relative to mass center
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
                // Degenerate: all points identical → one region per node
                for i in bucket {
                    let sub = Region::new(vec![i], nodes);
                    self.subregions.push(sub);
                }
            }
        }
    }

    fn apply_force(
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
            // Approximate
            lin_repulsion_region(n_idx, self, nodes, coefficient, adjust_sizes);
        } else {
            for sub in &self.subregions {
                sub.apply_force(n_idx, nodes, theta, coefficient, adjust_sizes);
            }
        }
    }
}

// ── Force functions ──────────────────────────────────────────────────────────

/// Compute the effective directional radius of a node along a given vector (`pos_diff`).
/// If `size_wh` (width & height) is non-zero, projects the ray onto the node's rectangular bounding box.
/// Otherwise, falls back to scalar circular radius (`node.size`).
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

fn lin_repulsion(i: usize, j: usize, nodes: &mut [Node], coefficient: f64, adjust_sizes: bool) {
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
            // Geometrically separate outside bounds + half node width/radius padding
            let padding = 0.5 * (r1 + r2);
            let target_dist = r1 + r2 + padding;
            let overlap = target_dist - euclidean;
            if euclidean > 1e-6 {
                let shift = dist_vec.scale(0.5 * overlap / euclidean);
                n1.pos = n1.pos.add(shift);
                n2.pos = n2.pos.sub(shift);
            }
            // Forces zeroed out for overlapping pair
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

fn lin_repulsion_region(
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

fn lin_gravity(n: &mut Node, g: f64) {
    let d = n.pos.length();
    if d > 1e-4 {
        let factor = n.mass * g / d;
        n.force = n.force.sub(n.pos.scale(factor));
    }
}

fn strong_gravity(n: &mut Node, g: f64) {
    if n.pos.x != 0.0 || n.pos.y != 0.0 {
        let factor = n.mass * g;
        n.force = n.force.sub(n.pos.scale(factor));
    }
}

fn lin_attraction(
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
            return; // no attraction while overlapping or touching
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

fn log_attraction(
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

// ── Adaptive speed ───────────────────────────────────────────────────────────

fn adjust_speed_and_apply_forces(
    nodes: &mut [Node],
    speed: &mut f64,
    speed_efficiency: &mut f64,
    jitter_tolerance: f64,
    adjust_sizes: bool,
    scaling_ratio: f64,
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

    // Adjust efficiency
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

    // Limit speed rise
    let max_rise = 0.9;
    *speed += (target_speed - *speed).min(max_rise * *speed);

    // Percentage-Based Scale-Invariant Displacement Clamping:
    // Max displacement per tick = 5% of current graph bounding extent (clamped to [10.0, 250.0] px)
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

    // Apply forces
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

// ── Simulation Step & Layout functions ───────────────────────────────────────

/// Run a single step of ForceAtlas2 layout.
/// Returns the average node displacement magnitude in this step.
pub fn force_atlas2_step(
    nodes: &mut [Node],
    edges: &[Edge],
    settings: &Settings,
    speed: &mut f64,
    speed_efficiency: &mut f64,
) -> f64 {
    if nodes.is_empty() {
        return 0.0;
    }

    let outbound_att_compensation = if settings.outbound_attraction_distribution {
        let sum_mass: f64 = nodes.iter().map(|n| n.mass).sum();
        sum_mass / nodes.len() as f64
    } else {
        1.0
    };

    // Save old forces and reset
    for n in nodes.iter_mut() {
        n.old_force = n.force;
        n.force = Vec2::zero();
    }

    // 1. Repulsion
    if settings.barnes_hut_optimize && nodes.len() > 50 {
        let indices: Vec<usize> = (0..nodes.len()).collect();
        let mut root = Region::new(indices, nodes);
        root.build_subregions(nodes);

        for i in 0..nodes.len() {
            root.apply_force(
                i,
                nodes,
                settings.barnes_hut_theta,
                settings.scaling_ratio,
                settings.adjust_sizes,
            );
        }
    } else {
        // Exact O(n²)
        for i in 0..nodes.len() {
            for j in 0..i {
                lin_repulsion(i, j, nodes, settings.scaling_ratio, settings.adjust_sizes);
            }
        }
    }

    // 2. Gravity
    for n in nodes.iter_mut() {
        if settings.strong_gravity_mode {
            strong_gravity(n, settings.gravity);
        } else {
            lin_gravity(n, settings.gravity);
        }
    }

    // 3. Attraction
    let attr_coefficient = if settings.outbound_attraction_distribution {
        outbound_att_compensation
    } else {
        1.0
    };

    for e in edges {
        let w = if settings.edge_weight_influence == 0.0 {
            1.0
        } else if settings.edge_weight_influence == 1.0 {
            e.weight
        } else {
            e.weight.powf(settings.edge_weight_influence)
        };

        if settings.lin_log_mode {
            log_attraction(
                e.source,
                e.target,
                w,
                nodes,
                settings.outbound_attraction_distribution,
                attr_coefficient,
                settings.adjust_sizes,
            );
        } else {
            lin_attraction(
                e.source,
                e.target,
                w,
                nodes,
                settings.outbound_attraction_distribution,
                attr_coefficient,
                settings.adjust_sizes,
            );
        }
    }

    // 4. Adaptive speed + apply forces
    let pos_before: Vec<Vec2> = nodes.iter().map(|n| n.pos).collect();
    adjust_speed_and_apply_forces(
        nodes,
        speed,
        speed_efficiency,
        settings.jitter_tolerance,
        settings.adjust_sizes,
        settings.scaling_ratio,
        settings.slow_down,
    );

    if let Some(fixed_idx) = settings.fixed_node_idx {
        if fixed_idx < nodes.len() {
            nodes[fixed_idx].pos = Vec2::new(0.0, 0.0);
        }
    }

    let mut total_disp = 0.0;
    for (i, n) in nodes.iter().enumerate() {
        total_disp += n.pos.sub(pos_before[i]).length();
    }
    total_disp / (nodes.len() as f64)
}

/// Run ForceAtlas2 for a fixed number of iterations.
///
/// * `nodes` – mutable list of nodes (positions will be updated)
/// * `edges` – list of edges
/// * `settings` – algorithm parameters
/// * `iterations` – number of steps
pub fn force_atlas2(nodes: &mut [Node], edges: &[Edge], settings: &Settings, iterations: usize) {
    if nodes.is_empty() {
        return;
    }

    let mut speed = 1.0;
    let mut speed_efficiency = 1.0;

    for _iter in 0..iterations {
        force_atlas2_step(nodes, edges, settings, &mut speed, &mut speed_efficiency);
    }
}

/// Build nodes + edges from a simple adjacency list (undirected, unweighted).
/// Initial positions are placed on a circle.
pub fn from_adjacency(adj: &[Vec<usize>]) -> (Vec<Node>, Vec<Edge>) {
    let n = adj.len();
    let mut nodes = Vec::with_capacity(n);
    let mut degree = vec![0usize; n];

    for (i, neighbors) in adj.iter().enumerate() {
        for &j in neighbors {
            if i < j {
                degree[i] += 1;
                degree[j] += 1;
            }
        }
    }

    for i in 0..n {
        let angle = 2.0 * PI * i as f64 / n as f64;
        let r = (n as f64).sqrt() * 10.0;
        nodes.push(Node::new(
            r * angle.cos(),
            r * angle.sin(),
            (degree[i] + 1) as f64,
        ));
    }

    let mut edges = Vec::new();
    for (i, neighbors) in adj.iter().enumerate() {
        for &j in neighbors {
            if i < j {
                edges.push(Edge {
                    source: i,
                    target: j,
                    weight: 1.0,
                });
            }
        }
    }

    (nodes, edges)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_force_atlas2_small_graph_positions_finite() {
        let adj = vec![vec![1], vec![0, 2], vec![1, 3], vec![2]];
        let (mut nodes, edges) = from_adjacency(&adj);

        let settings = Settings {
            barnes_hut_optimize: false,
            ..Default::default()
        };

        force_atlas2(&mut nodes, &edges, &settings, 100);

        for n in &nodes {
            assert!(n.pos.x.is_finite() && n.pos.y.is_finite());
        }
    }

    #[test]
    fn test_force_atlas2_equilateral_triangle_remains_2d() {
        // Triangle A -> B -> C -> A initialized in 2D triangle
        let mut nodes = vec![
            Node::new(0.0, 100.0, 3.0),
            Node::new(-86.6, -50.0, 3.0),
            Node::new(86.6, -50.0, 3.0),
        ];
        let edges = vec![
            Edge {
                source: 0,
                target: 1,
                weight: 1.0,
            },
            Edge {
                source: 1,
                target: 2,
                weight: 1.0,
            },
            Edge {
                source: 2,
                target: 0,
                weight: 1.0,
            },
        ];

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..100 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        // Triangle area formula: 0.5 * |x1(y2 - y3) + x2(y3 - y1) + x3(y1 - y2)|
        let area = 0.5
            * (nodes[0].pos.x * (nodes[1].pos.y - nodes[2].pos.y)
                + nodes[1].pos.x * (nodes[2].pos.y - nodes[0].pos.y)
                + nodes[2].pos.x * (nodes[0].pos.y - nodes[1].pos.y))
                .abs();

        assert!(
            area > 1.0,
            "Triangle should maintain 2D shape and not collapse into 1D line, area was {}",
            area
        );
    }

    #[test]
    fn test_force_atlas2_circular_graph_stability() {
        let n = 20;
        let mut adj = vec![Vec::new(); n];
        for i in 0..n {
            adj[i].push((i + 1) % n);
            adj[(i + 1) % n].push(i);
        }
        let (mut nodes, edges) = from_adjacency(&adj);

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..200 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        for n in &nodes {
            let dist = n.pos.length();
            assert!(dist.is_finite(), "Node position must be finite");
            assert!(
                dist < 2000.0,
                "Node position should not explode to infinity: dist = {}",
                dist
            );
        }
    }

    #[test]
    fn test_force_atlas2_rectangular_node_directional_radius() {
        let mut node = Node::new(0.0, 0.0, 1.0);
        node.size_wh = Vec2::new(100.0, 20.0);

        // Horizontal approach (along X axis)
        let r_x = effective_directional_radius(Vec2::new(200.0, 0.0), 200.0, &node);
        assert!(
            (r_x - 50.0).abs() < 1e-4,
            "Horizontal radius should equal half-width (50.0), got {}",
            r_x
        );

        // Vertical approach (along Y axis)
        let r_y = effective_directional_radius(Vec2::new(0.0, 100.0), 100.0, &node);
        assert!(
            (r_y - 10.0).abs() < 1e-4,
            "Vertical radius should equal half-height (10.0), got {}",
            r_y
        );
    }

    #[test]
    fn test_force_atlas2_overlap_repositioning_and_zero_force() {
        let mut nodes = vec![Node::new(0.0, 0.0, 1.0), Node::new(2.0, 0.0, 1.0)];
        nodes[0].size = 10.0;
        nodes[1].size = 10.0;

        lin_repulsion(0, 1, &mut nodes, 1.0, true);

        let dist = nodes[0].pos.sub(nodes[1].pos).length();
        // Combined radii = 20.0 + padding (10.0) = 30.0
        assert!(
            dist >= 25.0,
            "Overlapping nodes should be separated outside bounds with padding, dist = {}",
            dist
        );
        assert_eq!(
            nodes[0].force,
            Vec2::zero(),
            "Force on overlapping node should be zero"
        );
        assert_eq!(
            nodes[1].force,
            Vec2::zero(),
            "Force on overlapping node should be zero"
        );
    }

    #[test]
    fn test_force_atlas2_edge_clearance_visible() {
        let mut nodes = vec![Node::new(0.0, 0.0, 1.0), Node::new(500.0, 0.0, 1.0)];
        nodes[0].size_wh = Vec2::new(40.0, 40.0);
        nodes[1].size_wh = Vec2::new(40.0, 40.0);
        let edges = vec![Edge {
            source: 0,
            target: 1,
            weight: 1.0,
        }];

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..100 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        let dist = nodes[0].pos.sub(nodes[1].pos).length();
        let clearance = dist - 40.0; // Subtract radii (20 + 20)
        assert!(
            clearance > 5.0,
            "Edge clearance between node borders must be clearly visible (> 5px), got {}",
            clearance
        );
    }

    #[test]
    fn test_force_atlas2_edge_lengths_vary_by_degree() {
        // Hub node connected to leaves
        let mut nodes = vec![
            Node::new(0.0, 0.0, 5.0),   // Hub (mass 5)
            Node::new(50.0, 0.0, 1.0),  // Leaf 1 (mass 1)
            Node::new(-50.0, 0.0, 1.0), // Leaf 2 (mass 1)
            Node::new(0.0, 50.0, 1.0),  // Leaf 3 (mass 1)
        ];
        let edges = vec![
            Edge {
                source: 0,
                target: 1,
                weight: 1.0,
            },
            Edge {
                source: 0,
                target: 2,
                weight: 1.0,
            },
            Edge {
                source: 0,
                target: 3,
                weight: 1.0,
            },
        ];

        let settings = Settings::default();
        let mut speed = 1.0;
        let mut speed_eff = 1.0;

        for _ in 0..100 {
            force_atlas2_step(&mut nodes, &edges, &settings, &mut speed, &mut speed_eff);
        }

        let d_hub_leaf = nodes[0].pos.sub(nodes[1].pos).length();
        assert!(d_hub_leaf.is_finite(), "Distance must be finite");
        assert!(
            d_hub_leaf > 10.0,
            "Hub-leaf edge distance should be positive and non-zero"
        );
    }
}
