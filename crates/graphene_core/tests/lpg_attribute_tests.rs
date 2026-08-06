use graphene_core::{
    CompactString, EdgeData, EdgeDirection, GraphState, NodeData, PropValue, Properties, Size2,
    Vec2,
};

#[test]
fn test_lpg_node_data_and_properties() {
    let mut state = GraphState::<()>::new();

    let mut props = Properties::new();
    props.insert("name".into(), PropValue::Text("Alice".into()));
    props.insert("age".into(), PropValue::Int(30));
    props.insert("active".into(), PropValue::Bool(true));

    let node_data = NodeData::new(vec!["Person", "User"], props);
    let n1 = state.add_node_with_data(Vec2::new(0.0, 0.0), Size2::new(100.0, 40.0), node_data);

    assert_eq!(state.display_label(n1), Some("Alice"));
    assert_eq!(state.get_node_prop(n1, "age"), Some(&PropValue::Int(30)));
    assert_eq!(state.get_node_prop(n1, "active"), Some(&PropValue::Bool(true)));

    let labels = state.node_labels(n1).unwrap();
    assert_eq!(labels.len(), 2);
    assert_eq!(labels[0].as_str(), "Person");
    assert_eq!(labels[1].as_str(), "User");

    state.set_node_prop(n1, "city", PropValue::Text("New York".into()));
    assert_eq!(
        state.get_node_prop(n1, "city"),
        Some(&PropValue::Text("New York".into()))
    );
}

#[test]
fn test_lpg_edge_data_directions_and_adjacency() {
    let mut state = GraphState::<()>::new();

    let n1 = state.add_node_with_data(
        Vec2::new(0.0, 0.0),
        Size2::new(50.0, 50.0),
        NodeData::with_label("Person"),
    );
    let n2 = state.add_node_with_data(
        Vec2::new(100.0, 0.0),
        Size2::new(50.0, 50.0),
        NodeData::with_label("Person"),
    );
    let n3 = state.add_node_with_data(
        Vec2::new(50.0, 100.0),
        Size2::new(50.0, 50.0),
        NodeData::with_label("Person"),
    );

    let e1 = state.add_edge_with_data(
        n1,
        n2,
        EdgeData::with_label("KNOWS", EdgeDirection::Directed),
    );
    let e2 = state.add_edge_with_data(
        n3,
        n1,
        EdgeData::with_label("FOLLOWS", EdgeDirection::Reverse),
    );
    let e3 = state.add_edge_with_data(
        n2,
        n3,
        EdgeData::with_label("MUTUAL", EdgeDirection::Bidirectional),
    );

    let out_n1 = state.outgoing(n1);
    assert_eq!(out_n1.len(), 2); // e1 (outgoing to n2) + e2 (reverse from n3 to n1 means n1 is outgoing)
    assert!(out_n1.contains(&(e1, n2)));
    assert!(out_n1.contains(&(e2, n3)));

    let inc_n1 = state.incoming(n1);
    assert_eq!(inc_n1.len(), 0);

    let out_n2 = state.outgoing(n2);
    assert!(out_n2.contains(&(e3, n3)));

    let inc_n3 = state.incoming(n3);
    assert!(inc_n3.contains(&(e3, n2)));
}
