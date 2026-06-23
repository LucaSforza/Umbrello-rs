# Phase 1 Architecture Audit — Milestones 1–5

**Date:** 2026-06-23
**Scope:** Full Cargo workspace, all crate contents, all domain model decisions.
**Goal:** Identify design debt, premature complexity, and actionable fixes before Phase 2.

---

## 1. Current State (Milestone 5)

### Workspace (21 crates, but only 2 have real code)

```
xtask/                              # Dev tooling — working
crates/uml-common/                  # 3 files, 62 lines (error types, version constants)
crates/uml-core/                    # 7 files, 3189 lines (the ONLY real code)
crates/uml-xmi/                     # 3 stub files (20–35 lines each)
crates/uml-persistence/             # 2 stub files
crates/uml-undo/                    # 1 stub file
crates/uml-diagram/                 # 2 stub files
crates/uml-layout/                  # 1 stub file
crates/uml-render/                  # 1 stub file
crates/uml-export/                  # 1 stub file
crates/uml-codegen/                 # 3 files, partly real (ProgramLanguage enum + CodeWriter)
crates/uml-codegen-cpp/             # 1 stub file
crates/uml-codegen-java/            # 1 stub file
crates/uml-codegen-python/          # 1 stub file
crates/uml-codegen-rust/            # 1 stub file
crates/uml-import/                  # 2 stub files
crates/uml-import-cpp/              # 1 stub file
crates/uml-import-java/             # 1 stub file
crates/uml-import-python/           # 1 stub file
apps/uml-cli/                       # 1 stub file
apps/umbrello-desktop/              # 1 stub file
```

### `uml-core` contents (the only active crate, 3189 lines)

| File | Lines | Contents |
|------|-------|----------|
| `src/elements.rs` | 960 | `NamedElement` trait, `ElementBase`, `ClassifierData`, `Package`, `Class`, `Interface`, `Enum`, `Relationship`, `Attribute`, `Operation`, `Parameter`, `TemplateParameter`, `EnumLiteral`, `ModelElement` enum (5 variants) |
| `src/types.rs` | 799 | `ObjectType` (30 variants), `AssociationType` (12), `DiagramType` (11), `Visibility` (4), `ParameterDirection` (4) + tests (32) |
| `src/repository.rs` | 1170 | `UmlModel` with `IndexMap`, parent index, cascading remove, cycle detection, reference validation, 27 tests |
| `src/id.rs` | 128 | `UmlId` (UUID-backed), 8 tests |
| `src/lib.rs` | 34 | Module declarations + re-exports |
| `src/event.rs` | 22 | Stub |
| `src/model.rs` | 14 | Stub |

### Test coverage: 110 tests

| Location | Count |
|----------|-------|
| `elements.rs` | 26 unit tests |
| `types.rs` | 24 unit tests |
| `repository.rs` | 38 unit tests |
| `id.rs` | 8 unit tests |
| `tests/id_tests.rs` | 8 integration tests |
| `tests/serde_roundtrip.rs` | 6 integration tests |
| **Total** | **110** |

---

## 2. Audit of Each Milestone

### M1: Cargo Workspace (commit `2b8b165`)

**Decision:** Created 21-crate workspace with 4 dependency tiers.

**Finding: PREMATURE CRATE PROLIFERATION — UNRESOLVED.** The `crate_boundary_review.md`
document (written after M1) identified 12 premature leaf crates and recommended
commenting them out of the workspace manifest. Despite clear analysis (see
`docs/crate_boundary_review.md` §3), none have been removed. The workspace still
compiles 21 crates every time, but 19 of them are empty stubs. While stubs
contribute minimal compilation cost (there is little or no code to compile), they
create structural noise: 12 directories × (Cargo.toml + src/lib.rs) = 24 files
that must be maintained and navigated. A developer scanning `members = [...]`
sees 21 crates and assumes the project is further along than it is.

