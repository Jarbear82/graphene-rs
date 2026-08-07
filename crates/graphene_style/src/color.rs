use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use crate::ColorValue;

/// 8-bit RGB color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// 8-bit RGBA color
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rgba {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }
}

impl Rgba {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02X}{:02X}{:02X}{:02X}", self.r, self.g, self.b, self.a)
    }

    /// Convert to RGB by dropping alpha (useful when you only need the color channels)
    pub fn to_rgb(&self) -> Rgb {
        Rgb::new(self.r, self.g, self.b)
    }
}

impl fmt::Display for Rgb {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Display for Rgba {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Deterministic hash -> RGB (alpha is not produced)
pub fn string_to_rgb(s: &str) -> Rgb {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    let h = hasher.finish();

    Rgb {
        r: ((h >> 16) & 0xFF) as u8,
        g: ((h >> 8) & 0xFF) as u8,
        b: (h & 0xFF) as u8,
    }
}

/// Deterministic hash -> RGBA (all four channels come from the hash)
pub fn string_to_rgba(s: &str) -> Rgba {
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    let h = hasher.finish();

    Rgba {
        r: ((h >> 24) & 0xFF) as u8,
        g: ((h >> 16) & 0xFF) as u8,
        b: ((h >> 8) & 0xFF) as u8,
        a: (h & 0xFF) as u8,
    }
}

/// The three candidate foreground colors
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Foreground {
    Black,
    Grey, // exact mid-grey #808080
    White,
}

impl Foreground {
    pub fn to_rgb(self) -> Rgb {
        match self {
            Foreground::Black => Rgb::new(0, 0, 0),
            Foreground::Grey => Rgb::new(128, 128, 128),
            Foreground::White => Rgb::new(255, 255, 255),
        }
    }

    pub fn to_hex(self) -> String {
        self.to_rgb().to_hex()
    }
}

impl fmt::Display for Foreground {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// WCAG relative luminance (0.0 - 1.0)
pub fn relative_luminance(c: &Rgb) -> f64 {
    fn channel(c: u8) -> f64 {
        let c = c as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }

    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

/// WCAG contrast ratio between two colors (>= 1.0)
pub fn contrast_ratio(a: &Rgb, b: &Rgb) -> f64 {
    let l1 = relative_luminance(a);
    let l2 = relative_luminance(b);
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

/// Returns the foreground color (Black / Grey / White) that has the highest
/// WCAG contrast against the given background.
pub fn best_foreground(bg: &Rgb) -> Foreground {
    let candidates = [
        Foreground::Black,
        Foreground::Grey,
        Foreground::White,
    ];

    candidates
        .into_iter()
        .max_by(|a, b| {
            let ca = contrast_ratio(bg, &a.to_rgb());
            let cb = contrast_ratio(bg, &b.to_rgb());
            ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap() // safe - array is non-empty
}

/// Convenience overload for RGBA backgrounds (alpha is ignored for contrast)
pub fn best_foreground_rgba(bg: &Rgba) -> Foreground {
    best_foreground(&bg.to_rgb())
}

/// Select label foreground color for a node based on its background fill color.
pub fn node_label_foreground(node_bg: &Rgb) -> Foreground {
    best_foreground(node_bg)
}

/// Select label foreground color for a node based on its RGBA fill color.
pub fn node_label_foreground_rgba(node_bg: &Rgba) -> Foreground {
    best_foreground_rgba(node_bg)
}

/// Select label foreground color for an edge based on the canvas background color.
/// Edge text labels should not be derived from the edge's color but rather the background of the canvas.
pub fn edge_label_foreground(canvas_bg: &Rgb) -> Foreground {
    best_foreground(canvas_bg)
}

/// Select label foreground color for an edge based on the RGBA canvas background color.
pub fn edge_label_foreground_rgba(canvas_bg: &Rgba) -> Foreground {
    best_foreground_rgba(canvas_bg)
}

/// Strategy for calculating text label foreground colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelContrastMode {
    /// Automatically pick highest WCAG contrast candidate (Black, Grey, White)
    WcagAuto,
    /// Fixed custom foreground color
    Fixed(Rgb),
}

/// Configuration for color generation and contrast calculation settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColorConfig {
    pub label_contrast_mode: LabelContrastMode,
    pub auto_node_colors: bool,
    pub auto_edge_colors: bool,
    pub canvas_background: Rgb,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            label_contrast_mode: LabelContrastMode::WcagAuto,
            auto_node_colors: false,
            auto_edge_colors: false,
            canvas_background: Rgb::new(30, 30, 30),
        }
    }
}

impl ColorConfig {
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolve the node label foreground color under current configuration.
    pub fn resolve_node_label_foreground(&self, node_bg: &Rgb) -> Rgb {
        match self.label_contrast_mode {
            LabelContrastMode::WcagAuto => node_label_foreground(node_bg).to_rgb(),
            LabelContrastMode::Fixed(c) => c,
        }
    }

