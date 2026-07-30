# Property Authoring and Edge Interaction Fixes

**Status:** implemented and approved by G4 review
**Scope:** property-panel interaction ownership, classifier-feature authoring, UML edge endpoints/styles, and live edge tracking during node drag
**Plan date:** 2026-07-30

## Goal

Make the existing Properties inspector usable without losing selection, provide command-backed creation/editing/deletion of classifier attributes and operations, render all six supported relationship kinds with visible UML notation, and keep connected edges attached to a node throughout a native drag gesture.

## Verified Current Behavior

- `apps/umbrello/src/canvas.rs::render_canvas` inspects the global primary-click state for background deselection but does not constrain the click position to `canvas_rect`. A click in the right `SidePanel` therefore calls `clear_selection()` because no canvas node handled it.
- `apps/umbrello/src/property_editor.rs` exposes persistent editable base and relationship fields, but classifier attributes and operations are copied into `ClassifierSnapshot` and rendered as labels only.
- `Attribute`, `Operation`, `Parameter`, `TypeReference`, and `ClassifierData` already exist in `uml-core`; the XMI reader and writer already preserve them. No reversible classifier-feature update command exists.
- `screen_edge_paths` builds center-to-center paths. `draw_edges` places target arrowheads only 20 screen pixels from the target center and source diamonds at the source center, then node fills are painted over the edges. With normal 160x60 nodes, all arrowheads are hidden under opaque node fills.
- During drag, a node uses `drag_preview_pos`, while edge paths use the frame-start `Diagram` clone with stored node bounds. Edges therefore move only after `MoveNode` commits on release.
- The worktree was clean at investigation start. Existing tests cover base property editing, relationship drafts, edge creation, and one-command native drag, but not panel click ownership, classifier-feature CRUD, boundary-anchored UML notation, or live edge tracking.

## Scope

1. Constrain canvas background deselection to primary clicks whose pointer position is inside the canvas and not on a node or edge.
2. Add one reversible `UpdateClassifierFeatures` command that atomically replaces editable attributes and operations on an existing classifier.
3. Add a persistent classifier draft to Properties with Add/Delete/Edit for attributes, operations, and operation parameters, plus Apply/Revert.
4. Expose the classifier editor through the existing seven generic MCP tools using semantic property/action targets; do not add an eighth tool.
5. Clip the first and last edge segments to source/target node rectangle boundaries and place UML arrowheads/diamonds at those visible boundaries.
6. Make edge path calculation use transient drag-preview bounds for a dragged source or target.
7. Add focused core, UI, geometry, history, persistence, and native MCP QA coverage.

## Explicit Non-goals

- Template-parameter, enum-literal, stereotype, port, note, or constraint authoring.
- Attribute/operation reordering, stable subordinate IDs, overload validation, language-specific signature validation, or code-generation integration.
- A model-element type chooser/autocomplete. Editing a type as text creates a primitive/external `TypeReference`; an untouched existing model-ID reference remains unchanged.
- Dynamic node resizing, resize handles, orthogonal routing editors, automatic layout, or exact C++ pixel parity.
- Relationship endpoint editing or new relationship kinds.
- XMI schema changes or preservation of original subordinate feature IDs; current feature IDs remain writer-generated.

## Architectural Decisions and Invariants

### Canvas interaction ownership

Background deselection continues to avoid an overlapping `Sense::click`, because that previously stole pointer ownership from node drag responses. The global click is accepted only when its `press_origin` (falling back to the latest pointer position) lies inside the current `canvas_rect`. Clicks in menus, tree, tool palette, or Properties are not canvas background clicks.

### Classifier feature mutation

`UpdateClassifierFeatures` is a model-only command in `uml-core`. It identifies the classifier by `UmlId`, captures the old `ClassifierData`, and accepts a replacement whose `templates` equal the current templates. It may change only `attributes` and `operations`; classifier identity, element base fields, and templates are preserved.

The command follows the optimistic snapshot pattern used by `UpdateRelationship`: execute/undo verify the currently stored classifier data matches the expected snapshot before replacing it. Invalid IDs, non-classifiers, stale snapshots, repeated execute/undo, or template changes fail without mutation. One Properties Apply action creates one history entry; draft edits, Add/Delete buttons, and Revert are transient and do not dirty the model.

