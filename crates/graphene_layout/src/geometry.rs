use graphene_core::math::{Size2, Vec2};
use graphene_style::EdgeCurveStyle;

/// Compute the midpoint of a curve for edge label positioning.
pub fn compute_curve_midpoint(
    source: Vec2,
    target: Vec2,
    style: EdgeCurveStyle,
    curvature: f32,
) -> Vec2 {
    match style {
        EdgeCurveStyle::Straight | EdgeCurveStyle::Taxi => (source + target) * 0.5,
        EdgeCurveStyle::UnbundledBezier(cp1, cp2) => {
            // Evaluated cubic curve midpoint at t=0.5
            source * 0.125 + cp1 * 0.375 + cp2 * 0.375 + target * 0.125
        }
        EdgeCurveStyle::Bezier | EdgeCurveStyle::Segmented => {
            let mid = (source + target) * 0.5;
            let dx = target.x - source.x;
            let dy = target.y - source.y;
            let len = (dx * dx + dy * dy).sqrt();

            let ctrl = if len > 0.0 {
                Vec2::new(mid.x - (dy / len) * curvature, mid.y + (dx / len) * curvature)
            } else {
                mid
            };

            // Quadratic bezier evaluated at t=0.5: 0.25 P0 + 0.5 P1 + 0.25 P2
            source * 0.25 + ctrl * 0.5 + target * 0.25
        }
    }
}

/// Compute the intersection/clipping point where an edge vector intersects a rectangular node boundary.
pub fn compute_edge_clipping(center: Vec2, size: Size2, direction: Vec2) -> Vec2 {
    let w = size.w;
    let h = size.h;

    let dx = direction.x;
    let dy = direction.y;

    if dx == 0.0 && dy == 0.0 {
        return center;
    }

    if dx == 0.0 && dy > 0.0 {
        return Vec2::new(center.x, center.y + h / 2.0);
    }
    if dx == 0.0 && dy < 0.0 {
        return Vec2::new(center.x, center.y - h / 2.0);
    }

    let dir_slope = dy / dx;
    let node_slope = if w > 0.0 { h / w } else { 0.0 };

    if dx > 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(center.x + w / 2.0, center.y + (w * dy / (2.0 * dx)));
    }
    if dx < 0.0 && dir_slope >= -node_slope && dir_slope <= node_slope {
        return Vec2::new(center.x - w / 2.0, center.y - (w * dy / (2.0 * dx)));
    }
    if dy > 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(center.x + (h * dx / (2.0 * dy)), center.y + h / 2.0);
    }
    if dy < 0.0 && (dir_slope <= -node_slope || dir_slope >= node_slope) {
        return Vec2::new(center.x - (h * dx / (2.0 * dy)), center.y - h / 2.0);
    }

    center
}

/// Compute waypoints for Manhattan/Taxi style grid routing between source and target points.
pub fn compute_taxi_path(source: Vec2, target: Vec2) -> (Vec2, Vec2) {
    let mid_x = (source.x + target.x) / 2.0;
    (Vec2::new(mid_x, source.y), Vec2::new(mid_x, target.y))
}

/// Compute perpendicular offset for bezier curve control point calculation.
pub fn compute_perpendicular_offset(source: Vec2, target: Vec2, magnitude: f32) -> Vec2 {
    let dx = target.x - source.x;
    let dy = target.y - source.y;
    let len = (dx * dx + dy * dy).sqrt();

    if len > 0.0 {
        Vec2::new(-dy / len, dx / len) * magnitude
    } else {
        Vec2::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_curve_midpoint_straight() {
        let src = Vec2::new(0.0, 0.0);
        let tgt = Vec2::new(10.0, 10.0);
        let mid = compute_curve_midpoint(src, tgt, EdgeCurveStyle::Straight, 0.0);
        assert_eq!(mid, Vec2::new(5.0, 5.0));
    }

    #[test]
    fn test_compute_taxi_path() {
        let src = Vec2::new(0.0, 0.0);
        let tgt = Vec2::new(10.0, 20.0);
        let (wp1, wp2) = compute_taxi_path(src, tgt);
        assert_eq!(wp1, Vec2::new(5.0, 0.0));
        assert_eq!(wp2, Vec2::new(5.0, 20.0));
    }

    #[test]
    fn test_compute_edge_clipping() {
        let center = Vec2::new(0.0, 0.0);
        let size = Size2::new(100.0, 50.0);
        let dir = Vec2::new(1.0, 0.0);
        let clip = compute_edge_clipping(center, size, dir);
        assert_eq!(clip, Vec2::new(50.0, 0.0));
    }
}
