# Milestone 20, Phase 3 — Implementation Complete

**Date:** 2026-06-26  
**Phase:** 3 of 3 — GUI: Rendering, Tool Palette, Keyboard Shortcuts  
**Commit:** `0429f21` — `feat(app): add Actor and UseCase rendering, tool palette, and keyboard shortcuts`

---

## What Was Implemented

### 1. `apps/umbrello/src/rendering.rs` — element_color() extension

Added two new match arms to `element_color()`:
- `ModelElement::Actor(_)` → `egui::Color32::from_rgb(255, 200, 170)` (light orange/salmon)
- `ModelElement::UseCase(_)` → `egui::Color32::from_rgb(255, 180, 180)` (light coral/pink)

### 2. `apps/umbrello/src/canvas.rs` — draw_partitioned_node() extension

Added two new match arms:
- **Actor arm:** Draws a stick-figure icon (head circle, vertical body line, horizontal arms at shoulder height, two diagonal legs) with the actor name centered below the figure.
- **UseCase arm:** Draws an ellipse (approximated via `rect_stroke` with large corner radius) with the use case name centered inside.

Both arms are placed between the Package arm and the wildcard `_` fallback.

### 3. `apps/umbrello/src/tool_palette.rs` — ToolMode extension

- Added `CreateActor` and `CreateUseCase` variants to `ToolMode` enum.
- Updated `label()`: "🧑 Actor", "⬭ UseCase"
- Updated `tooltip()`: "Create an Actor (T)", "Create a UseCase (U)"
- Updated `is_creation_tool()`: includes `CreateActor | CreateUseCase`
- Updated `create_element_for_tool()`: Actor and UseCase cases with `generate_unique_name`
- Updated `render_tool_palette()`: Actor and UseCase buttons in the node-creation tools section

### 4. `apps/umbrello/src/app.rs` — Keyboard shortcuts

Added two keyboard shortcuts in the `!ctx.wants_keyboard_input()` block:
- `T` → `CreateActor` tool
- `U` → `CreateUseCase` tool

Both reset `preview_position` and `drag_source_node_id` on activation.

### 5. `apps/umbrello/src/tests.rs` — 13 new tests (APP-28 through APP-40)

| Test ID | Name | Status |
|---------|------|--------|
| APP-28 | `tool_actor_is_creation` | ✅ |
| APP-29 | `tool_usecase_is_creation` | ✅ |
| APP-30 | `tool_actor_not_edge` | ✅ |
| APP-31 | `tool_usecase_not_edge` | ✅ |
| APP-32 | `create_element_for_actor` | ✅ |
| APP-33 | `create_element_for_usecase` | ✅ |
| APP-34 | `place_actor_dirty_flag` | ✅ |
| APP-35 | `place_usecase_dirty_flag` | ✅ |
| APP-36 | `actor_unique_naming` | ✅ |
| APP-37 | `usecase_unique_naming` | ✅ |
| APP-38 | `actor_undo_redo` | ✅ |
| APP-39 | `actor_color` | ✅ |
| APP-40 | `usecase_color` | ✅ |

---

## Test Coverage

| Test Suite | Count |
|-----------|-------|
| `apps/umbrello` unit tests | 71 (+13 from M19 baseline of 58) |
| `cargo test --workspace` total | **311 tests**, all passing |

---

## Verification

```sh
cargo fmt --all --check          # ✅ No diffs
cargo clippy --workspace --all-targets -- -D warnings  # ✅ Zero warnings
cargo test --workspace           # ✅ 311 tests, all passing
```

---

## Files Modified

| File | Changes |
|------|---------|
| `apps/umbrello/src/rendering.rs` | +3 lines (2 element_color cases) |
| `apps/umbrello/src/canvas.rs` | +63 lines (Actor stick-figure + UseCase ellipse rendering) |
| `apps/umbrello/src/tool_palette.rs` | +25 lines (2 ToolMode variants, label/tooltip, creation_tool, create_element, palette buttons) |
| `apps/umbrello/src/app.rs` | +12 lines (T and U keyboard shortcuts) |
| `apps/umbrello/src/tests.rs` | +141 lines (13 new tests) |

**Total:** 5 files, +240 lines, -4 lines

---

## Notes

- Phases 1 and 2 were completed prior to this work. The `uml-core` types (Actor, UseCase) and `uml-io` XMI reader/writer were already in place.
- No changes were needed to `uml-core`, `uml-io`, or `uml-codegen`.
- The existing undo/redo infrastructure (`CreateElement` + `AddNodeToDiagram` commands) works generically for Actor and UseCase elements.
- The property editor works automatically through the `NamedElement` trait.
- Edge creation (Actor ↔ UseCase associations) works via the existing `CreateEdge` command.
