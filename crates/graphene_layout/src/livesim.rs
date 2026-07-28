use crate::quadtree::Quadtree;
use graphene_core::{math::Vec2, GraphState, HierarchyExt};
use std::cell::RefCell;
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
}

/// Live force-directed simulation that can be advanced frame-by-frame
///
/// This struct maintains internal state for the simulation and allows
/// incremental updates via the `tick()` method.
pub struct LiveForceSimulation {
    /// Simulation parameters
    pub k_rep: f32,
    pub k_att: f32,
    pub gravity: f32,
    pub ideal_length: f32,
    /// Temperature for simulated annealing
    pub temperature: f32,
    pub cooling_rate: f32,
    pub use_barnes_hut: bool,
    pub theta: f32,
}

impl LiveForceSimulation {
    /// Create a new live force simulation with default parameters
    pub fn new() -> Self {
        Self {
            k_rep: 2500.0,
            k_att: 0.06,
            gravity: 0.3,
            ideal_length: 50.0,
            temperature: 10.0,
            cooling_rate: 0.95,
            use_barnes_hut: true,
            theta: 0.5,
        }
    }

    /// Update a simulation parameter live without resetting state
    pub fn update_param(&mut self, param: LiveSimParam) {
        match param {
            LiveSimParam::Repulsion(v) => self.k_rep = v,
            LiveSimParam::Attraction(v) => self.k_att = v,
            LiveSimParam::Gravity(v) => self.gravity = v,
            LiveSimParam::IdealLength(v) => self.ideal_length = v,
            LiveSimParam::Temperature(v) => self.temperature = v,
            LiveSimParam::CoolingRate(v) => self.cooling_rate = v,
            LiveSimParam::BarnesHut { enabled, theta } => {
                self.use_barnes_hut = enabled;
                self.theta = theta;
            }
        }
    }

    /// Advance the simulation by one step
    pub fn tick(&mut self, state: &mut GraphState<impl Copy + Default>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        // Use RefCell pattern to get mutable references to positions
        // This is safe because we're not aliasing during the tick
        let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();

        let mut displacements = vec![Vec2::default(); n];

        // Barnes-Hut approximation for repulsion
        if self.use_barnes_hut && n > 100 {
            let quadtree = Quadtree::build(&positions);
            for i in 0..n {
                let pos_i = positions[i];
                let force =
                    quadtree.accumulate_repulsion(i, pos_i, &positions, self.k_rep, self.theta);
                displacements[i] += force;
            }
        } else {
            // Direct N-body calculation
            for i in 0..n {
                let pos_i = positions[i];
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let pos_j = positions[j];
                    let delta = pos_i - pos_j;
                    let dist = delta.len();
                    if dist > 0.1 {
                        let force = self.k_rep / (dist * dist);
                        let dir = delta.normalize();
                        displacements[i] += dir * force;
                    }
                }
            }
        }

