use crate::centrality::{compute_all_centrality_with_config, CentralityConfig, CentralityScores};
use crate::connectivity::{find_articulation_points, find_bridges, get_components_summary};
use crate::metrics::{
    compute_average_degree, compute_clustering_coefficient, compute_density, compute_reciprocity,
};
use crate::spectrum::algebraic_connectivity;
use graphene_core::{GraphState, NodeId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AnalysisConfig {
    pub top_k_rankings: usize,
    pub centrality: CentralityConfig,
}

impl Default for AnalysisConfig {
    fn default() -> Self {
        Self {
            top_k_rankings: 5,
            centrality: CentralityConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphAnalysisReport {
    pub is_directed: bool,
    pub node_count: usize,
    pub edge_count: usize,
    pub density: f32,
    pub average_degree: f32,
    pub reciprocity: f32,
    pub clustering_coefficient: f32,
    pub algebraic_connectivity: f64,
    pub connected_components_count: usize,
    pub strongly_connected_components_count: usize,
    pub articulation_point_count: usize,
    pub bridge_count: usize,
    pub top_pagerank: Vec<(NodeId, f32)>,
    pub top_betweenness: Vec<(NodeId, f32)>,
    pub top_degree: Vec<(NodeId, f32)>,
    pub centralities: CentralityScores,
}

impl GraphAnalysisReport {
    pub fn analyze<S: Copy + Default>(state: &GraphState<S>, directed: bool) -> Self {
        Self::analyze_with_config(state, directed, AnalysisConfig::default())
    }

    pub fn analyze_with_config<S: Copy + Default>(
        state: &GraphState<S>,
        directed: bool,
        config: AnalysisConfig,
    ) -> Self {
        let node_count = state.node_index_to_id.len();
        let edge_count = state.edges.len();

        let density = compute_density(state, directed);
        let average_degree = compute_average_degree(state);
        let reciprocity = if directed {
            compute_reciprocity(state)
        } else {
            1.0
        };
        let clustering_coefficient = compute_clustering_coefficient(state);
        let alg_conn = algebraic_connectivity(state);

        let (wcc, scc) = get_components_summary(state);
        let ap = find_articulation_points(state);
        let bridges = find_bridges(state);

        let centralities = compute_all_centrality_with_config(state, directed, config.centrality);

        let top_k = config.top_k_rankings;

        let mut pr_sorted: Vec<(NodeId, f32)> = centralities
            .page_rank
            .iter()
            .map(|(&k, &v)| (k, v))
            .collect();
        pr_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        pr_sorted.truncate(top_k);

        let mut bet_sorted: Vec<(NodeId, f32)> = centralities
            .betweenness
            .iter()
            .map(|(&k, &v)| (k, v))
            .collect();
        bet_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        bet_sorted.truncate(top_k);

        let mut deg_sorted: Vec<(NodeId, f32)> = centralities
            .degree
            .iter()
            .map(|(&k, &v)| (k, v))
            .collect();
        deg_sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        deg_sorted.truncate(top_k);

        Self {
            is_directed: directed,
            node_count,
            edge_count,
            density,
            average_degree,
            reciprocity,
            clustering_coefficient,
            algebraic_connectivity: alg_conn,
            connected_components_count: wcc.len(),
            strongly_connected_components_count: scc.len(),
            articulation_point_count: ap.len(),
            bridge_count: bridges.len(),
            top_pagerank: pr_sorted,
            top_betweenness: bet_sorted,
            top_degree: deg_sorted,
            centralities,
        }
    }
}
