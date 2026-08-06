pub mod animation;
pub mod hierarchy;
pub mod topology;
pub mod visuals;

pub use animation::GraphAnimation;
pub use hierarchy::HierarchyExt;
pub use topology::GraphTopology;
pub use visuals::GraphVisuals;

use crate::types::*;
use slotmap::SlotMap;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct GraphState<S: Copy = ()> {
    pub node_keys: SlotMap<NodeId, usize>,
    pub node_index_to_id: Vec<NodeId>,

    pub edge_keys: SlotMap<EdgeId, usize>,
    pub edge_index_to_id: Vec<EdgeId>,

    pub nodes: DenseStorage<NodeData>,
    pub node_kinds: DenseStorage<NodeKind>,
    pub edges: DenseStorage<EdgeData>,

    pub hierarchy: Hierarchy,
    pub edge_sources: DenseStorage<NodeId>,
    pub edge_targets: DenseStorage<NodeId>,

    pub positions: DenseStorage<Vec2>,
    pub sizes: DenseStorage<Size2>,
    pub selected: SelectionStore,

    pub computed_styles: DenseStorage<S>,
    pub edge_computed_styles: DenseStorage<S>,

    pub dirty_flags: DirtyFlags,
    pub animations: AnimationRegistry,
    pub event_log: VecDeque<GraphEvent<S>>,
    pub string_arena: StringArena,
    pub is_batching: bool,
    pub is_ui_mode: bool,
    pub node_labels: DenseStorage<Option<StringId>>,
    pub cached_node_sizes: DenseStorage<Option<Size2>>,
    pub node_uuids: DenseStorage<String>,
}

impl<S: Copy + Default> GraphState<S> {
    pub fn new() -> Self {
        Self {
            node_keys: SlotMap::with_key(),
            node_index_to_id: Vec::new(),
            edge_keys: SlotMap::with_key(),
            edge_index_to_id: Vec::new(),
            nodes: DenseStorage::new(),
            node_kinds: DenseStorage::new(),
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
            event_log: VecDeque::new(),
            string_arena: StringArena::new(),
            is_batching: false,
            is_ui_mode: true,
            node_labels: DenseStorage::new(),
            cached_node_sizes: DenseStorage::new(),
            node_uuids: DenseStorage::new(),
        }
    }

    pub fn batch<F, R>(&mut self, f: F) -> R
    where
        F: FnOnce(&mut Self) -> R,
    {
        let was_batching = self.is_batching;
        self.is_batching = true;
        let result = f(self);
        self.is_batching = was_batching;
        if !self.is_batching {
            self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY | DirtyFlags::POSITION_DIRTY;
        }
        result
    }

    pub fn add_node(&mut self, pos: Vec2, size: Size2) -> NodeId {
        self.add_node_kind(pos, size, NodeKind::Vertex)
    }

    pub fn add_node_kind(&mut self, pos: Vec2, size: Size2, kind: NodeKind) -> NodeId {
        let idx = self.positions.len();

        let id = self.node_keys.insert(idx);
        self.node_index_to_id.push(id);

        self.positions.insert(pos);
        self.sizes.insert(size);
        self.nodes.insert(NodeData::default());
        self.node_kinds.insert(kind);
        self.hierarchy.insert();
        self.selected.insert();
        self.computed_styles.insert(S::default());
        self.node_labels.insert(None);
        self.cached_node_sizes.insert(Some(size));
        self.node_uuids.insert(uuid::Uuid::new_v4().to_string());

        self.push_event(GraphEvent::NodeAdded { id });
        if !self.is_batching {
            self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY;
        }

        id
    }

    pub fn add_node_with_data(
        &mut self,
        pos: Vec2,
        size: Size2,
        data: NodeData,
    ) -> NodeId {
        let id = self.add_node(pos, size);
        if let Some(&idx) = self.node_keys.get(id) {
            let computed_size = data.compute_expansion_size(size);
            self.nodes.set(idx, data);
            *self.sizes.get_mut(idx) = computed_size;
        }
        id
    }

    pub fn add_edge_with_data(
        &mut self,
        source: NodeId,
        target: NodeId,
        data: EdgeData,
    ) -> EdgeId {
        let id = self.add_edge(source, target, data.clone());
        if let Some(&idx) = self.edge_keys.get(id) {
            self.edges.set(idx, data);
        }
        id
    }

    pub fn node_labels(&self, id: NodeId) -> Option<&Labels> {
        let idx = *self.node_keys.get(id)?;
        Some(&self.nodes.get(idx).labels)
    }

