# Milestone 24 — Usability Foundations

**Status:** implemented and approved  
**Scope:** coherent project/diagram authoring, diagram-aware tools, clickable model browser, relationship selection/editing, and interaction correctness  
**Audit date:** 2026-07-29

## Goal

Make the existing Umbrello-RS feature set usable as a coherent UML editor instead of a collection of independently reachable capabilities. A user must create an XMI project first, create and immediately enter any diagram kind that the current semantic/rendering stack can honestly author, see only valid creation choices for that diagram, reuse and select model elements from the browser, and select and edit relationships from the canvas.

M24 is a usability and correctness milestone. It does not add new UML element kinds.

## Personally Verified Current Behavior

The architect inspected the current implementation and exercised the native application through its stdio MCP server. The worktree was clean at audit start.

### MCP evidence

The following sequence was run against `target/debug/umbrello --mcp-stdio` using `ui_inspect`, `ui_select`, `ui_click`, `ui_drag`, and `ui_screenshot`:

1. A fresh application exposed only `diagram.new_class`; all element tools, including `tool.use_case`, were enabled before any diagram existed.
2. Activating `diagram.new_class` created a Class diagram but left `active_diagram = null`.
3. After manually activating that diagram, `tool.use_case` remained enabled and successfully created `UseCase_1` in the Class diagram.
4. A Class and the UseCase could be connected with `tool.association`.
5. After edge creation, `ui_inspect` exposed zero `edge` targets and no relationship property targets.
6. `ui_screenshot` returned a synchronized native viewport image and revision metadata, confirming that the audited state rendered.

### Code-audit findings

- `tree.rs` offers “New Class Diagram” only while the diagram list is empty; there is no Use Case, Component, or Deployment creation flow and no second-diagram flow.
- `new_class_diagram()` mutates `UmlModel` directly, does not mark the file dirty, and does not activate the result.
- `ToolMode` has no diagram compatibility policy. Native buttons, keyboard shortcuts, app methods, and MCP targets all permit every tool in every diagram.
- “Elements” consists of passive labels. Existing semantic elements cannot be selected there or added to another diagram.
- Canvas interaction allocates responses only for nodes. Edges have no hit testing, selection state, highlight, semantic targets, or property editor.
- `Relationship` already stores editable name, documentation, kind, role names, multiplicities, and navigability, but no command or UI exposes those fields.
- The property documentation editor recreates its local edit value from the model each frame, so multi-frame typing is not a stable edit transaction.
- Abstract/static controls are shown even for element kinds where they have no useful authoring meaning.
- Node dragging executes `MoveNode` repeatedly while the pointer is moving, producing many history entries for one gesture.
- The keyboard status assignment runs every idle frame and can overwrite meaningful operation/error messages. Ctrl+Z/Ctrl+Y are not global shortcuts.
- The left panel is not scrollable despite a long palette and unbounded diagram/element lists.
- `File > New` clears to an untitled in-memory model instead of establishing an XMI project path first.
- `prompt_save_if_dirty()` returns success after invoking Save even when Save As is cancelled or saving fails, risking loss of unsaved work.

## Scope

1. Add reversible core commands for diagram creation and relationship-detail updates.
2. Replace “New” with a project-first flow that chooses and writes an XMI file before replacing the current model.
3. Add a reusable New Diagram dialog for the four diagram kinds currently supported end-to-end: Class, Use Case, Component, and Deployment.
4. Activate a newly created diagram, mark the project dirty, and make diagram creation undoable.
5. Enforce a single diagram/tool compatibility matrix in palette rendering, keyboard shortcuts, native placement, edge creation, and MCP discovery/dispatch.
6. Make the model browser scrollable and its elements clickable; allow compatible existing elements to be added to the active diagram through the existing visual-node command path.
7. Select edges by precise path hit testing, highlight them, expose them through MCP, and edit their semantic details through commands.
8. Repair persistent property buffers, one-command-per-drag behavior, global undo/redo shortcuts, and status preservation.

## Explicit Non-goals

