# Milestone 19 Phase 1 Review — `CreateEdge` Command

**Status:** APPROVED  
**Reviewer:** Umbrello-RS Reviewer  
**Date:** 2026-06-26  
**Commit reviewed:** Current working tree (Phase 1 scope only)

---

## Scope

This review covers **Phase 1** of Milestone 19 as defined in `docs/designs/milestone_19.md`:

1. The `CreateEdge` struct and `impl Command` in `crates/uml-core/src/undo/commands.rs`
2. Re-exports from `crates/uml-core/src/undo/mod.rs` and `crates/uml-core/src/lib.rs`
3. The 6 unit tests (CMD-10 through CMD-15) in `commands.rs`

---

## Review Checks

### 1. Design Compliance (Section 3.3)

| Aspect | Result | Notes |
|--------|--------|-------|
| Struct fields | ✅ | All 7 fields match design exactly |
| Constructor `new(diagram_id, source_node_id, target_node_id, kind)` | ✅ | Signature matches; all 6 AssociationType variants handled |
| Constructor body (Relationship creation, id capture, EdgeId generation, description) | ✅ | Matches design exactly |
| `execute()` — insert Relationship + add ViewEdge (Direct routing) | ✅ | Error for already-executed (`InvalidOperation`); error for bad diagram_id |
| `undo()` — remove ViewEdge first, then Relationship | ✅ | Diagram removal uses `if let Some` (as designed); relationship removal errors on not found |
| `description()` | ✅ | Returns stored `kind_name` string |

### 2. Correctness

| Behavior | Result | Notes |
|----------|--------|-------|
| `execute()` inserts Relationship into model | ✅ | Uses `model.insert()` |
| `execute()` adds ViewEdge to diagram | ✅ | Uses `LineRouting::Direct` as designed |
| Double-execute returns error | ✅ | `take()` on already-`None` returns `InvalidOperation` |
| Bad diagram returns error | ✅ | `get_diagram_mut()` returns `InvalidOperation` |
| `undo()` removes ViewEdge from diagram | ✅ | Uses `d.remove_edge()` |
| `undo()` removes Relationship from model | ✅ | Uses `model.remove()`, stores back in `Option` |
| `undo()` errors on missing relationship | ✅ | Returns `CommandError::ElementNotFound` |
| Snapshot pattern (execute → undo → re-execute) | ✅ | Tested in CMD-15; `Option` pattern works correctly |

### 3. Pattern Consistency

| Aspect | Result | Notes |
|--------|--------|-------|
| Uses `Option<ModelElement>` snapshot | ✅ | Same pattern as `CreateElement` |
| Uses `CommandError` properly | ✅ | `InvalidOperation` for precondition failures, `ElementNotFound` for undo failures |
| Implements `Command` trait | ✅ | execute/undo/description all present |
| `#[must_use]` on constructor | ✅ | Consistent with project convention |
| `#[derive(Debug)]` | ✅ | Consistent with all other commands |
| No `unsafe` | ✅ | `forbid(unsafe_code)` in crate |
| No `unwrap()`/`expect()` in production | ✅ | All `unwrap()` calls are in `#[cfg(test)]` blocks only |

### 4. Re-exports

| Export point | Status | Notes |
|-------------|--------|-------|
| `undo/mod.rs` line 9 | ✅ | `pub use commands::CreateEdge;` |
| `lib.rs` | ✅ | `pub use undo::commands;` makes it accessible as `uml_core::commands::CreateEdge` |

### 5. Test Quality (CMD-10 through CMD-15)

| Test ID | Name | Status | Verifies |
|---------|------|--------|----------|
| CMD-10 | `create_edge_execute_generalization` | ✅ | Execute inserts Relationship + ViewEdge (Direct routing); checks fields individually |
| CMD-11 | `create_edge_undo_generalization` | ✅ | Undo removes BOTH model element and diagram edge |
| CMD-12 | `create_edge_execute_all_kinds` | ✅ | All 6 AssociationType variants produce correct kind + edge |
| CMD-13 | `create_edge_diagram_not_found` | ✅ | Error on bad diagram_id |
| CMD-14 | `create_edge_description` | ✅ | Description contains "Generalization" and "Dependency" |
| CMD-15 | `create_edge_undo_then_redo` | ✅ | execute → undo → execute restores both Relationship + ViewEdge |

All 6 tests pass. They follow existing test patterns (use `setup_model_with_two_nodes()` helper, direct command manipulation, exhaustive assertions).

### 6. Regressions

| Check | Result |
|-------|--------|
| `cargo test --workspace` | ✅ All 262 tests pass (256 previous + 6 new) |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ Zero warnings |
| All existing test suites still pass | ✅ |

---

## Supplementary Checks

### Imports

The file `commands.rs` imports the necessary types at lines 3-8:

```rust
use crate::elements::{ModelElement, Relationship};
use crate::id::UmlId;
use crate::repository::UmlModel;
use crate::types::{AssociationType, Visibility};
use super::{Command, CommandError};
```

These cover `Relationship`, `UmlModel`, `AssociationType`, `Command`, and `CommandError`. The types `DiagramId`, `EdgeId`, `LineRouting`, `ViewEdge` are used directly via `crate::diagram::*` without explicit `use` imports (they are `use`d at the call site) — this works because they're fully qualified in the code. ✅

### Test Setup Function

The `setup_model_with_two_nodes()` helper (lines 981-1000):
- Creates a model with a diagram, two Class elements, and two ViewNodes ✅
- Returns `(UmlModel, DiagramId, UmlId, UmlId)` for convenient use in tests ✅
- Each test gets a fresh model, so no test pollution ✅

---

## Verdict

**APPROVED.** The `CreateEdge` command implementation is fully compliant with the design specification in `docs/designs/milestone_19.md`. All 6 design-required tests exist and pass. No regressions. No clippy warnings. No production `unwrap()` calls.

**Note:** This review covers Phase 1 only. Phases 2 (Tool Palette Extension) and 3 (Canvas Drag-to-Connect + Rubber-Band Preview) will require separate reviews when implemented.
