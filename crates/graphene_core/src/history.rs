use crate::state::GraphState;
use std::collections::VecDeque;

/// Snapshot-based undo/redo manager
#[derive(Debug, Clone)]
pub struct UndoRedoManager<S: Copy + Default> {
    undo_stack: VecDeque<GraphState<S>>,
    redo_stack: Vec<GraphState<S>>,
    max_history: usize,
}

impl<S: Copy + Default> UndoRedoManager<S> {
    pub fn new() -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_history: 100,
        }
    }

    pub fn with_capacity(max_history: usize) -> Self {
        Self {
            undo_stack: VecDeque::new(),
            redo_stack: Vec::new(),
            max_history,
        }
    }

    pub fn record_state(&mut self, state: &GraphState<S>) {
        self.undo_stack.push_back(state.clone());
        if self.undo_stack.len() > self.max_history {
            self.undo_stack.pop_front();
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self, current: &mut GraphState<S>) -> bool {
        if let Some(prev) = self.undo_stack.pop_back() {
            self.redo_stack.push(current.clone());
            *current = prev;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, current: &mut GraphState<S>) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push_back(current.clone());
            *current = next;
            true
        } else {
            false
        }
    }
}

impl<S: Copy + Default> Default for UndoRedoManager<S> {
    fn default() -> Self {
        Self::new()
    }
}
