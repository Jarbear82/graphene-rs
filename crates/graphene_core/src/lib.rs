pub mod math;
pub mod fixtures;

use bitflags::bitflags;
use bitvec::vec::BitVec;
use slotmap::{new_key_type, SecondaryMap, SlotMap};
use std::time::Duration;

pub use math::{Size2, Vec2};

new_key_type! {
    pub struct NodeId;
    pub struct EdgeId;
}

/// Safe wrapper around a parallel array. Operates purely on `usize` indices.
/// **Key invariant:** All DenseStorage instances in GraphState share the same length.
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

    /// Swap-and-pop deletion. Caller must guarantee `idx` is valid.
    pub fn remove(&mut self, idx: usize) -> T {
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

    /// Direct mutation — O(1), no swap-and-pop. Critical for position updates.
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
}

impl StringArena {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    pub fn intern(&mut self, s: String) -> StringId {
        if let Some(pos) = self.strings.iter().position(|x| x == &s) {
            pos as u32
        } else {
            let id = self.strings.len() as u32;
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

/// Specialized selection store — a `DenseStorage<bool>` wrapper around BitVec.
#[derive(Debug, Clone)]
pub struct SelectionStore {
    bits: BitVec,
}

impl SelectionStore {
    pub fn new() -> Self {
        Self { bits: BitVec::new() }
    }

    pub fn insert(&mut self) -> usize {
        let idx = self.bits.len();
        self.bits.push(false);
        idx
    }

    pub fn remove(&mut self, idx: usize) -> bool {
        let last = self.bits.len() - 1;
        if idx != last {
            let last_val = self.bits[last];
            self.bits.set(idx, last_val);
        }
        self.bits.pop().unwrap()
    }

    pub fn get(&self, idx: usize) -> bool {
        self.bits[idx]
    }

    pub fn set(&mut self, idx: usize, value: bool) {
        self.bits.set(idx, value);
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
        const POSITION_DIRTY  = 1 << 0;   // node positions changed → redraw edges
        const TOPOLOGY_DIRTY  = 1 << 1;   // nodes/edges added or removed
        const STYLE_DIRTY     = 1 << 2;   // styles updated → recompute ComputedStyle
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

// === Animation Registry (Phase 1 Fix: O(1) cleanup via SecondaryMap) ===
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

/// Append-only event log for undo/redo — with **coalescing** to prevent memory explosion.
#[derive(Debug, Clone)]
pub enum GraphEvent<S> {
    NodeAdded { id: NodeId },
    EdgeAdded { id: EdgeId, source: NodeId, target: NodeId },
    NodeRemoved { id: NodeId, old_pos: Vec2 },
    EdgeRemoved { id: EdgeId, source: NodeId, target: NodeId },
    PositionChanged { id: NodeId, old_pos: Vec2, new_pos: Vec2 },
    StyleChanged { id: NodeId, old_style: S, new_style: S },
}

/// Maximum undo stack depth. Acts as a ring buffer capacity.
pub const MAX_EVENT_LOG_LENGTH: usize = 1000;

/// GraphState acts as the "conductor" — it owns the SlotMap + reverse index
/// and coordinates all operations across parallel DenseStorage instances.
#[derive(Debug, Clone)]
pub struct GraphState<S: Copy = ()> {
    // === THE CONDUCTOR: Sole source of truth for NodeId ↔ dense index mapping ===
    pub node_keys: SlotMap<NodeId, usize>,      // NodeId → dense index
    pub node_index_to_id: Vec<NodeId>,          // dense index → NodeId (reverse map)

    // === THE CONDUCTOR: Sole source of truth for EdgeId ↔ dense index mapping ===
    pub edge_keys: SlotMap<EdgeId, usize>,      // EdgeId → dense index
    pub edge_index_to_id: Vec<EdgeId>,          // dense index → EdgeId (reverse map)

    // === ENTITY DATA (DenseStorage-backed) ===
    pub nodes: DenseStorage<NodeData>,
    pub edges: DenseStorage<EdgeData>,

    // === TOPOLOGY ===
    pub hierarchy: Hierarchy,
    pub edge_sources: DenseStorage<NodeId>,
    pub edge_targets: DenseStorage<NodeId>,

    // === GEOMETRY & STATE (DenseStorage — index-only, conductor-coordinated) ===
    pub positions: DenseStorage<Vec2>,
    pub sizes: DenseStorage<Size2>,
    pub selected: SelectionStore,

    // === COMPUTED PRESENTATION ===
    pub computed_styles: DenseStorage<S>,
    pub edge_computed_styles: DenseStorage<S>,

    // === DIRTY FLAGS / INVALIDATION ===
    pub dirty_flags: DirtyFlags,

    // === ANIMATION (slotmap-backed for O(1) cleanup) ===
    pub animations: AnimationRegistry,

    // === EVENT LOG (with coalescing) ===
    pub event_log: Vec<GraphEvent<S>>,

    // === PRESENTATION STRINGS ===
    pub string_arena: StringArena,
}

impl<S: Copy + Default> GraphState<S> {
    pub fn new() -> Self {
        Self {
            node_keys: SlotMap::with_key(),
            node_index_to_id: Vec::new(),
            edge_keys: SlotMap::with_key(),
            edge_index_to_id: Vec::new(),
            nodes: DenseStorage::new(),
            edges: DenseStorage::new(),
            hierarchy: Hierarchy::new(),
            edge_sources: DenseStorage::new(),
            edge_targets: DenseStorage::new(),
            positions: DenseStorage::new(),
            sizes: DenseStorage::new(),
            selected: SelectionStore::new(),
            computed_styles: DenseStorage::new(),
            edge_computed_styles: DenseStorage::new(),
            dirty_flags: DirtyFlags::empty(),
            animations: AnimationRegistry::new(),
            event_log: Vec::new(),
            string_arena: StringArena::new(),
        }
    }

    // === NODE LIFECYCLE (Conductor Pattern) ===

    pub fn add_node(&mut self, pos: Vec2, size: Size2) -> NodeId {
        // 1. All arrays are currently the same length — this is our new dense index.
        let idx = self.positions.len();

        // 2. Conductor registers the new ID
        let id = self.node_keys.insert(idx);
        self.node_index_to_id.push(id);

        // 3. Insert into all parallel storages
        self.positions.insert(pos);
        self.sizes.insert(size);
        self.nodes.insert(NodeData::default());
        self.hierarchy.insert();
        self.selected.insert();
        self.computed_styles.insert(S::default());

        self.push_event(GraphEvent::NodeAdded { id });
        self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY;

        id
    }

    /// O(1) Direct Mutation. No swap-and-pop thrashing.
    pub fn set_node_position(&mut self, id: NodeId, new_pos: Vec2) {
        let Some(&idx) = self.node_keys.get(id) else { return };
        let old_pos = *self.positions.get(idx);

        if old_pos == new_pos {
            return;
        }

        // Direct mutation — no array permutation, no cache thrashing
        self.positions.set(idx, new_pos);

        self.push_event(GraphEvent::PositionChanged {
            id,
            old_pos,
            new_pos,
        });
        self.dirty_flags |= DirtyFlags::POSITION_DIRTY;
    }

    /// Swap-and-pop deletion — coordinated across all parallel DenseStorage instances.
    pub fn remove_node(&mut self, id: NodeId) {
        let Some(idx) = self.node_keys.remove(id) else { return };
        let last_idx = self.node_index_to_id.len() - 1;

        // Retrieve old position BEFORE any removals (needed for event)
        let old_pos = *self.positions.get(idx);

        // Dissolve child relationships to bypass this node
        let mut curr_child = *self.hierarchy.first_child.get(idx);
        while let Some(child_id) = curr_child {
            if let Some(&child_idx) = self.node_keys.get(child_id) {
                let next_child = *self.hierarchy.next_sibling.get(child_idx);
                self.hierarchy.parent.set(child_idx, None);
                self.hierarchy.next_sibling.set(child_idx, None);
                self.hierarchy.prev_sibling.set(child_idx, None);
                curr_child = next_child;
            } else {
                break;
            }
        }

        // Unlink from hierarchy
        self.unlink_from_hierarchy(id, idx);

        // Update conductor's reverse mapping if we are not removing the last element
        if idx != last_idx {
            let displaced_id = self.node_index_to_id[last_idx];
            self.node_keys[displaced_id] = idx;
            self.node_index_to_id[idx] = displaced_id;
        }
        self.node_index_to_id.pop();

        // Remove from ALL parallel storages using swap-and-pop
        self.positions.remove(idx);
        self.sizes.remove(idx);
        self.nodes.remove(idx);
        self.hierarchy.remove(idx);
        self.selected.remove(idx);
        self.computed_styles.remove(idx);

        // Clean up animation tracks (slotmap O(1))
        self.animations.tracks.remove(id);

        // Remove edges connected to this node
        let mut edges_to_remove = Vec::new();
        for (i, &src) in self.edge_sources.iter().enumerate() {
            let tgt = self.edge_targets[i];
            if src == id || tgt == id {
                edges_to_remove.push(self.edge_index_to_id[i]);
            }
        }
        for edge_id in edges_to_remove {
            self.remove_edge(edge_id);
        }

        self.push_event(GraphEvent::NodeRemoved { id, old_pos });
        self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY;
    }

    fn unlink_from_hierarchy(&mut self, _id: NodeId, idx: usize) {
        let parent = *self.hierarchy.parent.get(idx);
        let prev = *self.hierarchy.prev_sibling.get(idx);
        let next = *self.hierarchy.next_sibling.get(idx);

        if let Some(prev_sib_id) = prev {
            if let Some(&prev_idx) = self.node_keys.get(prev_sib_id) {
                self.hierarchy.next_sibling.set(prev_idx, next);
            }
        } else if let Some(p_id) = parent {
            if let Some(&p_idx) = self.node_keys.get(p_id) {
                self.hierarchy.first_child.set(p_idx, next);
            }
        }

        if let Some(next_sib_id) = next {
            if let Some(&next_idx) = self.node_keys.get(next_sib_id) {
                self.hierarchy.prev_sibling.set(next_idx, prev);
            }
        }

        // Clear the node's own hierarchy pointers
        self.hierarchy.parent.set(idx, None);
        self.hierarchy.next_sibling.set(idx, None);
        self.hierarchy.prev_sibling.set(idx, None);
    }

    pub fn reparent_node(&mut self, child_id: NodeId, parent_id: Option<NodeId>) {
        let Some(&child_idx) = self.node_keys.get(child_id) else { return };
        
        // 1. Unlink child from current parent
        self.unlink_from_hierarchy(child_id, child_idx);

        // 2. Link to new parent
        if let Some(p_id) = parent_id {
            let Some(&p_idx) = self.node_keys.get(p_id) else { return };
            
            self.hierarchy.parent.set(child_idx, Some(p_id));
            let old_first = *self.hierarchy.first_child.get(p_idx);
            
            self.hierarchy.next_sibling.set(child_idx, old_first);
            self.hierarchy.prev_sibling.set(child_idx, None);
            
            if let Some(old_first_id) = old_first {
                if let Some(&old_first_idx) = self.node_keys.get(old_first_id) {
                    self.hierarchy.prev_sibling.set(old_first_idx, Some(child_id));
                }
            }
            
            self.hierarchy.first_child.set(p_idx, Some(child_id));
        } else {
            self.hierarchy.parent.set(child_idx, None);
            self.hierarchy.next_sibling.set(child_idx, None);
            self.hierarchy.prev_sibling.set(child_idx, None);
        }

        self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY;
    }

    // === EDGE LIFECYCLE ===

    pub fn add_edge(&mut self, source: NodeId, target: NodeId, data: EdgeData) -> EdgeId {
        let idx = self.edges.len();
        let id = self.edge_keys.insert(idx);
        self.edge_index_to_id.push(id);

        self.edges.insert(data);
        self.edge_sources.insert(source);
        self.edge_targets.insert(target);
        self.edge_computed_styles.insert(S::default());

        self.push_event(GraphEvent::EdgeAdded { id, source, target });
        self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY;

        id
    }

    pub fn remove_edge(&mut self, id: EdgeId) {
        let Some(idx) = self.edge_keys.remove(id) else { return };
        let last_idx = self.edge_index_to_id.len() - 1;

        let source = *self.edge_sources.get(idx);
        let target = *self.edge_targets.get(idx);

        if idx != last_idx {
            let displaced_id = self.edge_index_to_id[last_idx];
            self.edge_keys[displaced_id] = idx;
            self.edge_index_to_id[idx] = displaced_id;
        }
        self.edge_index_to_id.pop();

        self.edges.remove(idx);
        self.edge_sources.remove(idx);
        self.edge_targets.remove(idx);
        self.edge_computed_styles.remove(idx);

        self.push_event(GraphEvent::EdgeRemoved { id, source, target });
        self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY;
    }

    /// Push an event with automatic coalescing and bounded capacity.
    pub fn push_event(&mut self, event: GraphEvent<S>) {
        match event {
            GraphEvent::PositionChanged { id: new_id, new_pos: incoming_pos, .. }
                if matches!(self.event_log.last(), Some(GraphEvent::PositionChanged { id: last_id, .. }) if last_id == &new_id) =>
            {
                if let Some(GraphEvent::PositionChanged { new_pos: last_pos, .. }) = self.event_log.last_mut() {
                    *last_pos = incoming_pos;
                }
                return;
            }
            _ => {}
        }
        self.event_log.push(event);
        if self.event_log.len() > MAX_EVENT_LOG_LENGTH {
            self.event_log.remove(0);
        }
    }
}

impl<S: Copy + Default> Default for GraphState<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Copy + Default> GraphState<S> {
    pub fn tick_animations(&mut self, dt: std::time::Duration) {
        let mut completed = Vec::new();
        for (node_id, track) in self.animations.tracks.iter_mut() {
            let Some(&idx) = self.node_keys.get(node_id) else { continue; };
            match track {
                AnimationTrack::Position { from, to, duration, elapsed } => {
                    *elapsed += dt;
                    let progress = if duration.is_zero() {
                        1.0
                    } else {
                        (elapsed.as_secs_f32() / duration.as_secs_f32()).min(1.0)
                    };
                    let current = *from * (1.0 - progress) + *to * progress;
                    self.positions.set(idx, current);
                    self.dirty_flags |= DirtyFlags::POSITION_DIRTY;
                    if progress >= 1.0 {
                        completed.push(node_id);
                    }
                }
                _ => {}
            }
        }
        for node_id in completed {
            self.animations.tracks.remove(node_id);
        }
    }

    pub fn to_dot(&self) -> String {
        let mut dot = String::new();
        dot.push_str("digraph G {\n");
        for idx in 0..self.node_index_to_id.len() {
            dot.push_str(&format!("  node_{} [label=\"Node {}\"];\n", idx, idx));
        }
        for idx in 0..self.edges.len() {
            let src = self.edge_sources[idx];
            let tgt = self.edge_targets[idx];
            let src_idx = self.node_keys[src];
            let tgt_idx = self.node_keys[tgt];
            dot.push_str(&format!("  node_{} -> node_{};\n", src_idx, tgt_idx));
        }
        dot.push_str("}\n");
        dot
    }

    pub fn to_json(&self) -> String {
        let serialized = SerializedGraph {
            nodes: (0..self.node_index_to_id.len()).map(|idx| {
                let pos = self.positions[idx];
                let size = self.sizes[idx];
                SerializedNode { x: pos.x, y: pos.y, w: size.w, h: size.h }
            }).collect(),
            edges: (0..self.edges.len()).map(|idx| {
                let src = self.edge_sources[idx];
                let tgt = self.edge_targets[idx];
                SerializedEdge {
                    source_idx: self.node_keys[src],
                    target_idx: self.node_keys[tgt],
                }
            }).collect(),
        };
        serde_json::to_string_pretty(&serialized).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        let serialized: SerializedGraph = serde_json::from_str(json)
            .map_err(|e| e.to_string())?;
        
        let mut state = GraphState::new();
        let mut node_ids = Vec::new();
        for n in serialized.nodes {
            let id = state.add_node(Vec2::new(n.x, n.y), Size2::new(n.w, n.h));
            node_ids.push(id);
        }
        for e in serialized.edges {
            if e.source_idx < node_ids.len() && e.target_idx < node_ids.len() {
                state.add_edge(node_ids[e.source_idx], node_ids[e.target_idx], EdgeData::default());
            }
        }
        Ok(state)
    }
}

// Snapshot-based undo/redo manager
#[derive(Debug, Clone)]
pub struct UndoRedoManager<S: Copy + Default> {
    undo_stack: Vec<GraphState<S>>,
    redo_stack: Vec<GraphState<S>>,
}

impl<S: Copy + Default> UndoRedoManager<S> {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn record_state(&mut self, state: &GraphState<S>) {
        self.undo_stack.push(state.clone());
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: &mut GraphState<S>) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(current.clone());
            *current = prev;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current: &mut GraphState<S>) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(current.clone());
            *current = next;
            true
        } else {
            false
        }
    }
}

// JSON Serialization types
#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedNode {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedEdge {
    pub source_idx: usize,
    pub target_idx: usize,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct SerializedGraph {
    pub nodes: Vec<SerializedNode>,
    pub edges: Vec<SerializedEdge>,
}

