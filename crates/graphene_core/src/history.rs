use crate::state::GraphState;

/// Snapshot-based undo/redo manager
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

impl<S: Copy + Default> Default for UndoRedoManager<S> {
    fn default() -> Self {
        Self::new()
    }
}
