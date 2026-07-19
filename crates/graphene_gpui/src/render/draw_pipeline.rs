use graphene_core::{math::{Size2, Vec2}, DirtyFlags, GraphState};
use graphene_style::{ComputedStyle, EdgeCurveStyle, ColorValue, LabelId, NodeShape, StylingTarget};

#[derive(Debug, Clone)]
pub struct Viewport {
    pub offset: Vec2,
    pub zoom: f32,
    pub bounds: gpui::Bounds<f32>,
}

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
        // Size scales with zoom
        let screen_size = gpui::size(size.w * self.zoom, size.h * self.zoom);
        
        let node_bounds = gpui::Bounds {
            origin: gpui::point(screen_pos.x - screen_size.width / 2.0, screen_pos.y - screen_size.height / 2.0),
            size: screen_size,
        };

        self.bounds.intersects(&node_bounds)
    }

    pub fn fit_to_graph(&mut self, state: &GraphState<ComputedStyle>) {
        if state.node_index_to_id.is_empty() {
            self.offset = Vec2::default();
            self.zoom = 1.0;
            return;
        }
        let mut x_min = f32::MAX;
        let mut x_max = f32::MIN;
        let mut y_min = f32::MAX;
        let mut y_max = f32::MIN;
        for &id in &state.node_index_to_id {
            if let Some(&idx) = state.node_keys.get(id) {
                let pos = *state.positions.get(idx);
                x_min = x_min.min(pos.x);
                x_max = x_max.max(pos.x);
                y_min = y_min.min(pos.y);
                y_max = y_max.max(pos.y);
            }
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
            self.zoom = z_x.min(z_y).clamp(0.2, 3.0);
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

    pub fn update(&mut self, state: &GraphState<ComputedStyle>, viewport: &Viewport) {
        // Respect dirty flags to avoid redundant work (skip rebuild if nothing changed)
        if !state.dirty_flags.contains(DirtyFlags::POSITION_DIRTY | DirtyFlags::TOPOLOGY_DIRTY) {
            return;
        }

        self.commands.clear();

        // 1. Collect and batch edges (Phase 1 accepts linear scan)
        let mut edge_instances = Vec::new();
        let mut label_instances = Vec::new();

        for i in 0..state.edges.len() {
            let src = *state.edge_sources.get(i);
            let tgt = *state.edge_targets.get(i);

            let (Some(&src_idx), Some(&tgt_idx)) = (state.node_keys.get(src), state.node_keys.get(tgt)) else {
                continue;
            };

            let pos_src = *state.positions.get(src_idx);
            let pos_tgt = *state.positions.get(tgt_idx);

            // Fetch computed style for edge
            let style = state.edge_computed_styles.get(i);
            if let StylingTarget::Edge(ref edge_style) = style.target {
                if !edge_style.visible {
                    continue;
                }

                let width = match edge_style.line_width {
                    graphene_style::LengthValue::Pixels(px) => px,
                    graphene_style::LengthValue::Ratio(r) => r * 10.0, // fallback scaling
                };

                edge_instances.push(EdgeInstance {
                    source: pos_src,
                    target: pos_tgt,
                    curve_style: edge_style.curve_style,
                    color: edge_style.line_color,
                    width,
                });

                if let Some(lbl_id) = edge_style.label {
                    let mid_point = Vec2::new((pos_src.x + pos_tgt.x) / 2.0, (pos_src.y + pos_tgt.y) / 2.0);
                    label_instances.push(LabelInstance {
                        pos: mid_point,
                        text_id: lbl_id,
                        font_size: edge_style.label_font_size,
                        color: edge_style.line_color,
                    });
                }
            }
        }

        if !edge_instances.is_empty() {
            self.commands.push(DrawCommand::Edges(EdgeBatch {
                instances: edge_instances,
            }));
        }

        // 2. Frustum culling & batching nodes
        let mut node_instances = Vec::new();

        for (idx, &_id) in state.node_index_to_id.iter().enumerate() {
            let pos = *state.positions.get(idx);
            let size = *state.sizes.get(idx);

            // Frustum culling check
            if !viewport.is_visible(pos, size) {
                continue;
            }

            let style = state.computed_styles.get(idx);
            if let StylingTarget::Node(ref node_style) = style.target {
                if !node_style.visible {
                    continue;
                }

                let border_width = match node_style.border_width {
                    graphene_style::LengthValue::Pixels(px) => px,
                    graphene_style::LengthValue::Ratio(r) => r * size.w,
                };

                node_instances.push(NodeInstance {
                    pos,
                    size,
                    shape: node_style.shape,
                    color: node_style.fill_color,
                    border_color: node_style.border_color,
                    border_width,
                });

                if let Some(lbl_id) = node_style.label {
                    // Position label slightly below center or inside the node bounds
                    label_instances.push(LabelInstance {
                        pos,
                        text_id: lbl_id,
                        font_size: node_style.label_font_size,
                        color: node_style.border_color,
                    });
                }
            }
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
