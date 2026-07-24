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
}
