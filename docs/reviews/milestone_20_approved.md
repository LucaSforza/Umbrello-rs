# Milestone 20 — Review Approval

**Status:** APPROVED

**Review date:** 2026-06-26

**Reviewed commits:** `f4a72b6`, `e3a5a95`, `0429f21` (plus the fixup adding `parse_usecase_with_comment`)

**Reviewer:** Umbrello-RS Reviewer

---

## Summary

The Milestone 20 implementation (Actor & UseCase Element Types) covers all three phases: domain model types in `uml-core`, XMI reader/writer in `uml-io`, and GUI rendering/tool palette in `apps/umbrello`. The implementation is clean, well-tested, and fully compliant with the design document.

---

## Review Dimensions

### 1. Design Compliance

| Requirement | Status | Notes |
|-------------|--------|-------|
| `Actor` struct with only `ElementBase` | ✅ | `elements.rs:636` |
| `UseCase` struct with only `ElementBase` | ✅ | `elements.rs:667` |
| `ModelElement::Actor(Actor)` and `ModelElement::UseCase(UseCase)` variants | ✅ | `elements.rs:712-715` |
| Match arms in `base()`, `base_mut()`, `object_type()` | ✅ | Lines 729-730, 744-745, 758-759 |
| `classifier_data()` / `classifier_data_mut()` returns `None` | ✅ | Lines 800, 811 |
| `is_classifier()`/`is_package()` unchanged (wildcard correct) | ✅ | Lines 782-790 |
| `validate_references()` empty arm | ✅ | `repository.rs:388-390` |
| Re-exports in `lib.rs` | ✅ | Lines 45, 47 |
| XMI reader: `parse_simple_element()` helper | ✅ | `reader.rs:962-974` |
| XMI reader: `parse_actor()` / `parse_usecase()` | ✅ | `reader.rs:976-990` |
| XMI reader: dispatch in `Event::Start` | ✅ | Lines 350-361 |
| XMI reader: dispatch in `Event::Empty` | ✅ | Lines 483-494 |
| XMI writer: `write_simple_element()` helper | ✅ | `writer.rs:286-310` |
| XMI writer: `write_element()` Actor/UseCase dispatch | ✅ | Lines 277-278 |
| XMI writer: `guess_widget_type()` Actor/UseCase | ✅ | Lines 881-882 |
| `element_color()` Actor (light orange) | ✅ | `rendering.rs:18` |
| `element_color()` UseCase (light coral) | ✅ | `rendering.rs:19` |
| Stick-figure rendering for Actor | ✅ | `canvas.rs:561-604` |
| Ellipse rendering for UseCase | ✅ | `canvas.rs:606-623` |
| `CreateActor` / `CreateUseCase` `ToolMode` variants | ✅ | `tool_palette.rs:31,33` |
| `is_creation_tool()` includes both | ✅ | Lines 107-108 |
| `create_element_for_tool()` cases | ✅ | Lines 168-175 |
| Palette buttons | ✅ | Lines 251-252 |
| Keyboard shortcuts (T, U) | ✅ | `app.rs:237-246` |

### 2. Test Coverage

| Test Suite | Expected | Actual | Status |
|------------|----------|--------|--------|
| `uml-core` elements unit tests | ~147 | 158 | ✅ (includes pre-existing tests) |
| `uml-core` serde_roundtrip | ~8 | 8 | ✅ |
| `uml-io` XMI tests | ~57 | 59 | ✅ |
| `uml-io` real corpus | ~2 | 1 | ✅ (test-DUC.xmi loaded, actors/usecases counted) |
| `apps/umbrello` tests | ~71 | 71 | ✅ |
| **Total** | **~309** | **312** | **✅** |

All specific test IDs from the design exist and pass:

- **DM-20** through **DM-27**: `actor_creation`, `usecase_creation`, `actor_model_element_insert`, `usecase_model_element_insert`, `actor_not_classifier`, `usecase_not_classifier`, `actor_not_container`, `serde_roundtrip_actor`, `serde_roundtrip_usecase` ✅
- **SRT-8, SRT-9**: `serde_roundtrip_actor`, `serde_roundtrip_usecase` ✅
- **XMI-20 through XMI-24**: `parse_actor_from_xmi`, `parse_usecase_from_xmi`, `parse_actor_in_package`, `parse_usecase_in_package`, `parse_actor_with_stereotype`, `parse_usecase_with_comment` ✅
- **XMI-25 through XMI-30**: `write_actor_to_xmi`, `write_usecase_to_xmi`, `guess_actor_widget_type`, `guess_usecase_widget_type`, `actor_roundtrip`, `usecase_roundtrip` ✅
- **CORP-1**: `load_real_duc_xmi_actors_usecases` (≥4 actors, ≥9 use cases) ✅
- **APP-28 through APP-40**: All 13 app tests ✅

Note: The design lists XMI-24 as `parse_usecase_with_comment`. This test was missing from the original implementation and has been added during this review (test verifies UseCase with `comment="asfs"` attribute parses without error). All other tests were present and passing.

### 3. Architecture Rules

| Rule | Status | Notes |
|------|--------|-------|
| No changes to `uml-codegen` | ✅ | Zero modifications |
| `uml-core` remains pure (no GUI deps) | ✅ | No egui or GUI types added |
| No circular dependencies | ✅ | Dependency graph unchanged |
| XMI format compatibility | ✅ | `original_xmi_id` preserved via `build_base()` |
| Composition over inheritance | ✅ | Actor/UseCase are bare `ElementBase` with no `ClassifierData` |

### 4. Code Quality

| Check | Status | Notes |
|-------|--------|-------|
| No `unwrap()`/`expect()` in production code | ✅ | All occurrences are in `#[cfg(test)]` blocks |
| No dead code | ✅ | All new code is referenced |
| `#[must_use]` on public functions | ✅ | Present on `Actor::new()` and `UseCase::new()` |
| Consistent naming | ✅ | Follows existing patterns |
| Error handling | ✅ | `Result<_, XmiParseError>` / `Result<_, XmiWriteError>` used throughout |

### 5. Compilation & Testing

| Check | Result |
|-------|--------|
| `cargo test --workspace` | ✅ 312 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | ✅ Zero warnings |
| `cargo fmt --all --check` | ✅ Exit code 0 (only stable-rustfmt warnings about nightly-only config) |

---

## Minor Issues Fixed During Review

1. **Missing test (XMI-24):** The `parse_usecase_with_comment` test was missing from the reader test suite. Added it at `reader.rs:2625-2654`. This test verifies that a `<UML:UseCase>` element with a `comment="asfs"` attribute parses correctly without errors (the `comment` attribute is gracefully ignored by `build_base()`).

---

## Conclusion

**APPROVED.** The implementation is architecturally sound, design-compliant, well-tested, and passes all verification checks. The code quality is high, with proper error handling, no unsafe code, and clean separation of concerns across the three crates.
