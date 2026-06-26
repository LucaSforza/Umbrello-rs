# Milestone 17 — Tool Palette & Interactive Element Creation — Review

**Review Date:** 2026-06-26  
**Commit:** `44ea833`  
**Files Changed:** `apps/umbrello/src/app.rs` (only), `docs/implementations/milestone_17_done.md` (completion report)  
**Test Count:** 233 all passing (216 base + 17 new M17 tests)

---

## Verdict: **APPROVED** ✅

The implementation satisfies all sub-tasks A–K from the design document (Section 7) and meets all 15 review conditions. Zero issues found.

---

## Detailed Review

### Sub-task Verification (Section 7)

| Sub-task | Status | Notes |
|----------|--------|-------|
| **A** — `ToolMode` enum + `current_tool` field | ✅ | 6 variants, `Debug + Clone + Copy + PartialEq + Eq`. Field initialized to `Select` in `new()` |
| **B** — `generate_unique_name` | ✅ | Scans existing names, finds `{base}_{N}` matches, gap-fills from 1 |
| **C** — `create_element_for_tool` factory | ✅ | Correct variant for each tool; Interface sets `is_abstract = true` |
| **D** — `place_element` | ✅ | Uses `execute_command()` (not raw `history.execute()`); 2 commands: CreateElement + AddNodeToDiagram |
| **E** — `render_tool_palette` | ✅ | `SelectableLabel` per variant, active highlight, status message update |
| **F** — Integrate into left panel | ✅ | Called before `render_tree()` in `SidePanel::left` closure |
| **G** — Background click handler | ✅ | Activates only for `is_creation_tool()`, `Sense::click()` (not drag), auto-resets to Select |
| **H** — Ghost preview rectangle | ✅ | Semi-transparent blue 160×60 rect centered on cursor |
| **I** — Crosshair cursor | ✅ | `set_cursor_icon(Crosshair)` at top of `render_canvas` |
| **J** — Keyboard shortcuts | ✅ | Uses `consume_key` with `Modifiers::NONE` (better than design's `key_pressed()`), guarded by `!ctx.wants_keyboard_input()` |
| **K** — Tests | ✅ | 17 tests (T1–T17) present and meaningful |

### Architecture Compliance (Section 8 of Design)

| Rule | Status | Evidence |
|------|--------|----------|
| `uml-core` untouched | ✅ | `git diff 44ea833^..44ea833 -- crates/` shows zero output |
| `uml-io` untouched | ✅ | Same as above |
| All mutations via Commands | ✅ | `place_element` calls `execute_command()` which wraps `History::execute()` with dirty tracking |
| No inheritance emulation | ✅ | `ToolMode` is a plain enum with pattern matching |
| egui immediate mode | ✅ | All palette state in `UmbrelloApp` fields; no retained-mode patterns |
| `cargo test --workspace` passes | ✅ | 233/233 tests pass |
| 17 new tests | ✅ | T1–T17 all present |

### Deviation from Design

The implementation uses `ctx.input_mut(|i| i.consume_key(...))` for keyboard shortcuts instead of the design's `ctx.input(|i| i.key_pressed(...))`. This is a **positive deviation** — `consume_key` prevents key repeat triggers and modifier conflicts, which is more robust than the design's original approach. All other details match exactly.

### Test Quality Assessment (T1–T17)

| Test | Verifies | Quality |
|------|----------|---------|
| T1 | `current_tool` defaults to `Select` | ✅ |
| T2 | All 6 labels are non-empty | ✅ |
| T3 | `is_creation_tool` returns correct values | ✅ |
| T4 | First name is `"{base}_1"` | ✅ |
| T5 | Increments when `_1` exists | ✅ |
| T6 | Gap-filling (`_1` + `_3` → `_2`) | ✅ |
| T7 | Class creation via factory | ✅ |
| T8 | Package creation via factory | ✅ |
| T9 | Model contains new element after `place_element` | ✅ |
| T10 | Diagram has ViewNode at correct position | ✅ |
| T11 | `is_dirty` set after placement | ✅ |
| T12 | Tool resets to Select (simulates handler flow) | ✅ (contract test — reset happens in caller, not `place_element`) |
| T13 | Undo removes ViewNode then element (two-step) | ✅ |
| T14 | Select is not a creation tool | ✅ |
| T15 | Created element visible in model iteration | ✅ |
| T16 | 6 tools with unique labels | ✅ |
| T17 | `element_color` returns correct color per type | ✅ |

### Clippy & Formatting

- **`cargo clippy --workspace --all-targets -- -D warnings`** — passes cleanly (no warnings or errors)
- **`cargo fmt --all --check`** — passes (the Warning lines are pre-existing nightly-only rustfmt config options, not formatting issues)
- Dead-code annotations on `name_counters`, `tooltip()`, and `loaded_from_xmi` are explicitly noted in the design as reserved for future use and are acceptable.

---

## Summary

All sub-tasks A–K are correctly implemented. The code is architecturally clean with zero changes to `uml-core` or `uml-io`, all mutations go through `execute_command()`, and 17 well-structured tests cover the new functionality comprehensively. No regressions. The implementation can proceed to subsequent milestones (M18 Property Editor, M19 Edge Creation).
