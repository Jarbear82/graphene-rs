use crate::types::*;

#[derive(Debug, Clone)]
pub struct GraphAnimation {
    pub animations: AnimationRegistry,
}

impl Default for GraphAnimation {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphAnimation {
    pub fn new() -> Self {
        Self {
            animations: AnimationRegistry::new(),
        }
    }
}
