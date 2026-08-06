use crate::force_atlas2::{self, force_atlas2_step, Edge as FA2Edge, Node as FA2Node, Settings as FA2Settings};
use graphene_core::{math::Vec2, GraphState, HierarchyExt, NodeId};
use std::collections::HashSet;

/// Simulation termination conditions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StopCondition {
    /// Auto stop when node displacement / kinetic energy drops below threshold
    Auto { min_energy: f64 },
    /// Auto stop when movement drops below a fraction of the analytical resting spring length
    Equilibrium { relative_tolerance: f64 },
    /// Temperature cooled simulated annealing stopping condition
    TempCooled { temperature: f64, cooling_rate: f64, min_temp: f64 },
    /// Run for a fixed number of iterations
    Iterations { max_iterations: usize },
}

impl Default for StopCondition {
    fn default() -> Self {
        Self::Equilibrium {
            relative_tolerance: 0.005,
        }
    }
}

/// Tunable parameters for live force-directed simulation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LiveSimParam {
    Repulsion(f32),
    Attraction(f32),
    Gravity(f32),
    IdealLength(f32),
    Temperature(f32),
    CoolingRate(f32),
    BarnesHut { enabled: bool, theta: f32 },
    // ForceAtlas2 specific settings
    ScalingRatio(f64),
    LinLog(bool),
    OutboundAttraction(bool),
    AdjustSizes(bool),
    EdgeWeightInfluence(f64),
    JitterTolerance(f64),
    StrongGravity(bool),
    SlowDown(f64),
    FixedNode(Option<usize>),
    StopCondition(StopCondition),
}

/// Live force-directed simulation powered by ForceAtlas2
#[derive(Clone)]
pub struct LiveForceSimulation {
    /// ForceAtlas2 algorithm settings
    pub settings: FA2Settings,
    /// ForceAtlas2 adaptive speed
    pub speed: f64,
    pub speed_efficiency: f64,
    /// Backward-compatibility parameters
    pub k_rep: f32,
    pub k_att: f32,
    pub gravity: f32,
    pub ideal_length: f32,
    pub temperature: f32,
    pub cooling_rate: f32,
    pub use_barnes_hut: bool,
    pub theta: f32,
    /// Simulation termination condition
    pub stop_condition: StopCondition,
    pub iteration_count: usize,
    pub last_displacement: f64,
    /// Nodes currently pinned as kinematic bodies (e.g. during drag)
    pub pinned_nodes: HashSet<NodeId>,
    /// Cached node state (for old force retention)
    cached_nodes: Vec<FA2Node>,
}

impl LiveForceSimulation {
    /// Create a new live force simulation with default parameters
    pub fn new() -> Self {
        let settings = FA2Settings::default();
        Self {
            settings: settings.clone(),
            speed: 1.0,
            speed_efficiency: 1.0,
            k_rep: (settings.scaling_ratio * 1000.0) as f32,
            k_att: 0.06,
            gravity: settings.gravity as f32,
            ideal_length: 50.0,
            temperature: 10.0,
            cooling_rate: 0.95,
            use_barnes_hut: settings.barnes_hut_optimize,
            theta: settings.barnes_hut_theta as f32,
            stop_condition: StopCondition::default(),
            iteration_count: 0,
            last_displacement: 1.0,
            pinned_nodes: HashSet::new(),
            cached_nodes: Vec::new(),
        }
    }

    /// Pin a node to prevent force simulation displacement during user dragging
    pub fn pin_node(&mut self, id: NodeId) {
        self.pinned_nodes.insert(id);
    }

    /// Unpin a node to resume normal force simulation
    pub fn unpin_node(&mut self, id: NodeId) {
        self.pinned_nodes.remove(&id);
    }

    /// Clear all kinematic pinned nodes
    pub fn clear_pinned_nodes(&mut self) {
        self.pinned_nodes.clear();
    }

