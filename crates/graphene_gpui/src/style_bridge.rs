use graphene_style::{ColorValue, EdgeStyle, NodeStyle};

pub fn color_value_to_hsla(color: ColorValue) -> gpui::Hsla {
    let rgba = match color {
        ColorValue::Rgba(r, g, b, a) => gpui::rgba(
            ((r * 255.0) as u32) << 24
                | ((g * 255.0) as u32) << 16
                | ((b * 255.0) as u32) << 8
                | (a * 255.0) as u32,
        ),
    };
    rgba.into()
}

pub fn color_value_to_rgba(color: ColorValue) -> gpui::Rgba {
    match color {
        ColorValue::Rgba(r, g, b, a) => gpui::rgba(
            ((r * 255.0) as u32) << 24
                | ((g * 255.0) as u32) << 16
                | ((b * 255.0) as u32) << 8
                | (a * 255.0) as u32,
        ),
    }
}

pub use color_value_to_rgba as color_to_gpui;
pub use color_value_to_rgba as color_value_to_gpui_color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiTheme {
    pub bg: gpui::Rgba,
    pub panel_bg: gpui::Rgba,
    pub border: gpui::Rgba,
    pub accent: gpui::Rgba,
    pub text: gpui::Rgba,
    pub text_dim: gpui::Rgba,
    pub node_fill: gpui::Rgba,
    pub node_border: gpui::Rgba,
    pub edge_color: gpui::Rgba,
}

pub type GpuiTheme = UiTheme;

impl UiTheme {
    pub fn from_style(theme: &graphene_style::Theme) -> Self {
        Self {
            bg: color_value_to_rgba(theme.bg),
            panel_bg: color_value_to_rgba(theme.panel_bg),
            border: color_value_to_rgba(theme.border),
            accent: color_value_to_rgba(theme.accent),
            text: color_value_to_rgba(theme.text),
            text_dim: color_value_to_rgba(theme.text_dim),
            node_fill: color_value_to_rgba(theme.node_fill),
            node_border: color_value_to_rgba(theme.node_border),
            edge_color: color_value_to_rgba(theme.edge_color),
        }
    }
}

impl From<&graphene_style::Theme> for UiTheme {
    fn from(theme: &graphene_style::Theme) -> Self {
        Self::from_style(theme)
    }
}

#[derive(Debug, Clone)]
pub struct StyleBridgeAdapter {
    pub default_node_style: NodeStyle,
    pub default_edge_style: EdgeStyle,
}

impl Default for StyleBridgeAdapter {
    fn default() -> Self {
        Self {
            default_node_style: NodeStyle::default(),
            default_edge_style: EdgeStyle::default(),
        }
    }
}

impl StyleBridgeAdapter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn node_fill_hsla(&self, style: &NodeStyle) -> gpui::Hsla {
        color_value_to_hsla(style.fill_color)
    }

    pub fn node_border_hsla(&self, style: &NodeStyle) -> gpui::Hsla {
        color_value_to_hsla(style.border_color)
    }

    pub fn edge_line_hsla(&self, style: &EdgeStyle) -> gpui::Hsla {
        color_value_to_hsla(style.line_color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_style::Theme;

    #[test]
    fn test_color_value_to_hsla_roundtrip() {
        let color = ColorValue::Rgba(1.0, 0.0, 0.0, 1.0);
        let rgba = color_value_to_rgba(color);
        assert_eq!(rgba, gpui::rgba(0xff0000ff));
        let hsla = color_value_to_hsla(color);
        assert_eq!(hsla, gpui::Hsla::from(rgba));
    }

    #[test]
    fn test_style_bridge_adapter_defaults() {
        let adapter = StyleBridgeAdapter::new();
        let fill_hsla = adapter.node_fill_hsla(&adapter.default_node_style);
        let border_hsla = adapter.node_border_hsla(&adapter.default_node_style);
        assert_ne!(fill_hsla, border_hsla);
    }

    #[test]
    fn test_ui_theme_from_style() {
        let theme = Theme::one_dark();
        let ui_theme = UiTheme::from_style(&theme);
        assert_eq!(ui_theme.bg, color_value_to_rgba(theme.bg));
        assert_eq!(ui_theme.panel_bg, color_value_to_rgba(theme.panel_bg));
        assert_eq!(ui_theme.accent, color_value_to_rgba(theme.accent));
    }
}

