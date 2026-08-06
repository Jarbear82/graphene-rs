use graphene_core::math::{Size2, Vec2};
use graphene_core::{EdgeData, EdgeId, GraphState, NodeId};
use graphene_layout::{GraphUpdate, RenderSnapshot};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct NodeViewData<S: Copy + Send + 'static> {
    pub id: NodeId,
    pub pos: Vec2,
    pub size: Size2,
    pub label: String,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub data: S,
    pub node_data: graphene_core::NodeData,
}

#[derive(Debug, Clone)]
pub struct EdgeViewData {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub data: EdgeData,
}

#[derive(Debug, Clone, Default)]
pub struct NodeSizeCache {
    cache: HashMap<(String, u32, usize), Size2>,
}

impl NodeSizeCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
    }

    pub fn get_or_measure(
        &mut self,
        text_system: &gpui::TextSystem,
        label: &str,
        font_size: f32,
        max_len: usize,
        min_size: Size2,
        padding: Vec2,
    ) -> Size2 {
        let font_size_key = font_size as u32;
        let key = (label.to_string(), font_size_key, max_len);

        if let Some(&size) = self.cache.get(&key) {
            return size;
        }

        let display_text = if max_len > 0 && label.chars().count() > max_len {
            let truncated: String = label.chars().take(max_len).collect();
            format!("{}...", truncated)
        } else {
            label.to_string()
        };

        let font_id = text_system.resolve_font(&gpui::font(".SystemUIFont"));
        let font_size_px = gpui::px(font_size);
        let mut measured_w = 0.0;

        for ch in display_text.chars() {
            if let Ok(adv) = text_system.advance(font_id, font_size_px, ch) {
                measured_w += f32::from(adv.width);
            } else {
                measured_w += font_size * 0.6;
            }
        }

        let measured_h = font_size * 1.4 + padding.y;
        let final_size = Size2::new(
            (measured_w + padding.x).max(min_size.w),
            measured_h.max(min_size.h),
        );

        self.cache.insert(key, final_size);
        final_size
    }

    pub fn get_or_measure_window(
        &mut self,
        text_system: &gpui::WindowTextSystem,
        label: &str,
        font_size: f32,
        max_len: usize,
        min_size: Size2,
        padding: Vec2,
    ) -> Size2 {
        let font_size_key = font_size as u32;
        let key = (label.to_string(), font_size_key, max_len);

        if let Some(&size) = self.cache.get(&key) {
            return size;
        }

        let display_text = if max_len > 0 && label.chars().count() > max_len {
            let truncated: String = label.chars().take(max_len).collect();
            format!("{}...", truncated)
        } else {
            label.to_string()
        };

        let font_size_px = gpui::px(font_size);
        let runs = [gpui::TextRun {
            len: display_text.len(),
            font: gpui::font(".SystemUIFont"),
            color: gpui::Hsla::default(),
            background_color: None,
            underline: None,
            strikethrough: None,
        }];

        let shaped_line = text_system.shape_line(
            gpui::SharedString::from(display_text),
            font_size_px,
            &runs,
            None,
        );

        let measured_w = f32::from(shaped_line.width);
        let measured_h = font_size * 1.4 + padding.y;
        let final_size = Size2::new(
            (measured_w + padding.x).max(min_size.w),
            measured_h.max(min_size.h),
        );

        self.cache.insert(key, final_size);
        final_size
    }
}

#[derive(Debug, Clone)]
pub struct GraphView<S: Copy + Send + 'static> {
    pub nodes: HashMap<NodeId, NodeViewData<S>>,
    pub node_order: Vec<NodeId>,
    pub edges: HashMap<EdgeId, EdgeViewData>,
    pub edge_order: Vec<EdgeId>,
    pub version: u64,
    pub size_cache: NodeSizeCache,
}

