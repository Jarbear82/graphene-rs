use graphene_core::{GraphState, Vec2, Size2, EdgeData, AnimationTrack, UndoRedoManager};
use std::time::Duration;

#[test]
fn test_animation_driving() {
    let mut state: GraphState<()> = GraphState::new();
    let id = state.add_node(Vec2::new(0.0, 0.0), Size2::new(10.0, 10.0));
    
    // Add animation track for position from (0,0) to (100, 100) over 300ms
    state.animations.tracks.insert(id, AnimationTrack::Position {
        from: Vec2::new(0.0, 0.0),
        to: Vec2::new(100.0, 100.0),
        duration: Duration::from_millis(300),
        elapsed: Duration::ZERO,
    });

    // Tick 150ms (halfway)
    state.tick_animations(Duration::from_millis(150));
    let idx = state.node_keys[id];
    let pos = *state.positions.get(idx);
    assert_eq!(pos.x, 50.0);
    assert_eq!(pos.y, 50.0);
    assert!(!state.animations.tracks.is_empty());

    // Tick another 150ms (finished)
    state.tick_animations(Duration::from_millis(150));
    let pos_end = *state.positions.get(idx);
    assert_eq!(pos_end.x, 100.0);
    assert_eq!(pos_end.y, 100.0);
    assert!(state.animations.tracks.is_empty());
}

#[test]
fn test_undo_redo_manager() {
    let mut state: GraphState<()> = GraphState::new();
    let mut manager = UndoRedoManager::new();

    // 1. Initial State: no nodes
    assert_eq!(state.node_index_to_id.len(), 0);

    // Record state before adding a node
    manager.record_state(&state);
    let id = state.add_node(Vec2::new(10.0, 20.0), Size2::new(10.0, 10.0));
    assert_eq!(state.node_index_to_id.len(), 1);

    // Record state before moving the node
    manager.record_state(&state);
    state.set_node_position(id, Vec2::new(50.0, 60.0));
    let idx = state.node_keys[id];
    assert_eq!(*state.positions.get(idx), Vec2::new(50.0, 60.0));

    // 2. Undo move
    let undone = manager.undo(&mut state);
    assert!(undone);
    let idx_undone = state.node_keys[id];
    assert_eq!(*state.positions.get(idx_undone), Vec2::new(10.0, 20.0));

    // 3. Undo addition
    let undone_add = manager.undo(&mut state);
    assert!(undone_add);
    assert_eq!(state.node_index_to_id.len(), 0);

    // 4. Redo addition
    let redone_add = manager.redo(&mut state);
    assert!(redone_add);
    assert_eq!(state.node_index_to_id.len(), 1);
    
    // 5. Redo move
    let redone_move = manager.redo(&mut state);
    assert!(redone_move);
    let id_redone = state.node_index_to_id[0];
    let idx_redone = state.node_keys[id_redone];
    assert_eq!(*state.positions.get(idx_redone), Vec2::new(50.0, 60.0));
}

#[test]
fn test_json_dot_serialization() {
    let mut state: GraphState<()> = GraphState::new();
    let n1 = state.add_node(Vec2::new(10.0, 10.0), Size2::new(20.0, 20.0));
    let n2 = state.add_node(Vec2::new(100.0, 100.0), Size2::new(30.0, 30.0));
    state.add_edge(n1, n2, EdgeData::default());

    // Serialize to DOT
    let dot = state.to_dot();
    assert!(dot.contains("digraph G {"));
    assert!(dot.contains("node_0"));
    assert!(dot.contains("node_1"));
    assert!(dot.contains("node_0 -> node_1"));

    // Serialize to JSON
    let json = state.to_json();
    assert!(json.contains("nodes"));
    assert!(json.contains("edges"));

    // Deserialize from JSON
    let restored: GraphState<()> = GraphState::from_json(&json).unwrap();
    assert_eq!(restored.node_index_to_id.len(), 2);
    assert_eq!(restored.edges.len(), 1);

    let pos1 = restored.positions[0];
    let pos2 = restored.positions[1];
    assert_eq!(pos1.x, 10.0);
    assert_eq!(pos1.y, 10.0);
    assert_eq!(pos2.x, 100.0);
    assert_eq!(pos2.y, 100.0);
}