    /// Reset internal step counters and temperature
    pub fn reset_simulation(&mut self) {
        self.iteration_count = 0;
        self.speed = 1.0;
        self.speed_efficiency = 1.0;
        self.temperature = 10.0;
        self.last_displacement = 1.0;
        self.pinned_nodes.clear();
        self.cached_nodes.clear();
    }

    /// Automatically infer optimal ForceAtlas2 settings from the target graph state
    pub fn infer_settings_from_state(&mut self, state: &GraphState<impl Copy + Default>) {
        let n = state.node_index_to_id.len();
        let e = state.edges.len();
        let avg_radius = if n > 0 {
            let sum_r: f32 = (0..n)
                .map(|i| {
                    let s = *state.sizes.get(i);
                    s.w.max(s.h) / 2.0
                })
                .sum();
            (sum_r / n as f32) as f64
        } else {
            20.0
        };

        self.settings = FA2Settings::infer_settings(n, e, avg_radius);
        self.use_barnes_hut = self.settings.barnes_hut_optimize;
        self.theta = self.settings.barnes_hut_theta as f32;
        self.gravity = self.settings.gravity as f32;
    }

    /// Update a simulation parameter live without resetting state
    pub fn update_param(&mut self, param: LiveSimParam) {
        match param {
            LiveSimParam::Repulsion(v) => {
                self.k_rep = v;
                self.settings.scaling_ratio = (v as f64).max(0.1);
            }
            LiveSimParam::Attraction(v) => self.k_att = v,
            LiveSimParam::Gravity(v) => {
                self.gravity = v;
                self.settings.gravity = v as f64;
            }
            LiveSimParam::IdealLength(v) => self.ideal_length = v,
            LiveSimParam::Temperature(v) => self.temperature = v,
            LiveSimParam::CoolingRate(v) => self.cooling_rate = v,
            LiveSimParam::BarnesHut { enabled, theta } => {
                self.use_barnes_hut = enabled;
                self.theta = theta;
                self.settings.barnes_hut_optimize = enabled;
                self.settings.barnes_hut_theta = theta as f64;
            }
            LiveSimParam::ScalingRatio(v) => self.settings.scaling_ratio = v,
            LiveSimParam::LinLog(v) => self.settings.lin_log_mode = v,
            LiveSimParam::OutboundAttraction(v) => self.settings.outbound_attraction_distribution = v,
            LiveSimParam::AdjustSizes(v) => self.settings.adjust_sizes = v,
            LiveSimParam::EdgeWeightInfluence(v) => self.settings.edge_weight_influence = v,
            LiveSimParam::JitterTolerance(v) => self.settings.jitter_tolerance = v,
            LiveSimParam::StrongGravity(v) => self.settings.strong_gravity_mode = v,
            LiveSimParam::SlowDown(v) => self.settings.slow_down = v,
            LiveSimParam::FixedNode(idx) => self.settings.fixed_node_idx = idx,
            LiveSimParam::StopCondition(cond) => self.stop_condition = cond,
        }
    }

