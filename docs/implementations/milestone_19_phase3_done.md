# Milestone 19 Phase 3 — Implementation Report

**Phase 3: Canvas Drag-to-Connect + Rubber-Band Preview**

## Summary

Implemented the canvas interaction for click-drag edge creation with real-time rubber-band preview, completing Milestone 19. All three phases now work together: `CreateEdge` command (Phase 1), tool palette with 6 edge tools + `place_edge()` method (Phase 2), and the canvas drag interaction (Phase 3).

## Files Modified

| File | Changes |
|------|---------|
| `apps/umbrello/src/app.rs` | Added `drag_source_node_id: Option<UmlId>` and `pointer_was_down: bool` fields to `UmbrelloApp` struct; initialized to `None`/`false` in `new()`; updated Escape handler to cancel edge drag |
| `apps/umbrello/src/tool_palette.rs` | Removed `#[allow(dead_code)]` from `is_edge_tool()`, `association_type()`, and `place_edge()` (now used by canvas.rs); added `self.drag_source_node_id = None` to all palette button click handlers |
| `apps/umbrello/src/canvas.rs` | Restructured interaction loop to branch by tool mode (Select/is_edge_tool/is_creation_tool); added rubber-band preview rendering after `draw_edges()` using semi-transparent gray with correct arrowheads; added edge drag detection via `Sense::drag()` on node rects in edge tool mode; added global button-release detection for target node; added continuous `request_repaint()` during edge drag; updated background deselect to only fire in Select mode |
| `apps/umbrello/src/tests.rs` | Added 7 new tests (APP-20 to APP-24, APP-26, APP-27) |

## New Types/Functions

**Fields on `UmbrelloApp`:**
- `drag_source_node_id: Option<UmlId>` — tracks the source node during an edge drag
- `pointer_was_down: bool` — mouse button state tracking (future-proof, currently unused)

## Test Coverage

### New Tests (APP-20 to APP-24, APP-26, APP-27)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| APP-20 | `place_edge_creates_relationship` | `place_edge()` creates a `Relationship` in the model between source and target |
| APP-21 | `place_edge_creates_view_edge` | The active diagram gains a `ViewEdge` after `place_edge()` |
| APP-22 | `place_edge_dirty_flag` | `is_dirty` becomes `true` after `place_edge()` |
| APP-23 | `place_edge_undo_removes_both` | Undo removes both the `Relationship` and the `ViewEdge` |
| APP-24 | `place_edge_undo_redo_restores` | After undo → redo, the edge is fully restored |
| APP-26 | `drag_source_node_id_defaults_none` | New `UmbrelloApp` has `drag_source_node_id: None` |
| APP-27 | `edge_tool_labels_nonempty` | All 6 edge tool labels are non-empty |

### Updated Test Locations

- `crates/uml-core/src/undo/commands.rs` — `CreateEdge` command tests (CMD-10 through CMD-15, Phase 1)
- `apps/umbrello/src/tests.rs` — APP-16 to APP-27 (Phase 2 + Phase 3)

### Final Test Count

| Test Suite | Count |
|------------|-------|
| `umbrello` app tests | 58 |
| `uml-core` unit tests | 149 |
| `uml-core` id_tests | 8 |
| `uml-core` serde_roundtrip | 6 |
| `uml-core` diagram_geometry | 2 |
| `uml-core` history | 4 |
| `uml-io` XMI tests | 46 |
| `uml-io` real corpus | 1 |
| `uml-io` doctests | 1 |
| **Total** | **275** |

## Architecture Decisions

- **`Sense::drag()` per node for edge tools**, not `click_and_drag()` — edge creation needs to detect release on *any* node, not just the initiating node.
- **Semi-transparent rubber-band** (`rgba(100, 100, 100, 120)`) — visually distinguishes in-progress edges from committed black edges.
- **Rubber-band drawn before nodes** — node bodies occlude the rubber-band ends, providing clean visual termination.
- **One-shot tool reset** — after successful edge creation, tool resets to Select (matches node creation pattern).
- **Escape cancels drag without resetting tool** — first Escape cancels drag, second Escape resets to Select.

## Verification

```sh
cargo test --workspace              # 275 tests pass
cargo clippy --workspace --all-targets -- -D warnings  # Zero warnings
cargo fmt --all --check             # No formatting diffs
cargo build -p umbrello            # Compiles without errors
```

## Commit History

```
15c71f6 feat(app): add click-drag edge creation with rubber-band preview on canvas
```
