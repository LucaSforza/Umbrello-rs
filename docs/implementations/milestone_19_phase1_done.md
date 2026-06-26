# Milestone 19 Phase 1 — Implementation Report

## Task
Implement Phase 1 of Milestone 19: Add the `CreateEdge` undoable command in `uml-core`.

## Files Modified

| File | Change |
|------|--------|
| `crates/uml-core/src/undo/commands.rs` | Added `CreateEdge` struct + `impl Command for CreateEdge` (~90 lines), 6 new tests (~180 lines). Updated imports: `Relationship`, `AssociationType`, `EdgeId`, `LineRouting`, `ViewEdge`. |
| `crates/uml-core/src/undo/mod.rs` | Added re-export `pub use commands::CreateEdge;` |
| `docs/designs/milestone_19.md` | Already existed (read-only design reference) |

## Types/Functions Added

- **`CreateEdge`** (struct) — fields: `diagram_id`, `relationship_id`, `edge_id`, `source_node_id`, `target_node_id`, `relationship_element`, `description`
- **`CreateEdge::new(diagram_id, source_node_id, target_node_id, kind: AssociationType)`** — constructs a `Relationship` via the appropriate constructor (e.g., `Relationship::new_generalization`), generates `UmlId` and `EdgeId`, wraps in `ModelElement::Relationship`
- **`impl Command for CreateEdge`** — `execute()` inserts the relationship into `UmlModel` and adds a `ViewEdge` with `LineRouting::Direct` to the diagram; `undo()` removes both; `description()` returns the kind name
- **`setup_model_with_two_nodes()`** — test helper function

## Tests Added (6)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| CMD-10 | `create_edge_execute_generalization` | `CreateEdge::execute` inserts a Generalization Relationship + ViewEdge (Direct routing) into model + diagram |
| CMD-11 | `create_edge_undo_generalization` | `CreateEdge::undo` removes both the Relationship from model and ViewEdge from diagram |
| CMD-12 | `create_edge_execute_all_kinds` | All 6 `AssociationType` variants produce correct `Relationship.kind` and renderable edges |
| CMD-13 | `create_edge_diagram_not_found` | `execute()` returns `Err(InvalidOperation)` when diagram_id doesn't exist |
| CMD-14 | `create_edge_description` | `description()` returns a string containing the relationship kind name |
| CMD-15 | `create_edge_undo_then_redo` | After execute → undo → execute, the edge is fully restored (both Relationship + ViewEdge) |

## Test Count

| Suite | Before | After | Delta |
|-------|--------|-------|-------|
| `uml-core` unit tests | 143 | 149 | +6 |
| **Workspace total** | **257** | **263** | **+6** |

## Verification

```
❯ cargo test -p uml-core -- create_edge    → 6 passed
❯ cargo test -p uml-core                   → 149 passed (all)
❯ cargo test --workspace                   → 263 passed (all)
❯ cargo clippy -p uml-core -- -D warnings  → zero warnings
❯ cargo clippy --workspace --all-targets -- -D warnings → zero warnings
❯ cargo fmt --all --check                  → no formatting diffs
```

## Notes

- Only Phase 1 implemented per specification. No changes to `tool_palette.rs`, `canvas.rs`, `app.rs`, `uml-io`, or `uml-codegen`.
- `CreateEdge` follows the snapshot pattern (store `relationship_element: Option<ModelElement>`, `take()` on execute, restore on undo) consistent with the existing `CreateElement` command.
- The command atomically creates both a `Relationship` model element and a `ViewEdge` in the diagram, making edge creation a single undoable step.
