# Milestone 23 — Component, Node, and Artifact

**Status:** implementation plan
**Reference implementation:** `../umbrello/umbrello/umlmodel/{umlcomponent,umlnode,umlartifact}.*` and `../umbrello/umbrello/umlwidgets/{componentwidget,nodewidget,artifactwidget}.*`
**Scope:** end-to-end domain, XMI, native GUI, and semantic MCP support for three structural UML elements

## Goal

Add Component, deployment Node, and Artifact as first-class `ModelElement` variants and make them usable through the complete Umbrello-RS path: construction, serde, UML 1.2 XMI read/write, diagram widget persistence, native palette creation, C++-inspired rendering, common property editing, undo/redo, and MCP GUI automation.

## Why This Is Milestone 23

M22 completed the P0 viewport foundation needed to inspect realistic component and deployment diagrams. Component, Node, and Artifact are the next highest-priority semantic gap in `AGENTS.md`; their `ObjectType` and diagram kinds already exist, but files containing these C++ Umbrello model tags currently drop them silently. Implementing the three related types together forms one coherent vertical slice and unlocks both Component and Deployment diagrams without adding a new subsystem.

Resize handles, package hierarchy UI, diagram tabs, and context menus remain valuable but expose or reorganize existing semantics. M23 instead expands interchange compatibility and the actual UML model supported by the rewrite.

## Current Behavior

- `ObjectType::{Component, Node, Artifact}` and `DiagramKind::{Component, Deployment}` exist.
- `ModelElement` has no corresponding variants; reader Start/Empty dispatch therefore ignores `UML:Component`, `UML:Node`, and `UML:Artifact`.
- The XMI diagram reader recognizes `componentwidget` and legacy `deploymentwidget`, but not `nodewidget` or `artifactwidget`. Widget references cannot resolve while the model elements are absent.
- The writer has no model-element or widget mapping for the three types.
- `ToolMode` supports seven node types; generic `CreateElementWithNode` already provides atomic placement and undo/redo.
- Canvas rendering falls back to a generic rectangle/name for unknown variants.
- MCP has generic tool, canvas, node, property, history, sync, and screenshot operations; only semantic palette targets are missing.
- The current checked-in XMI corpus contains Component View folders but no actual model/widget examples for these three types, so focused synthetic compatibility tests are required.

## C++ Compatibility Contract

### Model tags and scalar state

- `UMLComponent` (`umlmodel/umlcomponent.cpp`) writes `<UML:Component executable="0|1">`; default is `false`.
- `UMLNode` (`umlmodel/umlnode.cpp`) writes `<UML:Node>` and adds no scalar state beyond common UML object metadata.
- `UMLArtifact` (`umlmodel/umlartifact.cpp`) writes `<UML:Artifact drawas="N">`; values are `0=default`, `1=file`, `2=library`, `3=table`.
- All three preserve common IDs, name, visibility, stereotype reference, documentation, flags, and `original_xmi_id` through the Rust `ElementBase` path.

### Diagram widget tags

- Component: `componentwidget`
- Node: `nodewidget` (reader retains `deploymentwidget` as a legacy alias)
- Artifact: `artifactwidget`

Widget geometry and diagram zoom use the existing generic `ViewNode` and M22 XMI path.

### Rendering

- Component: UML 2 rectangle with the small component glyph at upper right; executable components use a visibly heavier outline.
- Node: C++-style three-dimensional box with top/side depth and centered bold name.
- Artifact: preserve all four draw modes and render a deterministic C++-inspired shape for each: normal box, folded-corner file, library icon, and table grid.

Exact Qt font metrics, theme colors, legacy UML 1 component tabs, ports, and pixel-identical rendering are not required.

## Scope

1. Add `Component`, `Node`, `Artifact`, and a four-value artifact draw-mode enum to `uml-core`.
2. Propagate all exhaustive `ModelElement` dispatch and serde behavior.
3. Parse/write model tags, scalar attributes, widget tags, geometry, IDs, and zoom in UML 1.2 XMI.
4. Add three native one-shot creation tools using `CreateElementWithNode`.
5. Add deterministic C++-inspired rendering and element colors.
6. Expose `tool.component`, `tool.node`, and `tool.artifact` through the existing seven MCP tools.
7. Exercise creation, rename, movement, undo/redo, synchronization, and screenshot in the actual native application through MCP.

