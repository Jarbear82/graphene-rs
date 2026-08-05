use crate::view::GraphView;
use graphene_core::math::{Size2, Vec2};
use graphene_style::{ColorValue, ComputedStyle, EdgeCurveStyle, LabelId, NodeShape, StylingTarget};

#[derive(Debug, Clone)]
pub struct Viewport {
    pub offset: Vec2,
    pub zoom: f32,
    pub bounds: gpui::Bounds<f32>,
}

pub const MIN_ZOOM: f32 = 0.00001;
pub const MAX_ZOOM: f32 = 100000.0;

impl Viewport {
    pub fn new(bounds: gpui::Bounds<f32>) -> Self {
        Self {
            offset: Vec2::default(),
            zoom: 1.0,
            bounds,
        }
    }

    pub fn model_to_screen(&self, pos: Vec2) -> gpui::Point<f32> {
        let x = (pos.x + self.offset.x) * self.zoom + self.bounds.origin.x + self.bounds.size.width / 2.0;
        let y = (pos.y + self.offset.y) * self.zoom + self.bounds.origin.y + self.bounds.size.height / 2.0;
        gpui::point(x, y)
    }

    pub fn screen_to_model(&self, p: gpui::Point<f32>) -> Vec2 {
        let x = (p.x - self.bounds.origin.x - self.bounds.size.width / 2.0) / self.zoom - self.offset.x;
        let y = (p.y - self.bounds.origin.y - self.bounds.size.height / 2.0) / self.zoom - self.offset.y;
        Vec2::new(x, y)
    }

    pub fn is_visible(&self, pos: Vec2, size: Size2) -> bool {
        let screen_pos = self.model_to_screen(pos);
        let screen_size = gpui::size(size.w * self.zoom, size.h * self.zoom);

        let node_bounds = gpui::Bounds {
            origin: gpui::point(screen_pos.x - screen_size.width / 2.0, screen_pos.y - screen_size.height / 2.0),
            size: screen_size,
        };

        self.bounds.intersects(&node_bounds)
    }

    pub fn fit_to_graph<S: Copy + Send + 'static>(&mut self, view: &GraphView<S>) {
        if view.node_order.is_empty() {
            self.offset = Vec2::default();
            self.zoom = 1.0;
            return;
        }
        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        let mut y_min = f32::MAX;
        let mut y_max = f32::MIN;
        for node in view.nodes.values() {
            x_min = x_min.min(node.pos.x);
            x_max = x_max.max(node.pos.x);
            y_min = y_min.min(node.pos.y);
            y_max = y_max.max(node.pos.y);
        }
        let cx_graph = (x_min + x_max) / 2.0;
        let cy_graph = (y_min + y_max) / 2.0;

        self.offset = Vec2::new(-cx_graph, -cy_graph);

        let w_graph = x_max - x_min + 100.0;
        let h_graph = y_max - y_min + 100.0;
        let w_canvas = self.bounds.size.width;
        let h_canvas = self.bounds.size.height;

        if w_canvas > 0.0 && h_canvas > 0.0 {
            let z_x = w_canvas / w_graph;
            let z_y = h_canvas / h_graph;
            self.zoom = z_x.min(z_y).clamp(MIN_ZOOM, MAX_ZOOM);
        } else {
            self.zoom = 1.0;
        }
    }
}

#[derive(Debug, Clone)]
pub struct NodeInstance {
    pub pos: Vec2,
    pub size: Size2,
    pub shape: NodeShape,
    pub color: ColorValue,
    pub border_color: ColorValue,
    pub border_width: f32,
}

#[derive(Debug, Clone)]
pub struct NodeBatch {
    pub instances: Vec<NodeInstance>,
}

#[derive(Debug, Clone)]
pub struct EdgeInstance {
    pub source: Vec2,
    pub target: Vec2,
    pub curve_style: EdgeCurveStyle,
    pub color: ColorValue,
    pub width: f32,
}

#[derive(Debug, Clone)]
pub struct EdgeBatch {
    pub instances: Vec<EdgeInstance>,
}

#[derive(Debug, Clone)]
pub struct LabelInstance {
    pub pos: Vec2,
    pub text_id: LabelId,
    pub font_size: f32,
    pub color: ColorValue,
}

#[derive(Debug, Clone)]
pub struct LabelBatch {
    pub instances: Vec<LabelInstance>,
}

#[derive(Debug, Clone)]
pub struct ImageTileBatch {
    pub urls: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum DrawCommand {
    Nodes(NodeBatch),
    Edges(EdgeBatch),
    Labels(LabelBatch),
    Images(ImageTileBatch),
}

pub struct RenderPipeline {
    pub commands: Vec<DrawCommand>,
}

impl RenderPipeline {
    pub fn new() -> Self {
        Self { commands: Vec::new() }
    }

