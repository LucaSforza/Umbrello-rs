# Milestone 17 — Tool Palette & Interactive Element Creation

**Status:** Design v1  
**Target Milestone:** M17  
**Dependencies:** M16 (File I/O), M15 (Rich UML Canvas), M11 (Undo/Redo — CreateElement, AddNodeToDiagram commands)

---

## 1. Objective

M16 delivered read/write File I/O and a fully rendered UML canvas. However, the application remains read-only for new content: users can view and move existing elements but cannot create new ones. The three HIGH-priority GUI gaps that unlock editing are the Tool Palette, Property Editor, and Edge Creation. Of these, **Tool Palette & Interactive Element Creation** is the prerequisite — without it, there are no elements to edit or connect.

M17 closes this gap by providing:

1. A **Tool Palette** — a vertical toolbar of UML element creation tools (Class, Interface, Enum, Datatype, Package).
2. **Click-to-Place** on the canvas — selecting a tool then clicking on the active diagram creates the element as a `ModelElement` + places a `ViewNode` at the click location.
3. **Smart default naming** — auto-generated unique names (e.g., `"Class_1"`, `"Package_3"`) to avoid collisions.
4. **Cursor & visual feedback** — crosshair cursor when a creation tool is active; ghost preview rectangle on hover.
5. **Keyboard shortcuts** — single-key activation of tools from the canvas.

**Out of scope:** Property editor (M18), edge creation (M19), resize handles, context menus, dynamic node sizing, undo macro commands. No changes to `uml-core` or `uml-io`.

---

## 2. Crates to Modify

| Crate | Changes | Rationale |
|-------|---------|-----------|
| `apps/umbrello` | **Primary** — new tool palette panel, `ToolMode` enum, click-to-create on canvas, cursor feedback, smart naming, keyboard shortcuts | All GUI interaction logic lives here |
| `uml-core` | **Zero changes** | `CreateElement` and `AddNodeToDiagram` commands already exist and are tested (M11) |
| `uml-io` | **Zero changes** | No persistence changes needed |
| `uml-codegen` | **Zero changes** | — |

**No new dependencies.** All functionality uses existing crates (egui, uml-core).

---

## 3. New Types, Fields, and Functions

### 3.1 `ToolMode` Enum

Add to `apps/umbrello/src/app.rs`:

```rust
/// The active tool in the tool palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolMode {
    /// Default: select and move existing nodes.
    Select,
    /// Create a new Class element on click.
    CreateClass,
    /// Create a new Interface element on click.
    CreateInterface,
    /// Create a new Enum element on click.
    CreateEnum,
    /// Create a new Datatype element on click.
    CreateDatatype,
    /// Create a new Package element on click.
    CreatePackage,
}

impl ToolMode {
    /// Human-readable label for the tool palette button.
    fn label(&self) -> &'static str {
        match self {
            Self::Select => "🖱 Select",
            Self::CreateClass => "📦 Class",
            Self::CreateInterface => "🔌 Interface",
            Self::CreateEnum => "🔢 Enum",
            Self::CreateDatatype => "📋 Datatype",
            Self::CreatePackage => "📁 Package",
        }
    }

    /// Short tooltip description.
    fn tooltip(&self) -> &'static str {
        match self {
            Self::Select => "Select and move elements (S, Esc)",
            Self::CreateClass => "Create a Class (C)",
            Self::CreateInterface => "Create an Interface (I)",
            Self::CreateEnum => "Create an Enum (E)",
            Self::CreateDatatype => "Create a Datatype (D)",
            Self::CreatePackage => "Create a Package (P)",
        }
    }

    /// Whether this tool creates a new element (i.e., changes cursor to crosshair).
    fn is_creation_tool(&self) -> bool {
        !matches!(self, Self::Select)
    }
}
```

### 3.2 `UmbrelloApp` — New Fields

```rust
pub struct UmbrelloApp {
    // ... existing fields ...
    /// The currently active tool in the tool palette.
    current_tool: ToolMode,
    /// Counter for auto-generated element names, keyed by element type name.
    /// Tracks the next suffix number for each type (e.g., "Class" → 3 means next is "Class_3").
    name_counters: std::collections::HashMap<String, u64>,
    /// Ghost-rectangle position for creation preview (in canvas coordinates).
    preview_position: Option<Point>,
}
```