- Authoring Sequence, Collaboration, State, Activity, Entity-Relationship, or Object diagrams. Their `DiagramKind` values and loaded XMI remain readable, but missing semantic element/message support makes creation misleading.
- New UML element or relationship kinds, stereotypes, include/extend first-class types, ports, instances, notes, or sequence messages.
- Diagram tabs, diagram rename/delete, model-element deletion semantics, context menus, resize handles, automatic layout, routing editors, or color/font customization.
- Pixel-identical C++ rendering, OS-global automation, a new MCP tool, or new dependencies.
- Rejecting or deleting historically loaded cross-kind content. Compatibility checks apply to new authoring actions only.

Deletion is deliberately deferred: “remove from this diagram” and “delete from the model and all diagrams” need distinct commands and confirmation UX. M24 must not expose a destructive shortcut with ambiguous semantics.

## Architectural Decisions and Invariants

### Honest authoring surface

Only Class, Use Case, Component, and Deployment diagrams are creatable. Other kinds continue to load and render as far as current persistence permits. This avoids advertising incomplete editors.

### Central compatibility policy

`ToolMode` owns one pure compatibility predicate used by all entry points. UI disabling alone is insufficient; `place_element()` and `place_edge()` must reject incompatible actions so keyboard and MCP callers cannot bypass the policy.

The initial matrix is intentionally conservative:

| Diagram | Node tools | Edge tools |
|---|---|---|
| Class | Package, Class, Interface, Enum, Datatype | Association, Generalization, Realization, Aggregation, Composition, Dependency |
| Use Case | Package, Actor, UseCase | Association, Generalization, Dependency |
| Component | Package, Interface, Component, Artifact | Association, Generalization, Realization, Aggregation, Composition, Dependency |
| Deployment | Component, Node, Artifact | Association, Dependency |

`Select` is valid whenever a diagram is active. No creation tool is enabled without an active supported diagram. Switching diagrams resets an incompatible active tool to Select. Existing cross-kind nodes loaded from XMI remain visible and selectable.

### Commands and history

User-created diagrams and relationship edits pass through `History`. Viewport, selection, draft text, and dialog visibility remain transient UI state.

- `CreateDiagram` owns a `Diagram` snapshot and preserves its ID across undo/redo.
- `UpdateRelationship` accepts a replacement `Relationship` only for the same relationship ID and endpoints. It preserves `original_xmi_id` and rejects non-relationship IDs. One Apply action is one history entry.
- Existing `AddNodeToDiagram` is used to place an existing compatible element in another diagram; the app validates model membership, compatibility, and absence before command execution.
- A drag gesture keeps transient preview geometry and executes exactly one `MoveNode` on release. Cancellation restores the original visual state without history mutation.

### Selection model

`selected_element_id` continues to identify a semantic `ModelElement`, including `ModelElement::Relationship`. Node selection and edge selection are mutually exclusive because they share this semantic selection. Browser selection can select an element that has no node in the active diagram; the property panel still edits semantic properties.

Active-diagram relationship targets use `edge:<relationship UmlId>`. Existing node targets remain `node:<element UmlId>`, and browser targets use `element:<element UmlId>`.

### Relationship editing

The relationship inspector provides:

- read-only ID and resolved source/target names;
- editable relationship kind, name, documentation;
- source and target role names, multiplicities, and navigability;
- Apply and Revert actions backed by a persistent draft.

Changing relationship endpoints and visual routing is out of scope. Empty optional role/multiplicity text commits as `None`. The kind choices are limited by the active diagram compatibility policy.

### Project-first file flow

File > New Project and Ctrl+N:

1. resolve dirty-model prompting without discarding data on cancel/failure;
2. request an XMI destination;
3. write a valid empty `UmlModel` to that path;
4. only after successful writing replace app state and set `current_file_path`;
5. leave the project with no diagram so the user explicitly chooses a supported diagram.

Cancelling the destination dialog or failing the initial write leaves the current model, path, history, and dirty state unchanged.

For deterministic MCP testing without opening a native dialog, selecting `file.new` and using `ui_set_text` with an absolute `.xmi` path invokes the same project initialization helper. The seven generic MCP tools remain unchanged.

### Error handling and compatibility

- Invalid diagram/tool combinations return structured errors and preserve state/history.
- Empty diagram names are rejected; generated defaults are unique and deterministic.
- Failed Save/Save As and cancelled dialogs do not authorize a destructive New/Open/Quit continuation.
- XMI schema, ordering, `original_xmi_id`, zoom persistence, and loaded unsupported diagrams remain unchanged.