impl<S: Copy + Default + Send + Sync + 'static> Default for GraphView<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Copy + Default + Send + Sync + 'static> GraphView<S> {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            node_order: Vec::new(),
            edges: HashMap::new(),
            edge_order: Vec::new(),
            version: 0,
            size_cache: NodeSizeCache::new(),
        }
    }

    pub fn from_state(state: &GraphState<S>) -> Self {
        let mut view = Self::new();
        view.load_preset(state);
        view
    }

    pub fn effective_node_position(
        &self,
        id: NodeId,
        drag_session: Option<&crate::interaction::state::DragSession>,
    ) -> Option<Vec2> {
        if let Some(session) = drag_session {
            if session.node_id == id {
                return Some(session.optimistic_pos);
            }
        }
        self.nodes.get(&id).map(|n| n.pos)
    }

    pub fn load_preset(&mut self, state: &GraphState<S>) {
        self.nodes.clear();
        self.node_order.clear();
        self.edges.clear();
        self.edge_order.clear();

        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            let pos = *state.positions.get(idx);
            let size = *state.sizes.get(idx);
            let label = state.display_label(id).unwrap_or_default().to_string();
            let parent = *state.hierarchy.parent.get(idx);
            let data = if idx < state.computed_styles.len() {
                *state.computed_styles.get(idx)
            } else {
                S::default()
            };

            let node_data = state.nodes.get(idx).clone();

            self.nodes.insert(
                id,
                NodeViewData {
                    id,
                    pos,
                    size,
                    label,
                    parent,
                    children: Vec::new(),
                    data,
                    node_data,
                },
            );
            self.node_order.push(id);
        }

        // Populate children hierarchy links
        for (idx, &id) in state.node_index_to_id.iter().enumerate() {
            if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
                if let Some(parent_node) = self.nodes.get_mut(&parent_id) {
                    parent_node.children.push(id);
                }
            }
        }

        for (idx, &id) in state.edge_index_to_id.iter().enumerate() {
            let source = *state.edge_sources.get(idx);
            let target = *state.edge_targets.get(idx);
            let data = state.edges.get(idx).clone();

            self.edges.insert(
                id,
                EdgeViewData {
                    id,
                    source,
                    target,
                    data,
                },
            );
            self.edge_order.push(id);
        }
    }

    pub fn apply_update(&mut self, update: GraphUpdate<S>) {
        match update {
            GraphUpdate::NodeAdded {
                id,
                pos,
                size,
                data,
                label,
                parent,
            } => {
                if !self.nodes.contains_key(&id) {
                    self.nodes.insert(
                        id,
                        NodeViewData {
                            id,
                            pos,
                            size,
                            label,
                            parent,
                            children: Vec::new(),
                            data,
                            node_data: graphene_core::NodeData::default(),
                        },
                    );
                    self.node_order.push(id);
                    if let Some(p_id) = parent {
                        if let Some(p_node) = self.nodes.get_mut(&p_id) {
                            p_node.children.push(id);
                        }
                    }
                }
            }
            GraphUpdate::NodeRemoved { id } => {
                if self.nodes.remove(&id).is_some() {
                    self.node_order.retain(|&x| x != id);
                    for node in self.nodes.values_mut() {
                        node.children.retain(|&x| x != id);
                        if node.parent == Some(id) {
                            node.parent = None;
                        }
                    }
                    self.edges.retain(|_, e| e.source != id && e.target != id);
                    self.edge_order.retain(|&eid| self.edges.contains_key(&eid));
                }
            }
            GraphUpdate::EdgeAdded {
                id,
                source,
                target,
                data,
            } => {
                if !self.edges.contains_key(&id) {
                    self.edges.insert(
                        id,
                        EdgeViewData {
                            id,
                            source,
                            target,
                            data,
                        },
                    );
                    self.edge_order.push(id);
                }
            }
            GraphUpdate::EdgeRemoved { id } => {
                if self.edges.remove(&id).is_some() {
                    self.edge_order.retain(|&x| x != id);
                }
            }
            GraphUpdate::NodeUpdated {
                id,
                pos,
                size,
                label,
                data,
            } => {
                if let Some(node) = self.nodes.get_mut(&id) {
                    if let Some(p) = pos {
                        node.pos = p;
                    }
                    if let Some(s) = size {
                        node.size = s;
                    }
                    if let Some(l) = label {
                        node.label = l;
                    }
                    if let Some(d) = data {
                        node.data = d;
                    }
                }
            }
            GraphUpdate::NodeReparented { child, parent } => {
                if let Some(c_node) = self.nodes.get_mut(&child) {
                    let old_parent = c_node.parent;
                    c_node.parent = parent;
                    if let Some(op) = old_parent {
                        if let Some(op_node) = self.nodes.get_mut(&op) {
                            op_node.children.retain(|&x| x != child);
                        }
                    }
                    if let Some(np) = parent {
                        if let Some(np_node) = self.nodes.get_mut(&np) {
                            if !np_node.children.contains(&child) {
                                np_node.children.push(child);
                            }
                        }
                    }
                }
            }
            GraphUpdate::PresetLoaded(state) => {
                self.load_preset(&state);
            }
            GraphUpdate::StateVersion(v) => {
                self.version = v;
            }
            _ => {}
        }
    }

    pub fn sync_positions_from_snapshot(&mut self, snapshot: &RenderSnapshot) {
        if snapshot.positions.len() == self.node_order.len() {
            for (idx, &id) in self.node_order.iter().enumerate() {
                if let Some(node) = self.nodes.get_mut(&id) {
                    node.pos = snapshot.positions[idx];
                    if idx < snapshot.sizes.len() {
                        node.size = snapshot.sizes[idx];
                    }
                }
            }
            self.version = snapshot.version;
        }
    }

    pub fn measure_and_cache_node_sizes(
        &mut self,
        text_system: &gpui::TextSystem,
        font_size: f32,
        max_label_len: usize,
        collapsed_parents: &std::collections::HashSet<NodeId>,
    ) {
        for (&id, node) in self.nodes.iter_mut() {
            let is_parent = !node.children.is_empty();
            let is_collapsed = collapsed_parents.contains(&id);

            let mut label_text = node.label.clone();
            if is_parent && is_collapsed {
                label_text = format!("[+] {}", label_text);
            }

            let measured_size = self.size_cache.get_or_measure(
                text_system,
                &label_text,
                font_size,
                max_label_len,
                Size2::new(40.0, 30.0),
                Vec2::new(20.0, 10.0),
            );

            node.size = measured_size;
        }
    }
}
