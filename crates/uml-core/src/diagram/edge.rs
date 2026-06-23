//! Visual edge types for diagrams.

use serde::{Deserialize, Serialize};

use super::geometry::Point;
use crate::id::UmlId;

/// Line routing style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineRouting {
    /// Direct straight line between endpoints.
    Direct,
    /// Orthogonal (right-angle) routing.
    Orthogonal,
    /// Polyline with intermediate waypoints.
    Polyline,
    /// Smooth spline curve.
    Spline,
}

/// Kind of edge label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeLabelKind {
    /// The relationship name.
    Name,
    /// Multiplicity at the source end.
    SourceMultiplicity,
    /// Multiplicity at the target end.
    TargetMultiplicity,
    /// Role name at the source end.
    SourceRole,
    /// Role name at the target end.
    TargetRole,
}

/// An edge label positioned on a diagram.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeLabel {
    /// Position of the label on the diagram.
    pub position: Point,
    /// The label text.
    pub text: String,
    /// The kind of label.
    pub kind: EdgeLabelKind,
}

/// A visual edge — the geometric representation of a relationship.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewEdge {
    /// The relationship this edge represents (model element UmlId).
    pub relationship_id: UmlId,
    /// The source node's model element ID.
    pub source_node_id: UmlId,
    /// The target node's model element ID.
    pub target_node_id: UmlId,
    /// Waypoints defining the path. If empty, a straight line is drawn.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub waypoints: Vec<Point>,
    /// Line routing style.
    pub routing: LineRouting,
    /// Optional labels.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<EdgeLabel>,
}

impl ViewEdge {
    /// Create a new view edge.
    #[must_use]
    pub fn new(
        relationship_id: UmlId,
        source_node_id: UmlId,
        target_node_id: UmlId,
        routing: LineRouting,
    ) -> Self {
        Self {
            relationship_id,
            source_node_id,
            target_node_id,
            waypoints: Vec::new(),
            routing,
            labels: Vec::new(),
        }
    }
}

/// Unique identifier for an edge within a diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(uuid::Uuid);

impl EdgeId {
    /// Create a new unique edge ID.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for EdgeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EdgeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_edge_creation() {
        let rel_id = UmlId::new();
        let src_id = UmlId::new();
        let tgt_id = UmlId::new();
        let edge = ViewEdge::new(rel_id, src_id, tgt_id, LineRouting::Orthogonal);
        assert_eq!(edge.relationship_id, rel_id);
        assert_eq!(edge.source_node_id, src_id);
        assert_eq!(edge.target_node_id, tgt_id);
        assert_eq!(edge.routing, LineRouting::Orthogonal);
        assert!(edge.waypoints.is_empty());
        assert!(edge.labels.is_empty());
    }

    #[test]
    fn edge_ids_are_unique() {
        let id1 = EdgeId::new();
        let id2 = EdgeId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn edge_label_kinds() {
        assert_ne!(EdgeLabelKind::Name, EdgeLabelKind::SourceRole);
        assert_eq!(EdgeLabelKind::SourceMultiplicity, EdgeLabelKind::SourceMultiplicity);
    }

    #[test]
    fn serde_roundtrip_view_edge() {
        let edge = ViewEdge::new(UmlId::new(), UmlId::new(), UmlId::new(), LineRouting::Spline);
        let json = serde_json::to_string(&edge).unwrap();
        let back: ViewEdge = serde_json::from_str(&json).unwrap();
        assert_eq!(edge, back);
    }
}