**Finding: `uml-common` is too thin.** At 62 lines (error type + version
constants), `uml-common` does not justify being a separate crate. Its content
could live in `uml-core` without creating coupling issues. The original rationale
was "a crate like `uml-import` that only needs error types shouldn't pull in all
of `uml-core`" — but `uml-import` is itself an empty stub crate. This is a
theoretical concern with no practical weight at Phase 1 scale. However, merging
it now would create unnecessary churn; this is a **LOW** priority item.

**Finding: `ProgrammingLanguage` still in `uml-codegen`.** The
`crate_boundary_review.md` (see §2) recommended moving `ProgrammingLanguage` from
`uml-codegen` to `uml-core/src/types.rs`. This was deferred. The enum currently
lives in `crates/uml-codegen/src/lib.rs`, creating an inverted dependency
problem: import crates need `ProgrammingLanguage` but would have to depend on
`uml-codegen` to get it. This should be fixed before any import crate is
implemented.

**Verdict: DEFERRED ACTIONS.** The M1 recommendations were sound but not
executed. Deferring was acceptable to maintain velocity through M2–M5, but these
should be resolved before M6.

**Decision:** Added `slotmap` as a workspace dependency early.

**Finding: CORRECTED.** Milestone 4 correctly replaced slotmap with indexmap in
`uml-core`. Slotmap remains in workspace deps for potential future use by other
crates. This is fine — the workspace-level dependency does not affect compile
times.

---

### M2: Core Types (commit `d139e77`)

**Decision:** `UmlId` backed by `uuid::Uuid`. `ObjectType` (27→30 variants),
`AssociationType` (12), `DiagramType` (11), `Visibility` (4),
`ParameterDirection` (4).

**Finding: SOUND.** These are well-designed, complete, and well-tested (45
tests). No structural issues. Key strengths:

- `UmlId` is a newtype over `uuid::Uuid` with `Display`, `FromStr`, `serde`
  support, and 16 tests (unit + integration for round-trip).
- All enums derive `Serialize`, `Deserialize`, `Clone`, `Copy`, `PartialEq`,
  `Eq`, `Hash`, `Debug`, `Display`, `FromStr`. The test-at-definition pattern
  (`#[test]` blocks right after each enum) ensures from_str round-trips for
  every variant.
- `ObjectType` has 30 variants covering all UML classifiers plus diagram-level
  types (note `object_language`, `Activity`, `Component`, `Node`, etc.).

**Finding: ASSOCIATIONTYPE HAS SEMANTIC DRIFT.** The enum carries 12 variants,
but several are diagram-specific or EER-specific rather than core UML
relationship types:

- `Anchor` — diagram edge for note attachments, not a UML relationship.
- `Containment` — deployment diagram containment.
- `Exception` — EER-specific.
- `Category2Parent`, `Child2Category` — EER-specific.

These were carried over from the C++ `Uml::AssociationType` enum, which itself
mixes levels of abstraction. For the domain model, only 6 variants are needed:
`Generalization`, `Association`, `Aggregation`, `Composition`, `Dependency`,
`Realization`. The diagram-specific variants should move to `uml-diagram` when
that crate is implemented.

No immediate action required — the 6 extra variants do no harm in the enum —
but this should be tracked as design debt (D4).

---

### M3: UML Metamodel (commit `c4fc9f9`)

**Decision:** `ModelElement` enum with 4 variants (`Package`, `Class`,
`Interface`, `Enum`). `NamedElement` trait. `ClassifierData` composition.
Subordinate types (`Attribute`, `Operation`, `Parameter`, `TemplateParameter`,
`EnumLiteral`) as pure value types without independent identity.

**Finding: SOUND.** The composition-over-inheritance approach is correct and
well-reasoned. The `domain_model_review.md` confirmed all 15 architecture
principles. Specific strengths:

- `ModelElement` is an enum, not a trait object — stack-allocated, no vtable,
  exhaustive match.
