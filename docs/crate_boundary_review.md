# Milestone 1 Crate Boundary Review

**Date:** 2026-06-23
**Context:** 21-crate workspace; only `uml-common` and `uml-core` have real code.
**Goal:** Eliminate premature crate boundaries before Phase 1 implementation begins.

---

## 1. Current Workspace State

The workspace has 21 crates across 4 dependency tiers:

| Tier | Crates | Status |
|------|--------|--------|
| **Tier 0** (foundation) | `uml-common`, `uml-core` | ✅ Active |
| **Tier 1** (infrastructure) | `uml-xmi`, `uml-persistence`, `uml-undo`, `uml-diagram` | Stubs |
| **Tier 2** (features) | `uml-layout`, `uml-render`, `uml-export`, `uml-codegen`, `uml-codegen-cpp`, `uml-codegen-java`, `uml-codegen-python`, `uml-codegen-rust`, `uml-import`, `uml-import-cpp`, `uml-import-java`, `uml-import-python` | Stubs |
| **Tier 3** (applications) | `uml-cli`, `umbrello-desktop` | Stubs |
| **Tooling** | `xtask` | ✅ Active |

---

## 2. Key Finding: `ProgrammingLanguage` Belongs in `uml-core`

The `ProgrammingLanguage` enum (21 variants) is currently defined in
`crates/uml-codegen/src/lib.rs`:

```rust
// crates/uml-codegen/src/lib.rs (current location — WRONG)
pub enum ProgrammingLanguage {
    Ada, ActionScript, Cpp, CSharp, D, Idl, Java, JavaScript,
    Pascal, Perl, Php4, Php5, Python, Ruby, Rust,
    Sql, MySql, PostgreSql, Tcl, Vala, XmlSchema,
}
```

### Why it must move to `uml-core/src/types.rs`

1. **It is a domain type, not a codegen type.** The language of a UML model is an
   intrinsic property, not an artifact of code generation. In the C++ codebase,
   `ProgrammingLanguage::Enum` lives in `basictypes.h` alongside `DiagramType`,
   `Visibility`, and `ObjectType`.

2. **Import needs it too.** The `uml-import` crate must reference
   `ProgrammingLanguage` in its `ImportRegistry` API — importers *produce* models
   for a given language. With the enum in `uml-codegen`, `uml-import` would have
   an inverted dependency on `uml-codegen`, which is architecturally wrong.

3. **Eliminates the `uml-codegen` → `uml-core` bottleneck.** Currently
   `uml-codegen` depends on `uml-core`. If codegen and import both depend on
   `uml-codegen` just for the enum, we create a false coupling between two
   independent feature areas.

4. **Follows the existing pattern.** `ObjectType`, `Visibility`, `DiagramType`,
   and `AssociationType` all live in `uml-core/src/types.rs`. The module doc
   already says: *"Enumerations: ObjectType, AssociationType, DiagramType,
   Visibility, etc."* — `ProgrammingLanguage` completes that set.

### Concrete change

```rust
// — In crates/uml-core/src/types.rs, add:
//
// /// Programming languages supported by Umbrello.
// #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
// pub enum ProgrammingLanguage {
//     Ada,
//     ActionScript,
//     Cpp,
//     CSharp,
//     D,
//     Idl,
//     Java,
//     JavaScript,
//     Pascal,
//     Perl,
//     Php4,
//     Php5,
//     Python,
//     Ruby,
//     Rust,
//     Sql,
//     MySql,
//     PostgreSql,
//     Tcl,
//     Vala,
//     XmlSchema,
// }

// — In crates/uml-codegen/src/lib.rs, replace:
pub use uml_core::types::ProgrammingLanguage;
// and remove the local enum definition.
```

After this move, the `uml-codegen` crate becomes a thin trait+writer crate that
depends on `uml-core` for the enum. The `uml-import` crate can also depend on
`uml-core` for the enum without any dependency on `uml-codegen`.

---

## 3. Problem: 12 Premature Leaf Crates

### Verdicts

| Crate | Phase | Verdict |
|-------|-------|---------|
| `uml-codegen-cpp` | Phase 13 | ❌ Comment out. Empty stub; C++ generation is Phase 13. |
| `uml-codegen-java` | Phase 14 | ❌ Comment out. Empty stub; Java generation is Phase 14. |
| `uml-codegen-python` | Phase 15 | ❌ Comment out. Empty stub; Python generation is Phase 15. |
| `uml-codegen-rust` | Phase 22 | ❌ Comment out. Rust generation is Phase 22 — far future. |
| `uml-import-cpp` | Phase 9 | ❌ Comment out. Empty stub; C++ import is Phase 9. |
| `uml-import-java` | Phase 10 | ❌ Comment out. Empty stub; Java import is Phase 10. |
| `uml-import-python` | Phase 11 | ❌ Comment out. Empty stub; Python import is Phase 11. |
| `uml-render` | Phase 18 | ❌ Comment out. Depends on diagram types not yet designed. |
| `uml-export` | Phase 18+ | ❌ Comment out. Depends on `uml-render`; no diagram output yet. |
| `uml-persistence` | Phase 5 | ❌ Comment out. Depends on `uml-xmi` (stub). Real types (`FileFormat`, `PersistenceError`, `StorageBackend`) should move to `uml-common` if needed now, or wait for Phase 5. |
| `uml-cli` | Phase 7 | ❌ Comment out. Depends on `uml-persistence` (stub). Only a `clap` skeleton. |
| `umbrello-desktop` | Phase 20 | ❌ Comment out. GUI application; years away. |