    pub fn node_props(&self, id: NodeId) -> Option<&Properties> {
        let idx = *self.node_keys.get(id)?;
        Some(&self.nodes.get(idx).props)
    }

    pub fn set_node_prop(&mut self, id: NodeId, key: impl Into<CompactString>, val: PropValue) {
        if let Some(&idx) = self.node_keys.get(id) {
            let base_size = (*self.cached_node_sizes.get(idx)).unwrap_or_else(|| *self.sizes.get(idx));
            let node = self.nodes.get_mut(idx);
            node.props.insert(key.into(), val);
            let new_size = node.compute_expansion_size(base_size);
            *self.sizes.get_mut(idx) = new_size;
        }
    }

    pub fn get_node_prop(&self, id: NodeId, key: &str) -> Option<&PropValue> {
        let idx = *self.node_keys.get(id)?;
        self.nodes.get(idx).props.get(key)
    }

    pub fn display_label<'a>(&'a self, id: NodeId) -> Option<&'a str> {
        let idx = *self.node_keys.get(id)?;
        self.nodes.get(idx).display_label()
    }

    pub fn set_node_expansion_mode(&mut self, id: NodeId, mode: DataExpansionMode) {
        if let Some(&idx) = self.node_keys.get(id) {
            let base_size = (*self.cached_node_sizes.get(idx)).unwrap_or_else(|| *self.sizes.get(idx));
            let node = self.nodes.get_mut(idx);
            node.expansion_mode = mode;
            let new_size = node.compute_expansion_size(base_size);
            *self.sizes.get_mut(idx) = new_size;
        }
    }

    pub fn edge_labels(&self, id: EdgeId) -> Option<&Labels> {
        let idx = *self.edge_keys.get(id)?;
        Some(&self.edges.get(idx).labels)
    }

    pub fn edge_props(&self, id: EdgeId) -> Option<&Properties> {
        let idx = *self.edge_keys.get(id)?;
        Some(&self.edges.get(idx).props)
    }

    pub fn set_edge_prop(&mut self, id: EdgeId, key: impl Into<CompactString>, val: PropValue) {
        if let Some(&idx) = self.edge_keys.get(id) {
            self.edges.get_mut(idx).props.insert(key.into(), val);
        }
    }

    pub fn outgoing(&self, id: NodeId) -> Vec<(EdgeId, NodeId)> {
        let mut result = Vec::new();
        for (i, &src) in self.edge_sources.iter().enumerate() {
            let tgt = self.edge_targets[i];
            let edge_id = self.edge_index_to_id[i];
            let dir = self.edges.get(i).direction;
            match dir {
                EdgeDirection::Directed => {
                    if src == id {
                        result.push((edge_id, tgt));
                    }
                }
                EdgeDirection::Reverse => {
                    if tgt == id {
                        result.push((edge_id, src));
                    }
                }
                EdgeDirection::Bidirectional | EdgeDirection::Undirected => {
                    if src == id {
                        result.push((edge_id, tgt));
                    } else if tgt == id {
                        result.push((edge_id, src));
                    }
                }
            }
        }
        result
    }

    pub fn incoming(&self, id: NodeId) -> Vec<(EdgeId, NodeId)> {
        let mut result = Vec::new();
        for (i, &src) in self.edge_sources.iter().enumerate() {
            let tgt = self.edge_targets[i];
            let edge_id = self.edge_index_to_id[i];
            let dir = self.edges.get(i).direction;
            match dir {
                EdgeDirection::Directed => {
                    if tgt == id {
                        result.push((edge_id, src));
                    }
                }
                EdgeDirection::Reverse => {
                    if src == id {
                        result.push((edge_id, tgt));
                    }
                }
                EdgeDirection::Bidirectional | EdgeDirection::Undirected => {
                    if tgt == id {
                        result.push((edge_id, src));
                    } else if src == id {
                        result.push((edge_id, tgt));
                    }
                }
            }
        }
        result
    }

    pub fn add_hyperedge_proxy(&mut self, pos: Vec2, size: Size2, targets: &[NodeId]) -> NodeId {
        let proxy_id = self.add_node_kind(pos, size, NodeKind::HyperedgeProxy);
        for &tgt in targets {
            self.add_edge(proxy_id, tgt, EdgeData::default());
        }
        proxy_id
    }

    pub fn set_node_position(&mut self, id: NodeId, new_pos: Vec2) {
        let Some(&idx) = self.node_keys.get(id) else { return };
        let old_pos = *self.positions.get(idx);

        if old_pos == new_pos {
            return;
        }

        self.positions.set(idx, new_pos);

        self.push_event(GraphEvent::PositionChanged {
            id,
            old_pos,
            new_pos,
        });
        self.dirty_flags |= DirtyFlags::POSITION_DIRTY;
    }

    pub fn translate_node_and_descendants(&mut self, id: NodeId, delta: Vec2) {
        if delta.x == 0.0 && delta.y == 0.0 {
            return;
        }

        let mut stack = vec![id];
        while let Some(curr_id) = stack.pop() {
            if let Some(&idx) = self.node_keys.get(curr_id) {
                let old_pos = *self.positions.get(idx);
                let new_pos = old_pos + delta;
                self.positions.set(idx, new_pos);

                self.push_event(GraphEvent::PositionChanged {
                    id: curr_id,
                    old_pos,
                    new_pos,
                });

                let mut child = *self.hierarchy.first_child.get(idx);
                while let Some(c_id) = child {
                    stack.push(c_id);
                    if let Some(&c_idx) = self.node_keys.get(c_id) {
                        child = *self.hierarchy.next_sibling.get(c_idx);
                    } else {
                        break;
                    }
                }
            }
        }
        self.dirty_flags |= DirtyFlags::POSITION_DIRTY;
    }

    pub fn remove_node(&mut self, id: NodeId) {
        let Some(idx) = self.node_keys.remove(id) else { return };
        let last_idx = self.node_index_to_id.len() - 1;

        let old_pos = *self.positions.get(idx);

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

        self.unlink_from_hierarchy(id, idx);

        if idx != last_idx {
            let displaced_id = self.node_index_to_id[last_idx];
            self.node_keys[displaced_id] = idx;
            self.node_index_to_id[idx] = displaced_id;
        }
        self.node_index_to_id.pop();

        self.positions.remove(idx);
        self.sizes.remove(idx);
        self.nodes.remove(idx);
        self.node_kinds.remove(idx);
        self.hierarchy.remove(idx);
        self.selected.remove(idx);
        self.computed_styles.remove(idx);
        self.node_labels.remove(idx);
        self.cached_node_sizes.remove(idx);
        self.node_uuids.remove(idx);

        self.animations.tracks.remove(id);

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

        self.hierarchy.parent.set(idx, None);
        self.hierarchy.next_sibling.set(idx, None);
        self.hierarchy.prev_sibling.set(idx, None);
    }

    pub fn reparent_node(&mut self, child_id: NodeId, parent_id: Option<NodeId>) {
        let Some(&child_idx) = self.node_keys.get(child_id) else { return };

        if let Some(p_id) = parent_id {
            if p_id == child_id {
                return;
            }
            let mut curr = Some(p_id);
            while let Some(curr_id) = curr {
                if curr_id == child_id {
                    return;
                }
                let Some(&curr_idx) = self.node_keys.get(curr_id) else { break };
                curr = *self.hierarchy.parent.get(curr_idx);
            }
        }

        self.unlink_from_hierarchy(child_id, child_idx);

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

    pub fn push_event(&mut self, event: GraphEvent<S>) {
        match event {
            GraphEvent::PositionChanged { id: new_id, new_pos: incoming_pos, .. }
                if matches!(self.event_log.back(), Some(GraphEvent::PositionChanged { id: last_id, .. }) if last_id == &new_id) =>
            {
                if let Some(GraphEvent::PositionChanged { new_pos: last_pos, .. }) = self.event_log.back_mut() {
                    *last_pos = incoming_pos;
                }
                return;
            }
            _ => {}
        }
        self.event_log.push_back(event);
        if self.event_log.len() > MAX_EVENT_LOG_LENGTH {
            self.event_log.pop_front();
        }
    }

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
            if let (Some(&src_idx), Some(&tgt_idx)) = (self.node_keys.get(src), self.node_keys.get(tgt)) {
                dot.push_str(&format!("  node_{} -> node_{};\n", src_idx, tgt_idx));
            }
        }
        dot.push_str("}\n");
        dot
    }

    #[cfg(feature = "serde")]
    pub fn to_json(&self) -> String {
        crate::serde_impl::to_json(self)
    }

    #[cfg(feature = "serde")]
    pub fn from_json(json: &str) -> Result<Self, String> {
        crate::serde_impl::from_json(json)
    }

    pub fn add_edge_with_policy<Ty: EdgeType, P: InsertPolicy<Ty>>(
        &mut self,
        source: NodeId,
        target: NodeId,
        data: EdgeData,
    ) -> Result<EdgeId, GraphError> {
        P::validate::<S>(self, source, target)?;
        Ok(self.add_edge(source, target, data))
    }

    pub fn set_ui_mode(&mut self, is_ui: bool) {
        self.is_ui_mode = is_ui;
    }

    pub fn is_ui_mode(&self) -> bool {
        self.is_ui_mode
    }

    pub fn set_node_label(&mut self, id: NodeId, label: &str) {
        let Some(&idx) = self.node_keys.get(id) else { return };
        let string_id = self.string_arena.intern(label.to_string());
        self.node_labels.set(idx, Some(string_id));
        self.cached_node_sizes.set(idx, None);
        self.dirty_flags |= DirtyFlags::CONTENT_DIRTY | DirtyFlags::SIZE_DIRTY;
    }

    pub fn get_node_label(&self, id: NodeId) -> Option<&str> {
        let &idx = self.node_keys.get(id)?;
        let string_id = (*self.node_labels.get(idx))?;
        self.string_arena.get(string_id)
    }

    pub fn get_node_uuid(&self, id: NodeId) -> Option<&str> {
        let &idx = self.node_keys.get(id)?;
        Some(self.node_uuids.get(idx).as_str())
    }

    pub fn find_node_by_uuid(&self, uuid_str: &str) -> Option<NodeId> {
        self.node_index_to_id
            .iter()
            .copied()
            .find(|&id| self.get_node_uuid(id) == Some(uuid_str))
    }

    pub fn update_cached_node_size(&mut self, id: NodeId, measured_size: Size2) {
        let Some(&idx) = self.node_keys.get(id) else { return };
        self.cached_node_sizes.set(idx, Some(measured_size));
        if self.is_ui_mode {
            self.sizes.set(idx, measured_size);
            self.dirty_flags |= DirtyFlags::POSITION_DIRTY;
        }
    }

    pub fn get_cached_node_size(&self, id: NodeId) -> Option<Size2> {
        let &idx = self.node_keys.get(id)?;
        *self.cached_node_sizes.get(idx)
    }

    pub fn node_count(&self) -> usize {
        self.node_index_to_id.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edge_index_to_id.len()
    }

    pub fn add_node_with_label(&mut self, pos: Vec2, size: Size2, label: &str) -> NodeId {
        let id = self.add_node(pos, size);
        self.set_node_label(id, label);
        id
    }

    pub fn find_node_by_label(&self, label: &str) -> Option<NodeId> {
        self.node_index_to_id
            .iter()
            .copied()
            .find(|&id| self.get_node_label(id) == Some(label))
    }

    pub fn add_edge_by_label(&mut self, src_label: &str, tgt_label: &str, data: EdgeData) -> Option<EdgeId> {
        let src = self.find_node_by_label(src_label)?;
        let tgt = self.find_node_by_label(tgt_label)?;
        Some(self.add_edge(src, tgt, data))
    }
}

