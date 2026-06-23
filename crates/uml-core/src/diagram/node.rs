//! Visual node types for diagrams.

use serde::{Deserialize, Serialize};

use super::geometry::Rect;
use crate::id::UmlId;

/// A visual node on a diagram — the geometric representation of a model element.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewNode {
    /// The model element this node represents.
    pub model_element_id: UmlId,
    /// Position and size on the diagram canvas.
    pub bounds: Rect,
    /// Z-order for layering (higher = on top).
    pub z_order: i32,
    /// Whether this node is visible.
    pub visible: bool,
}

impl ViewNode {
    /// Create a new view node with default properties.
    #[must_use]
    pub fn new(model_element_id: UmlId, bounds: Rect) -> Self {
        Self {
            model_element_id,
            bounds,
            z_order: 0,
            visible: true,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn view_node_defaults() {
        let node = ViewNode::new(UmlId::new(), Rect::new(0.0, 0.0, 100.0, 50.0));
        assert!(node.visible);
        assert_eq!(node.z_order, 0);
        assert_eq!(node.bounds.width(), 100.0);
    }
}