- `ClassifierData` is factored out and reused by `Class`, `Interface`, `Enum`
  (and, later, `Datatype`, `Entity`, `Component`).
- Subordinate types are value types — no `UmlId`, no independent lifecycle.
  Attributes belong to their owning classifier.

**Finding: DUPLICATED TYPE REFERENCE PATTERN (D1 — HIGH).** The pattern
`type_id: Option<UmlId>` + `type_name: Option<String>` appears in three
independent locations:

```rust
// Attribute
pub type_id: Option<UmlId>,
pub type_name: Option<String>,

// Operation
pub return_type_id: Option<UmlId>,
pub return_type_name: Option<String>,

// Parameter
pub type_id: Option<UmlId>,
pub type_name: Option<String>,
```

This is identical to the pattern in the C++ codebase, but in Rust we can do
better. The duplication means:
- Three places to update if the representation changes.
- No invariant enforcement (both `Some`, both `None`, or mixed — the last is
  meaningless).
- Repeated serde attributes, doc comments, validation logic.

**This is the single most impactful refactoring target in the codebase.** See
Recommendations (§5) for the proposed `TypeReference` extraction.

**Finding: CLASSIFIERDATA ACCESS ASYMMETRY (D5 — LOW).** Elements can access
their classifier data via `model_element.classifier_data()`, but there is no
trait like `HasClassifierData`. The `is_classifier()` method on `ModelElement`
returns a `bool`, but callers that know the element is a classifier must still
`classifier_data().unwrap()`. This is minor but creates a pattern of guarded
unwraps that could be eliminated with a trait method returning
`Option<&ClassifierData>`.

**Finding: SUBORDINATE TYPE NAMES (minor).** `EnumLiteral` is the only
subordinate type that doesn't follow the pattern of the owning parent
(`Attribute` on `Class`/`Interface`, `Operation` on `Class`/`Interface`,
`EnumLiteral` on `Enum`). No issue — this is correct — but worth noting for
consistency when adding new types.

---

### M4: Model Repository (commit `a4f4979`)

**Decision:** `UmlModel` with `IndexMap<UmlId, ModelElement>` + parent_index.
Cascading remove. Cycle detection. Reference validation.

**Finding: SOUND.** The `IndexMap` over `SlotMap` decision was well-reasoned
(see `model_repository_review.md`). The implementation is clean:

- `parent_index: IndexMap<UmlId, HashSet<UmlId>>` enables O(1) child lookup.
- `cascade_remove()` removes children transitively, preventing dangling parent
  references.
- `add_to_package()` detects cycles in O(depth) by walking ancestors.
- `validate_references()` checks that every `type_id` reference resolves to an
  existing element, logging warnings for broken references.

**Finding: VALIDATE_REFERENCES IS O(N×M) BUT ACCEPTABLE.** For 10,000 elements
with ~5 references each, this is 50,000 lookups — sub-millisecond with `IndexMap`
hash lookups. No issue.

**Finding: NO DELETE/CASCADE EVENT EMISSION (D7 relationship).** The `event.rs`
module is a 22-line stub. `cascade_remove()` currently just removes elements
without notifying anything. When undo/redo or widget synchronization is
implemented, remove operations must emit events. This is not a defect — events
were intentionally deferred — but the stub should either be removed (cleaner)
or implemented (blocking).

**Tests: 38 unit tests + 6 serde roundtrip integration tests.** Covers all public
methods, including edge cases: removing non-existent elements, cycles, duplicate
adds, and serialization round-trips.

---

### M5: Relationships (commit `84e4dd4`)

**Decision:** `Relationship` as `ModelElement` variant with 6 constructor methods
(`new_generalization`, `new_association`, `new_aggregation`, `new_composition`,
`new_dependency`, `new_realization`). Cascading cleanup on remove. Query methods
(`relationships()`, `get_related_classifiers()`).