        // Edge attractions
        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);
            if let (Some(&u), Some(&v)) = (state.node_keys.get(src), state.node_keys.get(tgt)) {
                let pos_u = positions[u];
                let pos_v = positions[v];
                let delta = pos_v - pos_u;
                let dist = delta.len();
                if dist > 0.1 {
                    let force = self.k_att * (dist - self.ideal_length);
                    let dir = delta.normalize();
                    displacements[u] += dir * force;
                    displacements[v] -= dir * force;
                }
            }
        }

        // Gravity towards center
        let mut center = Vec2::default();
        for i in 0..n {
            center += positions[i];
        }
        center = center / n as f32;

        for i in 0..n {
            let pos = positions[i];
            let delta = center - pos;
            displacements[i] += delta * self.gravity;
        }

        // Apply displacements with temperature cap
        for i in 0..n {
            let disp = displacements[i];
            let disp_len = disp.len();
            if disp_len > 0.01 {
                let capped_disp = disp.normalize() * disp_len.min(self.temperature);
                let old_pos = *state.positions.get(i);
                state.positions.set(i, old_pos + capped_disp);
            }
        }

        // Apply collision resolution
        self.resolve_collisions(state, 2.0);

        // Cool down
        self.temperature *= self.cooling_rate;
    }

    /// Resolve node collisions to prevent overlap
    pub fn resolve_collisions(&self, state: &mut GraphState<impl Copy + Default>, padding: f32) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        // Get current positions and sizes
        let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();

        let sizes: Vec<(f32, f32)> = (0..n)
            .map(|i| {
                let size = *state.sizes.get(i);
                (size.w, size.h)
            })
            .collect();

        // Simple overlap resolution using hierarchy-aware collision detection
        for i in 0..n {
            let pos_i = positions[i];
            let (w_i, h_i) = sizes[i];

            for j in (i + 1)..n {
                let pos_j = positions[j];
                let (w_j, h_j) = sizes[j];

                // Check if nodes overlap
                let dx = pos_i.x - pos_j.x;
                let dy = pos_i.y - pos_j.y;
                let dist_sq = dx * dx + dy * dy;

                let min_dist_x = (w_i + w_j) / 2.0 + padding;
                let min_dist_y = (h_i + h_j) / 2.0 + padding;

                if dist_sq < min_dist_x * min_dist_x && dist_sq < min_dist_y * min_dist_y {
                    let dist = dist_sq.sqrt().max(0.1);
                    let overlap_x = min_dist_x - dist.abs();
                    let overlap_y = min_dist_y - dist.abs();

                    // Calculate repulsion direction
                    let fx = dx / dist * overlap_x;
                    let fy = dy / dist * overlap_y;

                    // Apply repulsion with hierarchy awareness
                    if let (Some(&idx_i), Some(&idx_j)) = (
                        state.node_keys.get(state.node_index_to_id[i]),
                        state.node_keys.get(state.node_index_to_id[j]),
                    ) {
                        // Check hierarchy relationship
                        let is_parent_child =
                            state.is_ancestor(idx_i, idx_j) || state.is_ancestor(idx_j, idx_i);

                        if is_parent_child {
                            // Parent-child: move child away from parent
                            if state.is_ancestor(idx_j, idx_i) {
                                // j is child of i - move j away
                                let curr_pos = *state.positions.get(idx_j);
                                state.positions.set(idx_j, curr_pos + Vec2::new(fx, fy));
                            } else if state.is_ancestor(idx_i, idx_j) {
                                // i is child of j - move i away
                                let curr_pos = *state.positions.get(idx_i);
                                state.positions.set(idx_i, curr_pos + Vec2::new(-fx, -fy));
                            }
                        } else {
                            // Independent nodes: repel both
                            let curr_pos_i = *state.positions.get(idx_i);
                            state.positions.set(idx_i, curr_pos_i + Vec2::new(fx, fy));

                            let curr_pos_j = *state.positions.get(idx_j);
                            state.positions.set(idx_j, curr_pos_j + Vec2::new(-fx, -fy));
                        }
                    } else {
                        // Fallback: move both nodes
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
}

impl Default for LiveForceSimulation {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Copy + Default> crate::traits::IterativeLayout<S> for LiveForceSimulation {
    fn step(&mut self, state: &mut GraphState<S>) -> bool {
        if self.temperature < 0.01 {
            return false;
        }
        self.tick(state);
        self.temperature >= 0.01
    }

    fn is_converged(&self) -> bool {
        self.temperature < 0.01
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
/// Communicates via `Arc<RwLock<RenderSnapshot>>` for zero-lock-contention UI frame rendering.
pub struct AsyncLiveSimulationHandle {
    snapshot: std::sync::Arc<std::sync::RwLock<RenderSnapshot>>,
    stop_signal: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread_handle: Option<std::thread::JoinHandle<()>>,
}

impl AsyncLiveSimulationHandle {
    /// Spawn a background worker thread that executes `LiveForceSimulation::tick` iterations asynchronously.
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
                if stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
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

    /// Read the latest snapshot frame buffer using Arc read-lock.
    /// Fast and non-blocking for UI frame rendering.
    pub fn latest_snapshot(&self) -> RenderSnapshot {
        self.snapshot
            .read()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Signal the background simulation worker to stop.
    pub fn stop(&self) {
        self.stop_signal.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Wait for the background simulation thread to finish.
    pub fn join(mut self) {
        self.stop();
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }

    /// Synchronize the latest background positions back into a GraphState instance on the main thread.
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