    /// Advance the simulation by one ForceAtlas2 step
    pub fn tick(&mut self, state: &mut GraphState<impl Copy + Default>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        if self.iteration_count == 0 {
            self.infer_settings_from_state(state);
        }

        // Calculate node degrees and build FA2 edges
        let mut degrees = vec![0usize; n];
        let mut edges = Vec::new();

        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                if u < n && v < n {
                    degrees[u] += 1;
                    degrees[v] += 1;
                    edges.push(FA2Edge {
                        source: u,
                        target: v,
                        weight: 1.0,
                    });
                }
            }
        }

        // Sync cached nodes
        if self.cached_nodes.len() != n {
            self.cached_nodes = (0..n)
                .map(|i| {
                    let pos = *state.positions.get(i);
                    let size = *state.sizes.get(i);
                    let radius = (size.w.max(size.h) / 2.0) as f64 + 5.0;
                    let mass = (degrees[i] + 1) as f64;
                    let mut node = FA2Node::new(pos.x as f64, pos.y as f64, mass);
                    node.size = radius;
                    node.size_wh = force_atlas2::Vec2::new(size.w as f64, size.h as f64);
                    node
                })
                .collect();
        } else {
            for i in 0..n {
                let pos = *state.positions.get(i);
                let size = *state.sizes.get(i);
                self.cached_nodes[i].pos = force_atlas2::Vec2::new(pos.x as f64, pos.y as f64);
                self.cached_nodes[i].mass = (degrees[i] + 1) as f64;
                self.cached_nodes[i].size = (size.w.max(size.h) / 2.0) as f64 + 5.0;
                self.cached_nodes[i].size_wh = force_atlas2::Vec2::new(size.w as f64, size.h as f64);
            }
        }

        // Dynamic Kinetic Energy Theta Ramping:
        // Start theta at 0.5 during initial chaotic movements for high long-range precision,
        // then smoothly ramp theta up to 1.2 as kinetic displacement cools down.
        if self.settings.barnes_hut_optimize {
            let cool_ratio = (1.0 - (self.last_displacement / 10.0).min(1.0)).max(0.0);
            self.settings.barnes_hut_theta = 0.5 + 0.7 * cool_ratio;
        }

        // Two-Phase Speed Optimization: Use fast point-repulsion during initial coarse iterations (< 15) for N >= 50
        let original_adjust_sizes = self.settings.adjust_sizes;
        if n >= 50 && self.iteration_count < 15 {
            self.settings.adjust_sizes = false;
        }

        // Execute ForceAtlas2 iteration
        let disp = force_atlas2_step(
            &mut self.cached_nodes,
            &edges,
            &self.settings,
            &mut self.speed,
            &mut self.speed_efficiency,
        );

        self.settings.adjust_sizes = original_adjust_sizes;

        self.last_displacement = disp;
        self.iteration_count += 1;

        // Sync positions back to GraphState, preserving pinned kinematic nodes
        for i in 0..n {
            let node_id = state.node_index_to_id[i];
            if self.pinned_nodes.contains(&node_id) {
                let p = *state.positions.get(i);
                self.cached_nodes[i].pos = force_atlas2::Vec2::new(p.x as f64, p.y as f64);
                self.cached_nodes[i].old_force = force_atlas2::Vec2::zero();
            } else {
                let p = self.cached_nodes[i].pos;
                state.positions.set(i, Vec2::new(p.x as f32, p.y as f32));
            }
        }

        crate::collision::center_layout_at_origin(state);

        if matches!(self.stop_condition, StopCondition::TempCooled { .. }) {
            self.temperature *= self.cooling_rate;
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }

    /// Resolve node collisions to prevent overlap
    pub fn resolve_collisions(&self, state: &mut GraphState<impl Copy + Default>, padding: f32) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();
        let sizes: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let size = *state.sizes.get(i);
                (size.w, size.h)
            })
            .collect();

        for i in 0..n {
            let pos_i = positions[i];
            let (w_i, h_i) = sizes[i];

            for j in (i + 1)..n {
                let pos_j = positions[j];
                let (w_j, h_j) = sizes[j];

                let dx = pos_i.x - pos_j.x;
                let dy = pos_i.y - pos_j.y;
                let dist_sq = dx * dx + dy * dy;

                let min_dist_x = (w_i + w_j) / 2.0 + padding;
                let min_dist_y = (h_i + h_j) / 2.0 + padding;

                if dist_sq < min_dist_x * min_dist_x && dist_sq < min_dist_y * min_dist_y {
                    let dist = dist_sq.sqrt().max(0.1);
                    let overlap_x = min_dist_x - dist.abs();
                    let overlap_y = min_dist_y - dist.abs();

                    let fx = dx / dist * overlap_x;
                    let fy = dy / dist * overlap_y;

                    if let (Some(&idx_i), Some(&idx_j)) = (
                        state.node_keys.get(state.node_index_to_id[i]),
                        state.node_keys.get(state.node_index_to_id[j]),
                    ) {
                        let is_parent_child =
                            state.is_ancestor(idx_i, idx_j) || state.is_ancestor(idx_j, idx_i);

                        if is_parent_child {
                            if state.is_ancestor(idx_j, idx_i) {
                                let curr_pos = *state.positions.get(idx_j);
                                state.positions.set(idx_j, curr_pos + Vec2::new(fx, fy));
                            } else if state.is_ancestor(idx_i, idx_j) {
                                let curr_pos = *state.positions.get(idx_i);
                                state.positions.set(idx_i, curr_pos + Vec2::new(-fx, -fy));
                            }
                        } else {
                            let curr_pos_i = *state.positions.get(idx_i);
                            state.positions.set(idx_i, curr_pos_i + Vec2::new(fx, fy));

                            let curr_pos_j = *state.positions.get(idx_j);
                            state.positions.set(idx_j, curr_pos_j + Vec2::new(-fx, -fy));
                        }
                    } else {
                        let curr_pos_i = *state.positions.get(i);
                        state.positions.set(i, curr_pos_i + Vec2::new(fx, fy));

                        let curr_pos_j = *state.positions.get(j);
                        state.positions.set(j, curr_pos_j + Vec2::new(-fx, -fy));
                    }
                }
            }
        }

        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }

    /// Calculate analytical resting spring length (equilibrium distance) for ForceAtlas2:
    /// L_rest = sqrt(k_r * m_avg^2 / k_a)
    pub fn resting_spring_length(&self, state: &GraphState<impl Copy + Default>) -> f64 {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return 50.0;
        }

        let kr = self.settings.scaling_ratio;
        let ka = if self.settings.outbound_attraction_distribution {
            let sum_mass: f64 = (0..n).map(|i| (state.edges.len() / n.max(1) + 1) as f64).sum();
            sum_mass / n as f64
        } else {
            1.0
        };

        let avg_mass = if n > 0 {
            (2.0 * state.edges.len() as f64 / n as f64) + 1.0
        } else {
            2.0
        };

        (kr * avg_mass * avg_mass / ka).sqrt().max(1.0)
    }

    /// Check if simulation has converged for a given GraphState according to active StopCondition
    pub fn is_converged_for_state(&self, state: &GraphState<impl Copy + Default>) -> bool {
        match self.stop_condition {
            StopCondition::Auto { min_energy } => {
                let threshold = if min_energy > 0.0 {
                    min_energy
                } else {
                    0.005 * self.resting_spring_length(state)
                };
                self.last_displacement < threshold
            }
            StopCondition::Equilibrium { relative_tolerance } => {
                let threshold = relative_tolerance * self.resting_spring_length(state);
                self.last_displacement < threshold
            }
            StopCondition::TempCooled { min_temp, .. } => (self.temperature as f64) < min_temp,
            StopCondition::Iterations { max_iterations } => self.iteration_count >= max_iterations,
        }
    }

    /// Check if simulation has converged according to active StopCondition
    pub fn is_converged(&self) -> bool {
        match self.stop_condition {
            StopCondition::Auto { min_energy } => self.last_displacement < min_energy.max(0.01),
            StopCondition::Equilibrium { relative_tolerance } => self.last_displacement < (relative_tolerance * 50.0).max(0.01),
            StopCondition::TempCooled { min_temp, .. } => (self.temperature as f64) < min_temp,
            StopCondition::Iterations { max_iterations } => self.iteration_count >= max_iterations,
        }
    }
}

