# Milestone 18 — Property Editor Panel: Implementation Complete

**Date:** 2026-06-26  
**Initial test count:** 233  
**Final test count:** 257 (+24)

---

## Summary

Milestone 18 is fully implemented across all four phases. The monolithic `app.rs` (~1652 lines) has been split into 9 modular source files, three new undo commands have been added to `uml-core`, element selection tracking with visual highlight has been implemented, and a full property editor panel renders on the right side of the application window.

---

## Commits Made

### 1. `refactor: split app.rs into modular source files`
- Created 7 new files under `apps/umbrello/src/`:
  - `tool_palette.rs` — ToolMode enum, `create_element_for_tool`, `place_element`, `render_tool_palette`
  - `canvas.rs` — `render_canvas`, `draw_partitioned_node`, `draw_edges`
  - `rendering.rs` — `element_color`, `visibility_symbol`, `type_display`, arrowhead drawing functions
  - `menu.rs` — `render_menu`, `menu_file_new/open/save/save_as`
  - `tree.rs` — `render_tree` (diagram list + element list)
  - `file_io.rs` — `prompt_save_if_dirty`
  - `tests.rs` — All `#[cfg(test)]` tests extracted from `app.rs`
- Updated `app.rs` to keep only the `UmbrelloApp` struct, constructor, `execute_command`, `generate_unique_name`, `update_title`, and the `eframe::App` impl (`update()` method).
- Updated `main.rs` with `mod` declarations for all new modules.
- Used `pub(crate)` visibility for cross-module types and methods.

### 2. `feat(uml-core): add ChangeVisibility, ChangeElementFlags, ChangeDocumentation commands`
- **ChangeVisibility**: Snapshots old visibility on construction, applies new on execute, restores old on undo.
- **ChangeElementFlags**: Atomically sets both `is_abstract` and `is_static` (both flags in a single command).
- **ChangeDocumentation**: Snapshots old documentation text, applies new on execute, restores old on undo.
- Added 9 unit tests (CMD-01 through CMD-09) covering execute, undo, and element-not-found error paths for each command.
- Re-exported via `undo/mod.rs` and `lib.rs` (already auto-exported by `pub use undo::commands`).

### 3. `feat(app): add element selection tracking and highlight`
- Added `selected_element_id: Option<UmlId>` and `name_edit_buffer: String` fields to `UmbrelloApp`.
- Initialized to `None` / `String::new()` in constructor.
- Node click handler in `render_canvas` sets `selected_element_id` and populates `name_edit_buffer` from the clicked element's name.
- Selection highlight: 2.5px blue (`Color32::from_rgb(0, 120, 215)`) border drawn on top of the normal border in `draw_partitioned_node`.
- Background click handler (in Select mode) clears selection on empty canvas click.
- Escape key: clears selection if one is active; otherwise resets tool to Select.
- Added 4 tests (APP-01 through APP-04).

### 4. `feat(app): add property editor panel with editable name, visibility, flags, documentation`
- Created `property_editor.rs` with `render_property_editor()` implementing:
  - "Nothing selected" placeholder with centered weak text.
  - Read-only Type (from `object_type().as_str()`) and ID (truncated to 20 chars).
  - Editable name field → `RenameElement` command on Enter or focus loss.
  - Visibility dropdown → `ChangeVisibility` command (4 options with symbol + name).
  - Abstract / Static checkboxes → `ChangeElementFlags` command.
  - Documentation multiline → `ChangeDocumentation` command on focus loss.
  - Read-only classifier details section (attribute/operation listing with visibility symbols and type names).
- Added `visibility_name()` helper to `rendering.rs`.
- Wired via `egui::SidePanel::right("property_panel")` in `app.rs` `update()`.
- Uses snapshot-based design to avoid borrow-checker conflicts (extracts data into owned values before closures).
- Added 11 tests (APP-05 through APP-15).

---

## Test Coverage Details

### New Command Tests (uml-core: CMD-01 through CMD-09)

