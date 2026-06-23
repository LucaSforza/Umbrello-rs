//! Undo/redo command system for Umbrello-RS.
//!
//! Implements the Command pattern using a bounded undo stack. Commands are
//! model-only — they operate on `ModelRepository` and have no GUI dependencies.
//! Supports macro grouping for multi-step operations.

/// The undo/redo stack.
#[derive(Debug)]
pub struct UndoStack;

impl UndoStack {
    /// Create a new undo stack with the given maximum depth.
    #[must_use]
    pub fn new(_max_depth: usize) -> Self {
        Self
    }

    /// Returns whether there are commands to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        false
    }

    /// Returns whether there are commands to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        false
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(100)
    }
}
