//! Undo/redo command system for Umbrello-RS.
//!
//! Implements the Command pattern using a bounded undo stack. Commands are
//! model-only — they operate on `UmlModel` and have no GUI dependencies.
//! Supports macro grouping for multi-step operations.

pub mod commands;

pub use commands::CreateEdge;

use std::fmt::Debug;

use crate::id::UmlId;
use crate::repository::{ModelError, UmlModel};

/// A reversible operation on a UmlModel.
///
/// Each command encapsulates both the forward operation (`execute`) and its
/// reverse (`undo`). Commands are stored on the `History` stack and can be
/// replayed via `History::undo()` / `History::redo()`.
pub trait Command: Debug + Send {
    /// Execute the command. Called once when the user performs the action.
    ///
    /// # Errors
    ///
    /// Returns `CommandError` if the command cannot be applied to the model
    /// (e.g., element not found, duplicate ID, invalid operation).
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;

    /// Undo the command. Reverse the effect of `execute()`.
    ///
    /// # Errors
    ///
    /// Returns `CommandError` if the undo cannot be applied (e.g., element
    /// was modified externally, invalid model state).
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;

    /// Human-readable description of this command.
    fn description(&self) -> &str;

    /// Optionally merge this command with a subsequent command of the same type.
    fn merge(&self, _other: &dyn Command) -> Option<Box<dyn Command>> {
        None
    }
}

/// Errors that can occur during command execution.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// An element was not found.
    #[error("element not found: {0}")]
    ElementNotFound(UmlId),

    /// The operation is invalid in the current model state.
    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    /// A model-level error occurred.
    #[error("model error: {0}")]
    Model(#[from] ModelError),
}

/// Manages undo/redo history for UmlModel mutations.
///
/// All model mutations should go through `History::execute()` to ensure
/// they are tracked for undo/redo. Direct mutation of `UmlModel` bypasses
/// the history.
#[derive(Debug)]
pub struct History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_depth: usize,
    /// If true, commands are executed but not pushed onto the stack.
    /// Used during file loading where undo is meaningless.
    disabled: bool,
}

impl History {
    /// Create a new history manager with the given maximum undo depth.
    #[must_use]
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_depth,
            disabled: false,
        }
    }

    /// Execute a command and push it onto the undo stack.
    ///
    /// Clears the redo stack — a new action invalidates redo history.
    /// If the history is disabled, the command is executed but not pushed.
    ///
    /// # Errors
    ///
    /// Returns `CommandError` if the command's `execute()` call fails.
    pub fn execute(
        &mut self,
        mut cmd: Box<dyn Command>,
        model: &mut UmlModel,
    ) -> Result<(), CommandError> {
        cmd.execute(model)?;
        if !self.disabled {
            self.redo_stack.clear();
            self.undo_stack.push(cmd);
            // Trim to max_depth
            while self.undo_stack.len() > self.max_depth {
                self.undo_stack.remove(0);
            }
        }
        Ok(())
    }

    /// Undo the most recent command.
    ///
    /// The undone command is moved to the redo stack.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::InvalidOperation` if there is nothing to undo,
    /// or propagates the command's own undo error.
    pub fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let mut cmd = self
            .undo_stack
            .pop()
            .ok_or_else(|| CommandError::InvalidOperation("nothing to undo".into()))?;
        cmd.undo(model)?;
        self.redo_stack.push(cmd);
        Ok(())
    }

    /// Redo the most recently undone command.
    ///
    /// The redone command is moved back to the undo stack.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::InvalidOperation` if there is nothing to redo,
    /// or propagates the command's own execute error.
    pub fn redo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let mut cmd = self
            .redo_stack
            .pop()
            .ok_or_else(|| CommandError::InvalidOperation("nothing to redo".into()))?;
        cmd.execute(model)?;
        self.undo_stack.push(cmd);
        Ok(())
    }

    /// Returns `true` if there are commands to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Returns `true` if there are commands to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Description of the command that would be undone.
    #[must_use]
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.description())
    }

    /// Description of the command that would be redone.
    #[must_use]
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.description())
    }

    /// Disable history recording. Commands are still executed but not tracked.
    /// Use this during file loading.
    pub fn set_disabled(&mut self, disabled: bool) {
        self.disabled = disabled;
    }

    /// Returns `true` if history recording is disabled.
    #[must_use]
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Clear all undo/redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    /// Number of commands in the undo stack.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of commands in the redo stack.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }
}

