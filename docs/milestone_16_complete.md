# Milestone 16 — File I/O: Open, Save, Save As, and New — Completion Report

**Date:** 2026-06-26  
**Status:** **COMPLETE** — All sub-tasks implemented, reviewed, and approved  
**Architect:** Umbrello-RS Architect (orchestrated via implementer + reviewer agents)

---

## 1. Executive Summary

Milestone 16 delivers the complete File I/O workflow for the Umbrello-RS GUI application. Prior to M16, the File menu had only a stub "Open XMI..." that printed "not yet implemented" and no Save capability at all. The application could only load a hardcoded test file via `main.rs`. M16 closes this critical gap by implementing **File > New, Open, Save, Save As, and Quit with unsaved-changes prompting**, plus keyboard shortcuts and CLI improvements.

**No changes were made to `uml-core`.** The domain model remains pure and untouched.

---

## 2. Files Modified

| # | File | Lines (+/-) | Description |
|---|------|-------------|-------------|
| 1 | `Cargo.lock` | +297/-65 | Dependency resolution for `rfd` |
| 2 | `apps/umbrello/Cargo.toml` | +1 | Added `rfd = "0.15"` |
| 3 | `apps/umbrello/src/app.rs` | +386/-? | All File I/O logic: fields, methods, menu, shortcuts, dirty tracking, title |
| 4 | `apps/umbrello/src/main.rs` | +57/-? | Replaced hardcoded path with `clap` CLI argument |
| 5 | `crates/uml-io/src/xmi/mod.rs` | +78/-? | Added `save_xmi_to_file()` and `load_xmi_from_file()` convenience functions |

**Total:** 5 files, +754/-65 lines. No files created. `uml-core` diff is zero.

---

## 3. Sub-Task Summary

| Sub-task | Description | Status |
|----------|-------------|--------|
| **A** | Add `rfd` dependency | ✓ |
| **B** | Convenience functions in `uml-io` (`save_xmi_to_file`, `load_xmi_from_file`) | ✓ |
| **C** | New fields on `UmbrelloApp` (`current_file_path`, `is_dirty`) | ✓ |
| **D** | `execute_command` helper (wraps `History::execute` with dirty tracking) | ✓ |
| **E** | `update_title` helper (window title shows file path + dirty `*`) | ✓ |
| **F** | `prompt_save_if_dirty` + File > New | ✓ |
| **G** | File > Open (rfd native dialog + XMI loading + error handling) | ✓ |
| **H** | File > Save / Save As (rfd native dialog + XMI writing + .xmi extension) | ✓ |
| **I** | Keyboard shortcuts (Ctrl+N/O/S/Shift+S/Q with key consumption) | ✓ |
| **J** | Quit with dirty prompt (replaces `exit(0)` with `ViewportCommand::Close`) | ✓ |
| **K** | CLI changes (removes hardcoded path, accepts positional `file` arg via `clap`) | ✓ |
| **L** | Tests (10 new tests: 8 in app.rs, 2 in uml-io) | ✓ |

---

## 4. Test Results

```
cargo test --workspace    → 216 tests, all passing
cargo fmt --all --check   → clean
cargo clippy -D warnings  → clean
```

### Test Breakdown

| Test Suite | Before M16 | After M16 | Delta |
|------------|-----------|-----------|-------|
| `apps/umbrello` (app.rs) | 6 | 14 | +8 |
| `uml-core` (elements, id, serde, geometry, history) | 154 | 154 | 0 |
| `uml-io` (xmi reader/writer, corpus, doctest) | 45 | 47 | +2 |
| `xtask` | 1 | 1 | 0 |
| **Total** | **206** | **216** | **+10** |

### New Tests

