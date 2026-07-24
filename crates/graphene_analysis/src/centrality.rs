use graphene_algo::{
    betweenness_centrality, closeness_centrality_normalized, degree_centrality_normalized,
    page_rank,
};
use graphene_core::{GraphState, NodeId};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CentralityConfig {
    pub degree_exponent: f32,
    pub pagerank_damping: f32,
    pub pagerank_tolerance: f32,
    pub pagerank_max_iterations: usize,
}

impl Default for CentralityConfig {
    fn default() -> Self {
        Self {
            degree_exponent: 0.5,
            pagerank_damping: 0.85,
            pagerank_tolerance: 0.0001,
            pagerank_max_iterations: 100,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CentralityScores {
    pub degree: HashMap<NodeId, f32>,
    pub closeness: HashMap<NodeId, f32>,
    pub betweenness: HashMap<NodeId, f32>,
    pub page_rank: HashMap<NodeId, f32>,
}

pub fn compute_all_centrality<S: Copy>(
    state: &GraphState<S>,
    directed: bool,
) -> CentralityScores {
    compute_all_centrality_with_config(state, directed, CentralityConfig::default())
}

pub fn compute_all_centrality_with_config<S: Copy>(
    state: &GraphState<S>,
    directed: bool,
    config: CentralityConfig,
) -> CentralityScores {
    let deg_res = degree_centrality_normalized(state, directed, config.degree_exponent, |_| 1.0);
    let close_res = closeness_centrality_normalized(state, true, |_| 1.0);
    let bet_res = betweenness_centrality(state);
    let pr_res = page_rank(
        state,
        config.pagerank_damping,
        config.pagerank_tolerance,
        config.pagerank_max_iterations,
        |_| 1.0,
    );

    CentralityScores {
        degree: deg_res.degrees,
        closeness: close_res,
        betweenness: bet_res,
        page_rank: pr_res,
    }
}