impl Default for History {
    fn default() -> Self {
        Self::new(100)
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::needless_pass_by_value)]
mod tests {
    use super::*;
    use crate::elements::{Class, ModelElement};

    /// Helper to get the element id before handing it to a command.
    fn capture_id_and_create(elem: ModelElement) -> (Box<dyn Command>, UmlId) {
        let id = elem.id();
        (Box::new(commands::CreateElement::new(elem)), id)
    }

    #[test]
    fn history_starts_empty() {
        let h = History::new(100);
        assert!(!h.can_undo());
        assert!(!h.can_redo());
        assert_eq!(h.undo_depth(), 0);
    }

    #[test]
    fn execute_pushes_to_undo() {
        let mut model = UmlModel::new();
        let mut history = History::new(100);

        let (cmd, _id) = capture_id_and_create(ModelElement::Class(Class::new("Test")));
        history.execute(cmd, &mut model).unwrap();

        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(model.len(), 1);
    }

    #[test]
    fn undo_removes_element() {
        let mut model = UmlModel::new();
        let mut history = History::new(100);

        let (cmd, _id) = capture_id_and_create(ModelElement::Class(Class::new("Test")));
        history.execute(cmd, &mut model).unwrap();
        assert_eq!(model.len(), 1);

        history.undo(&mut model).unwrap();
        assert_eq!(model.len(), 0);
        assert!(!history.can_undo());
        assert!(history.can_redo());
    }

    #[test]
    fn redo_restores_element() {
        let mut model = UmlModel::new();
        let mut history = History::new(100);

        let (cmd, _id) = capture_id_and_create(ModelElement::Class(Class::new("Test")));
        history.execute(cmd, &mut model).unwrap();
        history.undo(&mut model).unwrap();
        history.redo(&mut model).unwrap();

        assert_eq!(model.len(), 1);
        assert!(history.can_undo());
        assert!(!history.can_redo());
    }

    #[test]
    fn new_action_clears_redo() {
        let mut model = UmlModel::new();
        let mut history = History::new(100);

        let (cmd_a, _id_a) = capture_id_and_create(ModelElement::Class(Class::new("A")));
        history.execute(cmd_a, &mut model).unwrap();
        history.undo(&mut model).unwrap();
        assert!(history.can_redo());

        // New action clears redo
        let (cmd_b, _id_b) = capture_id_and_create(ModelElement::Class(Class::new("B")));
        history.execute(cmd_b, &mut model).unwrap();
        assert!(!history.can_redo());
    }

    #[test]
    fn max_depth_trims_oldest() {
        let mut model = UmlModel::new();
        let mut history = History::new(3);

        for i in 0..5 {
            let (cmd, _id) =
                capture_id_and_create(ModelElement::Class(Class::new(format!("Class{i}"))));
            history.execute(cmd, &mut model).unwrap();
        }

        // Only 3 most recent commands remain
        assert_eq!(history.undo_depth(), 3);
        // Undo all 3
        for _ in 0..3 {
            history.undo(&mut model).unwrap();
        }
        // But 5 elements were created, so 2 remain
        assert_eq!(model.len(), 2);
    }

    #[test]
    fn disabled_mode() {
        let mut model = UmlModel::new();
        let mut history = History::new(100);
        history.set_disabled(true);

        let (cmd, _id) = capture_id_and_create(ModelElement::Class(Class::new("Test")));
        history.execute(cmd, &mut model).unwrap();

        assert_eq!(model.len(), 1);
        assert!(!history.can_undo()); // not tracked
    }
}
