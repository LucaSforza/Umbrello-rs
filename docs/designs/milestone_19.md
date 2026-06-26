# Milestone 19 — Interactive Edge Creation

**Status:** Design v1  
**Target Milestone:** M19  
**Dependencies:** M18 (Property Editor), M17 (Tool Palette & Node Creation), M15 (Rich Arrowhead Rendering), M11 (Undo/Redo Engine)

---

## 1. Objective

M17 and M18 delivered node creation and property inspection on the canvas. However, the tool still cannot create **relationships** between nodes — the defining feature of any UML editor. Users can place classes, interfaces, packages, etc., but cannot draw a line between them to express inheritance, dependency, or association.

M19 closes this gap by providing:

1. **Six new tool palette buttons** (grouped below a separator) for relationship types: Generalization, Realization, Association, Aggregation, Composition, Dependency.
2. **Click-drag-to-connect** on the canvas — mousedown on a source node, drag a rubber-band line to a target node, mouseup to create the relationship.
3. **A new `CreateEdge` undoable command** that atomically creates a `Relationship` model element and a `ViewEdge` in the active diagram.
4. **Real-time rubber-band preview** — during the drag, a semi-transparent line with the appropriate arrowhead is drawn from the source node center to the cursor.
5. **One-shot tool mode** — after successfully creating an edge, the tool resets to Select (same pattern as node creation tools). Escape or click on empty space cancels the drag.
6. **Undo/Redo support** — `CreateEdge` is a single undoable operation; undo removes both the relationship and the visual edge.

**Out of scope:** Edge labels, multiplicity setting, relationship role names, edge routing customization (waypoints, orthogonal routing), edge deletion from the canvas, edge selection/property editing, relationship creation from the property editor. Those are deferred to future milestones. No changes to `uml-io` or `uml-codegen`.

---

## 2. Crates to Modify

| Crate | Changes | Rationale |
|-------|---------|-----------|
| `uml-core` | **Light touch** — 1 new `CreateEdge` command in `undo/commands.rs`; re-export from `undo/mod.rs` and `lib.rs` | Edge creation must be undoable |
| `apps/umbrello` | **Primary** — tool palette extension (+6 variants), canvas drag-to-connect interaction, rubber-band preview rendering | All GUI interaction logic lives here |
| `uml-io` | **Zero changes** | No persistence changes needed |
| `uml-codegen` | **Zero changes** | — |

**No new dependencies.** All functionality uses existing crates (egui, uml-core). The `Relationship` struct, its six constructors, `ViewEdge`, `EdgeId`, and all arrowhead drawing functions already exist and are well-tested.

---

## 3. New Types, Fields, and Functions

### 3.1 `ToolMode` Extension (in `apps/umbrello/src/tool_palette.rs`)

Add six new edge-creation variants to `ToolMode`:

```rust
/// The active tool in the tool palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolMode {
    // ── Existing variants ──
    Select,
    CreateClass,
    CreateInterface,
    CreateEnum,
    CreateDatatype,
    CreatePackage,

    // ── New edge-creation variants (M19) ──
    /// Create a Generalization (subclass → superclass, hollow triangle arrowhead).
    CreateGeneralization,
    /// Create a Realization (class → interface, dashed line + hollow triangle).
    CreateRealization,
    /// Create a plain Association (solid line, no arrowhead).
    CreateAssociation,
    /// Create an Aggregation (whole → part, hollow diamond at source).
    CreateAggregation,
    /// Create a Composition (whole → part, filled diamond at source).
    CreateComposition,
    /// Create a Dependency (dashed line + open arrow at target).
    CreateDependency,
}
```

#### New methods on `ToolMode`:

