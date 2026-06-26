# Milestone 16 — File I/O: Open, Save, Save As, and New — Review

**Date:** 2026-06-26  
**Status:** **APPROVED** ✓  
**Reviewer:** Umbrello-RS Reviewer (automated)

---

## Review Dimensions

| Dimension | Assessment |
|-----------|------------|
| **Correctness** | Implementation matches the design spec for all 12 sub-tasks (A–L). All edge cases handled (no path → Save As, dirty prompt on New/Open/Quit, save-as extension normalization). |
| **Architecture** | `uml-core` is untouched. Zero modifications to domain model, repository, types, or diagram module. All mutations go through Commands. No GUI code leaked into `uml-io` — only pure convenience functions. |
| **Rust Idioms** | `Option<PathBuf>` for optional paths, `Result` for I/O operations, `match` for dialog results. No trait object abuse. |
| **Performance** | No O(n²) loops. `update_title()` sends `ViewportCommand` every frame (standard egui pattern, negligible overhead). |
| **Safety** | No `unsafe` anywhere in `apps/umbrello/src/`. No new `unwrap()`/`expect()` in production code paths (pre-existing undo/redo `unwrap()` calls are guard-checked). |
| **Test Coverage** | 10 new tests (8 in `app.rs`: T1–T7 + T2b; 2 in `uml-io/xmi/mod.rs`: T10–T11), all passing. Total test count: 216 (up from 206 baseline). |

---

## Verification Results

```
✓ cargo test --workspace   — 216 tests, all pass
✓ cargo fmt --all --check  — clean (nightly-feature warnings only, pre-existing)
✓ cargo clippy --workspace --all-targets -- -D warnings — clean
✓ uml-core diff            — zero changes (confirmed via git diff)
```

---

## Sub-Task Confirmation

| Sub-task | Design Section | Status | Notes |
|----------|---------------|--------|-------|
| **A** — `rfd` dependency | §2, §3.1 | ✓ | `rfd = "0.15"` added to `apps/umbrello/Cargo.toml` |
| **B** — Convenience functions | §3.3 | ✓ | `save_xmi_to_file()` / `load_xmi_from_file()` in `uml-io/src/xmi/mod.rs` |
| **C** — New fields + setters | §3.1, §6.3 | ✓ | `current_file_path`, `is_dirty`, `set_current_file_path()` |
| **D** — `execute_command` helper | §7.1 | ✓ | Wraps `History::execute` with `is_dirty = true` on success |
| **E** — `update_title` helper | §5.4 | ✓ | Formats "Umbrello-RS — {path} *" pattern |
| **F** — File > New | §5.1, §5.2 | ✓ | Prompts if dirty, resets model/history/path/diagram |
| **G** — File > Open | §5.1 | ✓ | rfd file dialog, loads XMI, shows error dialog on failure |
| **H** — File > Save / Save As | §5.1 | ✓ | Delegates to Save As when no path; ensures .xmi extension |
| **I** — Keyboard shortcuts | §5.1 | ✓ | Ctrl+N/O/S/Shift+S/Q with `consume_key` pattern |
| **J** — Quit with dirty prompt | §5.2 | ✓ | Both menu button and Ctrl+Q call `prompt_save_if_dirty()` |
| **K** — CLI changes | §6 | ✓ | `clap::Parser` with optional `file` arg, hardcoded path removed |
| **L** — Tests | §8 | ✓ | 10 new tests: T1–T7 in `app.rs`, T10–T11 in `xmi/mod.rs` |

---

## Observations (Non-Blocking)

1. **Ctrl+Shift+S safety net** (`app.rs` lines 790–794): The separate `consume_key(CTRL|SHIFT, S)` handler is dead code in practice because the preceding `consume_key(CTRL, S)` handler already captures Ctrl+Shift+S and checks `modifiers.shift` internally. The duplicate is harmless but unnecessary. Consider removing in a future cleanup pass.

2. **Undo/Redo `unwrap()`** (`app.rs` lines 254, 266, 277, 285): These pre-date M16 and are guard-checked by `can_undo()`/`can_redo()`. Not introduced by this milestone.

3. **Silent CLI load failure**: If the CLI file argument points to a nonexistent or invalid XMI file, the app starts with an empty model and no error message. This matches the design spec (§6.2), but consider adding a status message in a future improvement.

4. **`loaded_from_xmi` field**: Still present with `#[allow(dead_code)]`. The design spec does not use it. Set to `true` in `menu_file_open()` (line 135). Consider removing in a future cleanup.

---

## Architecture Compliance

| Rule | Status |
|------|--------|
| `uml-core` must have ZERO changes | ✓ Confirmed via `git diff` |
| All mutations via Commands | ✓ `execute_command` helper wraps `History::execute` |
| No `unwrap()`/`expect()` on user-facing code paths | ✓ No new instances introduced |
| Traits over inheritance | ✓ Not applicable — no new types introduced |
| `cargo test --workspace` must pass | ✓ 216 tests, all green |

---

## Conclusion

The M16 implementation is **complete, correct, and well-tested**. All sub-tasks (A–L) match the design specification. Architecture boundaries are respected. The four non-blocking observations above are minor quality notes that do not affect functionality.

**Verdict: APPROVED**
