use bitflags::bitflags;
use graphene_core::{EdgeId, NodeId};
use std::collections::HashMap;

/// Arena-indexed label — 4 bytes, Copy, no per-node heap allocation
pub type LabelId = u32;

#[derive(Debug, Clone, Default)]
pub struct StringArena {
    /// Centralized storage. Labels point into this by index.
    pub strings: Vec<String>,
}

impl StringArena {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    pub fn intern(&mut self, s: String) -> LabelId {
        if let Some(pos) = self.strings.iter().position(|x| x == &s) {
            pos as u32
        } else {
            let id = self.strings.len() as u32;
            self.strings.push(s);
            id
        }
    }

    pub fn get(&self, id: LabelId) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorValue {
    Rgba(f32, f32, f32, f32), // [0.0..1.0] range for all channels — no gpui leak
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthValue {
    Pixels(f32),
    Ratio(f32), // relative to node size (e.g., border width = 0.05 * radius)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeShape {
    Ellipse,
    Rectangle,
    Triangle,
    Square,
    Diamond,
    Pentagon,
    Hexagon,
    Octagon,
    Star,
    Ribbon,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EdgeCurveStyle {
    Straight,
    Bezier,
    Segmented,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeStyle {
    pub fill_color: ColorValue,
    pub border_color: ColorValue,
    pub border_width: LengthValue,
    pub shape: NodeShape,
    pub label: Option<LabelId>, // 4 bytes — arena index, trivially Copy
    pub label_font_size: f32,
    pub visible: bool,
}

impl Default for NodeStyle {
    fn default() -> Self {
        Self {
            fill_color: ColorValue::Rgba(0.8, 0.8, 0.8, 1.0),
            border_color: ColorValue::Rgba(0.2, 0.2, 0.2, 1.0),
            border_width: LengthValue::Pixels(2.0),
            shape: NodeShape::Ellipse,
            label: None,
            label_font_size: 14.0,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EdgeStyle {
    pub line_color: ColorValue,
    pub line_width: LengthValue,
    pub curve_style: EdgeCurveStyle,
    pub label: Option<LabelId>,
    pub label_font_size: f32,
    pub visible: bool,
}

impl Default for EdgeStyle {
    fn default() -> Self {
        Self {
            line_color: ColorValue::Rgba(0.2, 0.2, 0.2, 1.0),
            line_width: LengthValue::Pixels(1.5),
            curve_style: EdgeCurveStyle::Straight,
            label: None,
            label_font_size: 12.0,
            visible: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StylingTarget {
    Node(NodeStyle),
    Edge(EdgeStyle),
}

impl Default for StylingTarget {
    fn default() -> Self {
        Self::Node(NodeStyle::default())
    }
}

// Computed style per element — flattened array read by the renderer. All fields are Copy.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ComputedStyle {
    pub target: StylingTarget,
}

/// Patch — only the properties explicitly set by a rule. None = "don't override".
#[derive(Debug, Clone, Copy, Default)]
pub struct StylePatch {
    pub fill_color: Option<ColorValue>,
    pub border_color: Option<ColorValue>,
    pub border_width: Option<LengthValue>,
    pub shape: Option<NodeShape>,
    pub label: Option<LabelId>,
    pub label_font_size: Option<f32>,
    pub visible: Option<bool>,

    pub line_color: Option<ColorValue>,
    pub line_width: Option<LengthValue>,
    pub curve_style: Option<EdgeCurveStyle>,
}

impl StylePatch {
    /// Merge a patch into an existing ComputedStyle. Only Some fields overwrite.
    pub fn merge_into(self, computed: &mut ComputedStyle) {
        match computed.target {
            StylingTarget::Node(ref mut node_style) => {
                if let Some(v) = self.fill_color {
                    node_style.fill_color = v;
                }
                if let Some(v) = self.border_color {
                    node_style.border_color = v;
                }
                if let Some(v) = self.border_width {
                    node_style.border_width = v;
                }
                if let Some(v) = self.shape {
                    node_style.shape = v;
                }
                if let Some(v) = self.label {
                    node_style.label = Some(v);
                }
                if let Some(v) = self.label_font_size {
                    node_style.label_font_size = v;
                }
                if let Some(v) = self.visible {
                    node_style.visible = v;
                }
            }
            StylingTarget::Edge(ref mut edge_style) => {
                if let Some(v) = self.line_color {
                    edge_style.line_color = v;
                }
                if let Some(v) = self.line_width {
                    edge_style.line_width = v;
                }
                if let Some(v) = self.curve_style {
                    edge_style.curve_style = v;
                }
                if let Some(v) = self.label {
                    edge_style.label = Some(v);
                }
                if let Some(v) = self.label_font_size {
                    edge_style.label_font_size = v;
                }
                if let Some(v) = self.visible {
                    edge_style.visible = v;
                }
            }
        }
    }
}

/// ClassId — resolved once at application startup from class name strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClassId(pub u32);

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct StateFlags: u8 {
        const SELECTED = 1 << 0;
        const GRABBED = 1 << 1;
        const HOVERED = 1 << 2;
    }
}

/// Selector — no Strings, no heap allocation. All variants are Copy/enum-backed.
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    All,
    NodeType,
    EdgeType,
    NodeOf(NodeId), // exact ID match
    EdgeOf(EdgeId),
    Class(ClassId),    // u32 — O(1) bitfield or direct comparison
    State(StateFlags), // bitfield: SELECTED | GRABBED | HOVERED
}

#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selector: Selector,
    pub patch: StylePatch,
}

#[derive(Debug, Clone, Default)]
pub struct ClassStore {
    pub node_classes: HashMap<NodeId, Vec<ClassId>>,
    pub edge_classes: HashMap<EdgeId, Vec<ClassId>>,
    pub class_names: HashMap<String, ClassId>,
    pub next_class_id: u32,
}

impl ClassStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_or_create_class(&mut self, name: &str) -> ClassId {
        if let Some(&id) = self.class_names.get(name) {
            id
        } else {
            let id = ClassId(self.next_class_id);
            self.next_class_id += 1;
            self.class_names.insert(name.to_string(), id);
            id
        }
    }

    pub fn add_class_to_node(&mut self, node: NodeId, class: ClassId) {
        self.node_classes.entry(node).or_default().push(class);
    }

    pub fn add_class_to_edge(&mut self, edge: EdgeId, class: ClassId) {
        self.edge_classes.entry(edge).or_default().push(class);
    }
}

pub fn matches_selector(
    selector: &Selector,
    node_id: Option<NodeId>,
    edge_id: Option<EdgeId>,
    class_store: &ClassStore,
    state_flags: StateFlags,
) -> bool {
    match selector {
        Selector::All => true,
        Selector::NodeType => node_id.is_some(),
        Selector::EdgeType => edge_id.is_some(),
        Selector::NodeOf(id) => node_id == Some(*id),
        Selector::EdgeOf(id) => edge_id == Some(*id),
        Selector::Class(class_id) => {
            if let Some(nid) = node_id {
                class_store
                    .node_classes
                    .get(&nid)
                    .map(|classes| classes.contains(class_id))
                    .unwrap_or(false)
            } else if let Some(eid) = edge_id {
                class_store
                    .edge_classes
                    .get(&eid)
                    .map(|classes| classes.contains(class_id))
                    .unwrap_or(false)
            } else {
                false
            }
        }
        Selector::State(flags) => state_flags.contains(*flags),
    }
}

#[derive(Debug, Clone, Default)]
pub struct RuleEngine {
    pub rules: Vec<StyleRule>,
    pub class_rules: HashMap<ClassId, Vec<StyleRule>>,
    pub state_rules: Vec<StyleRule>,
    pub general_rules: Vec<StyleRule>,
}

impl RuleEngine {
    pub fn new(rules: Vec<StyleRule>) -> Self {
        let mut class_rules: HashMap<ClassId, Vec<StyleRule>> = HashMap::new();
        let mut state_rules = Vec::new();
        let mut general_rules = Vec::new();

        for rule in &rules {
            match &rule.selector {
                Selector::Class(class_id) => {
                    class_rules.entry(*class_id).or_default().push(rule.clone());
                }
                Selector::State(_) => {
                    state_rules.push(rule.clone());
                }
                _ => {
                    general_rules.push(rule.clone());
                }
            }
        }

        Self {
            rules,
            class_rules,
            state_rules,
            general_rules,
        }
    }

    pub fn compute_node_style(
        &self,
        node_id: NodeId,
        class_store: &ClassStore,
        state_flags: StateFlags,
    ) -> ComputedStyle {
        let mut computed = ComputedStyle {
            target: StylingTarget::Node(NodeStyle::default()),
        };

        // 1. General rules
        for rule in &self.general_rules {
            if matches_selector(
                &rule.selector,
                Some(node_id),
                None,
                class_store,
                state_flags,
            ) {
                rule.patch.merge_into(&mut computed);
            }
        }

        // 2. Class specific rules
        if let Some(classes) = class_store.node_classes.get(&node_id) {
            for class_id in classes {
                if let Some(rules) = self.class_rules.get(class_id) {
                    for rule in rules {
                        rule.patch.merge_into(&mut computed);
                    }
                }
            }
        }

        // 3. State rules
        for rule in &self.state_rules {
            if matches_selector(
                &rule.selector,
                Some(node_id),
                None,
                class_store,
                state_flags,
            ) {
                rule.patch.merge_into(&mut computed);
            }
        }

        computed
    }

    pub fn compute_edge_style(
        &self,
        edge_id: EdgeId,
        class_store: &ClassStore,
        state_flags: StateFlags,
    ) -> ComputedStyle {
        let mut computed = ComputedStyle {
            target: StylingTarget::Edge(EdgeStyle::default()),
        };

        // 1. General rules
        for rule in &self.general_rules {
            if matches_selector(
                &rule.selector,
                None,
                Some(edge_id),
                class_store,
                state_flags,
            ) {
                rule.patch.merge_into(&mut computed);
            }
        }

        // 2. Class specific rules
        if let Some(classes) = class_store.edge_classes.get(&edge_id) {
            for class_id in classes {
                if let Some(rules) = self.class_rules.get(class_id) {
                    for rule in rules {
                        rule.patch.merge_into(&mut computed);
                    }
                }
            }
        }

        // 3. State rules
        for rule in &self.state_rules {
            if matches_selector(
                &rule.selector,
                None,
                Some(edge_id),
                class_store,
                state_flags,
            ) {
                rule.patch.merge_into(&mut computed);
            }
        }

        computed
    }
}
