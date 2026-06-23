# Umbrello-RS Testing Strategy

> **Document:** `rust-rewrite/docs/testing_strategy.md`
> **Status:** Active
> **Last updated:** 2026-06-23
>
> This document defines the complete testing approach for the Umbrello-RS project:
> philosophy, pyramid layers, infrastructure, patterns, and concrete test plans
> for each milestone.

---

## Table of Contents

1. [Testing Philosophy](#1-testing-philosophy)
2. [The Testing Pyramid](#2-the-testing-pyramid)
3. [Test Infrastructure](#3-test-infrastructure)
4. [Testing Patterns by Domain](#4-testing-patterns-by-domain)
5. [XMI Compatibility Testing](#5-xmi-compatibility-testing)
6. [CI Integration](#6-ci-integration)
7. [Test Coverage Targets](#7-test-coverage-targets)
8. [Immediate Next Steps](#8-immediate-next-steps)
9. [Crate-by-Crate Testing Plan](#9-crate-by-crate-testing-plan)

---

## 1. Testing Philosophy

### 1.1 Core Principles

**Tests are documentation.**
A test is the most honest form of documentation — it cannot lie. Every public API
surface must have tests that show typical usage, edge cases, and error paths. When
a developer reads a module's tests, they should understand what the module does,
how to use it, and what guarantees it provides.

**Tests enable fearless refactoring.**
Umbrello-RS is a rewrite of a 20+ year old codebase. The existing C++ code has
limited test coverage (13 Qt Test files covering a subset of functionality).
The Rust rewrite must achieve comprehensive coverage so that we can refactor,
optimize, and evolve the code with confidence.

**Every public API surface must be tested.**
If a function, type, or trait is `pub`, it must have tests. Private
implementation details may remain untested, but every external contract must be
verified. This applies to re-exports — if a type is re-exported from `lib.rs`,
its tests live in the defining module but must validate the public contract.

**XMI compatibility is the critical invariant.**
The most important property of Umbrello-RS is that it can read XMI files written
by C++ Umbrello and write XMI files that C++ Umbrello can read. This is the
single highest-risk area and receives the most rigorous testing (see
[Section 5](#5-xmi-compatibility-testing)).

**Tests must be fast.**
Developer experience depends on fast feedback. Tests are categorized by speed:

| Category | Max Time | Frequency | CI Stage |
|----------|----------|-----------|----------|
| Unit tests | < 1s (entire crate) | Every `cargo test` | Default |
| Property tests | < 5s | Every `cargo test` | Default |
| Integration tests | < 30s | Every `cargo test` | Default |
| Snapshot tests | < 10s | Every `cargo test` | Default |
| XMI compatibility | < 120s | CI + explicit | `--ignored` |
| Cross-tool verification | < 300s | CI nightly | `--ignored` |

### 1.2 What We Do NOT Test

- **Panic paths.** We use `no_panic` patterns (Result returns, not unwrap/expect).
  If code can panic, fix it — don't test for the panic.
- **External library correctness.** We test our usage of libraries, not the libraries
  themselves. Assume `uuid`, `serde`, `quick-xml` are correct.
- **Generated code correctness (in production).** We snapshot-test code generator
  output; we do not compile and run the generated code in CI (that's a cross-tool
  verification step, done on a best-effort basis).

### 1.3 Testing in the Build System

Tests are a first-class concern in the Cargo workspace:

```toml
# Each crate's Cargo.toml MUST include:
[dev-dependencies]
serde_json = "1"      # Round-trip test helper
pretty_assertions = "1"  # Readable assertion output

# For crates that need property-based testing:
proptest = "1"

# For crates that produce output (codegen, XMI, rendering):
insta = "1"
```

The workspace `.cargo/config.toml` maintains optimal compile times:

```toml
[profile.test]
opt-level = 0         # Faster compile, enable debug assertions
debug = 1             # Line tables for test failure backtraces
incremental = true
```

---

## 2. The Testing Pyramid

Umbrello-RS follows a five-layer testing pyramid. The percentages indicate
relative effort and test count, not absolute line coverage.

```
                    ╱╲
                   ╱  ╲
                  ╱ E2E ╲            ~2%
                 ╱════════╲
                ╱ Snapshot ╲         ~3%
               ╱════════════╲
              ╱ Integration  ╲       ~15%
             ╱════════════════╲
            ╱  Property-Based  ╲     ~10%
           ╱════════════════════╲
          ╱    Unit Tests        ╲   ~70%
         ╱════════════════════════╲
```

### Layer 1: Unit Tests (~70% of tests)

**Location:** `#[cfg(test)] mod tests { ... }` inline in every source module.

**Framework:** `cargo test` (built-in).

**Scope:**
- Individual functions and methods
- Type invariants (construct, access, mutate, destroy)
- Enum variant coverage (every variant tested)
- Error paths (invalid input → correct error)
- Edge cases (empty collections, boundary values, `None` values)

**Characteristics:**
- No I/O (no filesystem, no network, no environment variables)
- No async runtime (`#[tokio::test]` only where code actually awaits)
- Deterministic — no random test ordering dependencies
- Fast — entire crate's unit tests must complete in < 1s
- Stateless — each test creates its own data, no shared mutable state

**Examples:**
```rust
// crates/uml-core/src/id.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_produces_unique_ids() {
        let a = UmlId::new();
        let b = UmlId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn default_is_valid_and_unique() {
        let a = UmlId::default();
        let b = UmlId::default();
        assert_ne!(a, b);
    }

    #[test]
    fn display_produces_uuid_string() {
        let id = UmlId::new();
        let s = id.to_string();
        assert_eq!(s.len(), 36); // standard UUID format
        assert_eq!(s.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn serde_round_trip() {
        let id = UmlId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: UmlId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn clone_produces_equal_id() {
        let a = UmlId::new();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn ordering_is_consistent() {
        let a = UmlId::new();
        let b = UmlId::new();
        // UUIDs are random, so we can't predict order.
        // But we can check consistency: if a < b then a != b
        if a < b {
            assert_ne!(a, b);
        }
    }
}
```

### Layer 2: Property-Based Tests (~10% of tests)

**Location:** `proptest!` blocks in `#[cfg(test)] mod tests` or in a
`proptest-regressions/` directory for regression reproduction.

**Framework:** `proptest` 1.x.

**Scope:**
- Type invariants that hold across all valid inputs
- Round-trip properties (serialize → deserialize → assert_eq)
- Model mutation properties (undo(execute(x)) == original state)
- Collection invariants (no duplicates, no dangling references, all refs resolvable)
- String formatting properties (output matches regex, no panics)

**When to use:**
- Serde serialization for complex types with many fields
- XMI format round-tripping
- Any function that accepts a wide input domain
- Undo/redo command sequences
- Validation logic

**Examples:**
```rust
// crates/uml-core/src/types.rs
proptest! {
    /// Any ObjectType variant serializes and deserializes losslessly.
    #[test]
    fn object_type_serde_roundtrip(variant in prop::sample::any::<ObjectType>()) {
        let json = serde_json::to_string(&variant).unwrap();
        let deserialized: ObjectType = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(variant, deserialized);
    }
}

// crates/uml-core/src/id.rs
proptest! {
    /// Any UmlId round-trips through its Display/FromStr representation.
    #[test]
    fn umlid_display_roundtrip(id: UmlId) {
        let s = id.to_string();
        let parsed: UmlId = s.parse().unwrap();
        prop_assert_eq!(id, parsed);
    }
}
```

**Arbitrary implementations:**
Each crate provides `proptest::arbitrary::Arbitrary` for its key types:

```rust
// crates/uml-core/src/id.rs
#[cfg(any(test, feature = "proptest"))]
impl proptest::arbitrary::Arbitrary for UmlId {
    type Parameters = ();
    type Strategy = proptest::strategy::BoxedStrategy<Self>;

    fn arbitrary_with(_: Self::Parameters) -> Self::Strategy {
        proptest::strategy::Just(UmlId::new()).boxed()
    }
}
```

### Layer 3: Integration Tests (~15% of tests)

**Location:** `tests/` directory at crate or workspace level.

**Framework:** `cargo test` — each file in `tests/` compiles as a separate binary.

**Scope:**
- Cross-crate interactions (e.g., uml-xmi + uml-core)
- File I/O (read from `tests/fixtures/`, write to temp directory)
- End-to-end workflows within a crate boundary
- Multi-step scenarios (create → modify → serialize → deserialize → verify)

**Organization (per crate):**
```
crates/<crate>/
└── tests/
    ├── mod.rs                      # Test helpers shared across test files
    ├── fixtures/                   # Reference files (XMI, codegen output)
    │   ├── test-1.2.xmi
    │   ├── test-2.1.xmi
    │   └── ...
    └── xmi_roundtrip.rs            # Integration tests for XMI loading/saving
```

**Examples:**
```rust
// crates/uml-core/tests/model_construction.rs
use uml_core::model::{UmlClass, UmlPackage};
use uml_core::types::{ObjectType, Visibility};
use uml_core::id::UmlId;

#[test]
fn test_build_simple_class_model() {
    // Build programmatic model (no I/O)
    let cls = UmlClass::builder()
        .name("Person")
        .visibility(Visibility::Public)
        .build()
        .unwrap();
    assert_eq!(cls.name(), "Person");
    assert_eq!(cls.object_type(), ObjectType::Class);
}
```

### Layer 4: Snapshot Tests (~3% of tests)

**Location:** Inline in `#[cfg(test)] mod tests` using `insta::assert_snapshot!`.

**Framework:** `insta` 1.x with `cargo-insta` for review workflow.

**Scope:**
- Code generation output (`.h`, `.cpp`, `.java`, `.py` files)
- XMI output (full XML document or significant fragments)
- Rendering output (SVG strings, pixel buffers for visual regression)
- Error messages (user-visible error formatting)
- CLI help text and usage output

**Workflow:**
1. Write test with `insta::assert_snapshot!(output)`
2. Run `cargo test` — fails with "new snapshot not reviewed"
3. Run `cargo insta review` — accept/reject/update snapshots
4. Snapshots stored in `src/snapshots/` alongside module code
5. On intentional changes, `UPDATE_EXPECT=1 cargo test` or `cargo insta accept`

**Examples:**
```rust
// crates/uml-codegen-cpp/src/generator.rs
#[test]
fn test_generate_simple_class_header() {
    let model = test_model::simple_class();
    let output = CppCodeGenerator::new(Default::default())
        .generate_header(&model)
        .unwrap();
    insta::assert_snapshot!(output);
}
```

**When NOT to use snapshots:**
- Output that contains timestamps, UUIDs, or non-deterministic data (use
  `insta::with_settings!({description => "..."}, ...)` or redact instead)
- Output that changes frequently during development (prefer unit tests)
- Very large output (snapshot files become unmanageable)

### Layer 5: End-to-End Tests (~2% of tests)

**Location:** `tests/` in binary crates (`apps/uml-cli/tests/`, `apps/umbrello-desktop/tests/`).

**Framework:** `assert_cmd` for CLI, `cargo test` with process spawn for GUI smoke tests.

**Scope:**
- CLI invocation with various arguments
- Complete workflows (load XMI → export → verify output exists)
- Error handling for invalid inputs
- Exit codes and stdout/stderr output format

**Examples:**
```rust
// apps/uml-cli/tests/cli_e2e.rs
use assert_cmd::Command;

#[test]
fn test_cli_list_languages() {
    let mut cmd = Command::cargo_bin("uml-cli").unwrap();
    let assert = cmd.arg("--languages").assert();
    assert
        .success()
        .stdout(predicates::str::contains("C++"))
        .stdout(predicates::str::contains("Java"))
        .stdout(predicates::str::contains("Python"));
}

#[test]
fn test_cli_load_and_print_summary() {
    let xmi_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/uml-xmi/tests/fixtures/test-1.2.xmi"
    );
    let mut cmd = Command::cargo_bin("uml-cli").unwrap();
    let assert = cmd.arg(xmi_path).assert();
    assert
        .success()
        .stdout(predicates::str::contains("Model:"))
        .stdout(predicates::str::contains("Objects:"));
}

#[test]
fn test_cli_validate_valid_xmi() {
    let xmi_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../crates/uml-xmi/tests/fixtures/test-1.2.xmi"
    );
    let mut cmd = Command::cargo_bin("uml-cli").unwrap();
    cmd.arg("--validate").arg(xmi_path)
        .assert()
        .success();
}

#[test]
fn test_cli_nonexistent_file() {
    let mut cmd = Command::cargo_bin("uml-cli").unwrap();
    cmd.arg("/nonexistent/path/file.xmi")
        .assert()
        .failure()
        .stderr(predicates::str::contains("error"))
        .stderr(predicates::str::contains("No such file"));
}
```

---

## 3. Test Infrastructure

### 3.1 Test Organization for uml-core

The `uml-core` crate is the foundation and must achieve the highest coverage first.
Its test organization serves as the template for all other crates.

```
crates/uml-core/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── id.rs
│   │   └── #[cfg(test)] mod tests { ... }
│   ├── types.rs
│   │   └── #[cfg(test)] mod tests { ... }
│   ├── model.rs
│   │   └── (to be expanded)
│   ├── repository.rs
│   │   └── (to be expanded)
│   └── event.rs
│       └── (to be expanded)
└── tests/
    ├── model_builder.rs     # Test helper for constructing test models
    ├── xmi_roundtrip.rs     # Integration tests (Phase 4+)
    ├── fixtures/
    │   ├── test-1.2.xmi
    │   ├── test-2.1.xmi
    │   ├── test-associations.xmi
    │   ├── test-components.xmi
    │   ├── test-statemachine.xmi
    │   ├── test-usecase.xmi
    │   ├── test-entity.xmi
    │   ├── test-foreign-dialect.xmi
    │   └── test-roundtrip.xmi
    └── import/              # Code import test files (Phase 8+)
        ├── cpp/
        │   ├── simple_class.h
        │   └── ...
        ├── java/
        │   └── ...
        └── python/
            └── ...
```

### 3.2 Test Helper Module

Each crate with significant testing needs should have a `test_utils` module
(gated behind `#[cfg(test)]` or a `test-utils` feature flag). For `uml-core`:

```rust
// crates/uml-core/src/test_utils.rs
//! Test utilities for constructing UML models and asserting invariants.
//!
//! This module is only compiled in test builds. It provides builders,
//! matchers, and fixture constants used across all test files.

use crate::types::*;
use crate::id::UmlId;

/// Construct a simple test model: one package with one class.
///
/// Returns (package_id, class_id) after inserting into the repository.
pub fn simple_package_with_class(repo: &mut crate::repository::ModelRepository)
    -> (/*package_key*/, /*class_key*/)
{
    // ... builder pattern
    todo!("implement when ModelRepository has storage")
}

/// Assert that two model elements are deeply equal.
///
/// Unlike PartialEq, this provides detailed diagnostics on nested fields.
pub fn assert_model_eq(left: &crate::model::ModelElement, right: &crate::model::ModelElement) {
    assert_eq!(left, right, "Model elements not equal:\nleft:  {left:#?}\nright: {right:#?}");
}
```

**Rule for test helpers:**
- If shared across multiple test files → put in `src/test_utils.rs` with `#[cfg(test)]`
- If only used in one test file → put inline in that file
- If useful across crates → make it a public module behind a `test-utils` feature flag

### 3.3 Test Fixture Constants

For small reference data (XMI snippets, codegen output), use `include_str!()`
to embed them as constants:

```rust
// crates/uml-xmi/tests/xmi_roundtrip.rs
const TEST_XMI_1_2: &str = include_str!("fixtures/test-1.2.xmi");
const TEST_XMI_2_1: &str = include_str!("fixtures/test-2.1.xmi");
const TEST_ASSOCIATIONS: &str = include_str!("fixtures/test-associations.xmi");
```

For large files or binary data, use `include_bytes!()` and handle at runtime.

### 3.4 Workspace dev-dependencies

Standard dev-dependencies that should appear in every crate's `Cargo.toml`:

```toml
[dev-dependencies]
serde_json = "1"
pretty_assertions = "1"

# Conditionally add:
#   proptest = "1"          # For property-based tests
#   insta = "1"             # For snapshot tests
#   tempfile = "3"          # For tests that write temp files
#   assert_cmd = "2"        # For CLI E2E tests (only in binary crates)
#   predicates = "3"        # For predicate-based assertions with assert_cmd
```

**Current state (uml-core as of Milestone 1):**

```toml
# crates/uml-core/Cargo.toml
[dev-dependencies]
serde_json = "1"
# TODO: Add proptest = "1" in Phase 2 when property tests are written
# TODO: Add pretty_assertions = "1"
```

### 3.5 Test Configuration

Tests that require special setup or are slow use Rust's built-in test attributes:

```rust
/// Slow test that does XMI file I/O — excluded from default `cargo test`.
#[test]
#[ignore = "requires XMI test files in tests/fixtures/"]
fn test_roundtrip_large_xmi() {
    // ...
}

/// Test that requires the C++ Umbrello binary — only runs on CI.
#[test]
#[ignore = "requires C++ umbrello binary for cross-tool verification"]
fn test_cross_tool_roundtrip() {
    // ...
}
```

Environment variables for test configuration:

| Variable | Purpose | Default |
|----------|---------|---------|
| `UMBRELLO_TEST_XMI_DIR` | Override path to XMI test fixtures | `tests/fixtures/` relative to crate |
| `UMBRELLO_TEST_CPP_BIN` | Path to C++ umbrello binary for cross-tool tests | (unset = skip) |
| `UMBRELLO_TEST_UPDATE_SNAPSHOTS` | Force update codegen snapshots | `0` |
| `UMBRELLO_TEST_NUM_ITERATIONS` | Number of iterations for property tests | `256` |

---

## 4. Testing Patterns by Domain

| Domain | Primary Test Type | Key Invariants |
|--------|-------------------|----------------|
| `UmlId` | Unit + Property | generation uniqueness, XMI string round-trip |
| `ObjectType` | Unit | serde round-trip, Display impl, conversion |
| `AssociationType` | Unit | enum completeness, serde round-trip |
| `DiagramType` | Unit | default variant, serde round-trip |
| `Visibility` | Unit | default variant, UML symbol formatting |
| `ParameterDirection` | Unit | default variant, serde round-trip |
| Model types | Unit + Property | clone equality, serde round-trip |
| ModelRepository | Unit + Property | insert/get consistency, no leaks |
| XMI reader | Integration + Snapshot | byte-identical output with C++ |
| XMI writer | Integration + Snapshot | valid XML, valid XMI schema |
| Code generators | Snapshot | output matches golden files |
| Code importers | Integration | AST → UML mapping correctness |
| Undo/redo | Unit + Property | undo(execute(x)) == original state |
| Rendering | Snapshot | pixel comparison with reference images |
| CLI | E2E | exit codes, stdout/stderr |

### 4.1 Pattern: Serde Round-Trip Test

Every serializable type must have a JSON round-trip test:

```rust
#[test]
fn object_type_serde_roundtrip() {
    let variants = [
        ObjectType::Class,
        ObjectType::Interface,
        // ... all variants ...
    ];
    for variant in &variants {
        let json = serde_json::to_string(variant).unwrap();
        let deserialized: ObjectType = serde_json::from_str(&json).unwrap();
        assert_eq!(*variant, deserialized, "Failed round-trip for {variant:?}");
    }
}
```

### 4.2 Pattern: Default Invariant Test

Every type with `Default` must verify default has expected semantics:

```rust
#[test]
fn visibility_default_is_public() {
    assert_eq!(Visibility::default(), Visibility::Public);
}

#[test]
fn diagram_type_default_is_undefined() {
    assert_eq!(DiagramType::default(), DiagramType::Undefined);
}

#[test]
fn parameter_direction_default_is_in() {
    // When ParameterDirection exists:
    // assert_eq!(ParameterDirection::default(), ParameterDirection::In);
}
```

### 4.3 Pattern: Display Format Test

Every type with `Display` must verify the format matches expectations:

```rust
#[test]
fn visibility_display_symbols() {
    assert_eq!(Visibility::Public.to_string(), "+");
    assert_eq!(Visibility::Protected.to_string(), "#");
    assert_eq!(Visibility::Private.to_string(), "-");
    assert_eq!(Visibility::Implementation.to_string(), "~");
}
```

### 4.4 Pattern: Enum Exhaustiveness Test

Every enum must verify it has the expected number of variants and that each
variant is reachable:

```rust
#[test]
fn object_type_has_all_variants() {
    // Count variants via strum or manual enumeration
    let expected_count = 27; // Update when variants change
    let actual_count = ObjectType::VARIANTS.len(); // requires strum::EnumCount
    assert_eq!(actual_count, expected_count);
}

#[test]
fn object_type_variants_are_distinct() {
    let mut map = std::collections::HashSet::new();
    for variant in ObjectType::VARIANTS {
        assert!(map.insert(variant), "Duplicate variant: {variant:?}");
    }
}
```

### 4.5 Pattern: Uniqueness Invariant Test

For types that generate unique values (UmlId, ObjectKey):

```rust
#[test]
fn umlid_uniqueness_in_loop() {
    let mut set = std::collections::HashSet::new();
    for _ in 0..10_000 {
        let id = UmlId::new();
        assert!(set.insert(id), "Duplicate UmlId generated");
    }
}
```

### 4.6 Pattern: Round-Trip Integration Test

For I/O workflows (XMI load/save, save/load):

```rust
#[test]
fn xmi_roundtrip_creates_identical_model() {
    // Arrange: load known XMI
    let input = include_str!("fixtures/test-1.2.xmi");
    let mut reader = XmiReader::new(input.as_bytes()).unwrap();
    let model = reader.read_document().unwrap();

    // Act: write to buffer
    let mut buffer = Vec::new();
    XmiWriter::new(&mut buffer, XmiVersion::V1_2)
        .unwrap()
        .write_document(&model)
        .unwrap();

    // Assert: reload produces same model
    let mut reader2 = XmiReader::new(buffer.as_slice()).unwrap();
    let model2 = reader2.read_document().unwrap();
    assert_eq!(model, model2);
}
```

---

## 5. XMI Compatibility Testing

This is the highest-risk area. The entire rewrite is worthless if it cannot
faithfully read and write Umbrello XMI files.

### 5.1 Golden File Tests

**Setup:**
1. Copy all test XMI files from the C++ Umbrello `test/` directory into
   `crates/uml-xmi/tests/fixtures/`.
2. Each test file is loaded, verified for structural correctness, written back,
   and the output compared byte-for-byte with the original.

**Compatibility Matrix:**

| Test File | XMI Version | Dialect | Features Covered | Status |
|-----------|-------------|---------|------------------|--------|
| `test-1.2.xmi` | 1.2 | Native | Classes, attributes, operations | Planned |
| `test-2.1.xmi` | 2.1 | Native | packagedElement style | Planned |
| `test-associations.xmi` | 1.2 | Native | All association types (14) | Planned |
| `test-components.xmi` | 1.2 | Native | Component/deployment diagrams | Planned |
| `test-statemachine.xmi` | 1.2 | Native | State machine diagram | Planned |
| `test-usecase.xmi` | 1.2 | Native | Use case diagram | Planned |
| `test-entity.xmi` | 1.2 | Native | Entity relationship | Planned |
| `test-foreign-dialect.xmi` | 1.2 | NSUML | Foreign dialect | Planned |
| `test-roundtrip.xmi` | 1.2 | Native | Comprehensive model | Planned |
| `argo-example.zargo` | 1.2 | ArgoUML | ArgoUML import | Future |
| `rose-example.mdl` | 1.2 | Rational | Rational Rose import | Future |

**Test implementation:**

```rust
// crates/uml-xmi/tests/xmi_roundtrip.rs

/// Test that every golden XMI file can be loaded without error.
#[test]
#[ignore = "requires XMI test files in tests/fixtures/"]
fn test_load_all_golden_files() {
    let files = [
        "test-1.2.xmi",
        "test-2.1.xmi",
        "test-associations.xmi",
        "test-components.xmi",
        "test-statemachine.xmi",
        "test-usecase.xmi",
        "test-entity.xmi",
        "test-foreign-dialect.xmi",
        "test-roundtrip.xmi",
    ];
    for fname in &files {
        let path = format!("{}/tests/fixtures/{fname}", env!("CARGO_MANIFEST_DIR"));
        let input = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Failed to read {fname}: {e}"));

        let mut reader = XmiReader::new(input.as_bytes())
            .unwrap_or_else(|e| panic!("Failed to parse {fname}: {e}"));
        let model = reader.read_document()
            .unwrap_or_else(|e| panic!("Failed to build model from {fname}: {e}"));

        assert!(
            model.object_count() > 0,
            "Empty model loaded from {fname}"
        );
    }
}

/// Test round-trip: load → save → load → compare model equality.
#[test]
#[ignore = "requires XMI test files in tests/fixtures/"]
fn test_xmi_roundtrip_structural() {
    let files = [
        "test-1.2.xmi",
        "test-associations.xmi",
        "test-roundtrip.xmi",
    ];
    for fname in &files {
        let path = format!("{}/tests/fixtures/{fname}", env!("CARGO_MANIFEST_DIR"));
        let input = std::fs::read_to_string(&path).unwrap();

        // Load
        let mut reader = XmiReader::new(input.as_bytes()).unwrap();
        let model = reader.read_document().unwrap();

        // Save to buffer
        let mut buffer = Vec::new();
        XmiWriter::new(&mut buffer, XmiVersion::V1_2)
            .unwrap()
            .write_document(&model)
            .unwrap();

        // Reload
        let mut reader2 = XmiReader::new(buffer.as_slice()).unwrap();
        let model2 = reader2.read_document().unwrap();

        // Compare structural identity (not byte-identical — XML whitespace may differ)
        assert_eq!(
            model.object_count(),
            model2.object_count(),
            "Object count mismatch after round-trip of {fname}"
        );
        assert_eq!(model, model2, "Model mismatch after round-trip of {fname}");
    }
}
```

### 5.2 Round-Trip Property Test

Property-based round-trip with generated models:

```rust
proptest! {
    /// For any valid UML model, serializing to XMI and deserializing
    /// produces an equal model.
    #[test]
    fn any_model_roundtrips_xmi(model in arb_uml_model()) {
        // Serialize
        let mut buffer = Vec::new();
        XmiWriter::new(&mut buffer, XmiVersion::V2_1)
            .unwrap()
            .write_document(&model)
            .unwrap();

        // Deserialize
        let mut reader = XmiReader::new(buffer.as_slice()).unwrap();
        let reloaded = reader.read_document().unwrap();

        prop_assert_eq!(model, reloaded);
    }
}

/// Generate arbitrary UML models for property testing.
fn arb_uml_model() -> impl proptest::strategy::Strategy<Value = UmlModelDocument> {
    // Strategy: generate 1-10 model elements with random types/names/visibilities
    proptest::collection::vec(arb_model_element(), 1..10)
        .prop_map(|elements| {
            let mut model = UmlModelDocument::default();
            // ... populate model from elements
            model
        })
}
```

### 5.3 Cross-Tool Verification

These tests verify that Rust output is compatible with C++ Umbrello:

```rust
/// Load a golden XMI file in Rust, save it, then load the saved file
/// in C++ Umbrello and verify no errors.
///
/// Requires UMBRELLO_TEST_CPP_BIN environment variable.
#[test]
#[ignore = "requires C++ umbrello binary for cross-tool verification"]
fn test_rust_output_loadable_in_cpp() {
    let cpp_bin = std::env::var("UMBRELLO_TEST_CPP_BIN")
        .expect("UMBRELLO_TEST_CPP_BIN must be set");

    let input = include_str!("fixtures/test-1.2.xmi");

    // Load in Rust
    let mut reader = XmiReader::new(input.as_bytes()).unwrap();
    let model = reader.read_document().unwrap();

    // Save from Rust
    let mut rust_output = Vec::new();
    XmiWriter::new(&mut rust_output, XmiVersion::V1_2)
        .unwrap()
        .write_document(&model)
        .unwrap();

    // Save to temp file
    let tmp = tempfile::NamedTempFile::new().unwrap();
    std::fs::write(tmp.path(), &rust_output).unwrap();

    // Load in C++ Umbrello (export to verify, or check exit code)
    let output = std::process::Command::new(&cpp_bin)
        .arg(tmp.path())
        .arg("--validate")
        .output()
        .expect("Failed to run C++ Umbrello");
    assert!(
        output.status.success(),
        "C++ Umbrello failed to load Rust output:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
```

### 5.4 XMI Schema Validation

Every XMI test file (and every XMI output) should be validated against the
XMI DTD or schema. This is done as an xtask command and in CI:

```bash
# Manual check
cargo xtask check-xmi tests/fixtures/test-1.2.xmi

# CI: validate all fixtures
cargo xtask check-xmi-all
```

The `check-xmi` xtask command should:
1. Parse the XMI with quick-xml (structural validity)
2. Verify the XMI version declaration
3. Check that all `xmi:id` attributes are unique
4. Verify that `xmi:idref` values point to existing `xmi:id` values
5. Check for required elements (Model, UML:Namespace.ownedElement, etc.)

---

## 6. CI Integration

### 6.1 CI Test Stages

The CI pipeline has five test stages, each with a specific scope and expected
duration:

| Stage | Command | Scope | Max Time | Run On |
|-------|---------|-------|----------|--------|
| `fmt` | `cargo fmt --all --check` | Formatting | 30s | Every push |
| `clippy` | `cargo clippy --workspace --all-targets -- -D warnings` | Lints | 3min | Every push |
| `test` | `cargo test --workspace` | Unit + Integration + Snapshot | 10min | Every push |
| `test-slow` | `cargo test --workspace -- --ignored` | XMI round-trips, slow tests | 15min | PR merge, nightly |
| `compat` | `cargo test --workspace --test xmi_compat -- --include-ignored` | Cross-tool | 30min | Nightly |

### 6.2 CI Workflow

```yaml
# .github/workflows/ci.yml (test section)

jobs:
  fast-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: swatinem/rust-cache@v2
      - run: cargo test --workspace
        name: Fast tests (unit + integration + snapshot)
      - run: cargo test --workspace --doc
        name: Doc tests

  slow-tests:
    needs: fast-tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: swatinem/rust-cache@v2
      - run: cargo test --workspace -- --ignored
        name: Slow tests (XMI round-trips)

  xmi-validation:
    needs: fast-tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: swatinem/rust-cache@v2
      - run: cargo xtask check-xmi-all
        name: Validate all XMI test files

  coverage:
    needs: fast-tests
    if: github.event_name == 'pull_request'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo install cargo-tarpaulin
      - run: cargo tarpaulin --workspace --out xml
        name: Generate coverage report
      - uses: codecov/codecov-action@v4
        with:
          file: cobertura.xml
```

### 6.3 Local Test Automation

Developers should use `cargo xtask` for local CI simulation:

```bash
# Run all tests that CI runs
cargo xtask test

# Run a specific test category
cargo test --workspace                                # fast tests
cargo test --workspace -- --ignored                    # slow tests
cargo test --workspace --test xmi_compat               # XMI compat tests

# Run tests for a specific crate
cargo test --package uml-core
cargo test --package uml-xmi -- --ignored

# Watch mode for TDD
cargo watch -x "test --package uml-core"

# Coverage
cargo tarpaulin --package uml-core --out html
```

### 6.4 Per-Crate Test Script

Each crate should pass this checklist before being considered "tested":

```bash
cd crates/<crate>
cargo test                                     # All unit + integration tests pass
cargo test -- --ignored                         # Slow tests pass (if any)
cargo clippy -- -D warnings                     # No warnings
cargo fmt --check                               # Properly formatted
cargo doc --no-deps                             # Doc builds without errors
```

### 6.5 Coverage in CI

Coverage is generated on PRs and reported to Codecov. The CI configuration:

```yaml
coverage:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: llvm-tools-preview
    - uses: Swatinem/rust-cache@v2
    - run: cargo install cargo-llvm-cov
    - run: cargo llvm-cov --workspace --lcov --output-path lcov.info
    - uses: codecov/codecov-action@v4
      with:
        file: lcov.info
```

Coverage thresholds are enforced via `codecov.yml` at the workspace root:

```yaml
# codecov.yml
coverage:
  status:
    project:
      default:
        target: auto
        threshold: 1%
      uml-common:
        target: 100%
      uml-core:
        target: 95%
      uml-xmi:
        target: 90%
      uml-persistence:
        target: 85%
      uml-undo:
        target: 95%
      uml-diagram:
        target: 85%
      uml-codegen:
        target: 80%
      uml-import:
        target: 80%
      codegen-*:
        target: 80%
      import-*:
        target: 80%
      uml-render:
        target: 70%
      uml-layout:
        target: 85%
      uml-export:
        target: 75%
      uml-cli:
        target: 75%
      umbrello-desktop:
        target: 60%
```

---

## 7. Test Coverage Targets

### 7.1 Line and Branch Coverage by Crate

| Crate | Line Coverage | Branch Coverage | Rationale |
|-------|---------------|-----------------|-----------|
| `uml-common` | 100% | 95%+ | Foundation — errors, versions, logging. Small, trivial to cover fully. |
| `uml-core` | 95%+ | 90%+ | Domain model — all types, enums, repository. Critical for correctness. |
| `uml-xmi` | 90%+ | 85%+ | XMI serialization — high risk but well-defined I/O with clear paths. |
| `uml-persistence` | 85%+ | 80%+ | File I/O — many valid paths, some error paths hard to simulate. |
| `uml-undo` | 95%+ | 90%+ | Command pattern — deterministic, all paths testable. |
| `uml-diagram` | 85%+ | 80%+ | Diagram model — data structures similar to uml-core. |
| `uml-codegen` | 80%+ | 75%+ | Framework trait — some branches are registry edge cases. |
| `uml-import` | 80%+ | 75%+ | Framework trait — import error paths vary by language. |
| `uml-codegen-*` | 80%+ | 75%+ | Generated output — format strings, many edge cases. |
| `uml-import-*` | 80%+ | 75%+ | AST parsing — coverage limited by tree-sitter grammar coverage. |
| `uml-layout` | 85%+ | 80%+ | Algorithms — deterministic with clear edge cases. |
| `uml-render` | 70%+ | 65%+ | Rendering — pixel-level code, some branches hard to unit test. |
| `uml-export` | 75%+ | 70%+ | Export pipeline — multistep, some error recovery paths. |
| `uml-cli` | 75%+ | 70%+ | CLI — argument parsing well-covered, some error paths. |
| `umbrello-desktop` | 60%+ | N/A | GUI — event loop, widget code. Snapshot tests, not coverage-driven. |

### 7.2 Coverage Exemptions

Some code is excluded from coverage requirements:

```rust
// E1: Test-only code
#[cfg(test)]
mod tests { /* excluded from coverage */ }

// E2: Dead code warnings (unreachable arms)
#[allow(unreachable_patterns)]
fn handle_impossible() { /* ... */ }

// E3: Panic paths (should not exist in production code)
// If you write unwrap()/expect(), the test that covers the failure path
// is the test for the logic that prevents the panic — not the panic itself.
```

Coverage is measured with `#[no_coverage]` sparingly — only for items that
are genuinely untestable (e.g., calls to `std::process::exit()`).

### 7.3 Quality Gates

| Gate | Threshold | Enforcement |
|------|-----------|-------------|
| New code must have tests | 100% of new `pub` items | Code review |
| Line coverage per crate | See table above | CI + Codecov status check |
| Branch coverage per crate | See table above | CI + Codecov status check |
| No untested error paths | All `Result` types have failure tests | Code review + coverage |
| No dead test code | All test functions exercised | `cargo test --no-run` + grep for `#[test]` |
| Snapshot review | All snapshots reviewed | `cargo insta review` in CI |

---

## 8. Immediate Next Steps

### 8.1 Milestone 2: Test Implementation Plan

For the types already implemented in Milestone 1 (or being implemented in
Milestone 2: `UmlId`, `ObjectType`, `AssociationType`, `DiagramType`,
`Visibility`, `ParameterDirection`), the following tests are required:

#### UmlId Tests (8 tests)

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 1 | `new_produces_unique_ids` | Unit | Two consecutive `new()` calls produce different IDs |
| 2 | `default_is_valid` | Unit | `default()` produces a valid, unique ID |
| 3 | `display_produces_uuid_string` | Unit | `Display` outputs a 36-char UUID string |
| 4 | `serde_round_trip` | Unit | JSON serialize → deserialize preserves value |
| 5 | `clone_produces_equal_id` | Unit | `Clone` produces an ID equal to the original |
| 6 | `equality_and_hash_consistency` | Unit | `Eq` + `Hash` are consistent (same ID in HashSet found) |
| 7 | `ordering_is_consistent` | Unit | `Ord` is transitive |
| 8 | `umlid_uniqueness_under_iteration` | Property | No duplicate IDs in 10,000 consecutive `new()` calls |

**Implementation:**

```rust
// crates/uml-core/src/id.rs

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn new_produces_unique_ids() {
        let a = UmlId::new();
        let b = UmlId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn default_produces_valid_id() {
        let id = UmlId::default();
        let s = id.to_string();
        assert!(!s.is_empty());
    }

    #[test]
    fn default_ids_are_unique() {
        let a = UmlId::default();
        let b = UmlId::default();
        assert_ne!(a, b);
    }

    #[test]
    fn display_produces_uuid_string() {
        let id = UmlId::new();
        let s = id.to_string();
        // UUID format: xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
        assert_eq!(s.len(), 36);
        assert_eq!(s.chars().filter(|&c| c == '-').count(), 4);
        // All hex chars plus hyphens
        assert!(s.chars().all(|c| c.is_ascii_hexdigit() || c == '-'));
    }

    #[test]
    fn serde_round_trip() {
        let id = UmlId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: UmlId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn clone_produces_equal_id() {
        let a = UmlId::new();
        let b = a;
        assert_eq!(a, b);
    }

    #[test]
    fn equality_and_hash_consistency() {
        let a = UmlId::new();
        let b = a;
        let mut set = HashSet::new();
        assert!(set.insert(a));
        assert!(!set.insert(b)); // b is duplicate of a
        assert!(set.contains(&a));
        assert!(set.contains(&b));
    }

    #[test]
    fn ordering_is_consistent() {
        let a = UmlId::new();
        let b = UmlId::new();
        // Ord is derived from the UUID; just check consistency
        assert_eq!(a.cmp(&a), std::cmp::Ordering::Equal);
        assert_eq!(b.cmp(&b), std::cmp::Ordering::Equal);
        assert!(a.cmp(&b) == b.cmp(&a).reverse());
    }

    #[test]
    fn umlid_uniqueness_under_iteration() {
        let mut set = HashSet::new();
        for _ in 0..10_000 {
            let id = UmlId::new();
            assert!(
                set.insert(id),
                "Duplicate UmlId generated — UUID collision (extremely unlikely)"
            );
        }
        assert_eq!(set.len(), 10_000);
    }

    proptest! {
        /// Any UmlId string representation round-trips through serde.
        #[test]
        fn umlid_serde_roundtrip_prop(id: UmlId) {
            let json = serde_json::to_string(&id).unwrap();
            let deserialized: UmlId = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(id, deserialized);
        }
    }
}
```

#### ObjectType Tests (5 tests)

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 1 | `serde_round_trip` | Unit | Every variant JSON round-trips |
| 2 | `display_impl_non_empty` | Unit | `Display` outputs non-empty for every variant |
| 3 | `debug_impl_non_empty` | Unit | `Debug` outputs non-empty for every variant |
| 4 | `all_variants_covered` | Unit | All expected variants exist (count check) |
| 5 | `variant_conversion_consistency` | Property | `Serialize` produces same string for same variant |

**Implementation (to be added to types.rs):**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: return all ObjectType variants as a slice.
    fn all_object_types() -> &'static [ObjectType] {
        &[
            ObjectType::Class,
            ObjectType::Interface,
            ObjectType::Enumeration,
            ObjectType::Datatype,
            ObjectType::Entity,
            ObjectType::Package,
            ObjectType::Folder,
            ObjectType::Component,
            ObjectType::Artifact,
            ObjectType::Actor,
            ObjectType::UseCase,
            ObjectType::Node,
            ObjectType::Port,
            ObjectType::Category,
            ObjectType::Instance,
            ObjectType::Attribute,
            ObjectType::Operation,
            ObjectType::Template,
            ObjectType::EnumLiteral,
            ObjectType::EntityAttribute,
            ObjectType::UniqueConstraint,
            ObjectType::ForeignKeyConstraint,
            ObjectType::CheckConstraint,
            ObjectType::Association,
            ObjectType::Role,
            ObjectType::Stereotype,
            ObjectType::InstanceAttribute,
        ]
    }

    #[test]
    fn serde_round_trip() {
        for variant in all_object_types() {
            let json = serde_json::to_string(variant).unwrap();
            let deserialized: ObjectType = serde_json::from_str(&json).unwrap();
            assert_eq!(
                *variant,
                deserialized,
                "Round-trip failed for ObjectType::{variant:?}"
            );
        }
    }

    #[test]
    fn display_impl_non_empty() {
        for variant in all_object_types() {
            let s = variant.to_string();
            assert!(
                !s.is_empty(),
                "Display produced empty string for ObjectType::{variant:?}"
            );
        }
    }

    #[test]
    fn debug_impl_non_empty() {
        for variant in all_object_types() {
            let s = format!("{variant:?}");
            assert!(
                !s.is_empty(),
                "Debug produced empty string for ObjectType::{variant:?}"
            );
        }
    }

    #[test]
    fn all_variants_covered() {
        assert_eq!(all_object_types().len(), 27);
    }

    #[test]
    fn serde_uses_consistent_names() {
        // Verify that serde rename rules produce expected JSON output.
        // Lowercase (serde(rename_all = "lowercase")) is assumed.
        for variant in all_object_types() {
            let json = serde_json::to_value(variant).unwrap();
            let s = json.as_str().unwrap();
            // All lowercase, no underscores
            assert!(
                s.chars().all(|c| c.is_ascii_lowercase()),
                "ObjectType::{variant:?} serialized as '{s}' which is not all lowercase"
            );
        }
    }

    proptest! {
        #[test]
        fn object_type_serde_roundtrip_prop(variant in prop::sample::any::<ObjectType>()) {
            let json = serde_json::to_string(&variant).unwrap();
            let deserialized: ObjectType = serde_json::from_str(&json).unwrap();
            prop_assert_eq!(variant, deserialized);
        }
    }
}
```

#### DiagramType Tests (4 tests)

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 1 | `default_is_undefined` | Unit | Default variant is `Undefined` |
| 2 | `serde_round_trip` | Unit | Every variant JSON round-trips |
| 3 | `display_impl_non_empty` | Unit | Display is non-empty for every variant |
| 4 | `all_variants_accounted` | Unit | Count check (10 variants + Undefined) |

**Implementation:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn all_diagram_types() -> &'static [DiagramType] {
        &[
            DiagramType::Undefined,
            DiagramType::Class,
            DiagramType::UseCase,
            DiagramType::Sequence,
            DiagramType::Collaboration,
            DiagramType::State,
            DiagramType::Activity,
            DiagramType::Component,
            DiagramType::Deployment,
            DiagramType::EntityRelationship,
            DiagramType::Object,
        ]
    }

    #[test]
    fn default_is_undefined() {
        assert_eq!(DiagramType::default(), DiagramType::Undefined);
    }

    #[test]
    fn serde_round_trip() {
        for variant in all_diagram_types() {
            let json = serde_json::to_string(variant).unwrap();
            let deserialized: DiagramType = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, deserialized, "Round-trip failed for {variant:?}");
        }
    }

    #[test]
    fn all_variants_accounted() {
        assert_eq!(all_diagram_types().len(), 11); // 10 named + Undefined
    }
}
```

#### Visibility Tests (5 tests)

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 1 | `default_is_public` | Unit | Default visibility is `Public` |
| 2 | `display_uml_symbols` | Unit | `+`, `#`, `-`, `~` for each variant |
| 3 | `serde_round_trip` | Unit | Every variant JSON round-trips |
| 4 | `debug_impl_readable` | Unit | Debug shows variant name |
| 5 | `from_str_and_display_roundtrip` | Unit | Parsing Display output recovers variant |

**Implementation:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_public() {
        assert_eq!(Visibility::default(), Visibility::Public);
    }

    #[test]
    fn display_uml_symbols() {
        assert_eq!(Visibility::Public.to_string(), "+");
        assert_eq!(Visibility::Protected.to_string(), "#");
        assert_eq!(Visibility::Private.to_string(), "-");
        assert_eq!(Visibility::Implementation.to_string(), "~");
    }

    #[test]
    fn serde_round_trip() {
        let variants = [
            Visibility::Public,
            Visibility::Protected,
            Visibility::Private,
            Visibility::Implementation,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).unwrap();
            let deserialized: Visibility = serde_json::from_str(&json).unwrap();
            assert_eq!(*variant, deserialized, "Round-trip failed for {variant:?}");
        }
    }
}
```

#### ParameterDirection Tests (4 tests)

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 1 | `default_is_in` | Unit | Default direction is `In` |
| 2 | `serde_round_trip` | Unit | Every variant JSON round-trips |
| 3 | `display_non_empty` | Unit | Every variant has a non-empty Display |
| 4 | `all_variants_distinct` | Unit | All four variants are distinct values |

(Implementation follows the same patterns as above.)

#### AssociationType Tests (to be defined when implemented)

| # | Test Name | Type | Description |
|---|-----------|------|-------------|
| 1 | `serde_round_trip` | Unit | Every variant JSON round-trips |
| 2 | `default_is_association` | Unit | Default is `Association` (if applicable) |
| 3 | `all_variants_accounted` | Unit | Count check (all C++ variants covered) |
| 4 | `display_matches_uml` | Unit | Display output matches UML notation |
| 5 | `from_xmi_string` | Unit | Can be constructed from XMI string identifiers |

### 8.2 Test File Creation Order

When implementing Phase 2+ features, tests must be written in this order:

1. **Type system tests** (enums, UmlId, value types)
2. **Model data tests** (struct fields, builder, constructors)
3. **Model mutation tests** (events, validation, change application)
4. **Serialization tests** (XMI read/write, JSON round-trip)
5. **Repository tests** (insert/get/remove, iteration, hierarchy)
6. **File I/O tests** (load/save, compression, atomic save)
7. **Undo/redo tests** (command execution, stack, macro grouping)
8. **Code generation tests** (output snapshots)
9. **Code import tests** (AST parsing, model construction)
10. **CLI E2E tests** (argument parsing, workflows)
11. **GUI smoke tests** (loading, rendering, interaction)

### 8.3 GitHub Issue Templates for Test Gaps

When a bug is found in production code, follow this procedure:

1. **Write a failing test** that reproduces the bug
2. **Fix the code** to make the test pass
3. **Add property tests** if the bug suggests a class of related issues
4. **Document the fix** in the test name and comments

---

## 9. Crate-by-Crate Testing Plan

### 9.1 uml-common (Tier 0)

**Current state:** Milestone 1 complete. 3 source files.

**Source files and test requirements:**

| File | Tests Required | Priority | Status |
|------|---------------|----------|--------|
| `error.rs` | All error variants display correctly. `From` impls work. `Send + Sync` satisfied. | High | Planned |
| `version.rs` | Constants have expected values. Constants are non-empty. | Medium | Planned |

**Test count estimate:** 4 unit tests.

### 9.2 uml-core (Tier 0.5)

**Current state:** Milestone 1 complete. 6 source files.

**Source files and test requirements:**

| File | Tests Required | Priority | Status |
|------|---------------|----------|--------|
| `id.rs` | 8 unit + 1 property (see §8.1) | High | Planned |
| `types.rs` | ObjectType (5), DiagramType (4), Visibility (5), ParameterDirection (4) | High | Planned |
| `event.rs` | All 4 event variants constructable. Debug produces output. | Medium | Planned |
| `model.rs` | Placeholder — add tests when model types are implemented. | High (future) | N/A |
| `repository.rs` | Placeholder — add tests when storage is implemented. | High (future) | N/A |

**Test file requirements in `tests/`:**

| File | Tests | Priority |
|------|-------|----------|
| `model_builder.rs` | Test helpers for construction | High |
| `type_metadata.rs` | Cross-type invariants (e.g., no duplicate string representations) | Medium |

**Test count estimate:** 30+ unit tests, 3 property tests, 2 integration test files.

### 9.3 uml-xmi (Tier 1)

**Current state:** Empty crate skeleton.

**Source files and test requirements:**

| File | Tests Required | Priority |
|------|---------------|----------|
| `reader.rs` | Load valid XMI 1.2, load valid XMI 2.1, error on invalid XML, error on unknown dialect | High |
| `writer.rs` | Write XMI 1.2, write XMI 2.1, snapshot output, round-trip | High |
| `v1_2.rs` | All element types serialize/deserialize correctly | High |
| `v2_1.rs` | All element types serialize/deserialize correctly | High |
| `error.rs` | All error variants display correctly | Medium |
| `registry.rs` | Forward ref resolution, duplicate ID detection | High |
| `dialect.rs` | Dialect detection from root tag and attributes | Medium |

**Test files in `tests/`:**

| File | Tests | Priority |
|------|-------|----------|
| `xmi_roundtrip.rs` | Load all golden files, save/load round-trip | High |
| `xmi_schema.rs` | Validate XMI output against DTD/schema | High |
| `xmi_dialects.rs` | Test foreign dialect detection and parsing | Medium |

**Test count estimate:** 50+ unit tests, 10 integration tests, 3 test files.

### 9.4 uml-undo (Tier 1)

**Current state:** Empty crate skeleton.

**Source files and test requirements:**

| File | Tests Required | Priority |
|------|---------------|----------|
| `command.rs` | Trait implementable, description works, merge logic | High |
| `stack.rs` | push/undo/redo sequence, macro grouping, clean tracking, max size | High |
| `commands/*.rs` | Each command type: execute applies change, undo reverses it | High |

**Test files in `tests/`:**

| File | Tests | Priority |
|------|-------|----------|
| `undo_scenarios.rs` | Complex undo/redo sequences | High |
| `macro_commands.rs` | Macro grouping and atomicity | Medium |

**Key property test:**
```rust
proptest! {
    fn undo_reverses_execute(seq in arb_command_sequence()) {
        let mut model = ObjectRepository::new();
        let mut stack = UndoStack::new();
        let snapshot = model.clone();
        // Execute all commands
        for cmd in &seq {
            stack.push(cmd.clone(), &mut model).unwrap();
        }
        // Undo all commands
        while stack.can_undo() {
            stack.undo(&mut model).unwrap();
        }
        prop_assert_eq!(snapshot, model);
    }
}
```

**Test count estimate:** 30+ unit tests, 1 property test, 2 test files.

### 9.5 uml-persistence (Tier 2)

**Current state:** Empty crate skeleton.

**Source files and test requirements:**

| File | Tests Required | Priority |
|------|---------------|----------|
| `backend.rs` | Extensions list correct, can_handle matches extensions | High |
| `xmi_storage.rs` | Save XMI, load XMI, round-trip | High |
| `compression.rs` | Gzip/bzip2/zip save and load | Medium |
| `error.rs` | Error display | Medium |

**Test files in `tests/`:**

| File | Tests | Priority |
|------|-------|----------|
| `save_load_roundtrip.rs` | Save model → load → compare | High |
| `atomic_save.rs` | Crash recovery, temp file cleanup | Medium |
| `autosave.rs` | Interval, version management | Low |

**Test count estimate:** 20+ unit tests, 3 integration test files.

### 9.6 uml-codegen (Tier 1) + Language Plugins

**Test approach (all codegen crates follow same pattern):**

```rust
// crates/uml-codegen-cpp/tests/generated_output.rs

fn build_test_model() -> ObjectRepository {
    let mut repo = ObjectRepository::new();
    let person = ClassBuilder::new()
        .name("Person")
        .visibility(Visibility::Public)
        .add_attribute(AttributeBuilder::new()
            .name("name").type_name("std::string").visibility(Visibility::Private))
        .add_attribute(AttributeBuilder::new()
            .name("age").type_name("int").visibility(Visibility::Private))
        .add_operation(OperationBuilder::new()
            .name("getName").return_type("std::string").visibility(Visibility::Public))
        .build().unwrap();
    repo.insert(ModelElement::Class(Box::new(person)));
    repo
}

#[test]
fn test_generate_simple_class() {
    let repo = build_test_model();
    let config = CppConfig::default();
    let generator = CppCodeGenerator::new(config);
    let files = generator.generate(&repo, &GenerationConfig::default()).unwrap();

    // Should produce exactly 2 files (.h and .cpp)
    assert_eq!(files.len(), 2);

    // Header file
    let header = files.iter().find(|f| f.path.ends_with(".h")).unwrap();
    insta::assert_snapshot!("simple_class_header", &header.content);

    // Source file
    let source = files.iter().find(|f| f.path.ends_with(".cpp")).unwrap();
    insta::assert_snapshot!("simple_class_source", &source.content);
}
```

**Test count estimate per language plugin:** 15+ snapshot tests.

### 9.7 uml-import (Tier 1) + Language Plugins

**Test approach:**

```rust
// crates/uml-import-cpp/tests/import_class.rs

const SIMPLE_CLASS: &str = include_str!("fixtures/cpp/simple_class.h");

#[test]
fn test_import_simple_class() {
    let mut repo = ObjectRepository::new();
    let mut importer = CppImporter::new(Default::default()).unwrap();
    let changes = importer.import_file(&Path::new("simple_class.h"), SIMPLE_CLASS).unwrap();
    let mut undo = UndoStack::new();

    for change in changes {
        undo.push(Box::new(change), &mut repo).unwrap();
    }

    // Verify class was created
    let classes: Vec<_> = repo.iter_by_type(ObjectType::Class).collect();
    assert_eq!(classes.len(), 1);
    let (_, cls) = classes[0];
    assert_eq!(cls.name(), "Person");

    // Verify attribute
    let attrs: Vec<_> = repo.iter_by_type(ObjectType::Attribute).collect();
    assert_eq!(attrs.len(), 2); // name, age
}
```

**Test count estimate per language plugin:** 20+ integration tests.

### 9.8 uml-cli (Tier 3)

**Test approach:**

```rust
// apps/uml-cli/tests/cli_e2e.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_succeeds() {
    Command::cargo_bin("uml-cli")
        .unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"))
        .stdout(predicate::str::contains("Options"));
}

#[test]
fn test_languages() {
    Command::cargo_bin("uml-cli")
        .unwrap()
        .arg("--languages")
        .assert()
        .success()
        .stdout(predicate::str::contains("C++"))
        .stdout(predicate::str::contains("Java"))
        .stdout(predicate::str::contains("Python"));
}

#[test]
fn test_export_formats() {
    Command::cargo_bin("uml-cli")
        .unwrap()
        .arg("--export-formats")
        .assert()
        .success();
}

#[test]
fn test_nonexistent_file() {
    Command::cargo_bin("uml-cli")
        .unwrap()
        .arg("/nonexistent/model.xmi")
        .assert()
        .failure()
        .stderr(predicate::str::contains("error"))
        .code(predicate::eq(1));
}
```

**Test count estimate:** 12+ E2E tests.

### 9.9 umbrello-desktop (Tier 3)

**Test approach:**

GUI testing is limited. Focus on:
1. **Smoke tests** — app starts and renders
2. **Snapshot tests** — rendering output compared to reference images
3. **Unit tests** — panel logic, dialog validation, event handling

```rust
// apps/umbrello-desktop/tests/smoke.rs

#[test]
#[ignore = "requires display server"]
fn test_app_starts_and_creates_window() {
    // Use headless rendering or virtual framebuffer
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    // Smoke test: app can be initialized without panicking
    let app = UmbrelloApp::new(Default::default());
    // Verify initial state
    assert_eq!(app.diagrams().len(), 1); // "untitled" diagram created
    assert!(!app.model().is_empty()); // default model exists
}
```

**Test count estimate:** 5+ smoke tests, 20+ unit tests for panel logic.

---

## Appendix A: Test Template for New Crates

When creating a new crate, use this template for the initial test setup:

```toml
# crates/<name>/Cargo.toml
[dev-dependencies]
serde_json = "1"
pretty_assertions = "1"

# For property-based testing:
proptest = "1"

# For snapshot testing:
insta = "1"

# For temp files:
tempfile = "3"
```

```rust
// crates/<name>/src/lib.rs
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
```

```rust
// crates/<name>/src/<module>.rs

/// Public function that does something.
pub fn example_function(input: &str) -> Result<String, ExampleError> {
    // ...
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_example_function_success() {
        let result = example_function("valid input").unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_example_function_error() {
        let result = example_function("");
        assert!(result.is_err());
        match result {
            Err(ExampleError::EmptyInput) => {} // expected
            _ => panic!("Unexpected error variant"),
        }
    }

    proptest! {
        #[test]
        fn test_example_function_no_panic(input in ".*") {
            // Should never panic, even for random input
            let _ = example_function(&input);
        }
    }
}
```

```rust
// crates/<name>/tests/mod.rs
// Test helpers shared across integration test files.

/// Create a default test configuration.
pub fn test_config() -> crate::Config {
    crate::Config::default()
}
```

---

## Appendix B: Test Checklist for Code Review

| Criterion | Required? |
|-----------|-----------|
| Every `pub` function has at least one test | ✓ |
| Every enum variant is tested | ✓ |
| Error paths are tested (not just happy path) | ✓ |
| Serde round-trip tested for serializable types | ✓ |
| `Default` invariant tested when `Default` is implemented | ✓ |
| Clone produces equal value when `Clone` is implemented | ✓ |
| `Display` produces non-empty output when implemented | ✓ |
| `Debug` produces non-empty output | ✓ |
| No `unwrap()`/`expect()` in production code (gated by clippy) | ✓ |
| Property tests added for types with complex invariants | Recommended |
| Snapshot tests added for any formatted output | Recommended |
| Integration tests added for cross-crate workflows | ✓ (new feature) |

---

## Appendix C: Running Tests Reference

```bash
# === Basic ===

# Run all tests in workspace
cargo test --workspace

# Run tests for a single crate
cargo test --package uml-core

# Run a specific test by name
cargo test --package uml-core -- umlid_uniqueness

# Run tests matching a pattern
cargo test --package uml-core -- serde

# === Test Categories ===

# Run only fast (non-ignored) tests
cargo test --workspace

# Run only slow (ignored) tests
cargo test --workspace -- --ignored

# Run both fast and slow
cargo test --workspace -- --include-ignored

# Run only integration tests (tests/ directory)
cargo test --workspace --test '*'

# Run only doc tests
cargo test --workspace --doc

# === Property Tests ===

# Run more iterations for property tests
PROPTEST_CASES=10000 cargo test --package uml-core

# Shrink a failing property test case
PROPTEST_VERBOSE=1 cargo test --package uml-core

# === Snapshot Tests ===

# Review pending snapshots
cargo insta review

# Accept all pending snapshots
cargo insta accept

# Reject all pending snapshots
cargo insta reject

# Update snapshots (accept all + re-run tests)
cargo insta test --accept

# === Coverage ===

# Generate HTML coverage report
cargo tarpaulin --package uml-core --out html

# Generate LCOV report (for Codecov)
cargo llvm-cov --workspace --lcov --output-path lcov.info

# === Test Debugging ===

# Show test output (println! etc.)
cargo test -- --nocapture

# Run tests with backtrace
RUST_BACKTRACE=1 cargo test

# Run tests with logging
RUST_LOG=debug cargo test

# Run tests in a single thread (for tests with shared resources)
cargo test -- --test-threads=1

# === CI Simulation ===

# Simulate CI: format → clippy → build → test
cargo xtask ci

# Simulate full CI including slow tests
cargo xtask ci && cargo test --workspace -- --ignored
```

---

*End of Testing Strategy Document*
