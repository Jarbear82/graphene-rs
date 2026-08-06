//! Types and helper structures for ForceAtlas2 layout.

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
    pub size_wh: Vec2,   // rectangular node dimensions (width, height) for exact AABB size awareness
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
        settings.gravity = (g_base * (1.5 - density)).clamp(0.2, 5.0);

        settings
    }
}