### Persistent classifier draft

`UmbrelloApp` stores `Option<(UmlId, ClassifierDraft)>`. The draft owns editable copies of `attributes` and `operations`, including operation parameters. Selection changes, undo/redo, open/new, Apply, and Revert refresh or clear it with the existing property-buffer lifecycle.

Properties supports:

- attributes: name, type text, visibility, initial value, and static flag;
- operations: name, return-type text, visibility, static/abstract/virtual flags;
- parameters: name, type text, direction, default value;
- Add/Delete controls at each applicable level and explicit Apply/Revert.

New entries use deterministic gap-filled names (`attribute_1`, `operation_1`, `parameter_1`) within the current draft. Empty optional values become `None`. Empty type text becomes `TypeReference::unspecified()`. Editing non-empty type text creates `TypeReference::primitive(text)` and clears `model_id`; untouched model-backed type references retain their original `model_id`.

The UI rejects empty attribute/operation/parameter names on Apply and leaves the draft/model unchanged with a visible status error. Duplicate names are permitted because UML operation overloading and language-specific constraints are outside this milestone.

### Edge geometry and notation

One shared screen path remains the source of truth for rendering, hit testing, selection highlighting, and tests. Before transforming to screen coordinates, its first segment exits the source `Rect` boundary and its last segment enters the target `Rect` boundary. Waypoints remain unchanged and determine first/last segment direction when present.

An axis-aligned slab/Liang-Barsky intersection helper must handle horizontal, vertical, diagonal, waypoint, zero-length, overlapping, and zero-size cases without panic or non-finite output. Degenerate cases use a deterministic center fallback and may omit an arrowhead when no usable final direction exists.

With clipped endpoints:

- Generalization: solid line plus hollow triangle at target boundary.
- Realization: dashed line plus hollow triangle at target boundary.
- Association: plain solid line.
- Aggregation: solid line plus hollow diamond at source boundary.
- Composition: solid line plus filled diamond at source boundary.
- Dependency: dashed line plus open arrow at target boundary.

Missing/non-relationship semantic references are skipped rather than silently rendered as associations. Existing persisted `AssociationType` remains the only style discriminator.

### Live drag

`screen_edge_paths` clones only endpoint `ViewNode` values needed for path calculation and substitutes `drag_preview_pos` while `drag_node_id` matches either endpoint. This transient geometry never mutates `UmlModel`, dirty state, or history. Release still commits exactly one `MoveNode`; cancel/no-motion behavior remains unchanged.

## Data and Control Flow

```text
node/browser click
  -> select_element
  -> refresh_property_buffers
  -> ClassifierDraft(attributes, operations, parameters)
  -> transient edits/Add/Delete
  -> Apply
  -> validate draft
  -> UpdateClassifierFeatures
  -> History::execute
  -> dirty + state revision + refreshed draft

primary click frame
  -> pointer position inside canvas_rect?
  -> not handled by node/edge?
  -> clear semantic selection

Diagram + ViewEdge + Relationship.kind
  -> drag-adjusted endpoint bounds
  -> boundary-clipped shared ScreenEdgePath
  -> hit testing / selection highlight / UML renderer
```

## Persistence, Compatibility, Error Handling, and UI Effects

- No reader/writer changes are expected. Existing UML 1.2 feature serialization must round-trip UI-authored attributes, operations, return types, and parameters.
- `ElementBase.original_xmi_id` is untouched. Writer-assigned subordinate IDs continue through the collision-safe allocator.
- Failed classifier Apply, stale command execution, or invalid feature data must not mutate the model, dirty state, history, or templates.
- Undo/redo restores the complete attribute/operation snapshot and refreshes visible drafts through existing application normalization paths.
- Edge clipping affects only view geometry; semantic endpoints, waypoints, ordering, XMI, and command history are unchanged.
- Native Properties and MCP operate the same draft/apply path. MCP target IDs are index-based within the current draft and are rediscovered after Add/Delete; callers must not cache stale indices.

## Ordered Subtasks

### S1 — Reversible classifier-feature command

