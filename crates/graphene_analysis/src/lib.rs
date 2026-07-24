pub mod centrality;
pub mod connectivity;
pub mod metrics;
pub mod report;
pub mod spectrum;

pub use centrality::{compute_all_centrality, compute_all_centrality_with_config, CentralityConfig, CentralityScores};
pub use connectivity::{find_articulation_points, find_bridges, get_components_summary};
pub use metrics::{
    compute_average_degree, compute_clustering_coefficient, compute_density, compute_reciprocity,
};
pub use report::{AnalysisConfig, GraphAnalysisReport};
pub use spectrum::algebraic_connectivity;

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{EdgeData, GraphState, Size2, Vec2};

    #[test]
    fn test_analysis_report() {
        let mut state = GraphState::<()>::new();
        let n0 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
        let n1 = state.add_node(Vec2::new(10.0, 0.0), Size2::new(10.0, 10.0));
        let n2 = state.add_node(Vec2::new(5.0, 10.0), Size2::new(10.0, 10.0));

        state.add_edge(n0, n1, EdgeData::default());
        state.add_edge(n1, n2, EdgeData::default());
        state.add_edge(n2, n0, EdgeData::default());

        let report = GraphAnalysisReport::analyze(&state, true);
        assert_eq!(report.node_count, 3);
        assert_eq!(report.edge_count, 3);
        assert!(report.density > 0.0);
        assert_eq!(report.connected_components_count, 1);
        assert_eq!(report.top_pagerank.len(), 3);
    }
}