| Test ID | Name | Status |
|---------|------|--------|
| CMD-01 | `change_visibility_execute` | ✅ |
| CMD-02 | `change_visibility_undo` | ✅ |
| CMD-03 | `change_visibility_new_element_not_found` | ✅ |
| CMD-04 | `change_flags_execute` | ✅ |
| CMD-05 | `change_flags_undo` | ✅ |
| CMD-06 | `change_flags_new_element_not_found` | ✅ |
| CMD-07 | `change_documentation_execute` | ✅ |
| CMD-08 | `change_documentation_undo` | ✅ |
| CMD-09 | `change_documentation_new_element_not_found` | ✅ |

### Selection Tests (APP-01 through APP-04)

| Test ID | Name | Status |
|---------|------|--------|
| APP-01 | `selected_element_id_defaults_to_none` | ✅ |
| APP-02 | `select_node_sets_selected_element_id` | ✅ |
| APP-03 | `deselect_on_background_click` | ✅ |
| APP-04 | `name_edit_buffer_populates_on_selection` | ✅ |

### Property Editor Tests (APP-05 through APP-15)

| Test ID | Name | Status |
|---------|------|--------|
| APP-05 | `rename_element_via_property_editor` | ✅ |
| APP-06 | `visibility_dropdown_changes_visibility` | ✅ |
| APP-07 | `visibility_change_undo_restores` | ✅ |
| APP-08 | `flag_toggle_sets_abstract_and_static` | ✅ |
| APP-09 | `flag_toggle_undo_restores_flags` | ✅ |
| APP-10 | `documentation_edit_persists` | ✅ |
| APP-11 | `documentation_change_undo_reverts` | ✅ |
| APP-12 | `classifier_details_displayed_for_class` | ✅ |
| APP-13 | `classifier_details_hidden_for_package` | ✅ |
| APP-14 | `property_editor_placeholder_when_none_selected` | ✅ |
| APP-15 | `dirty_flag_set_on_property_change` | ✅ |

---

## Files Modified

| File | Change |
|------|--------|
| `apps/umbrello/src/app.rs` | Reduced from ~1652 to ~220 lines; added selection fields, property panel wiring, Escape deselection |
| `apps/umbrello/src/main.rs` | Added module declarations for 7 new modules |
| `apps/umbrello/src/canvas.rs` | New — canvas rendering + selection highlight |
| `apps/umbrello/src/file_io.rs` | New — `prompt_save_if_dirty` |
| `apps/umbrello/src/menu.rs` | New — menu rendering + file operations |
| `apps/umbrello/src/property_editor.rs` | New — full property editor panel |
| `apps/umbrello/src/rendering.rs` | New — rendering helpers + `visibility_name` |
| `apps/umbrello/src/tests.rs` | New — all 46 unit tests |
| `apps/umbrello/src/tool_palette.rs` | New — ToolMode + creation methods |
| `apps/umbrello/src/tree.rs` | New — tree/panel rendering |
| `crates/uml-core/src/undo/commands.rs` | Added 3 new command types + 9 tests |

---

## Issues / Deviations from Design Document

1. **Borrow checker workaround in property_editor.rs**: The design doc's straightforward approach of holding a reference to `elem` (from `self.model.get()`) while calling `self.execute_command()` within closures ran into Rust's borrow checker. The implementation uses a snapshot pattern: all data needed for the panel is extracted into owned values before any closures are created. This is functionally equivalent and test-identical but uses an intermediate `ClassifierSnapshot` struct.

2. **Tests.rs lint allow**: The test module is `#[cfg(test)]` gated in `main.rs`. When `clippy --all-targets` runs, the binary target raises `dead_code` and `unused_imports` warnings for the test code. These are suppressed with `#![allow(unused_imports, dead_code)]` at the top of `tests.rs`.

3. **No changes to `uml-io` or `uml-codegen`**: As specified in the design document.

---

## Verification

```sh
cargo test --workspace            # 257 passed, 0 failed
cargo clippy --workspace --all-targets -- -D warnings  # 0 warnings
cargo fmt --all --check           # 0 diffs
```