impl Default for LiveForceSimulation {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Copy + Default> crate::traits::IterativeLayout<S> for LiveForceSimulation {
    fn step(&mut self, state: &mut GraphState<S>) -> bool {
        if self.is_converged_for_state(state) {
            return false;
        }
        self.tick(state);
        !self.is_converged_for_state(state)
    }

    fn is_converged(&self) -> bool {
        LiveForceSimulation::is_converged(self)
    }
}

/// Thread-safe immutable frame snapshot containing position and sizing data for UI frame rendering.
#[derive(Debug, Clone, Default)]
pub struct RenderSnapshot {
    pub positions: Vec<Vec2>,
    pub sizes: Vec<graphene_core::Size2>,
    pub version: u64,
    pub is_ui_mode: bool,
}

/// Handle managing a background simulation worker thread.
pub struct AsyncLiveSimulationHandle {
    snapshot: std::sync::Arc<std::sync::RwLock<RenderSnapshot>>,
    stop_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl AsyncLiveSimulationHandle {
    pub fn spawn<S: Copy + Default + Send + Sync + 'static>(
        mut sim: LiveForceSimulation,
        mut state: GraphState<S>,
        max_iterations: usize,
    ) -> Self {
        let n = state.node_index_to_id.len();
        let initial_positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();
        let initial_sizes: Vec<graphene_core::Size2> = (0..n).map(|i| *state.sizes.get(i)).collect();

        let snapshot = std::sync::Arc::new(std::sync::RwLock::new(RenderSnapshot {
            positions: initial_positions,
            sizes: initial_sizes,
            version: 0,
            is_ui_mode: state.is_ui_mode,
        }));
        let stop_signal = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        let snapshot_clone = std::sync::Arc::clone(&snapshot);
        let stop_clone = std::sync::Arc::clone(&stop_signal);

        let thread_handle = std::thread::spawn(move || {
            for step in 1..=max_iterations {
                if stop_clone.load(std::sync::atomic::Ordering::Relaxed) || sim.is_converged() {
                    break;
                }

                sim.tick(&mut state);

                let n_curr = state.node_index_to_id.len();
                let current_positions: Vec<Vec2> = (0..n_curr).map(|i| *state.positions.get(i)).collect();
                let current_sizes: Vec<graphene_core::Size2> = (0..n_curr).map(|i| *state.sizes.get(i)).collect();

                if let Ok(mut lock) = snapshot_clone.write() {
                    lock.positions = current_positions;
                    lock.sizes = current_sizes;
                    lock.version = step as u64;
                    lock.is_ui_mode = state.is_ui_mode;
                }
            }
        });

        Self {
            snapshot,
            stop_signal,
            thread_handle: Some(thread_handle),
        }
    }

