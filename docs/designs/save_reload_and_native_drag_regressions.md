# Save/Reload and Native Drag Regressions

**Status:** implemented; integrated validation passed; final independent review approved
**Scope:** repair invalid XMI produced by Umbrello-RS and restore native mouse dragging of selected diagram nodes  
**Method:** strict red-green-regression TDD with independent final MCP/native-gesture QA

## Goal

An Umbrello-RS project containing a Class diagram and one Class must save to globally valid UML 1.2 XMI, reload without error or application failure, and preserve the diagram node. A user must be able to select that class and then move it with a primary-button press/move/release gesture; the gesture must create exactly one undoable `MoveNode` command.

## Verified Current Behavior

The supplied `/home/softdream/Programming/cazzate/umbrello-rs/prova.xmi` contains:

- `<UML:Model xmi.id="rs00000001">`;
- `<UML:Class xmi.id="rs00000001">`;
- a `classwidget` referring to the same class ID.

The semantic `UML:Model` and `UML:Class` duplicate causes `XmiReader::register_id()` to return `XmiParseError::DuplicateId("rs00000001")`. The reader is enforcing a valid invariant; the file produced by the writer is invalid.

`XmiWriter` pre-assigns IDs for model elements but its generated wrapper, diagram, association-widget, and feature IDs are allocated by an independent counter. Preserved `ElementBase.original_xmi_id` values do not reserve the generated namespace, so generated IDs can collide with preserved or previously generated IDs.

In `apps/umbrello/src/canvas.rs`, node rectangles register `Sense::click_and_drag()`. When an element is selected, a later full-canvas `ui.interact(..., Sense::click())` is registered for background deselection. This overlapping foreground interaction can claim pointer input above the selected node. Existing drag tests call `preview_node_position()` and `move_node_to()` directly and therefore do not reproduce native pointer routing.

The screenshot's “Not Responding” state occurs while the native error dialog is modal. No separate panic path has been established. Failed open already avoids replacing `self.model`; regression coverage must preserve this non-destructive behavior and ensure malformed input returns an error rather than panicking.

Attributes and operations already load, render inside class/interface compartments, and appear read-only in Properties. There is no attribute/operation authoring command or UI. That is a missing feature, not part of these bug fixes.

## Scope

1. Add a regression that saves and reloads the smallest user scenario: one Class diagram and one Class node.
2. Make all writer-assigned XMI IDs globally collision-free while preserving non-conflicting `original_xmi_id` values and deterministic output.
3. Add a frame-level native pointer regression covering select followed by primary-button drag and release.
4. Remove overlapping canvas hit regions so node input has precedence and background clicks still deselect.
5. Extend the existing generic MCP `ui_drag` path only as needed to exercise the same native pointer gesture semantics, rather than validating only a direct model-space command.
6. Verify failed loads remain non-destructive and panic-free.

## Explicit Non-goals

- Attribute, operation, or parameter authoring.
- Changes to UML semantic IDs (`UmlId`) or `ElementBase.original_xmi_id` preservation.
- Relaxing the reader to accept duplicate semantic XMI IDs.
- XMI 2.x, foreign dialects, byte-identical output, or unrelated persistence changes.
- Resize handles, snapping, routing, layout, or other canvas interactions.
- New external dependencies or changes to the seven generic MCP tool names.

## Architectural Decisions and Invariants

### XMI identity allocation

The writer owns one document-wide allocation policy. Before emission it reserves every non-empty preserved `original_xmi_id`. Generated IDs must probe until an unused value is found and reserve it immediately. The model wrapper and every synthetic ID use that same allocator. A preserved ID remains unchanged when unique; duplicate preserved IDs must produce a structured write error or be deterministically remapped according to the smallest implementation consistent with existing public error conventions. The writer must never emit duplicate defining `xmi.id` values.

Widget references may repeat a semantic element ID because they are references in Umbrello's extension format; they are not new semantic definitions. The regression must distinguish these references from defining IDs.

### Reader behavior

`XmiReader::register_id()` continues rejecting duplicate semantic definitions. The fix belongs in the writer. Loading malformed input returns `XmiParseError` and must not mutate the currently open application model.

### Native drag ownership

Node interactions take precedence over canvas background deselection. Background handling must not register a later full-canvas widget that overlaps nodes. A primary gesture uses transient preview state while held and commits one `MoveNode` on release. Selection, zoom conversion, dirty state, and undo/redo behavior remain command-driven.

### MCP QA fidelity

The MCP surface retains `ui_drag`. If its current node branch only calls `move_node_to()` directly, extend its arguments/protocol internally so the reviewer can request a native-equivalent press/move/release gesture through egui input. Direct semantic movement may remain for compatibility, but final QA must explicitly use the gesture mode, synchronize, inspect the changed node, undo it, and capture a screenshot.

## Data and Control Flow

```text
save -> reserve preserved element IDs
     -> allocate wrapper and synthetic IDs from one collision-aware set
     -> write XMI -> read XMI -> resolve classwidget to Class

pointer press on selected node -> node interaction owns pointer
pointer move                  -> transient model-space preview
pointer release               -> one MoveNode command -> dirty + history

malformed open -> parser error -> retain current model/path/history/dirty state
```

