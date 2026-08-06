use graphene_core::{math::Vec2, EdgeId, NodeId};

#[derive(Clone, Debug)]
pub struct FixedNodeConstraint {
    pub node_id: NodeId,
    pub position: Vec2,
}

#[derive(Clone, Debug, Default)]
pub struct AlignmentConstraint {
    pub horizontal: Vec<Vec<NodeId>>,
    pub vertical: Vec<Vec<NodeId>>,
}

#[derive(Clone, Debug)]
pub enum RelativePlacementConstraint {
    LeftRight {
        left: NodeId,
        right: NodeId,
        gap: f32,
    },
    TopBottom {
        top: NodeId,
        bottom: NodeId,
        gap: f32,
    },
}

#[derive(Clone, Debug, Default)]
pub struct FCoseConstraints {
    pub fixed_nodes: Vec<FixedNodeConstraint>,
    pub alignment: AlignmentConstraint,
    pub relative_placement: Vec<RelativePlacementConstraint>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FCosePhase {
    DraftLayout,
    ComponentPacking,
    ConstraintSatisfaction,
    LayoutPolishing,
}

impl std::fmt::Display for FCosePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FCosePhase::DraftLayout => write!(f, "Phase I: Draft Layout Generation (Spectral)"),
            FCosePhase::ComponentPacking => write!(f, "Phase II: Component Packing"),
            FCosePhase::ConstraintSatisfaction => write!(f, "Phase III: Constraint Satisfaction"),
            FCosePhase::LayoutPolishing => write!(f, "Phase IV: Layout Polishing (Spring Embedder)"),
        }
    }
}

pub static FCOSE_PHASES: [FCosePhase; 4] = [
    FCosePhase::DraftLayout,
    FCosePhase::ComponentPacking,
    FCosePhase::ConstraintSatisfaction,
    FCosePhase::LayoutPolishing,
];

/// fCoSE fast compound graph layout algorithm.
///
/// Reference: Balci, H., & Dogrusoz, U. (2021). "fCoSE: A fast compound graph layout algorithm."
/// IEEE Transactions on Visualization and Computer Graphics, 28(12), 4282–4293.
pub struct FCoseLayout {
    pub iterations: usize,
    pub ideal_edge_length: f32,
    pub nesting_factor: f32,
    pub gravity: f32,
    pub node_repulsion: f32,
    pub initial_temp: f32,
    pub cooling_factor: f32,
    pub randomize: bool,
    pub compound_padding: f32,
    pub gravity_range: f32,
    pub gravity_compound: f32,
    pub gravity_range_compound: f32,
    pub tile: bool,
    pub tiling_padding_horizontal: f32,
    pub tiling_padding_vertical: f32,
    pub pack_components: bool,
    pub node_dimensions_include_labels: bool,
    pub current_phase_idx: usize,

    pub constraints: FCoseConstraints,

    pub node_repulsion_metric: Option<NodeRepulsionMetric>,
    pub ideal_edge_length_metric: Option<EdgeMetric>,
    pub edge_elasticity_metric: Option<EdgeMetric>,
}

/// Enum dispatch provider for node repulsion metrics in fCoSE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeRepulsionMetric {
    Constant(f32),
    NodePinned { target_id: NodeId, pinned_val: f32, default_val: f32 },
}

impl NodeRepulsionMetric {
    #[inline(always)]
    pub fn evaluate(&self, id: NodeId) -> f32 {
        match self {
            NodeRepulsionMetric::Constant(val) => *val,
            NodeRepulsionMetric::NodePinned { target_id, pinned_val, default_val } => {
                if id == *target_id { *pinned_val } else { *default_val }
            }
        }
    }
}

/// Enum dispatch provider for edge metrics in fCoSE.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeMetric {
    Constant(f32),
    Scaled { base: f32, scale: f32 },
}

impl EdgeMetric {
    #[inline(always)]
    pub fn evaluate(&self, _id: EdgeId) -> f32 {
        match self {
            EdgeMetric::Constant(val) => *val,
            EdgeMetric::Scaled { base, scale } => base * scale,
        }
    }
}

impl Default for FCoseLayout {
    fn default() -> Self {
        Self {
            iterations: 150,
            ideal_edge_length: 50.0,
            nesting_factor: 1.2,
            gravity: 1.5,
            node_repulsion: 4500.0,
            initial_temp: 50.0,
            cooling_factor: 0.95,
            randomize: true,
            compound_padding: 12.0,
            gravity_range: 380.0,
            gravity_compound: 1.0,
            gravity_range_compound: 1.5,
            tile: true,
            tiling_padding_horizontal: 10.0,
            tiling_padding_vertical: 10.0,
            pack_components: true,
            node_dimensions_include_labels: false,
            current_phase_idx: 0,
            constraints: FCoseConstraints::default(),
            node_repulsion_metric: None,
            ideal_edge_length_metric: None,
            edge_elasticity_metric: None,
        }
    }
}
