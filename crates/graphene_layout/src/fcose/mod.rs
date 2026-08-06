pub mod solver;
pub mod types;

pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{math::Vec2, GraphState};

    #[test]
    fn test_fcose_layout_builder_configuration() {
        let mut dummy_state = GraphState::<()>::new();
        let n1 = dummy_state.add_node(Vec2::default(), graphene_core::Size2::default());
        let n2 = dummy_state.add_node(Vec2::default(), graphene_core::Size2::default());

        let layout = FCoseLayout::default()
            .with_iterations(300)
            .with_ideal_edge_length(75.0)
            .with_nesting_factor(1.5)
            .with_gravity(2.0)
            .with_node_repulsion(5000.0)
            .with_initial_temp(80.0)
            .with_cooling_factor(0.9)
            .with_randomize(false)
            .with_compound_padding(18.0)
            .with_gravity_range(400.0)
            .with_gravity_compound(1.2)
            .with_gravity_range_compound(1.8)
            .with_tile(false)
            .with_tiling_padding_horizontal(15.0)
            .with_tiling_padding_vertical(15.0)
            .with_pack_components(false)
            .with_node_dimensions_include_labels(true)
            .with_fixed_node_constraint(FixedNodeConstraint {
                node_id: n1,
                position: Vec2::new(10.0, 20.0),
            })
            .with_alignment_constraint(AlignmentConstraint {
                horizontal: vec![vec![n1, n2]],
                vertical: vec![],
            })
            .with_relative_placement_constraint(RelativePlacementConstraint::LeftRight {
                left: n1,
                right: n2,
                gap: 30.0,
            })
            .with_node_repulsion_metric(NodeRepulsionMetric::Constant(6000.0))
            .with_ideal_edge_length_metric(EdgeMetric::Constant(80.0))
            .with_edge_elasticity_metric(EdgeMetric::Constant(1.5));

        assert_eq!(layout.iterations, 300);
        assert_eq!(layout.ideal_edge_length, 75.0);
        assert_eq!(layout.nesting_factor, 1.5);
        assert_eq!(layout.gravity, 2.0);
        assert_eq!(layout.node_repulsion, 5000.0);
        assert_eq!(layout.initial_temp, 80.0);
        assert_eq!(layout.cooling_factor, 0.9);
        assert_eq!(layout.randomize, false);
        assert_eq!(layout.compound_padding, 18.0);
        assert_eq!(layout.gravity_range, 400.0);
        assert_eq!(layout.gravity_compound, 1.2);
        assert_eq!(layout.gravity_range_compound, 1.8);
        assert_eq!(layout.tile, false);
        assert_eq!(layout.tiling_padding_horizontal, 15.0);
        assert_eq!(layout.tiling_padding_vertical, 15.0);
        assert_eq!(layout.pack_components, false);
        assert_eq!(layout.node_dimensions_include_labels, true);
        assert_eq!(layout.constraints.fixed_nodes.len(), 1);
        assert_eq!(layout.constraints.alignment.horizontal.len(), 1);
        assert_eq!(layout.constraints.relative_placement.len(), 1);
        assert!(layout.node_repulsion_metric.is_some());
        assert!(layout.ideal_edge_length_metric.is_some());
        assert!(layout.edge_elasticity_metric.is_some());
    }
}
