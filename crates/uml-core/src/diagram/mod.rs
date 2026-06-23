//! Diagram model for Umbrello-RS.
//!
//! Provides pure data structures for diagram composition: node positions and
//! sizes, edge routing, scene state, without any rendering logic.
//! Separated from rendering to allow CLI tooling to work with diagrams
//! without GPU/windowing dependencies.

pub mod edge;
pub mod geometry;
pub mod node;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::id::UmlId;
use indexmap::IndexMap;

pub use edge::{EdgeId, EdgeLabel, EdgeLabelKind, LineRouting, ViewEdge};
pub use geometry::{Point, Rect, Size};
pub use node::ViewNode;

/// Unique identifier for a diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagramId(Uuid);

impl DiagramId {
    /// Create a new unique diagram ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for DiagramId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for DiagramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Kinds of diagrams supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramKind {
    /// Class diagram.
    Class,
    /// Use case diagram.
    UseCase,
    /// Sequence diagram.
    Sequence,
    /// Collaboration diagram.
    Collaboration,
    /// State diagram.
    State,
    /// Activity diagram.
    Activity,
    /// Component diagram.
    Component,
    /// Deployment diagram.
    Deployment,
    /// Entity-relationship diagram.
    EntityRelationship,
    /// Object diagram.
    Object,
}

impl DiagramKind {
    /// Return the string representation of this diagram kind.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Class => "Class",
            Self::UseCase => "UseCase",
            Self::Sequence => "Sequence",
            Self::Collaboration => "Collaboration",
            Self::State => "State",
            Self::Activity => "Activity",
            Self::Component => "Component",
            Self::Deployment => "Deployment",
            Self::EntityRelationship => "EntityRelationship",
            Self::Object => "Object",
        }
    }
}

/// A diagram — a visual container for nodes and edges.
///
/// Each diagram has a type, metadata, and collections of ViewNodes and
/// ViewEdges. Diagrams are owned by UmlModel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    /// Unique identifier for this diagram.
    pub id: DiagramId,
    /// Human-readable name.
    pub name: String,
    /// The type of diagram.
    pub kind: DiagramKind,
    /// Visual nodes keyed by the model element's UmlId.
    pub nodes: IndexMap<UmlId, ViewNode>,
    /// Visual edges keyed by EdgeId.
    pub edges: IndexMap<EdgeId, ViewEdge>,
}

impl Diagram {
    /// Create a new diagram.
    #[must_use]
    pub fn new(name: impl Into<String>, kind: DiagramKind) -> Self {
        Self {
            id: DiagramId::new(),
            name: name.into(),
            kind,
            nodes: IndexMap::new(),
            edges: IndexMap::new(),
        }
    }

    /// Add a node to the diagram.
    pub fn add_node(&mut self, element_id: UmlId, node: ViewNode) {
        self.nodes.insert(element_id, node);
    }

    /// Remove a node by model element ID.
    pub fn remove_node(&mut self, element_id: UmlId) -> Option<ViewNode> {
        self.nodes.shift_remove(&element_id)
    }

    /// Get a node by model element ID.
    #[must_use]
    pub fn get_node(&self, element_id: UmlId) -> Option<&ViewNode> {
        self.nodes.get(&element_id)
    }

    /// Get a mutable node by model element ID.
    pub fn get_node_mut(&mut self, element_id: UmlId) -> Option<&mut ViewNode> {
        self.nodes.get_mut(&element_id)
    }

    /// Add an edge to the diagram.
    pub fn add_edge(&mut self, edge_id: EdgeId, edge: ViewEdge) {
        self.edges.insert(edge_id, edge);
    }

    /// Remove an edge by ID.
    pub fn remove_edge(&mut self, edge_id: EdgeId) -> Option<ViewEdge> {
        self.edges.shift_remove(&edge_id)
    }

    /// Number of nodes in the diagram.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges in the diagram.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn create_diagram() {
        let d = Diagram::new("Main", DiagramKind::Class);
        assert_eq!(d.name, "Main");
        assert_eq!(d.kind, DiagramKind::Class);
        assert_eq!(d.node_count(), 0);
        assert_eq!(d.edge_count(), 0);
    }

    #[test]
    fn add_and_remove_node() {
        let mut d = Diagram::new("Test", DiagramKind::Class);
        let elem_id = UmlId::new();
        let node = ViewNode::new(elem_id, Rect::new(10.0, 20.0, 100.0, 50.0));
        d.add_node(elem_id, node);
        assert_eq!(d.node_count(), 1);

        let removed = d.remove_node(elem_id);
        assert!(removed.is_some());
        assert_eq!(d.node_count(), 0);
    }

    #[test]
    fn diagram_ids_are_unique() {
        let id1 = DiagramId::new();
        let id2 = DiagramId::new();
        assert_ne!(id1, id2);
    }
}
