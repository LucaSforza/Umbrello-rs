# Umbrello-RS — Agent Guide

> **Rust rewrite of Umbrello** — a 20+ year old C++/Qt UML modeling tool.  
> Repository root: `rust-rewrite/` within the [umbrello](https://invent.kde.org/sdk/umbrello) monorepo.  
> C++ source (`../umbrello/`, `../lib/`, `../unittests/`) is **READ-ONLY** reference material — never modify.

---

## Table of Contents

1. [Project Overview](#project-overview)
2. [Workspace Layout](#workspace-layout)
3. [Test Coverage](#test-coverage)
4. [Milestones Completed](#milestones-completed)
5. [Architecture Decisions](#architecture-decisions)
6. [Key Domain Types](#key-domain-types)
7. [Completeness Gap Analysis](#completeness-gap-analysis)
8. [Build & Run](#build--run)
9. [How to Contribute](#how-to-contribute)
10. [Architecture Rules](#architecture-rules)
11. [Reference: C++ Source Map](#reference-c-source-map)

---

## Project Overview

Umbrello-RS is a ground-up Rust rewrite of the [Umbrello](https://apps.kde.org/umbrello/) UML modeller, a KDE application that has been developed continuously since 2001. The rewrite preserves the UML 1.2 XMI interchange format for compatibility with the original, while building a modern architecture in Rust.

**Current state:** 257 tests passing across 5 crates. The core domain model is complete (18 milestones). The GUI application (egui/eframe) renders partitioned class boxes with semantic edges, supports full File I/O (Open, Save, Save As, New) with native dialogs and dirty-flag tracking, provides a tool palette for interactive element creation (click-to-place on canvas), and features a property editor panel for inspecting and modifying element properties. Major gaps remain in element type coverage, XMI completeness, and edge creation.

**Repo:** <https://invent.kde.org/sdk/umbrello> | **C++ original:** 2500+ files | **Rust rewrite:** ~45 source files

---

## Workspace Layout

```
rust-rewrite/
├── Cargo.toml                       # Workspace root (5 members, resolver = "2")
├── rust-toolchain.toml              # Rust 1.85+ (stable, rustfmt + clippy)
├── rustfmt.toml                     # Formatting config
├── deny.toml                        # cargo-deny dependency audit
├── .github/                         # CI workflows
│
├── crates/
│   ├── uml-core/                    # Domain model (the "kernel")
│   │   └── src/
│   │       ├── lib.rs               # Re-exports, crate entrypoint
│   │       ├── id.rs                # UmlId (UUID-backed identifier)
│   │       ├── types.rs             # ObjectType (30 vars), AssociationType (6), DiagramType (11), Visibility, ParameterDirection
│   │       ├── elements.rs          # ModelElement enum, ElementBase, ClassifierData, Package, Class, Interface, Enum, Datatype, Relationship, TypeReference, Attribute, Operation, Parameter
│   │       ├── repository.rs        # UmlModel (arena, parent_index, cycle detection, reference validation, diagrams)
│   │       ├── model.rs             # (future — higher-level model operations)
│   │       ├── event.rs             # Model change event types
│   │       ├── common/
│   │       │   └── mod.rs           # Shared error types (UmbrelloError)
│   │       ├── diagram/
│   │       │   ├── mod.rs           # Diagram, DiagramId, DiagramKind
│   │       │   ├── node.rs          # ViewNode
│   │       │   ├── edge.rs          # ViewEdge, EdgeId, EdgeLabel, LineRouting
│   │       │   └── geometry.rs      # Point, Size, Rect
│   │       ├── undo/
│   │       │   ├── mod.rs           # Command trait, History (bounded stack)
│   │       │   └── commands.rs      # 8 concrete commands (Create, Delete, Rename, Move, AddNode, RemoveNode, MoveNode, ResizeNode)
│   │       ├── layout/
│   │       │   └── mod.rs           # STUB — auto-layout not implemented
│   │       ├── render/
│   │       │   └── mod.rs           # Rendering abstractions (extracted from app in future)
│   │       └── xmi/
│   │           └── mod.rs           # XMI helpers used by uml-io (shared types/constants)
│   │
│   ├── uml-io/                      # Persistence layer
│   │   └── src/
│   │       ├── lib.rs               # FileFormat enum, crate re-exports
│   │       ├── storage.rs           # Storage abstraction (load/save from files)
│   │       └── xmi/
│   │           ├── mod.rs           # Public API: read_xmi(), write_xmi()
│   │           ├── reader.rs        # 2-pass parser (UML 1.2 XMI → UmlModel + diagrams)
│   │           ├── writer.rs        # XMI serializer (UmlModel → XMI 1.2 XML)
│   │           └── error.rs         # XmiParseError types
│   │
│   └── uml-codegen/                 # Code generation (stubs)
│       └── src/
│           ├── lib.rs               # CodeGenerator trait, ProgrammingLanguage enum
│           ├── writer.rs            # CodeWriter (indentation, braces, file output)
│           ├── registry.rs          # GeneratorRegistry (stub)
│           ├── cpp.rs               # STUB
│           ├── java.rs              # STUB
│           ├── python.rs            # STUB
│           └── rust.rs              # STUB
│
├── apps/
│   └── umbrello/                    # GUI application
│       └── src/
│           ├── main.rs              # Entry point (eframe)
│           └── app.rs               # UmbrelloApp — menus, canvas, rich rendering
│
├── xtask/                           # Dev task runner
│   └── src/
│       └── main.rs                  # Custom build/CI commands
│
├── docs/                            # Architecture documentation (23 documents)
│   ├── domain_model_v1.md           # Domain model design
│   ├── model_repository_v1.md       # Repository design
│   ├── relationships_v1.md          # Relationship design
│   ├── xmi_persistence_architecture_v1.md  # XMI persistence design
│   ├── ui_rich_rendering_spec_v1.md # Canvas rendering specification
│   ├── command_architecture_v1.md   # Undo/redo design
│   └── ...                          # Reviews, audits, and milestone documents
│
├── tests/                           # (empty — integration tests live in crate test dirs)
├── analysis/                        # Static analysis & profiling artifacts
├── planning/                        # Project planning documents
└── research/                        # Research notes on UML standards, XMI formats, etc.
```

### Crate Dependency Graph

```
apps/umbrello  ────  uml-core
    │                    │
    │                    ├── diagram/
    │                    ├── undo/
    │                    └── types + elements + repository
    │
uml-io  ───────────────  uml-core (uses reader/writer access)
uml-codegen  ─────────── uml-core (uses model data for generation)
xtask  ────────────────── (standalone CLI)
```

No circular dependencies. `uml-core` is the foundational crate with zero dependencies on other workspace crates.

---

## Test Coverage

**Total: 257 tests, all passing** (as of Milestone 18).

### By Crate

| Test Suite | Count | What It Covers |
|------------|-------|-----------------|
| `uml-core` unit tests | 134 | `elements.rs` (element creation, serde, relationships, TypeReference), `repository.rs` (insert/remove, parent_index, cycle detection, validation, cascading cleanup), `types.rs` (enum properties, serde round-trips, uniqueness), `diagram/mod.rs` (Diagram CRUD, DiagramKind round-trip) |
| `uml-core` id_tests | 8 | `id.rs` — UmlId generation, equality, ordering, Display, serde, UUIDv4 properties |
| `uml-core` serde_roundtrip | 6 | External serde round-trip tests for element types |
| `uml-core` diagram_geometry | 2 | `diagram/geometry.rs` — Point, Size, Rect construction and arithmetic |
| `uml-core` history | 4 | `undo/mod.rs` — History stack, execute/undo/redo, max_depth, disabled mode |
| `uml-io` XMI tests | 46 | `reader.rs` — parsing of Package, Class, Interface, Enum, Datatype, attributes, operations, parameters, Generalization, Association, Dependency, Abstraction/Realization; `writer.rs` — writing back to XMI; `xmi/mod.rs` — `save_xmi_to_file` / `load_xmi_from_file` convenience functions |
| `uml-io` real corpus | 1 | Load `../test/test-COG.xmi` (a real Umbrello file), verify 18 diagrams, 70+ nodes, 57+ edges |
| `apps/umbrello` tests | 46 | `tests.rs` — visibility symbols, type display, element colors, dirty-flag tracking, file I/O (New/Open/Save round-trip), tool palette, element creation, smart naming, selection tracking, property editor commands |
| Doctests | 1 | `crates/uml-io/src/xmi/writer.rs` — XmiWriter usage example |

### Test Commands

```sh
# Run all tests
cargo test --workspace

# Run a specific test suite
cargo test -p uml-core id_tests
cargo test -p uml-io xmi
cargo test -p umbrello

# Run a single test by name
cargo test -p uml-core serde_roundtrip_model_element
```

### Testing Philosophy

- **Unit tests** live in `#[cfg(test)] mod tests` blocks at the bottom of each source file.
- **Pure functions** are preferred — they are trivially testable.
- **Floating-point equality** tests use `assert!((a - b).abs() < EPSILON)` pattern, with `#[allow(clippy::float_cmp)]` where exact comparison is intended.
- **Serde round-trip tests** verify JSON serialization/deserialization for every domain type.
- **Visual rendering** is tested manually (12 visual test cases defined in the M15 spec) — automated screenshot testing is deferred.
- **XMI corpus tests** load real Umbrello files and verify structural completeness.

---

## Milestones Completed

### M1 — Cargo workspace, crate stubs, CI, rustfmt, clippy
- Workspace with 5 crates (originally 21, consolidated in M7)
- CI with `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
- `rust-toolchain.toml` pins stable Rust with rustfmt + clippy components
- `deny.toml` for dependency auditing

### M2 — Core types: UmlId, ObjectType, AssociationType, DiagramType, Visibility, ParameterDirection
- 45 tests on enum variants, Display impls, serde round-trips, uniqueness
- `UmlId` backed by UUIDv4 with `new()`, deterministic `Display`, `Ord`/`Hash`
- All enums are `Copy + Clone + PartialEq + Eq + Serialize + Deserialize`

### M3 — UML metamodel: NamedElement trait, ElementBase, ClassifierData, Package, Class, Interface, Enum, ModelElement enum
- 66 tests on element creation, naming, classifier data access, serde
- `ModelElement` enum with 6 variants (Package, Class, Interface, Enum, Datatype, Relationship)
- Composition over inheritance: `ClassifierData` embedded in Class/Interface/Enum
- `TypeReference` with display_name resolution against model

### M4 — Model repository: UmlModel with IndexMap storage, parent_index, cascading remove, cycle detection, reference validation
- 94 tests covering insert/remove, iteration order, cascading cleanup, cycle detection, reference validation
- `UmlModel` owns all elements, packages reference children by `UmlId`
- `parent_index: HashMap<UmlId, Vec<UmlId>>` for O(1) parent lookup and cycle detection
- `validate_references()` checks all inter-element references for dangling pointers

### M5 — Relationships: Relationship as ModelElement variant, 6 constructor methods, query methods, cascading cleanup
- 110 tests (incremental); 6 `AssociationType` variants (purified from 12 in M7)
- Constructor methods: `new_generalization()`, `new_realization()`, `new_association()`, `new_aggregation()`, `new_composition()`, `new_dependency()`
- Cascading relationship cleanup when source/target elements are removed
- Query methods: `relationships_of()`, `generalizations_of()`, `realizations_of()`

### M6 — TypeReference extraction: deduplicated type_id/type_name pattern, architecture audit
- 118 tests (incremental); `TypeReference` extracted from attribute-specific code to shared pattern
- `TypeReference` with `model_id` and `type_name` fields (at most one set)
- `display_name()` resolves model references via `UmlModel` lookup

### M7 — Workspace consolidation: 21→5 crates, AssociationType purification 12→6 variants
- 117 tests (incremental); merged `uml-common`, `uml-xmi`, `uml-undo`, `uml-diagram`, `uml-layout`, `uml-render` into `uml-core`
- Removed 6 AssociationType variants (Coll_Resol, State, Activity, Containment, Protocol, Relationship) to keep only pure UML 2.x types
- Single workspace Cargo.toml with shared dependency versions

### M8 — XMI reader: two-pass parsing, ElementBase.original_xmi_id, DataType variant
- 128 tests (incremental); `XmiReader` with deferred cross-reference resolution
- `original_xmi_id: Option<String>` on `ElementBase` for round-trip compatibility
- Datatype element type added to ModelElement

### M9 — Feature parsing: attributes, operations, parameters, relationships from XMI, real corpus test
- 144 tests (incremental); full feature parsing in reader
- Real corpus test loads `../test/test-COG.xmi`
- Attributes, operations, parameters with type references and direction

### M10 — XMI writer: round-trip validation, full serialization pipeline
- 160 tests (incremental); `XmiWriter` produces valid UML 1.2 XMI
- Round-trip tests: load XMI → write XMI → load again → structural equality
- XMI layout persistence (diagrams with nodes and edges)

### M11 — Undo/redo engine: Command trait, History, 4 commands
- 176 tests (incremental); `Command` trait with `execute()`, `undo()`, `merge()`
- `History` bounded stack (configurable max_depth, trim oldest when full)
- 8 commands: CreateElement, DeleteElement, RenameElement, MoveElement, AddNodeToDiagram, RemoveNodeFromDiagram, MoveNode, ResizeNode
- `disabled` mode for batch loading (execute but don't track)

### M12 — Headless diagram engine: Diagram, ViewNode, ViewEdge, Point/Size/Rect
- 190 tests (incremental); pure data structures, no rendering
- `Diagram` with `IndexMap<UmlId, ViewNode>` and `IndexMap<EdgeId, ViewEdge>`
- Visual commands for diagram mutations

### M13 — XMI layout persistence: diagram read/write from XMI
- 200 tests (incremental); 18 diagrams, 70+ nodes, 57+ edges from corpus
- Diagram geometry (Point, Size, Rect) serialized alongside model elements
- `original_xmi_id` preserved for round-trip

### M14 — egui canvas prototype: interactive window, drag-to-move nodes, undo/redo
- 200 tests (incremental); GUI application running with egui/eframe
- File menu (Quit only, Open stubbed), Edit menu (Undo/Redo with Ctrl+Z/Y)
- Left panel: diagram list, element flat list, "New Class Diagram" button
- Canvas: draggable colored rectangles for each model element
- `loaded_from_xmi` flag in app constructor

### M15 — Rich UML canvas: partitioned class boxes, semantic edge engine with 6 arrowhead types
- **206 tests** (incremental + 6 new app tests)
- 5 node types rendered with compartments: Class (stereotype/name/attributes/operations), Interface, Enum, Datatype, Package
- 6 edge types: Generalization (hollow triangle), Realization (dashed + hollow triangle), Aggregation (hollow diamond), Composition (filled diamond), Dependency (dashed + open arrow), Association (plain line)
- Arrowhead geometry uses proper vector math (direction, perpendicular, vertex computation)
- Dashed line rendering for Realization and Dependency

### M16 — File I/O: Open, Save, Save As, New + dirty tracking + CLI
- **216 tests** (incremental + 10 new: 8 app, 2 uml-io)
- File menu fully functional: New, Open, Save, Save As, Quit with unsaved-changes prompting
- Native file dialogs via `rfd` crate (XMI filter, extension enforcement)
- Dirty-flag tracking — `*` in title bar on any model mutation; clears on save/open/new
- Keyboard shortcuts: Ctrl+N (New), Ctrl+O (Open), Ctrl+S (Save), Ctrl+Shift+S (Save As), Ctrl+Q (Quit)
- Window title reflects current file path + dirty state
- Error handling: native dialogs for I/O failures and XMI parse errors
- CLI overhaul: `clap` positional `file` argument replaces hardcoded `test-COG.xmi` path
- `save_xmi_to_file()` / `load_xmi_from_file()` convenience functions in `uml-io`
- `execute_command` helper wraps `History::execute` with automatic dirty tracking
- Zero changes to `uml-core`

### M17 — Tool Palette & Interactive Element Creation
- **233 tests** (incremental + 17 new: all in app)
- `ToolMode` enum with 6 variants (Select, CreateClass, CreateInterface, CreateEnum, CreateDatatype, CreatePackage)
- Vertical tool palette panel in the left sidebar with `SelectableLabel` buttons; active tool highlighted
- Click-to-place on canvas: selecting a creation tool then clicking on the active diagram creates the element + places a `ViewNode` at the click position
- Tool auto-resets to `Select` after successful placement (one-shot creation)
- Smart default naming: `generate_unique_name()` scans existing names, gap-fills suffixes (`"Class_1"`, `"Class_2"`, etc.)
- Ghost preview: semi-transparent 160×60 blue rectangle at cursor position when hovering with creation tool
- Crosshair cursor when creation tool is active
- Keyboard shortcuts: S=Select, C=Class, I=Interface, E=Enum, D=Datatype, P=Package, Esc=Select (only when `!ctx.wants_keyboard_input()`)
- Element placement uses two commands (`CreateElement` + `AddNodeToDiagram`) via `execute_command()` with dirty tracking
- Zero changes to `uml-core` or `uml-io`

### M18 — Property Editor Panel
- **257 tests** (incremental + 24 new: 9 uml-core, 15 app)
- Modular source split: `app.rs` (~1652 lines) split into 9 files (`canvas.rs`, `tool_palette.rs`, `rendering.rs`, `menu.rs`, `tree.rs`, `file_io.rs`, `property_editor.rs`, `tests.rs`, `app.rs`)
- 3 new undo commands: `ChangeVisibility`, `ChangeElementFlags`, `ChangeDocumentation` (follow `RenameElement` snapshot pattern)
- Selection tracking: click a node to select it, highlighted with 2.5px blue border, Escape/background-click to deselect
- Right-side property editor panel (`egui::SidePanel::right`):
  - "Nothing selected" placeholder when no element is selected
  - Read-only Type and ID display
  - Editable name field → `RenameElement` command (commit on Enter or focus loss)
  - Visibility dropdown (Public/Protected/Private/Implementation) → `ChangeVisibility` command
  - Abstract / Static toggle checkboxes → `ChangeElementFlags` command (atomic both-flags)
  - Documentation multiline text area → `ChangeDocumentation` command (commit on focus loss)
  - Read-only classifier details: lists attributes and operations with visibility symbols and type names
- `visibility_name()` helper added to rendering module
- Zero changes to `uml-io` or `uml-codegen`

---

## Architecture Decisions

| Decision | Rationale |
|----------|-----------|
| **IndexMap over HashMap** | Deterministic insertion-order iteration for reproducible tests and XMI round-trip stability |
| **ModelElement enum over class hierarchy** | Type-safe pattern-match dispatch, no RTTI (`isUML*()`/`asUML*()`), exhaustiveness checked by compiler |
| **Composition over inheritance** | `ClassifierData` embedded in Class/Interface/Enum/DataType avoids deep inheritance chains; no diamond problem |
| **UmlId as sole key** | UUID-backed with serde support; no secondary `ObjectKey` mapping as in C++ codebase |
| **Box\<dyn Command\> over enum Command** | Open-closed principle — new commands don't require enum variant changes; plugin-friendly |
| **egui over Slint/Iced** | Immediate mode eliminates state synchronization; well-suited for diagram canvas with frequent repaints |
| **XMI bridge strategy** | `original_xmi_id` preserved on `ElementBase` for round-trip compatibility with C++ Umbrello |
| **Two-pass XMI parsing** | First pass extracts structural elements; second pass resolves cross-references (type IDs, relationship endpoints, stereotype IDs) — handles forward references in XMI files |
| **UmlModel owns all elements** | Single arena (`IndexMap<UmlId, ModelElement>`); packages reference children by ID, never own them |
| **All mutations via Commands** | `History::execute()` for every user-initiated mutation; direct mutation only during XMI loading (with history disabled) |
| **Core domain is pure** | `uml-core` has no GUI dependencies, no I/O — rendering, persistence, and code generation are separate crates |
| **thiserror for errors** | Structured error types (`ModelError`, `CommandError`, `XmiParseError`) with `Display` + `Error` impls |

---

## Key Domain Types

### Identity

```rust
// crates/uml-core/src/id.rs
pub struct UmlId(Uuid);  // UUID v4, Copy + Clone + Ord + Hash + Serialize + Deserialize
```

### Enumerations

```rust
// crates/uml-core/src/types.rs — all Copy + Clone + PartialEq + Eq + Serialize + Deserialize

ObjectType          // 30 variants: Class, Interface, Enumeration, Datatype, Entity, Package, Folder,
                    //   Component, Artifact, Actor, UseCase, Node, Port, Category, Instance,
                    //   Attribute, Operation, Template, EnumLiteral, EntityAttribute,
                    //   UniqueConstraint, ForeignKeyConstraint, CheckConstraint,
                    //   Association, Role, Generalization, Realization, Dependency,
                    //   Stereotype, InstanceAttribute
AssociationType     // 6 variants: Association, Generalization, Realization, Aggregation, Composition, Dependency
DiagramType         // 11 variants: Undefined, Class, UseCase, Sequence, Collaboration, State, Activity,
                    //   Component, Deployment, EntityRelationship, Object
Visibility          // 4 variants: Public(+), Protected(#), Private(-), Implementation(~)
ParameterDirection  // 4 variants: In, Out, InOut, Return
DiagramKind         // 10 variants: Class, UseCase, Sequence, Collaboration, State, Activity,
                    //   Component, Deployment, EntityRelationship, Object
```

### Domain Model

```rust
// crates/uml-core/src/elements.rs

ElementBase {
    id: UmlId,
    name: String,
    visibility: Visibility,
    stereotype_id: Option<UmlId>,
    documentation: String,
    is_abstract: bool,
    is_static: bool,
    original_xmi_id: Option<String>,
}

TypeReference {
    model_id: Option<UmlId>,          // references a model classifier
    type_name: Option<String>,        // primitive/external type name
    // At most one set — both None = void/unspecified
}

ClassifierData {
    attributes: Vec<Attribute>,
    operations: Vec<Operation>,
    templates: Vec<TemplateParameter>,
}

// Element types — each embeds ElementBase + optional ClassifierData
Package     { base: ElementBase, children: Vec<UmlId> }
Class       { base: ElementBase, classifier: ClassifierData }
Interface   { base: ElementBase, is_abstract: true, classifier: ClassifierData }
Enum        { base: ElementBase, classifier: ClassifierData, literals: Vec<EnumLiteral> }
Datatype    { base: ElementBase, classifier: ClassifierData }
Relationship { base: ElementBase, kind: AssociationType, source_id: UmlId, target_id: UmlId, ... }

// Type-safe dispatch
enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
    Datatype(Datatype),
    Relationship(Relationship),
}

trait NamedElement {
    fn base(&self) -> &ElementBase;
    fn base_mut(&mut self) -> &mut ElementBase;
    fn id(&self) -> UmlId;
    fn name(&self) -> &str;
    fn object_type(&self) -> ObjectType;
}
```

### Repository

```rust
// crates/uml-core/src/repository.rs

UmlModel {
    elements: IndexMap<UmlId, ModelElement>,    // arena — owns all elements
    parent_index: HashMap<UmlId, Vec<UmlId>>,   // reverse: element → parent packages
    diagrams: Vec<Diagram>,                     // visual diagrams
}
// Methods: insert, remove, get, get_mut, iter, contains, len, is_empty,
//   add_to_package, remove_from_package, parents_of, retain, drain,
//   validate_references, relationships_of, generalizations_of, realizations_of,
//   add_diagram, remove_diagram, get_diagram, get_diagram_mut
```

### Diagram Model

```rust
// crates/uml-core/src/diagram/

Diagram {
    id: DiagramId,
    name: String,
    kind: DiagramKind,
    nodes: IndexMap<UmlId, ViewNode>,
    edges: IndexMap<EdgeId, ViewEdge>,
}

ViewNode {
    model_element_id: UmlId,
    bounds: Rect,
    visible: bool,
    z_order: i32,
}

ViewEdge {
    relationship_id: UmlId,
    source_node_id: UmlId,
    target_node_id: UmlId,
    routing: LineRouting,
    waypoints: Vec<Point>,
}

Point { x: f64, y: f64 }
Size  { width: f64, height: f64 }
Rect  { x: f64, y: f64, width: f64, height: f64 }
```

### Undo/Redo

```rust
// crates/uml-core/src/undo/

trait Command: Debug + Send {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn description(&self) -> &str;
    fn merge(&self, other: &dyn Command) -> Option<Box<dyn Command>>;
}

History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_depth: usize,
    disabled: bool,
}
// Methods: execute, undo, redo, can_undo, can_redo, set_disabled, clear

// Commands: CreateElement, DeleteElement, RenameElement, MoveElement,
//           AddNodeToDiagram, RemoveNodeFromDiagram, MoveNode, ResizeNode
```

---

## Completeness Gap Analysis

### Priority Legend

| Priority | Meaning |
|----------|---------|
| **HIGH** | Blocks core functionality; needed for minimum viable product |
| **MEDIUM** | Important feature used by many users; implement next |
| **LOW** | Nice-to-have; defer to post-MVP |

---

### Domain Model — HIGH Priority

These element types are defined in the `ObjectType` enum but have no corresponding Rust struct or `ModelElement` variant. They represent real UML concepts that the C++ codebase supports.

| Element | Required For | Estimated Work | Status |
|---------|-------------|----------------|--------|
| **Actor** | Use case diagrams | Add struct + ModelElement variant + XMI reader/writer | NOT STARTED |
| **UseCase** | Use case diagrams | Add struct + ModelElement variant + XMI reader/writer | NOT STARTED |
| **Component** | Component diagrams | Add struct + ModelElement variant + XMI reader/writer | NOT STARTED |
| **Node** | Deployment diagrams | Add struct + ModelElement variant + XMI reader/writer | NOT STARTED |
| **Artifact** | Deployment/component diagrams | Add struct + ModelElement variant + XMI reader/writer | NOT STARTED |
| **Port** | Class/component diagrams | Add struct + ModelElement variant | NOT STARTED |
| **Instance** | Object diagrams | Add struct + ModelElement variant | NOT STARTED |
| **Category** | EER diagrams | Add struct + ModelElement variant | NOT STARTED |
| **Entity** | ER diagrams | Add struct with entity attributes + constraints | NOT STARTED |
| **Stereotype registry** | UML profiles, tag definitions | Add `Stereotype` struct + `stereotype` tag parsing in XMI | NOT STARTED |
| **Template parameter integration** | Generic classes | `TemplateParameter` and `templates` field exist but unused in XMI reader/writer | PARTIAL |

**Implementation pattern** (example for Actor):

```rust
// 1. Add struct
pub struct Actor { pub base: ElementBase }

// 2. Add ModelElement variant
enum ModelElement {
    // ... existing ...
    Actor(Actor),
}

// 3. Add match arms in base(), base_mut(), object_type(), is_classifier(), etc.
// 4. Add XMI reader case in reader.rs
// 5. Add XMI writer case in writer.rs
// 6. Add relationship target-side rendering in app.rs
// 7. Write tests
```

---

### XMI / Persistence — HIGH Priority

| Feature | Current State | Required Work |
|---------|--------------|---------------|
| **Actor/UseCase element parsing** | Reader skips `UML:Actor`, `UML:UseCase` XML elements | Add parser cases + struct creation |
| **Note widget parsing** | Notes in XMI diagrams not mapped | Add note widget type + parsing |
| **Sequence diagram messages** | `UML:Message` elements ignored | Add message relationship type + parsing |
| **State diagram elements** | `UML:StateVertex`, `UML:StateMachine`, `UML:Transition` not parsed | Add state machine types + parsing |
| **Activity diagram elements** | `UML:ActivityGraph`, `UML:ActionState`, `UML:Partition` not parsed | Add activity types + parsing |
| **XMI 2.1 support** | Reader/writer only handle UML 1.2 format (xmi.version="1.2") | Add XMI 2.x namespace handling, different attribute formats |
| **Foreign XMI dialect support** | NSUML, Unisys, Embarcadero variants not supported | Add dialect detection + format adapters |
| **Rose .mdl import** | Rational Rose format; C++ has `lib/petalnode` parser | Implement petal parser in Rust |
| **ArgoUML .zargo import** | Java-based ArgoUML format (ZIP + XMI) | Implement .zargo extraction + dialect adapter |
| **DTD/XSD validation** | No schema validation during XMI loading | Add optional validation pass |
| **Compression support in reader** | `.xmi.tgz`, `.xmi.tar.bz2` not loaded (writer supports compression) | Add decompression with flate2/bzip2 before parsing |
| **Full round-trip byte-identical test** | Current round-trip preserves semantics but reorders XMI — not byte-identical | Compare parsed model content, not XMI bytes |
| **Comprehensive XMI corpus** | Only one real XMI file (`test-COG.xmi`) in corpus tests | Collect and test against diverse real-world XMI files |

### Key XMI Reader Gaps (line-number references to reader.rs)

The XMI reader at `crates/uml-io/src/xmi/reader.rs` (~2416 lines) currently handles:
- Lines 100-1000: Generalization, Association, Dependency, Abstraction/Realization parsing
- Lines 1000-1500: Package, Class, Interface, Enum, DataType parsing
- Lines 1500-2000: Attribute, Operation, Parameter parsing
- Lines 2000+: Diagram, ViewNode, ViewEdge parsing

**Not handled:**
- `UML:Actor`, `UML:UseCase`, `UML:Component`, `UML:Node`, `UML:Artifact` — not parsed (silently skipped)
- `UML:TaggedValue` — stereotype properties not parsed
- `UML:Stereotype` — stereotype definitions not parsed
- `UML:Message` — sequence diagram messages not parsed
- `UML:State`, `UML:Transition` — state machine elements not parsed
- Foreign namespace elements (`org.omg.xmi.namespace.*`) — skipped

---

### Code Generation — MEDIUM Priority

| Generator | Current State | Required Work |
|-----------|--------------|---------------|
| **C++** | `crates/uml-codegen/src/cpp.rs` is a stub (empty struct + `CodeGenerator` impl) | Implement full C++ code generation: header guards, includes, class members, constructors, destructors, getters/setters |
| **Java** | `crates/uml-codegen/src/java.rs` is a stub | Implement full Java generation: package declaration, imports, class/interface, getters/setters |
| **Python** | `crates/uml-codegen/src/python.rs` is a stub | Implement Python generation: class definition, `__init__`, type annotations, docstrings |
| **Rust** | `crates/uml-codegen/src/rust.rs` is a stub | Implement Rust generation: `struct`, `impl`, `pub`/private, derive macros |
| **Ada** | Not started | — |
| **ActionScript** | Not started | — |
| **C#** | Not started | — |
| **D** | Not started | — |
| **IDL** | Not started | — |
| **JavaScript** | Not started | — |
| **Pascal** | Not started | — |
| **Perl** | Not started | — |
| **PHP** | Not started | — |
| **Ruby** | Not started | — |
| **SQL** | Not started | — |
| **Tcl** | Not started | — |
| **Vala** | Not started | — |
| **XML Schema** | Not started | — |

**Dependencies blocking generators:**
- `CodeWriter` at `crates/uml-codegen/src/writer.rs` has no unit tests (currently ~100 lines)
- `GeneratorRegistry` at `crates/uml-codegen/src/registry.rs` is a stub
- `ProgrammingLanguage` enum is defined but unused
- Multiple generators are gated behind Cargo features (`#[cfg(feature = "cpp")]`, etc.) that are not defined in `Cargo.toml`

---

### Code Import — MEDIUM Priority

| Feature | Current State | Notes |
|---------|--------------|-------|
| **C++ code import** | Not started | Available: tree-sitter-cpp grammar (tree-sitter dependency already in workspace) |
| **Java code import** | Not started | tree-sitter-java grammar available |
| **Python code import** | Not started | tree-sitter-python grammar available |
| **Other languages** | Not started | Could use tree-sitter grammars |
| **ImportRegistry** | Stub only | Similar to GeneratorRegistry pattern |

---

### Diagram Engine — MEDIUM Priority

| Feature | Current State | Required Work |
|---------|--------------|---------------|
| **Auto-layout algorithms** | `crates/uml-core/src/layout/mod.rs` is empty stubs | Implement graph-based layout (topological sort for hierarchies, Sugiyama for class diagrams) |
| **Sequence diagram layout** | Not started | Vertical timeline layout with message ordering |
| **Collaboration diagram layout** | Not started | Graph layout with numbered messages |
| **State diagram layout** | Not started | Orthogonal layout for states + transitions |
| **Activity diagram layout** | Not started | Swimlane + flow layout |
| **ER diagram layout** | Not started | Entity-relationship layout |
| **Edge routing optimization** | Waypoints exist in `ViewEdge` but unused | Waypoint simplification, orthogonal routing (avoiding node overlaps) |
| **Snap-to-grid** | Geometry exists (`Point`, `Rect` with grid-related fields) but not enforced | Grid snapping in UI drag operations |

---

### GUI — HIGH Priority

| Feature | Current State | Priority Notes |
|---------|--------------|----------------|
| **File open dialog** | Implemented in M16 via rfd native dialog + XMI loading | ~~HIGH~~ **DONE** |
| **File save / Save As** | Implemented in M16 via rfd native dialog + XMI writing | ~~HIGH~~ **DONE** |
| **Tool palette** | Implemented in M17 via vertical toolbar + click-to-place on canvas | ~~HIGH~~ **DONE** |
| **Property editor panel** | Implemented in M18 via right-side panel with name, visibility, flags, documentation, classifier details | ~~HIGH~~ **DONE** |
| **Resize handles** | Drag corner/edge handles not implemented | **MEDIUM** — nodes fixed size (can be worked around) |
| **Edge creation** | Click-and-drag to create relationships not implemented | **HIGH** — can't create relationships visually |
| **Zoom controls** | No slider, fit-to-window, zoom in/out | **MEDIUM** — essential for large diagrams |
| **Pan/scroll** | Middle-button drag to pan not implemented | **MEDIUM** — can't navigate large canvases |
| **Multiple diagram tabs** | No tabbed interface for switching diagrams | **MEDIUM** — can only view one diagram at a time |
| **Tree view with hierarchy** | Left panel shows flat element list | **MEDIUM** — should show package hierarchy tree |
| **Context menus** | No right-click menus on nodes/edges | **MEDIUM** — essential for element actions |
| **Dynamic node sizing** | Spec exists at `docs/ui_rich_rendering_spec_v1.md` but not implemented | **MEDIUM** — auto-calculate height from content |
| **Color customization** | Colors are hardcoded per element type | **LOW** — UI customization deferred |
| **Font customization** | Fonts are hardcoded | **LOW** — UI customization deferred |

---

### Undo/Redo — LOW Priority

| Feature | Current State | Notes |
|---------|--------------|-------|
| **Macro command support** | `Command::merge()` exists but no grouping API | Needed for multi-step operations (e.g., "create class + add attribute" as single undo step) |
| **Command serialization** | No save/restore for undo history | Session restore feature — low priority |
| **ChangeVisibility command** | Not implemented | Needed when property editor visibility control is built |
| **ChangeStereotype command** | Not implemented | Needed when property editor stereotype control is built |

---

### Testing — MEDIUM Priority

| Feature | Current State | Required Work |
|---------|--------------|---------------|
| **Property-based tests** | Planned but not implemented | Use `proptest` crate for model invariants (e.g., "remove then undo always restores state") |
| **Snapshot tests** | Planned but not implemented | Use `insta` crate for rendered output |
| **Fuzz testing** | Not started | Fuzz XMI parser with `cargo-fuzz` or `afl` |
| **Cross-tool verification** | Not started | Load XMI → Rust → save → verify in C++ Umbrello |
| **CI pipeline** | GitHub Actions exists but may need updating | Check workflow still builds and tests |
| **Performance benchmarks** | Not started | Use `criterion.rs` for model operation benchmarks |

---

### Infrastructure — LOW Priority

| Feature | Current State | Notes |
|---------|--------------|-------|
| **i18n/l10n** | No localization support | The C++ codebase has full KDE i18n — Rust needs `tr!()` equivalent |
| **Accessibility** | No screen reader support | Alt text, keyboard navigation, ARIA |
| **Benchmark suite** | Not started | Add `criterion.rs` benchmarks for model operations, XMI parsing, rendering |
| **Documentation generation** | Stub only | `mdBook` for user docs exists as empty `docs/` (not the same as architecture docs) |
| **Packaging** | Not started | Need `.deb`, `.rpm`, Flatpak, Windows MSI, macOS `.dmg` |
| **CLI export** | `--export` flag works but incomplete | CLI functionality from original Umbrello (see `--help`) |

---

## Build & Run

### Prerequisites

```sh
# Rust toolchain (via rustup)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
```

### Build

```sh
cd rust-rewrite

# Build everything (debug)
cargo build --workspace

# Build release
cargo build --release

# Build a specific crate
cargo build -p uml-core
cargo build -p uml-io
cargo build -p umbrello
```

### Test

```sh
# Run all tests
cargo test --workspace

# Run with display (for GUI tests — may need QT_QPA_PLATFORM=offscreen)
QT_QPA_PLATFORM=offscreen cargo test --workspace

# Run with output for debugging
cargo test --workspace -- --nocapture

# Run a specific test name pattern
cargo test -p uml-core serde_roundtrip

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Format
cargo fmt --all --check
cargo fmt --all
```

### Run

```sh
# Launch GUI with an XMI file
cargo run -p umbrello -- ../test/test-COG.xmi

# Launch GUI without a file (empty model)
cargo run -p umbrello

# Launch with any supported XMI file
cargo run -p umbrello -- path/to/your/model.xmi
```

### Common Issues

| Issue | Solution |
|-------|----------|
| `rustfmt` not found | `rustup component add rustfmt` |
| `clippy` not found | `rustup component add clippy` |
| Missing display for GUI tests | `QT_QPA_PLATFORM=offscreen cargo test` or `xvfb-run cargo test` |
| `cargo test` slow | `cargo test --workspace --no-fail-fast` (shows all failures at once) |
| Compilation errors after git pull | `cargo clean && cargo build` (stale lock file) |

---

## How to Contribute

### Workflow

1. **Pick a gap** from the [Completeness Gap Analysis](#completeness-gap-analysis) above.
2. **Read the design docs** in `docs/` relevant to the area (domain model, XMI, undo, etc.).
3. **Check the C++ reference** — find corresponding code in `../umbrello/`, `../lib/`, `../unittests/`.
4. **Implement incrementally** — add types, then XMI reader, then XMI writer, then GUI, then tests.
5. **Run tests** before submitting: `cargo fmt && cargo clippy -- -D warnings && cargo test`.
6. **Do not break existing tests** — `cargo test --workspace` must pass.

### Code Style

- Rust 2021 edition, `#![forbid(unsafe_code)]` in all library crates.
- `#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]` — document all public items.
- Module-level doc comments (`//!`) with architecture notes.
- Unit tests in `#[cfg(test)] mod tests` at file bottom.
- Prefer `Result<T, thiserror::Error>` over panics/`unwrap()`/`expect()`.
- Serde derives: `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`.
- Crate exports in `lib.rs` with re-exports: `pub use module::Type;`.

### Testing Requirements

Every PR must:
1. Add tests for new functionality.
2. Not reduce existing test pass count.
3. Run `cargo clippy --workspace --all-targets -- -D warnings` without warnings.
4. Run `cargo fmt --all --check` without changes.

For new element types, add:
- Unit tests for construction, name, ID, object_type
- Serde round-trip test (JSON)
- XMI reader test (parse from XML string)
- XMI writer test (write to XML, verify output)
- If applicable: diagram rendering test (in app.rs)

---

## Architecture Rules

### Core Principles

1. **Architecture-first.** Design docs in `docs/` before implementation. Review before code. Existing review documents (`milestone*_review.md`, `*_review.md`) document the architecture review process and conditions.

2. **No inheritance emulation.** Use traits, enums, and composition — never trait objects to simulate class hierarchies. The `ModelElement` enum pattern is the canonical dispatch mechanism.

3. **Core domain is 100% semantic UML.** `uml-core` must not contain any visual or GUI concepts (no egui, no colors, no fonts, no painter). `Rect`, `Point`, `Size` are data-structure types, not rendering types.

4. **ID-based references.** Elements reference each other by `UmlId`, never by raw pointers, indices, or references. The `UmlModel` owns all elements and provides lookup.

5. **All mutations via Commands.** Use `History::execute()` for every user-initiated model change. Direct mutation of `UmlModel` is permitted only during XMI loading (with `History::set_disabled(true)`).

6. **XMI compatibility.** `original_xmi_id` must be preserved on every `ElementBase`. Round-trip tests must validate that the parsed model maintains all structural properties after serialize+deserialize.

7. **cargo test --workspace must pass.** Never break existing tests. Always run the full suite.

8. **No modification of C++ source files.** The parent directory (`../umbrello/`, `../lib/`, `../unittests/`) is read-only reference material. All new code goes into `rust-rewrite/`.

### Crate Boundaries

```
uml-core (PURE)
  │
  ├── uml-io (depends on uml-core for model types)
  │     Uses: XmiReader → UmlModel, UmlModel → XmiWriter
  │     No GUI, no rendering
  │
  ├── uml-codegen (depends on uml-core for model types)
  │     Uses: UmlModel → source code
  │     No GUI, no rendering
  │
  └── apps/umbrello (depends on uml-core for all domain types)
        Uses: UmbrelloApp ← UmlModel + Diagram + Command
        All rendering, all GUI, all interaction
```

### Adding a New Element Type (Checklist)

- [ ] Add struct definition in `elements.rs`
- [ ] Add variant to `ModelElement` enum
- [ ] Add match arms in `ModelElement::base()`, `base_mut()`, `id()`, `name()`, `object_type()`, `is_classifier()`, `classifier_data()`
- [ ] Add match arm in `ModelElement::base()` (serde)
- [ ] Add to `NamedElement` impl
- [ ] Add XMI reader case in `uml-io/src/xmi/reader.rs`
- [ ] Add XMI writer case in `uml-io/src/xmi/writer.rs`
- [ ] Add rendering case in `apps/umbrello/src/app.rs` (`draw_partitioned_node`)
- [ ] Add visualization to `element_color()` in `app.rs`
- [ ] Write unit tests: creation, serde round-trip, XMI round-trip
- [ ] Run full test suite

---

## Reference: C++ Source Map

When implementing a feature, consult the C++ reference code at these locations:

| C++ Area | Path | Purpose |
|----------|------|---------|
| **Model objects** | `../umbrello/umlobject.h` | `UMLObject` base class (→ `ElementBase`) |
| **Class** | `../umbrello/umlclassifier.h/..` | Classifier inheritance chain |
| **Package** | `../umbrello/umlpackage.h` | Package with child elements |
| **Association** | `../umbrello/umlassociation.h` | 25+ association types (our 6 are subset) |
| **Basic types** | `../umbrello/basictypes.h` | Original enum definitions |
| **XMI reader** | `../umbrello/xmireader.h/..` | Reference for XMI parsing logic |
| **XMI writer** | `../umbrello/xmiwriter.h/..` | Reference for XMI serialization |
| **Undo/redo** | `../umbrello/commands/` | Qt undo framework commands |
| **Widgets** | `../umbrello/umlwidgets/` | C++ widget rendering (→ `app.rs`) |
| **Code gen** | `../umbrello/codegenerators/` | 15+ language code generators |
| **Code import** | `../umbrello/codeimport/` | Importers for C++, Java, Python, etc. |
| **Tests** | `../unittests/` | C++ test cases (Qt Test framework) |
| **Model files** | `../test/` | Sample .xmi files for testing |
| **Rose import** | `../lib/petalnode.*` | Rational Rose .mdl import (petal parser) |
| **PHP import** | `../lib/kdevplatform/` + `../lib/kdev5-php/` | PHP import (KDevelop dependency) |

---

## Document References

Key architecture documents in `docs/` to read before implementing:

| Document | Covers | Read Before |
|----------|--------|-------------|
| `domain_model_v1.md` | UML metamodel design, composition pattern, element types | Adding new element types |
| `model_repository_v1.md` | UmlModel arena, parent_index, cycle detection | Working with repository |
| `relationships_v1.md` | Relationship types, 6 AssociationType variants | Adding relationship types |
| `xmi_persistence_architecture_v1.md` | XMI reader/writer design, two-pass parsing, original_xmi_id | XMI-related work |
| `command_architecture_v1.md` | Command trait, History stack, concrete commands | Undo/redo work |
| `ui_rich_rendering_spec_v1.md` | Canvas rendering: zones, edges, arrowheads, dynamic sizing | GUI rendering work |
| `diagram_geometry_architecture_v1.md` | Point/Size/Rect data model, coordinate system | Diagram layout work |
| `workspace_consolidation_v2.md` | Crate boundaries, 21→5 consolidation | Crate structure questions |
| `testing_strategy.md` | Test philosophy, property-based testing plans | Adding tests |
| `phase1_architecture_audit.md` | Initial architecture decisions and rationale | Understanding design choices |

---

*Last updated: 2026-06-26 · Umbrello-RS Milestone 18*