**Initialization:**
- On `new()`: `current_tool = ToolMode::Select`, `name_counters = HashMap::new()`, `preview_position = None`.

### 3.3 `UmbrelloApp` — New Methods

```rust
impl UmbrelloApp {
    /// Generate a unique default name for a new element of the given type.
    /// Scans existing elements to find the next available suffix.
    /// E.g., if "Class_1" and "Class_2" exist, returns "Class_3".
    fn generate_unique_name(&self, base: &str) -> String;

    /// Create a ModelElement of the appropriate type with a default name.
    fn create_element_for_tool(&self, tool: ToolMode) -> ModelElement;

    /// Place a newly created element on the active diagram at the given position.
    /// Executes CreateElement + AddNodeToDiagram commands.
    /// Returns Ok if both commands succeed.
    fn place_element(&mut self, tool: ToolMode, pos: Point) -> Result<(), String>;

    /// Render the tool palette panel.
    fn render_tool_palette(&mut self, ui: &mut egui::Ui);
}
```

### 3.4 Smart Naming Algorithm (`generate_unique_name`)

```
1. Start with base name (e.g., "Class").
2. Collect all existing element names from the model.
3. Find all names matching the pattern "{base}_{N}" where N is a positive integer.
4. Determine the next available suffix by scanning from 1 upward.
5. Return "{base}_{next_suffix}".

Special case: if no "{base}_1" exists, return "{base}_1".
Special case: if the model has a bare "{base}" (unlikely with auto-naming, but possible), treat it as suffix 0 and start from 1.
```

### 3.5 `create_element_for_tool` — Element Factory

```rust
fn create_element_for_tool(&self, tool: ToolMode) -> ModelElement {
    match tool {
        ToolMode::CreateClass => {
            let name = self.generate_unique_name("Class");
            ModelElement::Class(Class::new(&name))
        }
        ToolMode::CreateInterface => {
            let name = self.generate_unique_name("Interface");
            let mut iface = Interface::new(&name);
            iface.base.is_abstract = true; // Interfaces are always abstract
            ModelElement::Interface(iface)
        }
        ToolMode::CreateEnum => {
            let name = self.generate_unique_name("Enum");
            ModelElement::Enum(Enum::new(&name))
        }
        ToolMode::CreateDatatype => {
            let name = self.generate_unique_name("Datatype");
            ModelElement::Datatype(Datatype::new(&name))
        }
        ToolMode::CreatePackage => {
            let name = self.generate_unique_name("Package");
            ModelElement::Package(Package::new(&name))
        }
        ToolMode::Select => unreachable!("Select tool does not create elements"),
    }
}
```

---

## 4. XMI Changes

**None.** The XMI reader and writer are unchanged. Created elements will be persisted when the user saves the model (using the existing save pipeline from M16).

---

## 5. UI Changes

### 5.1 Layout Overview

```
┌────────────────────────────────────────────────────────────┐
│  File  Edit  [↩ Undo] [↪ Redo] │ Status message            │
├──────────┬─────────────────────────────────────────────────┤
│          │                                                  │
│  🖱 Sel. │                                                  │
│  📦 Class│                                                  │
│  🔌 Iface│                 CANVAS                           │
│  🔢 Enum │                                                  │
│  📋 DType│                                                  │
│  📁 Pkg  │                                                  │
│          │                                                  │
│ ──────── │                                                  │
│ Diagrams │                                                  │
│ ...      │                                                  │
│ Elements │                                                  │
│ ...      │                                                  │
│          │                                                  │
└──────────┴─────────────────────────────────────────────────┘
```

The tool palette is a vertical strip at the top of the existing left `SidePanel`. It sits above the "Diagrams" and "Elements" sections.

### 5.2 Tool Palette Panel

Rendered via `render_tool_palette()` at the top of the existing `SidePanel::left`:

```rust
fn render_tool_palette(&mut self, ui: &mut egui::Ui) {
    ui.heading("Tools");
    ui.add_space(4.0);
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
            self.status_message = format!("Tool: {}", tool.label());
        }
    }
    ui.separator();
}
```

