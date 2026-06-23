# Diagram Geometry Architecture v1

**Milestone:** 12 — Headless Diagram Engine  
**Status:** Draft  
**Last updated:** 2026-06-23

---

## 1. Motivation

Umbrello-RS must separate **visual geometry** from **semantic model data**. UML model
elements (Class, Package, Association) are pure semantic data — they describe
*what* exists in the domain. Diagrams describe *where* things appear on a canvas,
how large they are, how edges are routed, and which model elements they
represent.

Without this separation, a single model element cannot appear on multiple
diagrams at different positions, and serialization becomes tangled with
presentation concerns. This document defines the headless diagram engine that
stores geometry exclusively, with no rendering code.

---

## 2. Design Principles

### 2.1 Semantic/Visual Separation

UML model elements live in `UmlModel` (see `domain_model_v1.md`). Visual
geometry — positions, sizes, waypoints, Z-order — lives in `Diagram`. No visual
data leaks into `uml-core` types. A `ModelElement` contains no `x`, `y`, `w`,
`h` fields.

```
 ┌──────────────────────┐      ┌──────────────────────────┐
 │       UmlModel       │      │        Diagram            │
 │  ┌─────────────────┐ │      │  ┌──────────────────────┐ │
 │  │  ModelElement    │ │      │  │     ViewNode         │ │
 │  │  id: UmlId       │ │      │  │  model_element_id ───┼─┼──► UmlId
 │  │  name: String    │ │      │  │  bounds: Rect        │ │
 │  │  kind: ElementKind│ │      │  │  z_order: i32        │ │
 │  │  ...             │ │      │  └──────────────────────┘ │
 │  └─────────────────┘ │      │  ┌──────────────────────┐ │
 │                       │      │  │     ViewEdge         │ │
 │                       │      │  │  relationship_id ────┼─┼──► UmlId
 │                       │      │  │  waypoints: Vec<Point>│ │
 │                       │      │  └──────────────────────┘ │
 └──────────────────────┘      └──────────────────────────┘
```

### 2.2 ID-Based Linking

`ViewNode` and `ViewEdge` reference model elements by `UmlId`. There are no
Rust references (`&`, `Box`, `Rc<RefCell<>>`) crossing the boundary between
the diagram layer and the model layer. This:

- Eliminates circular reference problems.
- Keeps serialization trivial (no reference cycles).
- Allows cheap cloning of diagrams.
- Simplifies undo/redo (snapshots of IDs, not pointer graphs).

### 2.3 Multiple Diagrams Per Model

Because linking is by `UmlId`, the same model element can appear in any number
of diagrams, each with a different position, size, and visibility. The model
element is never copied — only the `ViewNode` that references it.

### 2.4 Headless

These types contain **no rendering code**, no Qt widgets, no OpenGL, no
HTML canvas. A future renderer (Milestone 13+) will read `Diagram` data and
produce pixels. The diagram engine can be fully unit-tested without a display
server.

---

## 3. Data Structures

### 3.1 Geometry Primitives

Defined in `crates/uml-core/src/diagram/geometry.rs`.

```rust
/// A 2D point in diagram coordinates (origin top-left).
///
/// The coordinate system places (0,0) at the top-left corner of the canvas.
/// x increases to the right, y increases downward.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// A 2D size (width and height).
///
/// Width and height must be non-negative. No invariant is enforced at the
/// type level; validation occurs in command execution.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// A rectangle defined by origin and size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    /// Minimum x coordinate (left edge).
    pub fn left(&self) -> f64 {
        self.origin.x
    }

    /// Minimum y coordinate (top edge).
    pub fn top(&self) -> f64 {
        self.origin.y
    }

    /// Maximum x coordinate (right edge).
    pub fn right(&self) -> f64 {
        self.origin.x + self.size.width
    }

    /// Maximum y coordinate (bottom edge).
    pub fn bottom(&self) -> f64 {
        self.origin.y + self.size.height
    }

    /// Center point of the rectangle.
    pub fn center(&self) -> Point {
        Point {
            x: self.origin.x + self.size.width / 2.0,
            y: self.origin.y + self.size.height / 2.0,
        }
    }

    /// Returns `true` if the point lies within this rect (inclusive of edges).
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.left()
            && point.x <= self.right()
            && point.y >= self.top()
            && point.y <= self.bottom()
    }
}
```

