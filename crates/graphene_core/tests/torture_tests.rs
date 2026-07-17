use graphene_core::{EdgeData, GraphState, Size2, Vec2, MAX_EVENT_LOG_LENGTH};
use std::collections::HashSet;

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (self.state >> 32) as u32
    }

    fn next_range(&mut self, min: usize, max: usize) -> usize {
        let diff = max - min;
        if diff == 0 {
            return min;
        }
        min + (self.next_u32() as usize % diff)
    }
}

#[test]
fn test_phase1_torture_suite() {
    let mut state = GraphState::<()>::new();
    let mut rng = SimpleRng::new(42);

    let num_nodes = 10000;
    let num_edges = 15000;
    let num_deletions = 5000;

    let mut nodes = Vec::new();

    // 1. Insert 10,000 nodes with parent hierarchy
    for i in 0..num_nodes {
        let pos = Vec2::new(i as f32, (i * 2) as f32);
        let size = Size2::new(10.0, 10.0);
        let id = state.add_node(pos, size);
        nodes.push(id);

        // Periodically establish hierarchical relationships (e.g. tree)
        if i > 0 && rng.next_range(0, 10) < 3 {
            let parent_idx = rng.next_range(0, i);
            let parent_id = nodes[parent_idx];
            state.reparent_node(id, Some(parent_id));
        }
    }

    // 2. Insert 15,000 edges
    for _ in 0..num_edges {
        let u_idx = rng.next_range(0, num_nodes);
        let v_idx = rng.next_range(0, num_nodes);
        let u = nodes[u_idx];
        let v = nodes[v_idx];
        state.add_edge(u, v, EdgeData::default());
    }

    // 3. Randomly delete 5,000 nodes
    let mut deleted = HashSet::new();
    for _ in 0..num_deletions {
        // Pick a non-deleted node
        let mut idx;
        loop {
            idx = rng.next_range(0, num_nodes);
            if !deleted.contains(&nodes[idx]) {
                break;
            }
        }
        let id = nodes[idx];
        state.remove_node(id);
        deleted.insert(id);
    }

    // 4. Assert index_to_id.len() == DenseStorage length for all parallel storages
    let expected_len = num_nodes - num_deletions;
    assert_eq!(state.node_index_to_id.len(), expected_len);
    assert_eq!(state.positions.len(), expected_len);
    assert_eq!(state.sizes.len(), expected_len);
    assert_eq!(state.nodes.len(), expected_len);
    assert_eq!(state.hierarchy.parent.len(), expected_len);
    assert_eq!(state.hierarchy.first_child.len(), expected_len);
    assert_eq!(state.hierarchy.next_sibling.len(), expected_len);
    assert_eq!(state.hierarchy.prev_sibling.len(), expected_len);
    assert_eq!(state.selected.len(), expected_len);
    assert_eq!(state.computed_styles.len(), expected_len);

    // 5. Assert iterating over index_to_id yields valid, dense data with no holes
    for (idx, &id) in state.node_index_to_id.iter().enumerate() {
        assert!(state.node_keys.contains_key(id), "Conductor mapping has holes");
        assert_eq!(state.node_keys[id], idx, "Reverse mapping mismatch");
        // Verify index access is valid
        let _pos = state.positions.get(idx);
        let _size = state.sizes.get(idx);
    }

    // 6. Assert hierarchy pointers remain valid and acyclic
    for (idx, &id) in state.node_index_to_id.iter().enumerate() {
        // Verify parent points to a valid, active node
        if let Some(parent_id) = *state.hierarchy.parent.get(idx) {
            assert!(state.node_keys.contains_key(parent_id), "Parent points to deleted node");
            // Check for cycle via simple traversal upwards
            let mut current = parent_id;
            let mut visited = HashSet::new();
            visited.insert(id);
            while let Some(p_id) = state.node_keys.get(current).and_then(|&c_idx| {
                *state.hierarchy.parent.get(c_idx)
            }) {
                assert!(visited.insert(p_id), "Hierarchy contains a parent cycle!");
                current = p_id;
            }
        }

        // Verify siblings
        if let Some(next_sib_id) = *state.hierarchy.next_sibling.get(idx) {
            assert!(state.node_keys.contains_key(next_sib_id), "Next sibling points to deleted node");
            // Verify next sibling's prev_sibling is either Some(id) or points back to id
            let next_sib_idx = state.node_keys[next_sib_id];
            let prev_of_next = *state.hierarchy.prev_sibling.get(next_sib_idx);
            assert_eq!(prev_of_next, Some(id), "Sibling backlink mismatch");
        }

        if let Some(prev_sib_id) = *state.hierarchy.prev_sibling.get(idx) {
            assert!(state.node_keys.contains_key(prev_sib_id), "Prev sibling points to deleted node");
            let prev_sib_idx = state.node_keys[prev_sib_id];
            let next_of_prev = *state.hierarchy.next_sibling.get(prev_sib_idx);
            assert_eq!(next_of_prev, Some(id), "Sibling forwardlink mismatch");
        }
    }

    // 7. Assert event_log never exceeds MAX_EVENT_LOG_LENGTH
    assert!(state.event_log.len() <= MAX_EVENT_LOG_LENGTH, "Event log exceeded capacity: {}", state.event_log.len());
}
