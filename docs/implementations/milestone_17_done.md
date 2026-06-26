# Milestone 17 — Tool Palette & Interactive Element Creation — Completion Report

**Date:** 2026-06-26  
**Branch:** (implementation branch)  
**Files modified:** 1 (`apps/umbrello/src/app.rs`)

---

## Summary

Implemented a tool palette (vertical toolbar) in the left panel of the GUI app, enabling users to select a UML element type (Class, Interface, Enum, Datatype, Package) and click on the canvas to create and place new elements. All changes are confined to `apps/umbrello/src/app.rs` with zero changes to `uml-core` or `uml-io`.

---

## What Was Built

### A. `ToolMode` Enum
- 6 variants: `Select`, `CreateClass`, `CreateInterface`, `CreateEnum`, `CreateDatatype`, `CreatePackage`
- Methods: `label()`, `tooltip()`, `is_creation_tool()`
- Added `current_tool: ToolMode` field to `UmbrelloApp`, initialized to `Select`

### B. `generate_unique_name(&self, base: &str) -> String`
- Scans all existing model element names
- Finds names matching `"{base}_{N}"` and determines the next available suffix
- Returns `"{base}_{next}"` (e.g., `"Class_1"`, `"Package_3"`)
- Handles gaps correctly (if `Class_1` and `Class_3` exist, returns `Class_2`)

### C. `create_element_for_tool(&self, tool: ToolMode) -> ModelElement`
- Factory method matching on `ToolMode`
- Constructs the appropriate element variant with auto-generated name
- Interface elements set `is_abstract = true` (as per UML semantics)

### D. `place_element(&mut self, tool: ToolMode, pos: Point) -> Result<(), String>`
- Executes two commands via `execute_command`: `CreateElement` + `AddNodeToDiagram`
- Returns error string if no active diagram
- Uses `Size::new(160.0, 60.0)` as the default ViewNode size

### E. `render_tool_palette(&mut self, ui: &mut egui::Ui)`
- Vertical toolbar with `SelectableLabel` buttons for each tool
- Active tool is visually highlighted
- Clicking a tool sets `current_tool`, resets `preview_position`, updates status message
- Includes separator below the palette

### F. Integration into Left Panel
- `render_tool_palette(ui)` called at the top of the `SidePanel::left` closure
- `render_tree(ui)` follows with spacing

### G. Background Click Handler in `render_canvas`
- When creation tool is active and mouse hovers over canvas: sets `preview_position` for ghost preview
- On click: calls `place_element`, auto-resets tool to `Select`
- If no active diagram: shows "No active diagram. Create a diagram first." status message
- Uses `Sense::click()` (not drag) to avoid interfering with node interactions

### H. Ghost Preview Rectangle
- Semi-transparent blue rectangle (160×60, rounded corners radius 4)
- Centered on cursor position
- Drawn when `preview_position` is `Some`

### I. Crosshair Cursor
- `ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair)` when `current_tool.is_creation_tool()`
- Set at the top of `render_canvas` for immediate visual feedback

### J. Keyboard Shortcuts
- Only when `!ctx.wants_keyboard_input()`:
  - S = Select, C = Class, I = Interface, E = Enum, D = Datatype, P = Package, Esc = Select
- Uses `consume_key` with `Modifiers::NONE` to avoid conflicts with Ctrl+ shortcuts
- Status message updated on tool change

### K. Additional Fields
- `name_counters: HashMap<String, u64>` — (reserved for future cache optimization)
- `preview_position: Option<Point>` — ghost rectangle position in canvas coordinates

---

## Files Modified

| File | Changes |
|------|---------|
| `apps/umbrello/src/app.rs` | Added ~340 lines: ToolMode enum (65 lines), new fields (5), new methods (130), render_canvas additions (80), update() additions (60), tests (170) |

---

## Test Coverage (17 new tests)

| # | Test | Status |
|---|------|--------|
| T1 | `tool_mode_defaults_to_select` | ✅ |
| T2 | `tool_mode_select_label` | ✅ |
| T3 | `tool_mode_is_creation_tool` | ✅ |
| T4 | `generate_unique_name_first` | ✅ |
| T5 | `generate_unique_name_increments` | ✅ |
| T6 | `generate_unique_name_finds_gap` | ✅ |
| T7 | `create_element_for_tool_class` | ✅ |
| T8 | `create_element_for_tool_package` | ✅ |
| T9 | `place_element_creates_in_model` | ✅ |
| T10 | `place_element_adds_node_to_diagram` | ✅ |
| T11 | `place_element_dirty_flag` | ✅ |
| T12 | `tool_resets_after_placement` | ✅ |
| T13 | `place_element_undo_removes_both` | ✅ |
| T14 | `selection_persists_before_click` | ✅ |
| T15 | `new_element_visible_on_canvas` | ✅ |
| T16 | `tool_palette_buttons_exist` | ✅ |
| T17 | `element_color_for_new_type` | ✅ |

---

## Test Results

```
cargo fmt --all   → passes
cargo clippy --workspace --all-targets -- -D warnings   → passes
cargo test --workspace   → 233 tests passing (216 base + 17 new)
```

---

## Deviations from Design

None. Implementation follows the design document exactly.

---

## Architecture Compliance

- ✅ `uml-core` is untouched — no changes to any crate outside `apps/umbrello`
- ✅ All mutations via `execute_command()` which wraps `History::execute()` with dirty tracking
- ✅ No XMI, persistence, or code generation changes
- ✅ `ToolMode` is a plain enum with pattern matching, consistent with `ModelElement` dispatch pattern
- ✅ All existing 216 tests remain green
- ✅ 17 new tests added in the `#[cfg(test)] mod tests` block