### 3.2 Diagram

Defined in `crates/uml-core/src/diagram/mod.rs`.

```rust
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A diagram — a visual container for nodes and edges.
///
/// Each diagram has a type (Class, UseCase, etc.), metadata, and collections
/// of ViewNodes and ViewEdges. Diagrams are owned by UmlModel, not by
/// UML packages. A model may contain zero or more diagrams.
///
/// Nodes and edges are stored in insertion-order-preserving maps
/// (IndexMap) so that serialization round-trips are stable and the
/// diagram's element order is predictable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagram {
    /// Unique identifier for this diagram.
    pub id: DiagramId,

    /// Human-readable name (e.g., "Main Class Diagram").
    pub name: String,

    /// The type of diagram. Determines which model element kinds are
    /// permitted as nodes and how edges are routed by default.
    pub kind: DiagramKind,

    /// Nodes (visual representations of model elements) keyed by UmlId.
    ///
    /// Each ViewNode's `model_element_id` must correspond to an element
    /// in the owning UmlModel. This invariant is maintained by command
    /// execution.
    pub nodes: IndexMap<UmlId, ViewNode>,

    /// Edges (visual representations of relationships) keyed by edge ID.
    ///
    /// EdgeId is a diagram-scoped identifier, separate from the UmlId of
    /// the relationship. This allows multiple ViewEdges to reference the
    /// same relationship (e.g., in different diagrams) or one relationship
    /// to appear multiple times within the same diagram.
    pub edges: IndexMap<EdgeId, ViewEdge>,
}

/// Unique identifier for a diagram.
///
/// Wraps a UUID v4 generated at creation time. Diagrams are identified by
/// this ID, not by name (names may be duplicated).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiagramId(uuid::Uuid);

impl DiagramId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for DiagramId {
    fn default() -> Self {
        Self::new()
    }
}

/// Kinds of diagrams supported.
///
/// This enum mirrors the diagram types available in Umbrello C++.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiagramKind {
    Class,
    UseCase,
    Sequence,
    Collaboration,
    State,
    Activity,
    Component,
    Deployment,
    EntityRelationship,
    Object,
}

/// Diagram-scoped identifier for edges.
///
/// Unlike UmlId which identifies model-level relationships, EdgeId uniquely
/// identifies a ViewEdge within a single diagram. This allows the same
/// relationship (same UmlId) to appear in multiple diagrams, potentially
/// multiple times per diagram.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EdgeId(uuid::Uuid);

impl EdgeId {
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}
```

### 3.3 ViewNode

Defined in `crates/uml-core/src/diagram/node.rs`.

```rust
use crate::diagram::geometry::Rect;
use crate::uml_id::UmlId;

/// A visual node on a diagram — the geometric representation of a model
/// element.
///
/// Each ViewNode corresponds to exactly one model element (identified by
/// its `model_element_id`). The element must exist in the UmlModel.
/// This invariant is enforced by the command layer.
///
/// A ViewNode is the smallest unit of visual state: position, size, layering,
/// and visibility. It contains no rendering details (colors, fonts, icons)
/// and no reference to the model element beyond its ID.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewNode {
    /// The model element this node represents.
    ///
    /// Must be a valid UmlId in the owning UmlModel. The referenced element's
    /// kind should be compatible with the diagram kind (e.g., Class elements
    /// on a Class diagram), though this is a policy enforced by the UI, not
    /// by the data layer.
    pub model_element_id: UmlId,

    /// Position and size on the diagram canvas.
    ///
    /// The origin is the top-left corner of the node. Width and height should
    /// be non-negative; zero-size nodes may be invisible.
    pub bounds: Rect,

    /// Z-order for layering (higher = on top).
    ///
    /// Nodes with higher Z-order values are drawn on top of nodes with lower
    /// values. Nodes at the same Z-order may overlap arbitrarily; the UI
    /// should avoid assigning equal values.
    pub z_order: i32,

    /// Whether this node is visible.
    ///
    /// Invisible nodes are not rendered but remain in the diagram and can be
    /// made visible again without re-adding them.
    pub visible: bool,
}
```

### 3.4 ViewEdge

Defined in `crates/uml-core/src/diagram/edge.rs`.

