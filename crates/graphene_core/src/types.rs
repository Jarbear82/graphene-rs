use bitflags::bitflags;
use bitvec::vec::BitVec;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use std::time::Duration;

pub use crate::math::{Size2, Vec2};

new_key_type! {
    pub struct NodeId;
    pub struct EdgeId;
}

// === GRAPH TYPING & CLASSIFICATION INTEGRATIONS ===

pub trait EdgeType: 'static + Send + Sync + std::fmt::Debug {
    const IS_DIRECTED: bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Directed;
impl EdgeType for Directed {
    const IS_DIRECTED: bool = true;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Undirected;
impl EdgeType for Undirected {
    const IS_DIRECTED: bool = false;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mixed;
impl EdgeType for Mixed {
    const IS_DIRECTED: bool = true;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    SelfLoopNotAllowed,
    ParallelEdgeNotAllowed,
    NodeNotFound(NodeId),
    EdgeNotFound(EdgeId),
    CycleDetected,
    SerializationError(String),
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::SelfLoopNotAllowed => write!(f, "Self-loops are not allowed under this insert policy"),
            GraphError::ParallelEdgeNotAllowed => write!(f, "Parallel edges are not allowed under this insert policy"),
            GraphError::NodeNotFound(id) => write!(f, "Node {:?} not found", id),
            GraphError::EdgeNotFound(id) => write!(f, "Edge {:?} not found", id),
            GraphError::CycleDetected => write!(f, "Hierarchy cycle detected"),
            GraphError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
        }
    }
}

impl std::error::Error for GraphError {}