```rust
impl ToolMode {
    /// Updated label() returning appropriate text for each variant.
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::Select => "🖱 Select",
            Self::CreateClass => "📦 Class",
            Self::CreateInterface => "🔌 Interface",
            Self::CreateEnum => "🔢 Enum",
            Self::CreateDatatype => "📋 Datatype",
            Self::CreatePackage => "📁 Package",
            Self::CreateGeneralization => "△ Generalization",
            Self::CreateRealization => "△ Realization",
            Self::CreateAssociation => "— Association",
            Self::CreateAggregation => "◇ Aggregation",
            Self::CreateComposition => "◆ Composition",
            Self::CreateDependency => "⇢ Dependency",
        }
    }

    /// Updated tooltip for each edge tool.
    fn tooltip(&self) -> &'static str { /* ... */ }

    /// Whether this tool is a creation tool (sets crosshair and shows ghost preview).
    /// Updated: node creation tools return true; edge tools return false
    /// (they use a different cursor/interaction).
    pub(crate) fn is_creation_tool(&self) -> bool {
        matches!(
            self,
            Self::CreateClass
                | Self::CreateInterface
                | Self::CreateEnum
                | Self::CreateDatatype
                | Self::CreatePackage
        )
    }

    /// Whether this tool creates edges (click-drag between nodes).
    pub(crate) fn is_edge_tool(&self) -> bool {
        matches!(
            self,
            Self::CreateGeneralization
                | Self::CreateRealization
                | Self::CreateAssociation
                | Self::CreateAggregation
                | Self::CreateComposition
                | Self::CreateDependency
        )
    }

    /// Map the edge tool variant to the corresponding `AssociationType`.
    pub(crate) fn association_type(&self) -> Option<AssociationType> {
        match self {
            Self::CreateGeneralization => Some(AssociationType::Generalization),
            Self::CreateRealization => Some(AssociationType::Realization),
            Self::CreateAssociation => Some(AssociationType::Association),
            Self::CreateAggregation => Some(AssociationType::Aggregation),
            Self::CreateComposition => Some(AssociationType::Composition),
            Self::CreateDependency => Some(AssociationType::Dependency),
            _ => None,
        }
    }
}
```

`is_creation_tool()` is narrowed to exclude edge tools, so the existing ghost-rectangle preview and background-click element creation are **not** triggered by edge tools.

### 3.2 `UmbrelloApp` — New Fields (in `app.rs`)

```rust
pub(crate) struct UmbrelloApp {
    // ... existing fields ...

    /// When an edge tool is active, this tracks the source node of a click-drag.
    /// Set to `Some(id)` on mousedown over a node; cleared on mouseup or Escape.
    pub(crate) drag_source_node_id: Option<UmlId>,

    /// Tracks whether the primary mouse button was down in the previous frame,
    /// used to detect edge-drag start transitions.
    pub(crate) pointer_was_down: bool,
}
```

Initialized to `None` / `false` in `UmbrelloApp::new()`.

### 3.3 `CreateEdge` Command (in `crates/uml-core/src/undo/commands.rs`)