## Control Flow

```text
New Project -> choose path -> write empty XMI -> replace app state
            -> New Diagram dialog -> CreateDiagram command -> activate diagram
            -> compatibility-filtered palette

Browser element click -> semantic selection -> property inspector
                      -> Add to active diagram -> AddNodeToDiagram command

Canvas edge click -> nearest visible edge segment -> relationship selection
                  -> persistent relationship draft -> Apply
                  -> UpdateRelationship command -> redraw + XMI persistence
```

## Ordered Subtasks

### S1 — Core reversible operations

**Owned files:**

- `crates/uml-core/src/undo/commands.rs`
- `crates/uml-core/src/undo/mod.rs`

**Dependencies:** none.

Implement and export `CreateDiagram` and `UpdateRelationship`. Add focused execute/undo/redo, validation, ID preservation, endpoint preservation, and failure-atomicity tests.

**Acceptance criteria:**

- Diagram create/undo/redo preserves diagram ID, kind, name, zoom, nodes, edges, and insertion position deterministically.
- Relationship update/undo/redo covers kind, base name/documentation, roles, multiplicities, and navigability.
- Relationship ID, source/target IDs, and original XMI ID cannot be changed through the update command.
- Invalid IDs and invalid replacement snapshots do not mutate the model.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-core create_diagram
cargo test -p uml-core update_relationship
cargo test -p uml-core undo
```

### S2 — Project-first and multi-diagram workflow

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/file_io.rs`
- `apps/umbrello/src/menu.rs`
- `apps/umbrello/src/tree.rs`
- `apps/umbrello/src/qa/control.rs`
- `apps/umbrello/src/qa/protocol.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S1.

Implement failure-safe New Project, a native New Diagram dialog, deterministic creation helpers for the four supported kinds, immediate activation, dirty/history integration, MCP project-path and per-kind diagram actions, global undo/redo, and state normalization after undo/redo/open/new.

**Acceptance criteria:**

- New Project writes the XMI before replacing current state; cancel/failure is non-destructive.
- Save-before-destructive-operation proceeds only after confirmed successful save or explicit discard.
- Users can create multiple supported diagrams at any time; each new diagram is active immediately.
- New diagram creation is one undo entry and redo restores the same diagram ID.
- Ctrl+Z/Ctrl+Y work without opening Edit; operation status is not overwritten on idle frames.
- MCP can create an empty project at a supplied path and create/activate all four supported diagram kinds without adding tools.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello project
cargo test -p umbrello diagram
cargo test -p umbrello file
cargo test -p umbrello qa
```

### S3 — Diagram-aware tools and model browser

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/tool_palette.rs`
- `apps/umbrello/src/tree.rs`
- `apps/umbrello/src/property_editor.rs`
- `apps/umbrello/src/qa/control.rs`
- `apps/umbrello/src/qa/protocol.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S2.

Add the central compatibility predicate, enforce it at every native/MCP entry point, make the left side scrollable, turn model entries into selectable targets, and add compatible existing elements to the active diagram through history. Keep loaded incompatible content visible.

**Acceptance criteria:**