**Finding: SOUND.** The design review confirmed XMI compatibility is the
decisive factor. `Relationship` is stored as a first-class element in the model
(receiving its own `UmlId`), not as an edge in a separate graph structure. This
matches XMI where `<ownedElement>` contains `<Generalization>` as a top-level
element. Key strengths:

- `source_id` + `target_id` fields on all relationship types (via
  `RelationshipData`).
- 6 typed constructors ensure callers can't mix up fields.
- `classifier_data()` returns `None` (relationships are not classifiers).
- Remove cascade: deleting a relationship's source or target triggers
  relationship removal.

**Finding: ASSOCIATIONTYPE ENUM HAS SEMANTIC DRIFT (D4 — MEDIUM).** Milestone 5
only implements 6 of 12 `AssociationType` variants as constructor methods. The
remaining 6 (`DirectedAssociation`, `Anchor`, `Containment`, `Exception`,
`Category2Parent`, `Child2Category`) are not used by any real code. The
`matches_association_type()` method on `RelationshipData` works for all 12, but
only 6 are reachable. This is not a bug — the enum was defined before the
implementation was scoped — but it creates dead code paths. When diagram
rendering is implemented, the unused variants should move to an
`uml-diagram::EdgeKind` enum.

**Finding: NO RELATIONSHIP-SPECIFIC VALIDATION.** `new_generalization(source,
target)` creates a `Generalization` even if source == target, or if either is
not a classifier. The C++ codebase checks these constraints. Consider adding:

```rust
pub fn new_generalization(
    source: UmlId,
    target: UmlId,
    model: &UmlModel,
) -> Result<Relationship, RelationshipError> {
    // Validate source/target are classifiers (Class, Interface, Enum)
    // Validate source != target
    // Validate no circular generalization
}
```

This can be deferred until validation infrastructure is built (Milestone 6+).

**Tests: 16 new tests** covering constructor methods, serialization, query
methods, and cascading removal.

---

## 3. Design Debt Inventory

| # | Severity | Area | Description | Resolution |
|---|----------|------|-------------|------------|
| D1 | **HIGH** | `elements.rs` | Duplicated type reference pattern (`type_id` + `type_name`) on `Attribute`, `Operation`, `Parameter` (6 redundant fields) | Extract `TypeReference` struct — see §5 |
| D2 | **MEDIUM** | workspace | 12 premature leaf crates still in workspace manifest despite clear recommendation to comment out | Comment out per `crate_boundary_review.md` table |
| D3 | **LOW** | workspace | `uml-common` too thin (62 lines) — could merge into `uml-core` | Defer until another crate actually needs `uml-common` types |
| D4 | **MEDIUM** | `types.rs` | `AssociationType` enum contains diagram-specific variants (`Anchor`, `Containment`, `Exception`, `Category2Parent`, `Child2Category`) plus unimplemented `DirectedAssociation` | Split when diagram crate is implemented — core types in `uml-core`, edge types in `uml-diagram` |
| D5 | **LOW** | `elements.rs` | `ClassifierData` access has no trait — callers must use `is_classifier()` + `classifier_data().unwrap()` | Add `HasClassifierData` trait or richer API |
| D6 | **LOW** | `elements.rs` | `object_type()` method on `ModelElement` collides with `ObjectType` enum name — confusing in code reading: `element.object_type()` | Rename to `element_kind()` when convenient |
| D7 | **LOW** | `model.rs`, `event.rs` | Two empty stub modules (14 + 22 lines) — dead code that compiles but does nothing | Remove modules or implement |
| D8 | **LOW** | workspace | `ProgrammingLanguage` enum lives in `uml-codegen` instead of `uml-core/src/types.rs` — creates inverted dependency for import crates | Move to `uml-core`, re-export from `uml-codegen` |
| D9 | **LOW** | `repository.rs` | No events emitted on remove — `event.rs` stub means `cascade_remove()` notifies nothing | Implement event stub when undo/redo is built |
| D10 | **LOW** | `elements.rs` | No relationship validation — `new_generalization` accepts source == target or non-classifier endpoints | Add `Result`-returning constructors with validation |