```rust
use crate::diagram::{DiagramId, EdgeId, LineRouting, ViewEdge};
use crate::elements::Relationship;
use crate::types::AssociationType;

/// Command to create a relationship edge between two nodes on a diagram.
///
/// On execute: inserts the Relationship into UmlModel, adds a ViewEdge to the diagram.
/// On undo: removes the ViewEdge from the diagram, removes the Relationship from the model.
///
/// Follows the snapshot pattern:
/// - `relationship_element` is `Some` before first execute / after undo.
/// - `execute()` takes it and inserts into the model.
/// - `undo()` removes it from the model and stores it back.
#[derive(Debug)]
pub struct CreateEdge {
    /// The diagram to add the edge to.
    diagram_id: DiagramId,
    /// The UmlId of the created Relationship element.
    relationship_id: UmlId,
    /// The EdgeId of the created ViewEdge.
    edge_id: EdgeId,
    /// The source node's model element ID.
    source_node_id: UmlId,
    /// The target node's model element ID.
    target_node_id: UmlId,
    /// The Relationship element; consumed on execute, restored on undo.
    relationship_element: Option<ModelElement>,
    /// Human-readable description.
    description: String,
}

impl CreateEdge {
    /// Create a command that will create a new relationship edge between two nodes.
    ///
    /// The relationship is constructed using the appropriate `Relationship` constructor
    /// based on `kind`, and both a `UmlId` and `EdgeId` are generated automatically.
    #[must_use]
    pub fn new(
        diagram_id: DiagramId,
        source_node_id: UmlId,
        target_node_id: UmlId,
        kind: AssociationType,
    ) -> Self {
        let rel = match kind {
            AssociationType::Generalization => {
                Relationship::new_generalization(source_node_id, target_node_id)
            },
            AssociationType::Realization => {
                Relationship::new_realization(source_node_id, target_node_id)
            },
            AssociationType::Association => {
                Relationship::new_association(source_node_id, target_node_id)
            },
            AssociationType::Aggregation => {
                Relationship::new_aggregation(source_node_id, target_node_id)
            },
            AssociationType::Composition => {
                Relationship::new_composition(source_node_id, target_node_id)
            },
            AssociationType::Dependency => {
                Relationship::new_dependency(source_node_id, target_node_id)
            },
        };
        let rel_id = rel.base.id;
        let edge_id = EdgeId::new();
        let kind_name = match kind {
            AssociationType::Generalization => "Generalization",
            AssociationType::Realization => "Realization",
            AssociationType::Association => "Association",
            AssociationType::Aggregation => "Aggregation",
            AssociationType::Composition => "Composition",
            AssociationType::Dependency => "Dependency",
        };
        let desc = format!("Create {kind_name} edge");
        Self {
            diagram_id,
            relationship_id: rel_id,
            edge_id,
            source_node_id,
            target_node_id,
            relationship_element: Some(ModelElement::Relationship(rel)),
            description: desc,
        }
    }
}

impl Command for CreateEdge {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // 1. Insert the relationship into the model
        let rel = self
            .relationship_element
            .take()
            .ok_or_else(|| CommandError::InvalidOperation("CreateEdge already executed".into()))?;
        model.insert(rel);

        // 2. Add the ViewEdge to the diagram
        let d = model
            .get_diagram_mut(self.diagram_id)
            .ok_or_else(|| CommandError::InvalidOperation("diagram not found".into()))?;
        d.add_edge(
            self.edge_id,
            ViewEdge::new(
                self.relationship_id,
                self.source_node_id,
                self.target_node_id,
                LineRouting::Direct,
            ),
        );
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // 1. Remove the ViewEdge from the diagram
        if let Some(d) = model.get_diagram_mut(self.diagram_id) {
            d.remove_edge(self.edge_id);
        }

        // 2. Remove the relationship from the model and store for re-execution
        self.relationship_element = model.remove(self.relationship_id);
        if self.relationship_element.is_none() {
            return Err(CommandError::ElementNotFound(self.relationship_id));
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

### 3.4 `place_edge` Method (in `apps/umbrello/src/tool_palette.rs`)

```rust
impl UmbrelloApp {
    /// Place a new relationship edge between two nodes on the active diagram.
    /// Executes a single `CreateEdge` command (atomic, one undo step).
    /// Returns `Ok(())` if the command succeeds.
    pub(crate) fn place_edge(
        &mut self,
        source_node_id: UmlId,
        target_node_id: UmlId,
    ) -> Result<(), String> {
        let kind = self
            .current_tool
            .association_type()
            .ok_or_else(|| "Current tool is not an edge tool".to_string())?;
        let diag_idx = self
            .active_diagram
            .ok_or_else(|| "No active diagram".to_string())?;
        let diagram_id = self.model.diagrams()[diag_idx].id;

        self.execute_command(Box::new(commands::CreateEdge::new(
            diagram_id,
            source_node_id,
            target_node_id,
            kind,
        )));

        Ok(())
    }
}
```

### 3.5 Re-exports

In `crates/uml-core/src/undo/mod.rs`:
```rust
pub use commands::{..., CreateEdge};
```

In `crates/uml-core/src/lib.rs`:
```rust
// Confirm CreateEdge is included in the commands re-export.
```

---

## 4. XMI Changes

None. Edge creation operates entirely on in-memory `UmlModel` and `Diagram` state. The existing XMI writer already serializes `Relationship` model elements and `ViewEdge` diagram entries. Saving the model after creating edges produces a valid XMI file with no format changes.

---

## 5. UI Changes (all in `apps/umbrello/src/`)

### 5.1 Tool Palette Panel (in `tool_palette.rs`)

Extend `render_tool_palette()` to include the six edge tool buttons below a separator:

```rust
pub(crate) fn render_tool_palette(&mut self, ui: &mut egui::Ui) {
    ui.heading("Tools");
    ui.add_space(4.0);

    // ── Selection + node creation tools ──
    for tool in &[
        ToolMode::Select,
        ToolMode::CreateClass,
        ToolMode::CreateInterface,
        ToolMode::CreateEnum,
        ToolMode::CreateDatatype,
        ToolMode::CreatePackage,
    ] {
        let selected = self.current_tool == *tool;
        let button = egui::SelectableLabel::new(selected, tool.label());
        if ui.add(button).clicked() {
            self.current_tool = *tool;
            self.preview_position = None;
            self.drag_source_node_id = None;
            self.status_message = format!("Tool: {}", tool.label());
        }
    }

    ui.separator();
    ui.label(egui::RichText::new("Edges").weak());

    // ── Edge creation tools ──
    for tool in &[
        ToolMode::CreateGeneralization,
        ToolMode::CreateRealization,
        ToolMode::CreateAssociation,
        ToolMode::CreateAggregation,
        ToolMode::CreateComposition,
        ToolMode::CreateDependency,
    ] {
        let selected = self.current_tool == *tool;
        let button = egui::SelectableLabel::new(selected, tool.label());
        if ui.add(button).clicked() {
            self.current_tool = *tool;
            self.preview_position = None;
            self.drag_source_node_id = None;
            self.status_message = format!("Tool: {}", tool.label());
        }
    }

    ui.separator();
}
```

### 5.2 Canvas Edge-Creation Interaction (in `canvas.rs`)

#### 5.2.1 Node Interaction for Edge Tools

In the node interaction loop (after `draw_partitioned_node` + `draw_edges`), add a branch for edge tools that allocates a drag-sensitive region over each node **separately from** the existing `Sense::click_and_drag()` used for node dragging in Select mode:

```rust
// ── Handle edge-creation drag ──────────────────────
if self.current_tool.is_edge_tool() {
    for &(model_element_id, rect, _, _) in &node_rects {
        // Use Sense::drag() to detect press + drag on this node
        let response = ui.allocate_rect(rect, egui::Sense::drag());

        // Start edge drag on the first frame of dragging over a node
        if response.dragged() && self.drag_source_node_id.is_none() {
            self.drag_source_node_id = Some(model_element_id);
            // Request repaint so the rubber-band appears immediately
            ui.ctx().request_repaint();
        }
    }
}
```

**Important:** This loop runs **after** the existing `click_and_drag()` loop for Select mode. When an edge tool is active, the existing drag logic for node movement in Select mode should **not** be executed (it's gated by `if !self.current_tool.is_creation_tool()` and `if !self.current_tool.is_edge_tool()`).

To keep the existing Select-mode interaction unchanged, restructure the canvas interaction loop as:

```rust
if self.current_tool == ToolMode::Select {
    // ── Select mode: click-to-select + drag-to-move ──
    // (existing code from canvas.rs lines 74-107, unchanged)
    for &(model_element_id, rect, orig_x, orig_y) in &node_rects {
        let sense = egui::Sense::click_and_drag();
        let response = ui.allocate_rect(rect, sense);
        // ... existing selection + move logic ...
    }
} else if self.current_tool.is_edge_tool() {
    // ── Edge tool: drag from source node ──
    for &(model_element_id, rect, _, _) in &node_rects {
        let response = ui.allocate_rect(rect, egui::Sense::drag());
        if response.dragged() && self.drag_source_node_id.is_none() {
            self.drag_source_node_id = Some(model_element_id);
            ui.ctx().request_repaint();
        }
    }
} else {
    // ── Creation tool: existing ghost + click-to-place ──
    // (existing code, unchanged)
}
```

#### 5.2.2 Global Edge Release Detection

After all node interaction loops, add a global check for mouse button release to determine where the edge drag ended:

```rust
// ── Edge drag: detect release on target node ────────
if self.drag_source_node_id.is_some() && self.current_tool.is_edge_tool() {
    let released = ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
    if released {
        let source_id = self.drag_source_node_id.take().unwrap();
        if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
            // Check all node rects to find which one (if any) the cursor is over
            let mut found_target = false;
            for &(target_id, target_rect, _, _) in &node_rects {
                if target_rect.contains(pointer_pos) && target_id != source_id {
                    if let Err(e) = self.place_edge(source_id, target_id) {
                        self.status_message = format!("Error: {e}");
                    } else {
                        self.status_message =
                            "Edge created — tool reset to Select".into();
                    }
                    // One-shot: reset to Select after successful placement
                    self.current_tool = ToolMode::Select;
                    found_target = true;
                    break;
                }
            }
            if !found_target {
                // Released on empty space, same node, or non-node area
                self.status_message = "Edge creation cancelled".into();
                // Don't reset tool — user can try again
            }
        }
        ui.ctx().request_repaint();
    }
}
```

#### 5.2.3 Edge Drag Repaint

Add a continuous repaint request while an edge drag is in progress, to keep the rubber-band preview visible:

```rust
if self.drag_source_node_id.is_some() && self.current_tool.is_edge_tool() {
    ui.ctx().request_repaint();
}
```

This should be placed in the `render_canvas()` method, alongside the existing repaint-while-dragging logic for node movement.

### 5.3 Rubber-Band Preview (in `canvas.rs`)

While an edge drag is in progress, draw a semi-transparent line from the source node center to the current cursor position, with the appropriate arrowhead for the active edge tool:

```rust
// ── Rubber-band preview during edge drag ────────────
if let Some(source_id) = self.drag_source_node_id {
    if self.current_tool.is_edge_tool() {
        let diagram = self.model.diagrams()[self.active_diagram.unwrap()].clone();
        if let (Some(source_node), Some(pointer_pos)) = (
            diagram.get_node(source_id),
            ui.ctx().pointer_latest_pos(),
        ) {
            let src_center = egui::pos2(
                (source_node.bounds.x() + source_node.bounds.width() / 2.0) as f32,
                (source_node.bounds.y() + source_node.bounds.height() / 2.0) as f32,
            );
            let cursor = pointer_pos;
            let dir = cursor - src_center;
            let len = dir.length();
            if len > 1.0 {
                let unit = dir / len;
                let perp = egui::vec2(-unit.y, unit.x);
                let preview_color = egui::Color32::from_rgba_premultiplied(100, 100, 100, 120);
                let painter = ui.painter();

                match self.current_tool {
                    ToolMode::CreateGeneralization => {
                        painter.line_segment(
                            [src_center, cursor],
                            egui::Stroke::new(1.5, preview_color),
                        );
                        draw_hollow_triangle(&painter, cursor, unit, perp, preview_color);
                    },
                    ToolMode::CreateRealization => {
                        draw_dashed_line(&painter, src_center, cursor, egui::Stroke::new(1.5, preview_color));
                        draw_hollow_triangle(&painter, cursor, unit, perp, preview_color);
                    },
                    ToolMode::CreateAssociation => {
                        painter.line_segment(
                            [src_center, cursor],
                            egui::Stroke::new(1.0, preview_color),
                        );
                    },
                    ToolMode::CreateAggregation => {
                        painter.line_segment(
                            [src_center, cursor],
                            egui::Stroke::new(1.5, preview_color),
                        );
                        draw_hollow_diamond(&painter, src_center, unit, perp, preview_color);
                    },
                    ToolMode::CreateComposition => {
                        painter.line_segment(
                            [src_center, cursor],
                            egui::Stroke::new(1.5, preview_color),
                        );
                        draw_filled_diamond(&painter, src_center, unit, perp, preview_color);
                    },
                    ToolMode::CreateDependency => {
                        draw_dashed_line(&painter, src_center, cursor, egui::Stroke::new(1.0, preview_color));
                        draw_open_arrow(&painter, cursor, unit, perp, preview_color);
                    },
                    _ => {},
                }
            }
        }
    }
}
```

**Design note:** The preview color is a semi-transparent dark gray (`rgba(100, 100, 100, 120)`), applied uniformly to all edge types. This avoids visual confusion with the fully-opaque black arrows of committed edges. The rubber-band is drawn **after** committed edges and **before** nodes, so committed edges remain visible and nodes occlude the rubber-band at their ends.

### 5.4 Keyboard Shortcuts (in `app.rs` `update()`)

Add six new keyboard shortcuts for edge tools, only active when `!ctx.wants_keyboard_input()`:

```rust
// ── Edge tool keyboard shortcuts ────────────────────
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::G)) {
    self.current_tool = crate::tool_palette::ToolMode::CreateGeneralization;
    self.drag_source_node_id = None;
}
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) {
    self.current_tool = crate::tool_palette::ToolMode::CreateRealization;
    self.drag_source_node_id = None;
}
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::A)) {
    self.current_tool = crate::tool_palette::ToolMode::CreateAssociation;
    self.drag_source_node_id = None;
}
// Note: 'C' is already used for CreateClass. Aggregation and Composition
// are accessed via the tool palette buttons only (no single-key shortcut
// since 'C' is taken and 'G' is for Generalization).
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::N)) {
    // 'N' is used for Ctrl+N (New File). Without Ctrl, it's free.
    self.current_tool = crate::tool_palette::ToolMode::CreateDependency;
    self.drag_source_node_id = None;
}
```

Updated Escape handler (in the keyboard shortcut section) should also cancel edge drag:

```rust
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
    if self.selected_element_id.is_some() {
        self.selected_element_id = None;
        self.name_edit_buffer.clear();
        self.status_message = "Selection cleared".into();
    } else if self.drag_source_node_id.is_some() {
        self.drag_source_node_id = None;
        self.status_message = "Edge creation cancelled".into();
    } else {
        self.current_tool = crate::tool_palette::ToolMode::Select;
    }
}
```

### 5.5 Import Updates (in `canvas.rs`)

Add `AssociationType` to the existing imports and ensure `draw_dashed_line`, `draw_hollow_triangle`, `draw_hollow_diamond`, `draw_filled_diamond`, `draw_open_arrow` are already imported from `crate::rendering` (they are — confirmed).

### 5.6 Status Message Updates

When an edge tool is selected, update the status message to indicate the interaction mode:

```
"Tool: △ Generalization — Click-drag from source node to target node"
```

This is handled by the existing `self.status_message = format!("Tool: {}", tool.label());` pattern in the palette click handler. The `label()` method returns the appropriate string.

---

## 6. Test Plan

### 6.1 `uml-core` — `CreateEdge` Command Tests (in `crates/uml-core/src/undo/commands.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| CMD-10 | `create_edge_execute_generalization` | `CreateEdge::execute` inserts a Generalization Relationship + ViewEdge (Direct routing) into model + diagram |
| CMD-11 | `create_edge_undo_generalization` | `CreateEdge::undo` removes both the Relationship from model and ViewEdge from diagram |
| CMD-12 | `create_edge_execute_all_kinds` | All 6 `AssociationType` variants produce correct `Relationship.kind` and renderable edges |
| CMD-13 | `create_edge_diagram_not_found` | `execute()` returns `Err(InvalidOperation)` when diagram_id doesn't exist |
| CMD-14 | `create_edge_description` | `description()` returns a string containing the relationship kind name |
| CMD-15 | `create_edge_undo_then_redo` | After execute → undo → execute, the edge is fully restored (both Relationship + ViewEdge) |

