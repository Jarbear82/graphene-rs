use bitflags::bitflags;
use graphene_core::{EdgeId, NodeId, StringId};
pub use graphene_core::StringArena;
use std::collections::HashMap;

pub type LabelId = StringId;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CompareOp {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

impl CompareOp {
    pub fn compare_f64(&self, a: f64, b: f64) -> bool {
        match self {
            CompareOp::Equal => (a - b).abs() < f64::EPSILON,
            CompareOp::NotEqual => (a - b).abs() >= f64::EPSILON,
            CompareOp::LessThan => a < b,
            CompareOp::LessThanOrEqual => a <= b,
            CompareOp::GreaterThan => a > b,
            CompareOp::GreaterThanOrEqual => a >= b,
        }
    }

    pub fn compare_i64(&self, a: i64, b: i64) -> bool {
        match self {
            CompareOp::Equal => a == b,
            CompareOp::NotEqual => a != b,
            CompareOp::LessThan => a < b,
            CompareOp::LessThanOrEqual => a <= b,
            CompareOp::GreaterThan => a > b,
            CompareOp::GreaterThanOrEqual => a >= b,
        }
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
    DataExists(graphene_core::StringId),
    DataFloatCompare(graphene_core::StringId, CompareOp, f64),
    DataIntCompare(graphene_core::StringId, CompareOp, i64),
    DataStrEquals(graphene_core::StringId, graphene_core::StringId),
    DataBoolEquals(graphene_core::StringId, bool),
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
    user_data: Option<&graphene_core::UserData>,
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
        Selector::DataExists(key) => {
            user_data.map(|ud| ud.fields.contains_key(key)).unwrap_or(false)
        }
        Selector::DataFloatCompare(key, op, val) => {
            if let Some(ud) = user_data {
                match ud.fields.get(key) {
                    Some(graphene_core::UserDataValue::Float(f)) => op.compare_f64(*f, *val),
                    Some(graphene_core::UserDataValue::Integer(i)) => op.compare_f64(*i as f64, *val),
                    _ => false,
                }
            } else {
                false
            }
        }
        Selector::DataIntCompare(key, op, val) => {
            if let Some(ud) = user_data {
                match ud.fields.get(key) {
                    Some(graphene_core::UserDataValue::Integer(i)) => op.compare_i64(*i, *val),
                    Some(graphene_core::UserDataValue::Float(f)) => op.compare_i64(*f as i64, *val),
                    _ => false,
                }
            } else {
                false
            }
        }
        Selector::DataStrEquals(key, val) => {
            if let Some(ud) = user_data {
                match ud.fields.get(key) {
                    Some(graphene_core::UserDataValue::String(s)) => s == val,
                    _ => false,
                }
            } else {
                false
            }
        }
        Selector::DataBoolEquals(key, val) => {
            if let Some(ud) = user_data {
                match ud.fields.get(key) {
                    Some(graphene_core::UserDataValue::Boolean(b)) => b == val,
                    _ => false,
                }
            } else {
                false
            }
        }
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
        user_data: Option<&graphene_core::UserData>,
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
                user_data,
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
                user_data,
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
        user_data: Option<&graphene_core::UserData>,
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
                user_data,
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
                user_data,
            ) {
                rule.patch.merge_into(&mut computed);
            }
        }

        computed
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    pub name: &'static str,
    pub bg: ColorValue,
    pub panel_bg: ColorValue,
    pub border: ColorValue,
    pub accent: ColorValue,
    pub text: ColorValue,
    pub text_dim: ColorValue,
    pub node_fill: ColorValue,
    pub node_border: ColorValue,
    pub edge_color: ColorValue,
}

