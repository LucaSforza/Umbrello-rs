# Milestone 20 — Phase 1 Completion Report

**Date:** 2026-06-26  
**Commit:** `f4a72b6` — "feat(uml-core): add Actor and UseCase domain types with ModelElement variants"

---

## Summary

Implemented Phase 1 of Milestone 20: Domain Model — Actor & UseCase Types. This covers the core domain model changes in `uml-core` only.

---

## Files Modified

| File | Change |
|------|--------|
| `crates/uml-core/src/elements.rs` | Added `Actor` struct, `UseCase` struct (each with `ElementBase` + `new()` constructor); added `Actor(Actor)` and `UseCase(UseCase)` variants to `ModelElement` enum; propagated match arms in `object_type()`, `base()`, `base_mut()`, `classifier_data()`, `classifier_data_mut()`; added 9 unit tests |
| `crates/uml-core/src/lib.rs` | Re-exported `Actor` and `UseCase` from `elements` module |
| `crates/uml-core/src/repository.rs` | Added `Actor`/`UseCase` arms to `validate_references()` (empty bodies — these types have no additional references beyond the generic stereotype check) |
| `crates/uml-core/tests/serde_roundtrip.rs` | Added `serde_roundtrip_actor` and `serde_roundtrip_usecase` tests |

---

## New Types & Functions

| Name | Kind | Description |
|------|------|-------------|
| `Actor` | Struct | Non-classifier, non-container element with `ElementBase` |
| `Actor::new(name)` | Constructor | Creates Actor with generated UmlId, Public visibility |
| `UseCase` | Struct | Non-classifier, non-container element with `ElementBase` |
| `UseCase::new(name)` | Constructor | Creates UseCase with generated UmlId, Public visibility |

---

## Match Arms Propagated

| Method | Pattern |
|--------|---------|
| `object_type()` | `Self::Actor(_) => ObjectType::Actor`, `Self::UseCase(_) => ObjectType::UseCase` |
| `base()` | `Self::Actor(a) => &a.base`, `Self::UseCase(u) => &u.base` |
| `base_mut()` | `Self::Actor(a) => &mut a.base`, `Self::UseCase(u) => &mut u.base` |
| `classifier_data()` | Added `Self::Actor(_) \| Self::UseCase(_)` to `None` arm |
| `classifier_data_mut()` | Added `Self::Actor(_) \| Self::UseCase(_)` to `None` arm |
| `validate_references()` | Added `ModelElement::Actor(_) \| ModelElement::UseCase(_)` empty arm |

No changes needed to `is_classifier()`, `is_package()`, `is_container()` — these use `matches!()` with explicit variant lists that correctly exclude non-classifier/non-package types.

---

## Test Coverage Summary

### New Tests in `elements.rs` (9 tests)

| Test Name | Verifies |
|-----------|----------|
| `actor_creation` | Actor created with correct name, Public visibility, no stereotype, empty doc |
| `actor_model_element_insert` | Actor insertable into UmlModel, retrievable by ID, `object_type()` == `ObjectType::Actor`, name preserved |
| `actor_not_classifier` | `ModelElement::Actor(...).is_classifier()` returns `false` |
| `actor_not_container` | `ModelElement::Actor(...).is_package()` returns `false` |
| `serde_roundtrip_actor` | Actor round-trips through JSON |
| `usecase_creation` | UseCase created with correct name, Public visibility, no stereotype |
| `usecase_model_element_insert` | UseCase insertable into UmlModel, `object_type()` == `ObjectType::UseCase` |
| `usecase_not_classifier` | `ModelElement::UseCase(...).is_classifier()` returns `false` |
| `serde_roundtrip_usecase` | UseCase round-trips through JSON |

### New Tests in `tests/serde_roundtrip.rs` (2 tests)

| Test Name | Verifies |
|-----------|----------|
| `serde_roundtrip_actor` | Actor struct round-trips through JSON |
| `serde_roundtrip_usecase` | UseCase struct round-trips through JSON |

### Test Count

| Suite | Previous | New | Total |
|-------|----------|-----|-------|
| `uml-core` elements | 140 | +9 | 149 |
| `uml-core` serde_roundtrip | 6 | +2 | 8 |
| **Total (uml-core)** | **172** | **+11** | **183** |

---

## Verification

```sh
cargo test -p uml-core      # 183 tests, all passed
cargo clippy -p uml-core -- -D warnings  # Zero warnings
cargo fmt --all --check     # No formatting diffs
```

---

## Next Steps (Not in This Phase)

- Phase 2: XMI reader/writer in `uml-io`
- Phase 3: GUI rendering, tool palette, keyboard shortcuts in `apps/umbrello`
