use crate::types::*;
use std::collections::HashSet;

use crate::GraphState;
/// Extension trait for hierarchy traversal operations on GraphState
pub trait HierarchyExt<S> {
    /// Check if `child_idx` is a descendant of `parent_idx`
    fn is_ancestor(&self, child_idx: usize, parent_idx: usize) -> bool;

    /// Get all leaf descendants of a node (nodes with no children)
    fn get_leaf_descendants(&self, node_idx: usize) -> Vec<usize>;

    /// Check if a node has any children
    fn is_parent(&self, idx: usize) -> bool;

    /// Calculate nesting depth of a node in the hierarchy
    fn get_nesting_depth(&self, node_id: NodeId) -> usize;

    /// Get the visible representative for a node in collapsed hierarchy
    fn get_visible_representative(
        &self,
        node_id: NodeId,
        collapsed_parents: &HashSet<NodeId>,
    ) -> NodeId;

    /// Get all descendants of a node (children, grandchildren, etc.)
    fn get_all_descendants(&self, node_idx: usize) -> Vec<usize>;
}

impl<S: Copy> HierarchyExt<S> for GraphState<S> {
    /// Check if `child_idx` is a descendant of `parent_idx`
    fn is_ancestor(&self, child_idx: usize, parent_idx: usize) -> bool {
        let parent_id = self.node_index_to_id[parent_idx];
        let mut curr_idx = child_idx;

        while let Some(parent_node_id) = *self.hierarchy.parent.get(curr_idx) {
            if parent_node_id == parent_id {
                return true;
            }
            if let Some(&next_idx) = self.node_keys.get(parent_node_id) {
                curr_idx = next_idx;
            } else {
                break;
            }
        }
        false
    }

    /// Get all leaf descendants of a node (nodes with no children)
    fn get_leaf_descendants(&self, node_idx: usize) -> Vec<usize> {
        let mut leaves = Vec::new();

        if self.hierarchy.first_child.get(node_idx).is_none() {
            // Node has no children, it's already a leaf
            leaves.push(node_idx);
            return leaves;
        }

        let mut stack = vec![node_idx];
        while let Some(curr) = stack.pop() {
            if self.hierarchy.first_child.get(curr).is_none() {
                // Leaf node
                leaves.push(curr);
            } else {
                // Traverse children
                let mut next_child = *self.hierarchy.first_child.get(curr);
                while let Some(child_id) = next_child {
                    if let Some(&child_idx) = self.node_keys.get(child_id) {
                        stack.push(child_idx);
                        next_child = *self.hierarchy.next_sibling.get(child_idx);
                    } else {
                        break;
                    }
                }
            }
        }

        leaves
    }

    /// Check if a node has any children
    fn is_parent(&self, idx: usize) -> bool {
        self.hierarchy.first_child.get(idx).is_some()
    }

    /// Calculate nesting depth of a node in the hierarchy
    fn get_nesting_depth(&self, mut node_id: NodeId) -> usize {
        let mut depth = 0;

        while let Some(&idx) = self.node_keys.get(node_id) {
            if let Some(parent_id) = *self.hierarchy.parent.get(idx) {
                node_id = parent_id;
                depth += 1;
            } else {
                break;
            }
        }

        depth
    }

    /// Get the visible representative for a node in collapsed hierarchy
    fn get_visible_representative(
        &self,
        mut node_id: NodeId,
        collapsed_parents: &HashSet<NodeId>,
    ) -> NodeId {
        let mut rep = node_id;

        while let Some(&idx) = self.node_keys.get(node_id) {
            if let Some(parent_id) = *self.hierarchy.parent.get(idx) {
                if collapsed_parents.contains(&parent_id) {
                    rep = parent_id; // Use collapsed parent as representative
                }
                node_id = parent_id;
            } else {
                break;
            }
        }

        rep
    }

