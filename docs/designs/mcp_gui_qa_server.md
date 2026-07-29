# MCP GUI QA Server

**Status:** Design v1  
**Scope:** opt-in automation and visual QA for `apps/umbrello`  
**Transport:** MCP over stdio, implemented in Rust with `rmcp`

## 1. Goal

Add an opt-in MCP server mode to the native Umbrello-RS application so an AI QA client can inspect the current UI, select a stable semantic target, operate that target, wait for rendering, and capture the current application window as a PNG. The server is a debugging and automated-QA surface, not an alternative end-user or domain API.

The server must be implemented in Rust with the official `rmcp` crate. `eframe::run_native` remains on the process main thread; MCP and Tokio run on a background thread and communicate with `UmbrelloApp` through bounded requests processed on the UI thread.

## 2. Current Behavior

- `apps/umbrello` is a single native `eframe` binary with no automation endpoint.
- Interaction is immediate-mode and spread across `menu.rs`, `tree.rs`, `tool_palette.rs`, `canvas.rs`, and `property_editor.rs`.
- Model mutations generally use `History` commands, but UI responses call application logic inline and expose no stable automation identities.
- The active diagram is stored as a vector index, while durable protocol identity must use `DiagramId`.
- egui/eframe 0.31.1 supports asynchronous viewport screenshots: `ViewportCommand::Screenshot(UserData)` yields `Event::Screenshot { user_data, image, .. }` in a later frame.
- The workspace declares Rust 1.85, while current `rmcp` 3.0.0 requires Rust 1.88. The installed compiler is Rust 1.92.
- The working tree was clean at investigation time.

## 3. Scope

### 3.1 MCP tools

Expose exactly these generic tools:

1. `ui_inspect` — return readiness, frame/state revisions, active tool/diagram, viewport zoom/pan, selected model element, status, and currently operable semantic targets.
2. `ui_select` — select one semantic QA target by its exact stable target ID. This changes the automation cursor only; selecting a canvas node in the application occurs when `ui_click` is invoked.
3. `ui_click` — activate the selected target. For the canvas target it accepts a required logical `(x, y)` position; other targets use their semantic action.
4. `ui_set_text` — replace and commit text for the selected editable target, initially element name and documentation.
5. `ui_drag` — drag the selected node to a logical model point, drag it to another node target while an edge tool is active, or pan the selected canvas by a screen-space delta while Select is active. This exercises the same action methods used by canvas interactions.
6. `ui_sync` — wait until at least one UI frame has rendered after a requested state revision, with cancellation and timeout.
7. `ui_screenshot` — capture the current native viewport and return an MCP image content block containing PNG data plus textual dimensions/revision metadata.

The intentionally small tool set represents UI operations, not individual Umbrello features. New visible actions extend the semantic target/action mapping rather than adding new MCP tools.

For `ui_drag`, the selected QA target is always the operation subject/source. A destination node ID is a necessary drag operand and is not an alternate source selector or selection bypass. Coordinate movement and node-to-node edge creation therefore remain expressible without replacing the automation cursor.

### 3.2 Initial semantic targets

Stable target IDs are ASCII protocol identifiers and never emoji labels or screen coordinates:

- `history.undo`, `history.redo`
- `file.new`, `file.save`, `app.quit` when the action can execute without opening an uncontrolled native picker or prompt
- `tool.select`, `tool.class`, `tool.interface`, `tool.enum`, `tool.datatype`, `tool.package`, `tool.actor`, `tool.use_case`
- `tool.generalization`, `tool.realization`, `tool.association`, `tool.aggregation`, `tool.composition`, `tool.dependency`
- `diagram.new_class` when visible
- `diagram:<DiagramId>` for each diagram
- `canvas` when a diagram is active
- `viewport.zoom_in`, `viewport.zoom_out`, `viewport.fit`, and `viewport.reset` when a diagram is active
- `node:<UmlId>` for each visible node in the active diagram
- `property.name`, `property.documentation`, `property.visibility.<value>`, `property.abstract`, `property.static` when an element is selected

Each inspected target reports kind, label, enabled/selected state, and any relevant model/diagram ID. Target lookup is exact. Stale or unavailable targets return structured tool errors rather than falling back to a similarly named control.