**Behavior:**
- Clicking a tool sets `current_tool` and updates the status bar.
- The active tool is visually highlighted (egui `SelectableLabel` handles this).
- Selecting a creation tool resets `preview_position` to `None`.
- Selecting `Select` reverts to the default interaction mode.

### 5.3 Click-to-Place on Canvas

In `render_canvas`, the current code handles node interactions in a loop over `node_rects` (lines ~393–422). We add a **background interaction zone** after the node loop:

```rust
// ── Background click for creation tools ─────────────────────
if self.current_tool.is_creation_tool() {
    if let Some(diag_idx) = self.active_diagram {
        let diagram_id = self.model.diagrams()[diag_idx].id;
        let bg_rect = ui.max_rect();
        let bg_response = ui.interact(bg_rect, ui.next_auto_id(), egui::Sense::click());

        // Hover preview
        if bg_response.hovered() {
            if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                self.preview_position = Some(Point::new(
                    f64::from(pointer_pos.x),
                    f64::from(pointer_pos.y),
                ));
            }
        } else {
            self.preview_position = None;
        }

        // Click to create
        if bg_response.clicked() {
            if let Some(click_pos) = bg_response.interact_pointer_pos() {
                let pos = Point::new(f64::from(click_pos.x), f64::from(click_pos.y));
                if let Err(e) = self.place_element(self.current_tool, pos) {
                    self.status_message = format!("Error: {e}");
                }
                // Reset tool to Select after creation
                self.current_tool = ToolMode::Select;
                self.preview_position = None;
            }
        }
    } else {
        // No active diagram — show message on click attempt
        let bg_response = ui.interact(ui.max_rect(), ui.next_auto_id(), egui::Sense::click());
        if bg_response.clicked() {
            self.status_message = "No active diagram. Create a diagram first.".into();
        }
    }
}
```

**Key design decisions:**
- The tool auto-resets to `Select` after a successful placement. This prevents accidental duplicate placements and follows the convention of most UML tools (one-shot creation).
- If no diagram is active, the click shows a status message rather than silently failing.
- The background interaction uses a low-priority `Sense::click()` — it does not interfere with node dragging because node interactions are processed first in the existing loop.

### 5.4 Ghost Preview Rectangle

When a creation tool is active and the mouse hovers over the canvas, draw a semi-transparent preview rectangle at the cursor position:

```rust
// Draw ghost preview
if let Some(preview_pos) = self.preview_position {
    let preview_rect = egui::Rect::from_min_size(
        egui::pos2(preview_pos.x() as f32 - 80.0, preview_pos.y() as f32 - 30.0),
        egui::Vec2::new(160.0, 60.0),
    );
    ui.painter().rect_filled(
        preview_rect,
        4.0,
        egui::Color32::from_rgba_premultiplied(100, 100, 255, 40),
    );
    ui.painter().rect_stroke(
        preview_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(100, 100, 255, 120)),
        egui::StrokeKind::Inside,
    );
}
```

The preview dimensions (160×60) match the default `ViewNode` size used when creating the `New Class Diagram` (line ~308 in existing `app.rs`).

### 5.5 Cursor Feedback

When a creation tool is active, set the egui cursor to crosshair:

```rust
if self.current_tool.is_creation_tool() {
    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
}
```

This is set in `render_canvas` and/or at the top of `update()`.

### 5.6 Keyboard Shortcuts

Add canvas-level keyboard shortcuts in `update()`. These only apply when the canvas has focus (i.e., the user is interacting with it). Since egui immediate mode doesn't have explicit focus tracking, we check that no text input widget has captured keyboard focus:

```rust
// ── Tool keyboard shortcuts ──────────────────────────────────
if !ctx.wants_keyboard_input() {
    if ctx.input(|i| i.key_pressed(egui::Key::S)) {
        self.current_tool = ToolMode::Select;
        self.status_message = "Tool: Select".into();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::C)) {
        self.current_tool = ToolMode::CreateClass;
        self.status_message = "Tool: Class".into();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::I)) {
        self.current_tool = ToolMode::CreateInterface;
        self.status_message = "Tool: Interface".into();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::E)) {
        self.current_tool = ToolMode::CreateEnum;
        self.status_message = "Tool: Enum".into();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::D)) {
        self.current_tool = ToolMode::CreateDatatype;
        self.status_message = "Tool: Datatype".into();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::P)) {
        self.current_tool = ToolMode::CreatePackage;
        self.status_message = "Tool: Package".into();
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        self.current_tool = ToolMode::Select;
        self.status_message = "Tool: Select".into();
    }
}
```

