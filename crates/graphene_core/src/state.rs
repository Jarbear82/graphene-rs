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
        }
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

        self.push_event(GraphEvent::NodeAdded { id });
        self.dirty_flags |= DirtyFlags::TOPOLOGY_DIRTY;

        id
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

    pub fn to_json(&self) -> String {
        crate::serde_impl::to_json(self)
    }

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
}

impl<S: Copy + Default> Default for GraphState<S> {
    fn default() -> Self {
        Self::new()
    }
}