**Owned files:**

- `docs/designs/ui_property_and_edge_interaction_fixes.md`
- `crates/uml-core/src/undo/commands.rs`
- `crates/uml-core/src/undo/mod.rs`

**Dependencies:** none.

Implement/export `UpdateClassifierFeatures` with snapshot, classifier/template validation, failure atomicity, and execute/undo/redo tests.

**Acceptance criteria:**

- Class, Interface, Enum, and Datatype feature snapshots can be changed and restored.
- Non-classifier/missing IDs, template changes, stale snapshots, repeated execution, and invalid undo are rejected without mutation.
- Attribute/operation/parameter/type-reference values survive execute, undo, and redo exactly.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-core update_classifier_features
cargo test -p uml-core undo
```

### S2 — Properties ownership and classifier authoring

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/property_editor.rs`
- `apps/umbrello/src/qa/control.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S1.

Fix canvas-only deselection, add persistent classifier drafts, implement complete attribute/operation/parameter Add/Edit/Delete with Apply/Revert, wire command/history/dirty/buffer synchronization, and expose equivalent generic MCP targets/actions.

**Acceptance criteria:**

- Clicking or typing anywhere in Properties does not clear selection; a genuine empty-canvas click still does.
- Classifier feature controls are absent for non-classifiers and relationships.
- Add/Edit/Delete and Revert are transient until Apply; Apply is one command and one dirty transition.
- All specified fields, operation parameters, visibility/direction enums, optional values, and type text are editable.
- Empty required names fail safely; template data and untouched model-backed type references are preserved.
- Undo/redo and selection changes refresh the visible draft without stale edits.
- Existing seven MCP tools discover and drive the same classifier authoring flow.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello property
cargo test -p umbrello classifier
cargo test -p umbrello qa
cargo test -p umbrello background_click
cargo test -p uml-io xmi
```

### S3 — Visible UML edge notation and live attachment

**Owned files:**

- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S2.

Implement robust endpoint clipping, boundary-anchored notation, invalid-semantic-edge skipping, and drag-preview endpoint substitution in the shared screen path.

**Acceptance criteria:**

- All six `AssociationType` values retain their distinct line/arrowhead dispatch.
- First/last path points lie on the appropriate node boundaries for straight and waypoint paths at different zoom levels.
- Arrowheads and diamonds are outside opaque node interiors and remain visible after nodes are painted.
- Hit testing and selection highlighting use the clipped path.
- Connected edge endpoints follow source or target drag preview every frame; no model/history mutation occurs until release.
- Overlap, coincident centers, malformed/missing semantic edges, and degenerate segments do not panic or produce non-finite points.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello edge
cargo test -p umbrello drag
cargo test -p umbrello boundary
```

## G4 Findings and Corrective Subtasks

The first full native MCP review invalidated two original assumptions: existing XMI feature persistence did not serialize `Parameter.default_value`, and the reader created a second semantic relationship from each persisted `assocwidget`. The populated parameter editor also exposed an unbounded SidePanel sizing defect. Repository behavior overrides the earlier “no reader/writer changes expected” assumption; the cycle includes the following corrective work before G4 can approve.

### S6 — Preserve parameter default values in XMI

**Owned files:**

- `crates/uml-io/src/xmi/reader.rs`
- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** S2F4.

Write non-empty/explicit `Parameter.default_value` as the C++-compatible UML 1.2 `value` attribute. Read `value` and accept legacy `initialValue` as a fallback. Add reader, writer, and semantic round-trip tests using operation parameters with all other editable fields.

**Acceptance criteria:**

- `Some` default text survives write/read exactly; `None` remains absent.
- Reader accepts both `value` and legacy `initialValue`, preferring canonical `value` when both are present.
- Output remains loadable by C++ Umbrello conventions and does not affect attribute initial values or return parameters.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io parameter_default
cargo test -p uml-io xmi
```

### S7 — Canonical relationship/widget identity without duplication

**Owned files:**

- `crates/uml-io/src/xmi/reader.rs`
- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** S6.