    pub fn update(&mut self, view: &GraphView<ComputedStyle>, viewport: &Viewport) {
        self.commands.clear();

        let mut edge_instances = Vec::new();
        let mut label_instances = Vec::new();

        for edge in view.edges.values() {
            let (Some(src_node), Some(tgt_node)) = (view.nodes.get(&edge.source), view.nodes.get(&edge.target)) else {
                continue;
            };

            edge_instances.push(EdgeInstance {
                source: src_node.pos,
                target: tgt_node.pos,
                curve_style: EdgeCurveStyle::Straight,
                color: ColorValue::Rgba(0.5, 0.5, 0.5, 1.0),
                width: 2.0,
            });
        }

        if !edge_instances.is_empty() {
            self.commands.push(DrawCommand::Edges(EdgeBatch {
                instances: edge_instances,
            }));
        }

        let mut node_instances = Vec::new();

        for node in view.nodes.values() {
            if !viewport.is_visible(node.pos, node.size) {
                continue;
            }

            let border_width = match node.data.target {
                StylingTarget::Node(ref node_style) => match node_style.border_width {
                    graphene_style::LengthValue::Pixels(px) => px,
                    graphene_style::LengthValue::Ratio(r) => r * node.size.w,
                },
                _ => 2.0,
            };

            let shape = match node.data.target {
                StylingTarget::Node(ref node_style) => node_style.shape,
                _ => NodeShape::Ellipse,
            };

            let fill_color = match node.data.target {
                StylingTarget::Node(ref node_style) => node_style.fill_color,
                _ => ColorValue::Rgba(0.2, 0.6, 0.9, 1.0),
            };

            let border_color = match node.data.target {
                StylingTarget::Node(ref node_style) => node_style.border_color,
                _ => ColorValue::Rgba(1.0, 1.0, 1.0, 1.0),
            };

            node_instances.push(NodeInstance {
                pos: node.pos,
                size: node.size,
                shape,
                color: fill_color,
                border_color,
                border_width,
            });
        }

        if !node_instances.is_empty() {
            self.commands.push(DrawCommand::Nodes(NodeBatch {
                instances: node_instances,
            }));
        }

        if !label_instances.is_empty() {
            self.commands.push(DrawCommand::Labels(LabelBatch {
                instances: label_instances,
            }));
        }
    }
}

impl Default for RenderPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_model_screen_roundtrip() {
        let bounds = gpui::Bounds {
            origin: gpui::Point { x: 0.0, y: 0.0 },
            size: gpui::Size {
                width: 800.0,
                height: 600.0,
            },
        };
        let mut viewport = Viewport::new(bounds);
        viewport.offset = Vec2::new(10.0, -20.0);
        viewport.zoom = 1.5;

        let original_model_pos = Vec2::new(123.45, -67.89);
        let screen_pos = viewport.model_to_screen(original_model_pos);
        let reconstructed_model_pos = viewport.screen_to_model(screen_pos);

        assert!((original_model_pos.x - reconstructed_model_pos.x).abs() < 1e-4);
        assert!((original_model_pos.y - reconstructed_model_pos.y).abs() < 1e-4);
    }

    #[test]
    fn test_viewport_is_visible_bounds_check() {
        let bounds = gpui::Bounds {
            origin: gpui::Point { x: 0.0, y: 0.0 },
            size: gpui::Size {
                width: 800.0,
                height: 600.0,
            },
        };
        let viewport = Viewport::new(bounds);

        let visible_node_pos = Vec2::new(0.0, 0.0);
        let node_size = Size2::new(50.0, 50.0);
        assert!(viewport.is_visible(visible_node_pos, node_size));

        let far_node_pos = Vec2::new(-2000.0, -2000.0);
        assert!(!viewport.is_visible(far_node_pos, node_size));
    }

    #[test]
    fn test_viewport_zoom_100_percent_one_to_one_scale() {
        let bounds = gpui::Bounds {
            origin: gpui::Point { x: 10.0, y: 20.0 },
            size: gpui::Size {
                width: 1000.0,
                height: 800.0,
            },
        };
        let mut viewport = Viewport::new(bounds);
        viewport.zoom = 1.0;

        let p1_graph = Vec2::new(0.0, 0.0);
        let p2_graph = Vec2::new(150.0, 75.0);
        let size_graph = Size2::new(80.0, 40.0);

        let p1_screen = viewport.model_to_screen(p1_graph);
        let p2_screen = viewport.model_to_screen(p2_graph);

        let dx_screen = p2_screen.x - p1_screen.x;
        let dy_screen = p2_screen.y - p1_screen.y;

        assert_eq!(dx_screen, 150.0);
        assert_eq!(dy_screen, 75.0);

        let screen_w = size_graph.w * viewport.zoom;
        let screen_h = size_graph.h * viewport.zoom;
        assert_eq!(screen_w, 80.0);
        assert_eq!(screen_h, 40.0);
    }
}
