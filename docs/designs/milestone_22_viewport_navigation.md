# Milestone 22 — Viewport Navigation and Zoom Persistence

**Status:** implementation plan
**Reference implementation:** `../umbrello/umbrello/umlview.cpp`, `umlapp.cpp`, `umlscene.cpp`
**Scope:** diagram zoom, pan, fit controls, XMI zoom compatibility, and MCP QA exposure

## Goal

Make large UML diagrams navigable by adding the viewport behavior provided by C++ Umbrello: bounded zoom, cursor-anchored wheel zoom, middle-button panning, fit/reset controls, and persisted per-diagram zoom. The existing generic MCP QA server must expose and exercise these visible actions without adding feature-specific MCP tools.

## Current Behavior

- `apps/umbrello/src/canvas.rs` maps `ViewNode.bounds` directly to screen coordinates. Drawing, hit testing, node movement, edge previews, and creation all assume a 1:1 untransformed canvas.
- `UmbrelloApp` has no pan state or common model/screen coordinate conversion.
- `Diagram` has no zoom metadata. `uml-io` ignores the XMI `zoom` attribute and the writer always emits `zoom="100"`.
- The menu has no View controls or zoom status.
- MCP snapshots expose no viewport state or viewport actions; `ui_drag` only accepts node targets.
- C++ `UMLView::setZoom` clamps to 10–500%, `wheelEvent` applies a 1.15 factor around the cursor, and middle-button drag pans. C++ XMI saves and restores diagram zoom.

## Scope

1. Persist a bounded per-diagram zoom percentage in `uml-core` with a serde-compatible 100% default.
2. Read and write the Umbrello UML 1.2 XMI `diagram@zoom` attribute.
3. Add transient per-diagram pan state to the GUI and centralize model-to-screen and screen-to-model transforms.
4. Apply the transform consistently to nodes, edges, previews, creation, hit testing, and drag movement.
5. Add wheel zoom anchored at the pointer, middle-button pan, and visible View actions for Fit, 100%, Zoom Out, and Zoom In.
6. Expose viewport state and actions through semantic MCP targets; allow `ui_drag` on `canvas` to pan.
7. Verify the actual native application through the stdio MCP server, synchronization, and a screenshot.

## Non-goals

- No node resize handles, scrollbars, minimap, pinch gesture, tabs, grid, snap-to-grid, or multi-selection.
- No persistence of pan offset; C++ XMI compatibility requires zoom, not the transient viewport origin.
- No command/history entries and no dirty flag for viewport-only changes.
- No new MCP tools, dependencies, network transport, or C++ modifications.
- No change to semantic model coordinates when zooming or panning.
- No speculative general rendering framework.

## Architectural Decisions and Invariants

### Persisted zoom, transient pan

`Diagram` owns `zoom_percent: f64` because C++ stores zoom per diagram in XMI. It defaults to 100 and is clamped to the inclusive range 10–500. The field uses a serde default so older serialized Rust data remains readable. Pan remains application state keyed by `DiagramId`; switching diagrams restores each transient pan for the current process but saving does not serialize it.

`uml-core` remains GUI-independent. It stores only numeric diagram metadata and contains no egui types.

### Coordinate transform

The app uses one affine contract for the active canvas:

```text
screen = canvas_origin + pan + model * scale
model  = (screen - canvas_origin - pan) / scale
scale  = zoom_percent / 100
```

All visual geometry and pointer operations use these helpers. Node model bounds and edge waypoints remain unchanged by viewport operations. Node drag screen deltas are converted back to model deltas. Creation positions are converted from screen to model coordinates.

### Zoom behavior

- Valid range: 10–500%, matching `UMLView::setZoom`.
- Wheel: multiply/divide by 1.15 and adjust pan so the model point under the cursor remains fixed, matching `UMLView::wheelEvent`.
- Visible Zoom In/Out actions use the C++ application control increment of ±5 percentage points.
- Reset sets 100% and zero pan.
- Fit computes the union of visible node bounds, applies a small viewport margin, chooses the smaller width/height scale, clamps it, and centers the result. Empty diagrams reset to 100% and zero pan.
- Zoom changes update the active `Diagram.zoom_percent` for persistence but are view-state changes: they do not enter command history or set the document dirty flag. Saving later for another reason preserves the current zoom.

### MCP compatibility

The seven existing generic tools remain unchanged. `ui_inspect` adds numeric viewport fields and the targets:

- `viewport.zoom_in`
- `viewport.zoom_out`
- `viewport.fit`
- `viewport.reset`

`canvas` remains the semantic canvas target. With an edge/node creation tool it retains existing behavior; with Select active, `ui_drag` from `canvas` to a logical screen position pans by the supplied delta contract documented in the QA protocol. MCP viewport actions use the same application methods as visible controls, increment `state_revision`, request repaint, and never alter history or dirty state.

## Data Model and Control Flow

1. Loading XMI creates each `Diagram` and applies parsed `zoom`, defaulting invalid/missing values to 100 and clamping through the diagram API.
2. Activating/rendering a diagram reads its persisted zoom and transient pan.
3. UI or MCP action calls a shared viewport method.
4. The method updates zoom and/or pan, bumps semantic QA state, and requests repaint through the caller.
5. Rendering obtains the central-panel rectangle and transforms all diagram geometry.
6. Saving XMI writes the active numeric `Diagram.zoom_percent` instead of a hard-coded value.