## Ordered Subtasks

### S1 — Red persistence regressions

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`
- `crates/uml-io/src/xmi/mod.rs`

**Dependencies:** none.

Add tests before production changes. Cover an in-memory one-Class/one-Class-diagram save/reload, a preserved ID colliding with the generated `rs...` namespace, deterministic uniqueness of defining IDs, and the supplied failure shape. Run the focused tests and record the expected pre-fix failure. Commit the red tests only.

**Acceptance criteria:**

- At least one new test fails against the current writer for the verified collision reason.
- The test asserts semantic reload, class node geometry, and absence of duplicate defining IDs rather than brittle full-document text.
- Existing valid `original_xmi_id` preservation remains asserted.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io save_reload_one_class_diagram -- --nocapture
cargo test -p uml-io generated_xmi_ids_do_not_collide -- --nocapture
```

### S2 — Green XMI allocator fix

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`
- `crates/uml-io/src/xmi/error.rs`
- `crates/uml-io/src/xmi/mod.rs`

**Dependencies:** S1.

Implement one document-wide collision-aware allocator and route model wrapper and synthetic IDs through it. Preserve unique originals and deterministic ordering. Add or refine malformed-load coverage if needed; do not weaken duplicate rejection in the reader.

**Acceptance criteria:**

- The exact one-class scenario saves and reloads with one semantic Class and one diagram node referring to it.
- No defining ID collides with a preserved or generated ID.
- Unique original XMI IDs survive semantic round trips.
- Malformed duplicate input returns an error without panic.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io save_reload_one_class_diagram
cargo test -p uml-io generated_xmi_ids_do_not_collide
cargo test -p uml-io xmi
```

### S2F1 — Emit nested package definitions exactly once

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** S2; G3 reviewer finding.

Select structural emission roots from package containment instead of treating every non-wrapper element as top-level. Direct children of a package represented by the `UML:Model` wrapper and unparented elements are emitted at wrapper level; descendants of nested packages are emitted recursively exactly once. If repository state permits multiple package parents, use a deterministic canonical owner or a structured error so no defining ID is emitted twice.

**Acceptance criteria:**

- `UML Model -> Package P -> Class C` writes one defining Class ID, reloads, and preserves `P -> C` membership.
- Insertion order and multiple-parent edge cases cannot produce duplicate defining IDs.
- Relationships remain emitted once and existing round trips remain valid.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io nested_package_definitions_are_emitted_once
cargo test -p uml-io xmi
```

### S2F2 — Restore nested containment and canonical multi-parent emission

**Owned files:**

- `crates/uml-io/src/xmi/reader.rs`
- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** S2F1 inspection.

Attach each parsed structural child to the current package/model parent after insertion so save/reload restores `Package.children` and `parent_index`. Complete canonical ownership in recursive package writing: a multiply-parented child is emitted only by its chosen canonical package, including when the root is one of its parents.

**Acceptance criteria:**

- Nested `P -> C` membership and `parents_of(C)` survive round trip.
- Package nesting survives for start/end and self-closing structural elements.
- Multiple package parents never cause duplicate defining IDs; canonical choice is deterministic and covered.
- Existing corpus and reference validation remain green.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io nested_package_definitions_are_emitted_once
cargo test -p uml-io package_containment
cargo test -p uml-io xmi
cargo test --workspace
```

### S3 — Red native-drag regressions

**Owned files:**

- `apps/umbrello/src/tests.rs`
- `apps/umbrello/src/qa/protocol.rs`
- `apps/umbrello/src/qa/mcp.rs`

**Dependencies:** none.

Add a frame-level egui test that first selects a node and then supplies realistic primary press, movement, and release events over the rendered class. It must fail before the canvas fix because the overlapping background interaction prevents the node drag. Add MCP schema/protocol test coverage for native-equivalent gesture mode if required for final QA. Commit the red tests only.

**Acceptance criteria:**

- The test exercises `UmbrelloApp::update`/`render_canvas`, not direct `move_node_to()` calls.
- It proves selected-node movement, one history entry, undo restoration, and behavior at non-100% zoom.
- Pre-fix failure is recorded and attributable to pointer ownership/hit testing.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello native_pointer_drag_selected_node -- --nocapture
cargo test -p umbrello drag -- --nocapture
```

### S4 — Green native drag and MCP gesture fix

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/tests.rs`
- `apps/umbrello/src/qa/control.rs`
- `apps/umbrello/src/qa/protocol.rs`
- `apps/umbrello/src/qa/mcp.rs`

**Dependencies:** S3.

Repair interaction ordering/geometry so background deselection never steals input from nodes. Keep one command per gesture and extend `ui_drag` only enough to drive native-equivalent input for final QA.

**Acceptance criteria:**

