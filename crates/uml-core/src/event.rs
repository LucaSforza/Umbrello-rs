//! Model change events.
//!
//! Events represent mutations to the UML model. They are consumed by subscribers
//! (command system, undo stack, UI observers) to react to model changes.
//!
//! Stubbed for Milestone 1 — implementation begins in Phase 3.

/// A change to the UML model.
///
/// Each variant represents one type of model mutation. Events are immutable
/// records of what changed.
#[derive(Debug, Clone)]
pub enum ModelEvent {
    /// An object was created.
    ObjectCreated,
    /// An object was removed.
    ObjectRemoved,
    /// An object was renamed.
    ObjectRenamed,
    /// A property of an object changed.
    PropertyChanged,
}
