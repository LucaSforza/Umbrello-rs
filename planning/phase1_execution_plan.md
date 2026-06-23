# Phase 1 Execution Plan: Project Foundation (Milestone 1)

> **Document:** `rust-rewrite/planning/phase1_execution_plan.md`
> **Status:** Ready for execution
> **Date:** 2026-06-23
> **Estimated effort:** 1 week (single developer) or 2–3 days (2 developers parallel)
> **Risk:** Low — no domain logic, only build system and tooling configuration

---

## Table of Contents

1. [Overview](#1-overview)
2. [Crate Inventory](#2-crate-inventory)
3. [Task Decomposition & Ordering](#3-task-decomposition--ordering)
4. [File Creation Checklist](#4-file-creation-checklist)
5. [Workspace Dependency Table](#5-workspace-dependency-table)
6. [Crate Dependency Map](#6-crate-dependency-map)
7. [CI Pipeline Details](#7-ci-pipeline-details)
8. [Dev Tooling Details](#8-dev-tooling-details)
9. [Verification Steps](#9-verification-steps)
10. [Completion Criteria](#10-completion-criteria)
11. [Delegation Instructions](#11-delegation-instructions)
12. [Rollback Procedure](#12-rollback-procedure)

---

## 1. Overview

### 1.1 What Milestone 1 Delivers

A fully configured Cargo workspace with all 21 crate stubs, CI/CD pipeline, developer tooling
configuration, and automated quality gates. **No domain logic is implemented** — every crate
has an empty `src/lib.rs` with module declarations and doc comments, and compiles without
warnings.

### 1.2 What Does NOT Ship

- ❌ No UML model types or enums
- ❌ No XMI parsing or writing
- ❌ No code generation or import logic
- ❌ No diagram rendering
- ❌ No GUI
- ❌ No test files (XMI fixtures)
- ❌ No documentation (mdBook skeleton deferred)

These begin in Phase 2.

### 1.3 Key Architectural Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Edition | 2024 | Latest stable Rust edition; use `#[feature]` only for edition-gated items |
| Resolver | 2 | Required for workspace with optional dependencies (v2 resolves feature flags correctly) |
| MSRV | 1.85 | Current stable as of June 2026; pinned in `rust-toolchain.toml` |
| Package version | 1.0.0 | Start at 1.0.0 per crate_layout.md; crate versions track workspace version |
| `[forbid(unsafe_code)]` | All crates | Enforce memory safety by default; whitelist only crates that need FFI |
| `[warn(missing_docs)]` | All crates | Enforce documentation discipline from day one |
| `Cargo.lock` | Committed | Applications (`uml-cli`, `umbrello-desktop`) need deterministic builds |
| Dependency versions | Workspace-level | All shared deps defined once in `[workspace.dependencies]` |

---

## 2. Crate Inventory

### 2.1 Complete Crate Table

| # | Crate | Path | Tier | Type | Depends On | Purpose |
|---|-------|------|:----:|:----:|------------|---------|
| 1 | `uml-common` | `crates/uml-common/` | 0 | Library | — | Shared error types, logging, version constants |
| 2 | `uml-core` | `crates/uml-core/` | 0.5 | Library | `uml-common` | Pure UML domain model, types, enums, repository |
| 3 | `uml-xmi` | `crates/uml-xmi/` | 1 | Library | `uml-core` | XMI 1.2/2.1 serialization reader/writer |
| 4 | `uml-undo` | `crates/uml-undo/` | 1 | Library | `uml-core` | Command-pattern undo/redo stack |
| 5 | `uml-diagram` | `crates/uml-diagram/` | 1 | Library | `uml-core` | Diagram model: widgets, edges, scene data |
| 6 | `uml-codegen` | `crates/uml-codegen/` | 1 | Library | `uml-core` | Code generation framework trait + registry |
| 7 | `uml-import` | `crates/uml-import/` | 1 | Library | `uml-core` | Code import framework trait + registry |
| 8 | `uml-persistence` | `crates/uml-persistence/` | 2 | Library | `uml-core`, `uml-xmi`, `uml-undo` | File I/O pipeline, compression, format detection |
| 9 | `uml-render` | `crates/uml-render/` | 2 | Library | `uml-core`, `uml-diagram` | Diagram rendering backend trait + implementations |
| 10 | `uml-layout` | `crates/uml-layout/` | 2 | Library | `uml-diagram` | Auto-layout algorithms, graph-based layout |
| 11 | `uml-export` | `crates/uml-export/` | 3 | Library | `uml-render` | Image export: SVG, PNG, PDF |
| 12 | `uml-codegen-cpp` | `crates/uml-codegen-cpp/` | 2 | Library | `uml-codegen`, `uml-core` | C++ code generator |
| 13 | `uml-codegen-java` | `crates/uml-codegen-java/` | 2 | Library | `uml-codegen`, `uml-core` | Java code generator |
| 14 | `uml-codegen-python` | `crates/uml-codegen-python/` | 2 | Library | `uml-codegen`, `uml-core` | Python code generator |
| 15 | `uml-codegen-rust` | `crates/uml-codegen-rust/` | 2 | Library | `uml-codegen`, `uml-core` | Rust code generator |
| 16 | `uml-import-cpp` | `crates/uml-import-cpp/` | 2 | Library | `uml-import`, `uml-core` | C++ code importer (tree-sitter) |
| 17 | `uml-import-java` | `crates/uml-import-java/` | 2 | Library | `uml-import`, `uml-core` | Java code importer (tree-sitter) |
| 18 | `uml-import-python` | `crates/uml-import-python/` | 2 | Library | `uml-import`, `uml-core` | Python code importer (tree-sitter) |
| 19 | `uml-cli` | `apps/uml-cli/` | 3 | Binary | `uml-persistence`, `uml-export`, `uml-common` | CLI for headless operations |
| 20 | `umbrello-desktop` | `apps/umbrello-desktop/` | 3 | Binary | All foundation crates | Desktop GUI application |
| 21 | `xtask` | `xtask/` | — | Binary | — | Build/dev task runner (only `anyhow`, `clap`, `serde_json`, `walkdir`) |

### 2.2 Tier Diagram

```
                 ┌──────────────────────────────────────────────────────────────┐
    Tier 3       │  apps/uml-cli, apps/umbrello-desktop, crates/uml-export     │
    (leaves)     └──────────────┬────────────────────┬─────────────────────────┘
                               │                    │
                 ┌─────────────▼─────┐    ┌──────────▼──────────┐
    Tier 2       │  uml-persistence  │    │  uml-render         │
    (middle)     │  uml-layout       │    │  *-codegen-*        │
                 │                   │    │  *-import-*         │
                 └─────────┬─────────┘    └──────────┬──────────┘
                           │                        │
                 ┌─────────▼────────────────────────▼──────────┐
    Tier 1       │  uml-xmi   uml-undo   uml-diagram           │
    (core deps)  │  uml-codegen   uml-import                    │
                 └────────────────────┬─────────────────────────┘
                                      │
                 ┌────────────────────▼─────────────────────────┐
    Tier 0       │  uml-core   │   uml-common                    │
    (foundation) │  (depends on uml-common)                     │
                 └───────────────────────────────────────────────┘
```

### 2.3 Dependency Rules

1. A crate may only depend on crates from the **same or lower tier**.
2. `uml-common` must never depend on any workspace crate.
3. `uml-core` must never depend on Tier 1+ crates.
4. No circular dependencies at any tier.
5. Binary crates (`uml-cli`, `umbrello-desktop`, `xtask`) are always leaves.

---

## 3. Task Decomposition & Ordering

### 3.1 Task Graph

```
Task 1: Create directory structure
  │
  ├──► Task 2: Root Cargo.toml (workspace manifest)
  │       │
  │       ├──► Task 3: rust-toolchain.toml
  │       ├──► Task 4: rustfmt.toml
  │       ├──► Task 5: clippy.toml
  │       ├──► Task 6: deny.toml
  │       └──► Task 7: .cargo/config.toml
  │
  ├──► Task 8: Tier 0 crates (uml-common, uml-core)
  │       │
  │       ├──► Task 9: Tier 1 crates (uml-xmi, uml-undo, uml-diagram, uml-codegen, uml-import)
  │       │       │
  │       │       ├──► Task 10: Tier 2 crates (uml-persistence, uml-render, uml-layout,
  │       │       │   │          uml-export, *-codegen-*, *-import-*)
  │       │       │   │
  │       │       │   └──► Task 11: Tier 3 crates (uml-cli, umbrello-desktop)
  │       │       │
  │       │       └──► Task 12: xtask crate
  │
  ├──► Task 13: .gitignore updates
  │
  └──► Task 14: CI pipeline (.github/workflows/ci.yml)
          │
          └──► Task 15: Verification (build, test, clippy, fmt)
```

### 3.2 Parallelization Opportunities

| Task | Parallel With | Notes |
|------|--------------|-------|
| 2 (Root Cargo.toml) | 1 (Directory structure) | Must be sequential; root manifest needs dirs |
| 3–7 (Config files) | Each other | All independent; write in parallel |
| 8 (Tier 0) | 3–7 | Can write crate stubs while tooling config is written |
| 9 (Tier 1) | 8 | Must have `uml-core` done first |
| 10 (Tier 2) | 9 | Must have Tier 1 dependencies done |
| 11 (Tier 3) | 10 | Must have Tier 2 dependencies done |
| 12 (xtask) | 8–11 | Independent of crate structure |
| 13 (.gitignore) | 2–7 | Independent |
| 14 (CI) | 12 | Must have some crates stubbed for CI to pass |
| 15 (Verification) | All preceding | Must run last |

### 3.3 Recommended Order for a Single Developer

1. Create directory structure for all 21 crates
2. Write root `Cargo.toml` (workspace manifest)
3. Write 5 config files (`rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `.cargo/config.toml`)
4. Write Tier 0 crate stubs (2 crates)
5. Write Tier 1 crate stubs (5 crates)
6. Write Tier 2 crate stubs (9 crates)
7. Write Tier 3 crate stubs (3 crates: uml-export + 2 apps)
8. Write xtask crate
9. Write `.github/workflows/ci.yml`
10. Update `.gitignore`
11. Verify: `cargo build --workspace`, `cargo test --workspace`, `cargo clippy`, `cargo fmt --check`

---

## 4. File Creation Checklist

### 4.1 Workspace Root Files

| # | File | Purpose | Content Summary |
|---|------|---------|----------------|
| 1 | `Cargo.toml` | Workspace manifest | `[workspace]` with all 21 members, `[workspace.package]`, `[workspace.dependencies]`, `[workspace.lints]`, `[features]` |
| 2 | `rust-toolchain.toml` | Rust toolchain pinning | `channel = "stable"`, `components = ["rustfmt", "clippy", "rust-analyzer"]`, `target = "x86_64-unknown-linux-gnu"` |
| 3 | `rustfmt.toml` | Formatting rules | `tab_spaces = 4`, `edition = "2024"`, match project conventions |
| 4 | `clippy.toml` | Clippy configuration | Allowed/forbidden lints, MSRV, doc-valid-idents |
| 5 | `deny.toml` | cargo-deny configuration | License allowlist (GPL-2.0-or-later, MIT, Apache-2.0, etc.), advisory DB, bans |
| 6 | `.cargo/config.toml` | Build cache options | `[target.x86_64-unknown-linux-gnu]`, incremental, parallel compilation |
| 7 | `.github/workflows/ci.yml` | CI pipeline | Build, test, fmt, clippy, deny checks |
| 8 | `.gitignore` | Ignore patterns | `/target/`, IDE files, OS files, build artifacts |

### 4.2 Tier 0 Crate Files

#### `crates/uml-common/`

| # | File | Purpose |
|---|------|---------|
| 9 | `Cargo.toml` | Package manifest with `thiserror`, `serde`, `tracing`, `tracing-subscriber` |
| 10 | `src/lib.rs` | Module declarations: `error`, `logging`, `version`; re-exports; `#![forbid(unsafe_code)]` |

#### `crates/uml-core/`

| # | File | Purpose |
|---|------|---------|
| 11 | `Cargo.toml` | Package manifest with `uml-common`, `slotmap`, `uuid`, `bitflags`, `serde`, `thiserror`, `tracing` |
| 12 | `src/lib.rs` | Module declarations: `types`, `model`, `id`, `repository`, `event`, `traits`; re-exports |

### 4.3 Tier 1 Crate Files

#### `crates/uml-xmi/`

| # | File | Purpose |
|---|------|---------|
| 13 | `Cargo.toml` | Deps: `uml-core`, `quick-xml`, `serde`, `thiserror`, `tracing`; features: `xmi-v1`, `xmi-v2` |
| 14 | `src/lib.rs` | Modules: `reader`, `writer`, `v1_2`, `v2_1`, `error` |

#### `crates/uml-undo/`

| # | File | Purpose |
|---|------|---------|
| 15 | `Cargo.toml` | Deps: `uml-core`, `thiserror` |
| 16 | `src/lib.rs` | Modules: `command`, `stack`, `commands` |

#### `crates/uml-diagram/`

| # | File | Purpose |
|---|------|---------|
| 17 | `Cargo.toml` | Deps: `uml-core`, `serde`, `thiserror` |
| 18 | `src/lib.rs` | Modules: `types`, `scene`, `widgets`, `associations`, `factory` |

#### `crates/uml-codegen/`

| # | File | Purpose |
|---|------|---------|
| 19 | `Cargo.toml` | Deps: `uml-core`, `async-trait`, `serde`, `thiserror`, `tracing` |
| 20 | `src/lib.rs` | Modules: `generator`, `registry`, `writer`, `config` |

#### `crates/uml-import/`

| # | File | Purpose |
|---|------|---------|
| 21 | `Cargo.toml` | Deps: `uml-core`, `async-trait`, `tree-sitter`, `thiserror`, `tracing` |
| 22 | `src/lib.rs` | Modules: `importer`, `registry`, `utils` |

### 4.4 Tier 2 Crate Files

#### `crates/uml-persistence/`

| # | File | Purpose |
|---|------|---------|
| 23 | `Cargo.toml` | Deps: `uml-core`, `uml-xmi`, `uml-undo`, `tokio`, `serde`, `thiserror`, `tracing`; optional: `flate2`, `tar`, `bzip2`, `zip`; features: `compression`, `autosave` |
| 24 | `src/lib.rs` | Modules: `storage`, `xmi_storage`, `compression`, `error` |

#### `crates/uml-render/`

| # | File | Purpose |
|---|------|---------|
| 25 | `Cargo.toml` | Deps: `uml-diagram`, `uml-core`, `vello`, `cosmic-text`, `tracing` |
| 26 | `src/lib.rs` | Modules: `canvas`, `render`, `text`, `line`, `interaction` |

#### `crates/uml-layout/`

| # | File | Purpose |
|---|------|---------|
| 27 | `Cargo.toml` | Deps: `uml-diagram`; optional: `petgraph`; features: `auto-layout`, `force-layout` |
| 28 | `src/lib.rs` | Modules: `graph`, `force`, `grid`, `alignment` |

#### `crates/uml-export/`

| # | File | Purpose |
|---|------|---------|
| 29 | `Cargo.toml` | Deps: `uml-render`; optional: `resvg`, `image`; features: `png-export`, `svg-export`, `pdf-export` |
| 30 | `src/lib.rs` | Modules: `export`, `svg`, `png`, `pdf` |

#### `crates/uml-codegen-cpp/`

| # | File | Purpose |
|---|------|---------|
| 31 | `Cargo.toml` | Deps: `uml-codegen`, `uml-core`, `tracing` |
| 32 | `src/lib.rs` | Module declarations: `generator`, `header`, `source`, `type_mapping` |

#### `crates/uml-codegen-java/`

| # | File | Purpose |
|---|------|---------|
| 33 | `Cargo.toml` | Deps: `uml-codegen`, `uml-core`, `tracing` |
| 34 | `src/lib.rs` | Module declarations: `generator`, `type_mapping` |

#### `crates/uml-codegen-python/`

| # | File | Purpose |
|---|------|---------|
| 35 | `Cargo.toml` | Deps: `uml-codegen`, `uml-core`, `tracing` |
| 36 | `src/lib.rs` | Module declarations: `generator`, `type_mapping` |

#### `crates/uml-codegen-rust/`

| # | File | Purpose |
|---|------|---------|
| 37 | `Cargo.toml` | Deps: `uml-codegen`, `uml-core`, `tracing` |
| 38 | `src/lib.rs` | Module declarations: `generator`, `type_mapping` |

#### `crates/uml-import-cpp/`

| # | File | Purpose |
|---|------|---------|
| 39 | `Cargo.toml` | Deps: `uml-import`, `uml-core`, `tree-sitter`, `tree-sitter-cpp` |
| 40 | `src/lib.rs` | Module declarations: `importer`, `class`, `function`, `field`, `namespace` |

#### `crates/uml-import-java/`

| # | File | Purpose |
|---|------|---------|
| 41 | `Cargo.toml` | Deps: `uml-import`, `uml-core`, `tree-sitter`, `tree-sitter-java` |
| 42 | `src/lib.rs` | Module declarations: `importer`, `class`, `method`, `field` |

#### `crates/uml-import-python/`

| # | File | Purpose |
|---|------|---------|
| 43 | `Cargo.toml` | Deps: `uml-import`, `uml-core`, `tree-sitter`, `tree-sitter-python` |
| 44 | `src/lib.rs` | Module declarations: `importer`, `class`, `function`, `assignment` |

### 4.5 Tier 3 (Application) Crate Files

#### `apps/uml-cli/`

| # | File | Purpose |
|---|------|---------|
| 45 | `Cargo.toml` | Binary target; deps: `uml-persistence`, `uml-export`, `uml-common`, `clap`, `tokio`, `tracing`, `tracing-subscriber`, `anyhow`; optional: codegen/import crates; features: `codegen-cpp`, etc. |
| 46 | `src/main.rs` | Entry point with clap argument parsing; `fn main() -> Result<()>` |
| 47 | `src/lib.rs` | Shared CLI logic (export, import, validate) |

#### `apps/umbrello-desktop/`

| # | File | Purpose |
|---|------|---------|
| 48 | `Cargo.toml` | Binary target; deps: `uml-core`, `uml-diagram`, `uml-render`, `uml-persistence`, `uml-undo`, `uml-common`, `uml-layout`, `uml-codegen`, `uml-import`, `egui`, `eframe`; optional: language crates |
| 49 | `src/main.rs` | Entry point; `eframe::run_native` |
| 50 | `src/lib.rs` | App module: `app`, `panels`, `dialogs`, `canvas` |

### 4.6 Tooling Crates

#### `xtask/`

| # | File | Purpose |
|---|------|---------|
| 51 | `Cargo.toml` | Deps: `anyhow`, `clap`, `serde_json`, `walkdir` |
| 52 | `src/main.rs` | CLI dispatch: `build`, `test`, `docs`, `init-test-data`, `check-xmi` |

### 4.7 Total File Count

| Category | Count |
|----------|:-----:|
| Workspace root config files | 8 |
| Tier 0 crate files | 4 |
| Tier 1 crate files | 10 |
| Tier 2 crate files | 24 |
| Tier 3 crate files | 6 |
| xtask crate files | 2 |
| **Total** | **54** |

---

## 5. Workspace Dependency Table

### 5.1 All `[workspace.dependencies]` Entries

| Dependency | Version | Features | Used By |
|------------|:-------:|----------|---------|
| `serde` | 1 | `derive` | All crates (serialization) |
| `thiserror` | 2 | — | All crates (error types) |
| `tracing` | 0.1 | — | Most crates (diagnostics) |
| `tracing-subscriber` | 0.3 | — | `uml-common`, binaries |
| `slotmap` | 1 | — | `uml-core` (arena storage) |
| `uuid` | 1 | `v4`, `serde` | `uml-core` (identifiers) |
| `bitflags` | 2 | — | `uml-core` (bitflag types) |
| `quick-xml` | 0.37 | — | `uml-xmi` (XML parsing) |
| `tokio` | 1 | `full` | `uml-persistence`, binaries |
| `clap` | 4 | `derive` | Binaries (`uml-cli`, `xtask`) |
| `async-trait` | 0.1 | — | `uml-codegen`, `uml-import` |
| `petgraph` | 0.7 | — | `uml-layout` (optional) |
| `tree-sitter` | 0.25 | — | `uml-import`, import crates |
| `anyhow` | 1 | — | `uml-cli`, `xtask` (error handling) |
| `serde_json` | 1 | — | `xtask` |
| `walkdir` | 2 | — | `xtask` |
| `flate2` | 1 | — | `uml-persistence` (optional) |
| `tar` | 0.4 | — | `uml-persistence` (optional) |
| `bzip2` | 0.5 | — | `uml-persistence` (optional) |
| `zip` | 2 | — | `uml-persistence` (optional) |
| `vello` | *latest* | — | `uml-render` |
| `cosmic-text` | *latest* | — | `uml-render` |
| `egui` | *latest* | — | `umbrello-desktop` |
| `eframe` | *latest* | — | `umbrello-desktop` |
| `tree-sitter-cpp` | *latest* | — | `uml-import-cpp` |
| `tree-sitter-java` | *latest* | — | `uml-import-java` |
| `tree-sitter-python` | *latest* | — | `uml-import-python` |

### 5.2 Dev Dependencies (Workspace-Level)

| Dependency | Version | Used By |
|------------|:-------:|---------|
| `serde_json` | 1 | `uml-core` (round-trip testing) |
| `quickcheck` | 1 | `uml-core` (property-based testing) |
| `quickcheck_macros` | 1 | `uml-core` |
| `insta` | *latest* | All crates (snapshot testing) |

### 5.3 Version Policy

- All dependencies use **caret requirements** (`"1"`, `"0.1"`, etc.) to automatically receive
  compatible updates.
- Exceptions: `vello`, `cosmic-text`, `egui`, `eframe` — these evolve rapidly; pin to
  specific minor versions (e.g., `"=0.28"`) and update explicitly.
- `tree-sitter-*` language crates track `tree-sitter` major version.
- Run `cargo update` before Phase 2 to pull latest compatible versions.

---

## 6. Crate Dependency Map

### 6.1 Inter-Crate Dependencies

| Crate | Path Dependencies | Direction |
|-------|-------------------|-----------|
| `uml-common` | *(none)* | — |
| `uml-core` | `uml-common` | → Tier 0 |
| `uml-xmi` | `uml-core` | → Tier 0.5 |
| `uml-undo` | `uml-core` | → Tier 0.5 |
| `uml-diagram` | `uml-core` | → Tier 0.5 |
| `uml-codegen` | `uml-core` | → Tier 0.5 |
| `uml-import` | `uml-core` | → Tier 0.5 |
| `uml-persistence` | `uml-core`, `uml-xmi`, `uml-undo` | → Tier 0.5, Tier 1 |
| `uml-render` | `uml-core`, `uml-diagram` | → Tier 0.5, Tier 1 |
| `uml-layout` | `uml-diagram` | → Tier 1 |
| `uml-export` | `uml-render` | → Tier 2 |
| `uml-codegen-cpp` | `uml-codegen`, `uml-core` | → Tier 1 |
| `uml-codegen-java` | `uml-codegen`, `uml-core` | → Tier 1 |
| `uml-codegen-python` | `uml-codegen`, `uml-core` | → Tier 1 |
| `uml-codegen-rust` | `uml-codegen`, `uml-core` | → Tier 1 |
| `uml-import-cpp` | `uml-import`, `uml-core` | → Tier 1 |
| `uml-import-java` | `uml-import`, `uml-core` | → Tier 1 |
| `uml-import-python` | `uml-import`, `uml-core` | → Tier 1 |
| `uml-cli` | `uml-persistence`, `uml-export`, `uml-common`; optional: codegen/import crates | → Tier 2 |
| `umbrello-desktop` | `uml-core`, `uml-diagram`, `uml-render`, `uml-persistence`, `uml-undo`, `uml-common`, `uml-layout`; optional: codegen/import crates | → Tier 2 |
| `xtask` | *(none — no workspace deps)* | — |

### 6.2 External Dependency per Crate

| Crate | External Dependencies |
|-------|----------------------|
| `uml-common` | `thiserror`, `serde`, `tracing`, `tracing-subscriber` |
| `uml-core` | `slotmap`, `uuid`, `bitflags`, `serde`, `thiserror`, `tracing` |
| `uml-xmi` | `quick-xml`, `serde`, `thiserror`, `tracing` |
| `uml-undo` | `thiserror` |
| `uml-diagram` | `serde`, `thiserror` |
| `uml-codegen` | `async-trait`, `serde`, `thiserror`, `tracing` |
| `uml-import` | `async-trait`, `tree-sitter`, `thiserror`, `tracing` |
| `uml-persistence` | `tokio`, `serde`, `thiserror`, `tracing`; opt: `flate2`, `tar`, `bzip2`, `zip` |
| `uml-render` | `vello`, `cosmic-text`, `tracing` |
| `uml-layout` | opt: `petgraph` |
| `uml-export` | opt: `resvg`, `image` |
| `uml-codegen-cpp` | `tracing` |
| `uml-codegen-java` | `tracing` |
| `uml-codegen-python` | `tracing` |
| `uml-codegen-rust` | `tracing` |
| `uml-import-cpp` | `tree-sitter`, `tree-sitter-cpp` |
| `uml-import-java` | `tree-sitter`, `tree-sitter-java` |
| `uml-import-python` | `tree-sitter`, `tree-sitter-python` |
| `uml-cli` | `clap`, `tokio`, `tracing`, `tracing-subscriber`, `anyhow` |
| `umbrello-desktop` | `egui`, `eframe` |
| `xtask` | `anyhow`, `clap`, `serde_json`, `walkdir` |

### 6.3 Feature Flag Propagation

```
Workspace feature "codegen-cpp" ──► uml-cli: "codegen-cpp" ──► uml-codegen-cpp
Workspace feature "codegen-java" ──► uml-cli: "codegen-java" ──► uml-codegen-java
Workspace feature "import-cpp"  ──► uml-cli: "import-cpp"  ──► uml-import-cpp
```

The workspace `[features]` section defines conveniences for building everything:

```toml
[features]
default = ["codegen-cpp", "codegen-java", "codegen-python", "codegen-rust",
           "import-cpp", "import-java", "import-python"]

codegen-cpp   = ["uml-cli?/codegen-cpp", "umbrello-desktop?/codegen-cpp"]
codegen-java  = ["uml-cli?/codegen-java", "umbrello-desktop?/codegen-java"]
codegen-python = ["uml-cli?/codegen-python", "umbrello-desktop?/codegen-python"]
codegen-rust  = ["uml-cli?/codegen-rust", "umbrello-desktop?/codegen-rust"]
import-cpp    = ["uml-cli?/import-cpp", "umbrello-desktop?/import-cpp"]
import-java   = ["uml-cli?/import-java", "umbrello-desktop?/import-java"]
import-python = ["uml-cli?/import-python", "umbrello-desktop?/import-python"]
```

---

## 7. CI Pipeline Details

### 7.1 Workflow File

`.github/workflows/ci.yml` — a single workflow with three jobs:

#### Job 1: `check` (fast — runs first)

```yaml
check:
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
      with:
        components: rustfmt, clippy
    - uses: swatinem/rust-cache@v2
      with:
        cache-on-failure: true
    - run: cargo fmt --all --check
      name: Check formatting
    - run: cargo clippy --workspace --all-targets -- -D warnings
      name: Clippy lint check
```

#### Job 2: `test` (runs after check)

```yaml
test:
  runs-on: ubuntu-latest
  needs: check
  steps:
    - uses: actions/checkout@v4
    - uses: dtolnay/rust-toolchain@stable
    - uses: swatinem/rust-cache@v2
    - run: cargo build --workspace
      name: Build workspace
    - run: cargo test --workspace
      name: Run tests
    - run: cargo doc --workspace --no-deps
      name: Build documentation
```

#### Job 3: `deny` (advisory/license audit)

```yaml
deny:
  runs-on: ubuntu-latest
  needs: check
  steps:
    - uses: actions/checkout@v4
    - uses: EmbarkStudios/cargo-deny-action@v2
      with:
        command: check advisories
        arguments: --hide-inclusion-graph
    - uses: EmbarkStudios/cargo-deny-action@v2
      with:
        command: check licenses
```

### 7.2 Caching Strategy

- Use `swatinem/rust-cache@v2` with `cache-on-failure: true`
- Cache key includes `Cargo.lock`, `rust-toolchain.toml`, and all `Cargo.toml` files
- Separate caches per job (check, test, deny)
- Warm cache: ~2 min restore, ~8 min cold build → ~30s warm build

### 7.3 Trigger Configuration

```yaml
on:
  push:
    branches: [main, develop]
  pull_request:
    branches: [main]
  schedule:
    - cron: '0 6 * * 1'  # Weekly Monday 06:00 UTC advisory check
```

### 7.4 Matrix Strategy (Future)

Once Phase 2+ adds platform-specific code, expand to a matrix:

```yaml
strategy:
  matrix:
    os: [ubuntu-latest, macos-latest, windows-latest]
```

Deferred until Phase 3 (when platform-dependent code like compression paths appears).

---

## 8. Dev Tooling Details

### 8.1 `rustfmt.toml` Configuration

```toml
# Umbrello-RS Rustfmt Configuration
# Matches project conventions from crate_layout.md

edition = "2024"

# 4-space indent (matching C++ project convention)
tab_spaces = 4
hard_tabs = false

# Line width
max_width = 120

# Imports
imports_granularity = "Module"
imports_layout = "HorizontalVertical"
group_imports = "StdExternalCrate"
reorder_imports = true
reorder_modules = true

# Newlines
newline_style = "Unix"

# Patterns
use_small_heuristics = "Max"
fn_call_width = 80
attr_fn_like_width = 60
struct_lit_width = 60
struct_variant_width = 60

# Misc
trailing_comma = "Vertical"
match_block_trailing_comma = true
binop_separator = "Front"
space_after_colon = true
space_before_colon = false
```

### 8.2 `clippy.toml` Configuration

```toml
# Umbrello-RS Clippy Configuration

# Minimum supported Rust version
msrv = "1.85"

# Allowed lints — these are style choices, not bugs
allow = [
    "module_name_repetitions",  # uml-core/src/model/model.rs is fine
    "doc_markdown",             # UML, XMI, Umbrello are not doc-markdown names
    "too_many_lines",           # Some generated code is verbose
    "similar_names",            # e.g., role_a / role_b in associations
]

# Forbidden patterns
deny = [
    "unsafe_code",              # No unsafe unless explicitly whitelisted
    "unreachable_pub",          # All pub items should be reachable
    "missing_docs_in_private_items",
]

# Warnings that must be addressed before merge
warn = [
    "cargo",
    "cloned_instead_of_copied",
    "large_enum_variant",
    "manual_string_new",
    "map_unwrap_or",
    "match_same_arms",
    "needless_late_init",
    "option_if_let_else",
    "redundant_else",
    "single_char_add_str",
    "single_char_push_str",
    "str_to_string",
    "suspicious_operation_groupings",
    "unnecessary_join",
    "unnecessary_literal_unwrap",
    "unnecessary_wraps",
    "use_self",
    "useless_asref",
    "wildcard_imports",
]
```

### 8.3 `deny.toml` Configuration

```toml
# cargo-deny configuration

[advisories]
vulnerability = "deny"
unmaintained = "deny"
yanked = "warn"
notice = "warn"
ignore = []

[licenses]
default = "deny"
# Allow the most common Rust ecosystem licenses
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "Unicode-DFS-2016",
    "MPL-2.0",
]
deny = [
    "AGPL-3.0",
    "AGPL-3.0-only",
    "CC0-1.0",          # Requires special approval
]
copyleft = "deny"
allow-osi-fsf-free = "neither"
confidence-threshold = 0.8

[[licenses.clarify]]
name = "unicode-ident"
expression = "MIT OR Apache-2.0"

[licenses.private]
ignore = true
# Our own workspace crates are GPL-2.0-or-later
registrations = [
    { name = "uml-common",     license = "GPL-2.0-or-later" },
    { name = "uml-core",       license = "GPL-2.0-or-later" },
    { name = "uml-xmi",        license = "GPL-2.0-or-later" },
    { name = "uml-undo",       license = "GPL-2.0-or-later" },
    { name = "uml-diagram",    license = "GPL-2.0-or-later" },
    { name = "uml-codegen",    license = "GPL-2.0-or-later" },
    { name = "uml-import",     license = "GPL-2.0-or-later" },
    { name = "uml-persistence",license = "GPL-2.0-or-later" },
    { name = "uml-render",     license = "GPL-2.0-or-later" },
    { name = "uml-layout",     license = "GPL-2.0-or-later" },
    { name = "uml-export",     license = "GPL-2.0-or-later" },
    { name = "uml-codegen-cpp",   license = "GPL-2.0-or-later" },
    { name = "uml-codegen-java",  license = "GPL-2.0-or-later" },
    { name = "uml-codegen-python",license = "GPL-2.0-or-later" },
    { name = "uml-codegen-rust",  license = "GPL-2.0-or-later" },
    { name = "uml-import-cpp",    license = "GPL-2.0-or-later" },
    { name = "uml-import-java",   license = "GPL-2.0-or-later" },
    { name = "uml-import-python", license = "GPL-2.0-or-later" },
    { name = "uml-cli",        license = "GPL-2.0-or-later" },
    { name = "umbrello-desktop", license = "GPL-2.0-or-later" },
    { name = "xtask",          license = "GPL-2.0-or-later" },
]

[bans]
multiple-versions = "deny"
skip-tree = []
deny = [
    # No duplicate dependencies allowed
]
```

### 8.4 `rust-toolchain.toml` Configuration

```toml
[toolchain]
channel = "stable"
components = ["rustfmt", "clippy", "rust-analyzer"]
targets = [
    "x86_64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-pc-windows-msvc",
]
```

### 8.5 `.cargo/config.toml` Configuration

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=lld"]

[profile.dev]
incremental = true
codegen-units = 256       # Faster compilation, worse optimization
debug = 1                  # Line tables only (smaller, faster)

[profile.release]
incremental = false
codegen-units = 1          # Best optimization
lto = "thin"               # ThinLTO for balance of speed and binary size
strip = "symbols"          # Remove debug symbols in release
opt-level = 3

# Compile faster by using more parallelism
build.jobs = 0             # 0 = use all available cores

# Cache in a separate directory (outside workspace for CI)
build.target-dir = "target"
```

### 8.6 xtask Commands

| Command | What It Does | Implementation |
|---------|-------------|----------------|
| `cargo xtask build` | `cargo build --workspace` | Runs `std::process::Command` |
| `cargo xtask test` | `cargo test --workspace` | Runs `std::process::Command` |
| `cargo xtask docs` | Build mdBook documentation | Placeholder (future) |
| `cargo xtask init-test-data` | Copy XMI files from C++ repo | Reads `UMBRELLO_SRC` env var, copies `*.xmi` files |
| `cargo xtask check-xmi <path>` | Validate XMI file structure | Placeholder (future — delegates to `uml-xmi`) |
| `cargo xtask lint` | `cargo clippy --workspace --all-targets -- -D warnings` | Runs `std::process::Command` |
| `cargo xtask format` | `cargo fmt --all` | Runs `std::process::Command` |
| `cargo xtask lint-deny` | `cargo deny check advisories && cargo deny check licenses` | Runs `std::process::Command` |
| `cargo xtask ci` | Run all CI checks sequentially (fmt → clippy → build → test → deny) | Meta-command calling sub-commands |

#### xtask `src/main.rs` Skeleton

```rust
//! xtask — development workflow automation for Umbrello-RS.
//!
//! Usage: cargo xtask <command>
//!
//! Commands:
//!   build         — cargo build --workspace
//!   test          — cargo test --workspace
//!   lint          — cargo clippy --workspace --all-targets -- -D warnings
//!   format        — cargo fmt --all
//!   lint-deny     — cargo deny check
//!   ci            — run all CI checks sequentially
//!   init-test-data — copy XMI test files from C++ Umbrello repo
//!   check-xmi <path> — validate XMI file (placeholder)

use clap::Parser;

#[derive(Parser)]
#[command(name = "xtask", about = "Umbrello-RS development workflow automation")]
enum Commands {
    Build,
    Test,
    Lint,
    Format,
    LintDeny,
    Ci,
    InitTestData,
    CheckXmi { path: String },
}

fn main() -> anyhow::Result<()> {
    let cmd = Commands::parse();
    match cmd {
        Commands::Build => run("cargo", ["build", "--workspace"]),
        Commands::Test => run("cargo", ["test", "--workspace"]),
        Commands::Lint => run("cargo", ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"]),
        Commands::Format => run("cargo", ["fmt", "--all"]),
        Commands::LintDeny => {
            run("cargo", ["deny", "check", "advisories"])?;
            run("cargo", ["deny", "check", "licenses"])
        }
        Commands::Ci => {
            println!(">>> cargo fmt --all --check");
            run("cargo", ["fmt", "--all", "--check"])?;
            println!(">>> cargo clippy...");
            run("cargo", ["clippy", "--workspace", "--all-targets", "--", "-D", "warnings"])?;
            println!(">>> cargo build...");
            run("cargo", ["build", "--workspace"])?;
            println!(">>> cargo test...");
            run("cargo", ["test", "--workspace"])?;
            println!(">>> cargo deny...");
            run("cargo", ["deny", "check", "advisories"])?;
            run("cargo", ["deny", "check", "licenses"])
        }
        Commands::InitTestData => {
            let src = std::env::var("UMBRELLO_SRC")
                .map_err(|_| anyhow::anyhow!("UMBRELLO_SRC not set"))?;
            run("cp", &["-r", &format!("{}/tests/data", src), "tests/data"])
        }
        Commands::CheckXmi { path: _path } => {
            // Placeholder — will delegate to uml-xmi in Phase 4
            println!("check-xmi: not yet implemented");
            Ok(())
        }
    }
}

fn run(program: &str, args: impl AsRef<[&str]>) -> anyhow::Result<()> {
    let output = std::process::Command::new(program)
        .args(args.as_ref())
        .spawn()?
        .wait()?;
    if !output.success() {
        anyhow::bail!("{} failed with exit code {:?}", program, output.code());
    }
    Ok(())
}
```

---

## 9. Verification Steps

### 9.1 Quick Verification Script

Run these commands in sequence. Any failure must be fixed before proceeding.

```bash
# Step 1: Clean check — ensure no stale artifacts
cargo clean

# Step 2: Format check
cargo fmt --all --check
# Expected: no output (or "Formatting complete" with no diffs)

# Step 3: Clippy lint check (strict)
cargo clippy --workspace --all-targets -- -D warnings
# Expected: no warnings, no errors

# Step 4: Build everything
cargo build --workspace
# Expected: all 21 crates compile, zero warnings

# Step 5: Run tests (zero tests expected — all pass trivially)
cargo test --workspace
# Expected: "running 0 tests" on each crate, "result: OK"

# Step 6: Documentation builds
cargo doc --workspace --no-deps
# Expected: no errors

# Step 7: Check dependency licenses
cargo deny check advisories
cargo deny check licenses
# Expected: no advisories, all licenses on allowlist

# Step 8: Verify xtask works
cargo run --package xtask -- build
cargo run --package xtask -- test
cargo run --package xtask -- lint
cargo run --package xtask -- format
# Expected: each succeeds
```

### 9.2 Expected Output Verification

| Command | Expected Output |
|---------|----------------|
| `cargo build --workspace` | `Compiling uml-common v1.0.0 ...` (21 crates), no errors |
| `cargo test --workspace` | `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` |
| `cargo clippy --all-targets -- -D warnings` | No output (zero warnings) |
| `cargo fmt --all --check` | No output (everything formatted) |
| `cargo deny check advisories` | No advisories found |
| `cargo deny check licenses` | All licenses OK |
| `cargo run --package xtask -- ci` | All steps pass in sequence |

### 9.3 Negative Tests (Should Fail)

```bash
# These should fail — verify CI gating works
echo "let x = 1;" >> crates/uml-common/src/lib.rs
cargo clippy --package uml-common -- -D warnings
# Should: fail with "unused variable: `x`"

cargo fmt --all --check
# Should: fail with "diff in crates/uml-common/src/lib.rs"

git checkout -- crates/uml-common/src/lib.rs  # Restore
```

---

## 10. Completion Criteria

### 10.1 Mandatory (All Must Pass)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| C1 | `cargo build --workspace` succeeds with zero warnings | Run command, check exit code 0 |
| C2 | `cargo test --workspace` passes with zero failures | Run command, check exit code 0 |
| C3 | `cargo clippy --workspace --all-targets -- -D warnings` passes | Run command, check exit code 0 |
| C4 | `cargo fmt --all --check` passes | Run command, check exit code 0 |
| C5 | `cargo deny check advisories` passes | Run command, check exit code 0 |
| C6 | `cargo deny check licenses` passes | Run command, check exit code 0 |
| C7 | `cargo doc --workspace --no-deps` succeeds | Run command, check exit code 0 |
| C8 | `cargo run --package xtask -- ci` passes | Run command, check exit code 0 |
| C9 | All 21 crate directories exist with `Cargo.toml` and `src/lib.rs` | `ls -d crates/*/ apps/*/ xtask/` |
| C10 | All `Cargo.toml` files have correct `name`, `version`, `edition`, `license` | Manual grep |
| C11 | All `src/lib.rs` files have `#![forbid(unsafe_code)]` | `grep -r 'forbid(unsafe_code)' crates/ apps/ xtask/` |

### 10.2 Quality Gates (Should Pass)

| # | Criterion | How to Verify |
|---|-----------|---------------|
| Q1 | Every crate has `#![warn(missing_docs)]` | `grep -r 'warn(missing_docs)' crates/ apps/` |
| Q2 | Every crate has a doc comment on `lib.rs` | Check first line of each `src/lib.rs` |
| Q3 | `.github/workflows/ci.yml` has all three jobs (check, test, deny) | Read workflow file |
| Q4 | `rust-toolchain.toml` pins `channel = "stable"` | Read file |
| Q5 | `deny.toml` has all 21 workspace crates registered as GPL-2.0-or-later | Read file |
| Q6 | All workspace members listed in root `Cargo.toml` `[workspace]` `members` | Count members |
| Q7 | All crate dependency paths are correct (relative to workspace root) | Verify each `Cargo.toml` path dep |
| Q8 | `Cargo.lock` is committed (no `.gitignore` rule for it) | Check `.gitignore` |
| Q9 | `.gitignore` covers `/target/`, IDE files, OS files | Read file |

### 10.3 Sign-off Checklist

```markdown
## Milestone 1 Sign-off

- [ ] All 21 crate stubs created and compiled
- [ ] Workspace `Cargo.toml` with correct members and workspace deps
- [ ] `rust-toolchain.toml` pins stable Rust
- [ ] `rustfmt.toml` matches 4-space indent convention
- [ ] `clippy.toml` configured (no unsafe, missing_docs warn)
- [ ] `deny.toml` with license allowlist + advisory checks
- [ ] `.cargo/config.toml` with build cache optimization
- [ ] `xtask` crate with build/test/lint/ci commands
- [ ] `.github/workflows/ci.yml` with 3 jobs
- [ ] `.gitignore` updated
- [ ] `cargo build --workspace` — PASS
- [ ] `cargo test --workspace` — PASS
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- [ ] `cargo fmt --all --check` — PASS
- [ ] `cargo deny check advisories` — PASS
- [ ] `cargo deny check licenses` — PASS
- [ ] `cargo run --package xtask -- ci` — PASS

Signed-off by: _____________  Date: _____________
```

---

## 11. Delegation Instructions

### 11.1 Worker Agent Task Splitting

The 15 tasks from Section 3 can be split across up to 3 worker agents:

#### Worker A: "Core Structure" (Tasks 1–7 + 13)

| Task | Description | Files to Create |
|------|-------------|-----------------|
| 1 | Create directory structure | `mkdir -p crates/{uml-common,uml-core,...} apps/{uml-cli,umbrello-desktop} xtask .github/workflows .cargo` |
| 2 | Root `Cargo.toml` | `Cargo.toml` |
| 3 | `rust-toolchain.toml` | `rust-toolchain.toml` |
| 4 | `rustfmt.toml` | `rustfmt.toml` |
| 5 | `clippy.toml` | `clippy.toml` |
| 6 | `deny.toml` | `deny.toml` |
| 7 | `.cargo/config.toml` | `.cargo/config.toml` |
| 13 | `.gitignore` updates | `.gitignore` |

**Hand-off:** Worker A must finish Tasks 1–2 before Workers B/C can start. Tasks 3–7 and 13 are independent and can proceed in parallel with Workers B/C once directories exist.

#### Worker B: "Crate Stubs" (Tasks 8–11)

| Task | Description | Files to Create |
|------|-------------|-----------------|
| 8 | Tier 0 crates | `crates/uml-common/Cargo.toml`, `src/lib.rs`; `crates/uml-core/Cargo.toml`, `src/lib.rs` |
| 9 | Tier 1 crates | 5 crates × 2 files each = 10 files |
| 10 | Tier 2 crates | 9 crates × 2 files each = 18 files |
| 11 | Tier 3 crates | 3 crates × 2-3 files each = 7 files |

**Order constraint:** Must create in tier order (0 → 1 → 2 → 3) because `Cargo.toml` path
dependencies must point to existing directories. However, Cargo does not require the
dependency crate to have a valid `Cargo.toml` at `cargo check` time — it just requires the
directory to exist. So Worker B can write all `Cargo.toml` files in parallel if the
directories already exist (Task 1 from Worker A must be complete).

**Dependency note:** Local path deps must use correct relative paths:
- `crates/*/Cargo.toml`: path deps are `path = "../<depname>"`
- `apps/*/Cargo.toml`: path deps are `path = "../../crates/<depname>"`

#### Worker C: "Tooling & CI" (Tasks 12, 14)

| Task | Description | Files to Create |
|------|-------------|-----------------|
| 12 | xtask crate | `xtask/Cargo.toml`, `xtask/src/main.rs` |
| 14 | CI pipeline | `.github/workflows/ci.yml` |

**Hand-off:** Worker C can start as soon as Worker A creates the directory structure.
No dependencies on crate stubs (CI just runs `cargo build --workspace`, which will fail
if crate stubs don't exist — but Worker C should create the CI file, and verification
happens after Worker B completes).

### 11.2 Communication Protocol

1. **Each worker writes to its own set of files** — no file conflicts between workers.
2. Workers may write in parallel as long as their tasks are independent.
3. After all workers complete, run **Verification Steps** (Section 9) from the workspace root.
4. If verification fails, identify the failing task and reassign to the appropriate worker.
5. Once verification passes, run **Completion Criteria** (Section 10) and obtain sign-off.

### 11.3 File Template for Crate Stubs

Every `Cargo.toml` follows this pattern:

```toml
[package]
name = "uml-<name>"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "<50-word description of the crate's purpose>"

[dependencies]
# Local path deps first
uml-core = { path = "../uml-core" }
# Then workspace deps
serde.workspace = true
thiserror.workspace = true
# Then external non-workspace deps
some_crate = "1.2"

[dev-dependencies]
# Testing deps
serde_json = "1"

[features]
# Feature flags
default = []
```

Every `src/lib.rs` follows this pattern:

```rust
//! <Crate_name> — <one-line description>
//!
//! <2-3 sentence detailed description of the crate's purpose and contents>

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

/// <Module description>
pub mod module_one;
/// <Module description>
pub mod module_two;
// ... etc.
```

### 11.4 Error Recovery

| Symptom | Likely Cause | Fix |
|---------|--------------|-----|
| `error[E0432]: unresolved import` | Module declared in `lib.rs` but no file exists | Create empty module file or remove declaration |
| `error: package `uml-xmi` depends on package `uml-core` which doesn't exist` | Wrong path in path dependency | Check relative path is correct from crate's location |
| `error: failed to select a version for `slotmap`` | Dependency version missing in workspace | Add to `[workspace.dependencies]` in root `Cargo.toml` |
| `error: Edition 2024 is unstable` | Rust toolchain too old | Run `rustup update stable` or switch to `edition = "2021"` |
| `warning: unused import` | Module has no public items yet | Add `#![allow(unused_imports)]` temporarily or suppress with empty re-export |
| `cargo deny: Banned dependency detected` | License not in allowlist | Add license to `deny.toml` or replace dependency |

If `cargo build --workspace` succeeds with warnings (not errors), run:

```bash
# Find all warnings
cargo build --workspace 2>&1 | grep "warning:"
# Fix each warning, then re-verify
```

---

## 12. Rollback Procedure

### 12.1 If CI Fails After Merge

1. **Immediate:** Stop all further merges. File a blocking issue.
2. **Diagnose:** Run verification steps locally. Identify which criterion fails.
3. **Fix:** The fix is always in the configuration files or crate manifests — no domain
   logic is at risk. Push a fix commit directly to `main`.
4. **Verify:** Re-run CI. If green, resume merges.
5. **Post-mortem:** Add a test or check that would have caught the failure earlier.

### 12.2 If Workspace Fails to Build Fresh

```bash
# Step 1: Clean everything
cargo clean
rm -rf target/

# Step 2: Verify toolchain
rustup show
# Should show: stable-<arch>-<os>, components: rustfmt, clippy

# Step 3: Minimal build (start from Tier 0)
cargo build --package uml-common
cargo build --package uml-core
# If these fail, the dependency versions or feature flags are wrong

# Step 4: Build outward by tier
cargo build --package uml-xmi
cargo build --package uml-undo
# ... etc.

# Step 5: If a specific crate fails, check its Cargo.toml in isolation
cd crates/<failing-crate>
cargo check 2>&1 | head -20
```

### 12.3 If All Else Fails

1. `git revert HEAD` to undo the Phase 1 commit.
2. Investigate the root cause (toolchain issue? dependency yanked? new Rust edition bug?).
3. Fix root cause, re-apply Phase 1 changes.
4. Re-verify.

---

*End of Phase 1 Execution Plan*