### 6.2 `apps/umbrello` — Edge Tool & Interaction Tests (in `tests.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| APP-16 | `edge_tool_is_edge_tool` | `ToolMode::CreateGeneralization.is_edge_tool()` returns `true` |
| APP-17 | `edge_tool_not_creation_tool` | `ToolMode::CreateGeneralization.is_creation_tool()` returns `false` |
| APP-18 | `edge_tool_association_type` | `ToolMode::CreateGeneralization.association_type()` returns `Some(AssociationType::Generalization)` |
| APP-19 | `select_not_edge_tool` | `ToolMode::Select.is_edge_tool()` returns `false` |
| APP-20 | `place_edge_creates_relationship` | Calling `place_edge(src, tgt)` with an edge tool active creates a `Relationship` in the model |
| APP-21 | `place_edge_creates_view_edge` | The active diagram gains a `ViewEdge` with `Direct` routing after `place_edge()` |
| APP-22 | `place_edge_dirty_flag` | `is_dirty` becomes `true` after executing `place_edge()` |
| APP-23 | `place_edge_undo_removes_both` | `History::undo()` after `place_edge()` removes both the `Relationship` and the `ViewEdge` |
| APP-24 | `place_edge_undo_redo_restores` | After undo → redo, the edge is fully restored |
| APP-25 | `edge_tool_no_active_diagram_errors` | `place_edge()` returns `Err(...)` when `active_diagram` is `None` |
| APP-26 | `drag_source_node_id_defaults_none` | New `UmbrelloApp` has `drag_source_node_id: None` |
| APP-27 | `edge_tool_select_label_clears_drag` | Clicking a Select tool palette button clears `drag_source_node_id` |