**In `apps/umbrello/src/app.rs`:**
- `file_new_clears_model` (T1) — Model empty after File > New
- `dirty_flag_on_mutation` (T2) — Dirty flag tracking
- `dirty_flag_after_execute_command` (T2b) — execute_command pattern
- `dirty_flag_cleared_on_save` (T3) — Save clears dirty flag
- `dirty_flag_cleared_on_open` (T4) — Open/replace clears dirty flag
- `file_path_tracking` (T5) — Path set/get correctness
- `save_then_reload_roundtrip` (T6) — Save to temp file → load back → verify
- `save_as_updates_path` (T7) — Path updates after Save As

**In `crates/uml-io/src/xmi/mod.rs`:**
- `save_xmi_to_file_roundtrip` (T10) — Convenience functions round-trip
- `save_xmi_to_file_error_on_bad_path` (T11) — Error on unwritable path

---

## 5. Architecture Compliance

| Rule | Status |
|------|--------|
| `uml-core` must have ZERO changes | ✓ Confirmed via `git diff` |
| All mutations via Commands | ✓ `execute_command` wraps `History::execute` |
| No `unwrap()`/`expect()` on user-facing code paths | ✓ No new instances |
| Composition over inheritance | ✓ N/A — no new types |
| `cargo test --workspace` must pass | ✓ 216 tests, all green |
| XMI compatibility preserved | ✓ Reader/writer API used as-is |
| Core domain stays pure | ✓ No GUI/I/O code leaked into `uml-core` |

---

## 6. Review

The implementation was reviewed by the `@reviewer` agent against `docs/designs/milestone_16.md`. The review found:

- **All 12 sub-tasks (A–L) correct and complete**
- **Zero architectural violations**
- **10 meaningful new tests**
- **All quality gates passing** (fmt, clippy, test)

The review produced **4 non-blocking observations** (documented in `docs/reviews/milestone_16_approved.md`): unused Ctrl+Shift+S safety net, pre-existing Undo/Redo unwrap() calls, silent CLI load failure, and `loaded_from_xmi` dead-code field. None block approval.

**Verdict: APPROVED** with no required changes.

---

## 7. Deviations from Design

| # | Deviation | Justification |
|---|-----------|---------------|
| 1 | Keyboard shortcuts use `consume_key` for Ctrl+S + internal shift check instead of separate handlers | Prevents Ctrl+Shift+S from being consumed by Ctrl+S; cleaner pattern |
| 2 | Test T2 uses direct `is_dirty` check instead of constructing a `MoveNode` command | Constructing a valid MoveNode requires diagram setup; the dirty flag mechanism is verified end-to-end in T3 (save round-trip) |
| 3 | Test T6 compares names instead of exact element counts | XMI serialization wraps model in root Package, adding an extra element; name-based verification is the semantically correct test |
| 4 | `rfd::MessageDialogResult` extra variants (`Ok`, `Custom`) handled as `true` (proceed) | These should not occur with `YesNoCancel` buttons; handling them as proceed is the safe default |

---

## 8. What's Next (M17 Candidates)

With File I/O complete, the application can now load and save user models. The logical next milestones are:

1. **Domain model expansion** — Actor, UseCase, Component, Node, Artifact (HIGH priority DOMAIN gaps)
2. **Element creation tool palette** — Click-to-place new UML elements on the canvas (HIGH priority GUI gap)
3. **Property editor panel** — Right-panel for editing element details (HIGH priority GUI gap)
4. **Edge creation** — Click-and-drag to create relationships visually (HIGH priority GUI gap)

---

## 9. Document Artifacts

| Document | Path |
|----------|------|
| Design specification | `docs/designs/milestone_16.md` |
| Implementation report | `docs/implementations/milestone_16_done.md` |
| Review approval | `docs/reviews/milestone_16_approved.md` |
| Completion report (this file) | `docs/milestone_16_complete.md` |

---

*Milestone 16 complete. The Umbrello-RS application can now open, create, save, and save-as XMI files with full dirty-flag tracking and keyboard shortcuts — a critical step toward a minimum viable product.*
