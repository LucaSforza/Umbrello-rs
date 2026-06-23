//! Diagram model for Umbrello-RS.
//!
//! Provides pure data structures for diagram composition: widget positions and
//! sizes, scene state, association routing, without any rendering logic.
//! Separated from `uml-render` to allow CLI tooling to work with diagrams
//! without GPU/windowing dependencies.

pub mod types;

/// A diagram scene — a collection of widgets and associations.
#[derive(Debug, Default)]
pub struct SceneData {
    /// Placeholder — widgets and associations will be added in Phase 16.
    _placeholder: (),
}