> **Note on test isolation:** As with M18, app-level tests in `tests.rs` do not require an egui `Context`. They exercise the data model directly by constructing commands, calling `execute_command()`, and `place_edge()`. The rubber-band rendering and drag interaction are tested manually/visually.

### 6.3 Manual / Visual Tests

| Test ID | What to Verify |
|---------|---------------|
| VIS-01 | Select each edge tool from palette; cursor icon should not change to crosshair (edge tools are not creation tools) |
| VIS-02 | Mousedown on a node, drag to another node, release → new edge appears with correct arrowhead style |
| VIS-03 | During drag, rubber-band preview shows semi-transparent line + correct arrowhead from source to cursor |
| VIS-04 | Release on the same node → no edge created, drag is cancelled |
| VIS-05 | Release on empty canvas → no edge created, drag is cancelled |
| VIS-06 | Press Escape during drag → edge creation cancelled, drag state cleared |
| VIS-07 | After successful edge creation, tool resets to Select (one-shot) |
| VIS-08 | Save → close → reopen → edges are preserved in XMI round-trip |
| VIS-09 | Ctrl+Z after creating an edge → both the relationship and visual edge disappear |
| VIS-10 | Ctrl+Shift+Z (redo) → edge returns |

### 6.4 Verification Commands

```sh
# Unit tests for new CreateEdge command
cargo test -p uml-core create_edge

# App-level edge creation tests
cargo test -p umbrello edge_tool
cargo test -p umbrello place_edge
cargo test -p umbrello drag_source

# Full suite
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

### 6.5 Expected Test Count (Post-M19)

| Test Suite | Current | New | Expected |
|------------|---------|-----|----------|
| `uml-core` commands | 9 (M18) + earlier | +6 | +6 incremental |
| `apps/umbrello` tests | 15 (M18) + earlier | +12 | +12 incremental |
| **Total** | **257** (M18) | +18 | **~275** |

---

## 7. Implementation Sequence

The `@implementer` MUST follow this order:

### Phase 1: `CreateEdge` Command (Commit 1)
1. Add `CreateEdge` struct + `impl Command` to `crates/uml-core/src/undo/commands.rs`.
2. Add import for `DiagramId`, `EdgeId`, `LineRouting`, `ViewEdge`, `Relationship` at top of file.
3. Re-export from `undo/mod.rs` (`pub use commands::CreateEdge;`).
4. Confirm re-export from `lib.rs`.
5. Write all 6 command unit tests (CMD-10 through CMD-15).
6. Verify: `cargo test -p uml-core && cargo clippy -p uml-core -- -D warnings`
7. **Commit** with message: `"feat(uml-core): add CreateEdge command for undoable relationship + ViewEdge creation"`

### Phase 2: Tool Palette Extension (Commit 2)
1. Add 6 edge-creation variants to `ToolMode` enum in `tool_palette.rs`.
2. Update `label()`, `tooltip()`, `is_creation_tool()` (narrow to node-creation tools only).
3. Add `is_edge_tool()` and `association_type()` methods.
4. Update `render_tool_palette()` to add separator + 6 edge buttons.
5. Add `place_edge()` method.
6. Update `app.rs` shortcut section with keyboard shortcuts (G for Generalization, R for Realization, A for Association, N for Dependency). Escape cancels edge drag.
7. Write APP-16 through APP-19, APP-25 tests in `tests.rs`.
8. Verify: `cargo test -p umbrello && cargo clippy -p umbrello -- -D warnings`
9. **Commit** with message: `"feat(app): add edge creation tool palette with 6 relationship types"`

### Phase 3: Canvas Drag-to-Connect + Rubber-Band Preview (Commit 3)
1. Add `drag_source_node_id: Option<UmlId>` and `pointer_was_down: bool` fields to `UmbrelloApp` (in `app.rs`). Initialize to `None` / `false` in `new()`.
2. Refactor `render_canvas()` interaction loop in `canvas.rs`:
   - Restructure to branch on `Select` / `is_edge_tool()` / `is_creation_tool()`.
   - Edge tools: allocate `Sense::drag()` on each node rect, track `drag_source_node_id` on `response.dragged() && self.drag_source_node_id.is_none()`.
   - Add global `button_released` check after all node loops to detect release on target node.
3. Add rubber-band preview drawing (after draw_edges, before node drawing) using existing arrowhead functions with semi-transparent color.
4. Add `request_repaint()` during active edge drag.
5. Add `AssociationType` import to `canvas.rs` if not already imported.
6. Write APP-20 through APP-24, APP-26, APP-27 tests.
7. Verify: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
8. **Commit** with message: `"feat(app): add click-drag edge creation with rubber-band preview on canvas"`

### Final Verification
```sh
cargo test --workspace            # All tests pass (expected: ~275)
cargo clippy --workspace --all-targets -- -D warnings  # Zero warnings
cargo fmt --all --check           # No formatting diffs
```

---

## 8. File Summary (After M19 — Changed Files Only)

```
Modified files:
  crates/uml-core/src/undo/commands.rs    # +CreateEdge struct + impl (~100 lines added, ~850→950 total)
  crates/uml-core/src/undo/mod.rs         # +1 re-export line
  crates/uml-core/src/lib.rs              # Check re-export (likely already covered by glob)
  apps/umbrello/src/tool_palette.rs       # +6 ToolMode variants, +2 methods, +place_edge, +6 palette buttons (~90 lines added, ~137→227 total)
  apps/umbrello/src/app.rs                # +2 fields, +6 keyboard shortcuts, updated Escape handler (~30 lines added)
  apps/umbrello/src/canvas.rs             # Restructured interaction loop + edge drag + rubber-band (~100 lines added, ~519→620 total)
  apps/umbrello/src/tests.rs              # +12 tests (~150 lines added, ~638→790 total)