### 3.3 Shared application actions

Extract small `UmbrelloApp` action methods for target activation, tool choice, diagram activation, selection, undo/redo, command-backed property edits, node movement, and edge creation. Existing egui response handlers and the QA dispatcher must call the same methods. MCP code must not mutate `UmlModel` directly.

`ui_click`, `ui_set_text`, and `ui_drag` execute on the eframe UI thread. Successful mutations increment `state_revision`, mark dirty where current UI semantics require it, request repaint, and return structured state. Failures propagate rather than being discarded.

## 4. Explicit Non-goals

- No network listener, HTTP transport, authentication system, or remote exposure.
- No MCP code or GUI dependency in `uml-core`, `uml-io`, or `uml-codegen`.
- No OS-global mouse/keyboard injection and no dependency such as `enigo`.
- No tool per model element type, menu command, or property.
- No arbitrary filesystem read/write tool. A startup XMI path may still be supplied to the normal CLI. Native Open/Save As dialogs are not automated in v1.
- No native-dialog screenshot or control guarantee; screenshot scope is the eframe viewport.
- No pixel-identical cross-platform snapshot assertion.
- No unrelated correction of existing UI defects unless required to share an action safely with MCP.
- No C++ source changes.

## 5. Architecture and Invariants

### 5.1 Process and thread ownership

```text
MCP client
   | JSON-RPC over stdin/stdout
   v
rmcp + Tokio runtime (background OS thread)
   | bounded std::sync::mpsc request queue
   | tokio oneshot reply per request
   v
UmbrelloApp::update (eframe main/UI thread)
   | shared application actions
   | egui repaint/screenshot commands
   v
native viewport
```

- `UmbrelloApp` and `UmlModel` never move behind `Arc<Mutex<_>>` and are never accessed by the MCP thread.
- A cloned `egui::Context` may be retained only for thread-safe `request_repaint()`/close signaling.
- The request queue is bounded. Queue-full, disconnected, not-ready, cancellation, and timeout conditions are explicit errors.
- The UI thread never blocks waiting for MCP.
- stdout is reserved exclusively for MCP framing in `--mcp-stdio` mode. Diagnostics use stderr.
- Closing MCP stdin closes the GUI; closing the GUI cancels and joins the MCP service with a finite timeout.

### 5.2 Revisions and synchronization

- `ui_frame` increments once per `UmbrelloApp::update`.
- `state_revision` increments after each successful semantic state change.
- `rendered_revision` records the state revision represented by a completed UI pass.
- `ui_sync(after_revision)` resolves only after a later completed pass has rendered at least that revision.
- Every operation response includes current revision metadata. Timeouts default to a bounded value and honor MCP cancellation.

### 5.3 Screenshot lifecycle

1. MCP allocates a correlation ID and queues a screenshot request.
2. The UI thread stores the pending reply and sends `ViewportCommand::Screenshot(UserData::new(id))`.
3. A later update consumes only the matching `Event::Screenshot`.
4. The RGBA `ColorImage` is encoded as PNG without filesystem intermediates.
5. MCP returns `ContentBlock::image(base64_png, "image/png")` plus text metadata.

Concurrent requests are correlated by ID. Missing events, cancellation, GUI shutdown, and timeout complete the request with an error. PNG encoding uses a direct `image` dependency with PNG-only features.

### 5.4 MCP and Rust versions

- Pin `rmcp = "=3.0.0"` with only server/macros/schemars/stdio features needed by this server.
- Raise workspace `rust-version` from 1.85 to 1.88, the minimum supported by rmcp 3.0.0. Keep the project edition at 2021.
- Pin no transitive dependencies manually unless Cargo resolution proves it necessary.

## 6. Data Model and Error Handling

QA protocol types are serde/schemars-compatible and remain in the application crate:

- `UiTarget { id, kind, label, enabled, selected, element_id, diagram_id }`
- `UiSnapshot { ready, ui_frame, state_revision, rendered_revision, active_tool, active_diagram, zoom_percent, pan_x, pan_y, selected_element, status, targets }`; viewport fields are null without an active diagram.
- typed parameter structs for select, click, text, drag, sync, and screenshot tools
- `QaError` variants for not ready, queue full, unavailable/stale target, wrong target kind, invalid coordinates/value, command failure, timeout, cancellation, screenshot failure, and shutdown