impl Theme {
    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha",
            bg: ColorValue::Rgba(0.12, 0.12, 0.18, 1.0),
            panel_bg: ColorValue::Rgba(0.09, 0.09, 0.15, 1.0),
            border: ColorValue::Rgba(0.19, 0.20, 0.27, 1.0),
            accent: ColorValue::Rgba(0.54, 0.71, 0.98, 1.0),
            text: ColorValue::Rgba(0.80, 0.84, 0.96, 1.0),
            text_dim: ColorValue::Rgba(0.65, 0.68, 0.78, 1.0),
            node_fill: ColorValue::Rgba(0.19, 0.20, 0.27, 1.0),
            node_border: ColorValue::Rgba(0.80, 0.84, 0.96, 1.0),
            edge_color: ColorValue::Rgba(0.27, 0.28, 0.35, 1.0),
        }
    }

    pub fn gruvbox_dark() -> Self {
        Self {
            name: "Gruvbox Dark",
            bg: ColorValue::Rgba(0.16, 0.16, 0.16, 1.0),
            panel_bg: ColorValue::Rgba(0.11, 0.13, 0.13, 1.0),
            border: ColorValue::Rgba(0.24, 0.22, 0.21, 1.0),
            accent: ColorValue::Rgba(0.84, 0.36, 0.05, 1.0),
            text: ColorValue::Rgba(0.98, 0.95, 0.78, 1.0),
            text_dim: ColorValue::Rgba(0.66, 0.60, 0.52, 1.0),
            node_fill: ColorValue::Rgba(0.24, 0.22, 0.21, 1.0),
            node_border: ColorValue::Rgba(0.98, 0.95, 0.78, 1.0),
            edge_color: ColorValue::Rgba(0.31, 0.29, 0.27, 1.0),
        }
    }

    pub fn one_dark() -> Self {
        Self {
            name: "One Dark",
            bg: ColorValue::Rgba(0.16, 0.17, 0.20, 1.0),
            panel_bg: ColorValue::Rgba(0.13, 0.15, 0.17, 1.0),
            border: ColorValue::Rgba(0.09, 0.10, 0.12, 1.0),
            accent: ColorValue::Rgba(0.60, 0.76, 0.47, 1.0),
            text: ColorValue::Rgba(0.67, 0.70, 0.75, 1.0),
            text_dim: ColorValue::Rgba(0.36, 0.39, 0.44, 1.0),
            node_fill: ColorValue::Rgba(0.24, 0.27, 0.32, 1.0),
            node_border: ColorValue::Rgba(0.67, 0.70, 0.75, 1.0),
            edge_color: ColorValue::Rgba(0.17, 0.19, 0.24, 1.0),
        }
    }

    pub fn github_light() -> Self {
        Self {
            name: "GitHub Light",
            bg: ColorValue::Rgba(1.0, 1.0, 1.0, 1.0),
            panel_bg: ColorValue::Rgba(0.96, 0.97, 0.98, 1.0),
            border: ColorValue::Rgba(0.82, 0.84, 0.87, 1.0),
            accent: ColorValue::Rgba(0.04, 0.41, 0.85, 1.0),
            text: ColorValue::Rgba(0.14, 0.16, 0.18, 1.0),
            text_dim: ColorValue::Rgba(0.34, 0.38, 0.42, 1.0),
            node_fill: ColorValue::Rgba(0.96, 0.97, 0.98, 1.0),
            node_border: ColorValue::Rgba(0.14, 0.16, 0.18, 1.0),
            edge_color: ColorValue::Rgba(0.82, 0.84, 0.87, 1.0),
        }
    }
}

pub struct ThemeRegistry {
    pub themes: Vec<Theme>,
}