## Non-goals

- Ports, provided/required interfaces, nested port movement, or component interface glyphs.
- Nested Component/Artifact containment, package-browser hierarchy changes, or a generic container refactor. C++ uses `UMLPackage` inheritance for these classes, but M23 models the serialized scalar identity needed by current interchange; containment requires a separate repository design.
- Component classifier features, deployment instances, manifestations, or Artifact file-system integration.
- New relationship kinds, resize handles, tabs, context menus, auto-layout, or dynamic sizing.
- New MCP tools or dependencies.
- XMI 2.x/foreign dialect support or C++ source changes.

## Architectural Decisions and Invariants

### Composition and enum dispatch

Each new type embeds `ElementBase`; `Component` adds `executable`, `Artifact` adds `ArtifactDrawMode`, and `Node` has no additional fields. They become explicit `ModelElement` variants. No inheritance or trait-object hierarchy is introduced.

The variants are not treated as current Rust classifiers and do not expose `ClassifierData`. They are not `Package` variants and do not participate in `Package::children` during M23. `ObjectType::is_container()` remains the existing high-level type capability; repository-backed nested component/artifact containment is explicitly deferred rather than faked through parent-index entries.

### Safe scalar defaults

- `Component::new()` sets `executable = false`.
- `Artifact::new()` sets `draw_as = ArtifactDrawMode::Default`.
- Missing or malformed XMI scalar values use those defaults.
- Unknown integer `drawas` values default safely to `Default`; they do not fail the whole document.

### Commands and persistence

Native/MCP creation uses the existing atomic `CreateElementWithNode` command. Common property edits continue through existing commands. XMI loading constructs model data directly as today. Diagram/model ordering stays deterministic.

### MCP surface

No eighth tool is added. The semantic target table gains the three tool IDs. Once created, the existing `node:<UmlId>` and property targets operate the variants generically.

## Data Flow

```text
UI or MCP tool selection
  -> ToolMode::{CreateComponent,CreateNode,CreateArtifact}
  -> construct ModelElement with unique default name
  -> CreateElementWithNode command
  -> UmlModel + active Diagram updated atomically
  -> generic selection/property/history/MCP node paths
  -> XMI writer emits model tag + scalar state + widget geometry
```

Reader flow dispatches Start and Empty forms to type-specific parsers, registers the `original_xmi_id`, and allows the existing diagram pass to resolve matching widget IDs.

## Ordered Subtasks

### S1 — Core domain types

**Owned files:**

- `crates/uml-core/src/elements.rs`
- `crates/uml-core/src/lib.rs`
- `crates/uml-core/src/repository.rs`
- `crates/uml-core/tests/serde_roundtrip.rs`

**Dependencies:** none.

Add the scalar enum/structs, constructors, public exports, all exhaustive match arms, unit tests, and external serde round trips. Keep all three non-classifier and non-package in current repository behavior.

**Acceptance criteria:**

- Construction creates unique IDs and correct `ObjectType` values.
- Defaults match C++.
- All variants support base/base_mut/name/object type dispatch.
- JSON round trips preserve executable and all artifact modes.
- Existing generic command/repository behavior compiles without special cases.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-core component
cargo test -p uml-core node
cargo test -p uml-core artifact
cargo test -p uml-core
```

**Commit boundary:** one focused `uml-core` feature commit created by the implementer.

### S2 — UML 1.2 XMI model and widget persistence

**Owned files:**

- `crates/uml-io/src/xmi/reader.rs`
- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** S1 commit.

Add Start/Empty reader dispatch, scalar parsing, writer dispatch/helpers, and widget mappings. Add synthetic parser/writer tests covering all three types, both element forms where relevant, scalar defaults, all artifact modes, IDs, geometry, and non-default diagram zoom. Preserve the root-level `XMI.extensions` Empty-event regression from M22.

**Acceptance criteria:**

- Semantic read/write/read preserves type, name, common metadata, executable, draw mode, original XMI IDs, widget identity/bounds, and zoom.
- `nodewidget` and `artifactwidget` parse; `deploymentwidget` remains accepted.
- Missing/malformed scalar attributes default safely.
- Unknown model/widget tags remain leniently skipped.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io component
cargo test -p uml-io node
cargo test -p uml-io artifact
cargo test -p uml-io xmi
```

