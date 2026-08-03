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
    pub force: Vec2,      // current force (dx, dy)
    pub old_force: Vec2,  // previous iteration force
    pub mass: f64,        // degree + 1
    pub size: f64,        // for adjust_sizes (radius)
}

impl Node {
    pub fn new(x: f64, y: f64, mass: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            force: Vec2::zero(),
            old_force: Vec2::zero(),
            mass,
            size: 0.0,
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
    pub edge_weight_influence: f64,             // 0 = ignore weights, 1 = normal
    pub jitter_tolerance: f64,                  // ~1.0 recommended
    pub barnes_hut_optimize: bool,
    pub barnes_hut_theta: f64,                  // ~1.2
    pub scaling_ratio: f64,                     // repulsion strength (higher = more spread)
    pub strong_gravity_mode: bool,
    pub gravity: f64,
    pub slow_down: f64,                         // multiplies final speed (usually 1.0)
    pub fixed_node_idx: Option<usize>,          // Pin specific node (e.g. node 0) at origin (0,0)
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
        settings.barnes_hut_theta = 0.5; // Start at 0.5 for high precision during initial chaotic ticks

        // Analytical Repulsion Scaling: Scale logarithmically for dense graphs to prevent collapse
        let v_scale = (nodes_count as f64).ln().max(1.0);
        let base_scaling = if avg_node_radius > 0.0 {
            (0.0005 * avg_node_radius * v_scale).clamp(0.01, 0.15)
        } else {
            (0.01 * v_scale).clamp(0.01, 0.15)
        };

        settings.scaling_ratio = base_scaling;
        settings.adjust_sizes = true;

        // Density-calibrated gravity: Relax gravity for dense networks (rho -> 1), tighten for sparse networks (rho -> 0)
        let g_base = 1.0;
        settings.gravity = g_base * (1.0 - density);

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
            scaling_ratio: 0.02,
            strong_gravity_mode: false,
            gravity: 1.0,
            slow_down: 1.0,
            fixed_node_idx: Some(0),
        }
    }
}

/// Barnes-Hut region (quadtree node)
struct Region {
    mass: f64,
    mass_center: Vec2,
    size: f64,
    nodes: Vec<usize>,          // indices into the global nodes vec
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

    fn apply_force(&self, n_idx: usize, nodes: &mut [Node], theta: f64, coefficient: f64, adjust_sizes: bool) {
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

fn lin_repulsion(i: usize, j: usize, nodes: &mut [Node], coefficient: f64, adjust_sizes: bool) {
    let (n1, n2) = if i < j {
        let (a, b) = nodes.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = nodes.split_at_mut(i);
        (&mut b[0], &mut a[j])
    };

    let dist_vec = n1.pos.sub(n2.pos);
    let euclidean = dist_vec.length();

    if euclidean == 0.0 {
        return;
    }

    let factor = if adjust_sizes {
        let distance = euclidean - n1.size - n2.size;
        if distance > 0.0 {
            coefficient * n1.mass * n2.mass / (distance * distance)
        } else {
            // Strong push on overlap
            100.0 * coefficient * n1.mass * n2.mass
        }
    } else {
        coefficient * n1.mass * n2.mass / (euclidean * euclidean)
    };

    let force = dist_vec.scale(factor);
    n1.force = n1.force.add(force);
    n2.force = n2.force.sub(force);
}

fn lin_repulsion_region(i: usize, region: &Region, nodes: &mut [Node], coefficient: f64, adjust_sizes: bool) {
    let n = &mut nodes[i];
    let dist_vec = n.pos.sub(region.mass_center);
    let euclidean = dist_vec.length();

    if euclidean == 0.0 {
        return;
    }

    let factor = if adjust_sizes {
        let distance = euclidean - n.size;
        if distance > 0.0 {
            coefficient * n.mass * region.mass / (distance * distance)
        } else {
            100.0 * coefficient * n.mass * region.mass
        }
    } else {
        coefficient * n.mass * region.mass / (euclidean * euclidean)
    };

    n.force = n.force.add(dist_vec.scale(factor));
}

fn lin_gravity(n: &mut Node, g: f64) {
    let d = n.pos.length();
    if d > 0.0 {
        let factor = n.mass * g / d;
        n.force = n.force.sub(n.pos.scale(factor));
    }
}

fn strong_gravity(n: &mut Node, g: f64, coefficient: f64) {
    if n.pos.x != 0.0 || n.pos.y != 0.0 {
        let factor = coefficient * n.mass * g;
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
    let (n1, n2) = if i < j {
        let (a, b) = nodes.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = nodes.split_at_mut(i);
        (&mut b[0], &mut a[j])
    };

    let dist_vec = n1.pos.sub(n2.pos);
    let euclidean = dist_vec.length();

    if adjust_sizes {
        let distance = euclidean - n1.size - n2.size;
        if distance <= 0.0 {
            return; // no attraction while overlapping
        }
    }

    let factor = if distributed {
        -coefficient * weight / n1.mass
    } else {
        -coefficient * weight
    };

    let force = dist_vec.scale(factor);
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
    let (n1, n2) = if i < j {
        let (a, b) = nodes.split_at_mut(j);
        (&mut a[i], &mut b[0])
    } else {
        let (a, b) = nodes.split_at_mut(i);
        (&mut b[0], &mut a[j])
    };

    let dist_vec = n1.pos.sub(n2.pos);
    let euclidean = dist_vec.length();

    if euclidean == 0.0 {
        return;
    }

    if adjust_sizes {
        let distance = euclidean - n1.size - n2.size;
        if distance <= 0.0 {
            return;
        }
    }

    let log_factor = (1.0 + euclidean).ln() / euclidean;
    let factor = if distributed {
        -coefficient * weight * log_factor / n1.mass
    } else {
        -coefficient * weight * log_factor
    };

    let force = dist_vec.scale(factor);
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

    // Adjust efficiency
    if total_effective_traction > 0.0 && total_swinging / total_effective_traction > 2.0 {
        if *speed_efficiency > min_speed_efficiency {
            *speed_efficiency *= 0.5;
        }
    }

    let target_speed = if total_swinging == 0.0 {
        f64::INFINITY
    } else {
        jt * *speed_efficiency * total_effective_traction / total_swinging
    };

    if total_swinging > jt * total_effective_traction {
        if *speed_efficiency > min_speed_efficiency {
            *speed_efficiency *= 0.7;
        }
    } else if *speed < 1000.0 {
        *speed_efficiency *= 1.3;
    }

    // Limit speed rise
    let max_rise = 0.5;
    *speed += (target_speed - *speed).min(max_rise * *speed);

    // Apply forces
    for n in nodes.iter_mut() {
        let swinging = n.mass * n.old_force.sub(n.force).length();
        let mut factor = *speed / (1.0 + (*speed * swinging).sqrt());
        factor *= slow_down;

        if adjust_sizes && n.size > 0.0 {
            factor = factor.min(10.0 / n.size);
        }

        let displacement = n.force.scale(factor);
        let disp_len = displacement.length();
        let max_disp = 12.0;
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
            strong_gravity(n, settings.gravity, settings.scaling_ratio);
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
        nodes.push(Node::new(r * angle.cos(), r * angle.sin(), (degree[i] + 1) as f64));
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
        let adj = vec![
            vec![1],
            vec![0, 2],
            vec![1, 3],
            vec![2],
        ];
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
}