## Persistence, Commands, UI, Compatibility, and Errors

- XMI output keeps existing canvas bounds behavior and changes only `zoom` to the diagram value rounded to a stable integer representation compatible with C++ Umbrello.
- Missing, malformed, non-finite, or out-of-range XMI zoom values resolve safely through the 100 default and 10–500 clamp; parsing the rest of the document must not fail.
- Viewport operations are not model commands, are not undoable, and do not dirty the file.
- Model mutations continue through commands. Transform logic only converts UI coordinates before existing commands are constructed.
- MCP rejects non-finite drag coordinates and wrong target kinds through existing structured `QaError` values.

## Ordered Subtasks

### S1 — Diagram zoom metadata and XMI round trip

**Owned files:**

- `crates/uml-core/src/diagram/mod.rs`
- `crates/uml-io/src/xmi/reader.rs`
- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** none.

Add the serde-compatible zoom field/API and focused unit tests. Parse, clamp, write, and semantically round-trip non-default zoom. Preserve existing diagram ordering and all other XMI behavior.

**Acceptance criteria:**

- New diagrams and old serde data default to 100%.
- Setting zoom clamps to 10–500% and rejects non-finite values by restoring/using a safe default.
- XMI missing/malformed zoom yields 100%; out-of-range values clamp.
- Read → write → read preserves a valid non-default zoom.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-core diagram
cargo test -p uml-io xmi
```

### S2 — Shared viewport transform and visible interaction

**Owned files:**

- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/menu.rs`
- `apps/umbrello/src/tests.rs`
- `apps/umbrello/src/viewport.rs` (new)

**Dependencies:** S1.

Implement transient pan keyed by diagram, shared coordinate conversion and viewport actions, transformed rendering/hit testing/interaction, wheel zoom, middle-button pan, fit/reset/in/out visible controls, and focused app tests. Keep all viewport-only operations outside command history and dirty tracking.

**Acceptance criteria:**

- Rendering, edge geometry, previews, hit tests, movement, and placement remain aligned at non-default zoom and pan.
- Wheel zoom preserves the model point under the cursor within floating-point tolerance.
- At 200%, a 20-pixel node drag changes model position by 10 units.
- Middle-button pan changes only pan state.
- Fit handles empty, negative-coordinate, and large diagrams.
- View controls show the active zoom percentage and are disabled without an active diagram.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello viewport
cargo test -p umbrello
```

### S3 — MCP viewport targets and automation

**Owned files:**

- `apps/umbrello/src/qa/control.rs`
- `apps/umbrello/src/qa/protocol.rs`
- `apps/umbrello/src/qa/mcp.rs`
- `apps/umbrello/src/tests.rs`
- `apps/umbrello/tests/mcp_stdio.rs`
- `docs/designs/mcp_gui_qa_server.md`

**Dependencies:** S2.

Add viewport snapshot fields and semantic action targets, route clicks to shared viewport methods, allow canvas drag panning under Select, update MCP schemas/descriptions only as necessary, and add protocol/transport tests. Do not add an eighth tool.

**Acceptance criteria:**

- `ui_inspect` reports zoom, pan, and enabled viewport targets.
- MCP zoom/fit/reset and canvas pan change viewport revisions but not dirty/history state.
- `ui_sync` observes a rendered viewport revision.
- Existing MCP interactions remain backward compatible.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p umbrello qa
cargo test -p umbrello --test mcp_stdio
cargo test -p umbrello
```

## Integrated Acceptance Criteria

1. A real C++ Umbrello XMI file with `zoom` loads at its stored bounded zoom and saves that zoom semantically.
2. A user can inspect a large/offset diagram using wheel zoom, middle pan, Fit, 100%, and ± controls.
3. Nodes and edges remain aligned; selection, dragging, edge creation, and element placement operate in model coordinates under the viewport transform.
4. Viewport changes do not create undo entries or dirty the document.
5. The existing MCP server can inspect, zoom, pan, synchronize, and capture the actual native application without protocol pollution on stdout.
6. No `unsafe`, new dependency, C++ change, or crate-boundary violation is introduced.

## Integrated Validation

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Runtime QA command:

```sh
cargo run -p umbrello -- --mcp-stdio tests/data/xmi/test-BVW.xmi
```

The final reviewer must use the running MCP server to call `ui_inspect`, select and activate viewport controls, pan the canvas, call `ui_sync`, and capture a screenshot. If the display environment blocks this, the reviewer must report the exact launch/display blocker and strongest attempted evidence.

## Integration and Review Gates

- **G1 (after S1):** architect inspects core/IO diff and targeted tests; no GUI dependency or XMI regression.
- **G2 (after S2):** architect inspects every transformed interaction path and targeted app tests.
- **G3 (after S3):** full workspace formatting, clippy, and tests pass.
- **G4 (final):** `AGENTS.md` is updated with durable M22 behavior and included in an independent reviewer scope. Reviewer must approve the integrated diff and exercise actual MCP runtime QA.

Any production defect found at a gate becomes a new fix subtask assigned to the implementer session that owns the affected subsystem, followed by repeated validation and review.