impl ThemeRegistry {
    pub fn new() -> Self {
        Self {
            themes: vec![
                Theme::catppuccin_mocha(),
                Theme::gruvbox_dark(),
                Theme::one_dark(),
                Theme::github_light(),
            ],
        }
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DataMapper {
    pub min_val: f32,
    pub max_val: f32,
}

impl DataMapper {
    pub fn new(min_val: f32, max_val: f32) -> Self {
        Self { min_val, max_val }
    }

    pub fn map_color(&self, val: f32, start: ColorValue, end: ColorValue) -> ColorValue {
        let t = if self.max_val > self.min_val {
            ((val - self.min_val) / (self.max_val - self.min_val)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        match (start, end) {
            (ColorValue::Rgba(r1, g1, b1, a1), ColorValue::Rgba(r2, g2, b2, a2)) => {
                ColorValue::Rgba(
                    r1 + t * (r2 - r1),
                    g1 + t * (g2 - g1),
                    b1 + t * (b2 - b1),
                    a1 + t * (a2 - a1),
                )
            }
        }
    }

    pub fn map_size(&self, val: f32, min_size: f32, max_size: f32) -> f32 {
        let t = if self.max_val > self.min_val {
            ((val - self.min_val) / (self.max_val - self.min_val)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        min_size + t * (max_size - min_size)
    }
}

#[derive(Debug, Clone, Default)]
pub struct StylingEngine {
    pub rule_engine: RuleEngine,
    pub node_bypasses: HashMap<NodeId, StylePatch>,
    pub edge_bypasses: HashMap<EdgeId, StylePatch>,
}

impl StylingEngine {
    pub fn new(rule_engine: RuleEngine) -> Self {
        Self {
            rule_engine,
            node_bypasses: HashMap::new(),
            edge_bypasses: HashMap::new(),
        }
    }

    pub fn set_node_bypass(&mut self, node: NodeId, patch: StylePatch) {
        self.node_bypasses.insert(node, patch);
    }

    pub fn clear_node_bypass(&mut self, node: NodeId) {
        self.node_bypasses.remove(&node);
    }

    pub fn set_edge_bypass(&mut self, edge: EdgeId, patch: StylePatch) {
        self.edge_bypasses.insert(edge, patch);
    }

    pub fn clear_edge_bypass(&mut self, edge: EdgeId) {
        self.edge_bypasses.remove(&edge);
    }

    pub fn compute_node_style(
        &self,
        node_id: NodeId,
        class_store: &ClassStore,
        state_flags: StateFlags,
        user_data: Option<&graphene_core::UserData>,
    ) -> ComputedStyle {
        let mut computed = self.rule_engine.compute_node_style(node_id, class_store, state_flags, user_data);
        if let Some(bypass) = self.node_bypasses.get(&node_id) {
            bypass.merge_into(&mut computed);
        }
        computed
    }

    pub fn compute_edge_style(
        &self,
        edge_id: EdgeId,
        class_store: &ClassStore,
        state_flags: StateFlags,
        user_data: Option<&graphene_core::UserData>,
    ) -> ComputedStyle {
        let mut computed = self.rule_engine.compute_edge_style(edge_id, class_store, state_flags, user_data);
        if let Some(bypass) = self.edge_bypasses.get(&edge_id) {
            bypass.merge_into(&mut computed);
        }
        computed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{GraphState, Vec2, Size2, UserDataValue};

    #[test]
    fn test_data_driven_selectors_and_bypasses() {
        let mut state: GraphState<ComputedStyle> = GraphState::new();
        let n1 = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));

        let key_weight = state.string_arena.intern("weight".to_string());
        let key_name = state.string_arena.intern("name".to_string());
        let key_active = state.string_arena.intern("active".to_string());
        let val_alice = state.string_arena.intern("Alice".to_string());

        // Setup user data for node
        let idx = state.node_keys[n1];
        state.nodes[idx].user_data.insert(key_weight, UserDataValue::Float(7.5));
        state.nodes[idx].user_data.insert(key_name, UserDataValue::String(val_alice));
        state.nodes[idx].user_data.insert(key_active, UserDataValue::Boolean(true));

        let class_store = ClassStore::new();

        // 1. Test Selector::DataExists
        let sel_exists = Selector::DataExists(key_weight);
        assert!(matches_selector(&sel_exists, Some(n1), None, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data)));

        let sel_not_exists = Selector::DataExists(999);
        assert!(!matches_selector(&sel_not_exists, Some(n1), None, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data)));

        // 2. Test Selector::DataFloatCompare
        let sel_float_gt = Selector::DataFloatCompare(key_weight, CompareOp::GreaterThan, 5.0);
        assert!(matches_selector(&sel_float_gt, Some(n1), None, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data)));

        let sel_float_lt = Selector::DataFloatCompare(key_weight, CompareOp::LessThan, 5.0);
        assert!(!matches_selector(&sel_float_lt, Some(n1), None, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data)));

        // 3. Test Selector::DataStrEquals
        let sel_str_eq = Selector::DataStrEquals(key_name, val_alice);
        assert!(matches_selector(&sel_str_eq, Some(n1), None, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data)));

        // 4. Test Selector::DataBoolEquals
        let sel_bool_eq = Selector::DataBoolEquals(key_active, true);
        assert!(matches_selector(&sel_bool_eq, Some(n1), None, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data)));

        // 5. Test RuleEngine with Data-Driven Selectors
        let rule_weight = StyleRule {
            selector: Selector::DataFloatCompare(key_weight, CompareOp::GreaterThan, 5.0),
            patch: StylePatch {
                fill_color: Some(ColorValue::Rgba(1.0, 0.0, 0.0, 1.0)),
                ..Default::default()
            },
        };
        let rule_engine = RuleEngine::new(vec![rule_weight]);
        let computed = rule_engine.compute_node_style(n1, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data));
        if let StylingTarget::Node(node_style) = computed.target {
            assert_eq!(node_style.fill_color, ColorValue::Rgba(1.0, 0.0, 0.0, 1.0));
        } else {
            panic!("Expected node style target");
        }

        // 6. Test StylingEngine with overrides/bypasses
        let mut styling_engine = StylingEngine::new(rule_engine);
        let computed_before = styling_engine.compute_node_style(n1, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data));
        if let StylingTarget::Node(node_style) = computed_before.target {
            assert_eq!(node_style.fill_color, ColorValue::Rgba(1.0, 0.0, 0.0, 1.0));
        }

        // Apply bypass
        let bypass_patch = StylePatch {
            fill_color: Some(ColorValue::Rgba(0.0, 0.0, 1.0, 1.0)),
            ..Default::default()
        };
        styling_engine.set_node_bypass(n1, bypass_patch);

        let computed_after = styling_engine.compute_node_style(n1, &class_store, StateFlags::empty(), Some(&state.nodes[idx].user_data));
        if let StylingTarget::Node(node_style) = computed_after.target {
            assert_eq!(node_style.fill_color, ColorValue::Rgba(0.0, 0.0, 1.0, 1.0));
        } else {
            panic!("Expected node style target");
        }
    }
}

