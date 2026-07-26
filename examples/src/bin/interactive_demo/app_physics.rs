use crate::app::DemoApp;
use graphene_layout::LiveForceSimulation;

impl DemoApp {
    pub fn run_physics_step(&mut self) {
        if self.state.node_index_to_id.is_empty() {
            return;
        }

        // Store drag node original position to restore after step if dragging
        let drag_info = self.interaction_state.drag_start.map(|(drag_id, _, _)| {
            let idx = *self.state.node_keys.get(drag_id).unwrap();
            (idx, *self.state.positions.get(idx))
        });

        let mut sim = LiveForceSimulation::new();
        sim.temperature = self.physics_temperature;
        sim.tick(&mut self.state);

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

        let sim = LiveForceSimulation::new();
        sim.resolve_collisions(&mut self.state, 12.0);

        if let Some((drag_idx, original_pos)) = drag_info {
            self.state.positions.set(drag_idx, original_pos);
        }
    }
}