    /// Get all descendants of a node (children, grandchildren, etc.)
    fn get_all_descendants(&self, node_idx: usize) -> Vec<usize> {
        let mut descendants = Vec::new();
        let mut stack = vec![node_idx];

        while let Some(curr) = stack.pop() {
            // Get first child
            if let Some(child_id) = *self.hierarchy.first_child.get(curr) {
                if let Some(&child_idx) = self.node_keys.get(child_id) {
                    descendants.push(child_idx);
                    stack.push(child_idx);

                    // Traverse siblings
                    let mut next_sibling = *self.hierarchy.next_sibling.get(child_idx);
                    while let Some(sib_id) = next_sibling {
                        if let Some(&sib_idx) = self.node_keys.get(sib_id) {
                            descendants.push(sib_idx);
                            stack.push(sib_idx);
                        }
                        if let Some(next_sib_idx) = self.node_keys.get(sib_id) {
                            next_sibling = *self.hierarchy.next_sibling.get(*next_sib_idx);
                        } else {
                            next_sibling = None;
                        }
                    }
                }
            }
        }

        descendants
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{Size2, Vec2};
    use crate::types::*;

    #[test]
    fn test_is_ancestor() {
        let mut state: GraphState<()> = GraphState::new();

        let parent_id = state.add_node(Vec2::default(), Size2::default());
        let child_id = state.add_node(Vec2::default(), Size2::default());
        let grandchild_id = state.add_node(Vec2::default(), Size2::default());

        state.reparent_node(child_id, Some(parent_id));
        state.reparent_node(grandchild_id, Some(child_id));

        let &parent_idx = state.node_keys.get(parent_id).unwrap();
        let &child_idx = state.node_keys.get(child_id).unwrap();
        let &grandchild_idx = state.node_keys.get(grandchild_id).unwrap();

        assert!(state.is_ancestor(child_idx, parent_idx));
        assert!(state.is_ancestor(grandchild_idx, parent_idx));
        assert!(state.is_ancestor(grandchild_idx, child_idx));
        assert!(!state.is_ancestor(parent_idx, child_idx));
    }

    #[test]
    fn test_get_leaf_descendants() {
        let mut state: GraphState<()> = GraphState::new();

        let root_id = state.add_node(Vec2::default(), Size2::default());
        let child1_id = state.add_node(Vec2::default(), Size2::default());
        let child2_id = state.add_node(Vec2::default(), Size2::default());
        let grandchild_id = state.add_node(Vec2::default(), Size2::default());

        state.reparent_node(child1_id, Some(root_id));
        state.reparent_node(child2_id, Some(root_id));
        state.reparent_node(grandchild_id, Some(child1_id));

        let &root_idx = state.node_keys.get(root_id).unwrap();
        let leaves = state.get_leaf_descendants(root_idx);

        assert!(leaves.contains(&state.node_keys.get(child2_id).unwrap().clone()));
        assert!(leaves.contains(&state.node_keys.get(grandchild_id).unwrap().clone()));
    }

    #[test]
    fn test_get_visible_representative() {
        let mut state: GraphState<()> = GraphState::new();

        let parent_id = state.add_node(Vec2::default(), Size2::default());
        let child_id = state.add_node(Vec2::default(), Size2::default());

        state.reparent_node(child_id, Some(parent_id));

        let collapsed_parents: HashSet<NodeId> = HashSet::from([parent_id]);

        // Child's representative should be parent (collapsed)
        assert_eq!(
            state.get_visible_representative(child_id, &collapsed_parents),
            parent_id
        );
    }

    #[test]
    fn test_get_nesting_depth() {
        let mut state: GraphState<()> = GraphState::new();

        let root_id = state.add_node(Vec2::default(), Size2::default());
        let child_id = state.add_node(Vec2::default(), Size2::default());
        let grandchild_id = state.add_node(Vec2::default(), Size2::default());

        state.reparent_node(child_id, Some(root_id));
        state.reparent_node(grandchild_id, Some(child_id));

        assert_eq!(state.get_nesting_depth(root_id), 0);
        assert_eq!(state.get_nesting_depth(child_id), 1);
        assert_eq!(state.get_nesting_depth(grandchild_id), 2);
    }

    #[test]
    fn test_get_all_descendants() {
        let mut state: GraphState<()> = GraphState::new();

        let root_id = state.add_node(Vec2::default(), Size2::default());
        let child1_id = state.add_node(Vec2::default(), Size2::default());
        let child2_id = state.add_node(Vec2::default(), Size2::default());
        let grandchild_id = state.add_node(Vec2::default(), Size2::default());

        state.reparent_node(child1_id, Some(root_id));
        state.reparent_node(child2_id, Some(root_id));
        state.reparent_node(grandchild_id, Some(child1_id));

        let &root_idx = state.node_keys.get(root_id).unwrap();
        let descendants = state.get_all_descendants(root_idx);

        assert_eq!(descendants.len(), 3);
        assert!(descendants.contains(&state.node_keys.get(child1_id).unwrap().clone()));
        assert!(descendants.contains(&state.node_keys.get(child2_id).unwrap().clone()));
        assert!(descendants.contains(&state.node_keys.get(grandchild_id).unwrap().clone()));
    }
}