No panic/unwrap is permitted in new production paths. Tool execution failures use MCP tool-error results; protocol/transport failures use rmcp service errors.

## 7. Persistence, Commands, UI, and Compatibility Effects

- `uml-core` and XMI formats are unchanged.
- User-visible and MCP-triggered model mutations continue through existing commands/history.
- `file.save` is enabled only when a current path already exists. `file.new` is disabled through MCP while dirty because v1 does not automate the native unsaved-changes dialog. This avoids destructive hidden policy.
- `ui_set_text` commits complete values deterministically; it does not emulate IME keystrokes.
- `ui_drag` uses logical egui/model coordinates for node movement and edge creation. When `canvas` is selected under Select, `(x, y)` is a requested screen-space pan delta in pixels, applied directly to the transient active-diagram pan; it is not an absolute pointer destination and does not require a prior pointer origin.
- Viewport actions are UI-only: they increment `state_revision`, request repaint, and never dirty the model or create history entries. `viewport.fit` requires a rendered canvas rectangle and returns `NotReady` before one exists.
- Normal application startup and behavior remain unchanged unless `--mcp-stdio` is supplied.
- MCP server metadata identifies the QA/debug purpose and reports tool support only; no resources or prompts are required.

## 8. Ordered Subtasks

### S1 — UI-thread QA control foundation

**Owned files:**

- `apps/umbrello/src/qa/mod.rs` (new)
- `apps/umbrello/src/qa/protocol.rs` (new)
- `apps/umbrello/src/qa/bridge.rs` (new)
- `apps/umbrello/src/qa/control.rs` (new)
- `apps/umbrello/src/qa/screenshot.rs` (new)
- `apps/umbrello/src/app.rs`
- `apps/umbrello/src/canvas.rs`
- `apps/umbrello/src/menu.rs`
- `apps/umbrello/src/property_editor.rs`
- `apps/umbrello/src/tool_palette.rs`
- `apps/umbrello/src/tree.rs`
- `apps/umbrello/src/tests.rs`

**Dependencies:** none.

Implement the bounded bridge, semantic targets/actions, revisions, sync barriers, screenshot correlation/PNG encoding, shared UI action methods, and protocol-neutral tests. Preserve normal no-MCP construction with an optional QA receiver/handle. Do not add rmcp transport code yet.

### S2 — rmcp adapter and opt-in process lifecycle

**Owned files:**

- `Cargo.toml`
- `Cargo.lock`
- `apps/umbrello/Cargo.toml`
- `apps/umbrello/src/main.rs`
- `apps/umbrello/src/qa/mod.rs`
- `apps/umbrello/src/qa/bridge.rs` only for exposing the existing ticket cancellation handle to async rmcp code
- `apps/umbrello/src/qa/mcp.rs` (new)
- `apps/umbrello/tests/mcp_stdio.rs` (new, if a display-independent transport test is practical)

**Dependencies:** S1 complete and validated.

Add pinned rmcp 3.0.0, minimum required direct dependencies/features, `--mcp-stdio`, the stdio server tools, cancellation/timeouts, main-thread GUI/background-runtime lifecycle, and transport-level tests. Do not expose a network transport.

The integrated S2 gate, rather than implementation-coupled tests of private frame-pump containers, verifies observable screenshot ordering: mutate, synchronize to the returned revision, capture, and confirm that image metadata is at least that rendered revision. Unit tests continue to cover PNG validity, bridge cancellation, and shutdown; native viewport timing is an integration concern.

### S1F2 — Review-derived atomicity and lifecycle corrections

**Owned files:**

- `crates/uml-core/src/undo/commands.rs`
- `crates/uml-core/src/undo/mod.rs` only if a re-export is required
- all S1-owned application and QA files
- `Cargo.lock` and `apps/umbrello/Cargo.toml` only as already changed by S1

**Dependencies:** S1 and S1F1 integrated; G1-R1 findings accepted.

