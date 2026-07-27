pub use graphene_fixtures::demos::*;
use graphene_fixtures::GraphFixture;
use graphene_style::ComputedStyle;

/// Load all converted Cytoscape Rust demo fixtures for the interactive application.
pub fn load_demo_fixtures() -> Vec<GraphFixture<ComputedStyle>> {
    let mut fixtures = Vec::new();
    add_cytoscape_demos(&mut fixtures);
    fixtures
}
