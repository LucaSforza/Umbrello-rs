# Milestone 18 — Property Editor Panel

**Status:** Design v1  
**Target Milestone:** M18  
**Dependencies:** M17 (Tool Palette & Interactive Element Creation), M11 (Undo/Redo — RenameElement command)

---

## 1. Objective

M17 delivered a tool palette that lets users create UML elements by clicking on the canvas. However, every element arrives with a generated name (`Class_1`, `Package_3`), default visibility (Public), empty documentation, and an empty classifier body. There is **no way to inspect or modify any property** of a created or loaded element. Without a property editor, the tool cannot even rename a class.

M18 closes this gap by providing:

1. **Selection tracking** — clicking a node on the canvas highlights it with a colored selection border and populates a right-side inspector panel.
2. **Read-only property summary** — the panel shows the selected element's type, internal ID, stereotype (if any), and classifier details (attribute/operation counts).
3. **Editable name** — a single-line text field that fires the existing `RenameElement` command when the user presses Enter or moves focus away.
4. **Visibility dropdown** — a combo box cycling through Public (+), Protected (#), Private (-), and Implementation (~), backed by a new `ChangeVisibility` command.
5. **Abstract / Static toggle checkboxes** — checked/unchecked state mirrors `ElementBase::is_abstract` and `ElementBase::is_static`, backed by a new `ChangeElementFlags` command.
6. **Documentation text area** — a multi-line edit box for the element's documentation/comment string, backed by a new `ChangeDocumentation` command.
7. **Read-only classifier display** — for Class, Interface, Enum, and Datatype elements, the panel lists attributes and operations by name and type (editing them is deferred to a future milestone to keep M18 self-contained).
8. **Deselection** — clicking the canvas background or pressing Escape clears the selection and restores the "nothing selected" placeholder.

**Out of scope:** Attribute/operation creation or editing, stereotype assignment, relationship property editing, element deletion from the property panel, resizing nodes dynamically to match content, drag-to-create edges, context menus. No changes to `uml-io` or `uml-codegen`.

---

## 2. Prerequisite: Split `apps/umbrello/src/app.rs`

Before implementing any M18 feature, the `@implementer` must **first** split the monolithic `app.rs` (~1652 lines) into multiple source files under `apps/umbrello/src/`. The split must pass all existing 233 tests and compile cleanly with `cargo clippy`.

### 2.1 Target File Structure

```
apps/umbrello/src/
├── main.rs                    # Entry point (UNCHANGED except `mod` declarations)
├── app.rs                     # UmbrelloApp struct + eframe::App impl + top-level orchestration (~300 lines)
├── tool_palette.rs            # ToolMode enum, create_element_for_tool, render_tool_palette (~150 lines)
├── canvas.rs                  # render_canvas, draw_partitioned_node, draw_edges, ghost preview (~450 lines)
├── rendering.rs               # Free functions: element_color, visibility_symbol, type_display, arrowhead drawing (~200 lines)
├── menu.rs                    # render_menu, menu_file_* helpers (~150 lines)
├── tree.rs                    # render_tree (left panel), diagram list, element list (~150 lines)
├── file_io.rs                 # File I/O helpers: prompt_save_if_dirty, save/load orchestration (~100 lines)
└── tests.rs                   # All #[cfg(test)] mod tests extracted from app.rs (~270 lines)
```

### 2.2 Split Rules

| Rule | Details |
|------|---------|
| **No logic changes** | Move code verbatim. Do not refactor, rename, or restructure functions during the split. |
| **Imports per file** | Each new file imports exactly what it uses. `use super::*` is acceptable for module-internal references. |
| **`pub(crate)` visibility** | All types and functions needed across files use `pub(crate)`. No `pub` unless truly public API. |
| **Tests in isolation** | All `#[cfg(test)]` blocks move into `tests.rs`. Import the parent module with `use super::*`. |
| **`main.rs` `mod` declarations** | `main.rs` declares all modules: `mod app; mod tool_palette; mod canvas; mod rendering; mod menu; mod tree; mod file_io; mod tests;` |
| **`app.rs` re-exports** | `app.rs` may `pub(crate) use` items from submodules for backward compatibility during the split. |
| **Commit point** | The implementer commits the split **before** beginning M18 feature work, with message `"refactor: split app.rs into modular source files"`. |

### 2.3 Verification (Split Commit)

```sh
cargo build -p umbrello          # Must compile
cargo test --workspace            # All 233 tests must pass
cargo clippy -p umbrello -- -D warnings  # Zero warnings
cargo fmt --all --check           # No formatting diffs
```

---

## 3. Crates to Modify

| Crate | Changes | Rationale |
|-------|---------|-----------|
| `apps/umbrello` | **Primary** — selection state, property panel UI, editable fields, wiring to commands | All GUI interaction logic lives here |
| `uml-core` | **Light touch** — 3 new undo commands in `undo/commands.rs`; re-export from `undo/mod.rs` and `lib.rs` | Property mutations must be undoable |
| `uml-io` | **Zero changes** | No persistence changes needed |
| `uml-codegen` | **Zero changes** | — |

**No new dependencies.** All functionality uses existing crates (egui, uml-core).

---

## 4. New Types, Fields, and Functions

### 4.1 `UmbrelloApp` — New Fields (in `app.rs`)

```rust
pub(crate) struct UmbrelloApp {
    // ... existing fields ...

    /// The currently selected element on the canvas, if any.
    /// Set by clicking a node; cleared by clicking background or pressing Escape.
    selected_element_id: Option<UmlId>,

    /// Cached property-panel edit buffer for the name field.
    /// Populated when a new element is selected; flushed to RenameElement on commit.
    name_edit_buffer: String,
}
```

Initialized to `None` / `String::new()` in `UmbrelloApp::new()`.

### 4.2 New Undo Commands (in `crates/uml-core/src/undo/commands.rs`)

#### `ChangeVisibility`

```rust
/// Command to change an element's visibility level.
#[derive(Debug)]
pub struct ChangeVisibility {
    element_id: UmlId,
    old_visibility: Visibility,
    new_visibility: Visibility,
    description: String,
}

impl ChangeVisibility {
    pub fn new(model: &UmlModel, id: UmlId, visibility: Visibility) -> Result<Self, CommandError>;
}

impl Command for ChangeVisibility {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn description(&self) -> &str;
}
```

Pattern: identical to `RenameElement` — snapshots old value on construction, applies new value on `execute`, restores old value on `undo`.

#### `ChangeElementFlags`

```rust
/// Command to toggle abstract/static flags on an element.
#[derive(Debug)]
pub struct ChangeElementFlags {
    element_id: UmlId,
    is_abstract: bool,
    is_static: bool,
    old_abstract: bool,
    old_static: bool,
    description: String,
}

impl ChangeElementFlags {
    pub fn new(model: &UmlModel, id: UmlId, is_abstract: bool, is_static: bool) -> Result<Self, CommandError>;
}

impl Command for ChangeElementFlags { /* ... */ }
```

Both flags are set atomically in a single command (a checkbox toggle that changed only one flag would still snapshot both, so a pair of rapid clicks merges cleanly). The `description` reads e.g. `"Set flags of 'MyClass': abstract=true, static=false"`.

#### `ChangeDocumentation`

```rust
/// Command to change an element's documentation text.
#[derive(Debug)]
pub struct ChangeDocumentation {
    element_id: UmlId,
    old_documentation: String,
    new_documentation: String,
    description: String,
}

impl ChangeDocumentation {
    pub fn new(model: &UmlModel, id: UmlId, documentation: String) -> Result<Self, CommandError>;
}

impl Command for ChangeDocumentation { /* ... */ }
```

### 4.3 Re-exports

In `crates/uml-core/src/undo/mod.rs`:
```rust
pub use commands::{..., ChangeVisibility, ChangeElementFlags, ChangeDocumentation};
```

In `crates/uml-core/src/lib.rs`:
```rust
// Already re-exports commands. Confirm the three new types are included.
```

---

## 5. XMI Changes

None. The property editor operates on in-memory `UmlModel` instances. No new XMI elements or attributes are introduced in M18.

---

## 6. UI Changes (all in `apps/umbrello/src/`)

### 6.1 Selection Highlight (in `canvas.rs`)

When `selected_element_id` is `Some(id)` and a `ViewNode` matches that ID, draw an additional selection border on top of the normal node border:

```
Stroke: 2.5 px, Color32::from_rgb(0, 120, 215)  // blue selection ring
```

Apply after the node's normal border stroke so it paints on top.

### 6.2 Canvas Click Handling (in `canvas.rs`)

Modify the existing `response.clicked()` handler in `render_canvas`:

```rust
if response.clicked() {
    self.selected_element_id = Some(model_element_id);
    // Populate name_edit_buffer with current element name
    if let Some(elem) = self.model.get(model_element_id) {
        self.name_edit_buffer = elem.name().to_string();
    }
    self.status_message = format!("Selected: {}", name);
}
```

Add a background click handler (sibling to the existing ghost-preview background handler) that clears selection when the user clicks empty canvas:

```rust
// In render_canvas, after all node interactions:
if !self.current_tool.is_creation_tool() {
    let bg = ui.interact(ui.max_rect(), ui.next_auto_id(), egui::Sense::click());
    if bg.clicked() {
        self.selected_element_id = None;
        self.name_edit_buffer.clear();
    }
}
```

**Important:** This background click handler must NOT interfere with the existing creation-tool background click handler. When a creation tool is active, clicking the canvas creates an element; clearing selection is not needed.

### 6.3 Escape to Deselect (in `app.rs` `update()` method)

```rust
// In the keyboard shortcut section:
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
    if self.selected_element_id.is_some() {
        self.selected_element_id = None;
        self.name_edit_buffer.clear();
        self.status_message = "Selection cleared".into();
    } else {
        self.current_tool = ToolMode::Select;
    }
}
```

### 6.4 Right-Side Property Panel (new file: `property_editor.rs`)

Create `apps/umbrello/src/property_editor.rs` containing:

```rust
use crate::app::UmbrelloApp;

impl UmbrelloApp {
    /// Render the right-side property editor panel.
    pub(crate) fn render_property_editor(&mut self, ui: &mut egui::Ui) {
        // See subsection 6.5 for detailed layout
    }
}
```

The panel is displayed via `egui::SidePanel::right("property_panel")` in the `eframe::App::update()` method, **after** the CentralPanel so it renders on the right:

```rust
egui::SidePanel::right("property_panel")
    .resizable(true)
    .default_width(280.0)
    .show(ctx, |ui| {
        self.render_property_editor(ui);
    });
```

### 6.5 Property Editor Panel Layout

```
┌─────────────────────────────────┐
│ Properties                      │  ← heading
├─────────────────────────────────┤
│ Nothing selected                │  ← shown when selected_element_id is None
│                                 │
│ Click a node on the canvas to   │
│ inspect its properties.         │
└─────────────────────────────────┘

When an element IS selected:

┌─────────────────────────────────┐
│ Properties                      │
├─────────────────────────────────┤
│ Type:      Class                │  ← read-only label
│ ID:        550e8400-e29b-...    │  ← read-only (first 20 chars)
│                                 │
│ Name:      [MyClass___________] │  ← editable TextEdit
│                                 │
│ Visibility:[Public          ▾] │  ← ComboBox (4 options)
│                                 │
│ ☐ Abstract   ☐ Static          │  ← Checkboxes
│                                 │
│ Documentation:                  │
│ ┌─────────────────────────────┐ │
│ │ This class represents...    │ │  ← multi-line TextEdit
│ │                             │ │
│ └─────────────────────────────┘ │
├─────────────────────────────────┤
│ Classifier Details              │  ← shown only for classifiers
├─────────────────────────────────┤
│ Attributes (3):                 │
│   + name : String               │  ← read-only list
│   - count : int                 │
│   # ratio : double              │
│                                 │
│ Operations (2):                 │
│   + getName() : String          │
│   - setCount(n : int) : void    │
└─────────────────────────────────┘
```

#### 6.5.1 "Nothing selected" Placeholder

```rust
if self.selected_element_id.is_none() {
    ui.add_space(20.0);
    ui.centered_and_justified(|ui| {
        ui.label(egui::RichText::new("Nothing selected").size(14.0).weak());
    });
    ui.add_space(8.0);
    ui.label("Click a node on the canvas to inspect its properties.");
    return;
}
```

#### 6.5.2 Read-Only Fields

```rust
let elem = self.model.get(selected_id).unwrap();
ui.label(format!("Type: {}", elem.object_type().as_str()));
ui.label(format!("ID: {}", &elem.id().to_string()[..20]));
ui.add_space(6.0);
```

#### 6.5.3 Editable Name

```rust
ui.horizontal(|ui| {
    ui.label("Name:");
    let response = ui.add(
        egui::TextEdit::singleline(&mut self.name_edit_buffer)
            .desired_width(ui.available_width())
    );
    // Commit rename on Enter or focus loss
    if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
        || response.lost_focus()
    {
        let new_name = self.name_edit_buffer.trim().to_string();
        if !new_name.is_empty() && new_name != elem.name() {
            if let Ok(cmd) = commands::RenameElement::new(&self.model, selected_id, new_name.clone()) {
                self.execute_command(Box::new(cmd));
                // Re-populate buffer to match the model
                self.name_edit_buffer = new_name;
            }
        }
    }
});
ui.add_space(4.0);
```

#### 6.5.4 Visibility Dropdown

```rust
ui.horizontal(|ui| {
    ui.label("Visibility:");
    let current_vis = elem.visibility();
    let vis_label = format!("{} {}", visibility_symbol(current_vis), visibility_name(current_vis));
    egui::ComboBox::from_id_salt("visibility_combo")
        .selected_text(vis_label)
        .show_ui(ui, |ui| {
            for vis in &[
                Visibility::Public,
                Visibility::Protected,
                Visibility::Private,
                Visibility::Implementation,
            ] {
                let label = format!("{} {}", visibility_symbol(*vis), visibility_name(*vis));
                if ui.selectable_label(current_vis == *vis, label).clicked() {
                    if *vis != current_vis {
                        if let Ok(cmd) = commands::ChangeVisibility::new(&self.model, selected_id, *vis) {
                            self.execute_command(Box::new(cmd));
                        }
                    }
                }
            }
        });
});
ui.add_space(4.0);
```

Helper function (in `rendering.rs` or `property_editor.rs`):

```rust
pub(crate) fn visibility_name(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "Public",
        Visibility::Protected => "Protected",
        Visibility::Private => "Private",
        Visibility::Implementation => "Impl",
    }
}
```

#### 6.5.5 Abstract / Static Checkboxes

```rust
ui.horizontal(|ui| {
    let base = elem.base();
    let mut new_abstract = base.is_abstract;
    let mut new_static = base.is_static;
    let changed_abs = ui.checkbox(&mut new_abstract, "Abstract").changed();
    let changed_sta = ui.checkbox(&mut new_static, "Static").changed();

    if changed_abs || changed_sta {
        if let Ok(cmd) = commands::ChangeElementFlags::new(
            &self.model, selected_id, new_abstract, new_static,
        ) {
            self.execute_command(Box::new(cmd));
        }
    }
});
ui.add_space(6.0);
```

#### 6.5.6 Documentation TextEdit

```rust
ui.label("Documentation:");
let base = elem.base();
let mut doc = base.documentation.clone();
let doc_edit = ui.add(
    egui::TextEdit::multiline(&mut doc)
        .desired_rows(3)
        .desired_width(ui.available_width())
);
if doc_edit.changed() {
    // Don't commit on every keystroke — commit on Enter or focus loss
}
if doc_edit.lost_focus() && doc != base.documentation {
    if !doc.trim().is_empty() || !base.documentation.is_empty() {
        if let Ok(cmd) = commands::ChangeDocumentation::new(&self.model, selected_id, doc) {
            self.execute_command(Box::new(cmd));
        }
    }
}
```

> **Note on commit strategy:** For `TextEdit::multiline`, committing on every keystroke would flood the undo stack. The M18 strategy commits only on **focus loss** (user tabs/clicks away). A future improvement could add a debounced commit timer, but that is out of scope.

#### 6.5.7 Classifier Details (Read-Only)

```rust
if let Some(classifier) = elem.classifier_data() {
    ui.separator();
    ui.heading("Classifier Details");
    ui.add_space(4.0);

    ui.label(format!("Attributes ({}):", classifier.attributes.len()));
    for attr in &classifier.attributes {
        let vis = visibility_symbol(attr.visibility);
        let type_name = attr.type_ref.display_name(Some(&self.model));
        ui.label(format!("  {} {}: {}", vis, attr.name, type_name));
    }

    ui.add_space(4.0);
    ui.label(format!("Operations ({}):", classifier.operations.len()));
    for op in &classifier.operations {
        let vis = visibility_symbol(op.visibility);
        let params: Vec<String> = op.parameters.iter()
            .map(|p| format!("{}: {}", p.name, p.type_ref.display_name(Some(&self.model))))
            .collect();
        let ret = op.return_type.display_name(Some(&self.model));
        ui.label(format!("  {} {}({}): {}", vis, op.name, params.join(", "), ret));
    }
}
```

### 6.6 Use of `execute_command` Helper

All property mutations use the existing `UmbrelloApp::execute_command()` method (introduced in M16), which wraps `History::execute()` with automatic dirty-flag tracking:

```rust
fn execute_command(&mut self, cmd: Box<dyn Command>) {
    if self.history.execute(cmd, &mut self.model).is_ok() {
        self.is_dirty = true;
    }
}
```

### 6.7 Keyboard Shortcuts (in `app.rs` `update()`)

No new keyboard shortcuts beyond the Escape-to-deselect behavior described in §6.3.

---

## 7. Test Plan

### 7.1 `uml-core` — New Command Tests (in `crates/uml-core/src/undo/commands.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| CMD-01 | `change_visibility_execute` | `ChangeVisibility::execute` sets Visibility::Private on a Class |
| CMD-02 | `change_visibility_undo` | `ChangeVisibility::undo` restores original Visibility::Public |
| CMD-03 | `change_visibility_new_element_not_found` | `ChangeVisibility::new` returns `Err(ElementNotFound)` for unknown ID |
| CMD-04 | `change_flags_execute` | `ChangeElementFlags::execute` sets abstract=true, static=true |
| CMD-05 | `change_flags_undo` | `ChangeElementFlags::undo` restores both flags to false |
| CMD-06 | `change_flags_new_element_not_found` | `ChangeElementFlags::new` returns `Err(ElementNotFound)` for unknown ID |
| CMD-07 | `change_documentation_execute` | `ChangeDocumentation::execute` sets doc to "A test class" |
| CMD-08 | `change_documentation_undo` | `ChangeDocumentation::undo` restores empty doc string |
| CMD-09 | `change_documentation_new_element_not_found` | `ChangeDocumentation::new` returns `Err(ElementNotFound)` for unknown ID |

### 7.2 `apps/umbrello` — Selection & Panel Tests (in `tests.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| APP-01 | `selected_element_id_defaults_to_none` | New `UmbrelloApp` has `selected_element_id: None` |
| APP-02 | `select_node_sets_selected_element_id` | After `simulate_click_on_node`, `selected_element_id` is `Some(id)` |
| APP-03 | `deselect_on_background_click` | After selection, `clear_selection()` sets `selected_element_id` to `None` |
| APP-04 | `name_edit_buffer_populates_on_selection` | After selecting a Class named "MyClass", `name_edit_buffer` equals `"MyClass"` |
| APP-05 | `rename_element_via_property_editor` | Programmatically set `name_edit_buffer` to `"Renamed"` and call rename, verify model reflects new name |
| APP-06 | `visibility_dropdown_changes_visibility` | Construct `ChangeVisibility` cmd, execute it, verify element visibility is now `Private` |
| APP-07 | `visibility_change_undo_restores` | Execute `ChangeVisibility`, undo, verify visibility is back to `Public` |
| APP-08 | `flag_toggle_sets_abstract_and_static` | Construct `ChangeElementFlags` cmd with `(true, true)`, execute, verify both flags |
| APP-09 | `flag_toggle_undo_restores_flags` | Execute `ChangeElementFlags(true, true)`, undo, verify both flags are `false` |
| APP-10 | `documentation_edit_persists` | Construct `ChangeDocumentation` cmd with `"Hello"`, execute, verify `base.documentation == "Hello"` |
| APP-11 | `documentation_change_undo_reverts` | Execute `ChangeDocumentation("Hello")`, undo, verify doc is empty |
| APP-12 | `classifier_details_displayed_for_class` | Verify property panel shows "Classifier Details" section for a Class |
| APP-13 | `classifier_details_hidden_for_package` | Verify property panel does NOT show "Classifier Details" for a Package |
| APP-14 | `property_editor_placeholder_when_none_selected` | Verify the "Nothing selected" message appears when `selected_element_id` is `None` |
| APP-15 | `dirty_flag_set_on_property_change` | After executing a `ChangeVisibility` via `execute_command`, verify `is_dirty == true` |

> **Note on test isolation:** The app-level tests in `tests.rs` do not require an egui `Context`. They exercise the `UmbrelloApp` data model directly by calling command constructors and `execute_command()`. The panel rendering code is tested implicitly by the unit tests; automated screenshot/inspection of egui panels is deferred.

### 7.3 Test-Friendly Design

To make `tests.rs` tests clean without an egui context, the `execute_command` method and all command constructors operate on `&mut UmlModel` and `&mut UmbrelloApp` — no egui state required. The file `property_editor.rs` should keep panel-layout logic separate from business logic:

- **Business logic** (command dispatch, name validation, type checks) lives in methods on `UmbrelloApp` and is testable without UI.
- **Layout logic** (`egui::SidePanel`, `ui.label(...)`, `ui.text_edit_singleline(...)`) lives in `render_property_editor()` and is tested visually/manually.

### 7.4 Verification Commands

```sh
# Unit tests for new commands
cargo test -p uml-core change_visibility
cargo test -p uml-core change_flags
cargo test -p uml-core change_documentation

# App-level selection + panel tests
cargo test -p umbrello selected_element
cargo test -p umbrello property_editor
cargo test -p umbrello visibility_change
cargo test -p umbrello flag_toggle
cargo test -p umbrello documentation_edit

# Full suite
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

---

## 8. Implementation Sequence

The implementer MUST follow this order:

### Phase 0: File Split (Commit 1)
1. Create all new module files (`tool_palette.rs`, `canvas.rs`, `rendering.rs`, `menu.rs`, `tree.rs`, `file_io.rs`, `property_editor.rs`, `tests.rs`).
2. Move code verbatim from `app.rs` into each module.
3. Update `main.rs` with `mod` declarations.
4. Update `app.rs` with `pub(crate) use` re-exports where needed.
5. Verify: `cargo build -p umbrello && cargo test --workspace && cargo clippy -p umbrello -- -D warnings`
6. **Commit** with message: `"refactor: split app.rs into modular source files"`

### Phase 1: New Commands (Commit 2)
1. Add `ChangeVisibility`, `ChangeElementFlags`, `ChangeDocumentation` to `crates/uml-core/src/undo/commands.rs`.
2. Re-export from `undo/mod.rs` and `lib.rs`.
3. Write all 9 command unit tests (CMD-01 through CMD-09).
4. Verify: `cargo test -p uml-core && cargo clippy -p uml-core -- -D warnings`
5. **Commit** with message: `"feat(uml-core): add ChangeVisibility, ChangeElementFlags, ChangeDocumentation commands"`

### Phase 2: Selection State (Commit 3)
1. Add `selected_element_id` and `name_edit_buffer` fields to `UmbrelloApp`.
2. Implement selection highlight border in `canvas.rs`.
3. Implement background click handler for deselection.
4. Add Escape key deselection to `app.rs` `update()`.
5. Write APP-01 through APP-04 tests in `tests.rs`.
6. Verify: `cargo test -p umbrello && cargo clippy -p umbrello -- -D warnings`
7. **Commit** with message: `"feat(app): add element selection tracking and highlight"`

### Phase 3: Property Editor Panel (Commit 4)
1. Create `property_editor.rs` with `render_property_editor()` containing:
   - "Nothing selected" placeholder
   - Read-only Type/ID display
   - Editable name field → `RenameElement`
   - Visibility dropdown → `ChangeVisibility`
   - Abstract/Static checkboxes → `ChangeElementFlags`
   - Documentation text area → `ChangeDocumentation`
   - Read-only classifier details section
2. Wire panel into `app.rs` `update()` via `egui::SidePanel::right`.
3. Add `visibility_name` helper to `rendering.rs`.
4. Write APP-05 through APP-15 tests.
5. Verify: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
6. **Commit** with message: `"feat(app): add property editor panel with editable name, visibility, flags, documentation"`

### Final Verification
```sh
cargo test --workspace            # All tests pass (expected: ~260+)
cargo clippy --workspace --all-targets -- -D warnings  # Zero warnings
cargo fmt --all --check           # No formatting diffs
```

---

## 9. File Summary (After M18)

```
apps/umbrello/src/
├── main.rs              # Entry point (mod declarations)
├── app.rs               # UmbrelloApp struct + eframe::App impl + top-level orchestration
├── tool_palette.rs      # ToolMode enum, create_element_for_tool, render_tool_palette
├── canvas.rs            # render_canvas, draw_partitioned_node, draw_edges, ghost preview, selection highlight
├── rendering.rs         # element_color, visibility_symbol, type_display, visibility_name, arrowhead drawing
├── menu.rs              # render_menu, menu_file_* helpers
├── tree.rs              # render_tree (left panel), diagram list, element list
├── file_io.rs           # prompt_save_if_dirty, save/load orchestration
├── property_editor.rs   # render_property_editor (right panel)
└── tests.rs             # All #[cfg(test)] mod tests

crates/uml-core/src/undo/
├── mod.rs               # Re-exports: +ChangeVisibility, +ChangeElementFlags, +ChangeDocumentation
└── commands.rs          # +3 new commands (~150 lines added, ~540→690 total)
```

---

## 10. Design Decisions

| Decision | Rationale |
|----------|-----------|
| Commit on focus-loss, not keystroke | Prevents flooding the undo stack with intermediate edits. Users expect undo to revert to the pre-edit state, not undo character-by-character. |
| `ChangeElementFlags` bundles both flags | Two checkboxes often get toggled together. A single command with both values avoids two undo steps for what the user perceives as one property change. The `merge()` method can be added later for adjacent flag-only changes. |
| Read-only classifier display | Editing attributes/operations requires dom-modal dialogs and complex list management (insert/reorder/delete). Keeping it read-only for M18 ensures the milestone stays focused and deliverable. Full classifier editing is a natural M19/M20 candidate. |
| Selection is per-app, not per-diagram | The user can see only one diagram at a time (no tabs yet). A single selection variable is sufficient. When multiple diagram tabs arrive (MEDIUM priority), selection can be scoped per-diagram. |
| `name_edit_buffer` as a cached field | The egui `TextEdit` widget requires a mutable `&mut String` with the same lifetime as the widget. Storing the buffer in `UmbrelloApp` avoids borrow-splitting gymnastics between the immutable model lookup and the mutable buffer. |
| No changes to `uml-io` | The property editor only modifies in-memory state. Saving the model writes everything via the existing XMI writer. |

---

*Last updated: 2026-06-26 · Umbrello-RS Milestone 18 Design v1*