    pub fn latest_snapshot(&self) -> RenderSnapshot {
        self.snapshot
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    pub fn stop(&self) {
        self.stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn join(mut self) {
        self.stop();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    pub fn apply_to_graph_state<S: Copy + Default>(&self, state: &mut GraphState<S>) {
        let snap = self.latest_snapshot();
        for (i, &pos) in snap.positions.iter().enumerate() {
            if i < state.node_index_to_id.len() {
                state.positions.set(i, pos);
            }
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

impl Drop for AsyncLiveSimulationHandle {
    fn drop(&mut self) {
        self.stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{math::Size2, GraphState};

    #[test]
    fn test_resting_spring_length_and_auto_stop() {
        let mut state = GraphState::<()>::default();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(20.0, 20.0));
        let n2 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(20.0, 20.0));
        state.add_edge(n1, n2, graphene_core::EdgeData::default());

        let mut sim = LiveForceSimulation::new();
        let l_rest = sim.resting_spring_length(&state);
        assert!(l_rest > 0.0, "Resting spring length must be positive: {}", l_rest);

        sim.stop_condition = StopCondition::Equilibrium { relative_tolerance: 0.01 };
        sim.last_displacement = 0.0001;
        assert!(sim.is_converged_for_state(&state), "Simulation should be converged when displacement < tolerance * L_rest");
    }
}
