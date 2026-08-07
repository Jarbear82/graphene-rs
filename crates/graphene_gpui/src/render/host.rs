use crate::interaction::state::InteractionState;
use crate::render::draw_pipeline::Viewport;
use crate::render::graph_canvas::{CanvasConfig, GraphCanvas};
use crate::view::GraphView;
use gpui::prelude::*;
use gpui::{IntoElement, SharedString, Styled};
use graphene_core::NodeId;
use graphene_style::ComputedStyle;
use std::collections::HashMap;

pub struct GraphCanvasHost<'a, S: Copy + Send + 'static = ComputedStyle> {
    pub view: &'a GraphView<S>,
    pub viewport: &'a Viewport,
    pub interaction_state: &'a InteractionState,
    pub theme: &'a graphene_style::Theme,
    pub selected_node: Option<NodeId>,
    pub node_labels: &'a HashMap<NodeId, String>,
    pub edge_labels: &'a HashMap<usize, String>,
    pub max_label_len: usize,
    pub collapsed_parents: &'a std::collections::HashSet<NodeId>,
    pub config: CanvasConfig,
    pub is_directed: bool,
    pub centrality_scores: Option<&'a HashMap<NodeId, f32>>,
    pub container_id: SharedString,
}

impl<'a> GraphCanvasHost<'a, ComputedStyle> {
    pub fn new(
        view: &'a GraphView<ComputedStyle>,
        viewport: &'a Viewport,
        interaction_state: &'a InteractionState,
        theme: &'a graphene_style::Theme,
        selected_node: Option<NodeId>,
        node_labels: &'a HashMap<NodeId, String>,
        edge_labels: &'a HashMap<usize, String>,
        max_label_len: usize,
        collapsed_parents: &'a std::collections::HashSet<NodeId>,
    ) -> Self {
        Self {
            view,
            viewport,
            interaction_state,
            theme,
            selected_node,
            node_labels,
            edge_labels,
            max_label_len,
            collapsed_parents,
            config: CanvasConfig::default(),
            is_directed: true,
            centrality_scores: None,
            container_id: SharedString::from("canvas-host-container"),
        }
    }

    pub fn with_config(mut self, config: CanvasConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_directed(mut self, is_directed: bool) -> Self {
        self.is_directed = is_directed;
        self
    }

    pub fn with_centrality_scores(mut self, scores: Option<&'a HashMap<NodeId, f32>>) -> Self {
        self.centrality_scores = scores;
        self
    }

    pub fn with_container_id(mut self, id: impl Into<SharedString>) -> Self {
        self.container_id = id.into();
        self
    }
}

impl<'a> IntoElement for GraphCanvasHost<'a, ComputedStyle> {
    type Element = gpui::AnyElement;

    fn into_element(self) -> Self::Element {
        let bg_color = crate::style_bridge::color_value_to_rgba(self.theme.bg);

        gpui::div()
            .id(self.container_id)
            .flex_1()
            .h_full()
            .relative()
            .overflow_hidden()
            .bg(bg_color)
            .child(
                GraphCanvas::new(
                    self.view,
                    self.viewport,
                    self.interaction_state,
                    self.theme,
                    self.selected_node,
                    self.node_labels,
                    self.edge_labels,
                    self.max_label_len,
                    self.collapsed_parents,
                )
                .with_directed(self.is_directed)
                .with_centrality_scores(self.centrality_scores)
                .with_config(self.config),
            )
            .into_any_element()
    }
}