- A selected node moves after primary press/move/release and remains at the new position.
- Clicking true background still clears selection.
- One drag produces one undoable command; undo restores the original bounds.
- Zoom conversion remains correct.
- MCP can distinguish and invoke native-equivalent node gesture QA without adding a new generic tool.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello native_pointer_drag_selected_node
cargo test -p umbrello background_click
cargo test -p umbrello drag
cargo test -p umbrello qa
```

### S5 — Integrated validation and durable record

**Owned files:**

- `AGENTS.md`
- `.opencode/agents/architect.md`
- `docs/designs/save_reload_and_native_drag_regressions.md`

**Dependencies:** S2, S4 and integrated checks.

Record only the corrected persistence guarantee, native drag behavior, MCP QA capability, source locations, and verified commands. Attribute/operation authoring remains explicitly deferred. Update the architect instructions so future `AGENTS.md` closure edits are delegated through an exact owned-file assignment that enumerates every required factual change, followed by architect inspection and reconciliation; this minimizes repeated exploratory tool calls while preserving architect accountability.

**Validation:**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### S4F1 — Multi-frame cumulative drag correction

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/tests.rs`
- `apps/umbrello/src/qa/control.rs`

**Dependencies:** S4.

Architect inspection found that the initial GREEN implementation applies `pointer.delta()` from the latest frame to the original node position. Real mouse motion spans multiple frames, so this can commit only the final incremental delta instead of the cumulative displacement. Refactor begin/update/release into shared app helpers used by both native canvas and MCP gesture mode. Native update computes cumulative screen displacement from press origin (or equivalent stable gesture origin), converts it once through viewport scale, and commits the final preview exactly once.

**Acceptance criteria:**

- A drag with at least three separate movement frames commits the full displacement from press origin to final pointer position.
- Intermediate movement deltas are not repeatedly applied to either the original or preview position.
- Native canvas and MCP gesture mode call the same begin/update/release state-transition helpers.
- Release commits exactly one command; undo restores exact original bounds.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello native_pointer_drag_accumulates_multiple_move_frames
cargo test -p umbrello native_pointer_drag_selected_node
cargo test -p umbrello native_pointer_drag_converts_non_100_zoom
cargo test -p umbrello qa_gesture_mode_uses_shared_behavior_and_commits_once
```

### S4F2 — No-op click must not create movement history

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** S4F1.

A normal mouse click commonly spans separate press and release frames. The shared state machine may begin on press even when there is no movement; release must clear drag state without executing a same-position `MoveNode`.

**Acceptance criteria:**

- Separate press/release frames without pointer movement select the node but do not change bounds, dirty state, or history depth.
- `commit_node_drag` clears every drag field on both moved and no-op completion.
- Real moved gestures and MCP gesture mode retain one-command behavior.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello native_pointer_click_without_motion_creates_no_move
cargo test -p umbrello drag
```

## Integration and Review Gates

### G1 — Persistence gate

After S2, inspect the actual diff and commit, run the focused `uml-io` tests, and independently save/reload the minimal model. Reject any reader relaxation or loss of `original_xmi_id`.

### G2 — Native interaction gate

After S4, inspect frame-level tests and confirm they do not bypass egui input routing. Run focused app tests and verify one-command history behavior.

### G3 — Final independent review and running MCP QA

The reviewer receives S1–S5, all commits from the cycle, relevant files, and integrated validation evidence. The reviewer must launch the actual Umbrello application with `--mcp-stdio` and:

1. create a temporary XMI project, Class diagram, and Class;
2. save and reopen it, then inspect that the diagram and class node survived;
3. select the node and invoke `ui_drag` in native-gesture mode;
4. `ui_sync`, inspect changed visible/model state, undo, and verify restoration;
5. capture a synchronized screenshot;
6. report exact blockers if the native window cannot launch or be captured.

Blocking or major findings create fix subtasks resumed with the original implementer and require repeated validation and final review.

## Integrated Acceptance Criteria

1. Umbrello-RS cannot emit the duplicate semantic ID pattern found in `prova.xmi`.
2. A one-Class Class diagram survives save/reload with geometry and identity references intact.
3. A malformed duplicate-ID XMI returns an error non-destructively and without panic.
4. A selected class can be dragged by a real primary-button gesture and one undo restores it.
5. Final reviewer QA exercises the running app through MCP using native-equivalent drag, synchronization, and screenshot evidence.
6. Attribute/operation authoring is unchanged and explicitly remains the next separate product task.

## Implementation Outcome

**Commits (oldest to newest):** `da60d53`, `dc91de2`, `6dbc530`, `94cac43`, `99193ea`, `1b7c98a`, `1ea5bc0`, `01a7868`

**Integrated validation:** first pass on 2026-07-30 (391 tests). Rerun after S2F2 on 2026-07-30 with **396 tests**:
```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

**G3 review findings and fixes:**
- G3 found duplicate nested-package emission in output XMI
- S2F1 eliminated duplicate definitions by writing each structural child under its canonical package parent exactly once
- S2F2 restored Package.children and parent_index on read so round-trip preserves containment; deterministic canonical ownership prevents multi-parent duplicates
- G3-R1 approved the corrected integrated state after independent validation and renewed live MCP gesture/sync/undo/screenshot QA
- Follow-up project skill `.opencode/skills/umbrello-mcp-qa/` versions the reusable MCP client and read-only/persistence smoke workflows used for automated native QA