---

## 4. Overengineering Check

| Decision | Overengineered? | Why |
|----------|----------------|------|
| 21-crate workspace | **Yes** | 19/21 crates are stubs. Only 2 have real code. 12 were explicitly identified as premature. |
| 4-tier dependency system | **Yes for current size** | With 2 active crates, 4 tiers is over-designed. But it will pay off as crates are implemented. Acceptable as forward-planning. |
| `NamedElement` trait | **No** | Clean abstraction, minimal overhead, enables generic code over named things. |
| `ClassifierData` composition | **No** | Correctly avoids inheritance. Enables `Class`, `Interface`, `Enum` to share fields without a trait object or virtual dispatch. |
| `IndexMap` over `HashMap` | **No** | Deterministic iteration (insertion order) justifies the dependency. Essential for XMI round-trip stability. |
| Cycle detection in `add_to_package` | **No** | Prevents a real bug class (infinite recursion on serialization / tree walk). O(depth) cost is negligible. |
| Cascading remove | **No** | Prevents dangling references and orphaned elements. Standard in any model repository. |
| 6 Relationship constructor methods | **No** | Convenient, zero-cost abstraction. Each constructor is 5–10 lines with clear field initialization. |
| UUID-backed `UmlId` | **No** | XMI requires globally unique IDs. `uuid` is the standard solution. `UmlId` newtype prevents ID confusion. |
| `#[serde(rename = "camelCase")]` on fields | **Minor** | Ensures XMI compatibility, but adds maintenance surface. Acceptable given the requirement. |

### Summary

Only one decision is clearly overengineered for the current phase: the 21-crate
workspace with 19 stubs. The 4-tier system and `ProgrammingLanguage` placement
are conceptual overengineering — they add complexity without benefit at the
current scale but would be correct at scale. All other decisions are appropriate
for the problem domain.

---

## 5. Recommendations

### 5.1 Immediate (M6): Extract `TypeReference` (D1 — HIGH)

The `type_id: Option<UmlId>` + `type_name: Option<String>` pattern is duplicated
on `Attribute`, `Operation` (as `return_type_id`/`return_type_name`), and
`Parameter`. Extract a single `TypeReference` struct:

```rust
/// A reference to a UML classifier or primitive type.
///
/// In UML, an attribute, parameter, or return type can reference either a
/// model classifier (by `UmlId`) or a built-in primitive type (by name).
/// At most one of the two fields is set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeReference {
    /// Reference to a model element (Class, Interface, Enum, Datatype, etc.)
    pub model_id: Option<UmlId>,

    /// Fallback name for primitive types (e.g., "int", "String", "bool")
    /// Used when there is no model element reference.
    pub type_name: Option<String>,
}
```

This eliminates 6 fields across 3 structs and replaces them with:

```rust
pub struct Attribute {
    // ... existing fields ...
    pub type_ref: TypeReference,
    // type_id, type_name removed
}

pub struct Operation {
    // ... existing fields ...
    pub return_type: TypeReference,
    // return_type_id, return_type_name removed
}

pub struct Parameter {
    // ... existing fields ...
    pub type_ref: TypeReference,
    // type_id, type_name removed
}
```

**Additional validation:** Add constructors or setters that enforce at most one
field is set:

```rust
impl TypeReference {
    /// Create a reference to a model classifier.
    pub fn model(id: UmlId) -> Self {
        Self { model_id: Some(id), type_name: None }
    }

    /// Create a reference to a primitive type by name.
    pub fn primitive(name: impl Into<String>) -> Self {
        Self { model_id: None, type_name: Some(name.into()) }
    }

    /// Returns `true` if this references a known model element.
    pub fn is_resolved(&self) -> bool {
        self.model_id.is_some()
    }

    /// Returns `true` if this is a primitive type name.
    pub fn is_primitive(&self) -> bool {
        self.type_name.is_some()
    }
}
```