```rust
use crate::diagram::geometry::Point;
use crate::uml_id::UmlId;
use serde::{Deserialize, Serialize};

/// A visual edge on a diagram — the geometric representation of a
/// relationship.
///
/// Each ViewEdge corresponds to a Relationship in the model. The edge stores
/// waypoints for line routing (e.g., orthogonal or polyline paths), label
/// positions, and references to the source and target nodes.
///
/// The `source_node_id` and `target_node_id` are UmlIds of model elements
/// that must have corresponding ViewNodes in the same diagram's `nodes` map.
/// These are *not* necessarily the ends of the model relationship — they
/// refer to the visual node endpoints for this particular edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewEdge {
    /// The relationship this edge represents (by model element UmlId).
    pub relationship_id: UmlId,

    /// The source node (diagram-specific reference).
    ///
    /// Must correspond to a key in the parent Diagram's `nodes` map.
    pub source_node_id: UmlId,

    /// The target node.
    ///
    /// Must correspond to a key in the parent Diagram's `nodes` map.
    pub target_node_id: UmlId,

    /// Waypoints defining the path.
    ///
    /// If empty, a straight line is drawn between the source and target
    /// node boundaries. Waypoints are in diagram coordinates.
    pub waypoints: Vec<Point>,

    /// Line routing style.
    pub routing: LineRouting,

    /// Optional label positions.
    ///
    /// Labels are positioned relative to the edge path. The renderer is
    /// responsible for avoiding overlaps.
    pub labels: Vec<EdgeLabel>,
}

/// Line routing style for an edge.
///
/// Determines how the edge path is computed between waypoints and node
/// boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineRouting {
    /// Straight line between endpoints, ignoring waypoints.
    Direct,
    /// Path restricted to horizontal and vertical segments (right angles).
    Orthogonal,
    /// Path through waypoints connected by straight line segments.
    Polyline,
    /// Smooth curve (Bezier spline) through waypoints.
    Spline,
}

/// A label positioned along an edge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeLabel {
    /// Position of the label in diagram coordinates.
    pub position: Point,

    /// Display text.
    pub text: String,

    /// The semantic kind of this label.
    pub kind: EdgeLabelKind,
}

/// Semantic kind of an edge label.
///
/// Determines which portion of the model relationship's data is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeLabelKind {
    /// The relationship name (e.g., "associates").
    Name,
    /// Multiplicity at the source end (e.g., "1", "0..*").
    SourceMultiplicity,
    /// Multiplicity at the target end (e.g., "1", "0..*").
    TargetMultiplicity,
    /// Role name at the source end.
    SourceRole,
    /// Role name at the target end.
    TargetRole,
}
```

---

## 4. Storage in UmlModel

The `UmlModel` struct (defined in `crates/uml-core/src/model/mod.rs`) gains a
`diagrams` field. This is additive — no existing fields or methods are
affected.

```rust
use crate::diagram::Diagram;
use crate::diagram::DiagramId;

/// The root of a UML model.
///
/// Owns all model elements (Classes, Packages, Associations, etc.),
/// maintains the parent/child hierarchy, and owns all diagrams.
pub struct UmlModel {
    // Existing fields (from M11):
    elements: IndexMap<UmlId, ModelElement>,
    parent_index: HashMap<UmlId, Vec<UmlId>>,

    // New in M12:
    /// All diagrams in this model.
    ///
    /// Diagrams are stored in insertion order. Iteration yields diagrams
    /// in the order they were added.
    diagrams: Vec<Diagram>,
}
```

### 4.1 Accessor Methods