- Native palette and MCP expose the same enabled state for every tool/diagram pair in the matrix.
- Mouse, keyboard, direct app helper, and MCP attempts cannot bypass compatibility checks.
- A UseCase cannot be newly placed in a Class diagram; Class cannot be newly placed in a Use Case diagram.
- Clicking an Elements entry selects it and populates Properties even when it is absent from the active diagram.
- A compatible selected element absent from the active diagram can be added once; duplicate or incompatible additions are disabled/rejected without history mutation.
- Diagram and element lists remain reachable with large models via scrolling.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello compatibility
cargo test -p umbrello browser
cargo test -p umbrello qa
```

### S4 — Relationship interaction and gesture correctness

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/property_editor.rs`
- `apps/umbrello/src/qa/control.rs`
- `apps/umbrello/src/qa/protocol.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S1, S3.

Implement shared edge path geometry, nearest-segment hit testing, selection highlighting, relationship inspector drafts and Apply/Revert, semantic edge/property MCP targets, persistent documentation editing, classifier-only flags, and one-command node drag gestures.

**Acceptance criteria:**

- Clicking within a zoom-independent screen tolerance of the nearest edge selects its relationship; clicking a node takes precedence.
- Selected edges have a visible highlight and remain selected across redraw; background/Escape clears selection.
- Relationship fields apply atomically through `UpdateRelationship`; undo/redo updates canvas, browser, inspector, and MCP state.
- Empty roles/multiplicities become `None`; endpoint IDs and original XMI IDs remain unchanged.
- `ui_inspect` exposes active-diagram `edge:<relationship-id>` targets and relationship property targets; MCP can select, edit, sync, and screenshot the result.
- Documentation text survives multiple frames before commit.
- Abstract/static controls appear only for classifier elements.
- One node drag gesture creates exactly one undoable movement, including at non-100% zoom.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello edge_selection
cargo test -p umbrello relationship
cargo test -p umbrello property
cargo test -p umbrello drag
cargo test -p umbrello qa
```

### S5 — Durable documentation and closure

**Owned files:**

- `AGENTS.md`
- `docs/designs/milestone_24_usability_foundations.md`
- `docs/reviews/milestone_24_final_review.md`

**Dependencies:** S1–S4 integrated, full validation passed, final review approved.

The architect updates `AGENTS.md` with durable capabilities, limitations, source locations, and verified commands. The final review record captures evidence and residual risk. No commit is requested unless the user later asks for commits.

## Integrated Acceptance Criteria

1. A new project has an XMI path before authoring starts; cancellation/failure never destroys the prior model.
2. Users can create and immediately use multiple Class, Use Case, Component, and Deployment diagrams.
3. New authoring cannot place diagram-inappropriate elements or edges through mouse, keyboard, helper, or MCP paths.
4. Existing elements are clickable in the browser and reusable across compatible diagrams.
5. Associations and other relationships are selectable, highlighted, semantically editable, undoable, persisted, and MCP-operable.
6. Property editing, keyboard history, drag history, status text, and left-panel scrolling no longer contain the audited interaction defects.
7. Existing XMI loading/writing, viewport behavior, original IDs, and deterministic ordering remain intact.

## Integration and Review Gates

- **G1 after S1:** architect inspects command failure atomicity and runs targeted core validation.
- **G2 after S2–S3:** independent reviewer checks project/diagram lifecycle, compatibility policy consistency, and browser reuse before edge work proceeds.
- **G3 after S4:** architect runs targeted app tests and the complete workspace checklist.
- **G4 final:** independent reviewer inspects the integrated diff and exercises the actual native application through MCP: create a project path, create at least Class and Use Case diagrams, verify invalid/valid tools, create/reuse browser elements, create/select/edit an association, undo/redo, synchronize, and capture a screenshot.

Final required commands:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

If native dialogs cannot run in the review environment, project creation must still be exercised through the MCP path-based helper and the reviewer must report the exact native-dialog blocker.

## Risks and Mitigations

- **Selection overlap:** node hit testing takes precedence; edge selection chooses the nearest segment under a fixed screen-space tolerance.
- **Policy drift:** one pure compatibility predicate feeds palette, shortcuts, helpers, and MCP target enablement; matrix tests enumerate all combinations.
- **History fragmentation:** diagram creation, relationship Apply, existing-node placement, and complete drag gestures each map to one command.
- **Data loss:** destructive flows depend on explicit save outcomes, not on a void menu callback.
- **Loaded legacy content:** restrictions are prospective only; loaded unsupported/cross-kind views are not rewritten or dropped.
- **Scope growth:** ambiguous deletion, resize, tabs, and advanced diagram editors remain explicit follow-up work rather than partial implementations.

## Closure

Implemented and independently approved on 2026-07-29. The final integrated workspace passed formatting, Clippy with all targets/features and warnings denied, and the complete workspace test suite. Final native-application MCP QA created an XMI project, authored and switched Class and Use Case diagrams, verified tool restrictions, reused a browser element across diagrams, created/selected/edited a relationship, exercised undo/redo and synchronization, and captured a PNG screenshot. See `docs/reviews/milestone_24_final_review.md`.
