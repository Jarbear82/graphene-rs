use graphene_fixtures::demos::add_cytoscape_demos;
use graphene_fixtures::GraphFixture;

#[test]
fn test_hubgs_and_lpg_scale_demos() {
    let mut fixtures = Vec::<GraphFixture<()>>::new();
    add_cytoscape_demos(&mut fixtures);

    // Verify fixture count (at least 12 demos present)
    assert!(fixtures.len() >= 12);

    let hubgs_small = fixtures.iter().find(|f| f.name.contains("HubGS RPG Knowledge Graph")).expect("HubGS Small demo present");
    assert_eq!(hubgs_small.state.node_count(), 4);

    let hubgs_med = fixtures.iter().find(|f| f.name.contains("HubGS Enterprise Schema (Medium")).expect("HubGS Medium demo present");
    assert_eq!(hubgs_med.state.node_count(), 30);

    let lpg_small = fixtures.iter().find(|f| f.name.contains("LPG Movie Network (Small")).expect("LPG Small demo present");
    assert_eq!(lpg_small.state.node_count(), 3);

    let lpg_large = fixtures.iter().find(|f| f.name.contains("LPG Cyber Threat Graph (Large")).expect("LPG Large demo present");
    assert_eq!(lpg_large.state.node_count(), 200);
}
