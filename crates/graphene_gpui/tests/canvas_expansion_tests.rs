use graphene_core::{DataExpansionMode, NodeData, PropValue, Properties, Vec2};
use graphene_gpui::render::graph_canvas::hex_to_rgba;
use graphene_layout::compute_curve_midpoint;
use graphene_style::EdgeCurveStyle;

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

#[test]
fn test_inline_edge_cutout_calculation() {
    let src = (0.0f32, 0.0f32);
    let tgt = (100.0f32, 0.0f32);
    let label = "ACTED_IN";
    let zoom = 1.0f32;

    let dx = tgt.0 - src.0;
    let dy = tgt.1 - src.1;
    let len = (dx * dx + dy * dy).sqrt();
    let ux = dx / len;
    let uy = dy / len;

    let text_gap = label.len() as f32 * 6.5 + 16.0 * zoom;
    let half_gap = (text_gap / 2.0).min(len * 0.4);

    let mid_x = (src.0 + tgt.0) / 2.0;
    let mid_y = (src.1 + tgt.1) / 2.0;

    let cut1_x = mid_x - ux * half_gap;
    let cut1_y = mid_y - uy * half_gap;
    let cut2_x = mid_x + ux * half_gap;
    let cut2_y = mid_y + uy * half_gap;

    assert!(cut1_x < mid_x);
    assert!(cut2_x > mid_x);
    assert_eq!(cut1_y, 0.0);
    assert_eq!(cut2_y, 0.0);
}

#[test]
fn test_straight_edge_zero_perpendicular_curvature() {
    let src = Vec2::new(0.0, 0.0);
    let tgt = Vec2::new(100.0, 0.0);
    let mid = compute_curve_midpoint(src, tgt, EdgeCurveStyle::Straight, 0.0);

    assert_eq!(mid.x, 50.0);
    assert_eq!(mid.y, 0.0);
}
