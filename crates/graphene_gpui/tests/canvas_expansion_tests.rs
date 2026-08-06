use graphene_core::{DataExpansionMode, NodeData, PropValue, Properties};
use graphene_gpui::render::graph_canvas::hex_to_rgba;

#[test]
fn test_hex_to_rgba_parsing() {
    let color1 = hex_to_rgba("#8b0000").expect("Valid red hex");
    assert_eq!(color1.r, 139.0 / 255.0);

    let color2 = hex_to_rgba("#2e8b57").expect("Valid green hex");
    assert_eq!(color2.g, 139.0 / 255.0);

    let color3 = hex_to_rgba("#4682b4").expect("Valid blue hex");
    assert_eq!(color3.b, 180.0 / 255.0);

    assert!(hex_to_rgba("invalid").is_none());
}

#[test]
fn test_dynamic_expansion_modes() {
    let mut props = Properties::new();
    props.insert("@display".into(), PropValue::Text("Aragorn".into()));
    props.insert("level".into(), PropValue::Int(50));
    props.insert("class".into(), PropValue::Text("Ranger".into()));
    props.insert("faction".into(), PropValue::Text("Fellowship".into()));

    let mut node_data = NodeData::new(vec!["Character", "Hero"], props);
    assert_eq!(node_data.expansion_mode, DataExpansionMode::Compact);

    node_data.expansion_mode = DataExpansionMode::Preview;
    assert_eq!(node_data.expansion_mode, DataExpansionMode::Preview);

    node_data.expansion_mode = DataExpansionMode::Full;
    assert_eq!(node_data.expansion_mode, DataExpansionMode::Full);
}
