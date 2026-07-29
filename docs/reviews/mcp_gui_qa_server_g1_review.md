# G1 Review — MCP GUI QA Server / S1

## Scope reviewed

Reviewed the working-tree diff against `HEAD`, including the untracked `apps/umbrello/src/qa/` implementation, for S1 in `docs/designs/mcp_gui_qa_server.md`. The untracked design document itself is architect-owned context and was not reviewed as implementation. No out-of-scope production changes were found; `Cargo.lock` reflects the two new direct application dependencies.

## Findings

### MAJOR — Requests can operate arbitrary IDs instead of the selected QA target

- **References:** `apps/umbrello/src/qa/protocol.rs:36-48`; `apps/umbrello/src/qa/control.rs:233-250`, `264-405`
- **Requirement violated:** Sections 3.1, 3.2, 6, and acceptance criteria 4 and 6-8 require `ui_select` to establish the automation cursor and `ui_click`/`ui_set_text`/`ui_drag` to operate that selected target. Exact target lookup must not become an alternate direct-control API.
- **Impact / reproduction:** After `ui_select("tool.class")`, a client can submit `Click { target_id: "history.undo", .. }`; after selecting one node it can drag or edit another by naming it in `Drag`/`SetText`. `selected_qa_target` is written only by `qa_select` and only consulted for selected rendering of diagram/node targets. It is omitted from `UiSnapshot`; property target `selected` values instead represent model flags, while tool values represent application state. Thus inspect cannot report the automation cursor and select-then-operate semantics do not exist.
- **Smallest acceptable correction / regression test:** Remove operation target IDs (and use the selected cursor) or reject an operation whose supplied target differs from `selected_qa_target`; add the selected QA target to `UiSnapshot` and define `UiTarget.selected` consistently. Test the specified select-tool/canvas/create, select-node/click/property/set-text/undo, node move, and edge-drag sequences, including a stale cursor and mismatched/direct target rejection.

### MAJOR — Command and file-save failures are acknowledged as success

- **References:** `apps/umbrello/src/tool_palette.rs:191-208`, `214-235`; `apps/umbrello/src/app.rs:114-128`; `apps/umbrello/src/qa/control.rs:326-330`, `413-422`, `284-289`; `apps/umbrello/src/menu.rs:151-170`
- **Requirement violated:** Sections 5.3 and 7 and acceptance criteria 6-8 and 10 require command-backed mutations and failures propagated to the QA caller.
- **Impact / reproduction:** `place_element` and `place_edge` call the compatibility `execute_command`, which discards `History::execute` errors, then return `Ok(())`. A failed `AddNodeToDiagram` can therefore leave the preceding element creation applied while QA reports success; a failed `CreateEdge` also reports success. `qa_click("file.save")` calls a void method that catches I/O errors, opens a native error dialog, and returns `Ok(())`. This is both an uncontrolled dialog in the automation path and a false successful operation.
- **Smallest acceptable correction / regression test:** Make the shared placement and save actions return `Result`, execute through `execute_command_result`, and propagate errors through QA. Preserve the normal UI dialog as a UI-only presentation layer around that result. Ensure multi-command placement has defined rollback/atomic behavior if its second command fails. Add injected-invalid-diagram/edge and unwritable-save-path tests that assert `QaError`, unchanged/consistent model and history, and no QA success response.

### MAJOR — UI and QA do not consistently share action routing, revisions, or repaint scheduling

- **References:** `apps/umbrello/src/tree.rs:12-27`; `apps/umbrello/src/qa/control.rs:496-530`; `apps/umbrello/src/canvas.rs:176-188`; `apps/umbrello/src/app.rs:142-179`, `308-383`, `386-389`
- **Requirement violated:** Sections 3.3 and 5.2 and acceptance criteria 4, 5, 7, 8, and 10 require the same actions for egui and QA, a state revision after successful semantic changes, repaint, and a rendered-revision barrier.
- **Impact / reproduction:** The visible New Class Diagram handler duplicates construction rather than calling `new_class_diagram`; its classifier set differs from the QA action. Canvas moving still creates a command and calls error-discarding `execute_command` directly. Keyboard tool selection mutates fields directly, bypassing `choose_tool` and revision updates. Conversely, QA actions in `qa_dispatch` do not call `ctx.request_repaint()`. After a QA creation/move/edit on an idle window, no subsequent pass is guaranteed, so `rendered_revision` remains behind and `ui_sync` can wait until the caller times out despite a successful operation.
- **Smallest acceptable correction / regression test:** Route tree, canvas, menu/keyboard, and QA handlers through result-returning shared actions; call `request_repaint()` for every successful QA semantic action before replying; and increment revisions exactly once for those actions. Add a frame-pump test proving that QA mutation makes `state_revision` advance, requests/causes a subsequent rendered pass, and releases `ui_sync`; test that visible and QA New Class Diagram produce the same diagram contents.

### MAJOR — Screenshot requests have no missing-event timeout or shutdown completion, and required PNG/lifecycle tests are absent

- **References:** `apps/umbrello/src/app.rs:62-70`, `142-179`, `181-208`; `apps/umbrello/src/qa/bridge.rs:28-49`; `apps/umbrello/src/qa/screenshot.rs:14-41`; `apps/umbrello/src/tests.rs:20-81`
- **Requirement violated:** Sections 5.1 and 5.3 and acceptance criteria 9, 10, and 12 require correlated screenshot completion for missing events, cancellation, timeout, and GUI shutdown, as well as valid-PNG tests.
- **Impact / reproduction:** A screenshot request whose event is never delivered remains indefinitely in `pending_screenshots`; no deadline/cancellation state is stored. Dropping the application simply drops reply senders, yielding a receiver-disconnect rather than the required structured `Shutdown` result, and pending sync replies receive the same treatment. `submit_timeout` only abandons the caller-side receiver and does not remove these pending entries. The only `qa_png` test checks eight signature bytes and the encoder-reported dimensions; it does not decode the bytes, test correlation/out-of-order events, metadata revisions, missing event handling, or shutdown. The required `cargo test -p umbrello screenshot` selects zero tests.
- **Smallest acceptable correction / regression test:** Track request deadline/cancellation and correlation state, expire each pending screenshot/sync with an explicit `QaError`, and implement app/service shutdown draining pending replies with `QaError::Shutdown`. Decode generated PNG bytes with the image decoder and assert dimensions; add tests for two correlation IDs, unmatched event preservation, timeout/missing event, cancellation/shutdown, and revision metadata.

## Validation observed

- `cargo test -p umbrello qa` — passed (3 tests).
- `cargo test -p umbrello screenshot` — passed with **0 tests selected**; this is a validation gap, not evidence for screenshot behavior.
- `cargo test -p umbrello property_editor` — passed (2 tests).
- `cargo fmt --all --check` — passed (with existing stable-rust warnings for unsupported rustfmt settings).
- `cargo clippy -p umbrello --all-targets --all-features -- -D warnings` — passed.
- `cargo test --workspace` — passed.

## Residual risks

The bridge correctly keeps `UmbrelloApp`/`UmlModel` on the UI thread and uses a bounded `sync_channel`; no new unsafe code was observed. However, `process_qa` drains with an unbounded `while try_recv` loop, so a continuously replenished producer can starve rendering despite the bounded queue. Address this while adding the frame/repaint tests (for example, use a per-frame request budget).

Verdict: CHANGES REQUIRED