Note: `ctx.wants_keyboard_input()` prevents shortcuts from firing while typing in a text field (relevant when the property editor is added in M18).

### 5.7 Left Panel Reorganization

The existing left `SidePanel` currently contains `render_tree()`, which renders the Diagrams list and Elements list. We restructure it so the tool palette appears first:

```rust
// In update():
egui::SidePanel::left("tree_panel")
    .resizable(true)
    .default_width(250.0)
    .show(ctx, |ui| {
        self.render_tool_palette(ui);   // NEW: tool palette at top
        ui.add_space(8.0);
        self.render_tree(ui);           // Existing diagrams + elements
    });
```

---

## 6. Test Plan

### 6.1 Unit Tests (in `app.rs` `#[cfg(test)]`)

| # | Test Name | What It Verifies |
|---|-----------|-----------------|
| T1 | `tool_mode_defaults_to_select` | `UmbrelloApp::new()` initializes `current_tool` to `Select` |
| T2 | `tool_mode_select_label` | `ToolMode::Select.label()` returns a non-empty string |
| T3 | `tool_mode_is_creation_tool` | `CreateClass`, `CreateInterface`, etc. return `true`; `Select` returns `false` |
| T4 | `generate_unique_name_first` | In an empty model, `generate_unique_name("Class")` returns `"Class_1"` |
| T5 | `generate_unique_name_increments` | With `"Class_1"` present, returns `"Class_2"` |
| T6 | `generate_unique_name_finds_gap` | With `"Class_1"` and `"Class_3"` present, returns `"Class_2"` (gap fill) |
| T7 | `create_element_for_tool_class` | `create_element_for_tool(CreateClass)` returns a `ModelElement::Class` with a unique name |
| T8 | `create_element_for_tool_package` | `create_element_for_tool(CreatePackage)` returns a `ModelElement::Package` with a unique name |
| T9 | `place_element_creates_in_model` | After `place_element(CreateClass, pos)`, the model contains the new element |
| T10 | `place_element_adds_node_to_diagram` | After `place_element`, the active diagram has a `ViewNode` for the new element |
| T11 | `place_element_dirty_flag` | After `place_element`, `is_dirty` is `true` |
| T12 | `tool_resets_after_placement` | After `place_element`, `current_tool` is `Select` |
| T13 | `place_element_undo_removes_both` | Undo after `place_element` removes both the element and the ViewNode |
| T14 | `selection_persists_before_click` | Clicking on canvas with `Select` tool does not change `current_tool` |

### 6.2 Rendering Tests

| # | Test Name | What It Verifies |
|---|-----------|-----------------|
| T15 | `new_element_visible_on_canvas` | After creating an element with the tool, it appears in the rendered element list |
| T16 | `tool_palette_buttons_exist` | The tool palette contains buttons for all 6 tools |
| T17 | `element_color_for_new_type` | The `element_color` function returns correct color for the created element type |

### 6.3 Manual Verification

- Launch app with empty model → Tool palette visible with Select highlighted
- Click "Class" in tool palette → status bar shows "Tool: 📦 Class"
- Hover over canvas → crosshair cursor appears, ghost rectangle follows pointer
- Click canvas → Class element appears at click location, tool resets to Select
- Check left panel "Elements" list → new element appears with name "Class_1"
- Press `C` key → tool switches to Class
- Create multiple classes → names auto-increment: "Class_1", "Class_2", "Class_3"
- Create Class → Undo (Ctrl+Z) → element and node both removed
- Create Class → Save → Open → element persists (round-trip via existing M16 pipeline)
- Click canvas with no active diagram → status message "No active diagram. Create a diagram first."
- New element types render with correct colors and compartments (via existing M15 rendering)

---

## 7. Implementation Sub-Task Order