    /// Resolve the edge label foreground color under current configuration.
    /// Uses canvas_background so that edge labels remain legible regardless of edge color.
    pub fn resolve_edge_label_foreground(&self) -> Rgb {
        match self.label_contrast_mode {
            LabelContrastMode::WcagAuto => edge_label_foreground(&self.canvas_background).to_rgb(),
            LabelContrastMode::Fixed(c) => c,
        }
    }
}

// ---------------------------------------------------------------------------
// ColorValue Conversions
// ---------------------------------------------------------------------------

impl From<Rgb> for ColorValue {
    fn from(c: Rgb) -> Self {
        ColorValue::Rgba(
            c.r as f32 / 255.0,
            c.g as f32 / 255.0,
            c.b as f32 / 255.0,
            1.0,
        )
    }
}

impl From<Rgba> for ColorValue {
    fn from(c: Rgba) -> Self {
        ColorValue::Rgba(
            c.r as f32 / 255.0,
            c.g as f32 / 255.0,
            c.b as f32 / 255.0,
            c.a as f32 / 255.0,
        )
    }
}

impl From<Foreground> for ColorValue {
    fn from(fg: Foreground) -> Self {
        fg.to_rgb().into()
    }
}

impl From<ColorValue> for Rgba {
    fn from(cv: ColorValue) -> Self {
        match cv {
            ColorValue::Rgba(r, g, b, a) => Rgba::new(
                (r.clamp(0.0, 1.0) * 255.0).round() as u8,
                (g.clamp(0.0, 1.0) * 255.0).round() as u8,
                (b.clamp(0.0, 1.0) * 255.0).round() as u8,
                (a.clamp(0.0, 1.0) * 255.0).round() as u8,
            ),
        }
    }
}

impl From<ColorValue> for Rgb {
    fn from(cv: ColorValue) -> Self {
        Rgba::from(cv).to_rgb()
    }
}

// ---------------------------------------------------------------------------
// Unit Tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_enough() {
        let a = string_to_rgb("hello");
        let b = string_to_rgb("world");
        assert_ne!(a, b);

        let a_rgba = string_to_rgba("hello");
        let b_rgba = string_to_rgba("world");
        assert_ne!(a_rgba, b_rgba);
    }

    #[test]
    fn contrast_picks_sensible() {
        // Pure black background -> white should win
        assert_eq!(best_foreground(&Rgb::new(0, 0, 0)), Foreground::White);

        // Pure white background -> black should win
        assert_eq!(best_foreground(&Rgb::new(255, 255, 255)), Foreground::Black);

        // Mid-grey background -> black or white both have the same ratio;
        // the implementation will pick one deterministically
        let mid = Rgb::new(128, 128, 128);
        let fg = best_foreground(&mid);
        assert!(matches!(fg, Foreground::Black | Foreground::White));
    }

    #[test]
    fn edge_and_node_contrast_helpers() {
        let node_bg = Rgb::new(20, 20, 20);
        assert_eq!(node_label_foreground(&node_bg), Foreground::White);

        let canvas_bg = Rgb::new(240, 240, 240);
        assert_eq!(edge_label_foreground(&canvas_bg), Foreground::Black);
    }

    #[test]
    fn color_config_resolution() {
        let config = ColorConfig {
            label_contrast_mode: LabelContrastMode::WcagAuto,
            auto_node_colors: true,
            auto_edge_colors: true,
            canvas_background: Rgb::new(255, 255, 255),
        };

        let dark_node_bg = Rgb::new(10, 10, 10);
        assert_eq!(config.resolve_node_label_foreground(&dark_node_bg), Foreground::White.to_rgb());
        assert_eq!(config.resolve_edge_label_foreground(), Foreground::Black.to_rgb());

        let fixed_config = ColorConfig {
            label_contrast_mode: LabelContrastMode::Fixed(Rgb::new(255, 0, 0)),
            ..Default::default()
        };
        assert_eq!(fixed_config.resolve_node_label_foreground(&dark_node_bg), Rgb::new(255, 0, 0));
        assert_eq!(fixed_config.resolve_edge_label_foreground(), Rgb::new(255, 0, 0));
    }

    #[test]
    fn color_value_conversions_roundtrip() {
        let rgb = Rgb::new(100, 150, 200);
        let cv: ColorValue = rgb.into();
        let back_rgb: Rgb = cv.into();
        assert_eq!(rgb, back_rgb);

        let rgba = Rgba::new(100, 150, 200, 255);
        let cv_rgba: ColorValue = rgba.into();
        let back_rgba: Rgba = cv_rgba.into();
        assert_eq!(rgba, back_rgba);
    }
}