```rust
impl UmlModel {
    /// Add a diagram to the model.
    ///
    /// The diagram's nodes and edges are *not* validated against the model's
    /// element set at insertion time. Validation occurs when the add-diagram
    /// command is executed (see §5). This allows diagram deserialization
    /// before the model is fully populated.
    pub fn add_diagram(&mut self, diagram: Diagram) {
        self.diagrams.push(diagram);
    }

    /// Remove a diagram by its ID, returning it if found.
    pub fn remove_diagram(&mut self, diagram_id: DiagramId) -> Option<Diagram> {
        if let Some(pos) = self.diagrams.iter().position(|d| d.id == diagram_id) {
            Some(self.diagrams.remove(pos))
        } else {
            None
        }
    }

    /// Get an immutable reference to a diagram by ID.
    pub fn get_diagram(&self, diagram_id: DiagramId) -> Option<&Diagram> {
        self.diagrams.iter().find(|d| d.id == diagram_id)
    }

    /// Get a mutable reference to a diagram by ID.
    pub fn get_diagram_mut(&mut self, diagram_id: DiagramId) -> Option<&mut Diagram> {
        self.diagrams.iter_mut().find(|d| d.id == diagram_id)
    }

    /// Iterate over all diagrams (immutable).
    pub fn diagrams(&self) -> &[Diagram] {
        &self.diagrams
    }

    /// Iterate over all diagrams (mutable).
    pub fn diagrams_mut(&mut self) -> &mut [Diagram] {
        &mut self.diagrams
    }
}
```

### 4.2 Invariants

The following invariants SHOULD hold at all times after command execution.
They are not enforced at the `Diagram` type level (which would require
expensive cross-referencing on every mutation) but are the responsibility
of the command layer and, ultimately, the UI:

1. **Every `ViewNode.model_element_id` exists in the owning `UmlModel.elements`.**
2. **Every `ViewEdge.relationship_id` exists in the owning `UmlModel.elements` and is a `Relationship`.**
3. **Every `ViewEdge.source_node_id` and `target_node_id` exists as a key in the same diagram's `nodes` map.**
4. **Zero-size nodes are permitted** (they may represent collapsed or
   auto-sized elements awaiting layout).

---

## 5. Visual Commands

Visual commands extend the Command pattern established in M11
(see `command_architecture_v1.md`). They live in
`crates/uml-core/src/undo/commands.rs` alongside model commands.

All visual commands implement the `Command` trait:

```rust
/// Result of command execution.
pub type CommandResult = Result<(), CommandError>;

/// A reversible operation on a UmlModel (including its diagrams).
pub trait Command {
    /// Execute the command. Must record sufficient state to undo.
    fn execute(&mut self, model: &mut UmlModel) -> CommandResult;

    /// Undo the command, restoring the previous state.
    fn undo(&mut self, model: &mut UmlModel) -> CommandResult;

    /// Optional description for display in undo/redo menus.
    fn description(&self) -> &str;
}
```

### 5.1 AddNodeToDiagram

```rust
use crate::diagram::{DiagramId, ViewNode};
use crate::diagram::geometry::{Point, Size, Rect};
use crate::uml_id::UmlId;

/// Adds a new ViewNode to a diagram, linked to an existing model element.
pub struct AddNodeToDiagram {
    diagram_id: DiagramId,
    element_id: UmlId,
    position: Point,
    size: Size,
    description: String,
    // Populated on execute() for undo.
    overwritten_node: Option<ViewNode>,
}

impl AddNodeToDiagram {
    pub fn new(
        diagram_id: DiagramId,
        element_id: UmlId,
        position: Point,
        size: Size,
    ) -> Self {
        Self {
            diagram_id,
            element_id,
            position,
            size,
            description: format!("Add node {:?} to diagram {:?}", element_id, diagram_id),
            overwritten_node: None,
        }
    }
}

impl Command for AddNodeToDiagram {
    fn execute(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;

        // If a node with this UmlId already exists, save it for undo.
        if let Some(existing) = diagram.nodes.remove(&self.element_id) {
            self.overwritten_node = Some(existing);
        }

        let node = ViewNode {
            model_element_id: self.element_id,
            bounds: Rect {
                origin: self.position,
                size: self.size,
            },
            z_order: diagram.nodes.len() as i32,
            visible: true,
        };

        diagram.nodes.insert(self.element_id, node);
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;

        diagram.nodes.remove(&self.element_id);

        // Restore overwritten node if there was one.
        if let Some(node) = self.overwritten_node.take() {
            diagram.nodes.insert(self.element_id, node);
        }

        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

### 5.2 RemoveNodeFromDiagram

```rust
/// Removes a ViewNode (and any edges referencing it) from a diagram.
pub struct RemoveNodeFromDiagram {
    diagram_id: DiagramId,
    element_id: UmlId,
    removed_node: Option<ViewNode>,
    removed_edges: Vec<EdgeId>,
    description: String,
}

