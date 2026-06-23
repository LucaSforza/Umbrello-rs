# Rust Ecosystem — Crate Candidates for Umbrello-RS

> **Status:** Research snapshot  
> **Date:** 2026-06-23  
> **Purpose:** Evaluate candidate Rust crates for each major concern in the Umbrello-RS rewrite.  
> **Context:** Based on the C++ Umbrello codebase analysis — 550+ files across UML model, diagram widgets, code generators (22 languages), code importers (10+ languages), XMI serialization, undo/redo, and UI.  
> **Methodology:** For each category, 2–4 candidates are evaluated with pros/cons/license/maintenance. A final section gives the overall recommendation stack.

---

## Table of Contents

1. [Serialization / XMI](#1-serialization--xmi)
2. [Graph Modeling](#2-graph-modeling)
3. [Parsing / Code Import](#3-parsing--code-import)
4. [GUI / Windowing](#4-gui--windowing)
5. [Plugin System](#5-plugin-system)
6. [Command / Undo-Redo](#6-command--undo-redo)
7. [Templating](#7-templating)
8. [Configuration](#8-configuration)
9. [Testing](#9-testing)
10. [Compression / Archive](#10-compression--archive)
11. [Image Export](#11-image-export)
12. [Logging](#12-logging)
13. [Error Handling](#13-error-handling)
14. [Internationalization (i18n)](#14-internationalization-i18n)
15. [Code Generation / AST Building](#15-code-generation--ast-building)
16. [Concurrency / Async](#16-concurrency--async)
17. [Data Structures](#17-data-structures)
18. [Overall Technology Stack Recommendation](#18-overall-technology-stack-recommendation)
19. [Crate Compatibility Matrix](#19-crate-compatibility-matrix)
20. [Where We Should Roll Our Own](#20-where-we-should-roll-our-own)

---

## 1. Serialization / XMI

The C++ codebase serializes the entire UML model to XMI (XML Metadata Interchange), a complex XML format with forward references, namespaces, and diagram-specific extensions. Every model object implements `saveToXMI()`/`loadFromXMI()`. The rewrite must produce XMI-compatible output for round-trip compatibility.

### Candidate 1.1: `quick-xml`

| Attribute | Detail |
|-----------|--------|
| **Description** | High-performance, streaming XML reader/writer. Low-level, no DOM. |
| **License** | MIT |
| **Latest version** | 0.36+ (active, frequent releases) |
| **Maintenance** | ⭐ Excellent. Very actively maintained, large community. |

**Pros:**
- Blazing fast — zero-copy reads, minimal allocations.
- Streaming API avoids loading entire XML into memory (the C++ save path already streams; load is DOM-based — we can improve this).
- `Writer` and `Reader` APIs map naturally to Umbrello's existing `QXmlStreamWriter`/`QDomDocument` pattern.
- Strong `serde` integration via separate `quick-xml` + `serde` feature flag.
- Handles namespaces, attributes, CDATA — all needed for XMI.

**Cons:**
- Low-level — requires manual event-loop for deserialization if not using serde.
- No schema or DTD validation (Umbrello's own XMI loading doesn't validate either, so this is fine).
- Non-trivial to implement forward-reference resolution (need custom deserializer that collects unresolved references).

**Recommendation:** ✅ **Preferred** — the foundation for all XMI work.

---

### Candidate 1.2: `serde` + `quick-xml` integration

| Attribute | Detail |
|-----------|--------|
| **Description** | `serde` is Rust's standard serialization framework. The `quick-xml` serde adapter (`quick_xml::de`/`ser`) provides serde-compatible XML serialization. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | serde 1.0; quick-xml 0.36+ |
| **Maintenance** | ⭐ Excellent. Serde is the most-used Rust library. |

**Pros:**
- Derive macros for all model types — eliminates 50+ handwritten `saveToXMI()` / `loadFromXMI()` methods.
- `#[serde(rename = "UML:Class")]` handles the XMI namespace-tag mapping.
- `#[serde(skip_serializing_if = "Option::is_none")]` handles optional XMI attributes.
- Custom `Serializer`/`Deserializer` can implement XMI-specific quirks (forward refs, `xmi:id` generation).
- Combined with `serde_with` crate for complex attribute transformations.

**Cons:**
- XMI's forward-reference pattern (`xmi:id` / `xmi:idref`) requires custom deserialization that serde doesn't handle natively.
- Requires a deserialization phase that collects all objects, then resolves references — a two-pass approach.
- serde's XML support is adapter-based (no native XML data model in serde), so complex XMI constructs may need manual `Deserialize` impls.
- Order of XML attributes matters in XMI; serde uses `HashMap`-like behavior for attributes.

**Recommendation:** ✅ **Preferred** — derive serde on model types, use `quick_xml::de`/`ser` for the XML layer, add a `resolve_references` pass.

---

### Candidate 1.3: `roxmltree` / `xmltree-rs`

| Attribute | Detail |
|-----------|--------|
| **Description** | `roxmltree` — read-only DOM-style XML parsing with XPath support. `xmltree-rs` — mutable DOM tree. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | roxmltree 0.20; xmltree 0.10 |
| **Maintenance** | ✅ Good. roxmltree actively maintained. xmltree-rs less active. |

**Pros:**
- DOM-style matches C++ `QDomDocument` approach for loading.
- `roxmltree` supports XPath — useful for querying specific XMI elements during import.
- `roxmltree` is zero-copy on the parsed document.
- Simpler mental model than streaming for deeply nested XMI.

**Cons:**
- Read-only (`roxmltree`) — cannot be used for writing.
- Full DOM in memory for the entire document (memory concerns for large models).
- No serde integration — manual tree traversal required.
- `xmltree-rs` is less maintained and slower.
- Would need a separate writer library (e.g., `quick-xml` for output).

**Recommendation:** ⚠️ **Alternative** — useful for XMI reading if we need XPath queries, but `quick-xml` + serde is more maintainable overall.

---

### Candidate 1.4: `xml-rs`

| Attribute | Detail |
|-----------|--------|
| **Description** | SAX-style XML reader/writer. Pure Rust XML parser. |
| **License** | MIT |
| **Latest version** | 0.8 (last release Apr 2023) |
| **Maintenance** | ⚠️ Moderate. Slower release cadence. |

**Pros:**
- Mature, well-known.
- Streaming SAX-style API (event-based) — similar to `QXmlStreamReader`.
- No `unsafe` code.

**Cons:**
- Significantly slower than `quick-xml` (benchmarks show 3–5× slower).
- No serde integration.
- Larger API surface than needed.
- Less maintained than `quick-xml`.

**Recommendation:** ❌ **Not recommended** — `quick-xml` outperforms it in every dimension.

---

### XMI Roadmap

| Phase | What | Crates |
|-------|------|--------|
| 1 | Model types with serde derives | `serde` |
| 2 | XMI writer (export) | `quick-xml` + serde |
| 3 | XMI reader (import) | `quick-xml` serde adapter + custom forward-ref resolver |
| 4 | Reference resolution pass | Custom crate logic |
| 5 | Legacy format support (XMI 1.2, 2.1) | serde attributes, version detection |
| 6 | Diagram/widget serialization | Same pattern, additional serde adapters |

---

## 2. Graph Modeling

The UML model is a graph: classifiers have relationships (associations, generalizations, dependencies), packages contain classifiers, diagrams contain widgets connected by association widgets. Graph operations needed: traversal, cycle detection, topological sort (for code generation ordering), layout.

### Candidate 2.1: `petgraph`

| Attribute | Detail |
|-----------|--------|
| **Description** | The most popular Rust graph library. Provides directed/undirected graphs, DAGs, union-find, and graph algorithms. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.6+ (stable, mature) |
| **Maintenance** | ⭐ Excellent. Widely used, well-documented, actively maintained. |

**Pros:**
- Rich algorithm suite: DFS, BFS, Dijkstra, A*, topological sort, strongly connected components, minimum spanning tree.
- `GraphMap` for `Copy` node types (useful for ID-based graphs).
- `StableGraph` preserves indices under removals (useful for interactive editing).
- `visit` module for flexible traversal — fits model-walking use cases.
- Used by rustc, cargo, and many production projects.

**Cons:**
- Stores nodes in arenas/slotmaps internally — adds a dependency for something we might implement inline.
- Graph and node indices are generational but not strongly typed by default.
- Adding/removing nodes frequently (as in interactive editing) requires `StableGraph`.
- The graph model (edges between nodes) doesn't directly map to UML's association roles (which carry multiplicity, visibility, etc.).

**Recommendation:** ✅ **Preferred** — for layout, dependency analysis, and relationship traversal. Use `petgraph::StableGraph` with `UmlId` as node index.

---

### Candidate 2.2: `daggy`

| Attribute | Detail |
|-----------|--------|
| **Description** | A DAG-specific graph library built on `petgraph`. Enforces acyclic property at construction. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.8 (stable) |
| **Maintenance** | ✅ Good. Niche but maintained. |

**Pros:**
- Compile-time guarantee of no cycles — useful for generalization hierarchies.
- Built on `petgraph`, so all `petgraph` algorithms available on the DAG.
- Efficient cycle checking at edge-insertion time.

**Cons:**
- Only useful for DAG-shaped data (generalization, containment).
- Associations in UML are fundamentally cyclic (bidirectional associations, dependency cycles).
- Cannot be the *only* graph library.

**Recommendation:** ⚠️ **Alternative** — use `daggy` specifically for generalization hierarchies (class → parent class). Not for the full model graph.

---

### Candidate 2.3: `graphrs`

| Attribute | Detail |
|-----------|--------|
| **Description** | Pure Rust graph library with focus on Python-style ergonomics. |
| **License** | MIT |
| **Latest version** | 0.2 (early stage) |
| **Maintenance** | ⚠️ Low. Single maintainer, low adoption. |

**Pros:**
- Clean API.
- Built-in visualization helpers.

**Cons:**
- Very small community compared to `petgraph`.
- Missing many algorithms `petgraph` has.
- Immature — API may change.

**Recommendation:** ❌ **Not recommended** — `petgraph` dominates for good reason.

---

### Candidate 2.4: Roll your own — slotmap + adjacency lists

| Attribute | Detail |
|-----------|--------|
| **Description** | Use `slotmap` for generational object storage, manual adjacency lists for relationships. |
| **License** | MIT / Apache 2.0 (slotmap) |
| **Latest version** | slotmap 1.0+ (mature) |
| **Maintenance** | ⭐ Excellent. `slotmap` is widely used and stable. |

**Pros:**
- Maximum control over memory layout and graph semantics.
- UML associations have rich data (multiplicity, role names, visibility) — edges in `petgraph` are single values, but associations are first-class objects.
- UML's model is not a homogeneous graph: objects of different types have different connection patterns.
- Avoids impedance mismatch between graph library and domain model.

**Cons:**
- Reimplement basic graph algorithms (cycle detection, topological sort).
- More boilerplate.
- No built-in layout algorithms.

**Recommendation:** ✅ **Preferred (partial)** — use `slotmap` for the primary `ModelRepository` (storing all `UmlObject` instances). Use `petgraph` for derived graphs (e.g., layout graph, dependency graph). Do not store associations as graph edges — store them as first-class `UmlAssociation` objects with ID references.

---

### Graph Strategy Summary

```
ModelRepository (slotmap arena)
  │
  ├── Primary object storage ── slotmap::SlotMap<UmlId, UmlObject>
  │
  ├── Association storage ──── Vec<UmlAssociation> (first-class objects)
  │
  ├── Dependency graph ─────── petgraph::StableGraph<UmlId, (), Directed>
  │     (derived from associations, used for code gen ordering)
  │
  ├── Generalization DAG ───── daggy::Dag<UmlId, ()>
  │     (derived from generalization associations)
  │
  └── Layout graph ─────────── petgraph::Graph<UmlId, ()>
        (for Graphviz-based auto layout)
```

---

## 3. Parsing / Code Import

Umbrello imports source code (C++, Java, Python, etc.) into UML model objects. The C++ version has a bespoke C++ parser (`lib/cppparser/`) and simple line-scanning importers for other languages.

### Candidate 3.1: `tree-sitter` + language grammars

| Attribute | Detail |
|-----------|--------|
| **Description** | Incremental parser framework with grammar definitions for 100+ languages. Produces concrete syntax trees (CSTs). |
| **License** | MIT |
| **Latest version** | 0.24+ (very active) |
| **Maintenance** | ⭐ Excellent. Large community, backed by GitHub/AWS. |

**Pros:**
- Supports all Umbrello import targets: C++, Java, Python, C#, SQL, JavaScript, TypeScript, Ruby, PHP, Go, Rust, etc.
- Robust against syntax errors (incremental parsing recovers gracefully) — critical for incomplete model-focused code.
- Produces CSTs with node types, ranges, and error nodes — easy to walk and map to UML.
- Language grammars are separate crates (`tree-sitter-cpp`, `tree-sitter-java`, etc.), so no build-time dependency on all languages.
- Pre-built WASM grammars available for testing.
- Can be used for code generation too (syntax-aware code insertion).

**Cons:**
- C runtime dependency (the parser uses C). Usually fine, but adds build complexity.
- CSTs are more verbose than ASTs — requires significant post-processing to extract UML-relevant structures.
- Grammar quality varies between languages (C++, Java are excellent; SQL, Ada less so).
- Not a purely idiomatic Rust experience (FFI to C parser).
- The `tree-sitter` Rust crate lacks some ergonomics (visitor traits don't exist; manual cursor navigation).

**Recommendation:** ✅ **Preferred** — for all code import. It is the only solution that covers all target languages with production-quality parsing.

---

### Candidate 3.2: `logos` (lexer) + hand-written parser

| Attribute | Detail |
|-----------|--------|
| **Description** | Fast, zero-allocation lexer generator (like `flex` but with proc macros). |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.14+ |
| **Maintenance** | ✅ Good. Stable, well-documented. |

**Pros:**
- Generates efficient lexers with minimal boilerplate.
- Ideal for C++ parser rewrite (replaces `lib/cppparser/`'s manual `Lexer`).
- No runtime dependency.
- Zero-cost abstraction — compile-time generated.

**Cons:**
- Only a *lexer* — you still need a parser on top.
- For full-language import (C++, Java), you need a full parser anyway — tree-sitter already provides this.
- Useful only for the C++ parser rewrite if we go that route.
- Simple line-scanning importers (Python, Ada) don't need a lexer generator.

**Recommendation:** ⚠️ **Alternative** — consider if we rewrite `lib/cppparser/` as a standalone Rust crate. Otherwise, `tree-sitter` is more comprehensive.

---

### Candidate 3.3: `pest` (PEG parser generator)

| Attribute | Detail |
|-----------|--------|
| **Description** | PEG parser generator with grammar files. Elegant, well-documented. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 2.7+ |
| **Maintenance** | ✅ Good. Active, well-established. |

**Pros:**
- Grammar files are declarative and readable.
- Good error reporting built-in.
- No external dependencies.

**Cons:**
- PEG parsers struggle with C++'s ambiguous grammar (template syntax, `>>` as template close vs right-shift, etc.).
- Performance degrades with complex grammars (C++ has ~1500 grammar rules).
- Not suitable for Java or Python without writing complete grammars.
- Each language would need a separate grammar — significant effort.
- Left-recursion issues with PEG.

**Recommendation:** ❌ **Not recommended** — for full-language parsing. Possibly useful for small DSLs within Umbrello (stereotype expressions, constraint languages).

---

### Candidate 3.4: `nom` / `chumsky`

| Attribute | Detail |
|-----------|--------|
| **Description** | Parser combinators. `nom` is the classic; `chumsky` is newer with better error messages. |
| **License** | MIT |
| **Latest version** | nom 7.0; chumsky 0.9 |
| **Maintenance** | ✅ Good for nom; ⚠️ chumsky is newer but actively developed. |

**Pros:**
- Pure Rust, no C dependencies.
- Very fast (`nom`).
- Fine-grained control over parsing.
- `chumsky` has excellent error recovery and reporting.

**Cons:**
- Writing parsers for full languages (C++, Java) is enormous effort.
- Combinator-based parsers for C++ are known to be extremely difficult due to ambiguity.
- Duplicates effort that `tree-sitter` already solves.
- `nom` error messages are famously bad for debugging.

**Recommendation:** ❌ **Not recommended** — for full-language importing. `nom` might be useful for the C++ parser rewrite at the token/lexer level combined with `logos`. `chumsky` could be used for small DSLs.

---

### Import Strategy Recommendation

| Language | Approach | Crate |
|----------|----------|-------|
| C++ | tree-sitter grammar → UML visitor | `tree-sitter` + `tree-sitter-cpp` |
| Java | tree-sitter grammar → UML visitor | `tree-sitter` + `tree-sitter-java` |
| Python | tree-sitter grammar → UML visitor | `tree-sitter` + `tree-sitter-python` |
| C# | tree-sitter grammar → UML visitor | `tree-sitter` + `tree-sitter-c-sharp` |
| PHP | tree-sitter grammar → UML visitor | `tree-sitter` + `tree-sitter-php` |
| Ada | tree-sitter grammar (available) | `tree-sitter` + `tree-sitter-ada` |
| SQL | tree-sitter grammar (available) | `tree-sitter` + `tree-sitter-sql` |
| IDL | Custom if needed; else tree-sitter | `tree-sitter` if grammar exists |
| Simple/line-based | Regex-based native scanner | `regex` crate (already in `all》) |
| Small DSLs | PEG parser | `pest` |

---

## 4. GUI / Windowing

The GUI decision is the most consequential. Umbrello requires: 2D diagram canvas, property editors, dock widgets, menus, toolbars, complex dialogs, drag-and-drop, and accessibility. The C++ version uses Qt/KDE deeply.

### Candidate 4.1: `slint`

| Attribute | Detail |
|-----------|--------|
| **Description** | Declarative GUI toolkit with native rendering. Uses `.slint` markup files for UI and Rust for logic. Compile-time checks. |
| **License** | Royalty-free license (dual: GPL 3.0 / commercial) |
| **Latest version** | 1.7+ (very active development) |
| **Maintenance** | ⭐ Excellent. Well-funded company (SixtyFPS GmbH). |

**Pros:**
- Declarative `.slint` files make UI layout concise and type-safe.
- Compile-time checking: no runtime UI errors for missing bindings, type mismatches.
- Native rendering (no web view) — uses GPU (Skia) or CPU (software) renderer.
- Canvas API for custom 2D drawing — could implement diagram rendering.
- Built-in support for common widgets: buttons, lists, tables, text input, scroll views.
- Small binary size.
- Growing community.

**Cons:**
- **No tree widget / QTreeView equivalent** yet — the UML tree view would need to be custom-built.
- No rich text / HTML rendering (for documentation display).
- `.slint` language is a DSL to learn — team needs ramp-up.
- Canvas API is lower-level than Qt's `QGraphicsScene` — implementing 94 widget types would be significant work.
- Dock widgets and detachable panels not natively supported.
- Smaller widget selection than Qt — no professional-grade table, tree, or property editor.
- Immature ecosystem for complex desktop applications.

**Recommendation:** ⚠️ **Consider** — promising but risky. For a pure Rust rewrite starting now, the immaturity of the desktop widget ecosystem is a concern. The diagram canvas could work, but the surrounding UI (property editor, tree view, 80+ dialogs) is very demanding.

---

### Candidate 4.2: `egui`

| Attribute | Detail |
|-----------|--------|
| **Description** | Immediate-mode GUI library. Pure Rust, runs on WebGPU/WebGL/GL/wgpu. Widely used for tools, editors, and debug UIs. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.28+ (very active) |
| **Maintenance** | ⭐ Excellent. Extremely active, large community. |

**Pros:**
- Pure Rust — no C or Qt dependencies.
- Immediate mode makes dynamic UIs (like UML widgets) very easy — just describe what to draw each frame.
- Excellent canvas/2D drawing API — ideal for diagram rendering.
- Strong text rendering, custom widget system, docking tabs, panels.
- `egui_plot` for plotting, `egui-wgpu` for GPU rendering.
- Runs natively, in WASM, and on mobile.
- Huge ecosystem: `egui_tiles`, `egui_file`, `egui_notify`, etc.
- Very active development (weekly releases).

**Cons:**
- Immediate mode means state management is manual (no retained widget tree).
- No native platform widgets — everything is custom-drawn (no OS-native text fields, combo boxes, etc.).
- Accessibility support is minimal.
- No built-in undo stack (we'd need our own — which we already plan).
- Performance: immediate mode redraws everything every frame. For large diagrams (1000+ widgets), CPU usage can be high.
- Complex dialogs (multi-page property editors, 8-page settings dialog) are more awkward to implement in immediate mode.
- No traditional tree widget or QTreeView — would need custom implementation.
- No print support out of the box.

**Recommendation:** ✅ **Preferred for canvas** — use `egui` for the diagram rendering canvas. ⚠️ **Consider for entire UI** — if combining with native panels through `eframe` and `egui_dock`, it could work for the full app. The biggest gap is the tree view and complex dialogs.

---

### Candidate 4.3: `iced`

| Attribute | Detail |
|-----------|--------|
| **Description** | Elm-architecture GUI library. Pure Rust, cross-platform, with a widget system styled after Qt/QML. |
| **License** | MIT |
| **Latest version** | 0.13 (active, but API churn) |
| **Maintenance** | ✅ Good. Active development, large community. |

**Pros:**
- Retained widget tree (like Qt) — familiar mental model.
- Elm architecture (Model → Update → View) provides clean state management.
- Good set of built-in widgets (button, text, text input, scrollable, container, row/column).
- `iced_glutin` / `iced_winit` for window management.
- Pure Rust, no C deps.
- Good documentation with examples.

**Cons:**
- **No canvas / custom 2D drawing API** comparable to `QPainter` or `egui`'s painter. Diagram rendering would need to be built from primitives.
- No tree widget.
- API is still undergoing breaking changes (not 1.0).
- Performance for complex custom 2D rendering is unproven.
- Smaller widget ecosystem than `egui`.
- No print or accessibility support.
- Dock widgets not supported.

**Recommendation:** ❌ **Not recommended** — the lack of a mature 2D canvas API is a dealbreaker for a diagramming application. Could be reconsidered in 1–2 years.

---

### Candidate 4.4: `tauri` (web frontend + Rust backend)

| Attribute | Detail |
|-----------|--------|
| **Description** | Build desktop apps with a web frontend (HTML/CSS/JS) and a Rust backend. Like Electron but using the OS webview. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 2.x (very active) |
| **Maintenance** | ⭐ Excellent. Large company backing, huge community. |

**Pros:**
- Web frontend gives access to the most mature UI ecosystem in existence.
- SVG/Canvas 2D rendering in the browser is excellent (diagram rendering would be straightforward).
- Tree widgets, property grids, complex dialogs — all solved problems with web UI frameworks.
- Can use TypeScript + React/Vue/Svelte for UI — massive developer pool.
- Tauri 2.0 has mobile support.
- Small binary compared to Electron.
- Backend (Rust) handles model, parsing, code generation — all our heavy lifting.

**Cons:**
- **Web frontend is not native** — looks and feels like a web app, not a native desktop app.
- IPC bridge between frontend and backend adds latency for real-time operations (dragging widgets, painting).
- Diagram rendering through web canvas means all rendering logic in JavaScript/TypeScript — no Rust rendering.
- Complex to set up build pipeline (npm + cargo).
- Accessibility must be handled via web standards (actually, this is a strength: web has excellent accessibility).
- KDE integration impossible — no KDialog, no KConfig, no KXmlGui.
- Creating 80+ dialogs in a web framework is a lot of work.

**Recommendation:** ⚠️ **Alternative** — practical but compromises on "native" feel. Best suited if Umbrello-RS wants to be a web-first or cross-platform tool. Not recommended if the goal is a native KDE-like experience.

---

### Candidate 4.5: `relm4` / `gtk4-rs`

| Attribute | Detail |
|-----------|--------|
| **Description** | Rust bindings for GTK4. `relm4` is an Elm-like wrapper on top of `gtk4-rs`. |
| **License** | LGPL 2.1+ (GTK) |
| **Latest version** | gtk4-rs 0.9; relm4 0.9 |
| **Maintenance** | ✅ Good. GNOME-backed, stable API. |

**Pros:**
- Mature widget toolkit with tree views, property grids, canvases, dialogs, drag-and-drop, accessibility.
- GTK4 has `GtkDrawingArea` for custom 2D rendering (via Cairo or OpenGL).
- Cross-platform (Linux, Windows, macOS).
- KDE integration possible via XDG standards.
- `relm4` provides clean Elm-architecture state management.
- Well-documented.

**Cons:**
- **GTK4 is not Qt** — the look and feel differs from a KDE application.
- Cairo rendering is slower than Skia or Direct2D — diagram performance may suffer on large models.
- Rust bindings are wrappers over C — some ergonomic friction, lifetime issues.
- No `QGraphicsScene` equivalent — custom scene management needed.
- Setting up GTK on KDE-centric systems is an aesthetic mismatch.
- Not "pure Rust" — heavy C dependency.

**Recommendation:** ⚠️ **Alternative** — practical, especially if the project wants to be GNOME-compatible. But Umbrello is historically a KDE application, and GTK4 on KDE feels foreign.

---

### Candidate 4.6: `druid` (discontinued)

| Attribute | Detail |
|-----------|--------|
| **Description** | Pure Rust GUI framework with reactive data model. |
| **License** | Apache 2.0 |
| **Latest version** | 0.8 (discontinued in favor of `xilem`) |
| **Maintenance** | ❌ Discontinued. Linebender team has moved on to `xilem`. |

**Pros:**
- Elegant design (widget-rs, data traits, lens).
- Pure Rust.

**Cons:**
- Discontinued — no future development.
- Missing too many widgets for Umbrello.
- Canvas was work-in-progress.

**Recommendation:** ❌ **Not recommended** — discontinued. `xilem` is too early to evaluate.

---

### GUI Strategy Recommendation

**Tiered approach:**

1. **Canvas rendering:** `egui` for the diagram canvas (immediate mode is well-suited for 2D diagramming).
2. **Shell / main window:** Either `egui` (via `eframe`) with docking (via `egui_dock`) or `tauri` for a hybrid approach.
3. **Tree view:** Roll our own in `egui` (it has `egui_tiles` and `Grid`, but no tree widget yet).
4. **Dialogs:** Roll our own in `egui` or use native OS dialogs where appropriate.

**Preferred option:** **`egui` for everything** — it has the best 2D canvas, the largest Rust-native widget ecosystem, and is pure Rust. The trade-off is the effort to build tree view, property grids, and complex dialogs.

**Fallback option:** **Hybrid `egui` canvas + `tauri` shell** — Rust handles all model/diagram logic with an `egui` canvas, and the shell UI (menus, dialogs, tree view) is web-based. More work but uses best-in-class tools for each concern.

---

## 5. Plugin System

Umbrello has a dead plugin system in `_unused/`. The rewrite could benefit from a plugin system for: code generators, code importers, diagram exporters, and widget types.

### Candidate 5.1: `abi_stable` / `abi_stable_crates`

| Attribute | Detail |
|-----------|--------|
| **Description** | Framework for Rust libraries with a stable ABI. Enables loading Rust `dyn` Trait objects across dynamic library boundaries. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.12 (moderate activity) |
| **Maintenance** | ⚠️ Moderate. Stable but not fast-moving. |

**Pros:**
- Type-safe: plugin interfaces are defined by Rust traits.
- No `unsafe` in user code (the crate handles FFI internally).
- Supports complex types (`Vec`, `String`, `Rc`, etc.) across the ABI boundary.
- Versioned interfaces with compatibility checks.

**Cons:**
- Requires plugin authors to use `abi_stable` traits and derive macros.
- Adds complexity to the build (plugin vs host must use compatible `abi_stable`).
- Limited to Rust plugins.
- Overhead for simple use cases (code generators that are compiled in).
- Small community — fewer examples and guides.

**Recommendation:** ⚠️ **Not yet** — too heavyweight for current needs. Re-evaluate when we want a Rust-only plugin ecosystem.

---

### Candidate 5.2: `libloading`

| Attribute | Detail |
|-----------|--------|
| **Description** | Safe wrapper around platform dynamic library loading (`dlopen`, `LoadLibrary`). |
| **License** | MIT |
| **Latest version** | 0.8 (stable) |
| **Maintenance** | ✅ Good. Simple, stable, well-tested. |

**Pros:**
- Simple, well-understood API.
- Supports any language that can expose C ABI functions.
- Minimal overhead.
- Works on all platforms.

**Cons:**
- C ABI only — plugin must expose C functions.
- Requires `unsafe` to call function pointers.
- No type safety — manual interface versioning.
- Memory management across boundary is manual.

**Recommendation:** ⚠️ **Alternative** — could be used for loading native code generators (e.g., Graphviz), but for Rust plugins, the `unsafe` burden is significant.

---

### Candidate 5.3: `wasmtime` / `wasmer` (WASM-based plugins)

| Attribute | Detail |
|-----------|--------|
| **Description** | WebAssembly runtimes. Plugins compile to WASM and are loaded at runtime. |
| **License** | Apache 2.0 |
| **Latest version** | wasmtime 24+ (very active); wasmer 4+ (active) |
| **Maintenance** | ⭐ Excellent. Large teams, big communities. |

**Pros:**
- Language-agnostic — plugins can be written in any language that compiles to WASM (Rust, C, C++, Go, Zig, etc.).
- Sandboxed execution — plugins cannot crash the host process.
- Deterministic, reproducible plugin behavior.
- `wasmtime` is the reference WASM runtime, well-engineered.
- Interface types and WASI for file system access.
- Versioned, future-proof — WASM is an industry standard.

**Cons:**
- Overhead of WASM execution (not zero-cost; function calls cross the WASM boundary).
- Plugin must compile to WASM — adds build complexity for plugin authors.
- String handling across WASM boundary is cumbersome (linear memory).
- Overkill for code generators that could just be compiled-in.
- Startup latency for loading many plugins.

**Recommendation:** ❌ **Not recommended** for v1 — too much infrastructure for uncertain benefit. But **consider for v2**: a code generator marketplace with WASM plugins would be compelling.

---

### Candidate 5.4: Registry pattern (no dynamic loading)

| Attribute | Detail |
|-----------|--------|
| **Description** | Simply use `Box<dyn Trait>` collections with a registration API. All code generators are compiled into the binary. |
| **License** | N/A |
| **Maintenance** | N/A — it's a design pattern, not a crate. |

**Pros:**
- Zero complexity — no dynamic loading, no ABI concerns, no WASM.
- Max performance — all function calls are direct.
- Compile-time checking — missing trait impls are compile errors.
- Simple to add new generators: implement trait, call `registry.register()`.
- Matches the old C++ pattern (CodeGenFactory switch), but better (open registry).

**Cons:**
- All code lives in one binary — code generators from third parties require source modifications.
- Larger binary size (but code generators are text output — not large).
- Cannot dynamically add features at runtime.

**Recommendation:** ✅ **Preferred** for v1. Register all code generators and importers at compile time. The `inventory` crate can be used for automatic registration via `ctor`, or a manual `CodeGenRegistry` singleton at startup. Dynamic loading can be layered on later if needed.

---

### Plugin Strategy Summary

| Concern | Approach | Mechanism |
|---------|----------|-----------|
| Code generators | Registry pattern | `CodeGenRegistry` with `Box<dyn CodeGenerator>` |
| Code importers | Registry pattern | `ImportRegistry` with `Box<dyn CodeImporter>` |
| Widget types | Compiled-in | All widgets known at compile time (as now) |
| Export formats | Registry pattern | `ExportRegistry` with `Box<dyn DiagramExporter>` |
| Future plugins (v2) | WASM | `wasmtime` for third-party extensions |

---

## 6. Command / Undo-Redo

The C++ codebase uses `QUndoStack` with 20+ `QUndoCommand` subclasses, cleanly split between model and widget commands.

### Candidate 6.1: Custom `Command` trait

| Attribute | Detail |
|-----------|--------|
| **Description** | A simple `trait Command { fn execute(&mut self); fn undo(&mut self); }` with a `Vec<Box<dyn Command>>` stack. |
| **License** | N/A |
| **Latest version** | N/A |
| **Maintenance** | N/A — trivial to implement. |

**Pros:**
- No external dependency.
- Full control over semantics (merge behavior, checkpointing, depth limits).
- Can tailor exactly to UML model needs.
- Simple, well-understood pattern.

**Cons:**
- Reimplement basic functionality (groups, compound commands, merge logic).
- No serialization built-in (for save-game / crash recovery).
- No automatic memory management for stale commands.

**Recommendation:** ✅ **Preferred** — the undo/redo needs of Umbrello are well-understood (20 command types). A custom trait with `enum Command` dispatch is straightforward. Estimated: ~200 lines of infrastructure, 1–2 days.

---

### Candidate 6.2: `undo` crate

| Attribute | Detail |
|-----------|--------|
| **Description** | Generic undo/redo library with linear history, branching, and checkpoints. |
| **License** | MIT |
| **Latest version** | 0.3 (last release 2022) |
| **Maintenance** | ⚠️ Low. Small community, few updates. |

**Pros:**
- Provides undo history management out of the box.
- Supports branching (alternative histories).
- Supports serialization of command lists.

**Cons:**
- Generic command type is less ergonomic than trait-based.
- No recent releases — may need forking.
- Small user base — bugs less likely to be found.
- Not obviously better than a custom implementation.

**Recommendation:** ❌ **Not recommended** — too niche, too risky. Custom is simpler.

---

### Candidate 6.3: `commuter` crate

| Attribute | Detail |
|-----------|--------|
| **Description** | Undo/redo library with automatic command merging, grouping, and infinite undo. |
| **License** | MIT |
| **Latest version** | 0.5 (moderate activity) |
| **Maintenance** | ⚠️ Low. Small community. |

**Pros:**
- Command merging built-in.
- Compound command support.

**Cons:**
- Similar maturity concerns to `undo`.
- Less documentation.
- API may not fit UML model well.

**Recommendation:** ❌ **Not recommended** — custom implementation is still the better call.

---

### Candidate 6.4: Event sourcing

| Attribute | Detail |
|-----------|--------|
| **Description** | Store all state changes as an append-only event log. Current state is derived by replaying events. Implementation via `event-store` or custom. |
| **License** | Various |
| **Maintenance** | N/A — architectural pattern, not a specific crate. |

**Pros:**
- Full audit trail of all changes.
- Time travel debugging.
- Crash recovery (replay from last checkpoint).
- Serialization comes naturally (events are serializable).

**Cons:**
- Dramatically different from current architecture.
- Replaying events on each load is slower than snapshotting.
- Complex to implement for interactive use (events must be very fine-grained).
- Overkill for a desktop UML editor.

**Recommendation:** ❌ **Not recommended** — event sourcing is appropriate for financial systems or collaborative editing. Umbrello doesn't need this complexity.

---

### Undo-Redo Strategy

```rust
/// Application state that undo/redo commands operate on.
/// Intentionally minimal — commands borrow what they need.
pub trait Command {
    /// Unique identifier for the command type (for merging).
    fn id(&self) -> &'static str;

    /// Apply the command forward.
    fn execute(&mut self, ctx: &mut CommandContext) -> Result<()>;

    /// Revert the command.
    fn undo(&mut self, ctx: &mut CommandContext) -> Result<()>;

    /// Optional: merge with a subsequent command (e.g., move widget).
    /// Returns true if `other` was merged into this command.
    fn merge(&mut self, _other: &dyn Command) -> bool {
        false
    }
}

pub struct UndoStack {
    history: Vec<Box<dyn Command>>,
    position: usize,  // 0 = empty, len = all done, <len = some undone
    limit: usize,     // max depth
    save_point: usize, // position at last save (for dirty tracking)
}

impl UndoStack {
    pub fn push(&mut self, mut cmd: Box<dyn Command>, ctx: &mut CommandContext) -> Result<()> {
        cmd.execute(ctx)?;
        // Try merge with previous
        if let Some(prev) = self.history.last_mut() {
            if prev.merge(&*cmd) {
                return Ok(());  // no new entry
            }
        }
        self.history.truncate(self.position);
        self.history.push(cmd);
        self.position = self.history.len();
        // Enforce depth limit
        if self.history.len() > self.limit {
            self.history.remove(0);
            self.position -= 1;
        }
        Ok(())
    }
}
```

---

## 7. Templating

Umbrello's code generators output source code by building strings directly (simple generators) or through a `CodeDocument` / `TextBlock` tree (advanced generators). Neither uses a template engine.

### Candidate 7.1: `askama`

| Attribute | Detail |
|-----------|--------|
| **Description** | Compile-time template engine. Templates are validated at compile time with full type safety. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.13 (active) |
| **Maintenance** | ✅ Good. Active, stable. |

**Pros:**
- Type-safe templates — template variables are checked at compile time.
- Excellent performance — renders at runtime as fast as `write!` macros.
- Template syntax is Jinja2-like, familiar to many.
- Supports template inheritance, blocks, macros, filters.
- Can use `askama` for all 22+ code generators.
- Great for producing structured output (class declarations, method bodies, etc.) — less error-prone than string building.

**Cons:**
- Templates compiled into binary — cannot be user-customizable (could be a downside for extending generators).
- Every language generator needs its own template files.
- Less flexible than runtime engines for conditionals with complex logic.
- Binary size impact (22 languages × templates).
- Templates must be known at compile time.

**Recommendation:** ✅ **Preferred** — for the simple code generators (Ada, ActionScript, C#, D, IDL, JavaScript, Pascal, Perl, PHP4/5, Python, Ruby, SQL, Tcl, Vala, XML Schema). Each can have an `askama` template compiled in.

---

### Candidate 7.2: `tera`

| Attribute | Detail |
|-----------|--------|
| **Description** | Runtime template engine, inspired by Jinja2 and Django templates. |
| **License** | MIT |
| **Latest version** | 1.20+ (very active) |
| **Maintenance** | ⭐ Excellent. Large community, widely used. |

**Pros:**
- Templates loaded at runtime — can be customized by users.
- Rich feature set: inheritance, macros, filters, functions.
- Familiar syntax for anyone who knows Jinja2/Django.
- Good error messages.

**Cons:**
- Runtime parsing overhead.
- Template errors are runtime errors — not caught at compile time.
- Type-unsafe — template variables can be wrong types.
- Larger dependency than `askama`.
- Templates must be bundled with the application.

**Recommendation:** ⚠️ **Alternative** — useful only if we want user-customizable templates. For a UML tool, compile-time is usually better (fewer moving parts).

---

### Candidate 7.3: `handlebars-rust`

| Attribute | Detail |
|-----------|--------|
| **Description** | Handlebars template engine (Mustache-like, logic-less). |
| **License** | MIT |
| **Latest version** | 6.0 (active) |
| **Maintenance** | ✅ Good. Mature, well-documented. |

**Pros:**
- Logic-less: templates are cleaner (no arbitrary code in templates).
- Pre-compilation possible for performance.
- Helper system for custom logic.
- Widely used in Rust web projects.

**Cons:**
- Mustache semantics can be limiting for code generation (no complex conditionals, no block assignment).
- Template syntax adds complexity over `write!`.
- Slower than `askama` for type-safe generation.

**Recommendation:** ❌ **Not recommended** — logic-less templates are a poor fit for code generation where conditional indentation, complex loops, and formatting details matter.

---

### Candidate 7.4: `tinytemplate`

| Attribute | Detail |
|-----------|--------|
| **Description** | Minimal template engine — just substitution of `{variables}`. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 1.2 (stable) |
| **Maintenance** | ⚠️ Low. Feature-complete, minimal. |

**Pros:**
- Tiny, simple.
- Good for "stamp" templates (file headers, license blocks).

**Cons:**
- No conditionals, no loops — too limited for full code generation.

**Recommendation:** ⚠️ **Use case-specific** — good for header/footer templates, not for code generators.

---

### Templating Strategy

| Generator type | Approach |
|----------------|----------|
| Simple generators (15 languages) | `askama` compile-time templates |
| Advanced generators (C++, Java, Ruby, D) | Hybrid: `askama` for structural templates + `CodeDocument` tree for editable code |
| File header/license templates | `tinytemplate` or just `write!` macros |
| Custom user templates (future) | `tera` for runtime user-customizable templates |

For the advanced generators (C++, Java, Ruby, D), the `CodeDocument` tree approach (from the C++ codebase) can be preserved — it allows bidirectional sync between code and model. `askama` templates can generate the initial `CodeDocument`, which is then edited.

---

## 8. Configuration

The C++ codebase uses `KConfig` + `KConfigXT` with a monolithic `OptionState` singleton accessed everywhere.

### Candidate 8.1: `figment`

| Attribute | Detail |
|-----------|--------|
| **Description** | Layered configuration library. Combine multiple sources (files, env vars, CLI args) into a single config. Serde-based. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.10 (active) |
| **Maintenance** | ✅ Good. Actively maintained (Rocket team). |

**Pros:**
- Layered config: default → file → environment → CLI arguments.
- Serde-compatible — config structs derive `Deserialize`.
- Supports TOML, JSON, YAML via providers.
- `Figment::from(...).merge(...)` composition is elegant.
- Handles nested configs well.
- Can parse CLI args (via `clap` integration).

**Cons:**
- Not as widely used as `config` crate.
- Slightly less documentation.
- Tied to serde (which we already use — not a con for us).

**Recommendation:** ✅ **Preferred** — layered config matches how Umbrello settings work (defaults + user config + CLI overrides).

---

### Candidate 8.2: `config` crate

| Attribute | Detail |
|-----------|--------|
| **Description** | Application configuration with support for multiple file formats and environment variables. |
| **License** | MIT |
| **Latest version** | 0.14 (moderate activity) |
| **Maintenance** | ⚠️ Moderate. Used by many projects but less active than figment. |

**Pros:**
- Multiple format support (TOML, JSON, YAML, RON, INI).
- Environment variable support.
- Well-known, good documentation.
- Works with serde.

**Cons:**
- Less flexible layering than `figment`.
- Thread-safety concerns with global config.
- Type-erased values in some APIs.
- Config auto-reloading less straightforward.

**Recommendation:** ⚠️ **Alternative** — more traditional approach. Good, but `figment`'s layering model fits better for Umbrello's settings architecture.

---

### Candidate 8.3: Serde + TOML directly

| Attribute | Detail |
|-----------|--------|
| **Description** | Skip config crates; just use `serde` + `toml` to deserialize a config file. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | toml 0.8 (active) |
| **Maintenance** | ⭐ Excellent. Serde ecosystem. |

**Pros:**
- Minimal dependencies.
- Full control.
- Simple mental model.

**Cons:**
- No layered merging.
- No environment variable support.
- Must manually handle missing vs default values.
- Must manually handle CLI override composition.

**Recommendation:** ⚠️ **Alternative** — acceptable for simple projects. For a complex desktop app with many settings, `figment` saves significant boilerplate.

---

### Configuration Strategy

```rust
// Settings are pure value types with serde derives.
// OptionState from C++ is split into logical groups.
use figment::Figment;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub show_documentation: bool,
    pub snap_to_grid: bool,
    pub grid_spacing: f64,
    pub antialias: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenSettings {
    pub overwrite_existing: bool,
    pub indent_size: u32,
    pub generate_accessors: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub ui: UiSettings,
    pub codegen: CodeGenSettings,
    // ... other groups
}

impl Default for Settings { /* factory defaults */ }

impl Settings {
    pub fn load() -> Result<Self> {
        Figment::new()
            .merge(Serialized::defaults(Settings::default()))
            .merge(Toml::file("umbrello.toml"))
            .merge(Env::prefixed("UMBRELLO_"))
            .extract()
            .map_err(Into::into)
    }
}
```

---

## 9. Testing

The C++ tests use Qt Test (QObject-based). For Rust, we have strong ecosystem options.

### Candidate 9.1: `rstest`

| Attribute | Detail |
|-----------|--------|
| **Description** | Fixture-based testing with parameterized cases and attribute-like macros. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.22 (very active) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- `#[rstest]` replaces `#[test]` with fixture setup/teardown.
- `#[case]` for parameterized tests (great for testing all UML object types).
- `#[values]` for combinatoric test inputs.
- Integrates with `proptest`.
- Excellent for testing: save/load round-trip, code generation output, import parsing.

**Cons:**
- Additional proc-macro — slightly longer compile times.
- Learning curve for fixture setup.

**Recommendation:** ✅ **Preferred** — especially for parameterized tests across 30+ UML object types and 22 languages.

---

### Candidate 9.2: `proptest`

| Attribute | Detail |
|-----------|--------|
| **Description** | Property-based testing: define invariants and generate random test inputs. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 1.5 (stable) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Finds edge cases that manually written tests miss.
- Shrinks failing cases to minimal reproduction.
- Great for: XMI round-trip correctness, serialization invariants, model constraints.
- Works with `rstest` via `#[rstest]` integration.

**Cons:**
- Overkill for trivial tests.
- Slower test execution.
- Random failures can be confusing to debug.

**Recommendation:** ✅ **Preferred** — for model invariants and XMI round-trip. Combine with `rstest`.

---

### Candidate 9.3: `insta`

| Attribute | Detail |
|-----------|--------|
| **Description** | Snapshot testing — compare output against stored reference. |
| **License** | Apache 2.0 |
| **Latest version** | 1.40+ (very active) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Ideal for code generation tests: snapshot the generated code for known inputs.
- Interactive review for updating snapshots.
- Works great with `cargo-insta` for reviewing diffs.
- Can use redactions for non-deterministic output (timestamps, UUIDs).

**Cons:**
- Snapshot files in repository can become large.
- Brittle to intentional output changes (must update all snapshots).
- Not a replacement for unit tests — catches regressions, not correctness.

**Recommendation:** ✅ **Preferred** — for code generator output, XMI output, and importer output.

---

### Candidate 9.4: `pretty_assertions`

| Attribute | Detail |
|-----------|--------|
| **Description** | Multi-line diff output for assertion failures. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 1.4 (stable) |
| **Maintenance** | ✅ Good. |

**Pros:**
- `assert_eq!` failures show colored diffs — invaluable for large string comparisons.
- Almost zero-cost — only active in test builds.

**Cons:**
- Only useful when assertions fail.
- Minor nightly feature needed for colored output on some platforms.

**Recommendation:** ✅ **Use by default** — no reason not to.

---

### Candidate 9.5: `test-case`

| Attribute | Detail |
|-----------|--------|
| **Description** | Procedural macro for concise parameterized test cases. |
| **License** | MIT |
| **Latest version** | 3.3 (active) |
| **Maintenance** | ✅ Good. |

**Pros:**
- `#[test_case("input", "expected")]` syntax is terse.
- Works with `#[should_panic]`.

**Cons:**
- Less flexible than `rstest`'s `#[case]`.
- No fixture support.

**Recommendation:** ⚠️ **Alternative** — `rstest` subsumes `test-case` functionality with more features.

---

### Testing Strategy

| Test type | Crate | Use case |
|-----------|-------|----------|
| Unit tests | `#[test]` (built-in) | Model logic, constraint checking |
| Parameterized tests | `rstest` | Test all 30+ object types, all 22 languages |
| Property-based tests | `proptest` | XMI round-trip invariants, model constraints |
| Snapshot tests | `insta` | Code generator output, XMI output |
| Integration tests | `#[test]` + custom harness | Full import → model → codegen pipeline |
| Doc tests | `rustdoc` | Example code in documentation |

---

## 10. Compression / Archive

Umbrello supports multiple archive formats: `.xmi.tgz`, `.xmi.tar.bz2`, `.zargo` (ZIP). The Rust standard library provides no compression.

### Candidate 10.1: `flate2` (gzip)

| Attribute | Detail |
|-----------|--------|
| **Description** | Gzip compression/decompression. Backed by `miniz_oxide` (pure Rust) or `zlib` (C). |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 1.0+ (very stable) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Pure Rust backend (`miniz_oxide`) or C backend (`zlib`) — choose based on speed vs build simplicity.
- Industry standard.
- Well-documented, easy API.
- Used by many Rust projects.

**Cons:**
- N/A — it's a standard choice.

**Recommendation:** ✅ **Preferred** — for `.xmi.tgz` format.

---

### Candidate 10.2: `tar`

| Attribute | Detail |
|-----------|--------|
| **Description** | Tar archive reading/writing (no compression — pairs with `flate2` / `bzip2`). |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.4 (stable) |
| **Maintenance** | ✅ Good. |

**Pros:**
- Standard tar support.
- Works seamlessly with `flate2` for `.tgz`.
- Streaming API for efficient I/O.

**Cons:**
- Minimal abstraction over tar format.

**Recommendation:** ✅ **Preferred** — paired with `flate2` for `.xmi.tgz`.

---

### Candidate 10.3: `bzip2`

| Attribute | Detail |
|-----------|--------|
| **Description** | Bzip2 compression. Rust bindings to libbz2 (C). |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.5 (stable) |
| **Maintenance** | ⚠️ Low maintenance. |

**Pros:**
- Handles `.xmi.tar.bz2` format.
- Better compression than gzip for some data.

**Cons:**
- C dependency (`libbz2`).
- Slower than gzip.

**Recommendation:** ⚠️ Keep for backward compatibility if needed.

---

### Candidate 10.4: `zip`

| Attribute | Detail |
|-----------|--------|
| **Description** | Read/write ZIP archives. Pure Rust. |
| **License** | MIT |
| **Latest version** | 2.2+ (active) |
| **Maintenance** | ✅ Good. |

**Pros:**
- Handles `.zargo` (ArgoUML) and `.xmi.zip` formats.
- Pure Rust.
- Streaming support.

**Cons:**
- ZIP format has quirks; the crate handles most of them.
- Slower than tar+compression for large archives.

**Recommendation:** ✅ **Preferred** — for `.zargo` compatibility.

---

### Compression Strategy

```rust
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;

// .xmi.tgz — XMI compressed with gzip
fn save_as_tgz(path: &Path, xmi_content: &str) -> Result<()> {
    let file = File::create(path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut archive = Builder::new(encoder);
    archive.append_string("model.xmi", xmi_content, &Default::default())?;
    archive.finish()?;
    Ok(())
}

// .zargo — ArgoUML ZIP
fn save_as_zargo(path: &Path, xmi_content: &str) -> Result<()> {
    let file = File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    zip.start_file("model.xmi", Default::default())?;
    zip.write_all(xmi_content.as_bytes())?;
    zip.finish()?;
    Ok(())
}
```

---

## 11. Image Export

Umbrello exports diagrams to: SVG, EPS, PNG, BMP, JPEG, DOT. The C++ version uses `QImageWriter`, `QSvgGenerator`, and `QPainter`.

### Candidate 11.1: `resvg` + `usvg`

| Attribute | Detail |
|-----------|--------|
| **Description** | `usvg` — an SVG simplification/normalization library. `resvg` — rasterizes SVG to PNG (using tiny-skia). |
| **License** | MPL 2.0 |
| **Latest version** | resvg 0.42+ (very active) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- The definitive Rust SVG rendering stack — produces correct, high-quality output.
- Can render Umbrello's diagram SVG output to PNG, BMP, JPEG.
- Handles complex SVG (CSS, text, gradients).
- Uses `tiny-skia` for fast rasterization.
- Outputs pixel-perfect images.

**Cons:**
- SVG only — cannot write SVG (but that's fine; we'd generate SVG via `quick-xml` or `svg` crate).
- Heavy dependency chain (xmlparser, fontdb, etc.).
- Overkill if we're not using SVG as the intermediate format.

**Recommendation:** ✅ **Preferred** — for rendering SVG diagrams to PNG/BMP/JPEG. Use as the rendering pipeline: diagram → SVG → `resvg` → PNG.

---

### Candidate 11.2: `image` crate

| Attribute | Detail |
|-----------|--------|
| **Description** | The standard Rust image loading/saving library. Supports PNG, JPEG, BMP, GIF, WebP, etc. |
| **License** | MIT |
| **Latest version** | 0.25 (very active) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Read and write many raster formats.
- Image processing operations (resize, crop, rotate).
- Well-documented.
- All major formats: PNG, JPEG, BMP, etc.

**Cons:**
- Raster only — cannot write SVG or EPS.
- Not a drawing library — cannot render diagrams from scratch.

**Recommendation:** ✅ **Preferred** — for the final raster output step. Use `image` to encode the pixel buffer from `resvg` to PNG/JPEG/BMP.

---

### Candidate 11.3: `plotters`

| Attribute | Detail |
|-----------|--------|
| **Description** | Drawing and charting library. Supports SVG, PNG, and WASM backends. |
| **License** | MIT |
| **Latest version** | 0.3 (moderate activity) |
| **Maintenance** | ⚠️ Moderate. |

**Pros:**
- Can draw basic shapes, text, lines.
- Supports multiple backends (SVG, PNG, bitmap).
- Could theoretically render UML diagrams.

**Cons:**
- Built for charting — coordinate system assumes data plots, not freeform diagrams.
- No support for arrows, association markers, complex widget layouts.
- Performance for interactive rendering is poor.
- Not designed for 2D diagram editors.

**Recommendation:** ❌ **Not recommended** — wrong abstraction for UML diagrams.

---

### Candidate 11.4: `raqote` (unmaintained)

| Attribute | Detail |
|-----------|--------|
| **Description** | CPU-based 2D rasterization library. Pure Rust, inspired by Skia. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.5 (last release 2022) |
| **Maintenance** | ❌ Unmaintained. |

**Pros:**
- Could render UML diagrams directly (path, fill, stroke, text).
- Pure Rust.

**Cons:**
- Unmaintained.
- Bugs not fixed.
- Limited font handling.
- `resvg` + `tiny-skia` is a better maintained alternative for the same use case.

**Recommendation:** ❌ **Not recommended** — unmaintained.

---

### Image Export Pipeline

```
Diagram Scene (egui canvas)
      │
      ▼
   Serialize widgets to SVG (quick-xml + custom SVG generator)
      │
      ▼
   resvg (parse SVG) ──► tiny-skia (rasterize)
      │
      ▼
   image crate (encode PNG/JPEG/BMP)
```

OR, for the common case where we can render from the egui canvas directly:

```
Diagram Scene (egui canvas)
      │
      ▼
   egui renders to pixel buffer
      │
      ▼
   image crate (encode PNG/JPEG/BMP)
```

---

## 12. Logging

The C++ codebase has a simple `Tracer` singleton with per-class control and a log dock widget.

### Candidate 12.1: `tracing`

| Attribute | Detail |
|-----------|--------|
| **Description** | Structured, async-aware logging and diagnostics framework. Spans, events, and subscribers. |
| **License** | MIT |
| **Latest version** | 0.1 (stable, active) |
| **Maintenance** | ⭐ Excellent. Tokio team, huge community. |

**Pros:**
- Structured logging: key-value pairs, spans with duration.
- Async-aware: `tracing` is the standard for async Rust.
- Subscriber ecosystem: `tracing-subscriber` for formatting, `tracing-appender` for file output, `tracing-chrome` for flamegraphs.
- `log` compatibility: emits `log` crate messages.
- Ideal for debugging complex import/codegen pipelines.

**Cons:**
- Larger dependency than `log`.
- The spans/events/fields model has a learning curve.
- More API surface than needed for simple logging.

**Recommendation:** ✅ **Preferred** — the structured logging and span support is invaluable for debugging code import and generation pipelines.

---

### Candidate 12.2: `log` + `env_logger`

| Attribute | Detail |
|-----------|--------|
| **Description** | Simple logging facade (`log`) with terminal output (`env_logger`). |
| **License** | MIT / Apache 2.0 |
| **Latest version** | log 0.4; env_logger 0.11 |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Simplest possible approach.
- `RUST_LOG` environment variable controls log level.
- Very fast.
- No async dependency.

**Cons:**
- Unstructured — string-only messages.
- No spans or durations.
- Harder to wire into a UI log widget (would need custom subscriber).

**Recommendation:** ⚠️ **Alternative** — fine for simple projects. For Umbrello, `tracing` offers more value for debugging the complex code generation and import pipelines.

---

### Candidate 12.3: `slog`

| Attribute | Detail |
|-----------|--------|
| **Description** | Structured, composable logging with multiple output drains. |
| **License** | MPL 2.0 / MIT / Apache 2.0 |
| **Latest version** | 2.7 (stable) |
| **Maintenance** | ⚠️ Low. Stable, but not actively developed. |

**Pros:**
- Structured logging (key-value).
- Composable: different loggers for different outputs.
- More flexible than `log`.

**Cons:**
- Less community adoption than `tracing`.
- No async integration.
- Verbose API.
- `tracing` has largely superseded it.

**Recommendation:** ❌ **Not recommended** — `tracing` offers everything `slog` does plus async support and broader ecosystem.

---

### Logging Strategy

```rust
// Use tracing throughout the codebase.
// Wire a subscriber at startup that:
// 1. Writes to stderr (for CLI/headless mode)
// 2. Feeds events to a ring buffer for the in-app log dock widget

use tracing::{info, warn, error, debug, span, Level};

// Example: trace a code generation pipeline
fn generate_code(model: &ModelRepository, lang: &str) -> Result<String> {
    let _span = span!(Level::INFO, "codegen", language = lang).entered();
    info!("Starting code generation for {}", lang);
    // ...
    debug!(num_classes = model.class_count(), "processed classes");
    // ...
    Ok(output)
}

// Custom subscriber -> LogDock widget
struct LogDockSubscriber {
    buffer: Arc<RwLock<VecDeque<LogEntry>>>,
}

impl tracing::Subscriber for LogDockSubscriber { ... }
```

---

## 13. Error Handling

The C++ codebase has inconsistent error handling — some functions return `bool`, some throw exceptions, many don't handle errors at all.

### Candidate 13.1: `thiserror`

| Attribute | Detail |
|-----------|--------|
| **Description** | Derive macro for `std::error::Error`. Creates idiomatic error types with `Display` and `source()`. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 2.0 (stable, active) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- `#[derive(Error)]` makes creating error types trivial.
- Supports `#[error("message {field}")]` formatting.
- Supports `#[source]` for error chaining.
- Zero runtime overhead.
- The standard approach for library code.

**Cons:**
- Requires defining error types manually.
- More verbose than `anyhow` for application code.

**Recommendation:** ✅ **Preferred** — for all library crates (model, persistence, codegen, import). Error types carry semantic meaning.

---

### Candidate 13.2: `anyhow`

| Attribute | Detail |
|-----------|--------|
| **Description** | Flexible error type for application-level code. `anyhow::Result<T>` wraps any `Error`. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 1.0 (stable) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- `anyhow::Error` is a boxed, type-erased error — easy to propagate.
- `.context("failed to load file")` adds human-readable context.
- `.with_context(|| format!("..."))` for lazy context.
- Excellent ergonomics for application code.

**Cons:**
- Hides the concrete error type — callers cannot match on specific errors.
- Not suitable for library APIs.

**Recommendation:** ✅ **Preferred** — for application glue code, CLI handling, and fallback error handling.

---

### Candidate 13.3: `eyre`

| Attribute | Detail |
|-----------|--------|
| **Description** | A fork/evolution of `anyhow` with pluggable error reporters. More customizable error reporting. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.6 (moderate activity) |
| **Maintenance** | ⚠️ Moderate. Smaller community than `anyhow`. |

**Pros:**
- Custom Report handlers for different error display.
- Same `.context()` API as `anyhow`.
- Slightly better error formatting.

**Cons:**
- API churn — not as stable as `anyhow`.
- Smaller ecosystem.
- `anyhow` is more widely understood.

**Recommendation:** ⚠️ **Not worth it** — `anyhow` is the standard and sufficient.

---

### Candidate 13.4: `miette`

| Attribute | Detail |
|-----------|--------|
| **Description** | Fancy error and diagnostic reporting framework. Colorful, annotated error messages with source snippets. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 7.0 (active) |
| **Maintenance** | ✅ Good. |

**Pros:**
- Beautiful, colored error output with source code snippets.
- Good for CLI tools — gives users actionable error information.
- Supports `#[derive(Diagnostic)]` with help text, severity, labels.

**Cons:**
- Overkill for a desktop GUI application (errors mostly shown in dialogs, not terminal).
- Adds dependency weight.
- Best for CLI tools, not library code.

**Recommendation:** ⚠️ **CLI only** — useful for the CLI interface (export, import). Not needed for the GUI path.

---

### Error Handling Strategy

```rust
// Library crates (model, persistence, codegen, import):
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ModelError {
    #[error("Object not found: {id}")]
    ObjectNotFound { id: UmlId },

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Cycle detected: {0}")]
    Cycle(String),
}

// Application code:
use anyhow::{Context, Result};

fn load_and_generate(path: &str) -> Result<String> {
    let xmi = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read {path}"))?;
    let model = xmi_parse(&xmi)
        .context("Failed to parse XMI")?;
    let code = generate_java(&model)
        .context("Failed to generate Java code")?;
    Ok(code)
}
```

---

## 14. Internationalization (i18n)

The C++ codebase uses KDE's `KLocalizedString` / `ki18n` for translations.

### Candidate 14.1: `fluent-rs`

| Attribute | Detail |
|-----------|--------|
| **Description** | Mozilla's Fluent localization system. Modern, designed for natural language translation with grammatical gender and plural rules. |
| **License** | Apache 2.0 |
| **Latest version** | 0.16 (active) |
| **Maintenance** | ✅ Good. Mozilla-backed. |

**Pros:**
- Modern, well-designed localization system.
- Handles pluralization, gender, and complex grammar rules.
- `.ftl` files are human-readable.
- Supports splitting translations by component (good for modular Umbrello).
- Well-documented.
- Rust-native.

**Cons:**
- Newer ecosystem — fewer tools and established workflows.
- Translation tooling (Poedit, Weblate) may not support `.ftl` as well as `.po`.
- Different from KDE's `ki18n` — existing Umbrello translations cannot be directly reused.
- Smaller translator community.

**Recommendation:** ✅ **Preferred** — if starting fresh, Fluent is the best designed system. The investment in `.ftl` tooling is worth it for the long term.

---

### Candidate 14.2: `gettext-rs`

| Attribute | Detail |
|-----------|--------|
| **Description** | Rust bindings to GNU gettext. The traditional Unix i18n system. |
| **License** | MIT / GPL |
| **Latest version** | 0.21 (stable) |
| **Maintenance** | ⚠️ Moderate. Bindings to a stable C library. |

**Pros:**
- Mature, well-understood ecosystem.
- Existing tools: Poedit, Weblate, `xgettext`.
- .po/.mo file format is widely used.
- Can potentially reuse Umbrello's existing `.po` translations? (Unlikely — they use KDE-specific markers.)

**Cons:**
- C dependency (`libintl` or `gettext`).
- C bindings — not idiomatic Rust.
- No native plural/gender support (uses C's `ngettext`).
- `gettext` is showing its age compared to Fluent.
- String formatting is `printf`-style (not Rust-friendly).

**Recommendation:** ❌ **Not recommended** — outdated design, C dependency, no advantage over Fluent.

---

### Candidate 14.3: `icu4x`

| Attribute | Detail |
|-----------|--------|
| **Description** | Unicode ICU4X — Unicode internationalization in Rust (collation, formatting, time zones, etc.). |
| **License** | Unicode / Apache 2.0 |
| **Latest version** | 1.5+ (very active) |
| **Maintenance** | ⭐ Excellent. Google/Unicode consortium backed. |

**Pros:**
- Industry-standard Unicode support.
- Collation (sorting), date/time formatting, number formatting, normalisation.
- Pure Rust, no ICU4C dependency.
- Modular — import only what you need.

**Cons:**
- Not a translation/message system — does not replace gettext or Fluent.
- Complements translation systems but does not compete with them.
- Messages in generated code (e.g., "Generated by Umbrello") don't need ICU-level formatting.

**Recommendation:** ⚠️ **Complementary** — use `icu4x` for formatting (dates in documentation, numbers in settings) but not for translation.

---

### i18n Strategy

| Concern | Approach |
|---------|----------|
| UI and in-app messages | `fluent-rs` with `.ftl` files per component |
| Numbers, dates, collation | `icu4x` |
| Generated code comments (i18n of outputs) | Same `fluent-rs` — or not translated at all (code comments in English) |

**Migration note:** Existing Umbrello `.po` files from KDE cannot be directly reused. The `.ftl` translation files will need to be created from scratch. This is a significant effort for a community of translators.

---

## 15. Code Generation / AST Building

The C++ code generators output source code either via `QTextStream` (simple) or through a `CodeDocument` tree (advanced). For Rust, we need to produce syntactically correct source code in 22 languages.

### Candidate 15.1: `codegen` crate

| Attribute | Detail |
|-----------|--------|
| **Description** | Programmatic Rust code generation. Not for generating *other* languages. |
| **License** | MIT |
| **Latest version** | 3.0 (stable) |
| **Maintenance** | ⚠️ Low. Used internally for Rust code gen. |

**Pros:**
- Generates valid Rust code.
- Builder pattern for functions, structs, enums, impls.

**Cons:**
- **Only generates Rust code** — useless for C++, Java, Python, etc.
- Not relevant for Umbrello's use case (we generate C++, Java, etc., from Rust).

**Recommendation:** ❌ **Not recommended** — wrong purpose.

---

### Candidate 15.2: `indoc`

| Attribute | Detail |
|-----------|--------|
| **Description** | Procedural macro for indented multi-line strings. `indoc!` removes common indentation. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 2.0 (stable) |
| **Maintenance** | ✅ Good. |

**Pros:**
- Clean template strings in source code.
- Removes leading whitespace based on indent level.
- Perfect for embedding code templates in Rust source.

**Cons:**
- Just string indentation — not a full template solution.

**Recommendation:** ✅ **Preferred** — use alongside `askama` or `write!` for formatting generated code strings.

---

### Candidate 15.3: `write!` macro

| Attribute | Detail |
|-----------|--------|
| **Description** | Built-in Rust formatting: `write!(f, "...", args)` / `writeln!`. |
| **License** | N/A |
| **Latest version** | N/A |
| **Maintenance** | ⭐ Part of Rust standard library. |

**Pros:**
- Zero dependencies.
- Full control over output.
- Fast.
- Format specifiers for alignment, padding, numeric formatting.

**Cons:**
- Error-prone for deeply nested code generation (manual indentation management).
- No template separation — logic and formatting mixed.
- Harder to maintain for 22 languages.

**Recommendation:** ✅ **Use always** — `writeln!` is the low-level building block. Used by templates and direct generators alike.

---

### Code Generation Strategy

| Layer | Approach | Crate |
|-------|----------|-------|
| Simple generators | Compile-time templates | `askama` |
| Advanced generators | CodeDocument tree (port from C++) + `askama` for init | Custom + `askama` |
| Formatting helpers | Indentation, line wrapping, brace management | Custom utility module |
| String building | Format strings | `write!` + `indoc` |
| Code model | Port `CodeDocument` / `TextBlock` tree | Custom |

The C++ `CodeDocument` model (advanced generators) should be ported — it's well-designed and enables bidirectional sync between code and model. Simple generators can use `askama` templates.

---

## 16. Concurrency / Async

Umbrello currently does minimal threading: XSLT transformations run in a background thread. The rest is single-threaded. The Rust version could benefit from concurrency for: file I/O, code import (multiple files), code generation (multiple classes), and layout.

### Candidate 16.1: `tokio`

| Attribute | Detail |
|-----------|--------|
| **Description** | The standard async runtime for Rust. Async I/O, timers, synchronization primitives. |
| **License** | MIT |
| **Latest version** | 1.40+ (very active) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Async file I/O for XMI loading/saving.
- `tokio::sync::broadcast` for the event bus (replaces Qt signals).
- `tokio::task::spawn_blocking` for CPU-intensive tasks (parsing, generation).
- Timers for autosave.
- Huge ecosystem.

**Cons:**
- Async is a significant learning curve for the team.
- GUI applications are fundamentally event-driven, not async. Integrating async runtime with an event loop can be tricky.
- Overhead of async runtime for a primarily single-threaded desktop app.
- The old codebase has no async — so this would be a big architectural shift.

**Recommendation:** ⚠️ **Consider** — use `tokio` for the background processing (import, export, generation, layout) but not for the main GUI thread. Use `tokio::runtime::Runtime` for background tasks, not for the entire application.

---

### Candidate 16.2: `rayon`

| Attribute | Detail |
|-----------|--------|
| **Description** | Data parallelism library. Parallel iterators (`par_iter()`). |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 1.10 (stable) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Easier than async: just replace `.iter()` with `.par_iter()`.
- Great for CPU-bound parallel work: generating code for multiple classifiers simultaneously, parsing multiple import files.
- Work-stealing thread pool — efficient CPU utilization.
- No runtime dependency — just a library.

**Cons:**
- No async I/O — cannot do parallel file reads/writes easily.
- Not for event-driven or long-running background tasks.
- Thread pool is global — fine for most use cases.

**Recommendation:** ✅ **Preferred** — for CPU parallelism in code generation and import. Parallel code generation across 30+ classifiers is a natural use case.

---

### Candidate 16.3: `crossbeam`

| Attribute | Detail |
|-----------|--------|
| **Description** | Concurrent programming tools: channels, scoped threads, atomic utilities. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 0.8 (stable) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Scoped threads — spawn threads that borrow from the parent scope.
- Channels — multi-producer, multi-consumer.
- `SegQueue`, `ArrayQueue` — concurrent queues for work stealing.

**Cons:**
- Lower-level than `rayon` — more boilerplate for parallel iteration.
- No async I/O support.

**Recommendation:** ⚠️ **Use selectively** — for channels between threads (e.g., progress reporting from import thread to UI). For data parallelism, `rayon` is preferred.

---

### Concurrency Strategy

```rust
// GUI thread: egui's event loop (single-threaded, no async)
// Background thread pool: rayon
// Async runtime: tokio (limited, for I/O-bound operations)

// Example: parallel code generation
use rayon::prelude::*;

fn generate_all(model: &ModelRepository, lang: &str) -> Vec<(String, String)> {
    model.classifiers()
        .par_iter()  // parallel iteration — uses rayon thread pool
        .map(|cls| generate_one(cls, lang))
        .collect()
}

// Example: background import with progress
use crossbeam::channel;

fn import_files(files: Vec<PathBuf>, progress: crossbeam::Sender<f32>) -> Result<ModelRepository> {
    // spawn thread, send progress updates via channel
}
```

---

## 17. Data Structures

The C++ codebase uses `QList`, `QVector`, and numerous custom list types (`UMLObjectList`, associated type specializations). The Rust rewrite needs efficient, safe data structures for the UML model.

### Candidate 17.1: `slotmap`

| Attribute | Detail |
|-----------|--------|
| **Description** | Generational index-based container. Like a `Vec` but indices are stable and generation-checked. |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 1.0+ (stable) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- **Ideal for UML model storage**: objects are stored in a `SlotMap<UmlId, UmlObject>`.
- Generational indices detect stale references (use-after-free protection).
- `SlotMap` — dense storage (iterates fast). `HopSlotMap` — fast removals. `DenseSlotMap` — fastest iteration.
- IDs are `u64` internally — serializable, thread-safe.
- Object removals do not invalidate other indices.

**Cons:**
- Slightly more overhead than a plain `Vec` (generation check on access).
- Keys are not type-safe by default (but we can wrap in newtypes).

**Recommendation:** ✅ **Preferred** — this should be the *primary* storage mechanism for all UML model objects. Use `SlotMap` for the `ModelRepository`.

---

### Candidate 17.2: `im` (persistent data structures)

| Attribute | Detail |
|-----------|--------|
| **Description** | Immutable/persistent data structures: `Vector`, `HashMap`, `HashSet`, `OrdMap`, `OrdSet`. |
| **License** | MPL 2.0 |
| **Latest version** | 16.0 (active) |
| **Maintenance** | ✅ Good. |

**Pros:**
- Persistent data structures enable cheap snapshots (undo history without copying).
- Structural sharing — cloning is O(1). Undo/redo history could share unchanged parts.
- Thread-safe by design (immutable).
- Useful for: code generation templates (shared state), model snapshots for undo.

**Cons:**
- Hash array mapped trie (HAMT) — slower than `slotmap` for iteration.
- Not appropriate for the primary object storage (objects are mutated in place).
- Higher memory overhead due to structural sharing.
- Complex ownership model.

**Recommendation:** ⚠️ **Use selectively** — `im::Vector` could be useful for undo history (store snapshots efficiently). Not for primary storage.

---

### Candidate 17.3: `dashmap`

| Attribute | Detail |
|-----------|--------|
| **Description** | Concurrent HashMap with fine-grained locking. |
| **License** | MIT |
| **Latest version** | 6.1 (active) |
| **Maintenance** | ✅ Good. |

**Pros:**
- Thread-safe hash map without a global lock.
- Ideal for: caches (stereotype lookup by name), quick ID lookups from multiple threads.
- Used in rayon parallel iterations safely.

**Cons:**
- Overhead of sharded locks.
- Only useful when multiple threads access the same map.
- In a primarily single-threaded GUI app, an `RwLock<HashMap>` may be sufficient.

**Recommendation:** ⚠️ **Use selectively** — for concurrent lookup tables (e.g., a stereoTypeRegistry accessed from code generation threads). Not needed for the main model store.

---

### Candidate 17.4: `indexmap`

| Attribute | Detail |
|-----------|--------|
| **Description** | Hash map with predictable iteration order (insertion order). |
| **License** | MIT / Apache 2.0 |
| **Latest version** | 2.6 (active) |
| **Maintenance** | ⭐ Excellent. |

**Pros:**
- Insertion-order preserving hash map.
- Useful when: you want `HashMap` semantics but need deterministic output (e.g., generated code, XMI output).
- Iteration order matches insertion order — important for round-trip fidelity.

**Cons:**
- Slightly slower than `std::collections::HashMap`.
- More memory per entry.

**Recommendation:** ✅ **Preferred** — for any map where iteration order matters. Code generation options, attribute/operation lists in classifiers, diagram widget lists.

---

### Data Structures Strategy

| Purpose | Crate | Why |
|---------|-------|-----|
| Primary object storage | `slotmap` | Generational indices, stable IDs, fast iteration |
| Association storage | `Vec<UmlAssociation>` | Simple list with ID references |
| Object lookup by name | `indexmap` | Deterministic order for model browsing |
| Concurrent caches | `dashmap` | Thread-safe registration |
| Undo snapshots | `im` (optional) | Persistent data structures for cheap snapshots |
| Attribute/Operation lists | `indexmap` | Deterministic order for code generation |
| Event/watcher lists | `std::vec::Vec` | Simple observer lists |

---

## 18. Overall Technology Stack Recommendation

### "Winning" Crates by Category

| Category | Preferred | Alternative | Not Recommended |
|----------|-----------|-------------|----------------|
| **Serialization / XMI** | `serde` + `quick-xml` | `roxmltree` | `xml-rs` |
| **Graph Modeling** | `petgraph` + `slotmap` | `daggy` (for DAGs only) | `graphrs` |
| **Parsing / Code Import** | `tree-sitter` + per-language grammars | `logos` (for lexer only) | `nom`, `pest`, `chumsky` |
| **GUI / Windowing** | `egui` (canvas) | `tauri` (hybrid) | `iced`, `druid`, `raqote` |
| **Plugin System** | Registry pattern (no crate) | `libloading` | `abi_stable`, `wasmtime` |
| **Command / Undo-Redo** | Custom `Command` trait | — | `undo`, `commuter`, event sourcing |
| **Templating** | `askama` | `tera` | `handlebars`, `tinytemplate` |
| **Configuration** | `figment` | `config` crate | — |
| **Testing** | `rstest` + `insta` + `proptest` | `test-case` | — |
| **Compression / Archive** | `flate2` + `tar` + `zip` | `bzip2` | — |
| **Image Export** | `resvg` + `image` | — | `plotters`, `raqote` |
| **Logging** | `tracing` | `log` + `env_logger` | `slog` |
| **Error Handling** | `thiserror` + `anyhow` | `eyre` | `miette` (CLI only) |
| **Internationalization** | `fluent-rs` | — | `gettext-rs` |
| **Code Generation / AST** | `indoc` + `askama` + custom | — | `codegen` crate |
| **Concurrency / Async** | `rayon` + `crossbeam` | `tokio` (limited) | — |
| **Data Structures** | `slotmap` + `indexmap` | `dashmap`, `im` | — |

### Minimal Dependency Set (v1)

These are the crates that should be in Cargo.toml from day one:

```toml
[dependencies]
# Core
serde = { version = "1", features = ["derive"] }
serde_with = "3"

# Serialization
quick-xml = "0.36"

# Data structures
slotmap = "1"
indexmap = "2"

# Graph
petgraph = "0.6"

# Parsing
tree-sitter = "0.24"  # optional until import phase

# GUI
egui = "0.28"
eframe = "0.28"       # windowing

# Configuration
figment = { version = "0.10", features = ["toml", "env"] }
toml = "0.8"

# Error handling
thiserror = "2"
anyhow = "1"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Templating
askama = "0.13"

# Compression
flate2 = "1"
tar = "0.4"
zip = "2"

# Image export
image = "0.25"
resvg = "0.42"
usvg = "0.42"

# Concurrency
rayon = "1.10"
crossbeam = "0.8"

# Testing
[dev-dependencies]
rstest = "0.22"
insta = "1"
proptest = "1"
pretty_assertions = "1"
```

### Extended / Phase 2 Dependencies

```toml
# i18n
fluent = "0.16"
fluent-bundle = "0.15"
fluent-resmgr = "0.6"
unic-langid = "0.9"

# Async I/O (for file operations)
tokio = { version = "1", features = ["fs", "io-util", "sync"] }

# Plugin system (v2)
wasmtime = "24"

# Additional
indoc = "2"
ico = "0.3"           # application icon handling
xdg = "2.6"           # XDG base directory spec
directories = "5"     # config/data paths
```

---

## 19. Crate Compatibility Matrix

| Crate | Works with | Notes |
|-------|-----------|-------|
| `serde` | All | Foundation for everything |
| `quick-xml` | `serde` | Via `quick_xml::de`/`ser` features |
| `egui` | `eframe` | Standard windowing setup |
| `eframe` | `egui`, `tracing` | Built-in logging integration |
| `figment` | `serde`, `toml`, `clap` | All providers work together |
| `tracing` | `tracing-subscriber`, `tracing-appender` | Ecosystem is well integrated |
| `rayon` | Any | No special integration needed |
| `crossbeam` | Any | Channel types work alongside `tokio` |
| `tree-sitter` | Any | C API — links via build.rs |
| `resvg` | `usvg`, `image` | Standard SVG rasterization pipeline |
| `slotmap` | `serde` | Feature: `slotmap/serde` for serialization |
| `petgraph` | `serde` | Feature: `petgraph/serde` |
| `askama` | None | Compile-time — no runtime deps |

### Known Conflicts

| Conflict | Explanation | Resolution |
|----------|-------------|------------|
| `egui` ↔ `tauri` | Both want to own the event loop | Choose one, not both |
| `tokio` ↔ `egui` | Two event loops | Use `tokio` only for background tasks, not as main runtime |
| `anyhow` ↔ `thiserror` | Different error philosophies | Both: `thiserror` for libs, `anyhow` for app glue |
| `rayon` ↔ `wasm` | Rayon doesn't compile to WASM | No issue — Umbrello-RS is a desktop app |

---

## 20. Where We Should Roll Our Own

Some things are better implemented than taken from a crate:

### 20.1 UML Model Types (core/model)

**Do NOT use a crate.** The UML model domain is specific to Umbrello. The 30+ object types have unique semantics (UML compliance, diagram interactions). No crate provides this.

**Do:** Define `UmlObject` enum, `UmlClassifier` struct, etc. with serde derives. Use `slotmap` for storage.

### 20.2 XMI Forward-Reference Resolver

**Do NOT use a crate.** XMI's forward-reference pattern (`xmi:id` → `xmi:idref`) is specific to the UML XMI format. No crate understands UML XMI semantics.

**Do:** Implement a two-phase deserializer: phase 1 collects all objects with `xmi:id`; phase 2 resolves `xmi:idref` attributes.

### 20.3 CodeDocument / TextBlock Tree

**Do NOT use a crate.** The advanced code generation architecture (document tree with sync, editing, and code-model consistency) is unique to Umbrello.

**Do:** Port the `CodeDocument`, `TextBlock`, `CodeOperation`, `CodeClassField`, `ClassifierCodeDocument` classes from C++. They are well-designed and worth preserving.

### 20.4 Widget Types (umlwidgets)

**Do NOT use a crate.** The 29+ diagram widget types, their rendering logic, interaction behavior, and UML-specific appearance are entirely custom.

**Do:** Port the widget hierarchy to Rust using `egui`'s custom widget API.

### 20.5 Association Routing Logic

**Do NOT use a crate.** AssociationWidget's 4 routing styles (Direct, Orthogonal, Polyline, Spline) and label placement are unique.

**Do:** Port the `AssociationLine` algorithm — it's mature and well-tested.

### 20.6 Interaction State Machine (ToolBarState)

**Do NOT use a crate.** The 7-state toolbar pattern (arrow, widget creation, association drawing, message creation) is specific to Umbrello.

**Do:** Port the state machine; it's a clean pattern.

### 20.7 Undo/Redo Infrastructure

**Do NOT use a crate** (as argued in section 6). The command pattern for UML operations is simple enough that a crate adds more complexity than it saves.

**Do:** Implement `UndoStack` with `Box<dyn Command>`.

### 20.8 Settings Model (OptionState)

**Do NOT use a crate** for the settings *model* — but use `figment` for the *persistence* layer. The `Settings` struct is specific to Umbrello.

**Do:** Define `Settings` with serde, load/save with `figment`.

### 20.9 Event Bus (Signal Replacement)

**Do NOT use a crate** for the core event bus (though you could use `tokio::sync::broadcast` or `event_listener`). The event types are UML-specific.

**Do:** Define `ModelEvent` enum (ObjectCreated, ObjectModified, etc.) and a simple `broadcast`-based bus.

### 20.10 Object/Widget Factories

**Do NOT use a crate.** The factory dispatch logic is simple enough to implement as registries (HashMap of type → constructor). No crate improves on this.

**Do:** `type CreateObjectFn = fn() -> UmlObject;` and a `HashMap<ObjectType, CreateObjectFn>`.

### Summary of "Roll Our Own"

| Module | Lines of code (estimated) | Priority |
|--------|--------------------------|----------|
| Model types (core/model) | 2,000–3,000 | P0 |
| CodeDocument tree | 1,500–2,500 | P0 |
| XMI forward-ref resolver | 300–500 | P0 |
| Widget types | 4,000–6,000 | P0 |
| Association routing | 800–1,200 | P0 |
| Interaction state machine | 500–800 | P1 |
| Undo/redo | 200–400 | P1 |
| Event bus | 100–200 | P1 |
| Factories | 200–400 | P1 |
| Settings model | 300–500 | P1 |

---

## Appendix: Quick Reference — Why Each "Winner"

| Crate | Why chosen over alternatives |
|-------|-----------------------------|
| `serde` + `quick-xml` | Fastest XML, serde ecosystem eliminates 50+ hand-written methods. The only combo that makes XMI maintainable. |
| `petgraph` | Most algorithms, best documentation, used by rustc. DAG-specific crate not flexible enough for general UML graph. |
| `slotmap` | Generational indices are perfect for ID-based model storage. `petgraph` and `slotmap` complement each other. |
| `tree-sitter` | Only solution that covers all 10+ import languages with production-grade parsers. |
| `egui` | Best Rust 2D canvas. Immediate mode fits diagram rendering well. Largest Rust-native GUI ecosystem. |
| `askama` | Type-safe templates catch errors at compile time. Simpler than `tera` for code generation. |
| `figment` | Layered config model matches settings architecture. Better than `config` for composeable defaults + file + env. |
| `tracing` | Structured logging aids debugging complex pipelines. Span model is invaluable for import/codegen profiling. |
| `thiserror` + `anyhow` | Industry standard. `thiserror` for libs, `anyhow` for apps. |
| `rayon` | Simplest way to parallelize code generation and import. `par_iter()` is cheaper than tokio for CPU work. |
| `fluent-rs` | Best designed i18n system. Modern plural/gender support. Worth investing in over `gettext`. |
| `resvg` + `image` | Industry-standard SVG rasterization. Outputs pixel-perfect PNG/JPEG/BMP. |
| `flate2` + `tar` + `zip` | The standard Rust archive toolset. |
| `rstest` + `insta` + `proptest` | Covers parameterized, snapshot, and property-based testing. Best-in-class for each. |
| `indexmap` | Deterministic iteration order matters for XMI round-trip fidelity. |
| `indoc` | Makes multi-line code template strings readable. |

---

*This document is a living reference. Revisit as the Rust ecosystem evolves and as the Umbrello-RS implementation progresses.*
