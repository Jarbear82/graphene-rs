use graphene_core::NodeId;
use std::collections::HashSet;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExpansionState {
    pub collapsed_parents: HashSet<NodeId>,
}

impl ExpansionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_collapsed(&self, id: NodeId) -> bool {
        self.collapsed_parents.contains(&id)
    }

    pub fn collapse(&mut self, id: NodeId) -> bool {
        self.collapsed_parents.insert(id)
    }

    pub fn expand(&mut self, id: NodeId) -> bool {
        self.collapsed_parents.remove(&id)
    }

    pub fn toggle(&mut self, id: NodeId) -> bool {
        if self.collapsed_parents.contains(&id) {
            self.collapsed_parents.remove(&id);
            false
        } else {
            self.collapsed_parents.insert(id);
            true
        }
    }

    pub fn clear(&mut self) {
        self.collapsed_parents.clear();
    }
}

impl Deref for ExpansionState {
    type Target = HashSet<NodeId>;

    fn deref(&self) -> &Self::Target {
        &self.collapsed_parents
    }
}

impl DerefMut for ExpansionState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.collapsed_parents
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use graphene_core::{math::{Size2, Vec2}, GraphState};
    use graphene_style::ComputedStyle;

    #[test]
    fn test_expansion_state_toggle() {
        let mut state = ExpansionState::new();
        let mut graph = GraphState::<ComputedStyle>::new();
        let id = graph.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));

        assert!(!state.is_collapsed(id));
        let is_collapsed = state.toggle(id);
        assert!(is_collapsed);
        assert!(state.is_collapsed(id));

        let is_collapsed = state.toggle(id);
        assert!(!is_collapsed);
        assert!(!state.is_collapsed(id));
    }

    #[test]
    fn test_expansion_state_deref() {
        let mut state = ExpansionState::new();
        let mut graph = GraphState::<ComputedStyle>::new();
        let id = graph.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
        state.insert(id);
        assert!(state.contains(&id));
        assert_eq!(state.len(), 1);
    }
}
