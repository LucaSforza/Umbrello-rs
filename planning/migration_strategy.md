# Umbrello-RS: Migration Strategy

> **Version:** 1.0
> **Date:** 2026-06-23
> **Status:** Draft — living document
> **Based on:** Comprehensive analysis of C++ Umbrello codebase (26.07.70-dev)

---

## Table of Contents

1. [Migration Philosophy](#1-migration-philosophy)
2. [Rewrite Strategy](#2-rewrite-strategy)
3. [Subsystem Extraction Order](#3-subsystem-extraction-order)
4. [Testing Strategy per Phase](#4-testing-strategy-per-phase)
5. [Compatibility Requirements](#5-compatibility-requirements)
6. [Risk Management](#6-risk-management)
7. [Timeline Estimate](#7-timeline-estimate)
8. [Crate Architecture](#8-crate-architecture)
9. [Decision Log](#9-decision-log)

---

## 1. Migration Philosophy

### 1.1 Core Principles

**This is a re-imagination, not a port.**

The goal is not to transliterate 500+ C++ source files into Rust, but to produce a tool that:
- Is architecturally modern (composition over inheritance, traits over virtual dispatch)
- Leverages Rust's strengths (memory safety, zero-cost abstractions, strong typing)
- Preserves the user-facing features and file formats of the original
- Remains maintainable and extensible for the next decade

### 1.2 Design Tenets

| Tenet | Implication |
|-------|-------------|
| **XMI Compatibility First** | All phases must preserve round-trip compatibility with C++ Umbrello XMI files. This is the bridge. |
| **Modern Rust Architecture** | Use enums instead of class hierarchies, traits instead of virtual base classes, generational arenas instead of raw pointers. |
| **Incremental Delivery** | Every phase produces working, testable, releasable code. No long-lived branches. |
| **Composition over Inheritance** | The C++ codebase has an 8-level deep class hierarchy rooted in `QObject`. The Rust version uses flat enums with composition. |
| **Strong Typing** | Make illegal states unrepresentable. Use newtypes for IDs, ownership, visibility, and type distinctions. |
| **Modularity** | Cargo workspace with fine-grained crates. No god objects. No circular dependencies. |
| **No Global Mutable State** | `UMLApp::app()` is referenced from 175+ call sites. Rust uses dependency injection and an explicit application context. |
| **Testing by Default** | Every crate has >90% coverage. Property-based tests for model operations, snapshot tests for rendering. |

### 1.3 What We Keep

- XMI 1.2 and 2.1 file format compatibility
- Feature parity for all 22 code generators and 10+ code importers
- Diagram types: Class, Sequence, Use Case, State, Activity, Collaboration/Communication, Component, Deployment, Entity Relationship
- All relationship types (association, aggregation, composition, dependency, generalization, realization, etc.)
- Undo/redo across all model mutations
- Auto-layout via GraphViz/petgraph

### 1.4 What We Change

- Flat enum-based type system instead of `QObject` hierarchy
- Generational arena storage instead of `QObject` parent-child trees
- Trait-based codegen and code import plugins
- `egui`/`vello` rendering pipeline instead of `QGraphicsView`
- CLI as a first-class citizen (not an afterthought)
- Pure Rust XMI parser/serializer (no `QDom`/`QXmlStreamWriter`)

---

## 2. Rewrite Strategy

### 2.1 Strangler Fig Pattern (Adapted)

The textbook approach is the [Strangler Fig pattern](https://martinfowler.com/bliki/StranglerFigApplication.html):
build new components alongside the old, gradually route functionality through new paths,
eventually remove the old.

**However**, Umbrello's architecture makes pure strangler fig infeasible:

```
Problem: Qt/QObject penetrates every class
  ┌──────────────────────────────────┐
  │ UMLObject : QObject             │  ← base of ALL model classes
  │ QGraphicsObject                  │  ← base of ALL widget classes
  │ QGraphicsView → QWidget → QObject  ← rendering
  └──────────────────────────────────┘
There is no "pure model layer" that can be extracted without touching 500+ files.
```

### 2.2 The XMI Bridge

Instead of incremental replacement of C++ components, we use **parallel development
with shared XMI format as the bridge**:

```
┌──────────────────────┐       XMI file        ┌──────────────────────┐
│  C++ Umbrello 5/6    │ ◄──────────────────►  │  Umbrello-RS (Rust)  │
│                      │   (bidirectional)      │                      │
│  Full GUI + editing  │                        │  Starting: CLI only  │
│  All codegens        │                        │  Growing: phases 1-9 │
│  All importers       │                        │  Eventually: full GUI│
└──────────────────────┘                        └──────────────────────┘
```

**Key implications:**
- Both versions read/write the same XMI files
- Users can switch between tools on the same project files
- No migration needed — just save in one, open in the other
- Validation: save in C++, load in Rust, compare in-memory model, save in Rust,
  load in C++, compare in-memory model, repeat
- Once Umbrello-RS reaches feature parity, C++ version enters maintenance mode
  (critical bug fixes only)

### 2.3 Development Model

```
Milestone 1: Foundation  ──► CLI can load XMI, inspect model, validate
         │
Milestone 2: Core Model  ──► CLI can create/modify/save models, round-trip
         │
Milestone 3: Persistence ──► Full save/load pipeline with compression
         │
Milestone 4: CLI          ──► Export (PNG, SVG), import basics
         │
Milestone 5-6: Code       ──► Code import + code generation (MVP languages)
         │
Milestone 7: Diagram      ──► Diagram model + layout
         │
Milestone 8: Rendering    ──► First visual output (PNG export)
         │
Milestone 9: GUI App      ──► Interactive editor replaces C++ for daily use
         │
Milestone 10: Polish      ──► Feature parity, i18n, packaging
```

Each milestone is 4–12 weeks. Between milestones, the software is usable
(even if only as a CLI tool or library).

### 2.4 Dogfooding Roadmap

| Phase | Dogfooding Use Case |
|-------|---------------------|
| 1–2 | CI validates Rust XMI output matches C++ XMI output |
| 3 | CI runs full save/load round-trip through both implementations |
| 4 | CLI used in CI pipelines for validation and export |
| 5–6 | Code import can be tested by importing C++ Umbrello's own source |
| 7 | CLI can generate layout for existing projects |
| 8 | Exported diagrams can be visually compared with C++ output |
| 9 | Team switches to Umbrello-RS for daily UML editing |
| 10 | Release to KDE community as beta |

---

## 3. Subsystem Extraction Order

The order is determined by **dependency analysis**: each phase builds only on
preceding phases. Phases 1–4 have no GUI code and no Qt dependency. Phases 5–6
depend only on the core model. Phases 7+ introduce visual and interactive elements.

### Phase 1: Foundation (Weeks 1–4)

**Goal:** Cargo workspace, core types, XMI I/O, compilation and testing infra.

**No C++ dependencies.** Pure Rust, only `serde`, `quick-xml`, `thiserror`.

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| Cargo workspace | Root workspace with 5–8 initial crates | CMake build system |
| `umbrello-types` | All enums: `ObjectType`, `AssociationType`, `DiagramType`, `VisibilityKind`, `Scope`, `ParmKind`, `SignatureType`, `ChangeFlag`, `DrawType`, etc. | `umlobject.h`, `association.h`, `umlview.h` — scattered enums |
| `umbrello-id` | UUID-based ID generation (wraps `uuid` crate) | `UniqueID` in `umldoc.h` |
| `umbrello-xmi` | XMI 1.2/2.1 read/write — `quick-xml` reader + `serde` tags. Handles header, documentation, namespace, XMI extensions. | `UMLDoc::saveToXMI()` / `loadFromXMI()` |
| `umbrello-error` | Error types: `ModelError`, `XmiError`, `IoError`, `CodegenError` | Scattered error handling |
| `umbrello-test` | Test helpers: XMI fixture loader, round-trip assertion macros | `unittests/testbase.h` |

**Key decisions:**
- XMI writing uses `quick-xml` writer (streaming, low allocation)
- XMI reading uses `quick-xml` events (streaming, no DOM)
- `uuid` v4 for all new IDs; accept C++ integer IDs on import
- All enums derive `Serialize`/`Deserialize`, `Copy`, `Clone`, `PartialEq`, `Hash`, `Display`

**Risk:** XMI parsing edge cases. Mitigate: parse all test XMI files from C++ test suite.

**Tests:**
- [x] All enums round-trip through string representation
- [x] ID generation produces valid UUIDs
- [x] Parse minimal valid XMI 1.2 and 2.1 files
- [x] Parse real-world XMI files from C++ test suite
- [x] Error handling on malformed XMI
- [x] Namespace prefix resolution

---

### Phase 2: Core Model (Weeks 5–12)

**Goal:** All UML model types as pure data structures, arena-based storage, mutation API, XMI round-trip.

**Depends on:** Phase 1 (types, XMI)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| `umbrello-model` crate | All UML model types as Rust structs | `umlmodel/` (69 files) |
| Arena storage | Generational index-based `UmlArena<T>` | `UMLObjectList`, `UMLDoc::m_objects` |
| `UmlModel` | Root model document — owns all arenas, root folders | `UMLDoc` |
| `UmlFolder` | Named container with children | `UMLFolder` |
| `UmlPackage` / `UmlClassifier` | Package and classifier types | `UMLPackage`, `UMLClassifier` |
| `UmlClass` / `UmlInterface` / etc. | Concrete classifier variants | `UMLClass`, `UMLInterface`, etc. |
| `UmlAssociation` | Relationship between two model elements | `UMLAssociation` |
| `UmlAttribute` / `UmlOperation` | Classifier features | `UMLAttribute`, `UMLOperation` |
| `UmlStereotype` | Stereotype definitions | `UMLStereotype` |
| Mutation API | `ModelMut` trait — create, delete, modify with change events | `Object_Factory`, direct setters |
| Change events | `ModelChange` enum — emitted on mutations | Qt signals |
| XMI round-trip | `impl XmiSerializable for UmlClass` — 40+ implementations | `saveToXMI` / `loadFromXMI` per class |
| Cross-reference resolution | Two-pass load: 1) create all objects, 2) resolve `UmlRef` | `XMIRefResolver` / `m_SecondaryId` |

**Key decisions:**
- **No `ObjectType` enum dispatch.** Use a top-level `UmlObject` enum with variants:
  ```rust
  pub enum UmlObject {
      Class(UmlClass),
      Interface(UmlInterface),
      Enum(UmlEnum),
      Attribute(UmlAttribute),
      Operation(UmlOperation),
      Association(UmlAssociation),
      Folder(UmlFolder),
      // ... one variant per concrete type
  }
  ```
  This replaces the C++ `QObject` → `UMLObject` → `UMLCanvasObject` → ... hierarchy.
- **No raw pointers.** All cross-references use `GenerationalIndex` (from `generational-arena` or custom).
  The arena is the single source of truth.
- **Mutability is explicit.** The `ModelMut` trait returns `Result` and requires `&mut self`.
  Immutable access is via `ModelQuery` trait with `&self`.
- **Stereotypes are strings** (with validation) rather than reference-counted objects, at least initially.
  Optimize later if needed.
- **Change events** use a simple callback or channel (`tokio::sync::watch` or a custom `SlotVec`),
  not Qt signals.

**Parallel work opportunity:** Multiple people can implement individual `XmiSerializable`
impls once the pattern is established (40+ types × ~2 days each = ~10 weeks sequential,
~2 weeks with 5 people).

**Tests:**
- [x] Each model type: create → serialize to XMI → parse back → assert equality
- [x] Full model: load C++-generated XMI → serialize Rust XMI → compare XML trees (canonicalized)
- [x] Cross-reference resolution with forward references
- [x] Arena: create, delete (with generational check), iterate
- [x] Mutation: create object, modify property, delete, verify change events
- [x] Stereotype assignment and resolution
- [ ] Property-based: random mutations preserve model invariants

---

### Phase 3: Persistence & Undo (Weeks 13–16)

**Goal:** File load/save pipeline, compression, foreign format import, undo/redo.

**Depends on:** Phase 2 (model)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| File format detection | Detect `.xmi`, `.xmi.tgz`, `.xmi.tar.bz2`, `.rose`, `.argo` | `UMLDoc::openDocument()` |
| Compression layer | `flate2` for gzip, `tar` for archive handling | `KTar` |
| File I/O pipeline | `fn load(path) -> Result<UmlModel>` + `fn save(path, model) -> Result<()>` | `UMLDoc::saveDocument()` |
| Foreign import: Rose | Parse Rational Rose MDL files → `UmlModel` | `roseimport.h` |
| Foreign import: ArgoUML | Parse ArgoUML PGML → `UmlModel` | `argoumlimport.h` |
| Undo/redo | Command pattern: `UndoStack<Command>` | `umbrello/cmds/` |
| `Command` trait | `trait Command { fn execute(), fn undo(), fn merge(), fn name() }` | `QUndoCommand` |
| Concrete commands | `CreateObject`, `DeleteObject`, `SetProperty`, `MoveWidget`, `ResizeWidget` | 20+ command classes |

**Key decisions:**
- Atomic save: write to temp file, rename on success (like C++ version)
- Compression uses pure Rust (`tar` + `flate2`), no KArchive dependency
- Foreign format importers are separate optional crates
- Undo/redo commands mirror the trait approach from Phase 2's mutation API
- Command merging: adjacent property changes coalesce

**Tests:**
- [x] Save plain `.xmi` → load → compare model
- [x] Save compressed `.xmi.tgz` → load → compare model
- [x] Save → load in C++ → save again → load in Rust → identical
- [x] Round trip: Rust → C++ → Rust → C++ (4-way comparison)
- [x] Rose import: known test files
- [x] ArgoUML import: known test files
- [x] Undo: execute → undo → verify model state
- [ ] Redo: execute → undo → redo → verify model state
- [x] Command merging: verify coalesced undo steps

---

### Phase 4: CLI (Weeks 17–20)

**Goal:** Command-line tool for all non-interactive operations.

**Depends on:** Phase 3 (persistence)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| `umbrello-cli` crate | Binary target with `clap` argument parsing | `main.cpp` (thin) |
| `umbrello-rs` binary | Workspace root binary — CLI + future GUI | `umbrello5`/`umbrello6` |
| `load` command | Load XMI file, validate, print summary | `--export` etc. |
| `validate` command | Validate XMI against DTD, check model consistency | (none in C++) |
| `export` command | Export diagrams to PNG/SVG | `--export` |
| `convert` command | Convert between XMI 1.2 and 2.1 | (partial) |
| `info` command | Print model statistics (class count, relationship count, etc.) | (none in C++) |
| `tree` command | Print model as indented tree | `UMLListView` (GUI only) |
| `query` command | Find objects by name, type, stereotype | (none in C++) |
| `import` command (basic) | Import source files → update XMI | `--import-files` |
| `diff` command | Compare two XMI files | (none in C++) |
| Exit codes | Consistent: 0 = success, 1 = error, 2 = validation failure | (inconsistent) |

**Key decisions:**
- CLI is the primary tool for CI/CD integration (e.g., validate UML models in CI pipelines)
- Export uses the same rendering pipeline as the GUI (Phase 8), just without interaction
- All CLI commands work on `.xmi` and `.xmi.tgz` files transparently
- Human-readable output (color, tables) vs machine-readable (JSON, JSON Lines)

**Parallel work opportunity:** CLI commands can be built by different people once the
model and persistence APIs are stable.

**Tests:**
- [x] All commands have `--help` output tested
- [x] `load` → `save` round-trip
- [x] `validate` on valid/invalid files
- [x] `export` produces non-empty PNG/SVG
- [x] `convert` between XMI versions
- [x] `diff` of identical and different files
- [ ] Exit code verification for all commands
- [x] Integration tests using files from C++ test suite

---

### Phase 5: Code Import — Plugin Architecture (Weeks 21–28)

**Goal:** Language importers as pluggable crates, tree-sitter integration.

**Depends on:** Phase 2 (model types)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| `CodeImport` trait | `trait CodeImport { fn import(&self, source: &str) -> Result<UmlPackage> }` | `Import_Utils` + per-language classes |
| Tree-sitter integration | Common language parser using `tree-sitter` bindings | `lib/cppparser/` (hand-written C++ parser) |
| C++ importer | Parse C++ headers → `UmlClass`, `UmlOperation`, etc. | `codeimport/cppimport.*` |
| Java importer | Parse Java → UML | `codeimport/javaimport.*` |
| Python importer | Parse Python → UML | `codeimport/pythonimport.*` |
| Other importers | Ada, C#, IDL, Pascal, SQL, Vala | 6+ importer classes |
| Import context | File discovery, resolve order, error collection | `Import_Utils` |
| Type resolution | Map language types → UML types, resolve qualified names | `ClassifierCacher` |

**Key decisions:**
- Tree-sitter provides a unified parsing foundation across 15+ languages
- Each importer is a separate crate in the workspace, enabling optional compilation
- C++ parser: use `tree-sitter-cpp` instead of the hand-written `lib/cppparser/` (which
  is 13 KLOC of C++ — not worth porting)
- PHP import: use `tree-sitter-php` instead of `kdevplatform` (removes KDE dependency)
- Importers produce a `UmlPackage` tree; the caller attaches it to the model
- Error recovery: import as much as possible, collect errors, don't fail on first issue

**Parallel work opportunity:** Each language importer can be developed independently
once the trait and tree-sitter integration are defined.

**Tests:**
- [ ] Each importer: parse known source file → compare UML output with expected
- [ ] C++: parse subset of Umbrello's own headers → verify all classes found
- [ ] Java: parse simple class hierarchy → verify inheritance
- [ ] Python: parse with decorators → verify stereotypes
- [ ] Round-trip: import source → export XMI → import XMI → compare
- [ ] Error collection: import malformed file → verify partial results + errors
- [ ] Type resolution: cross-file references resolved correctly

---

### Phase 6: Code Generation — Plugin Architecture (Weeks 29–36)

**Goal:** Code generators as pluggable crates, template engine integration.

**Depends on:** Phase 2 (model types)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| `CodeGenerator` trait | `trait CodeGenerator { fn generate(&self, model: &UmlModel, pkg: &UmlPackage) -> Result<GeneratedCode> }` | `CodeGenPolicy` + per-language writers |
| Template engine | Tera or Handlebars-based code generation | String concatenation + writers |
| Common code model | `CodeBlock`, `CodeClassField`, `CodeMethod`, etc. | `codegenerators/codeelements/` |
| C++ generator | Generate C++ from UML | `codegenerators/cpp/` |
| Java generator | Generate Java from UML | `codegenerators/java/` |
| Python generator | Generate Python from UML | `codegenerators/python/` |
| Other generators | 19 more languages | 19 writer classes |
| `GenerateResult` | Generated files: file path + content | `CodeDocument` |

**Key decisions:**
- Template-based generation (Tera templates) instead of string concatenation in code.
  Templates are embedded in the binary at compile time, but also loadable from a
  user-configurable directory.
- The common code model (class fields, methods, etc.) is shared across generators,
  not re-implemented per language.
- Each generator is a separate crate with its own templates.
- Generation is deterministic: same model → same output (barring template engine version).
- Override mechanism: users can replace individual templates.

**Parallel work opportunity:** Each language generator can be developed independently
once the trait and template engine are defined.

**Tests:**
- [ ] Each generator: generate from known model → compare output with expected
- [ ] Round-trip: create class in GUI → generate code → import code → compare model
- [ ] C++ round-trip: generate C++ → import C++ → compare model
- [ ] Template override system
- [ ] Determinism: same input → identical output (hash comparison)
- [ ] 22 generators × minimal test case each

---

### Phase 7: Diagram Model (Weeks 37–42)

**Goal:** Diagram data structures, positions, layout algorithms.

**Depends on:** Phase 2 (model types)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| `Diagram` struct | Metadata: type, name, zoom, grid settings | `UMLView` |
| `WidgetData` enum | All widget types: `ClassWidget`, `NoteWidget`, `BoxWidget`, etc. | `UMLWidget` hierarchy (94 files) |
| `WidgetPos` | Position, size, Z-order on the diagram | `QGraphicsItem::pos()` |
| `AssocWidgetData` | Visual association: line path, label positions, color | `AssociationWidget` |
| Diagram → model binding | Each widget references an `UmlRef` (model element ID) | `UMLWidget::m_pObject` |
| XMI for diagrams | Save/load diagram layout in XMI extension | `<diagrams>` in `XMI.extension` |
| Layout engine | petgraph-based topological layout for class diagrams | `layoutgenerator.*` (GraphViz) |
| Grid layout | Even spacing for new widgets | (implicit in scene) |

**Key decisions:**
- Diagram data is **separate from model data**. The model doesn't know about positions.
- `WidgetData` is an enum with ~30 variants — mirrors `UmlObject` approach.
- Layout uses `petgraph` for graph algorithms + custom layout logic.
  Optionally call out to GraphViz `dot` for complex layouts.
- Diagram XMI is stored in the `<XMI.extension>` element (same as C++), preserving
  compatibility.
- Floating text labels on associations are part of `AssocWidgetData`.

**Tests:**
- [ ] All 30 widget types serialize to XMI and back
- [ ] Diagram with 10+ widgets and associations round-trips
- [ ] Layout: arrange a class diagram and verify non-overlapping
- [ ] XMI compatibility: load C++ diagram layout, compare widget positions

---

### Phase 8: Diagram Rendering (Weeks 43–52)

**Goal:** Render diagrams to raster/vector output; interactive canvas for GUI.

**Depends on:** Phase 7 (diagram model)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| Rendering backend trait | `trait Renderer { fn draw_rect, fn draw_text, fn draw_line, ... }` | `QPainter` |
| `tiny-skia` backend | Raster rendering for PNG export | `QPainter` + `QPixmap` |
| `resvg` backend | SVG rendering for vector export | `QSvgGenerator` |
| Widget rendering | 30 widget drawers (`draw_class_widget`, `draw_note_widget`, etc.) | `UMLWidget::paint()` |
| Association rendering | Line routing, arrowheads, label placement | `AssociationWidget::paint()` |
| Sequence diagram rendering | Lifelines, activation bars, messages | `MessageWidget`, `SequenceLineWidget` |
| Canvas abstraction | Coordinates, viewport, zoom, pan | `QGraphicsView` + `QGraphicsScene` |
| Hit testing | Point-to-widget, point-to-association | `QGraphicsItem::contains()` |
| Export pipeline | Model → Diagram → Render → PNG/SVG bytes | `UMLView::saveToPng/Svg()` |

**Key decisions:**
- Two-tier rendering: a backend trait + concrete implementations.
  Makes it easy to add `wgpu`, `vello`, or WebGPU backends later.
- SVG export uses `resvg` for correct SVG rendering (not hand-rolled XML).
- Rendering is **stateless**: given a diagram model + renderer, produce output.
  No retained state in widgets.
- Sizes and fonts are computed from the diagram model, not from runtime widget metrics.
- Association routing: port the C++ line routing algorithm (it's a significant piece).

**Parallel work opportunity:** Widget rendering implementations can be split across
team members (30 widgets × ~2 days each = ~10 weeks, parallelizable to ~2 weeks).

**Tests:**
- [ ] Each widget type renders to PNG (snapshot test, compare with C++ generated PNG)
- [ ] Each widget type renders to SVG (string comparison, canonicalized)
- [ ] Association rendering: 10+ association types
- [ ] Sequence diagram: lifeline + message rendering
- [ ] Zoom: 25%, 50%, 100%, 200% renders
- [ ] Viewport: pan across large diagram
- [ ] Export matches C++ output pixel-for-pixel (for known widgets)

---

### Phase 9: GUI Application (Weeks 53–68)

**Goal:** Full interactive diagram editor replacing C++ Umbrello for daily use.

**Depends on:** Phase 8 (rendering), Phase 4 (CLI framework)

**GUI framework selection:** To be decided during Phase 1 prototyping.
Primary candidates: `egui` (immediate mode, pure Rust), `slint` (declarative, pure Rust),
`iced` (elm-architecture, pure Rust), or lightweight Qt bindings (rust-qt-bindings,
only if necessary). The decision will be documented in a separate GUI framework evaluation.

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| Application context | `AppContext` — dependency injection container | `UMLApp` (god object) |
| Window management | Main window + multiple diagram tabs | `UMLApp::m_tabWidget` |
| Dock widgets | Tree view (model browser), properties, bird view, command history | 6+ dock widgets |
| Menu system | File, Edit, View, Diagram, Code, Settings menus | `KXmlGuiWindow` + `umbrelloui.rc` |
| Toolbars | Main toolbar + work toolbar (drawing tools) | `KToolBar` |
| Status bar | Messages, zoom slider | `KStatusBar` |
| Canvas widget | Interactive diagram area with zoom/pan | `UMLView` + `UMLScene` |
| Widget interaction | Click, drag, resize, select, multi-select | `QGraphicsItem` event handlers |
| Drawing tools | Class tool, association tool, note tool, etc. | `ToolBarState` state machine |
| Property dialogs | Edit class properties, association properties, etc. | 80+ dialog classes |
| Model browser | Tree view showing all model elements | `UMLListView` |
| Welcome screen | Recent files, templates | `WelcomePageWidget` |
| Settings | Configuration dialog + persistent settings | `umbrello.kcfg` + KConfig |
| i18n | Fluent-based localization | `ki18n` + `po/` (62 languages) |
| Code generation UI | Wizard for code generation | `codegenwizard/` |

**Key decisions:**
- No direct port of dialogs. Each dialog is redesigned for the Rust UI framework.
- The interaction state machine (`ToolBarState`) is designed as an enum state machine,
  not a class hierarchy (12 classes in C++).
- Menu and toolbar definitions use a JSON or TOML config file, not XMLGUI XML.
- Settings use `confy` or `serde` + TOML for persistent config (not KConfig).
- i18n uses `rust-embed` + Fluent `ftl` files. Translation files initially cover
  English only; community contributions for other languages can follow.
- The tree view uses a custom widget with the UI framework's tree widget, not `QTreeView`.

**Tests:**
- [ ] Main window opens without crash
- [ ] Create, edit, save, re-open document
- [ ] All menu items activate correct actions
- [ ] Dock widgets show correct content
- [ ] Model browser: create class → appears in tree
- [ ] Canvas: add widget, move, resize, delete
- [ ] All association types drawable
- [ ] Undo/redo from GUI
- [ ] Settings persist across sessions
- [ ] i18n: language switching

---

### Phase 10: Enhancement & Polish (Weeks 69–80+)

**Goal:** Feature parity with C++ Umbrello, performance optimization, packaging.

**Depends on:** Phase 9 (GUI)

| Deliverable | Description | C++ Counterpart |
|-------------|-------------|-----------------|
| Bird view (minimap) | Overview of entire diagram | `BirdView` |
| Refactoring assistant | Rename, move, extract operations | `refactoring/` |
| DocBook/XHTML generation | Generate documentation from model | `docgenerators/` |
| Search/find | Find objects across model and diagrams | `finder/` |
| Clipboard | Copy/paste of model elements + widgets | `clipboard/` |
| Print support | Print and print preview | QPrinter + QPrintDialog |
| LSP integration | Language Server Protocol for UML | (new feature) |
| Performance optimization | Benchmarking, memory profiling, where needed | — |
| Packaging | AppImage, Flatpak, snap, distribution packages | — |
| Migration guide | How to switch from C++ Umbrello to Umbrello-RS | — |
| C++ maintenance | Critical bug fixes only | — |

**Key decisions:**
- LSP integration is a stretch goal — provides code completion for UML model editing
  (e.g., "implement interface" as code action)
- Clipboard uses internal format based on XMI fragments
- Refactoring assistant builds on the undo/redo system and mutation API

**Tests:**
- [ ] Search: find by name, type, stereotype across model
- [ ] Clipboard: copy class → paste into another diagram
- [ ] Print: output matches PNG export
- [ ] Refactoring: rename class updates all references
- [ ] Performance: load 10,000 element model in < 5 seconds

---

## 4. Testing Strategy per Phase

### 4.1 Phase-Specific Testing

| Phase | Test Type | Tools | Target |
|-------|-----------|-------|--------|
| 1 | Unit tests | `cargo test` | 100% of public API |
| 1 | XMI parse | Custom test harness + C++ XMI corpus | All C++ test XMI files |
| 2 | Unit + round-trip | `cargo test` | Every model type |
| 2 | Property-based | `proptest` | Model invariants |
| 2 | XMI round-trip | Rust ↔ C++ comparison | Known XMI files |
| 3 | Integration | Temp files + model compare | All compression formats |
| 3 | Undo/redo | Exhaustive command sequences | All command types |
| 4 | CLI integration | `assert_cmd` / `trycmd` | All CLI commands |
| 5 | Importer tests | Source → UML comparison | All importers |
| 6 | Generator tests | UML → code comparison | All generators |
| 7 | Serialization round-trip | `proptest` | All diagram data |
| 8 | Snapshot tests | PNG comparison against C++ output | All widget types |
| 9 | GUI integration | UI framework testing tools | Main workflows |
| 10 | Benchmark | `criterion` | Model operations, rendering |

### 4.2 Continuous Integration

```yaml
# .github/workflows/ci.yml (conceptual)
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo test --workspace
      - run: cargo clippy --workspace -- -D warnings
      - run: cargo fmt --check

  roundtrip:
    runs-on: ubuntu-latest
    steps:
      - run: ci/roundtrip.sh  # Save in C++, load in Rust, save in Rust, load in C++

  snapshot:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test -p umbrello-render --test snapshots

  benchmark:
    runs-on: ubuntu-latest
    steps:
      - run: cargo bench --workspace
```

### 4.3 Round-Trip Testing Pipeline

The most critical test is the cross-implementation round-trip:

```
┌──────────────────────────────────────────────────┐
│  Round-Trip Test Pipeline                         │
│                                                   │
│  1. C++ Umbrello loads known .xmi file            │
│  2. C++ saves to .xmi (reference A)               │
│  3. Umbrello-RS loads reference A                  │
│  4. Compare in-memory model (Rust) with expected   │
│  5. Umbrello-RS saves to .xmi (round-trip B)       │
│  6. C++ loads round-trip B                         │
│  7. Compare in-memory model (C++) with expected    │
│  8. C++ saves to .xmi (round-trip C)               │
│  9. Compare reference A with round-trip C          │
│     (canonicalized XML tree diff)                  │
│                                                   │
│  Acceptance: A ≅ C  (XML-tree identical)           │
│         AND  model(Rust) = expected                │
│         AND  model(C++) = expected                 │
└──────────────────────────────────────────────────┘
```

### 4.4 Property-Based Testing

For model operations, use `proptest` to generate random mutation sequences and
verify invariants:

```rust
proptest! {
    #[test]
    fn model_invariants_preserved(ops in gen_mutation_sequence(1..100)) {
        let mut model = UmlModel::new();
        for op in ops {
            match op {
                Mutation::CreateClass { name, .. } => {
                    model.create_class(&name)?;
                    // Invariant: all IDs are unique
                    assert_unique_ids(&model);
                }
                Mutation::DeleteObject { id } => {
                    model.delete_object(id)?;
                    // Invariant: no dangling references
                    assert_no_dangling_refs(&model);
                }
                // ...
            }
        }
    }
}
```

Invariants to check:
- All object IDs are unique
- No dangling cross-references
- Parent-child consistency (parent.contains(child))
- Association endpoints reference existing objects
- Folder tree is acyclic
- Package containment hierarchy is consistent

---

## 5. Compatibility Requirements

### 5.1 XMI File Format

The XMI file format is the **sole compatibility contract**. Both implementations must
produce byte-identical XMI for the same model (modulo canonicalization of whitespace
and namespace prefixes).

```xml
<!-- XMI 1.2 format (Umbrello default) -->
<XMI xmi.version="1.2" xmlns:UML="omg.org/UML1.3">
  <XMI.header>...</XMI.header>
  <XMI.content>
    <UML:Model xmi.id="..." name="..." isLeaf="false" isRoot="true" isAbstract="false">
      ...
    </UML:Model>
  </XMI.content>
  <XMI.extension>
    <diagrams>...</diagrams>
    <docsettings>...</docsettings>
    <listview>...</listview>
    <codegenerator>...</codegenerator>
  </XMI.extension>
</XMI>

<!-- XMI 2.1 format -->
<xmi:XMI xmi:version="2.1" xmlns:uml="http://www.omg.org/spec/UML/20110701">
  <xmi:Documentation>...</xmi:Documentation>
  <uml:Model xmi:id="...">...</uml:Model>
  <diagrams>... (as XMI.extension sibling) ...</diagrams>
</xmi:XMI>
```

### 5.2 Compatibility Requirements

| Requirement | Verification | Criticality |
|-------------|-------------|-------------|
| XMI 1.2 read | Load C++-generated `.xmi` files | **Critical** |
| XMI 1.2 write | C++ reads Rust-generated `.xmi` files | **Critical** |
| XMI 2.1 read | Load C++-generated `.xmi` files (UML2 mode) | **Critical** |
| XMI 2.1 write | C++ reads Rust-generated `.xmi` files (UML2 mode) | **Critical** |
| `.xmi.tgz` read | Load compressed C++ files | High |
| `.xmi.tgz` write | C++ reads compressed Rust files | High |
| Compressed round-trip | Rust ↔ C++ via compressed files | High |
| All model types round-trip | Every `ObjectType` (28+) survives round-trip | **Critical** |
| Diagram layout round-trip | Widget positions preserved | High |
| Stereotype round-trip | Stereotype assignments preserved | High |
| Tagged values round-trip | Custom tagged values preserved | Medium |
| Code gen state round-trip | Code generator settings preserved | Medium |
| List view state round-trip | Tree view expand/collapse state | Low |
| XMI extension elements | Non-standard elements preserved | Medium |
| DTD validation | XMI 1.2 conforms to DTD | Medium |
| Foreign format (Rose) | Read-only compatibility | Low |
| Foreign format (ArgoUML) | Read-only compatibility | Low |

### 5.3 Canonicalization

For comparing XMI output, we define a canonical form:

1. Sort namespace declarations alphabetically
2. Sort attributes within each element alphabetically (except `xmi.id` which comes first)
3. Sort child elements by type then by `xmi.id` or `name`
4. Normalize whitespace: indent 2 spaces, no trailing whitespace
5. Normalize `xmi.id` values (UUID format)
6. Ignore `timestamp` in docsettings (if present)

### 5.4 ID Compatibility

- **On read:** Accept both integer IDs (C++ format: `m_nId` as `int`) and UUID IDs
  (Rust format). Convert integers to UUIDs using a namespace-based scheme:
  `UUID::from_u64_pair(0, id as u64)`.
- **On write:** Write UUIDs in format `"UML-RS-xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"` for
  Rust-generated files. The C++ implementation must accept these on re-import.
  (If compatibility requires, provide a write mode that uses integer IDs.)
- **Cross-reference resolution:** All `xmi.id` references are resolved using a
  two-pass strategy: collect all IDs on first pass, resolve references on second pass.

### 5.5 DTD Validation

XMI 1.2 defines a DTD. The Rust XMI implementation should:
1. Produce valid DTD-conformant XMI 1.2 (the C++ version is mostly DTD-conformant)
2. Accept non-DTD-conformant XMI (the C++ version is lenient)
3. Provide a `--strict-dtd` flag for validation

XMI 2.1 is schema-based rather than DTD-based. The Rust implementation uses
namespace-aware parsing and schema validation (optional).

---

## 6. Risk Management

### 6.1 Risk Register

| # | Risk | Likelihood | Impact | Phase | Mitigation |
|---|------|------------|--------|-------|------------|
| R1 | **XMI incompatibility** prevents round-trip | Medium | **Critical** | 1–3 | Extensive round-trip test suite; weekly cross-build testing; canonical XML comparison |
| R2 | **Rendering quality** doesn't match C++ output | Medium | High | 8 | Pixel-level snapshot tests; visual comparison CI; layout algorithm parity |
| R3 | **Performance** regression vs C++ | Medium | Medium | 2+ | Early benchmarks; criterion CI; profile-guided optimization |
| R4 | **Feature loss** — some C++ feature not implemented | Low | Medium | 5–10 | Feature parity checklist; community input on prioritization |
| R5 | **Scope creep** — trying to do too much | High | High | All | Strict phase boundaries; MVP definition per phase; cut scope rather than delay |
| R6 | **Team availability** — loss of contributors | Medium | Medium | All | Documentation-first approach; modular architecture enables parallel work; clear onboarding path |
| R7 | **GUI framework choice** limits functionality | Medium | High | 9 | Prototype 3 candidate frameworks in Phase 1; define GUI requirements early; have fallback plan |
| R8 | **Tree-sitter integration** complexity for niche languages | Low | Low | 5 | Support hand-written parsers as fallback; community contributions for niche languages |
| R9 | **Qt/KDE users** resist migration | Medium | Low | 10 | XMI bridge enables coexistence; clear migration guide; maintain C++ version during transition |
| R10 | **i18n effort** for 62 languages | High | Medium | 9 | Automated translation tooling; community-driven translations; launch with English only |

### 6.2 Mitigation Strategies

**R1: XMI Incompatibility**
- Mandatory: every PR touching XMI code must add or update a round-trip test
- Weekly: full round-trip pipeline (Rust → C++ → Rust → C++) in CI
- Always: canary test against the 10 most complex C++ XMI files from the real codebase
- Fuzz testing: generate random model mutations, write XMI, re-read, verify equality

**R2: Rendering Quality**
- Snapshot test framework: compare PNG output of Rust and C++ rendering for same diagram
- Pixel-diff threshold: < 0.1% different pixels for known diagrams
- Font rendering: use same font (Liberation Sans) and font metrics
- Grid alignment tests: widget positions match C++ within 1 pixel

**R3: Performance**
- Set performance budgets before optimization:
  - Load 10,000 element model: < 5 seconds (C++: ~3 seconds)
  - Save 10,000 element model: < 3 seconds (C++: ~2 seconds)
  - Render 100-widget diagram: < 500ms (C++: ~200ms)
- Benchmark key operations every PR
- Profile-guided optimization (PGO) for release builds
- Benchmark suite in `cargo bench`

**R4/R5: Scope Management**
- Each phase has a clear MVP definition: "what must work to ship this phase"
- Features beyond MVP are tracked in a backlog and deferred
- Quarterly review: what to cut, what to accelerate
- Community feedback: prioritize based on actual usage

### 6.3 Feature Parity Checklist

Tracked as a living document: `rust-rewrite/planning/feature_parity.md`

Covers:
- [ ] All 28+ `ObjectType` values (model elements)
- [ ] All 25+ association types
- [ ] All 9 diagram types
- [ ] All 22 code generators
- [ ] All 10+ code importers
- [ ] All 80+ dialog types
- [ ] All menu items in `umbrelloui.rc`
- [ ] All toolbar actions
- [ ] All keyboard shortcuts
- [ ] All dock widgets
- [ ] All export formats
- [ ] All import formats

---

## 7. Timeline Estimate

### 7.1 Phase Timeline

| Phase | Description | Duration (person-weeks) | Parallelizable | Calendar (1 person) | Calendar (3 people) |
|-------|-------------|------------------------:|:---------------|--------------------:|--------------------:|
| 1 | Foundation | 4 | No | 4 weeks | 4 weeks |
| 2 | Core Model | 8 | Partial (types) | 8 weeks | 5 weeks |
| 3 | Persistence & Undo | 4 | No | 4 weeks | 3 weeks |
| 4 | CLI | 4 | Yes (commands) | 4 weeks | 2 weeks |
| 5 | Code Import (trait + 3 langs) | 8 | Yes (per lang) | 8 weeks | 4 weeks |
| 6 | Code Generation (trait + 3 langs) | 8 | Yes (per lang) | 8 weeks | 4 weeks |
| 7 | Diagram Model | 6 | No | 6 weeks | 5 weeks |
| 8 | Diagram Rendering | 10 | Yes (widget draw) | 10 weeks | 5 weeks |
| 9 | GUI Application | 16 | Yes (docks/tools) | 16 weeks | 8 weeks |
| 10 | Enhancement & Polish | 12–20 | Yes | 12–20 weeks | 8–12 weeks |

**Totals:**

| Metric | Value |
|--------|-------|
| Total estimated effort | 80–100 person-weeks (MVP, phases 1–9) |
| Total with polish | 100–140 person-weeks (phases 1–10) |
| Minimum calendar time (3 people) | ~12 months |
| Minimum calendar time (5 people) | ~9 months |
| First usable output (CLI) | Week 4 |
| First visual output (export) | Week 12 |
| First interactive GUI | Week 20 |

### 7.2 Parallel Work Opportunities

| Phase | Role A | Role B | Role C |
|-------|--------|--------|--------|
| 1 | Core types | XMI parser | Build system |
| 2 | Model structs | XMI serialization | Arena storage |
| 3 | File pipeline | Foreign import | Undo/redo |
| 4 | Export commands | Validate commands | Query commands |
| 5 | Importer trait + tree-sitter | C++ importer | Java/Python importers |
| 6 | Generator trait + templates | C++ generator | Java/Python generators |
| 7 | Diagram data structures | Layout algorithm | XMI diagram serialization |
| 8 | Rendering backend | Widget drawers (split by type) | Association rendering |
| 9 | Main window + canvas | Dock widgets | Dialogs (split by type) |
| 10 | Search + clipboard | DocBook generation | Performance + packaging |

### 7.3 Staffing Recommendations

- **Minimum viable team:** 2 full-time Rust engineers
  - 1 focused on core model + XMI + persistence
  - 1 focused on rendering + GUI
  - Shared responsibility for codegen/import
  - Timeline: ~18–24 months
- **Recommended team:** 3–4 engineers
  - 1: core model + XMI + persistence + CLI
  - 1: code generators + code importers
  - 1: diagram model + rendering
  - 1: GUI application + integration
  - Timeline: ~9–12 months
- **Ideal team:** 5–7 engineers
  - Same as above + 2 more for parallel codegen/import + 1 for QA/documentation
  - Timeline: ~6–9 months

### 7.4 Critical Path

The critical path through phases (longest sequential chain) is:

```
Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 7 → Phase 8 → Phase 9
```

Phases 5 and 6 can be done in parallel with 7/8. Phase 10 is fully parallelizable.

If the team is small (1–2 people), prioritize the critical path and defer
phases 5–6 until after the GUI ships.

---

## 8. Crate Architecture

### 8.1 Proposed Crate Layout

```
umbrello-rs/
├── Cargo.toml                   # Workspace root; the CLI/GUI binary
├── crates/
│   ├── umbrello-core/           # Core model types, enums, IDs
│   │   ├── Cargo.toml
│   │   └── src/                 # umbrello_types, umbrello_id
│   │
│   ├── umbrello-model/          # UML model data structures, arena storage
│   │   ├── Cargo.toml           # depends on umbrello-core
│   │   └── src/                 # UmlModel, UmlClass, UmlAssociation, etc.
│   │
│   ├── umbrello-xmi/            # XMI 1.2/2.1 serialization
│   │   ├── Cargo.toml           # depends on umbrello-core, umbrello-model
│   │   └── src/                 # reader, writer, dtd
│   │
│   ├── umbrello-persistence/    # File I/O, compression, foreign import
│   │   ├── Cargo.toml           # depends on umbrello-model, umbrello-xmi
│   │   └── src/                 # file_pipeline, rose_import, argo_import
│   │
│   ├── umbrello-undo/           # Undo/redo command stack
│   │   ├── Cargo.toml           # depends on umbrello-model
│   │   └── src/                 # command trait, concrete commands
│   │
│   ├── umbrello-codegen/        # Code generation trait + template engine
│   │   ├── Cargo.toml           # depends on umbrello-model
│   │   └── src/                 # CodeGenerator trait, template infra
│   │   │
│   │   ├── gen-cpp/             # C++ code generator
│   │   ├── gen-java/            # Java code generator
│   │   ├── gen-python/          # Python code generator
│   │   └── ...                  # 19 more generator crates
│   │
│   ├── umbrello-codeimport/     # Code import trait + tree-sitter infra
│   │   ├── Cargo.toml           # depends on umbrello-model
│   │   └── src/                 # CodeImport trait, parser infra
│   │   │
│   │   ├── imp-cpp/             # C++ importer (tree-sitter-cpp)
│   │   ├── imp-java/            # Java importer (tree-sitter-java)
│   │   ├── imp-python/          # Python importer (tree-sitter-python)
│   │   └── ...                  # 7+ more importer crates
│   │
│   ├── umbrello-diagram/        # Diagram data structures + layout
│   │   ├── Cargo.toml           # depends on umbrello-model
│   │   └── src/                 # Diagram, WidgetData, layout
│   │
│   ├── umbrello-render/         # Diagram rendering backends
│   │   ├── Cargo.toml           # depends on umbrello-diagram
│   │   └── src/                 # Renderer trait, tiny_skia, resvg
│   │   │
│   │   └── render-test/         # Snapshot test utilities
│   │
│   ├── umbrello-gui/            # GUI application
│   │   ├── Cargo.toml           # depends on everything above
│   │   └── src/                 # main window, docks, canvas, dialogs
│   │
│   └── umbrello-test/           # Shared test infrastructure
│       ├── Cargo.toml
│       └── src/                 # XMI fixtures, round-trip helpers, proptest strategies
│
├── cli/                         # CLI binary target
│   └── src/main.rs              # (thin wrapper)
│
├── assets/                      # Icons, templates, example files
│   ├── xmi-tests/               # C++-generated XMI files for round-trip testing
│   ├── templates/               # Code generation templates
│   └── icons/                   # Application icons
│
├── docs/                        # User documentation
├── ci/                          # CI scripts (round-trip testing, etc.)
└── scripts/                     # Development scripts (translation, etc.)
```

### 8.2 Dependency Graph

```
umbrello-gui
  ├── umbrello-render
  │     └── umbrello-diagram
  │           └── umbrello-model
  │                 └── umbrello-core
  ├── umbrello-undo
  │     └── umbrello-model
  ├── umbrello-codegen (optional)
  │     └── umbrello-model
  ├── umbrello-codeimport (optional)
  │     └── umbrello-model
  └── umbrello-persistence
        ├── umbrello-xmi
        │     ├── umbrello-model
        │     └── umbrello-core
        └── umbrello-model

umbrello-xmi depends ONLY on umbrello-core + umbrello-model
  → This is the foundation for the XMI bridge
  → CLI binary can be built with just umbrello-core + umbrello-model + umbrello-xmi + umbrello-persistence

No circular dependencies.
umbrello-core has ZERO internal dependencies (except standard library + serde + uuid).
```

### 8.3 Crate Versioning Policy

- All crates share the same major version as Umbrello-RS
- Breaking changes to XMI format → major version bump
- Breaking changes to crate APIs → major version bump (even before 1.0, follow semver)
- Crates are published to crates.io after Phase 3 (useful for other UML tools)

---

## 9. Decision Log

| # | Decision | Rationale | Date |
|---|----------|-----------|------|
| D1 | **Enum-based type hierarchy** over trait-based | The C++ hierarchy has 35+ concrete types. An enum `UmlObject` is simpler to serialize, pattern match, and store in arenas. Trait objects would add complexity without proportional benefit. | 2026-06-23 |
| D2 | **Generational arena indices** over `Arc`/`Rc` | UML objects have complex cross-references that can form cycles. Arenas with generational indices break cycles, provide O(1) access, and are `Copy`. No reference counting overhead. | 2026-06-23 |
| D3 | **XMI bridge** over incremental widget extraction | Qt penetration is too deep — there is no clean seam to extract "just the model" from C++. Parallel development with XMI as the bridge is the fastest path to a working system. | 2026-06-23 |
| D4 | **Two-pass XMI loading** over on-demand resolution | Simpler implementation. First pass creates all objects with placeholder IDs. Second pass resolves cross-references. Matches C++ approach, proven reliable. | 2026-06-23 |
| D5 | **Template-based code generation** over string builders | Templates (Tera) are more maintainable, user-customizable, and separate logic from presentation. C++ uses string concatenation which has led to 15+ similar-but-different implementations. | 2026-06-23 |
| D6 | **Tree-sitter for code import** over porting C++ parsers | The hand-written C++ parser is 13 KLOC. Tree-sitter provides robust parsers for 15+ languages with error recovery. The C++ parser also has known bugs that we inherit if we port it. | 2026-06-23 |
| D7 | **CLI first, GUI second** | CLI is faster to build, enables CI integration, and provides value early. The GUI is the most complex component and benefits from having stable model/XMI/render APIs before implementation. | 2026-06-23 |
| D8 | **Fluent for i18n** over gettext/PO files | Fluent is designed for modern UI frameworks, supports gender, plurals, and variants naturally. Pure Rust implementation available (fluent-rs). Avoids KDE i18n dependency. | 2026-06-23 |
| D9 | **UUID IDs** over integer IDs | UUIDs avoid collision issues in distributed workflows (multiple users editing models). Compatible with C++ through namespace-based conversion. | 2026-06-23 |
| D10 | **No direct Qt bindings** for GUI | Qt is popular but the bindings (rust-qt-bindings, cxx-qt) add complexity and limit Rust features. A pure Rust GUI framework (egui, slint, iced) is preferred for maintainability. | 2026-06-23 |
| D11 | **Workspace per importer/generator** | Each code generator and importer is a separate crate. This enables optional compilation (`cargo build --no-default-features`) and independent versioning. Community contributors can add languages without touching core. | 2026-06-23 |
| D12 | **Change events via channels** over callbacks | Channels (tokio::sync::watch or std::sync::mpsc) provide clean decoupling, support multiple listeners, and avoid the Qt signal/slot complexity. Thread-safe by default. | 2026-06-23 |

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| Arena | A flat-array storage with generational indices. Provides O(1) access, no aliasing, and safe deletion. |
| XMI Bridge | The strategy of using XMI file format as the shared interface between C++ and Rust implementations. |
| Strangler Fig | A migration pattern where new functionality gradually wraps and replaces legacy functionality. |
| XMI 1.2 | The primary XMI format used by Umbrello (OMG UML 1.3 metamodel). |
| XMI 2.1 | The newer XMI format (OMG UML 2.x metamodel), supported as optional in Umbrello. |
| `GenerationalIndex` | A tuple of (index, generation) used for safe arena access. Detects stale references via generation counter. |
| Round-trip | Reading an XMI file into a model, writing it back, and comparing the result with the original. |
| Canonicalization | Normalizing XML (sort attributes, whitespace, etc.) for comparison purposes. |
| `AppContext` | The Rust replacement for `UMLApp::app()` — an explicit dependency injection container passed to components. |

## Appendix B: References

- [Strangler Fig Application pattern](https://martinfowler.com/bliki/StranglerFigApplication.html) — Martin Fowler
- [XMI 1.2 Specification](https://www.omg.org/spec/XMI/1.2/) — OMG
- [XMI 2.1 Specification](https://www.omg.org/spec/XMI/2.1/) — OMG
- [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) — Incremental parsing library
- [Fluent localization](https://projectfluent.org/) — Mozilla's localization system
- [Cargo Workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html) — Rust documentation
- [Proptest](https://proptest-rs.github.io/proptest/intro.html) — Property-based testing for Rust

## Appendix C: Revision History

| Date | Version | Changes |
|------|---------|---------|
| 2026-06-23 | 1.0 | Initial document — consolidation of all analysis into migration strategy |