pub trait InsertPolicy<Ty: EdgeType> {
    fn validate<S: Copy>(
        state: &crate::state::GraphState<S>,
        source: NodeId,
        target: NodeId,
    ) -> Result<(), GraphError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AllowMulti;
impl<Ty: EdgeType> InsertPolicy<Ty> for AllowMulti {
    fn validate<S: Copy>(
        _state: &crate::state::GraphState<S>,
        _source: NodeId,
        _target: NodeId,
    ) -> Result<(), GraphError> {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SimpleOnly;
impl<Ty: EdgeType> InsertPolicy<Ty> for SimpleOnly {
    fn validate<S: Copy>(
        state: &crate::state::GraphState<S>,
        source: NodeId,
        target: NodeId,
    ) -> Result<(), GraphError> {
        if source == target {
            return Err(GraphError::SelfLoopNotAllowed);
        }
        for (i, &src) in state.edge_sources.iter().enumerate() {
            let tgt = state.edge_targets[i];
            if Ty::IS_DIRECTED {
                if src == source && tgt == target {
                    return Err(GraphError::ParallelEdgeNotAllowed);
                }
            } else {
                if (src == source && tgt == target) || (src == target && tgt == source) {
                    return Err(GraphError::ParallelEdgeNotAllowed);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NodeKind {
    Vertex,
    HyperedgeProxy,
}

impl Default for NodeKind {
    fn default() -> Self {
        NodeKind::Vertex
    }
}

/// Safe wrapper around a parallel array. Operates purely on `usize` indices.
#[derive(Debug, Clone)]
pub struct DenseStorage<T> {
    data: Vec<T>,
}

impl<T> DenseStorage<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn insert(&mut self, value: T) -> usize {
        let idx = self.data.len();
        self.data.push(value);
        idx
    }

    pub fn remove(&mut self, idx: usize) -> T {
        assert!(
            idx < self.data.len(),
            "DenseStorage::remove index out of bounds: idx={}, len={}",
            idx,
            self.data.len()
        );
        let last = self.data.len() - 1;
        if idx != last {
            self.data.swap(idx, last);
        }
        self.data.pop().unwrap()
    }

    pub fn get(&self, idx: usize) -> &T {
        &self.data[idx]
    }

    pub fn get_mut(&mut self, idx: usize) -> &mut T {
        &mut self.data[idx]
    }

    pub fn set(&mut self, idx: usize, value: T) {
        self.data[idx] = value;
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T> std::ops::Deref for DenseStorage<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T> std::ops::DerefMut for DenseStorage<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

impl<T> Default for DenseStorage<T> {
    fn default() -> Self {
        Self::new()
    }
}

pub type StringId = u32;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StringArena {
    pub strings: Vec<String>,
    pub lookup: std::collections::HashMap<String, StringId>,
}

impl StringArena {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
            lookup: std::collections::HashMap::new(),
        }
    }

    pub fn intern(&mut self, s: String) -> StringId {
        if let Some(&id) = self.lookup.get(&s) {
            id
        } else {
            let id = self.strings.len() as u32;
            self.lookup.insert(s.clone(), id);
            self.strings.push(s);
            id
        }
    }

    pub fn get(&self, id: StringId) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum UserDataValue {
    String(StringId),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl Eq for UserDataValue {}

impl std::hash::Hash for UserDataValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        core::mem::discriminant(self).hash(state);
        match self {
            UserDataValue::String(s) => s.hash(state),
            UserDataValue::Integer(i) => i.hash(state),
            UserDataValue::Float(f) => f.to_bits().hash(state),
            UserDataValue::Boolean(b) => b.hash(state),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct UserData {
    pub fields: std::collections::HashMap<StringId, UserDataValue>,
}

impl UserData {
    pub fn new() -> Self {
        Self {
            fields: std::collections::HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: StringId, value: UserDataValue) {
        self.fields.insert(key, value);
    }

    pub fn get(&self, key: StringId) -> Option<&UserDataValue> {
        self.fields.get(&key)
    }

    pub fn remove(&mut self, key: StringId) -> Option<UserDataValue> {
        self.fields.remove(&key)
    }
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct NodeData {
    pub user_data: UserData,
}

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct EdgeData {
    pub user_data: UserData,
}

/// Doubly-linked tree in SoA — O(1) reparenting and deletion.
#[derive(Debug, Clone)]
pub struct Hierarchy {
    pub parent: DenseStorage<Option<NodeId>>,
    pub first_child: DenseStorage<Option<NodeId>>,
    pub next_sibling: DenseStorage<Option<NodeId>>,
    pub prev_sibling: DenseStorage<Option<NodeId>>,
}

impl Hierarchy {
    pub fn new() -> Self {
        Self {
            parent: DenseStorage::new(),
            first_child: DenseStorage::new(),
            next_sibling: DenseStorage::new(),
            prev_sibling: DenseStorage::new(),
        }
    }

    pub fn insert(&mut self) -> usize {
        self.parent.insert(None);
        self.first_child.insert(None);
        self.next_sibling.insert(None);
        self.prev_sibling.insert(None)
    }

    pub fn remove(&mut self, idx: usize) {
        self.parent.remove(idx);
        self.first_child.remove(idx);
        self.next_sibling.remove(idx);
        self.prev_sibling.remove(idx);
    }
}

impl Default for Hierarchy {
    fn default() -> Self {
        Self::new()
    }
}

/// Specialized selection store — manages primary & secondary node selection and edge selection.
#[derive(Debug, Clone)]
pub struct SelectionStore {
    bits: BitVec,
    primary_node: Option<NodeId>,
    secondary_node: Option<NodeId>,
    selected_edge: Option<usize>,
}

impl SelectionStore {
    pub fn new() -> Self {
        Self {
            bits: BitVec::new(),
            primary_node: None,
            secondary_node: None,
            selected_edge: None,
        }
    }

    pub fn insert(&mut self) -> usize {
        let idx = self.bits.len();
        self.bits.push(false);
        idx
    }

    pub fn remove(&mut self, idx: usize) -> bool {
        assert!(
            idx < self.bits.len(),
            "SelectionStore::remove index out of bounds: idx={}, len={}",
            idx,
            self.bits.len()
        );
        let last = self.bits.len() - 1;
        if idx != last {
            let last_val = self.bits[last];
            self.bits.set(idx, last_val);
        }
        self.bits.pop().unwrap()
    }

    pub fn select_node(&mut self, id: NodeId, node_keys: &SlotMap<NodeId, usize>) {
        self.selected_edge = None;
        match (self.primary_node, self.secondary_node) {
            (None, _) => {
                self.primary_node = Some(id);
                self.secondary_node = None;
            }
            (Some(p), None) => {
                if p == id {
                    self.primary_node = Some(id);
                    self.secondary_node = None;
                } else {
                    self.primary_node = Some(p);
                    self.secondary_node = Some(id);
                }
            }
            (Some(p), Some(s)) => {
                if id == p {
                    self.primary_node = Some(id);
                    self.secondary_node = None;
                } else if id == s {
                    self.primary_node = Some(s);
                    self.secondary_node = None;
                } else {
                    self.primary_node = Some(s);
                    self.secondary_node = Some(id);
                }
            }
        }
        self.update_bits(node_keys);
    }

    pub fn select_edge(&mut self, edge_idx: usize) {
        self.primary_node = None;
        self.secondary_node = None;
        self.selected_edge = Some(edge_idx);
        self.bits.fill(false);
    }

    pub fn clear(&mut self) {
        self.primary_node = None;
        self.secondary_node = None;
        self.selected_edge = None;
        self.bits.fill(false);
    }

    pub fn primary_node(&self) -> Option<NodeId> {
        self.primary_node
    }

    pub fn secondary_node(&self) -> Option<NodeId> {
        self.secondary_node
    }

    pub fn selected_edge(&self) -> Option<usize> {
        self.selected_edge
    }

    pub fn is_primary(&self, id: NodeId) -> bool {
        self.primary_node == Some(id)
    }

    pub fn is_secondary(&self, id: NodeId) -> bool {
        self.secondary_node == Some(id)
    }

    fn update_bits(&mut self, node_keys: &SlotMap<NodeId, usize>) {
        self.bits.fill(false);
        if let Some(p) = self.primary_node {
            if let Some(&idx) = node_keys.get(p) {
                if idx < self.bits.len() {
                    self.bits.set(idx, true);
                }
            }
        }
        if let Some(s) = self.secondary_node {
            if let Some(&idx) = node_keys.get(s) {
                if idx < self.bits.len() {
                    self.bits.set(idx, true);
                }
            }
        }
    }

    pub fn get(&self, idx: usize) -> bool {
        if idx < self.bits.len() {
            self.bits[idx]
        } else {
            false
        }
    }

    pub fn set(&mut self, idx: usize, value: bool) {
        if idx < self.bits.len() {
            self.bits.set(idx, value);
        }
    }

    pub fn len(&self) -> usize {
        self.bits.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bits.is_empty()
    }
}

impl Default for SelectionStore {
    fn default() -> Self {
        Self::new()
    }
}

bitflags! {
    /// Bitfield tracking which subsystems need rebuilding next frame
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct DirtyFlags: u8 {
        const POSITION_DIRTY  = 1 << 0;
        const TOPOLOGY_DIRTY  = 1 << 1;
        const STYLE_DIRTY     = 1 << 2;
        const SIZE_DIRTY      = 1 << 3;
        const CONTENT_DIRTY   = 1 << 4;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyledProperty {
    BorderWidth,
    LabelFontSize,
}

#[derive(Clone, Debug)]
pub enum AnimationTrack {
    Position {
        from: Vec2,
        to: Vec2,
        duration: Duration,
        elapsed: Duration,
    },
    Style {
        property: StyledProperty,
        from: f64,
        to: f64,
        duration: Duration,
        elapsed: Duration,
    },
}

#[derive(Debug, Clone, Default)]
pub struct AnimationRegistry {
    pub tracks: SecondaryMap<NodeId, AnimationTrack>,
}

impl AnimationRegistry {
    pub fn new() -> Self {
        Self {
            tracks: SecondaryMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum GraphEvent<S> {
    NodeAdded { id: NodeId },
    EdgeAdded { id: EdgeId, source: NodeId, target: NodeId },
    NodeRemoved { id: NodeId, old_pos: Vec2 },
    EdgeRemoved { id: EdgeId, source: NodeId, target: NodeId },
    PositionChanged { id: NodeId, old_pos: Vec2, new_pos: Vec2 },
    StyleChanged { id: NodeId, old_style: S, new_style: S },
}

pub const MAX_EVENT_LOG_LENGTH: usize = 1000;