impl<S: Copy + Default> Default for GraphState<S> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_state_basic_operations() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(10.0, 20.0), Size2::new(50.0, 50.0));
        let n2 = state.add_node(Vec2::new(100.0, 200.0), Size2::new(50.0, 50.0));
        let _e1 = state.add_edge(n1, n2, EdgeData::default());

        assert_eq!(state.node_count(), 2);
        assert_eq!(state.edge_count(), 1);
        assert_eq!(state.positions.len(), 2);

        state.reparent_node(n2, Some(n1));
        assert_eq!(*state.hierarchy.parent.get(1), Some(n1));

        state.remove_node(n1);
        assert_eq!(state.node_count(), 1);
        assert_eq!(state.edge_count(), 0);
    }

    #[test]
    fn test_graph_state_json_serialization_roundtrip() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node(Vec2::new(5.0, 15.0), Size2::new(30.0, 30.0));
        let n2 = state.add_node(Vec2::new(45.0, 65.0), Size2::new(30.0, 30.0));
        let _e1 = state.add_edge(n1, n2, EdgeData::default());

        let json = state.to_json();
        let restored = GraphState::<()>::from_json(&json).expect("Deserialization failed");
        assert_eq!(restored.node_count(), 2);
        assert_eq!(restored.edge_count(), 1);
    }

    #[test]
    fn test_graph_state_batch_mutation() {
        let mut state = GraphState::<()>::new();
        assert!(!state.is_batching);

        let created_nodes = state.batch(|s| {
            assert!(s.is_batching);
            let mut nodes = Vec::new();
            for i in 0..100 {
                let id = s.add_node(Vec2::new(i as f32, i as f32), Size2::new(10.0, 10.0));
                nodes.push(id);
            }
            nodes
        });

        assert!(!state.is_batching);
        assert_eq!(created_nodes.len(), 100);
        assert_eq!(state.node_count(), 100);
        assert!(state.dirty_flags.contains(DirtyFlags::TOPOLOGY_DIRTY));
    }

    #[test]
    fn test_ui_mode_and_cached_node_sizes() {
        let mut state = GraphState::<()>::new();
        let id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(50.0, 50.0));

        assert!(state.is_ui_mode());
        state.set_node_label(id, "Test Node Content");
        assert_eq!(state.get_node_label(id), Some("Test Node Content"));
        assert!(state.dirty_flags.contains(DirtyFlags::CONTENT_DIRTY));
        assert!(state.dirty_flags.contains(DirtyFlags::SIZE_DIRTY));
        assert_eq!(state.get_cached_node_size(id), None);

        // Update cached size measured from UI
        let measured = Size2::new(140.0, 45.0);
        state.update_cached_node_size(id, measured);
        assert_eq!(state.get_cached_node_size(id), Some(measured));
        let idx = state.node_keys.get(id).copied().unwrap();
        assert_eq!(*state.sizes.get(idx), measured);

        // In headless mode, updating cache does not overwrite graph layout sizes unless in UI mode
        state.set_ui_mode(false);
        assert!(!state.is_ui_mode());
        let new_measured = Size2::new(200.0, 60.0);
        state.update_cached_node_size(id, new_measured);
        assert_eq!(state.get_cached_node_size(id), Some(new_measured));
        assert_eq!(*state.sizes.get(idx), measured);
    }

    #[test]
    fn test_graph_state_crud_operations() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node_with_label(Vec2::new(0.0, 0.0), Size2::new(40.0, 40.0), "Alpha");
        let n2 = state.add_node_with_label(Vec2::new(100.0, 100.0), Size2::new(40.0, 40.0), "Beta");

        assert_eq!(state.get_node_label(n1), Some("Alpha"));
        assert_eq!(state.find_node_by_label("Beta"), Some(n2));
        assert_eq!(state.find_node_by_label("Gamma"), None);

        let e1 = state.add_edge_by_label("Alpha", "Beta", EdgeData::default());
        assert!(e1.is_some());
        assert_eq!(state.edge_count(), 1);

        state.set_node_label(n1, "AlphaUpdated");
        assert_eq!(state.get_node_label(n1), Some("AlphaUpdated"));

        state.remove_node(n1);
        assert_eq!(state.node_count(), 1);
        assert_eq!(state.get_node_label(n1), None);
    }

    #[test]
    fn test_primary_secondary_selection_and_uuids() {
        let mut state = GraphState::<()>::new();
        let n1 = state.add_node_with_label(Vec2::new(0.0, 0.0), Size2::new(40.0, 40.0), "Node1");
        let n2 = state.add_node_with_label(Vec2::new(10.0, 10.0), Size2::new(40.0, 40.0), "Node2");
        let n3 = state.add_node_with_label(Vec2::new(20.0, 20.0), Size2::new(40.0, 40.0), "Node3");

        let uuid1 = state.get_node_uuid(n1).expect("UUID missing").to_string();
        let uuid2 = state.get_node_uuid(n2).expect("UUID missing").to_string();
        assert_ne!(uuid1, uuid2);
        assert_eq!(state.find_node_by_uuid(&uuid1), Some(n1));
        assert_eq!(state.find_node_by_uuid(&uuid2), Some(n2));

        // When label changes, UUID stays the same
        state.set_node_label(n1, "RenamedLabel");
        assert_eq!(state.get_node_uuid(n1), Some(uuid1.as_str()));

        // Test primary and secondary selection logic
        state.selected.select_node(n1, &state.node_keys);
        assert_eq!(state.selected.primary_node(), Some(n1));
        assert_eq!(state.selected.secondary_node(), None);

        // Second click selects secondary node without clearing primary
        state.selected.select_node(n2, &state.node_keys);
        assert_eq!(state.selected.primary_node(), Some(n1));
        assert_eq!(state.selected.secondary_node(), Some(n2));

        // Third click shifts previous secondary to primary and new click to secondary
        state.selected.select_node(n3, &state.node_keys);
        assert_eq!(state.selected.primary_node(), Some(n2));
        assert_eq!(state.selected.secondary_node(), Some(n3));
    }
}