This corrective subtask may add one concrete core command that atomically creates a model element and its diagram node. This narrow scope extension is required because preserving model, diagram, history, dirty state, and redo state on placement failure cannot be implemented correctly by manually undoing two separately recorded history entries. The command remains generic, semantic/diagram-data only, GUI-independent, ID-based, and contains no MCP concept. It replaces the existing two-history-entry placement behavior for both visible UI and QA.

The same subtask removes unreleased operation target IDs, adds a cancellation-capable bridge ticket for S2, defers screenshots until the requested revision has rendered, completes cancelled requests structurally, restores normal Save-to-Save-As fallback, and adds the lifecycle/frame/correlation regression coverage required by G1-R1.

## 9. Acceptance Criteria

1. `cargo run -p umbrello -- --mcp-stdio [optional.xmi]` runs a valid rmcp stdio server while displaying the normal Umbrello native window.
2. Without `--mcp-stdio`, startup and GUI behavior remain unchanged and stdout is not reserved.
3. MCP initialization and `tools/list` expose exactly the seven tools in Section 3.1.
4. `ui_inspect` returns deterministic stable IDs and current semantic/UI revision state.
5. A client can select a tool target, click it, select the canvas, click at a point, synchronize, inspect the created node, and see it in a screenshot.
6. A client can select a node, click it, select `property.name`, set text, synchronize, inspect the new name, and undo it using `history.undo`.
7. A client can create an edge by selecting an edge tool and using `ui_drag` from one node target to another.
8. A client can move a node with `ui_drag` to logical coordinates.
9. Screenshots are valid PNG MCP image content, correspond to the current viewport, and include dimensions and revision metadata.
10. Requests are bounded, run only on the UI thread, request repaint, honor cancellation/timeouts, and fail cleanly during startup/shutdown.
11. No new `unsafe`, no GUI/MCP leakage into core crates, no arbitrary filesystem tool, and no stdout diagnostics in MCP mode.
12. Existing tests remain passing and new public/behavioral contracts have meaningful tests.

## 10. Validation Commands

Targeted after S1:

```sh
cargo test -p umbrello qa
cargo test -p umbrello screenshot
cargo test -p umbrello property_editor
cargo clippy -p umbrello --all-targets --all-features -- -D warnings
```

Targeted after S2:

```sh
cargo test -p umbrello mcp
cargo test -p umbrello qa
cargo clippy -p umbrello --all-targets --all-features -- -D warnings
```

Integrated gate:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Display-dependent smoke test, when Xvfb or a real display is available:

```sh
xvfb-run -a cargo test -p umbrello --test mcp_stdio -- --ignored --nocapture
```

The current environment has no `xvfb-run`; inability to run this optional native-window smoke test must be reported explicitly rather than represented as a pass.

## 11. Integration and Review Gates

- **G1 (after S1):** architect inspection of owned diff and targeted tests; independent reviewer checks UI-thread ownership, command routing, target semantics, revisions, and screenshot lifecycle.
- **G2 (after S2):** architect inspection of dependency/MSRV changes, rmcp tool schemas, stdio purity, cancellation, startup/shutdown, and transport tests.
- **G3 (final):** full workspace formatting, clippy, tests, durable `AGENTS.md` update, final independent integrated review over all changed files.
- Any blocking or major finding becomes a new implementer fix subtask with explicit ownership and repeated validation/review.

## 12. Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Background server races or deadlocks the GUI | UI-owned app state, bounded queue, oneshot replies, no shared mutex around `UmbrelloApp` |
| Immediate-mode targets become stale | exact stable IDs, availability rebuilt from current app state, revision metadata, structured stale-target errors |
| Operation acknowledged before it is visible | `ui_sync` rendered-revision barrier; screenshot itself is a later-frame barrier |
| Screenshot requests complete out of order | correlation ID in egui `UserData` and pending map |
| stdio corruption | stdout exclusively for rmcp; all diagnostics to stderr |
| MCP host exits while GUI remains | EOF cancels service and sends viewport close; GUI close cancels service with bounded join |
| Native dialogs block automation | do not automate Open/Save As/dirty prompts in v1; use startup path and guarded direct actions |
| Semantic actions bypass visible behavior | existing egui handlers and MCP dispatcher are refactored to call the same action methods; inspected targets reflect actual current availability |
| Cross-platform screenshot differences | validate PNG structure/dimensions and use multimodal review, not byte-exact snapshots |
