# Milestone 20, Phase 2 — Actor & UseCase XMI Reader/Writer

**Commit:** `e3a5a95`
**Date:** 2026-06-26
**Status:** Complete

## Summary

Implemented XMI reader and writer support for `Actor` and `UseCase` element types in the `uml-io` crate. These are the simplest element pattern — bare `ElementBase` with no `ClassifierData` — and share a common helper pattern.

## Files Modified

| File | Changes |
|------|---------|
| `crates/uml-io/src/xmi/reader.rs` | +imports (`Actor`, `UseCase`); +`parse_simple_element()` helper; +`parse_actor()`/`parse_usecase()`; +4 dispatch cases (Event::Start + Event::Empty); +6 new tests |
| `crates/uml-io/src/xmi/writer.rs` | +import (`ElementBase`); +`write_simple_element()` helper; +`write_element()` dispatch (Actor, UseCase); +`guess_widget_type()` cases; +`model_with_various_types()` extended; +`round_trip_and_compare()` counts; +6 new tests |

No changes to `uml-core`, `uml-codegen`, or `apps/umbrello`.

## New Types/Functions

### `reader.rs`
- `XmiReader::parse_simple_element()` — shared helper for bare-`ElementBase` types
- `XmiReader::parse_actor()` — parses `<UML:Actor>` elements
- `XmiReader::parse_usecase()` — parses `<UML:UseCase>` elements

### `writer.rs`
- `XmiWriter::write_simple_element()` — writes self-closing UML element tags for bare-`ElementBase` types

## Test Coverage

### Reader Tests (6 new)
| Test | What it verifies |
|------|-----------------|
| `parse_actor_from_xmi` | Actor with name, visibility, ObjectType parsed correctly |
| `parse_usecase_from_xmi` | UseCase with name, visibility, ObjectType parsed correctly |
| `parse_actor_in_package` | Two actors inside a Model package |
| `parse_usecase_in_package` | Two use cases inside a Model package |
| `parse_actor_with_stereotype` | Actor with stereotype attribute parses without error (stereotype deferred) |
| `load_real_duc_xmi_actors_usecases` | Real `test-DUC.xmi` file: ≥4 actors, ≥9 use cases |

### Writer Tests (6 new)
| Test | What it verifies |
|------|-----------------|
| `write_actor_to_xmi` | Actor produces `<UML:Actor .../>` with correct attributes |
| `write_usecase_to_xmi` | UseCase produces `<UML:UseCase .../>` with correct attributes |
| `guess_actor_widget_type` | `guess_widget_type()` returns `"actorwidget"` |
| `guess_usecase_widget_type` | `guess_widget_type()` returns `"usecasewidget"` |
| `actor_roundtrip` | Actor survives write→read→compare cycle |
| `usecase_roundtrip` | UseCase survives write→read→compare cycle |

### Extended Coverage
- `model_with_various_types()` now includes Actor and UseCase
- `round_trip_and_compare()` now counts actors and usecases
- `write_various_types()` now checks for `UML:Actor` and `UML:UseCase` in output
- Existing `parse_all_cpp_test_files` corpus test automatically covers `test-DUC.xmi`

## Test Counts

| Suite | Before (M19) | Phase 1 Added | Phase 2 Added | Current |
|-------|-------------|---------------|---------------|---------|
| `uml-core` unit | 140 | +8 | 0 | 158 |
| `uml-core` id_tests | 8 | 0 | 0 | 8 |
| `uml-core` serde_roundtrip | 6 | +2 | 0 | 8 |
| `uml-core` diagram_geometry | 2 | 0 | 0 | 2 |
| `uml-core` history | 4 | 0 | 0 | 4 |
| `uml-io` XMI | 46 | 0 | +12 | 58 |
| `uml-io` real corpus | 1 | 0 | 0 | 1 |
| `apps/umbrello` | 58 | 0 | 0 | 58 |
| Doc-tests | 1 | 0 | 0 | 1 |
| **Total** | **275** | **+10** | **+12** | **298** |

## Verification

```sh
cargo test --workspace        # 298 passed
cargo clippy -- -D warnings   # 0 warnings
cargo fmt --all --check       # 0 diffs
```
