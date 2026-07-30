# G3-R1 Final Integrated Re-review — Save/Reload and Native Drag Regressions

**Plan:** `docs/designs/save_reload_and_native_drag_regressions.md`  
**Gate:** G3-R1  
**Scope:** Prior S1–S5/S4F1/S4F2 plus S2F1/S2F2; `afab75f..HEAD`; current `AGENTS.md` and plan date reconciliations.

## Prior finding resolution

The prior **MAJOR** finding was nested package double emission: a valid `UML Model -> P -> C` model wrote `C` both recursively and at wrapper level, producing duplicate defining XMI IDs.

**Resolved.** `write_model_wrapper()` now selects only unparented and root-canonically-owned structural elements for wrapper emission (`crates/uml-io/src/xmi/writer.rs:202-243`). `write_package()` emits only children for which that package is the same deterministic canonical owner (`writer.rs:452-510`). Relationships remain wrapper-level (`writer.rs:245-251`). The reader's `insert_with_containment()` attaches only structural `ModelElement` values to the active package, and both Start and Empty dispatch paths use it; feature values remain on their classifier parsing path (`crates/uml-io/src/xmi/reader.rs:317-420`, `467-557`, `insert_with_containment` at `reader.rs:1389-1398`).

The regression test now verifies one nested Class definition, strict read/resolve, `P.children` membership, and `parents_of(C)` (`writer.rs:1783-1869`). Additional tests cover direct root ownership, multi-level containment, root-wins and non-root multi-parent canonical ownership, and changed insertion order (`writer.rs:1872-2054`). No findings remain.

## Validation observed

- Inspected commits `1ea5bc0`, `01a7868`, and `302360c`, the complete integrated range, current documentation reconciliations, and surrounding reader/writer behavior. No new `unsafe`, unrelated production changes, reader duplicate-ID relaxation, or relationship regression was found.
- Passed independently:
  - `cargo fmt --all --check` (only existing stable-rustfmt warnings for nightly-only options)
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  - `cargo test -p uml-io nested_package_definitions_are_emitted_once`
  - `cargo test -p uml-io package_containment` (4 tests)
  - `cargo test -p uml-io xmi` (73 tests)
  - `cargo test --workspace` (396 tests)
- Re-exercised the actual `target/debug/umbrello --mcp-stdio` eframe application on `DISPLAY=:0` using the seven generic MCP tools and read-only `../test/test-COG.xmi`:
  - Activated a diagram and inspected four node targets.
  - Selected a node and called `ui_drag {x: 2000.0, y: 1500.0, gesture: true}`; Undo became enabled at revision 4.
  - `ui_sync {after_revision: 4}` returned rendered/state revision `4/4`.
  - Selected `history.undo` and clicked it; status was `Undo` and Redo became enabled.
  - `ui_screenshot` returned a PNG image block (474,864 base64 bytes) and metadata `{"height":1306,"rendered_revision":6,"state_revision":6,"width":1741}`.

## Residual risk / environment limitation

The mandatory running-app project-create/save/reopen sequence remains intentionally unperformed: reviewer policy permits writing only this report, while `file.new` would create a temporary XMI file. The focused persistence suite covers save/reload and the live non-writing MCP run covers `gesture=true`, synchronization, undo, and native screenshot. Attribute/operation authoring remains deferred as documented.

APPROVED
