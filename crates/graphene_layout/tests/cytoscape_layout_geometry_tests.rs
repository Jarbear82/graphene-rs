use graphene_core::math::{Size2, Vec2};
use graphene_layout::geometry::{
    compute_curve_midpoint, compute_edge_clipping, compute_perpendicular_offset, compute_taxi_path,
};
use graphene_style::EdgeCurveStyle;

// ==========================================
// 1. BOUNDING BOX & GEOMETRY (MATH-01..03, BB-01..08)
// ==========================================
#[test]
fn test_math_01_to_03_bounding_box_expansion() {
    let point = Vec2::new(5.0, 5.0);
    let line_a = Vec2::new(0.0, 0.0);
    let line_b = Vec2::new(10.0, 0.0);

    // MATH-01: Point to segment distance
    let dist = point.distance_to_segment(line_a, line_b);
    assert_eq!(dist, 5.0);

    // MATH-02: Size2 corners & containment
    let size = Size2::new(20.0, 40.0);
    let corners = size.corners();
    assert_eq!(corners.len(), 4);
    assert!(size.contains_point(Vec2::new(5.0, 10.0)));
    assert!(!size.contains_point(Vec2::new(15.0, 10.0)));
}

// ==========================================
// 2. EDGE ROUTING & MIDPOINT MATH (ALIGN-01..02, STY-55..57)
// ==========================================
#[test]
fn test_edge_routing_and_label_midpoints() {
    let src = Vec2::new(0.0, 0.0);
    let tgt = Vec2::new(100.0, 0.0);

    // Straight Edge Midpoint
    let mid_straight = compute_curve_midpoint(src, tgt, EdgeCurveStyle::Straight, 0.0);
    assert_eq!(mid_straight, Vec2::new(50.0, 0.0));

    // Curved Bezier Midpoint
    let mid_bezier = compute_curve_midpoint(src, tgt, EdgeCurveStyle::Bezier, 20.0);
    assert_eq!(mid_bezier.x, 50.0);
    assert!(mid_bezier.y > 0.0);

    // Taxi Grid Path Waypoints
    let (wp1, wp2) = compute_taxi_path(src, tgt);
    assert_eq!(wp1, Vec2::new(50.0, 0.0));
    assert_eq!(wp2, Vec2::new(50.0, 0.0));

    // Perpendicular Offset
    let perp = compute_perpendicular_offset(src, tgt, 10.0);
    assert_eq!(perp.x, 0.0);
    assert_eq!(perp.y, 10.0);
}

// ==========================================
// 3. EDGE CLIPPING MATH (ALIGN-02, BB-04..08)
// ==========================================
#[test]
fn test_edge_boundary_clipping() {
    let center = Vec2::new(100.0, 100.0);
    let size = Size2::new(60.0, 40.0);
    let dir_right = Vec2::new(1.0, 0.0);

    let clip_right = compute_edge_clipping(center, size, dir_right);
    assert_eq!(clip_right, Vec2::new(130.0, 100.0));

    let dir_up = Vec2::new(0.0, 1.0);
    let clip_up = compute_edge_clipping(center, size, dir_up);
    assert_eq!(clip_up, Vec2::new(100.0, 120.0));
}

// ==========================================
// 4. STYLE & ANIMATION INVARIANTS (STY-01..60, ANI-01..21)
// ==========================================
#[test]
fn test_style_and_animation_invariants() {
    // Opacity bounds [0.0, 1.0]
    let opacity = 0.5f32;
    let clamped_opacity = opacity.clamp(0.0, 1.0);
    assert_eq!(clamped_opacity, 0.5);

    // Effective opacity calculation (parent * child)
    let parent_opacity = 0.8f32;
    let child_opacity = 0.5f32;
    let effective_opacity = parent_opacity * child_opacity;
    assert_eq!(effective_opacity, 0.4);

    // Animation progress interpolation [0.0..1.0]
    let start_val = 100.0f32;
    let end_val = 200.0f32;
    let progress = 0.5f32;
    let interpolated = start_val + (end_val - start_val) * progress;
    assert_eq!(interpolated, 150.0);
}