Match C++ Umbrello by writing each `assocwidget@xmi.id` as the referenced semantic relationship XMI ID instead of allocating an unrelated ID. During read, pre-register semantic relationship XMI IDs and ensure the diagram `ViewEdge` and semantic definition resolve to one `UmlId` regardless of whether the widget or pass-2 semantic resolution is encountered first.

For old Rust output whose assocwidget ID differs from its semantic relationship ID, use a deterministic per-diagram ordered fallback over unclaimed semantic candidates with the same kind and directed endpoints. A candidate may be reused in another diagram, while parallel same-kind relationships between identical endpoints remain distinct by occurrence order. Only create a widget-only semantic relationship when no compatible semantic candidate exists. Never collapse relationships globally by `(kind, source, target)` alone.

**Acceptance criteria:**

- New writer output shares semantic/widget XMI identity and reloads one semantic relationship plus one view edge per authored edge.
- Generalization, Realization, Association, Aggregation, Composition, and Dependency round-trip without duplication.
- Parallel same-kind/same-endpoint relationships remain distinct.
- One semantic relationship shown in multiple diagrams remains one model element with multiple view edges.
- C++ shared-ID input, old Rust separate-ID input, semantic-only input, and genuinely widget-only foreign input all follow deterministic behavior.
- Repeated save/reload does not grow relationship count and the real corpus still parses.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io relationship_identity
cargo test -p uml-io round_trip_with
cargo test -p uml-io --test test_real_corpus
```

### S8 — Bound and scroll the Properties inspector

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/property_editor.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S2F3.

Keep the right SidePanel before CentralPanel, give it a bounded resizable width, wrap the inspector in a vertical `ScrollArea`, constrain text edits, and split dense parameter controls into narrow rows so content cannot force the panel to consume the canvas.

**Acceptance criteria:**

- At the default 1024x768 viewport with a populated classifier draft, the canvas retains a useful non-empty width of at least 300 logical pixels.
- At 1741x1306 the same draft leaves at least 600 logical pixels for the canvas and all classifier controls remain reachable by scrolling.
- Properties clicks preserve selection and empty-canvas clicks still clear it.
- Attribute/operation/parameter edits, Apply/Revert, relationship fields, and multi-frame draft persistence remain functional.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello property
cargo test -p umbrello classifier
cargo test -p umbrello layout
cargo test -p umbrello background_click
```

## Integration and Review Gates

### G1 — Core gate after S1

Architect inspects the commit/diff and runs S1 validation. No UI work starts until the command contract and failure atomicity pass.

### G2 — Properties gate after S2

Architect inspects selection ownership, drafts, command integration, MCP target parity, and tests; runs S2 validation. Regressions in native node drag or relationship properties block S3.

### G3 — Integrated static gate after S3

Run:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build -p umbrello
```

### G4 — Mandatory independent review and automatic native QA

The independent `reviewer` reviews the full plan and integrated commit range, including the final `AGENTS.md` update. It must load the project-local `umbrello-mcp-qa` skill and exercise the actual `target/debug/umbrello --mcp-stdio` application through the seven generic tools.

The QA scenario must dynamically create/open a project and Class diagram, create at least two classifier nodes, select one, interact with Properties without losing selection, add/edit/apply an attribute and an operation with a parameter, synchronize, verify history, create at least Generalization, Realization, Composition, and Dependency relationships, drag a connected node with gesture mode, synchronize, undo, save/relaunch, inspect persisted features/edges, and capture a synchronized PNG screenshot plus transcript under `/tmp/opencode/umbrello-property-edge-qa`.

Because static semantic snapshots cannot prove arrowhead visibility or intermediate native drag rendering, the reviewer must visually inspect screenshots captured after relationship creation and, where the generic MCP surface permits, during/after the gesture. Any environment blocker must be reported verbatim with the strongest attempted evidence. Blocking/major findings require a fix subtask assigned back to the original implementer followed by repeated validation and review.

## Completion Criteria

- S1-S8 and accepted fixes are committed with exact ownership.
- G3 passes with no warnings or failures.
- The automatic native MCP QA artifacts exist and the reviewer approves G4.
- `AGENTS.md` records the durable behavior, implementation locations, verified commands, and remaining limitations.
- Final status/diff/commit inspection shows no unrelated changes.