**Commit boundary:** one focused `uml-io` feature commit created by the implementer.

### S3 — Native creation and rendering

**Owned files:**

- `apps/umbrello/src/tool_palette.rs`
- `apps/umbrello/src/rendering.rs`
- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S1 commit.

Add three palette tools with non-conflicting labels/tooltips and no mandatory keyboard shortcut. Extend unique creation, one-shot placement, colors, and rendering. Keep the generic 160×60 placement bounds and viewport transform.

**Acceptance criteria:**

- Palette creation yields `Component_1`, `Node_1`, and `Artifact_1` with gap-filling names.
- Each placement is one atomic history entry; undo removes model+node and redo restores both.
- Rendering dispatch covers all three shapes and all artifact modes without panic.
- Selection highlighting and common property editing remain generic and functional.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello component
cargo test -p umbrello node
cargo test -p umbrello artifact
cargo test -p umbrello
```

### S4 — MCP semantic targets and app integration

**Owned files:**

- `apps/umbrello/src/qa/control.rs`
- `apps/umbrello/src/tests.rs`
- `apps/umbrello/tests/mcp_stdio.rs`

**Dependencies:** S3 integrated.

Add `tool.component`, `tool.node`, and `tool.artifact` to semantic discovery/dispatch and focused tests. Preserve exactly seven generic MCP tools and existing schemas.

**Acceptance criteria:**

- `ui_inspect` exposes all three enabled tool targets.
- MCP selection/canvas click creates each type through the same atomic app path.
- Created elements appear as durable `node:<UmlId>` targets and support generic selection, rename, movement, undo/redo, sync, and screenshot.
- Existing tool target IDs and transport behavior remain compatible.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello qa
cargo test -p umbrello --test mcp_stdio
cargo test -p umbrello
```

**Commit boundary for S3+S4:** after integrated app validation, one focused app feature commit created by the implementer owning the overlapping app tests.

### S5 — Durable documentation and closure

**Owned files:**

- `AGENTS.md`
- `docs/designs/milestone_23_component_node_artifact.md`
- `docs/reviews/milestone_23_g4_final_review.md`

**Dependencies:** S1–S4 validated and final review approved.

The architect updates `AGENTS.md`; an implementer creates the exact documentation closure commit after reviewer approval. Reconcile the domain/XMI gap tables, model type list, source map, milestone summary, and current test accounting.

## Integrated Acceptance Criteria

1. Component, Node, and Artifact are first-class Rust model variants with safe C++-compatible scalar defaults.
2. UML 1.2 XMI semantically round-trips all three model tags and widget geometry, including non-default zoom.
3. Native users can create, select, rename, move, undo, and redo each type.
4. C++-inspired shapes are visible and distinguishable at normal and transformed viewport scales.
5. MCP exposes the three palette targets through the existing seven tools and completes synchronized native screenshot QA.
6. No unsafe code, dependency addition, C++ modification, inheritance emulation, or unrelated worktree change is introduced.

## Integrated Validation

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

## Runtime MCP QA

Launch:

```sh
cargo run -p umbrello -- --mcp-stdio
```

Using a conforming newline-delimited MCP client while preserving the display environment:

1. Initialize and list exactly seven tools.
2. Create a class diagram if none exists.
3. Select `tool.component`, click canvas, synchronize, and inspect the new node.
4. Repeat for `tool.node` and `tool.artifact` at distinct positions.
5. Select one node, rename it through `property.name`, move it with `ui_drag`, and verify state.
6. Undo and redo an atomic creation.
7. Capture and inspect a synchronized PNG screenshot containing all three distinguishable shapes.
8. Verify protocol-only stdout and clean stdin-EOF shutdown.

## Integration and Review Gates

- **G1:** inspect S1 diff/commit, exhaustive dispatch, serde tests, and crate boundary.
- **G2:** inspect S2 diff/commit and semantic XMI round-trip including root-level widget output.
- **G3:** inspect integrated S3/S4 app diff/commit and run full workspace validation.
- **G4-final:** independent reviewer inspects the complete M23 commit range plus `AGENTS.md`, reruns targeted/full validation, and performs actual MCP runtime QA and screenshot capture.
- Any defect becomes a stable fix subtask resumed with the implementer that owns the affected subsystem; fixes receive separate implementer-created commits and the final gate repeats.
