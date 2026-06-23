# Umbrello-RS Crate Layout

> **Status:** Draft proposal  
> **Date:** 2026-06-23  
> **Scope:** Workspace structure for the Rust rewrite of the Umbrello UML modeling tool

---

## Table of Contents

1. [Workspace Tree Overview](#1-workspace-tree-overview)
2. [Rationale per Crate](#2-rationale-per-crate)
3. [Dependency Graph](#3-dependency-graph)
4. [Crate-by-Crate Detail](#4-crate-by-crate-detail)
5. [Feature Flags](#5-feature-flags)
6. [Cargo.toml Sketches](#6-cargotoml-sketches)
7. [Boundary Analysis: Why These Crate Boundaries](#7-boundary-analysis)
8. [Build Time Considerations](#8-build-time-considerations)
9. [Plugin Discovery for Language Crates](#9-plugin-discovery-for-language-crates)
10. [Workspace Tooling (xtask)](#10-workspace-tooling-xtask)
11. [Naming Conventions](#11-naming-conventions)
12. [Migration Path](#12-migration-path)

---

## 1. Workspace Tree Overview

```
umbrello-rs/                          # Workspace root
├── Cargo.toml                        # [workspace] manifest
├── rust-toolchain.toml               # Stable Rust + MSRV policy
├── .cargo/config.toml                # Build cache, linker opts
│
├── crates/
│   ├── uml-core/                     # Tier 0 — no workspace deps
│   ├── uml-common/                   # Tier 0 — no workspace deps
│   ├── uml-xmi/                      # Tier 1 — depends on uml-core
│   ├── uml-persistence/              # Tier 2 — depends on uml-core + uml-xmi + uml-undo
│   ├── uml-undo/                     # Tier 1 — depends on uml-core
│   ├── uml-diagram/                  # Tier 1 — depends on uml-core
│   ├── uml-layout/                   # Tier 2 — depends on uml-diagram
│   ├── uml-render/                   # Tier 2 — depends on uml-diagram
│   ├── uml-export/                   # Tier 3 — depends on uml-render
│   ├── uml-import/                   # Tier 1 — depends on uml-core
│   ├── uml-import-cpp/               # Tier 2 — depends on uml-import
│   ├── uml-import-java/              # Tier 2 — depends on uml-import
│   ├── uml-import-python/            # Tier 2 — depends on uml-import
│   ├── uml-codegen/                  # Tier 1 — depends on uml-core
│   ├── uml-codegen-cpp/              # Tier 2 — depends on uml-codegen
│   ├── uml-codegen-java/             # Tier 2 — depends on uml-codegen
│   ├── uml-codegen-python/           # Tier 2 — depends on uml-codegen
│   ├── uml-codegen-rust/             # Tier 2 — depends on uml-codegen
│   └── uml-export/                   # Tier 3 — depends on uml-render
│
├── apps/
│   ├── uml-cli/                      # Tier 3 — leaf binary
│   └── umbrello-desktop/             # Tier 3 — leaf binary
│
├── xtask/                            # Build/dev task runner
├── tests/                            # Integration tests
└── docs/                             # Documentation
```

---

## 2. Rationale per Crate

### 2.1 Why `uml-core` Exists

**Purpose:** Pure UML domain model. Every UML concept (class, interface, association, stereotype, package, etc.) lives here as plain Rust structs with no I/O, no GUI, no persistence knowledge.

**Why separate from everything else:**
- The C++ codebase's biggest architectural debt is that `UMLObject` depends on `UMLApp`, `UMLDoc`, `UMLScene`, and Qt. By making the model pure data, we eliminate all cycles.
- 79 source files in the C++ `umlmodel/` directory define ~25 distinct entity types. Housing them in one crate keeps the dependency graph flat (everything can reference everything within the crate) without leaking model details upward.
- `uml-core` must compile in under 10 seconds. It has minimal dependencies (`slotmap`, `uuid`, `bitflags`, `thiserror`, `serde`). This is the foundation everything else builds on.

**What would happen if we merged `uml-core` into `uml-diagram`?**
The diagram crate would need to be recompiled every time a model field changes, even though most changes to code generators, importers, and serialization don't touch diagram logic. Separation gives independent compile units.

**What would happen if we split it further (e.g., `uml-types`, `uml-ids`, `uml-model`)?**
The C++ codebase has 12 separate list-type headers that are really just `QList<SpecificType*>` — a sign they don't need to be separate. Keeping types, IDs, and model objects in one crate avoids that proliferation. The crate is small enough (< 5k lines) that splitting adds more build overhead (cargo metadata, inter-crate dependency resolution) than it saves.

### 2.2 Why `uml-common` Exists

**Purpose:** Shared utilities that don't belong in any domain crate: error types, logging setup, path utilities, version constants.

**Why separate from `uml-core`:**
- Error types used by all crates (e.g., `UmbrelloError`, `FileError`, `ParseError`) must not pull in model types. A crate can depend on `uml-common` without depending on `uml-core`.
- Logging setup (`tracing` subscriber initialization) is used by binaries but not by library crates.
- Keeps `uml-core` focused on domain logic, not infrastructure.

### 2.3 Why `uml-xmi` Exists

**Purpose:** XMI format reading and writing — the sole serialization format for Umbrello model files.

**Why separate from persistence:**
- XMI is a specific interchange format. The persistence layer may handle multiple backends (XMI, foreign formats like Rose `.mdl`, future JSON). XMI is complex enough (two versions, streaming vs DOM, DTDs, forward-reference resolution) to warrant its own crate.
- `uml-xmi` depends on `uml-core` (for model types) and `quick-xml` (for XML parsing). Isolating it means changes to the file format don't trigger recompilation of diagram, codegen, or GUI crates.

**Why separate from `uml-core`:**
- Serialization logic should not be baked into domain objects. The C++ pattern of `saveToXMI()` / `loadFromXMI()` on every class couples format to model. Here, `uml-xmi` implements `Serialize`/`Deserialize` on types imported from `uml-core`.

### 2.4 Why `uml-persistence` Exists

**Purpose:** The load/save pipeline: opening files, detecting format, dispatching to the correct reader/writer, handling compression (`.tgz`, `.tar.bz2`), autosave, and file locking.

**Why separate from `uml-xmi`:**
- Persistence is about *storage* (files, compression, format detection), not about *format* (how model data maps to XML). These are separate concerns.
- `uml-persistence` knows about XMI, but also about foreign formats (future Rose `.mdl`, ArgoUML `.zargo`). A single `StorageBackend` trait allows adding new formats without changing the load/save pipeline.

### 2.5 Why `uml-undo` Exists

**Purpose:** Command-pattern undo/redo stack, entirely model-aware but GUI-neutral.

**Why separate from `uml-core`:**
- The C++ codebase has 20+ command types. They reference both model objects and widgets. In Rust, commands should be pure operations on model state — no widget references.
- Separating undo into its own crate means it can be compiled independently and tested without loading any GUI libraries.

### 2.6 Why `uml-diagram` Exists

**Purpose:** The diagram model — widget data (position, size, visual properties), scene composition (which widgets exist, how they connect), without any rendering code.

**Why separate from `uml-render`:**
- This is the classic model/view separation. `uml-diagram` is pure data structures (widget trees, scene lists, association routes). `uml-render` knows how to paint them.
- CLI tooling (export, validation) needs to work with diagrams without needing a GPU or window system. `uml-diagram` has no rendering dependency.

### 2.7 Why `uml-layout` Exists

**Purpose:** Auto-layout algorithms: graph-based layout (Graphviz integration via `petgraph` or `graphviz-rust`), force-directed layout, grid snapping, alignment guides.

**Why separate from `uml-diagram`:**
- Layout algorithms are computationally intensive and have heavy dependencies (`petgraph`). They should be optional — a user may want to arrange widgets manually.
- Separating layout means we can feature-gate it (`feature = "auto-layout"`) without affecting the base diagram types.

### 2.8 Why `uml-render` Exists

**Purpose:** Diagram rendering to screen or offscreen surface. Uses `vello` / `wgpu` for GPU-accelerated canvas, `cosmic-text` for text layout.

**Why separate from `uml-export`:**
- Rendering is interactive (real-time canvas updates, viewport scrolling, zoom). Export is batch (write to file). They share drawing logic but have different performance characteristics.
- Export depends on rendering, not the other way around.

### 2.9 Why `uml-codegen` (framework) Exists

**Purpose:** The abstract code generation framework — `CodeGenerator` trait, `CodeWriter` helper, `GeneratorRegistry`, and common configuration types. No language-specific generators.

**Why separate from language generators:**
- This is the plugin interface. The framework crate has minimal dependencies. Adding a new language means creating a new crate that depends on `uml-codegen` — no modification to existing code.
- Eliminates the C++ `CodeGenFactory` switch-statement anti-pattern.

### 2.10 Why Language-Specific Crates Exist

**Purpose:** One crate per supported code generation language. Each implements `CodeGenerator` from `uml-codegen`.

**Rationale for one crate per language:**
- 22 languages in C++ means ~22 crates. This is manageable with workspace patterns.
- Users who only need C++ generation don't compile Java, Python, Ruby generators.
- Feature flags gate each language: `codegen-cpp = ["uml-codegen-cpp"]`.
- New community-contributed generators can be published as standalone crates and registered at runtime.

**Why not a single `uml-codegen-impls` crate?**
- Compile time: building all 22 generators together would be slow.
- Coupling: a bug in one generator shouldn't force recompilation of all others.
- Open-closed principle: adding a language means adding a file in an existing crate (violation) vs. adding a whole crate (clean extension).

### 2.11 Why `uml-import` (framework) Exists

**Purpose:** Abstract import framework — `LanguageImporter` trait, `ImportRegistry`, and common utilities for mapping parsed constructs to UML model objects. Mirror of `uml-codegen`.

### 2.12 Why Language-Specific Import Crates Exist

**Purpose:** One crate per import source language. Same rationale as codegen crates.

### 2.13 Why `uml-export` Exists

**Purpose:** Diagram export to image formats (SVG, PNG, PDF). Depends on `uml-render` to produce frames, then encodes to the target format.

**Why separate from `uml-render`:**
- Export is a leaf use case. The desktop app needs `uml-render` for interactive display, but a CLI tool only needs `uml-export` for batch export.
- Export dependencies (`resvg`, `image`, `pdf`) are heavy and should be optional.

### 2.14 Why `uml-cli` Exists

**Purpose:** Command-line interface for headless operations: export, import, validate, list languages/formats.

**Why separate from `umbrello-desktop`:**
- Single responsibility: the CLI binary should not depend on egui, wgpu, or any GUI framework.
- Package managers: Linux distros may want `umbrello-cli` without the full GUI.
- CI/CD: CLI-only testing avoids needing a display server.

### 2.15 Why `umbrello-desktop` Exists

**Purpose:** The full desktop GUI application. Owns `AppContext`, window management, panel layout, dialog lifecycle.

**Why separate from library crates:**
- The desktop app is one consumer of the library crates. Other consumers (CLI, CI scripts, IDE plugins) should not depend on egui or window management.
- The GUI framework choice (egui) is an implementation detail of this binary.

---

## 3. Dependency Graph

### 3.1 Text-Based Graph

```
Legend:
  ───→  "depends on" (direction of arrow)
  ~●~   optional dependency (feature-gated)

                             ┌──────────────┐
                             │  umbrello-    │
                             │  desktop      │  (binary)
                             └──┬───┬───┬───┘
                                │   │   │
                    ┌───────────┘   │   └──────────────────┐
                    ▼               ▼                      ▼
              ┌───────────┐  ┌───────────┐         ┌──────────────┐
              │  uml-cli   │  │ uml-export│         │  uml-render   │
              │  (binary)  │  │           │         │              │
              └─────┬─────┘  └─────┬─────┘         └──────┬───────┘
                    │              │                      │
                    ▼              │                      ▼
              ┌───────────┐       │                 ┌──────────────┐
              │ uml-      │       │                 │ uml-diagram   │
              │persistence │      │                 │              │
              └──┬───┬────┘      │                 └──────┬───────┘
                 │   │           │                        │
    ┌────────────┘   │           │                        │
    ▼                │           │                        │
┌─────────┐          │           │                   ┌─────────┐
│  uml-xmi │         │           │                   │ uml-core │
└────┬────┘          │           │                   └─────────┘
     │               │           │                        ▲
     │     ┌─────────┘           │                        │
     │     ▼                     │           ┌────────────┴────────────┐
     │  ┌───────┐                │           │                         │
     │  │uml-   │                │    ┌────────────┐            ┌──────────┐
     │  │undo   │                │    │ uml-codegen│            │ uml-     │
     │  └───────┘                │    └──────┬─────┘            │ import   │
     │                           │           │                  └────┬─────┘
     │                           │     ┌─────┼──────┐               │
     │                           │     │     │      │         ┌─────┼──────┐
     │                           │     ▼     ▼      ▼         ▼     │      ▼
     │                           │  ┌────┐ ┌────┐ ┌────┐ ┌────┐    │   ┌────┐
     │                           │  │cpp │ │java│ │py  │ │rust│ │cpp │   │java│
     │                           │  └────┘ └────┘ └────┘ └────┘ └────┘   └────┘
     │                           │   ~●~   ~●~   ~●~   ~●~   ~●~     ~●~
     ▼                           ▼
  ┌────────┐              ┌───────────┐
  │ uml-   │              │ uml-layout │
  │ common │              └─────┬─────┘
  └────────┘                    │
                           ┌────▼────┐
                           │uml-     │
                           │diagram  │
                           └─────────┘
```

### 3.2 Dependency Table (direction: row → column)

| Crate | Depends on | Optionally |
|-------|-----------|-----------|
| `uml-core` | `uml-common` | — |
| `uml-common` | — | — |
| `uml-xmi` | `uml-core` | — |
| `uml-undo` | `uml-core` | — |
| `uml-diagram` | `uml-core` | — |
| `uml-layout` | `uml-diagram` | — |
| `uml-render` | `uml-diagram`, `uml-core` | — |
| `uml-persistence` | `uml-core`, `uml-xmi`, `uml-undo` | — |
| `uml-codegen` | `uml-core` | — |
| `uml-codegen-cpp` | `uml-codegen` | — |
| `uml-codegen-java` | `uml-codegen` | — |
| `uml-codegen-python` | `uml-codegen` | — |
| `uml-codegen-rust` | `uml-codegen` | — |
| `uml-import` | `uml-core` | — |
| `uml-import-cpp` | `uml-import` | — |
| `uml-import-java` | `uml-import` | — |
| `uml-import-python` | `uml-import` | — |
| `uml-export` | `uml-render` | — |
| `uml-cli` | `uml-persistence`, `uml-export`, `uml-common` | codegen/import crates |
| `umbrello-desktop` | `uml-core`, `uml-diagram`, `uml-render`, `uml-persistence`, `uml-undo`, `uml-common`, `uml-layout` | codegen/import crates |

### 3.3 Dependency Rules (Enforced by Cargo)

```
Tier 0 (no workspace deps):    uml-common
Tier 0.5:                      uml-core (depends on uml-common)
Tier 1 (depends on core only):  uml-xmi, uml-undo, uml-diagram, uml-codegen, uml-import
Tier 2 (depends on tier 1):    uml-persistence, uml-render, uml-layout, *-codegen-*, *-import-*
Tier 3 (applications):         uml-cli, umbrello-desktop, uml-export
```

**Rule:** A crate may only depend on crates from the same or lower tier. No reverse dependencies (e.g., `uml-core` must never depend on `uml-render`).

---

## 4. Crate-by-Crate Detail

### 4.1 `uml-common` — Shared Utilities

**Purpose:** Foundational types used across all crates, with zero domain knowledge.

**Public API sketch:**

```rust
// lib.rs — re-exports
pub mod error;
pub mod logging;
pub mod version;

pub use error::*;
pub use logging::*;
pub use version::*;
```

**Key types:**

```rust
/// Umbrello-wide error enum. All crates produce these or more-specific errors
/// that implement `Into<UmbrelloError>`.
#[derive(Debug, thiserror::Error)]
pub enum UmbrelloError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),

    #[error("Duplicate ID: {0}")]
    DuplicateId(String),
}

/// Version constants matching C++ XMI_FILE_VERSION.
pub mod version {
    pub const UMBRELLO_VERSION: &str = "1.0.0";
    pub const XMI1_FILE_VERSION: &str = "1.7.6";
    pub const XMI2_FILE_VERSION: &str = "2.0.4";
    pub const UMBRELLO_VERSION_MAJOR: u32 = 1;
    pub const UMBRELLO_VERSION_MINOR: u32 = 0;
    pub const UMBRELLO_VERSION_PATCH: u32 = 0;
}

/// Logging setup using `tracing`.
pub fn init_logging(filter: &str) -> Result<(), UmbrelloError>;
pub fn init_logging_with_file(filter: &str, path: &Path) -> Result<(), UmbrelloError>;
```

**Dependencies:** `tracing`, `tracing-subscriber`, `thiserror`, `serde`.

**Not in this crate:** Any domain-specific types (no `UmlId`, no `ModelEvent`, no `ObjectType`). Those belong in `uml-core`.

---

### 4.2 `uml-core` — UML Domain Model

**Purpose:** All UML model types, the model repository (arena-based storage), type enums, ID types, stereotype management, association model. Pure data — no I/O, no GUI, no persistence.

**Public API sketch:**

```rust
// lib.rs — flattened re-exports
pub mod model;
pub mod types;
pub mod id;
pub mod repository;
pub mod event;
pub mod traits;

// Re-export key types at crate root for ergonomics
pub use id::{UmlId, ObjectKey, SceneId, DiagramId};
pub use types::*;
pub use model::*;
pub use repository::ModelRepository;
pub use event::ModelEvent;
pub use traits::*;
```

**Key types:**

```rust
// === ID types ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UmlId(uuid::Uuid);  // Universally unique, no collisions

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObjectKey(slotmap::DefaultKey);  // Arena index, cheap

// === ObjectType (clean Rust enum, not magic numbers) ===
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ObjectType {
    // Structural classifiers
    Class,
    Interface,
    Enumeration,
    Datatype,
    Entity,

    // Containers
    Package,
    Folder,
    Component,
    Artifact,

    // Leaf diagram nodes
    Actor,
    UseCase,
    Node,
    Port,
    Category,
    Instance,

    // Classifier children
    Attribute,
    Operation,
    Template,
    EnumLiteral,
    EntityAttribute,

    // Constraints
    UniqueConstraint,
    ForeignKeyConstraint,
    CheckConstraint,

    // Relationships
    Association,
    Role,

    // Infrastructure
    Stereotype,
    InstanceAttribute,
}

// === Main model enum (replaces 30-some C++ classes) ===
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UmlModelElement {
    Class(Box<UmlClass>),
    Interface(Box<UmlInterface>),
    Enumeration(Box<UmlEnumeration>),
    Datatype(Box<UmlDatatype>),
    Entity(Box<UmlEntity>),
    Package(Box<UmlPackage>),
    Folder(Box<UmlFolder>),
    Component(Box<UmlComponent>),
    Artifact(Box<UmlArtifact>),
    Actor(Box<UmlActor>),
    UseCase(Box<UmlUseCase>),
    Node(Box<UmlNode>),
    Port(Box<UmlPort>),
    Category(Box<UmlCategory>),
    Instance(Box<UmlInstance>),
    Attribute(Box<UmlAttribute>),
    Operation(Box<UmlOperation>),
    Template(Box<UmlTemplate>),
    EnumLiteral(Box<UmlEnumLiteral>),
    EntityAttribute(Box<UmlEntityAttribute>),
    UniqueConstraint(Box<UmlUniqueConstraint>),
    ForeignKeyConstraint(Box<UmlForeignKeyConstraint>),
    CheckConstraint(Box<UmlCheckConstraint>),
    Association(Box<UmlAssociation>),
    Role(Box<UmlRole>),
    Stereotype(Box<UmlStereotypeData>),
    InstanceAttribute(Box<UmlInstanceAttribute>),
}

// === Core traits ===
pub trait Identifiable {
    fn key(&self) -> ObjectKey;
    fn id(&self) -> UmlId;
}

pub trait Named {
    fn name(&self) -> &str;
    fn set_name(&mut self, name: String);
}

pub trait HasStereotype {
    fn stereotype(&self) -> Option<ObjectKey>;
}

// === Arena-based repository ===
pub struct ModelRepository {
    objects: SlotMap<ObjectKey, UmlModelElement>,
    stereotypes: SlotMap<ObjectKey, UmlStereotypeData>,
    associations: Vec<ObjectKey>,  // indexes into objects for Association variants
}

impl ModelRepository {
    pub fn new() -> Self;
    pub fn insert(&mut self, element: UmlModelElement) -> ObjectKey;
    pub fn get(&self, key: ObjectKey) -> Option<&UmlModelElement>;
    pub fn get_mut(&mut self, key: ObjectKey) -> Option<&mut UmlModelElement>;
    pub fn remove(&mut self, key: ObjectKey) -> Option<UmlModelElement>;
    pub fn find_by_id(&self, id: UmlId) -> Option<ObjectKey>;
    pub fn associations_for(&self, key: ObjectKey) -> Vec<&UmlAssociation>;

    // Bulk operations
    pub fn import_batch(&mut self, elements: Vec<UmlModelElement>) -> Vec<ObjectKey>;
}
```

**Dependencies:** `uml-common`, `slotmap`, `uuid`, `bitflags`, `serde` (with `derive`), `thiserror`.

---

### 4.3 `uml-xmi` — XMI Serialization

**Purpose:** Read and write XMI files in UML 1.2 and UML 2.1 formats. Streaming writer for output, event-based reader for input. Handles two-pass loading (forward reference resolution).

**Public API sketch:**

```rust
// lib.rs
pub mod reader;
pub mod writer;
pub mod v1_2;
pub mod v2_1;
pub mod error;

pub use error::XmiError;

/// Write a model repository to XMI.
pub struct XmiWriter<'a, W: std::io::Write> {
    inner: quick_xml::Writer<W>,
    repo: &'a ModelRepository,
    version: XmiVersion,
}

impl<'a, W: std::io::Write> XmiWriter<'a, W> {
    pub fn new(writer: W, repo: &'a ModelRepository, version: XmiVersion) -> Self;
    pub fn write_document(&mut self) -> Result<(), XmiError>;
}

/// Read model from XMI into a repository.
pub struct XmiReader<'a> {
    repo: &'a mut ModelRepository,
    unresolved: Vec<UnresolvedRef>,
}

impl<'a> XmiReader<'a> {
    pub fn new(repo: &'a mut ModelRepository) -> Self;
    pub fn read_document<R: std::io::Read>(
        &mut self,
        reader: R,
        version_hint: Option<XmiVersion>,
    ) -> Result<(), XmiError>;
    pub fn resolve_references(&mut self) -> Result<(), XmiError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmiVersion {
    V1_2,  // UML 1.2 (default, legacy)
    V2_1,  // UML 2.1 (alternative)
}
```

**Dependencies:** `uml-core`, `quick-xml` (streaming, not DOM), `serde` (for optional derives on helper types), `thiserror`.

---

### 4.4 `uml-persistence` — File I/O Pipeline

**Purpose:** File format detection, compression handling (`.tgz`, `.tar.bz2`), autosave, `StorageBackend` trait for multiple storage backends.

**Public API sketch:**

```rust
// lib.rs
pub mod storage;
pub mod xmi_storage;
pub mod compression;
pub mod error;

pub use storage::StorageBackend;
pub use xmi_storage::XmiStorage;
pub use error::PersistenceError;

/// Detected file format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    XmiPlain,
    XmiGzip,     // .xmi.tgz
    XmiBzip2,    // .xmi.tar.bz2
    Zargo,       // legacy ArgoUML ZIP
    RoseModel,   // Rational Rose .mdl (future)
}

impl FileFormat {
    pub fn detect(path: &Path) -> Option<FileFormat>;
}

/// Trait for storage backends (XMI, Rose, future formats).
#[async_trait]
pub trait StorageBackend {
    async fn load(&self, path: &Path, repo: &mut ModelRepository) -> Result<(), PersistenceError>;
    async fn save(&self, path: &Path, repo: &ModelRepository) -> Result<(), PersistenceError>;
}

/// Autosave manager.
pub struct AutosaveManager {
    interval: Duration,
    path: PathBuf,
    timer: Option<tokio::time::Interval>,
}

impl AutosaveManager {
    pub fn new(path: PathBuf, interval: Duration) -> Self;
    pub async fn autosave_loop(&mut self, repo: &ModelRepository) -> Result<(), PersistenceError>;
}
```

**Dependencies:** `uml-core`, `uml-xmi`, `uml-undo`, `tokio` (async file I/O), `flate2`, `tar`, `bzip2`, `zip`, `thiserror`, `tracing`.

---

### 4.5 `uml-undo` — Undo/Redo System

**Purpose:** Command-pattern undo/redo stack with fine-grained model operations. No GUI dependencies.

**Public API sketch:**

```rust
// lib.rs
pub mod command;
pub mod stack;
pub mod commands;

pub use command::UndoCommand;
pub use stack::UndoStack;

/// Core undoable command trait.
pub trait UndoCommand: std::fmt::Debug + Send {
    fn execute(&mut self, repo: &mut ModelRepository) -> Result<(), UndoError>;
    fn undo(&mut self, repo: &mut ModelRepository) -> Result<(), UndoError>;
    fn merge(&self, other: &dyn UndoCommand) -> Option<Box<dyn UndoCommand>>;
    fn name(&self) -> &str;
}

/// Undo stack with bounded capacity.
pub struct UndoStack {
    undo_commands: Vec<Box<dyn UndoCommand>>,
    redo_commands: Vec<Box<dyn UndoCommand>>,
    max_depth: usize,
    disabled: bool,  // true during file load
}

impl UndoStack {
    pub fn new(max_depth: usize) -> Self;
    pub fn push(&mut self, cmd: Box<dyn UndoCommand>);
    pub fn undo(&mut self, repo: &mut ModelRepository) -> Result<(), UndoError>;
    pub fn redo(&mut self, repo: &mut ModelRepository) -> Result<(), UndoError>;
    pub fn begin_macro(&mut self, name: &str);
    pub fn end_macro(&mut self, repo: &mut ModelRepository) -> Result<(), UndoError>;
    pub fn set_disabled(&mut self, disabled: bool);
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
}

// Concrete commands in `commands` module:
pub struct CmdCreateObject { /* ... */ }
pub struct CmdRemoveObject { /* ... */ }
pub struct CmdRenameObject { /* ... */ }
pub struct CmdSetVisibility { /* ... */ }
pub struct CmdSetStereotype { /* ... */ }
// ... etc., mapping to the 20+ C++ command types but without widget references
```

**Dependencies:** `uml-core`, `thiserror`.

---

### 4.6 `uml-diagram` — Diagram Model

**Purpose:** Data structures for diagram composition: widget trees, scene state, association routing data. No rendering.

**Public API sketch:**

```rust
// lib.rs
pub mod types;
pub mod scene;
pub mod widgets;
pub mod associations;
pub mod factory;

/// Diagram type (matches the C++ DiagramType enum).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramType {
    Class,
    UseCase,
    Sequence,
    Collaboration,
    State,
    Activity,
    Component,
    Deployment,
    EntityRelationship,
    Object,
}

/// Diagram-level metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagramMetadata {
    pub id: DiagramId,
    pub name: String,
    pub diagram_type: DiagramType,
    pub zoom: f64,
    pub snap_to_grid: bool,
    pub grid_x: f64,
    pub grid_y: f64,
}

/// Scene — pure data container for widgets and associations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneData {
    pub metadata: DiagramMetadata,
    pub widgets: Vec<WidgetData>,
    pub associations: Vec<EdgeData>,
}

/// Position, size, and visual properties of a widget.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetData {
    pub id: WidgetId,
    pub widget_type: WidgetType,
    pub associated_model_key: Option<ObjectKey>,
    pub position: Point,
    pub size: Size,
    pub z_order: i32,
    pub color: Color,
    pub font: FontProperties,
}

/// Association edge data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub id: EdgeId,
    pub association_type: AssociationType,
    pub source_widget: WidgetId,
    pub target_widget: WidgetId,
    pub line_type: LineRouting,
    pub labels: Vec<LabelData>,
}
```

**Dependencies:** `uml-core`, `serde`, `thiserror`.

---

### 4.7 `uml-layout` — Layout Algorithms

**Purpose:** Automatic arrangement of diagram widgets.

**Public API sketch:**

```rust
// lib.rs
pub mod graph;
pub mod force;
pub mod grid;
pub mod alignment;

pub use graph::GraphLayout;
pub use force::ForceDirectedLayout;
pub use grid::GridSnapper;
pub use alignment::AlignmentGuides;

/// Auto-layout trait.
pub trait LayoutEngine {
    fn layout(&self, scene: &mut SceneData) -> Result<(), LayoutError>;
}

/// Graphviz-based hierarchical layout.
pub struct GraphLayout {
    graphviz_path: Option<PathBuf>,
}

impl LayoutEngine for GraphLayout {
    fn layout(&self, scene: &mut SceneData) -> Result<(), LayoutError> {
        // 1. Build petgraph from scene widgets/associations
        // 2. Call dot binary (or use pure-Rust graph layout)
        // 3. Apply computed positions to widgets
    }
}
```

**Dependencies:** `uml-diagram`, `petgraph` (optional, for graph algorithm), `serde`.

---

### 4.8 `uml-render` — Diagram Rendering

**Purpose:** GPU-accelerated canvas for interactive diagram display. Text layout via `cosmic-text`.

**Public API sketch:**

```rust
// lib.rs
pub mod canvas;
pub mod render;
pub mod text;
pub mod line;
pub mod interaction;

pub use canvas::Canvas;

/// Rendering trait — implement per-widget-type.
pub trait WidgetRenderer {
    fn draw(&self, scene: &SceneData, widget: &WidgetData, canvas: &mut dyn Canvas);
}

/// Canvas abstraction (backed by vello/wgpu).
pub trait Canvas {
    fn draw_rect(&mut self, rect: Rect, fill: Color, stroke: Stroke);
    fn draw_text(&mut self, text: &str, font: &FontProperties, pos: Point);
    fn draw_line(&mut self, from: Point, to: Point, style: LineStyle);
    fn draw_polyline(&mut self, points: &[Point], style: LineStyle);
    fn set_clip(&mut self, rect: Rect);
    fn clear_clip(&mut self);
}
```

**Dependencies:** `uml-diagram`, `uml-core`, `vello` / `wgpu` (GPU canvas), `cosmic-text` (text layout), `color-eyre` (error handling), `tracing`.

---

### 4.9 `uml-codegen` — Code Generation Framework

**Purpose:** Abstract trait and registry for code generators. Shared types for configuration and output.

**Public API sketch:**

```rust
// lib.rs
pub mod generator;
pub mod registry;
pub mod writer;
pub mod config;

pub use generator::CodeGenerator;
pub use registry::GeneratorRegistry;
pub use writer::CodeWriter;
pub use config::CodeGenConfig;

/// Language identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProgrammingLanguage {
    Ada,
    ActionScript,
    Cpp,
    CSharp,
    D,
    IDL,
    Java,
    JavaScript,
    Pascal,
    Perl,
    Php4,
    Php5,
    Python,
    Ruby,
    Rust,
    Sql,
    MySql,
    PostgreSql,
    Tcl,
    Vala,
    XmlSchema,
}

/// A code generator for one programming language.
#[async_trait]
pub trait CodeGenerator: Send + Sync {
    fn language(&self) -> ProgrammingLanguage;
    fn file_extension(&self) -> &str;

    /// Generate code for a single classifier.
    async fn generate(
        &self,
        classifier_key: ObjectKey,
        repo: &ModelRepository,
        config: &CodeGenConfig,
    ) -> Result<GeneratedFile, CodeGenError>;

    /// Generate code for all classifiers in a set.
    async fn generate_all(
        &self,
        keys: &[ObjectKey],
        repo: &ModelRepository,
        config: &CodeGenConfig,
    ) -> Result<Vec<GeneratedFile>, CodeGenError>;
}

#[derive(Debug, Clone)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub content: String,
    pub language: ProgrammingLanguage,
}

/// Registry with language → generator mapping.
pub struct GeneratorRegistry {
    generators: HashMap<ProgrammingLanguage, Box<dyn CodeGenerator>>,
}

impl GeneratorRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, generator: Box<dyn CodeGenerator>);
    pub fn get(&self, lang: ProgrammingLanguage) -> Option<&dyn CodeGenerator>;
    pub fn all_languages(&self) -> Vec<ProgrammingLanguage>;
}

/// Helper for indentation-aware code output.
pub struct CodeWriter {
    content: String,
    indent_level: usize,
    indent_string: &'static str,
    line_width: usize,
}

impl CodeWriter {
    pub fn new(indent_string: &'static str, line_width: usize) -> Self;
    pub fn indent(&mut self);
    pub fn dedent(&mut self);
    pub fn write(&mut self, line: &str);
    pub fn write_fmt(&mut self, fmt: std::fmt::Arguments<'_>);
    pub fn newline(&mut self);
    pub fn into_string(self) -> String;
}
```

**Dependencies:** `uml-core`, `async-trait`, `serde`, `thiserror`, `tracing`.

---

### 4.10 Language-Specific Codegen Crates

**Purpose:** Concrete implementations of `CodeGenerator`.

**Sketch (for `uml-codegen-cpp`):**

```rust
// uml-codegen-cpp/src/lib.rs
use uml_codegen::{CodeGenerator, CodeGenConfig, CodeGenError, GeneratedFile, ProgrammingLanguage};

pub struct CppGenerator;

impl CodeGenerator for CppGenerator {
    fn language(&self) -> ProgrammingLanguage { ProgrammingLanguage::Cpp }
    fn file_extension(&self) -> &str { "cpp" }

    async fn generate(
        &self,
        classifier_key: ObjectKey,
        repo: &ModelRepository,
        config: &CodeGenConfig,
    ) -> Result<GeneratedFile, CodeGenError> {
        // Transform UML model into C++ source text
    }
}
```

**Dependencies:** `uml-codegen`, `uml-core`. No additional runtime deps unless needed for templates.

---

### 4.11 `uml-import` — Code Import Framework

**Purpose:** Abstract trait and registry for language importers.

**Public API sketch:**

```rust
// lib.rs
pub mod importer;
pub mod registry;
pub mod utils;

pub use importer::LanguageImporter;
pub use registry::ImportRegistry;

/// An importer that reads source files and produces UML model objects.
#[async_trait]
pub trait LanguageImporter: Send + Sync {
    fn language(&self) -> ProgrammingLanguage;
    fn file_extensions(&self) -> &[&str];

    /// Import a single source file into the repository.
    async fn import_file(
        &self,
        path: &Path,
        repo: &mut ModelRepository,
    ) -> Result<Vec<ObjectKey>, ImportError>;

    /// Import multiple files.
    async fn import_files(
        &self,
        paths: &[PathBuf],
        repo: &mut ModelRepository,
    ) -> Result<Vec<ObjectKey>, ImportError>;
}

/// Registry with file extension → importer mapping.
pub struct ImportRegistry {
    importers_by_ext: HashMap<String, Box<dyn LanguageImporter>>,
}

impl ImportRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, importer: Box<dyn LanguageImporter>);
    pub fn importer_for(&self, path: &Path) -> Option<&dyn LanguageImporter>;
}
```

**Dependencies:** `uml-core`, `async-trait`, `tree-sitter` (shared parsing infrastructure), `thiserror`, `tracing`.

---

### 4.12 Language-Specific Import Crates

**Purpose:** Concrete importers using tree-sitter grammars or custom parsers.

**Sketch (for `uml-import-cpp`):**

```rust
// uml-import-cpp/src/lib.rs
pub struct CppImporter;

impl LanguageImporter for CppImporter {
    fn language(&self) -> ProgrammingLanguage { ProgrammingLanguage::Cpp }
    fn file_extensions(&self) -> &[&str] { &["cpp", "h", "hpp", "cxx", "hxx", "cc", "hh"] }

    async fn import_file(
        &self,
        path: &Path,
        repo: &mut ModelRepository,
    ) -> Result<Vec<ObjectKey>, ImportError> {
        // 1. Parse with tree-sitter-cpp
        // 2. Walk CST, build UML model elements
        // 3. Insert into repo, return created keys
    }
}
```

**Dependencies:** `uml-import`, `uml-core`, `tree-sitter`, `tree-sitter-cpp` (or per-language grammar crate).

---

### 4.13 `uml-export` — Image Export

**Purpose:** Render diagrams to image files (SVG, PNG, PDF).

**Public API sketch:**

```rust
pub enum ExportFormat {
    Svg,
    Png(u32),  // resolution in DPI
    Pdf,
    Svgz,      // compressed SVG
}

pub struct Exporter;

impl Exporter {
    pub fn export(
        scene: &SceneData,
        format: ExportFormat,
        output: &Path,
    ) -> Result<(), ExportError>;
}
```

**Dependencies:** `uml-render`, `resvg` (for rasterization), `image` (for PNG encoding), `pdf` (crate TBD).

---

### 4.14 `uml-cli` — CLI Binary

**Purpose:** Headless command-line operations.

**Public API:**

```rust
// main.rs — thin dispatch to subcommands
#[derive(Parser)]
enum Cli {
    Export { file: PathBuf, format: String, output: Option<PathBuf> },
    Import { files: Vec<PathBuf>, output: PathBuf },
    Validate { file: PathBuf },
    Languages,
    Formats,
}

// lib.rs — shared CLI logic, testable
pub mod export;
pub mod import;
pub mod validate;

pub async fn run_export(file: PathBuf, format: String, output: Option<PathBuf>) -> Result<()>;
pub async fn run_import(files: Vec<PathBuf>, output: PathBuf) -> Result<()>;
pub async fn run_validate(file: PathBuf) -> Result<ValidationReport>;
pub fn list_languages() -> Vec<ProgrammingLanguage>;
pub fn list_formats() -> Vec<String>;
```

**Dependencies:** `clap` (with `derive`), `uml-persistence`, `uml-export`, `uml-common`, `tokio`. Optionally: language-specific codegen/import crates (feature-gated).

---

### 4.15 `umbrello-desktop` — GUI Application

**Purpose:** Full desktop application using egui/eframe.

**Public API sketch:**

```rust
// app.rs — Application state (replaces `UMLApp`)
pub struct UmbrelloApp {
    // Core state
    model_repo: ModelRepository,
    undo_stack: UndoStack,
    settings: Settings,
    event_bus: EventBus,

    // UI state
    open_diagrams: Vec<DiagramHandle>,
    active_diagram: Option<DiagramId>,
    panels: PanelState,
    dialogs: DialogState,

    // Registry
    codegen_registry: GeneratorRegistry,
    import_registry: ImportRegistry,

    // Rendering
    renderer: DiagramRenderer,
}

impl eframe::App for UmbrelloApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Layout panels, delegate to sub-views
        self.render_menu_bar(ctx);
        self.render_panels(ctx);
        self.render_diagram_canvas(ctx);
        self.render_dialogs(ctx);
    }
}
```

**Submodules:**

| Module | Purpose | Files |
|--------|---------|-------|
| `app.rs` | Application state, event loop | 1 |
| `windows/` | Top-level window management | 3-5 |
| `panels/` | Dock widgets: model tree, properties, etc. | 5-8 |
| `dialogs/` | Modal dialogs: class props, settings, etc. | 8-12 |
| `menus/` | Menu bar + context menu definitions | 2-3 |
| `canvas/` | egui canvas integration with `uml-render` | 2-3 |
| `i18n/` | Localization via `rust-i18n` or `fluent` | 1-2 |

**Dependencies:** `egui`, `eframe`, `uml-core`, `uml-diagram`, `uml-render`, `uml-persistence`, `uml-undo`, `uml-common`, `uml-layout`, `uml-codegen`, `uml-import`. Feature-gated optional deps.

---

## 5. Feature Flags

### 5.1 Crate-Level Features

| Feature | Crate(s) | Effect |
|---------|----------|--------|
| `xmi-v1` | `uml-xmi` | Enable XMI 1.2 support (default: on) |
| `xmi-v2` | `uml-xmi` | Enable XMI 2.1 support (default: on) |
| `auto-layout` | `uml-layout` | Enable graph-based layout algorithms (default: on) |
| `force-layout` | `uml-layout` | Enable force-directed layout (default: off) |
| `png-export` | `uml-export` | Enable PNG export |
| `svg-export` | `uml-export` | Enable SVG export (default: on) |
| `pdf-export` | `uml-export` | Enable PDF export |
| `autosave` | `uml-persistence` | Enable autosave loop (default: on) |
| `compression` | `uml-persistence` | Enable gzip/bzip2 support (default: on) |
| `gui` | `umbrello-desktop` | Enable GUI (always on for desktop binary) |

### 5.2 Language Feature Flags

Each language-specific codegen and import crate is feature-gated. The workspace `Cargo.toml` defines a feature for each:

```toml
[features]
default = [
    "codegen-cpp",
    "codegen-java",
    "codegen-python",
    "codegen-rust",
    "import-cpp",
    "import-java",
    "import-python",
]

# Code generators
codegen-cpp = ["uml-codegen-cpp"]
codegen-java = ["uml-codegen-java"]
codegen-python = ["uml-codegen-python"]
codegen-rust = ["uml-codegen-rust"]
codegen-ada = ["uml-codegen-ada"]
codegen-js = ["uml-codegen-js"]
codegen-cs = ["uml-codegen-cs"]
codegen-php = ["uml-codegen-php"]
codegen-sql = ["uml-codegen-sql"]
# ... all 22 languages

# Code importers
import-cpp = ["uml-import-cpp"]
import-java = ["uml-import-java"]
import-python = ["uml-import-python"]
# ... more languages
```

### 5.3 Dependency Resolution

When `codegen-cpp` is enabled, the workspace adds `uml-codegen-cpp` to the dependency list of the CLI or desktop binary. The `GeneratorRegistry` is populated with all enabled generators at startup.

```rust
// In binary:
fn build_generator_registry(features: &EnabledFeatures) -> GeneratorRegistry {
    let mut reg = GeneratorRegistry::new();
    #[cfg(feature = "codegen-cpp")]
    reg.register(Box::new(uml_codegen_cpp::CppGenerator));
    #[cfg(feature = "codegen-java")]
    reg.register(Box::new(uml_codegen_java::JavaGenerator));
    // ...
    reg
}
```

---

## 6. Cargo.toml Sketches

### 6.1 Workspace Root (`Cargo.toml`)

```toml
[workspace]
resolver = "2"
members = [
    "xtask",

    # Foundation
    "crates/uml-common",
    "crates/uml-core",

    # Infrastructure
    "crates/uml-xmi",
    "crates/uml-persistence",
    "crates/uml-undo",
    "crates/uml-diagram",
    "crates/uml-layout",

    # Rendering
    "crates/uml-render",
    "crates/uml-export",

    # Code generation framework + plugins
    "crates/uml-codegen",
    "crates/uml-codegen-cpp",
    "crates/uml-codegen-java",
    "crates/uml-codegen-python",
    "crates/uml-codegen-rust",

    # Code import framework + plugins
    "crates/uml-import",
    "crates/uml-import-cpp",
    "crates/uml-import-java",
    "crates/uml-import-python",

    # Applications
    "apps/uml-cli",
    "apps/umbrello-desktop",
]

[workspace.package]
version = "1.0.0"
edition = "2024"
rust-version = "1.85"
license = "GPL-2.0-or-later"
authors = ["The Umbrello Team"]

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
slotmap = "1"
uuid = { version = "1", features = ["v4", "serde"] }
bitflags = "2"
quick-xml = "0.37"
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
async-trait = "0.1"
petgraph = "0.7"
tree-sitter = "0.25"
```

### 6.2 `uml-core/Cargo.toml`

```toml
[package]
name = "uml-core"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Pure UML domain model — types, enums, arena-based repository"

[dependencies]
uml-common = { path = "../uml-common" }
slotmap.workspace = true
uuid.workspace = true
bitflags.workspace = true
serde.workspace = true
thiserror.workspace = true
tracing.workspace = true

[dev-dependencies]
serde_json = "1"
quickcheck = "1"
quickcheck_macros = "1"

[features]
default = []
# No feature gates needed — this crate is always fully compiled
```

### 6.3 `uml-xmi/Cargo.toml`

```toml
[package]
name = "uml-xmi"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "XMI 1.2 and 2.1 serialization for Umbrello model files"

[dependencies]
uml-core = { path = "../uml-core" }
quick-xml.workspace = true
serde.workspace = true
thiserror.workspace = true
tracing.workspace = true

[features]
default = ["xmi-v1", "xmi-v2"]
xmi-v1 = []
xmi-v2 = []
```

### 6.4 `uml-persistence/Cargo.toml`

```toml
[package]
name = "uml-persistence"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "File I/O pipeline, format detection, compression, autosave"

[dependencies]
uml-core = { path = "../uml-core" }
uml-xmi = { path = "../uml-xmi" }
uml-undo = { path = "../uml-undo" }
tokio.workspace = true
serde.workspace = true
thiserror.workspace = true
tracing.workspace = true

flate2 = { version = "1", optional = true }
tar = { version = "0.4", optional = true }
bzip2 = { version = "0.5", optional = true }
zip = { version = "2", optional = true }

[features]
default = ["compression", "autosave"]
compression = ["flate2", "tar", "bzip2", "zip"]
autosave = []
```

### 6.5 `uml-codegen/Cargo.toml`

```toml
[package]
name = "uml-codegen"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Code generation framework trait, registry, and shared utilities"

[dependencies]
uml-core = { path = "../uml-core" }
async-trait.workspace = true
serde.workspace = true
thiserror.workspace = true
tracing.workspace = true
```

### 6.6 `uml-codegen-cpp/Cargo.toml`

```toml
[package]
name = "uml-codegen-cpp"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "C++ code generator for Umbrello"

[dependencies]
uml-codegen = { path = "../uml-codegen" }
uml-core = { path = "../uml-core" }
tracing.workspace = true
```

### 6.7 `apps/uml-cli/Cargo.toml`

```toml
[package]
name = "uml-cli"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Umbrello command-line interface"

[dependencies]
uml-persistence = { path = "../../crates/uml-persistence" }
uml-export = { path = "../../crates/uml-export" }
uml-common = { path = "../../crates/uml-common" }
clap.workspace = true
tokio.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
anyhow = "1"

# Language-specific codegen/import — feature-gated
uml-codegen-cpp = { path = "../../crates/uml-codegen-cpp", optional = true }
uml-codegen-java = { path = "../../crates/uml-codegen-java", optional = true }
uml-codegen-python = { path = "../../crates/uml-codegen-python", optional = true }
uml-import-cpp = { path = "../../crates/uml-import-cpp", optional = true }
uml-import-java = { path = "../../crates/uml-import-java", optional = true }
uml-import-python = { path = "../../crates/uml-import-python", optional = true }

[features]
default = [
    "codegen-cpp",
    "codegen-java",
    "codegen-python",
]
codegen-cpp = ["uml-codegen-cpp"]
codegen-java = ["uml-codegen-java"]
codegen-python = ["uml-codegen-python"]
import-cpp = ["uml-import-cpp"]
import-java = ["uml-import-java"]
import-python = ["uml-import-python"]
```

### 6.8 `xtask/Cargo.toml`

```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
description = "Build and maintenance tasks"

[dependencies]
anyhow = "1"
clap.workspace = true
serde_json = "1"
walkdir = "2"
```

---

## 7. Boundary Analysis

### 7.1 What If We Merged `uml-core` + `uml-common`?

**Effect:** Every consumer of basic utilities (error types, version constants) would transitively depend on `slotmap`, `uuid`, and all of `uml-core`'s dependencies. A crate like `uml-import` that only needs error types from `uml-common` would now pull in the entire UML model.

**Verdict:** Keep separate. `uml-common` is for infrastructure; `uml-core` is for domain.

### 7.2 What If We Merged `uml-xmi` + `uml-persistence`?

**Effect:** The persistence crate would have to know about XML internals. A user writing a foreign format backend (e.g., Rose `.mdl`) would have to depend on XMI even if they only need the `StorageBackend` trait.

**Verdict:** Keep separate. XMI is one storage format; the `StorageBackend` trait supports many.

### 7.3 What If We Split `uml-core` into Multiple Crates (`uml-types`, `uml-model`, `uml-repository`)?

**Effect:** More crate boundaries means more `pub use` re-exports, more inter-crate dependency resolution at build time, and potential circular dependencies between types (e.g., `ObjectType` enum needs to be used by both `UmlModelElement` and `ModelRepository` — they'd need to be in the same crate or one would need to depend on the other).

**Verdict:** Keep as one. The C++ codebase's 12 separate list-type headers is a cautionary tale about over-splitting. A single crate with 5k-8k lines of model code is manageable and compiles fast.

### 7.4 What If We Merged All Codegen Crates into One `uml-codegen-all`?

**Effect:** Building all 22 language generators would take 2-5 minutes per compile. Changing one generator (e.g., fixing a C++ template bug) would recompile all 21 others. CI would be slower. Community contributions would require modifying a shared crate.

**Verdict:** Keep one crate per language. The workspace pattern makes cross-crate refactoring trivial (search/replace across the workspace).

### 7.5 What If We Merged `uml-diagram` + `uml-render`?

**Effect:** `uml-layout` would depend on `wgpu` and `cosmic-text` transitively. CLI tooling that reads diagram data but doesn't display it (validation, statistics) would pull in all GPU dependencies.

**Verdict:** Keep separate. This is the classic Model/View separation that Rust's type system enforces naturally.

### 7.6 What If We Merged `uml-undo` into `uml-core`?

**Effect:** The `UndoStack` pattern is an infrastructure concern (command history management), not a domain concern. Core model types would need to understand the concept of undoable commands, which would make serialization harder.

**Verdict:** Keep separate. `uml-core` is pure data; `uml-undo` is operational infrastructure.

### 7.7 Summary Table

| Decision | Rationale | C++ Anti-pattern Avoided |
|----------|-----------|-------------------------|
| `uml-common` separate from `uml-core` | Avoids pulling heavy deps just for error types | — |
| `uml-core` is one crate (not split) | Prevents circular type deps and over-splitting | 12 separate list headers |
| `uml-xmi` separate from `uml-persistence` | Format ↔ storage separation | Scattered saveToXMI |
| Codegen crate per language | Open/closed, compile time, community contributions | CodeGenFactory switch |
| `uml-diagram` separate from `uml-render` | Model ↔ view separation, CLI doesn't need GPU | QGraphicsScene coupling |
| `uml-undo` separate from `uml-core` | Infra ↔ domain separation | QUndoCommand in model |

---

## 8. Build Time Considerations

### 8.1 Compile Time Budget

| Crate | Estimated Raw LOC | Dependencies | Estimated Compile Time |
|-------|-------------------|-------------|----------------------|
| `uml-common` | ~500 | Minimal | ~5s |
| `uml-core` | ~8,000 | slotmap, uuid, serde | ~30s |
| `uml-xmi` | ~4,000 | quick-xml | ~20s |
| `uml-persistence` | ~2,000 | flate2, tar, zip | ~25s |
| `uml-undo` | ~1,500 | — | ~10s |
| `uml-diagram` | ~3,000 | — | ~15s |
| `uml-layout` | ~2,000 | petgraph | ~20s |
| `uml-render` | ~5,000 | vello, wgpu, cosmic-text | ~120s |
| `uml-codegen` | ~1,500 | — | ~10s |
| `uml-codegen-*` (each) | ~1,000 | — | ~8s each |
| `uml-import` | ~1,500 | tree-sitter | ~20s |
| `uml-import-*` (each) | ~2,000 | tree-sitter-* | ~30s each |
| `uml-export` | ~1,000 | resvg | ~20s |
| `uml-cli` | ~500 | clap | ~10s |
| `umbrello-desktop` | ~10,000 | egui | ~60s |

**Clean build (all crates, all features):** ~7-10 minutes (dominated by `uml-render` → `wgpu` shader compilation, and `tree-sitter` grammar build times).

### 8.2 Incremental Build Optimization

**Strategy 1: Feature gating.** The default features enable only the most common codegen/import languages (C++, Java, Python, Rust). A developer adding a new Python code generator doesn't wait for Ada or SQL generators to compile.

**Strategy 2: Separate workspace for heavy crates.** Consider establishing a `crates-heavy/` workspace member group for `uml-render`, `uml-export`, and `uml-layout`. These crates have GPU/layout dependencies and change less frequently than core model types.

**Strategy 3: sccache.** The `rust-toolchain.toml` should configure `sccache` for CI builds:

```toml
# .cargo/config.toml
[env]
SCCACHE_DIR = "/path/to/cache"

[target.x86_64-unknown-linux-gnu]
rustflags = ["-Csymbol-mangling-version=v0"]
```

**Strategy 4: Avoid serialization-heavy builds in dev loops.** Most development work touches only 2-3 crates. `cargo test -p uml-core` compiles only `uml-core` and `uml-common`. Keep dev loops focused.

### 8.3 What Recompiles When

| Change in → | Triggers recompilation of |
|-------------|--------------------------|
| `uml-common` | Everything (rarely changes) |
| `uml-core` | Everything (plan carefully) |
| `uml-xmi` | `uml-persistence`, `uml-cli`, `umbrello-desktop` |
| `uml-codegen-*` | Only the specific generator + binaries |
| `uml-import-*` | Only the specific importer + binaries |
| `uml-render` | `uml-export`, `umbrello-desktop` |
| `umbrello-desktop` | Only itself (leaf binary) |

### 8.4 CI Pipeline

```
Stage 1: Core (2 min)
  cargo check -p uml-common -p uml-core
  cargo test  -p uml-common -p uml-core

Stage 2: Infrastructure (3 min)
  cargo check -p uml-xmi -p uml-undo -p uml-diagram -p uml-codegen -p uml-import
  cargo test  -p uml-xmi -p uml-undo -p uml-diagram -p uml-codegen -p uml-import

Stage 3: Optional features (parallel, 5 min each)
  cargo test --features "codegen-cpp,codegen-java,codegen-python"
  cargo test --features "import-cpp,import-java,import-python"
  cargo test --features "auto-layout,force-layout,png-export,svg-export"

Stage 4: Full (5 min)
  cargo build --all-features
  cargo clippy --all-features -- -D warnings
```

---

## 9. Plugin Discovery for Language Crates

### 9.1 Design Goal

No central registry switch-statement (the C++ anti-pattern). Adding a new language requires:
1. Create a new crate (`uml-codegen-fortran`)
2. Register it in the `GeneratorRegistry` at startup
3. Optionally add a feature flag to the binaries

### 9.2 Compile-Time Registration (Preferred)

Using `cfg` flags in the binary crates:

```rust
// In uml-cli/src/main.rs or umbrello-desktop/src/registry_setup.rs
fn register_code_generators(registry: &mut GeneratorRegistry) {
    // Each enabled feature registers its generator
    #[cfg(feature = "codegen-cpp")]
    registry.register(Box::new(uml_codegen_cpp::CppGenerator));

    #[cfg(feature = "codegen-java")]
    registry.register(Box::new(uml_codegen_java::JavaGenerator));

    #[cfg(feature = "codegen-python")]
    registry.register(Box::new(uml_codegen_python::PythonGenerator));

    // A new language just adds:
    // #[cfg(feature = "codegen-fortran")]
    // registry.register(Box::new(uml_codegen_fortran::FortranGenerator));
}
```

**Pros:** Compile-time safety, no runtime overhead, straightforward.
**Cons:** Requires adding the crate to the workspace and adding the feature flag.

### 9.3 Dynamic Discovery via Link-Time Registration (Future Alternative)

For truly pluggable generators that ship as separate packages:

```rust
// Define a global registry using inventory or linkme
#[linkme::distributed_slice]
pub static CODE_GENERATORS: [fn() -> Box<dyn CodeGenerator>] = [..];

// In each generator crate:
#[linkme::distributed_slice(uml_codegen::CODE_GENERATORS)]
static REGISTER: fn() -> Box<dyn CodeGenerator> = || Box::new(CppGenerator);

// In the binary:
for gen_fn in CODE_GENERATORS {
    registry.register(gen_fn());
}
```

**Pros:** Truly plugin-based; third-party generators can be linked without modifying the binary.
**Cons:** Adds `linkme` (or `inventory`) dependency; runtime registration errors are possible.

### 9.4 Dynamic Loading via `dlopen` (Not Recommended)

**Approach:** Load `.so` files from a plugin directory at runtime.
**Verdict:** Too complex, unsafe, platform-dependent. Revisit only if there's a strong use case for shipping generators separately from the application.

### 9.5 Import Registry (Same Pattern)

The `ImportRegistry` follows the identical pattern:

```rust
fn register_importers(registry: &mut ImportRegistry) {
    #[cfg(feature = "import-cpp")]
    registry.register(Box::new(uml_import_cpp::CppImporter));

    #[cfg(feature = "import-java")]
    registry.register(Box::new(uml_import_java::JavaImporter));
}
```

### 9.6 Default Feature Set

The default features should cover the "big four" languages that cover 90% of use cases:

```toml
[features]
default = [
    "codegen-cpp",    # Most mature generator in C++ Umbrello
    "codegen-java",   # Also well-developed
    "codegen-python", # Popular, lightweight
    "codegen-rust",   # Our own language — dogfooding
    "import-cpp",     # Primary import language
    "import-java",    # Second most-used import
    "import-python",  # Third most-used import
]
```

---

## 10. Workspace Tooling (xtask)

### 10.1 Purpose

The `xtask` crate is a cargo-alike task runner invoked via `cargo xtask <task>`. It handles build-time and CI tasks that are too complex for shell scripts.

### 10.2 Defined Tasks

```text
cargo xtask:
  generate-test-fixtures    Generate XMI test files from model definitions
  validate-xmi <file>       Validate an XMI file against Umbrello DTD
  check-roundtrip <file>    Load → save → compare XMI files
  list-crates               Show workspace crate dependency graph
  check-features            Verify feature flag consistency
  generate-schema           Generate JSON Schema for model types
  doc-check                 Verify all doc links and examples
  migration-report          Report which C++ subsystems remain untranslated
```

### 10.3 Key Task Details

**`cargo xtask generate-test-fixtures`:**
Reads YAML/JSON model definitions and produces `.xmi` files in `tests/fixtures/`. This ensures test data is reproducible and versioned.

**`cargo xtask validate-xmi <file>`:**
Validates an XMI file against the embedded DTD rules. This is critical for ensuring backward compatibility with the C++ version's file format.

**`cargo xtask check-roundtrip <file>`:**
1. Load the XMI file via `uml-persistence`
2. Serialize the model back to XMI
3. Load the result again
4. Deep-compare in-memory model objects
5. Produce a diff report if they differ

**`cargo xtask list-crates`:**
Uses `cargo metadata` to extract the workspace graph and prints it as a DOT file or ASCII diagram, which is kept in sync with the dependency graph in this document.

### 10.4 xtask Implementation Sketch

```rust
// xtask/src/main.rs
use clap::Parser;

#[derive(Parser)]
enum Task {
    GenerateTestFixtures { source: PathBuf, output: PathBuf },
    ValidateXmi { file: PathBuf },
    CheckRoundtrip { file: PathBuf },
    ListCrates,
}

fn main() -> Result<()> {
    let task = Task::parse();
    match task {
        Task::GenerateTestFixtures { source, output } => gen_fixtures(source, output),
        Task::ValidateXmi { file } => validate_xmi(file),
        Task::CheckRoundtrip { file } => check_roundtrip(file),
        Task::ListCrates => list_crates(),
    }
}
```

### 10.5 Integration with CI

`cargo xtask` is called from `.gitlab-ci.yml`:

```yaml
validate-xmi:
  script:
    - cargo xtask validate-xmi tests/fixtures/class_diagram.xmi
    - cargo xtask check-roundtrip tests/fixtures/class_diagram.xmi

check-generators:
  script:
    - cargo xtask generate-test-fixtures --source model-defs/ --output tests/fixtures/
    - cargo test
```

---

## 11. Naming Conventions

### 11.1 Crate Names

| Pattern | Example | Rule |
|---------|---------|------|
| Framework crates | `uml-core`, `uml-xmi`, `uml-codegen` | Prefix `uml-` for Umbrello-specific crates |
| Language-specific codegen | `uml-codegen-cpp`, `uml-codegen-java` | `uml-codegen-{language}` in lowercase |
| Language-specific import | `uml-import-cpp`, `uml-import-java` | `uml-import-{language}` in lowercase |
| Binary crates | `uml-cli`, `umbrello-desktop` | `uml-cli` for CLI tools, `umbrello-` for GUI |
| Tooling | `xtask` | Single word, no prefix |

### 11.2 Module Names

| Pattern | Example | Rule |
|---------|---------|------|
| Within domain crates | `uml-core::model`, `uml-core::types` | Short, descriptive nouns |
| Plurals | `types`, `widgets`, `dialogs` | Prefer plural for collection modules |
| Avoid `mod.rs` | `uml-core/src/model/class.rs` | Cargo 2024 edition: `class.rs` not `class/mod.rs` |

### 11.3 Type Names

| Pattern | Example | Rule |
|---------|---------|------|
| Model types | `UmlClass`, `UmlAssociation`, `UmlPackage` | PascalCase with `Uml` prefix (distinguishes from Rust std types) |
| Enums | `ObjectType`, `AssociationType`, `Visibility` | PascalCase, descriptive, no Hungarian |
| Traits | `CodeGenerator`, `StorageBackend`, `LayoutEngine` | PascalCase, noun or noun-verb |
| Error types | `XmiError`, `PersistenceError`, `ImportError` | PascalCase, `Error` suffix |
| ID types | `UmlId`, `ObjectKey`, `SceneId`, `DiagramId` | PascalCase, `Id` or `Key` suffix |
| Command types | `CmdCreateObject`, `CmdRemoveWidget` | PascalCase, `Cmd` prefix (matching C++ convention) |

### 11.4 Function and Variable Names

| Pattern | Example | Rule |
|---------|---------|------|
| Public API functions | `repository.insert()`, `writer.write_document()` | snake_case |
| Getters | `obj.name()`, `obj.key()` | No `get_` prefix (Rust convention) |
| Setters | `obj.set_name()`, `obj.set_visibility()` | `set_` prefix (Rust convention) |
| Conversion | `object.try_into_classifier()` | `try_into_*` for fallible, `into_*` for infallible |

### 11.5 File Naming

| Pattern | Example | Rule |
|---------|---------|------|
| Per-type files | `class.rs`, `association.rs`, `package.rs` | One type per file for complex types |
| Module aggregators | `lib.rs` | Only at crate root |
| Tests | `class_test.rs` or inline `#[cfg(test)]` | Prefer inline tests; separate file only for complex integration tests |

---

## 12. Migration Path

### 12.1 Phase Sequence

| Phase | Crates | Can test with C++ Umbrello? | Milestone |
|-------|--------|---------------------------|-----------|
| 1 | `uml-common`, `uml-core` | No (pure data) | `cargo test -p uml-core` passes |
| 2 | `uml-xmi` | Yes: Rust reads C++ `.xmi`, compares model | Round-trip with C++ XMI files |
| 3 | `uml-undo` | No | Undo stack works |
| 4 | `uml-persistence` | Yes: Rust opens C++ `.xmi` files directly | File open/save works |
| 5 | `uml-codegen`, `uml-codegen-cpp` | Compare output with C++ codegen | Generated C++ matches C++ version |
| 6 | `uml-import`, `uml-import-cpp` | Import C++ files, compare model | Imported model matches C++ import |
| 7 | `uml-diagram`, `uml-layout` | Compare scene data with C++ | Diagram model matches |
| 8 | `uml-render`, `uml-export` | Compare rendered output visually | SVG/PNG output matches |
| 9 | `uml-cli` | Run headless, compare exports | CLI feature parity |
| 10 | `umbrello-desktop` | Replace C++ GUI | Full application |

### 12.2 Coexistence Strategy

During migration, the Rust and C++ codebases coexist:

1. **Rust reads C++ XMI files** → validates it can load any `.xmi` file the C++ version produces.
2. **Rust writes XMI files** → C++ version loads them, verifies no data loss.
3. **CI runs both** → `cargo test` on Rust, `ctest` on C++, compares outputs.
4. **Cutover** → when all features are implemented in Rust, the C++ version is removed.

### 12.3 What We Preserve From C++

- **File format compatibility** (XMI 1.2 and 2.1)
- **Model semantics** (all UML types, all diagram types)
- **CLI interface** (same flags, same behavior)
- **Code generation output** (identical source text for the same model)

### 12.4 What We Leave Behind

| C++ Feature | Rust Replacement | Reason |
|-------------|-----------------|--------|
| `UMLEpp::app()` singleton | `AppContext` with DI | Eliminate global state |
| QObject inheritance | Plain structs + tags | No Qt dependency |
| KDE XmlGui | egui native | Cross-platform GUI |
| KConfig | TOML/JSON settings file | Simpler, portable |
| QGraphicsScene/View | vello + wgpu custom canvas | GPU-accelerated |
| Shell exec Graphviz | Pure Rust layout (petgraph + custom) | Remove runtime dep |
| 12 list type headers | `Vec<T>` | Use standard library |

---

> **This document is a living proposal.** As implementation proceeds and we discover what works and what doesn't, these crate boundaries should be revisited and revised. The key invariants are:
> 1. No cycles between crates
> 2. Model is pure data
> 3. Each crate has a single, well-defined responsibility
> 4. Language-specific features are additive, not invasive