**Impact:** Reduces field count by 3. Reduces match noise in callers. Enables
future extension (e.g., adding `is_ordered` or `is_unique` for UML multiplicity).

### 5.2 Immediate (M6): Move `ProgrammingLanguage` to `uml-core` (D8 — LOW)

Move the enum from `crates/uml-codegen/src/lib.rs` to
`crates/uml-core/src/types.rs` and re-export from `uml-codegen`:

```rust
// In crates/uml-codegen/src/lib.rs, replace the local definition:
pub use uml_core::types::ProgrammingLanguage;
```

This fixes the inverted dependency that would block `uml-import` from depending
on `uml-core` for the enum without pulling in `uml-codegen`.

### 5.3 Immediate: Clean up workspace (D2 — MEDIUM)

Comment out the 12 premature leaf crates from `Cargo.toml` as specified in
`crate_boundary_review.md` §3. The workspace reduces from 22 members to 10:

```toml
[workspace]
resolver = "2"
members = [
    "xtask",
    "crates/uml-common",
    "crates/uml-core",
    "crates/uml-codegen",
    "crates/uml-import",
    "crates/uml-xmi",
    "crates/uml-undo",
    "crates/uml-diagram",
    "crates/uml-layout",

    # Commented out:
    # "crates/uml-persistence",
    # "crates/uml-codegen-cpp",
    # "crates/uml-codegen-java",
    # "crates/uml-codegen-python",
    # "crates/uml-codegen-rust",
    # "crates/uml-import-cpp",
    # "crates/uml-import-java",
    # "crates/uml-import-python",
    # "crates/uml-render",
    # "crates/uml-export",
    # "apps/uml-cli",
    # "apps/umbrello-desktop",
]
```

No code is deleted — only the workspace membership is removed. The directories
and their `Cargo.toml` files remain on disk for when they are needed.

### 5.4 Immediate (optional): Remove or implement stub modules (D7 — LOW)

Two modules in `uml-core` are empty stubs:

- `src/model.rs` (14 lines) — was intended for the top-level `UmlModel` but
  that lives in `repository.rs`
- `src/event.rs` (22 lines) — stub for future event emission

Either remove both (`pub mod model;` and `pub mod event;` from `lib.rs` plus
the files), or implement minimal scaffolding. Removal is cleaner — the event
system is not designed yet and the module exports nothing.

### 5.5 Future (M7+): Split `AssociationType` (D4 — MEDIUM)

When diagram rendering is implemented, split the enum:

- **`uml-core::types::RelationshipKind`** — the 6 core UML relationships:
  `Generalization`, `Association`, `Aggregation`, `Composition`, `Dependency`,
  `Realization`.
- **`uml-diagram::types::EdgeKind`** — diagram edge types: `Anchor`,
  `Containment`, `Exception`, `DirectedAssociation`, `Category2Parent`,
  `Child2Category`.

The `Relationship` element in `uml-core` uses `RelationshipKind`. Diagram
widgets use `EdgeKind`. This cleanly separates domain model from presentation.

### 5.6 Future (M7+): Add `HasClassifierData` trait (D5 — LOW)

```rust
/// Trait for model elements that carry classifier data
/// (attributes, operations, template parameters, etc.).
pub trait HasClassifierData {
    fn classifier_data(&self) -> Option<&ClassifierData>;
    fn classifier_data_mut(&mut self) -> Option<&mut ClassifierData>;
}
```

Implement on `Class`, `Interface`, `Enum` (and future `Datatype`, `Component`,
etc.). This eliminates match-and-unwrap patterns in generic code.

### 5.7 Future (M7+): Add relationship validation (D10 — LOW)

Convert the 6 `Relationship` constructors from pure-data initialization to
`Result`-returning functions:

