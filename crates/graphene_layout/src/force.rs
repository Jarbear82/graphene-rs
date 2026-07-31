use crate::pipeline::{Integrator, LayoutPhase, LayoutPipeline, ObjectiveTerm};
use crate::traits::Layout;
use graphene_core::GraphState;

/// Standard spring-embedder force-directed graph layout.
///
/// Reference: Eades, P. (1984). "A heuristic for graph drawing."
/// Congressus Numerantium, 42, 149–160.
pub struct ForceDirectedLayout {
    pub iterations: usize,
    pub ideal_length: f32,
    pub gravity: f32,
    pub k_rep: f32,
    pub k_att: f32,
    pub initial_temp: f32,
    pub use_barnes_hut: bool,
    pub theta: f32,
}

impl Default for ForceDirectedLayout {
    fn default() -> Self {
        Self {
            iterations: 150,
            ideal_length: 50.0,
            gravity: 0.1,
            k_rep: 2000.0,
            k_att: 0.05,
            initial_temp: 10.0,
            use_barnes_hut: true,
            theta: 0.5,
        }
    }
}

impl ForceDirectedLayout {
    pub fn with_iterations(mut self, iterations: usize) -> Self {
        self.iterations = iterations;
        self
    }

    pub fn with_ideal_length(mut self, length: f32) -> Self {
        self.ideal_length = length;
        self
    }

    pub fn with_gravity(mut self, gravity: f32) -> Self {
        self.gravity = gravity;
        self
    }

    pub fn with_k_rep(mut self, k_rep: f32) -> Self {
        self.k_rep = k_rep;
        self
    }

    pub fn with_k_att(mut self, k_att: f32) -> Self {
        self.k_att = k_att;
        self
    }

    pub fn with_initial_temp(mut self, temp: f32) -> Self {
        self.initial_temp = temp;
        self
    }

    pub fn with_use_barnes_hut(mut self, use_bh: bool) -> Self {
        self.use_barnes_hut = use_bh;
        self
    }

    pub fn with_theta(mut self, theta: f32) -> Self {
        self.theta = theta;
        self
    }
}

impl<S: Copy + Sync> Layout<S> for ForceDirectedLayout {
    fn compute(&mut self, state: &mut GraphState<S>) {
        let pipeline = LayoutPipeline {
            phases: vec![LayoutPhase {
                name: "force_directed",
                max_iterations: self.iterations,
                terms: vec![
                    ObjectiveTerm::ElectrostaticRepulsion {
                        strength: self.k_rep,
                        use_barnes_hut: self.use_barnes_hut,
                        theta: self.theta,
                    },
                    ObjectiveTerm::HookeSpring {
                        ideal_length: self.ideal_length,
                        elasticity: self.k_att,
                    },
                    ObjectiveTerm::Gravity {
                        strength: self.gravity,
                    },
                ],
                integrator: Integrator::Euler {
                    max_displacement: self.initial_temp,
                    cooling_factor: 0.95,
                },
                resolve_overlaps_padding: Some(10.0),
            }],
        };
        pipeline.run(state);
    }
}