```

No new files. No changes to `uml-io` or `uml-codegen`.

---

## 9. Design Decisions

| Decision | Rationale |
|----------|-----------|
| **`CreateEdge` is a single command** | Unlike node creation (which uses separate `CreateElement` + `AddNodeToDiagram`), edges are created in one undoable step. The user perceives "draw edge" as a single action; splitting into "create relationship" + "add view edge" doubles the undo stack. `CreateEdge` bundles both atomically. |
| **One-shot tool reset** | After successfully creating an edge, the tool resets to Select. This matches the node-creation tool behavior from M17 and prevents accidental edge spam. |
| **`Sense::drag()` per node, not `click_and_drag()`** | `click_and_drag()` fires `dragged()` continuously during a drag and `drag_stopped()` only on the initiating node. Edge creation needs to detect release on *any* node, so we use `Sense::drag()` for each node to detect the start, then check `button_released` globally against all node rects for the target. |
| **Semi-transparent rubber-band** | The preview uses `rgba(100, 100, 100, 120)` instead of fully opaque black to visually distinguish in-progress edges from committed ones. All six arrowhead types are previewed with the same color. |
| **No rubber-band when near source** | When the cursor is within ~1px of the source center (`len < 1.0`), no rubber-band is drawn — avoids degenerate rendering and flicker. |
| **`is_creation_tool()` narrowed** | Edge tools are NOT creation tools; they don't show the ghost rectangle, don't use crosshair cursor, and don't place elements on background click. This separation keeps the existing node-creation and edge-creation workflows cleanly distinct. |
| **No changes to `uml-io`** | No new XMI elements or attributes are introduced. The existing XMI writer already serializes `Relationship` model elements and `ViewEdge` entries. |
| **Aggregation/Composition diamond at source** | The arrowhead logic mirrors the existing `draw_edges()` behavior: Aggregation and Composition draw their diamond at the source end of the line. The rubber-band preview does the same. |
| **Escape cancels drag + keeps edge tool** | Unlike the node-placement tool (which resets to Select on Escape), edge tools remain active after cancelling a drag so the user can immediately try again. Pressing Escape a second time resets to Select. |

---

## 10. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| `Sense::drag()` interaction conflicts with `click_and_drag()` on same rect | Node drag in Select mode and edge drag in edge-tool mode use different `Sense` values on different frames. The interaction loop is branched (`if Select` / `else if edge` / `else creation`), so they never allocate on the same rect in the same frame. | Unit tests verify tool-specific behavior. |
| `button_released` not detected reliably | In egui, `pointer.button_released(Primary)` fires on the one frame where the button transitions from down to up. If the canvas doesn't update on that frame, the event is lost. | We already `request_repaint()` during drag and in the release handler. The continuous repaint during drag ensures the release frame is processed. |
| Large diagrams with many edges | Adding edges increases the draw load. The first frame after adding an edge shows all edges via `draw_edges()`. | The existing rendering is performant (test-COG.xmi has 57+ edges and renders smoothly). Each edge is ~4-6 line segments/vertices. |
| Keyboard shortcut collisions | `C` is already used for CreateClass, `A` for Association (but `C` was used first). Aggregation and Composition have no single-key shortcut — they require the palette. | The chosen shortcuts avoid collisions: G, R, A, N are free. 'N' without Ctrl is free (Ctrl+N is New File). |

---

*Last updated: 2026-06-26 · Umbrello-RS Milestone 19 Design v1*