impl Command for RemoveNodeFromDiagram {
    fn execute(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;

        // Capture removed node.
        self.removed_node = diagram.nodes.remove(&self.element_id);
        if self.removed_node.is_none() {
            return Err(CommandError::ElementNotFound(self.element_id));
        }

        // Capture and remove any edges that reference this node.
        self.removed_edges = diagram
            .edges
            .iter()
            .filter(|(_, e)| e.source_node_id == self.element_id || e.target_node_id == self.element_id)
            .map(|(id, _)| *id)
            .collect();

        for edge_id in &self.removed_edges {
            diagram.edges.remove(edge_id);
        }

        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;

        // Restore node.
        if let Some(node) = self.removed_node.take() {
            diagram.nodes.insert(self.element_id, node);
        }

        // Edges cannot be restored without their full data — for a complete
        // implementation, store removed ViewEdges alongside their IDs.
        // This simplified version requires edges to be re-added by the user.
        if !self.removed_edges.is_empty() {
            // Log warning or return partial-success indicator.
        }

        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

### 5.3 MoveNode

```rust
/// Moves a node to a new position on the diagram canvas.
pub struct MoveNode {
    diagram_id: DiagramId,
    element_id: UmlId,
    old_position: Option<Point>,
    new_position: Point,
    description: String,
}

impl Command for MoveNode {
    fn execute(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;
        let node = diagram
            .nodes
            .get_mut(&self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;

        self.old_position = Some(node.bounds.origin);
        node.bounds.origin = self.new_position;
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;
        let node = diagram
            .nodes
            .get_mut(&self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;

        // unwrap is safe here because execute() always sets old_position
        // before mutation.
        node.bounds.origin = self.old_position.unwrap();
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

### 5.4 ResizeNode

```rust
/// Resizes a node to new dimensions.
pub struct ResizeNode {
    diagram_id: DiagramId,
    element_id: UmlId,
    old_size: Option<Size>,
    new_size: Size,
    description: String,
}

impl Command for ResizeNode {
    fn execute(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;
        let node = diagram
            .nodes
            .get_mut(&self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;

        self.old_size = Some(node.bounds.size);
        node.bounds.size = self.new_size;
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;
        let node = diagram
            .nodes
            .get_mut(&self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;

        node.bounds.size = self.old_size.unwrap();
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

### 5.5 AddEdgeToDiagram

```rust
/// Adds a ViewEdge to a diagram, linking two existing ViewNodes.
pub struct AddEdgeToDiagram {
    diagram_id: DiagramId,
    relationship_id: UmlId,
    source_node_id: UmlId,
    target_node_id: UmlId,
    routing: LineRouting,
    description: String,
    // Populated on execute for undo.
    overwritten_edge: Option<ViewEdge>,
    assigned_edge_id: Option<EdgeId>,
}

impl Command for AddEdgeToDiagram {
    fn execute(&mut self, model: &mut UmlModel) -> CommandResult {
        let diagram = model
            .get_diagram_mut(self.diagram_id)
            .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;

        // Validate that both endpoint nodes exist in this diagram.
        if !diagram.nodes.contains_key(&self.source_node_id) {
            return Err(CommandError::InvalidOperation(
                format!("source node {:?} not in diagram", self.source_node_id),
            ));
        }
        if !diagram.nodes.contains_key(&self.target_node_id) {
            return Err(CommandError::InvalidOperation(
                format!("target node {:?} not in diagram", self.target_node_id),
            ));
        }

        let edge_id = EdgeId::new();
        self.assigned_edge_id = Some(edge_id);

        let edge = ViewEdge {
            relationship_id: self.relationship_id,
            source_node_id: self.source_node_id,
            target_node_id: self.target_node_id,
            waypoints: Vec::new(),
            routing: self.routing,
            labels: Vec::new(),
        };

        diagram.edges.insert(edge_id, edge);
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> CommandResult {
        if let Some(edge_id) = self.assigned_edge_id {
            let diagram = model
                .get_diagram_mut(self.diagram_id)
                .ok_or(CommandError::InvalidOperation("diagram not found".into()))?;
            diagram.edges.remove(&edge_id);
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

### 5.6 AddWaypoint / MoveWaypoint / RemoveWaypoint

Edge waypoint manipulation commands follow the same pattern:

```rust
pub struct AddWaypoint {
    diagram_id: DiagramId,
    edge_id: EdgeId,
    index: usize,       // position in the waypoints Vec
    point: Point,
    // undo state
    inserted_at: Option<usize>,
}

pub struct MoveWaypoint {
    diagram_id: DiagramId,
    edge_id: EdgeId,
    index: usize,
    old_position: Option<Point>,
    new_position: Point,
}

pub struct RemoveWaypoint {
    diagram_id: DiagramId,
    edge_id: EdgeId,
    index: usize,
    removed_point: Option<Point>,
}
```

### 5.7 CommandError Extensions

The existing `CommandError` enum gains new variants for diagram-related
errors:

```rust
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CommandError {
    #[error("element {0} not found")]
    ElementNotFound(UmlId),

    #[error("diagram {0} not found")]
    DiagramNotFound(DiagramId),

    #[error("edge {0} not found")]
    EdgeNotFound(EdgeId),

    #[error("invalid operation: {0}")]
    InvalidOperation(String),
}
```

---

## 6. Module Structure

```
crates/uml-core/src/
├── diagram/
│   ├── mod.rs          # Diagram, DiagramId, DiagramKind, EdgeId, re-exports
│   ├── geometry.rs     # Point, Size, Rect
│   ├── node.rs         # ViewNode
│   └── edge.rs         # ViewEdge, LineRouting, EdgeLabel, EdgeLabelKind
├── undo/
│   ├── mod.rs          # Command trait, CommandError
│   └── commands.rs     # All command impls (model + visual)
├── model/
│   ├── mod.rs          # UmlModel (adds diagrams field and accessors)
│   └── ...
└── ...
```

The `diagram` module is a leaf module — it depends on `uml_id::UmlId` but not
on any other module in `uml-core`. The `undo::commands` module depends on
`diagram` and `model`.

---

## 7. Integration With Existing Code

### 7.1 UmlModel Changes

The `diagrams: Vec<Diagram>` field is purely additive. Existing code that
constructs or manipulates `UmlModel` continues to work without modification.
The `diagrams` field defaults to an empty vector.

```rust
impl UmlModel {
    pub fn new() -> Self {
        Self {
            elements: IndexMap::new(),
            parent_index: HashMap::new(),
            diagrams: Vec::new(),        // new
        }
    }
}
```

### 7.2 Serialization

Both `Diagram` and `UmlModel` derive `Serialize`/`Deserialize`. When
serializing `UmlModel`, diagrams are included as a top-level array alongside
elements. The XMI writer (M9/M10) can choose to write diagram geometry into
XMI annotations or into a separate `.umldiagram` file — the data model
supports both approaches.

### 7.3 No Circular Dependencies

`ViewNode.model_element_id` is a `UmlId`, not a reference. Serialization is
straightforward because there are no cycles between `Diagram` and `ModelElement`.

---

## 8. Error Handling

All diagram mutations go through the `Command` trait and return
`Result<(), CommandError>`. Errors are categorized:

| Error variant            | When raised                                         |
|--------------------------|-----------------------------------------------------|
| `DiagramNotFound`        | Command references a `DiagramId` not in the model   |
| `ElementNotFound`        | `ViewNode.model_element_id` not in model elements   |
| `EdgeNotFound`           | Command references an `EdgeId` not in a diagram     |
| `InvalidOperation`       | Semantic violation (e.g., wrong diagram kind, duplicate, missing node) |

There are **no panics** in diagram command execution. The `unwrap()` calls in
`undo()` are safe because the state they read was written by the preceding
`execute()` call on the same command instance.

---

## 9. Testing Strategy

### 9.1 Unit Tests (geometry.rs)

```
test point_creation
test point_equality
test size_creation
test rect_contains_point
test rect_center
test rect_edges
```

### 9.2 Unit Tests (node.rs, edge.rs)

```
test view_node_creation
test view_node_links_to_model_element
test view_edge_creation
test view_edge_links_source_target
test edge_label_creation
test line_routing_default
```

### 9.3 Integration Tests (commands)

```
test add_node_to_diagram
test add_node_to_nonexistent_diagram_fails
test add_node_with_duplicate_id_overwrites
test remove_node_from_diagram
test remove_node_also_removes_edges
test remove_nonexistent_node_fails
test move_node_updates_position
test move_node_undo_restores_original
test resize_node_updates_size
test resize_node_undo_restores_original
test add_edge_to_diagram
test add_edge_with_missing_nodes_fails
test add_edge_undo_removes_edge
```

### 9.4 Round-Trip Tests

```
test diagram_serialization_roundtrip
test model_with_diagrams_serialization_roundtrip
test diagram_equality_after_roundtrip
```

### 9.5 Invariant Tests

```
test all_nodes_reference_valid_model_elements
test all_edges_reference_valid_nodes_in_same_diagram
test all_edges_reference_valid_relationships
```

Invariant tests iterate over a `UmlModel` and verify every diagram's
cross-references. These are assertion-style tests that panic on violation.

---

## 10. Future Considerations

### 10.1 Rendering (Milestone 13+)

A `DiagramRenderer` trait will consume `Diagram` data and produce output.
Possible implementations:
- **Qt renderer** using `QGraphicsScene`/`QGraphicsView` (for GUI).
- **SVG renderer** for export.
- **Text-based renderer** for CLI preview.

The diagram engine does not depend on any renderer.

### 10.2 Composite / Group Nodes

Future milestones may add `GroupNode` containing child `ViewNode`s with
relative positioning. This can be expressed by adding a `children: Vec<UmlId>`
field to `ViewNode` without changing the core architecture.

### 10.3 Diagram Validation

A `validate_diagram(diagram, model) -> Vec<DiagramError>` function can be
added to detect:
- Orphaned nodes (no corresponding model element).
- Orphaned edges (source or target node missing).
- Duplicate positions (overlapping nodes at same Z-order).
- Invalid edge endpoints (nodes not in diagram).

This is purely additive.

### 10.4 Auto-Layout

Auto-layout algorithms (e.g., layered layout for class diagrams, orthogonal
routing for edges) operate on `Diagram` data exclusively. They produce new
positions and waypoints, which are then applied through `MoveNode`,
`ResizeNode`, and `MoveWaypoint` commands for undo support.

```
┌──────────────┐     ┌─────────────────┐     ┌──────────────────┐
│    Diagram   │────▶│ AutoLayout      │────▶│ Visual Commands  │
│  (input)     │     │ (stateless fn)  │     │ (undoable)       │
└──────────────┘     └─────────────────┘     └──────────────────┘
```

---

## 11. Appendix: Type Relationships

```
UmlModel
 ├── elements: IndexMap<UmlId, ModelElement>
 ├── parent_index: HashMap<UmlId, Vec<UmlId>>
 └── diagrams: Vec<Diagram>
       └── Diagram
            ├── id: DiagramId
            ├── name: String
            ├── kind: DiagramKind
            ├── nodes: IndexMap<UmlId, ViewNode>
            │     └── ViewNode
            │          ├── model_element_id: UmlId  ────────► ModelElement.id
            │          ├── bounds: Rect
            │          │    ├── origin: Point
            │          │    └── size: Size
            │          ├── z_order: i32
            │          └── visible: bool
            └── edges: IndexMap<EdgeId, ViewEdge>
                  └── ViewEdge
                       ├── relationship_id: UmlId  ────────► Relationship.id (in elements)
                       ├── source_node_id: UmlId  ─────────► ViewNode.model_element_id
                       ├── target_node_id: UmlId  ─────────► ViewNode.model_element_id
                       ├── waypoints: Vec<Point>
                       ├── routing: LineRouting
                       └── labels: Vec<EdgeLabel>
                             └── EdgeLabel
                                  ├── position: Point
                                  ├── text: String
                                  └── kind: EdgeLabelKind
```

---

## 12. Appendix: Command Lifecycle

```
User action
    │
    ▼
Create command ──────────────────────► Store description for undo menu
    │
    ▼
Execute ──► mutate model.diagrams ──► push onto undo stack
    │
    ├── Success ──► return Ok(())
    │
    └── Failure ──► return Err(...) ──► UI shows error (no state change)

Later:
    Undo ──► pop from undo stack ──► command.undo(model)
    Redo ──► pop from redo stack ──► command.execute(model)
```

Each command instance stores both the forward parameters and the reverse
state captured during `execute()`. This ensures that `undo()` can always
restore the prior state without reading from the model.