```rust
pub fn new_generalization(
    source: UmlId,
    target: UmlId,
    model: &UmlModel,
) -> Result<Relationship, RelationshipError> {
    // Reject self-generalization
    // Reject if source or target is not a classifier
    // Reject circular generalization chains
    // ...
}
```

This matches the C++ validation model and prevents invalid model states.

---

## 6. Prioritized Action Plan

### Must-do before Milestone 6

| # | Priority | Action | Debt | Effort |
|---|----------|--------|------|--------|
| 1 | **P0** | Extract `TypeReference` — consolidate 6 fields into 1 struct | D1 | ~2 hours (code) + ~2 hours (test update) |
| 2 | **P0** | Move `ProgrammingLanguage` enum to `uml-core/src/types.rs` | D8 | ~30 minutes |
| 3 | **P1** | Comment out 12 premature leaf crates from `Cargo.toml` | D2 | ~15 minutes |
| 4 | **P2** | Remove or implement `model.rs` and `event.rs` stubs | D7 | ~10 minutes |

### Should-do in Milestone 7

| # | Priority | Action | Debt | Effort |
|---|----------|--------|------|--------|
| 5 | **P1** | Split `AssociationType` into `RelationshipKind` + `EdgeKind` | D4 | ~4 hours (includes diagram crate setup) |
| 6 | **P2** | Add `HasClassifierData` trait | D5 | ~1 hour |
| 7 | **P2** | Rename `object_type()` to `element_kind()` | D6 | ~30 minutes |
| 8 | **P2** | Add `Result`-based validation to Relationship constructors | D10 | ~2 hours |

### Can-defer indefinitely

| # | Priority | Action | Debt | Effort |
|---|----------|--------|------|--------|
| 9 | **P3** | Merge `uml-common` into `uml-core` | D3 | ~1 hour (not worth it until another crate needs `uml-common`) |
| 10 | **P3** | Implement event emission on remove | D9 | Part of undo/redo milestone |

### Total immediate effort

| Task | Hours |
|------|-------|
| TypeReference extraction | 4 |
| ProgrammingLanguage move | 0.5 |
| Workspace cleanup | 0.25 |
| Stub module cleanup | 0.15 |
| **Total (P0–P2)** | **~4.9 hours** |

---

## Appendix: Files Referenced

| Path | Lines | Description |
|------|-------|-------------|
| `crates/uml-core/src/elements.rs` | 960 | Metamodel: `ModelElement`, `NamedElement`, `ClassifierData`, all element types |
| `crates/uml-core/src/types.rs` | 799 | Enums: `ObjectType`, `AssociationType`, `DiagramType`, `Visibility`, `ParameterDirection` |
| `crates/uml-core/src/repository.rs` | 1170 | Model repository: `UmlModel` with `IndexMap`, parent index, remove cascade, cycle detection |
| `crates/uml-core/src/id.rs` | 128 | `UmlId` newtype over `uuid::Uuid` |
| `crates/uml-core/src/lib.rs` | 34 | Module declarations and re-exports |
| `crates/uml-core/src/event.rs` | 22 | Stub — event system not yet designed |
| `crates/uml-core/src/model.rs` | 14 | Stub — empty module |
| `crates/uml-codegen/src/lib.rs` | 60 | `ProgrammingLanguage` enum (wrong location) + `CodeWriter` trait |
| `crates/uml-common/src/error.rs` | 37 | `UmbrelloError` enum |
| `crates/uml-common/src/version.rs` | 10 | Version constants |
| `docs/crate_boundary_review.md` | 261 | M1 crate boundary analysis — recommendations not yet executed |
| `docs/domain_model_review.md` | 744 | M3 domain model review |
| `docs/model_repository_review.md` | 619 | M4 repository design review |
| `docs/relationships_review.md` | 1060 | M5 relationship design review |
| `docs/testing_strategy.md` | 2013 | Overall testing approach |
