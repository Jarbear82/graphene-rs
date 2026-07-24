use gpui::{Pixels, Point};
use graphene_style::ColorValue;

pub const LAYOUT_NAMES: &[&str] = &[
    "Circle",
    "ForceDirected",
    "CoSE",
    "KamadaKawai",
    "Sugiyama",
    "ReingoldTilford",
    "MDS",
    "Grid",
    "Concentric",
    "Bipartite",
    "WeightedForce",
    "CollisionForce",
    "DisconnectedPack",
    "Compound",
    "RegionalPartition",
    "fCoSE",
];

pub fn distance_to_segment(p: Point<Pixels>, a: Point<Pixels>, b: Point<Pixels>) -> f32 {
    let px_val = f32::from(p.x);
    let py_val = f32::from(p.y);
    let ax = f32::from(a.x);
    let ay = f32::from(a.y);
    let bx = f32::from(b.x);
    let by = f32::from(b.y);

    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq == 0.0 {
        let rx = px_val - ax;
        let ry = py_val - ay;
        return (rx * rx + ry * ry).sqrt();
    }

    let t = ((px_val - ax) * dx + (py_val - ay) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);

    let proj_x = ax + t * dx;
    let proj_y = ay + t * dy;

    let rx = px_val - proj_x;
    let ry = py_val - proj_y;
    (rx * rx + ry * ry).sqrt()
}

pub fn color_value_to_gpui_color(color_val: ColorValue) -> gpui::Rgba {
    match color_val {
        ColorValue::Rgba(r, g, b, a) => gpui::rgba(
            ((r * 255.0) as u32) << 24
                | ((g * 255.0) as u32) << 16
                | ((b * 255.0) as u32) << 8
                | (a * 255.0) as u32,
        ),
    }
}

pub struct Theme {
    pub bg: gpui::Rgba,
    pub panel_bg: gpui::Rgba,
    pub border: gpui::Rgba,
    pub accent: gpui::Rgba,
    pub text: gpui::Rgba,
    pub text_dim: gpui::Rgba,
}

impl Theme {
    pub fn from_style(theme: &graphene_style::Theme) -> Self {
        Self {
            bg: color_value_to_gpui_color(theme.bg),
            panel_bg: color_value_to_gpui_color(theme.panel_bg),
            border: color_value_to_gpui_color(theme.border),
            accent: color_value_to_gpui_color(theme.accent),
            text: color_value_to_gpui_color(theme.text),
            text_dim: color_value_to_gpui_color(theme.text_dim),
        }
    }
}
