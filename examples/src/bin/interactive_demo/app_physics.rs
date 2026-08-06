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

        let n = self.view.node_order.len();
        if n > 0 {
            let closest_idx = (0..n).min_by(|&a, &b| {
                let id_a = self.view.node_order[a];
                let id_b = self.view.node_order[b];
                let pos_a = self.view.nodes[&id_a].pos;
                let pos_b = self.view.nodes[&id_b].pos;
                let dist_a = pos_a.x * pos_a.x + pos_a.y * pos_a.y;
                let dist_b = pos_b.x * pos_b.x + pos_b.y * pos_b.y;
                dist_a.partial_cmp(&dist_b).unwrap_or(std::cmp::Ordering::Equal)
            });
            self.live_sim.update_param(LiveSimParam::FixedNode(closest_idx));
        }

        self.engine.send_command(graphene_layout::GraphCommand::StartLiveSim(self.live_sim.clone())).ok();
        self.telemetry_is_worker_thread = true;
    }

    pub fn run_physics_step(&mut self) {
        if self.view.node_order.is_empty() {
            return;
        }

        let drag_idx = self.interaction_state.drag_session.as_ref().and_then(|session| {
            self.view.node_order.iter().position(|&id| id == session.node_id)
        });

        self.engine.send_command(graphene_layout::GraphCommand::UpdateLiveSimParam(
            LiveSimParam::FixedNode(drag_idx)
        )).ok();
        self.telemetry_is_worker_thread = true;
    }

    pub fn resolve_collisions(&mut self) {
        // Epilogue and collision resolution managed inside GraphEngine loop
    }
}
