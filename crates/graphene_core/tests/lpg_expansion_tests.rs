use graphene_core::{
    DataExpansionMode, EdgeData, EdgeDirection, GraphState, NodeData, PropValue, Properties, Size2, Vec2,
};

#[test]
fn test_node_expansion_mode_transitions() {
    let mut state = GraphState::<()>::new();

    let mut props = Properties::new();
    props.insert("@display".into(), PropValue::Text("Aragorn".into()));
    props.insert("@background".into(), PropValue::Text("#8b0000".into()));
    props.insert("level".into(), PropValue::Int(50));
    props.insert("class".into(), PropValue::Text("Ranger".into()));

    let id = state.add_node_with_data(
        Vec2::new(0.0, 0.0),
        Size2::new(100.0, 40.0),
        NodeData::new(vec!["Character", "Hero"], props),
    );

    let initial_size = *state.sizes.get(0);
    assert_eq!(state.nodes.get(0).expansion_mode, DataExpansionMode::Compact);
    assert_eq!(state.display_label(id), Some("Aragorn"));

    state.set_node_expansion_mode(id, DataExpansionMode::Preview);
    assert_eq!(state.nodes.get(0).expansion_mode, DataExpansionMode::Preview);
    let preview_size = *state.sizes.get(0);
    assert!(preview_size.w >= initial_size.w || preview_size.h >= initial_size.h);

    state.set_node_expansion_mode(id, DataExpansionMode::Full);
    assert_eq!(state.nodes.get(0).expansion_mode, DataExpansionMode::Full);
    let full_size = *state.sizes.get(0);
    assert!(full_size.w >= preview_size.w || full_size.h >= preview_size.h);
}

#[test]
fn test_edge_label_and_multiplicity_generation() {
    let mut state = GraphState::<()>::new();
    let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));
    let n2 = state.add_node(Vec2::new(100.0, 0.0), Size2::new(50.0, 50.0));

    let mut edge = EdgeData::with_label("resides_in", EdgeDirection::Directed);
    edge.multiplicity = Some("(1)".into());
    let e1 = state.add_edge_with_data(n1, n2, edge);

    let edge_data = &state.edges.get(0);
    assert_eq!(edge_data.primary_label(), Some("resides_in"));
    assert_eq!(edge_data.multiplicity.as_deref(), Some("(1)"));
    assert_eq!(edge_data.direction, EdgeDirection::Directed);

    let labels = state.edge_labels(e1).unwrap();
    assert_eq!(labels[0].as_str(), "resides_in");
}

#[test]
fn test_background_property_resolution() {
    let mut props = Properties::new();
    props.insert("@background".into(), PropValue::Text("#2e8b57".into()));
    let node_data = NodeData::new(vec!["Location"], props);

    let bg_prop = node_data.props.get("@background");
    assert!(bg_prop.is_some());
    if let Some(PropValue::Text(hex)) = bg_prop {
        assert_eq!(hex.as_str(), "#2e8b57");
    } else {
        panic!("Expected @background PropValue::Text");
    }
}
