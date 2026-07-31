use crate::collision::resolve_overlaps;
use crate::quadtree::Quadtree;
use graphene_core::{math::Vec2, GraphState};

/// Composable force term — one physical effect applied per iteration.
pub enum ObjectiveTerm {
    HookeSpring {
        ideal_length: f32,
        elasticity: f32,
    },
    ElectrostaticRepulsion {
        strength: f32,
        use_barnes_hut: bool,
        theta: f32,
    },
    Gravity {
        strength: f32,
    },
}

pub enum Integrator {
    Euler {
        max_displacement: f32,
        cooling_factor: f32,
    },
}

/// A single named phase: N iterations of applying `terms` then integrating.
///
/// Reference: Declarative multi-phase physical force composition phase.
pub struct LayoutPhase {
    pub name: &'static str,
    pub max_iterations: usize,
    pub terms: Vec<ObjectiveTerm>,
    pub integrator: Integrator,
    pub resolve_overlaps_padding: Option<f32>,
}

/// Declarative layout pipeline.
///
/// Reference: Declarative multi-phase layout pipeline runner.
pub struct LayoutPipeline {
    pub phases: Vec<LayoutPhase>,
}

impl LayoutPipeline {
    pub fn run<S: Copy + Sync>(&self, state: &mut GraphState<S>) {
        for phase in &self.phases {
            self.run_phase(phase, state);
        }
    }

    fn run_phase<S: Copy + Sync>(&self, phase: &LayoutPhase, state: &mut GraphState<S>) {
        let n = state.node_index_to_id.len();
        if n == 0 {
            return;
        }

        let Integrator::Euler {
            max_displacement,
            cooling_factor,
        } = phase.integrator;
        let mut temp = max_displacement;

        for _iter in 0..phase.max_iterations {
            let mut disp = vec![Vec2::default(); n];

            for term in &phase.terms {
                match term {
                    ObjectiveTerm::HookeSpring {
                        ideal_length,
                        elasticity,
                    } => {
                        apply_spring_forces(state, &mut disp, *ideal_length, *elasticity);
                    }
                    ObjectiveTerm::ElectrostaticRepulsion {
                        strength,
                        use_barnes_hut,
                        theta,
                    } => {
                        if *use_barnes_hut && n > 100 {
                            let positions: Vec<Vec2> =
                                (0..n).map(|i| *state.positions.get(i)).collect();
                            let qt = Quadtree::build(&positions);
                            for i in 0..n {
                                disp[i] += qt.accumulate_repulsion(
                                    i,
                                    positions[i],
                                    &positions,
                                    *strength,
                                    *theta,
                                );
                            }
                        } else {
                            apply_direct_repulsion(state, &mut disp, *strength);
                        }
                    }
                    ObjectiveTerm::Gravity { strength } => {
                        apply_gravity(state, &mut disp, *strength);
                    }
                }
            }

            for i in 0..n {
                let len = disp[i].len();
                if len > 0.01 {
                    let capped = disp[i].normalize() * len.min(temp);
                    let old = *state.positions.get(i);
                    state.positions.set(i, old + capped);
                }
            }
            temp *= cooling_factor;
        }

        if let Some(padding) = phase.resolve_overlaps_padding {
            resolve_overlaps(state, padding);
        }
        state.dirty_flags |= graphene_core::DirtyFlags::POSITION_DIRTY;
    }
}

fn apply_spring_forces<S: Copy>(
    state: &GraphState<S>,
    disp: &mut [Vec2],
    ideal_length: f32,
    k: f32,
) {
    for i in 0..state.edges.len() {
        let (Some(&u), Some(&v)) = (
            state.node_keys.get(*state.edge_sources.get(i)),
            state.node_keys.get(*state.edge_targets.get(i)),
        ) else {
            continue;
        };
        if u == v {
            continue;
        }
        let (pu, pv) = (*state.positions.get(u), *state.positions.get(v));
        let delta = pv - pu;
        let dist = delta.len().max(0.01);
        let force = k * (dist - ideal_length);
        let dir = delta.normalize() * force;
        disp[u] += dir;
        disp[v] -= dir;
    }
}

#[cfg(feature = "parallel")]
fn apply_direct_repulsion_parallel<S: Copy + Sync>(
    state: &GraphState<S>,
    strength: f32,
) -> Vec<Vec2> {
    use rayon::prelude::*;

    let n = state.node_index_to_id.len();
    let positions: Vec<Vec2> = (0..n).map(|i| *state.positions.get(i)).collect();

    positions
        .par_iter()
        .enumerate()
        .map(|(i, &pi)| {
            let mut force = Vec2::default();
            for (j, &pj) in positions.iter().enumerate() {
                if i == j {
                    continue;
                }
                let delta = pi - pj;
                let dist = delta.len().max(0.01);
                force += delta.normalize() * (strength / (dist * dist));
            }
            force
        })
        .collect()
}

fn apply_direct_repulsion<S: Copy + Sync>(
    state: &GraphState<S>,
    disp: &mut [Vec2],
    strength: f32,
) {
    #[cfg(feature = "parallel")]
    {
        let parallel_disp = apply_direct_repulsion_parallel(state, strength);
        for (d, pd) in disp.iter_mut().zip(parallel_disp.into_iter()) {
            *d += pd;
        }
    }
    #[cfg(not(feature = "parallel"))]
    {
        let n = disp.len();
        for i in 0..n {
            let pi = *state.positions.get(i);
            for j in (i + 1)..n {
                let pj = *state.positions.get(j);
                let delta = pi - pj;
                let dist = delta.len().max(0.01);
                let force = strength / (dist * dist);
                let dir = delta.normalize() * force;
                disp[i] += dir;
                disp[j] -= dir;
            }
        }
    }
}

fn apply_gravity<S: Copy>(state: &GraphState<S>, disp: &mut [Vec2], strength: f32) {
    let n = disp.len();
    if n == 0 {
        return;
    }
    let mut center = Vec2::default();
    for i in 0..n {
        center += *state.positions.get(i);
    }
    center = center / n as f32;
    for i in 0..n {
        let pos = *state.positions.get(i);
        disp[i] += (center - pos) * strength;
    }
}
