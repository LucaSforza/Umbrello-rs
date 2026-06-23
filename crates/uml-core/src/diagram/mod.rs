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

    /// Return the numeric type number used in Umbrello XMI files.
    #[must_use]
    pub fn type_num(self) -> i32 {
        match self {
            Self::Class => 1,
            Self::UseCase => 2,
            Self::Sequence => 3,
            Self::Collaboration => 4,
            Self::State => 5,
            Self::Activity => 6,
            Self::Component => 7,
            Self::Deployment => 8,
            Self::EntityRelationship => 9,
            Self::Object => 10,
        }
    }

    /// Create a `DiagramKind` from a numeric type number
    /// (as stored in Umbrello XMI files).
    /// Returns `DiagramKind::Class` for unknown/0 values.
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn from_type_num(n: i32) -> Self {
        match n {
            1 => Self::Class,
            2 => Self::UseCase,
            3 => Self::Sequence,
            4 => Self::Collaboration,
            5 => Self::State,
            6 => Self::Activity,
            7 => Self::Component,
            8 => Self::Deployment,
            9 => Self::EntityRelationship,
            10 => Self::Object,
            _ => Self::Class,
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

    #[test]
    fn diagram_kind_type_num_roundtrip() {
        let kinds = [
            DiagramKind::Class,
            DiagramKind::UseCase,
            DiagramKind::Sequence,
            DiagramKind::Collaboration,
            DiagramKind::State,
            DiagramKind::Activity,
            DiagramKind::Component,
            DiagramKind::Deployment,
            DiagramKind::EntityRelationship,
            DiagramKind::Object,
        ];
        for kind in &kinds {
            let num = kind.type_num();
            let restored = DiagramKind::from_type_num(num);
            assert_eq!(*kind, restored, "roundtrip failed for {kind:?} via num {num}");
        }
    }

    #[test]
    fn diagram_kind_type_num_values() {
        assert_eq!(DiagramKind::Class.type_num(), 1);
        assert_eq!(DiagramKind::UseCase.type_num(), 2);
        assert_eq!(DiagramKind::Sequence.type_num(), 3);
        assert_eq!(DiagramKind::Collaboration.type_num(), 4);
        assert_eq!(DiagramKind::State.type_num(), 5);
        assert_eq!(DiagramKind::Activity.type_num(), 6);
        assert_eq!(DiagramKind::Component.type_num(), 7);
        assert_eq!(DiagramKind::Deployment.type_num(), 8);
        assert_eq!(DiagramKind::EntityRelationship.type_num(), 9);
        assert_eq!(DiagramKind::Object.type_num(), 10);
    }

    #[test]
    fn diagram_kind_from_type_num_defaults() {
        assert_eq!(DiagramKind::from_type_num(0), DiagramKind::Class);
        assert_eq!(DiagramKind::from_type_num(99), DiagramKind::Class);
        assert_eq!(DiagramKind::from_type_num(-1), DiagramKind::Class);
    }

    #[test]
    fn diagram_kind_all_from_type_num() {
        assert_eq!(DiagramKind::from_type_num(1), DiagramKind::Class);
        assert_eq!(DiagramKind::from_type_num(2), DiagramKind::UseCase);
        assert_eq!(DiagramKind::from_type_num(3), DiagramKind::Sequence);
        assert_eq!(DiagramKind::from_type_num(4), DiagramKind::Collaboration);
        assert_eq!(DiagramKind::from_type_num(5), DiagramKind::State);
        assert_eq!(DiagramKind::from_type_num(6), DiagramKind::Activity);
        assert_eq!(DiagramKind::from_type_num(7), DiagramKind::Component);
        assert_eq!(DiagramKind::from_type_num(8), DiagramKind::Deployment);
        assert_eq!(DiagramKind::from_type_num(9), DiagramKind::EntityRelationship);
        assert_eq!(DiagramKind::from_type_num(10), DiagramKind::Object);
    }
}
