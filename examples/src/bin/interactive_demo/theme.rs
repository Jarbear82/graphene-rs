use gpui::{Pixels, Point};
use graphene_style::ColorValue;

pub const LAYOUT_NAMES: &[&str] = graphene_layout::LayoutCommand::ALL_NAMES;

pub fn distance_to_segment(p: Point<Pixels>, a: Point<Pixels>, b: Point<Pixels>) -> f32 {
    graphene_gpui::interaction::state::distance_to_segment(
        gpui::point(f32::from(p.x), f32::from(p.y)),
        gpui::point(f32::from(a.x), f32::from(a.y)),
        gpui::point(f32::from(b.x), f32::from(b.y)),
    )
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
