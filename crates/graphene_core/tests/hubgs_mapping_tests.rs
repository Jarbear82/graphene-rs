use graphene_core::{
    EdgeData, EdgeDirection, GraphState, NodeData, PropValue, Properties, Size2, Vec2,
};

#[test]
fn test_hubgs_schema_and_instance_mapping() {
    let mut state = GraphState::<()>::new();

    // Create HubGS instance "hero_123" of type "Character"
    let mut hero_props = Properties::new();
    hero_props.insert("@display".into(), PropValue::Text("Aragorn".into()));
    hero_props.insert("@background".into(), PropValue::Text("#8b0000".into()));
    hero_props.insert("level".into(), PropValue::Int(50));

    let hero_data = NodeData::new(vec!["Character", "Hero"], hero_props);
    let hero_id = state.add_node_with_data(Vec2::new(10.0, 20.0), Size2::new(120.0, 45.0), hero_data);

    // Create HubGS instance "loc_456" of type "Location"
    let mut loc_props = Properties::new();
    loc_props.insert("@display".into(), PropValue::Text("Gondor".into()));
    let loc_data = NodeData::new(vec!["Location"], loc_props);
    let loc_id = state.add_node_with_data(Vec2::new(200.0, 20.0), Size2::new(120.0, 45.0), loc_data);

    // Create Role "resides_in" (outbound direction ->) with multiplicity (1)
    let mut role_data = EdgeData::with_label("resides_in", EdgeDirection::Directed);
    role_data.multiplicity = Some("(1)".into());
    let role_edge_id = state.add_edge_with_data(hero_id, loc_id, role_data);

    // Verify visualization mappings
    assert_eq!(state.display_label(hero_id), Some("Aragorn"));
    assert_eq!(state.display_label(loc_id), Some("Gondor"));
    assert_eq!(
        state.get_node_prop(hero_id, "@background"),
        Some(&PropValue::Text("#8b0000".into()))
    );

    let edge_data = state.edges.get(*state.edge_keys.get(role_edge_id).unwrap());
    assert_eq!(edge_data.direction, EdgeDirection::Directed);
    assert_eq!(edge_data.multiplicity.as_deref(), Some("(1)"));
}
