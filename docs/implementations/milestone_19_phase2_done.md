# Milestone 19 Phase 2 — Implementation Report

**Task:** Tool Palette Extension with 6 Edge-Creation Tool Variants  
**Commit:** `4e5856e`  
**Date:** 2026-06-26

---

## Summary

Implemented Phase 2 of M19 (edge-creation tool palette) as specified in `docs/designs/milestone_19.md`. Phase 1 (`CreateEdge` command + tests) was already implemented and present before starting. This phase extends the `ToolMode` enum with six edge variants, adds the `place_edge()` method, updates `render_tool_palette()` with an "Edges" section, adds keyboard shortcuts, and writes 5 app-level tests.

No changes were made to `canvas.rs` or `uml-core` — those belong to Phase 3.

---

## Files Modified

### `apps/umbrello/src/tool_palette.rs`
- Added 6 new `ToolMode` variants: `CreateGeneralization`, `CreateRealization`, `CreateAssociation`, `CreateAggregation`, `CreateComposition`, `CreateDependency`
- Updated `label()` to return appropriate strings (e.g., `"△ Generalization"`, `"— Association"`)
- Updated `tooltip()` with descriptions for each edge tool
- Narrowed `is_creation_tool()` to match only the 5 node-creation tools (edge tools return `false`)
- Added `is_edge_tool()` method (returns `true` for edge variants)
- Added `association_type()` method (maps edge tools to `AssociationType`, returns `None` for non-edge)
- Updated `create_element_for_tool()` with unreachable arms for edge variants
- Updated `render_tool_palette()`: node tools loop unchanged, then `ui.separator()`, "Edges" label, second loop for 6 edge tools. Clicking an edge tool sets `current_tool`, clears `preview_position`, updates status message
- Added `place_edge()` method: takes source/target `UmlId`, gets `AssociationType` from current tool, gets diagram ID, executes `CreateEdge` command

### `apps/umbrello/src/app.rs`
- Added keyboard shortcuts in `update()` tool shortcuts section:
  - `G` → `CreateGeneralization`
  - `R` → `CreateRealization`
  - `A` → `CreateAssociation`
  - `N` → `CreateDependency`
- Each shortcut clears `preview_position`
- Esc handler updated to also clear `preview_position` on reset to Select
- Added `clear preview_position` to existing tool shortcuts for consistency
- All shortcuts gated by `!ctx.wants_keyboard_input()`

### `apps/umbrello/src/tests.rs`
- Added imports: `AssociationType`, `Size`, `UmlId`
- Added `make_app_with_two_nodes()` helper function
- **APP-16** `edge_tool_is_edge_tool`: verifies all 6 edge tools return `true` for `is_edge_tool()`
- **APP-17** `edge_tool_not_creation_tool`: verifies all 6 edge tools return `false` for `is_creation_tool()`
- **APP-18** `edge_tool_association_type`: verifies correct `Some(AssociationType::*)` for each edge tool
- **APP-19** `select_not_edge_tool`: verifies `ToolMode::Select.is_edge_tool()` returns `false` and `association_type()` returns `None`
- **APP-25** `place_edge_no_diagram_errors`: creates app with no diagrams, calls `place_edge()`, verifies `Err(...)` with "No active diagram" message

---

## Test Coverage

| Test Suite | Count | Status |
|------------|-------|--------|
| `umbrello` unit tests | 51 | All pass (46 existing + 5 new) |
| `uml-core` unit tests | 149 | All pass (incl. CMD-10 through CMD-15 from Phase 1) |
| All workspace tests | 268 | All pass |

**New test count:** 268 (was 257 in M18)

---

## Checklist Results

- `cargo fmt --all --check` — OK (warnings are nightly-only config options, not errors)
- `cargo clippy --workspace --all-targets -- -D warnings` — Pass
- `cargo test --workspace` — 268 tests pass, 0 failures

---

## Phase 2 Scope Notes

- **Phase 1** (CreateEdge command) was already implemented: `CreateEdge` struct + 6 unit tests exist in `crates/uml-core/src/undo/commands.rs`
- **Phase 2** (this commit): Tool palette extension + keyboard shortcuts + `place_edge()` method + 5 app tests
- **Phase 3** (canvas drag-to-connect + rubber-band preview): NOT implemented — deferred to separate commit
- `is_edge_tool()`, `association_type()`, `place_edge()` are marked `#[allow(dead_code)]` since they'll be used in Phase 3