| Order | Sub-task | Key Changes | Lines (est.) |
|-------|----------|-------------|--------------|
| **A** | `ToolMode` enum + `current_tool` field | Add enum definition, add field to `UmbrelloApp`, initialize in `new()` | ~40 |
| **B** | `generate_unique_name` method | Implement name scanning algorithm, add `name_counters` field (or scan inline) | ~30 |
| **C** | `create_element_for_tool` factory | Match on `ToolMode`, construct appropriate `ModelElement` variant | ~35 |
| **D** | `place_element` method | Create element + add ViewNode to active diagram via commands | ~40 |
| **E** | `render_tool_palette` method | Vertical toolbar with `SelectableLabel` buttons, highlight active tool | ~30 |
| **F** | Integrate tool palette into left panel | Add `render_tool_palette(ui)` call in `update()` before `render_tree(ui)` | ~5 |
| **G** | Background click handler in `render_canvas` | Add `bg_response` + `place_element` call + auto-reset logic | ~40 |
| **H** | Ghost preview rectangle | Draw semi-transparent rect at `preview_position` | ~20 |
| **I** | Crosshair cursor | `set_cursor_icon(Crosshair)` when creation tool active | ~5 |
| **J** | Keyboard shortcuts | Add tool shortcuts (S, C, I, E, D, P, Esc) in `update()` | ~35 |
| **K** | Tests | T1–T17 unit + rendering tests | ~150 |

**Total estimated new lines:** ~430  
**Total estimated changed lines (existing code modified):** ~40  
**Files modified:** 1 (`apps/umbrello/src/app.rs`)

---

## 8. Architecture Compliance

- **`uml-core` is untouched** — all needed types, enums, and commands already exist.
- **All mutations via Commands** — `place_element` uses `execute_command()` which wraps `History::execute()` with dirty tracking (built in M16).
- **No XMI changes** — created elements are saved via the existing `XmiWriter` pipeline (M10, M16).
- **No inheritance emulation** — `ToolMode` is a plain enum with pattern matching, consistent with the `ModelElement` dispatch pattern.
- **egui immediate mode** — all tool palette state is stored in `UmbrelloApp` fields; no persistent widget state or retained-mode patterns.
- **`cargo test --workspace` must pass** — all 216 existing tests remain green; ~17 new tests added.
- **Two commands per element placement** — `CreateElement` + `AddNodeToDiagram` are executed as separate commands (no macro/group command needed for M17). Each is individually undoable. A future milestone may add command grouping for single-step undo.

---

## 9. Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Background click handler interferes with node dragging | Low | Node interactions are processed first in the existing loop. The background handler only activates for `Sense::click()` (not drag), so no conflict with `Sense::click_and_drag()`. |
| Name collision in `generate_unique_name` if elements are added externally (e.g., XMI load) | Low | The function scans all existing names at call time, not a static counter. XMI-loaded elements with names like "Class_1" are correctly accounted for. |
| Keyboard shortcut `S` conflicts with Ctrl+S (Save) | Low | The shortcut check uses `key_pressed()` without modifiers. Ctrl+S is consumed earlier by the modifier-aware shortcut handling (M16). The check uses `!ctx.wants_keyboard_input()` to avoid firing in text fields. |
| Ghost preview flickers during rapid mouse movement | Low | egui repaints at vsync (typically 60 fps). The preview position is updated from `pointer_latest_pos()` each frame. Flickering is not expected at 60 fps. |
| `S` key for Select conflicts with future text input in property editor | Medium | The `!ctx.wants_keyboard_input()` guard prevents this. When a text field has focus, keyboard shortcuts are suppressed. This is future-proof for M18. |
| Two separate commands mean undo requires two steps | Low (accepted) | This is consistent with the current command model. A `CompositeCommand` can be added later (LOW priority gap) to group Create+Place into a single undo step. |

---

## 10. Relation to Subsequent Milestones

| Upcoming Milestone | How M17 Enables It |
|--------------------|-------------------|
| M18 — Property Editor | Created elements can be selected and their properties edited |
| M19 — Edge Creation | Created elements can be connected via relationships |
| Future — Actor/UseCase types | Adding new element types to `ToolMode` enum + `create_element_for_tool` is trivial (one enum variant + one match arm each) |
| Future — Macro Commands | `place_element` can be refactored to use a `CompositeCommand` when available |

---

*Design document v1 — ready for implementation.*