### Harm of premature leaf crates

These 12 crates contribute:

- **12 directories × (Cargo.toml + src/lib.rs) = 24 empty/trivial files** that
  must be maintained and navigated.
- **10 inter-crate dependency edges** resolved at build time for code that does
  nothing.
- **False progress signal** — a developer scanning `members = [...]` sees 21
  crates and assumes the project is further along than it is.
- **Merge conflict surface** — every Cargo.toml addition modifies the workspace
  manifest, creating unnecessary churn.

The principle: **add a crate boundary when its types are implemented, tested,
and needed by at least two consumers, not when it is planned for a future
phase.**

---

## 4. Recommended Workspace Members

### Keep (9 crates + xtask)

```toml
[workspace]
resolver = "2"
members = [
    "xtask",

    # Foundation (Tier 0)
    "crates/uml-common",
    "crates/uml-core",

    # Infrastructure (Tier 1)
    "crates/uml-xmi",         # Needed for Phase 4 (XMI round-trip)
    "crates/uml-undo",        # Needed for Phase 6 (undo/redo)
    "crates/uml-diagram",     # Needed for Phase 16 (diagram model)

    # Features (Tier 2) — framework crates, leaf plugins commented out
    "crates/uml-codegen",     # Keep trait + CodeWriter; move ProgrammingLanguage to uml-core
    "crates/uml-import",      # Keep trait + ImportRegistry skeleton
    "crates/uml-layout",      # Keep GridSnapper/AlignmentGuides skeleton
]
```

### Comment out (add back when needed)

```toml
# Phase 5 — add back when uml-xmi is functional
# "crates/uml-persistence",

# Phase 7 — add back when persistence is ready
# "apps/uml-cli",

# Phase 9 — add back when import framework is real
# "crates/uml-import-cpp",

# Phase 10
# "crates/uml-import-java",

# Phase 11
# "crates/uml-import-python",

# Phase 13 — add back when codegen framework is real
# "crates/uml-codegen-cpp",

# Phase 14
# "crates/uml-codegen-java",

# Phase 15
# "crates/uml-codegen-python",

# Phase 18 — add back when diagram model is solid
# "crates/uml-render",

# Phase 18+
# "crates/uml-export",

# Phase 20 — add back when GUI begins
# "apps/umbrello-desktop",

# Phase 22
# "crates/uml-codegen-rust",
```

### When to add each back

| Crate | Trigger |
|-------|---------|
| `uml-persistence` | When `uml-xmi::reader` can deserialize a model and `uml-xmi::writer` can serialize one. |
| `uml-cli` | When `uml-persistence` can load/save files. |
| `uml-import-cpp` | When `uml-import` has a working `ImportRegistry::run()` and we have C++ parser bindings. |
| `uml-import-java` | Same trigger, Java grammar. |
| `uml-import-python` | Same trigger, Python grammar. |
| `uml-codegen-cpp` | When `uml-codegen` has a working `GeneratorRegistry::generate()` and C++ language-specific options. |
| `uml-codegen-java` | Same trigger, Java. |
| `uml-codegen-python` | Same trigger, Python. |
| `uml-render` | When `uml-diagram::SceneData` has widgets with positions/sizes and a paint target exists. |
| `uml-export` | When `uml-render` can produce a raster or vector frame. |
| `umbrello-desktop` | When a Qt or egui integration prototype is scoped. |
| `uml-codegen-rust` | When all other codegen plugins are done. |

---

## 5. Additional Observation: `uml-persistence` Has Real Types but Wrong Home

The `uml-persistence` crate has three real pieces that are not stubs:

- `FileFormat` enum (4 variants)
- `PersistenceError` enum (2 variants)
- `StorageBackend` trait (path, module `storage`)

These are useful types, but the crate depends on `uml-xmi` (a stub) and should
not exist as a separate crate yet. Options:

**Option A (recommended):** Move `FileFormat` and `PersistenceError` into
`uml-common`, comment out `uml-persistence`. Re-create the crate in Phase 5
when `uml-xmi` is functional.

**Option B:** Keep `uml-persistence` in the workspace but make `uml-xmi` an
optional dependency. Not worth the `#[cfg(feature)]` complexity for what is
effectively 90 lines of code.

Since `uml-persistence` is in the "comment out" list above, this is not a
blocking issue — the types can move into `uml-common` if the `StorageBackend`
trait is needed by Phase 2/3 code, or they can simply wait in a branch.

---

## 6. Summary of Actions

| # | Action | Files affected |
|---|--------|----------------|
| 1 | Move `ProgrammingLanguage` enum from `uml-codegen` → `uml-core/src/types.rs` | `crates/uml-codegen/src/lib.rs`, `crates/uml-core/src/types.rs` |
| 2 | Replace enum definition with `pub use uml_core::types::ProgrammingLanguage;` | `crates/uml-codegen/src/lib.rs` |
| 3 | Comment out 12 premature crates in `Cargo.toml` | `Cargo.toml` |
| 4 | Keep 9 crates + xtask as workspace members | `Cargo.toml` |
| 5 | (Optional) Move `FileFormat` + `PersistenceError` to `uml-common` | `crates/uml-common/src/lib.rs` |

After these changes, the workspace has **10 members** instead of 22 — a
manageable set where every crate has either real code or a clear, near-term
implementation phase.
