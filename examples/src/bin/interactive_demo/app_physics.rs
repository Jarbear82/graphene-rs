use crate::app::DemoApp;
use graphene_layout::{LiveSimParam, StopCondition};

impl DemoApp {
    pub fn sync_live_sim_params(&mut self) {
        self.live_sim.update_param(LiveSimParam::LinLog(self.fa2_lin_log));
        self.live_sim.update_param(LiveSimParam::OutboundAttraction(self.fa2_outbound));
        self.live_sim.update_param(LiveSimParam::StrongGravity(self.fa2_strong_gravity));
        self.live_sim.update_param(LiveSimParam::AdjustSizes(self.fa2_adjust_sizes));
        self.live_sim.update_param(LiveSimParam::ScalingRatio(self.fa2_scaling_ratio));
        self.live_sim.update_param(LiveSimParam::Gravity(self.gravity));
        self.live_sim.update_param(LiveSimParam::BarnesHut {
            enabled: self.use_barnes_hut,
            theta: self.theta,
        });

        let stop_cond = match self.fa2_stop_mode {
            0 => StopCondition::Auto { min_energy: 0.01 },
            1 => StopCondition::TempCooled {
                temperature: self.physics_temperature as f64,
                cooling_rate: 0.95,
                min_temp: 0.05,
            },
            2 => StopCondition::Iterations {
                max_iterations: self.fa2_max_iterations,
            },
            _ => StopCondition::default(),
        };
        self.live_sim.update_param(LiveSimParam::StopCondition(stop_cond));
    }

    pub fn reset_physics(&mut self) {
        self.live_sim.reset_simulation();
        self.sync_live_sim_params();

        // Pin the node closest to origin (0,0) when starting physics
        let n = self.state.node_index_to_id.len();
        if n > 0 {
            let closest_idx = (0..n).min_by(|&a, &b| {
                let pos_a = self.state.positions.get(a);
                let pos_b = self.state.positions.get(b);
                let dist_a = pos_a.x * pos_a.x + pos_a.y * pos_a.y;
                let dist_b = pos_b.x * pos_b.x + pos_b.y * pos_b.y;
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            });
            self.live_sim.update_param(LiveSimParam::FixedNode(closest_idx));
        }
    }

    pub fn run_physics_step(&mut self) {
        if self.state.node_index_to_id.is_empty() {
            return;
        }

        self.sync_live_sim_params();

        // Store drag node original position to restore after step if dragging
        let drag_info = self.interaction_state.drag_start.map(|(drag_id, _, _)| {
            let idx = *self.state.node_keys.get(drag_id).unwrap();
            (idx, *self.state.positions.get(idx))
        });

        self.live_sim.tick(&mut self.state);

        // Restore position of node currently being dragged
        if let Some((drag_idx, original_pos)) = drag_info {
            self.state.positions.set(drag_idx, original_pos);
        }
    }

    pub fn resolve_collisions(&mut self) {
        if self.state.node_index_to_id.is_empty() {
            return;
        }

        let drag_info = self.interaction_state.drag_start.map(|(drag_id, _, _)| {
            let idx = *self.state.node_keys.get(drag_id).unwrap();
            (idx, *self.state.positions.get(idx))
        });

        self.live_sim.resolve_collisions(&mut self.state, 12.0);

        if let Some((drag_idx, original_pos)) = drag_info {
            self.state.positions.set(drag_idx, original_pos);
        }
    }
}
