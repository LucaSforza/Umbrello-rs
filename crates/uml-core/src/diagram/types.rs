//! Diagram-specific type definitions.
//!
//! Stubbed for Milestone 1.

/// Identifier for a widget within a diagram scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WidgetId(u64);

/// Identifier for an edge (association line) within a diagram scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(u64);
