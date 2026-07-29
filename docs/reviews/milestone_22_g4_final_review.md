# Milestone 22 G4 Final Independent Review

**Scope:** integrated uncommitted `git diff HEAD` for S1, S2, and S3. The
assignment did not provide a report filename; this conventional review path is
used. Review included the listed code and documentation files, untracked
`viewport.rs` and `mcp_stdio.rs`, and surrounding reader, writer, canvas, and
QA control flow.

## Findings

### MAJOR — Root-level writer output loses every self-closing diagram widget on read

**Evidence: Proven by code path.** `XmiWriter` writes diagrams under the
root-level `<XMI.extensions>` container (not the in-model `<XMI.extension>`)
at `crates/uml-io/src/xmi/writer.rs:98-106`, and writes every widget as an
empty XML tag at `crates/uml-io/src/xmi/writer.rs:723-747`. The new Start-event
logic correctly enters root-level extensions and sets `inside_content = false`
still returns before extension dispatch whenever `inside_content` is false at
`crates/uml-io/src/xmi/reader.rs:439-450`. Consequently a writer-produced
`<classwidget .../>` (and every other empty widget) is ignored. A write/read
round trip retains the diagram metadata but drops its nodes, violating the
plan's semantic XMI round-trip and persistence requirements.

Smallest acceptable correction: make Empty events use the same
`inside_xmi_extension` admission/dispatch contract as Start events, including
root-level `<XMI.extensions>`. Add a regression that creates a diagram with at
least one node, writes it, reads it, and asserts node identity/bounds and zoom
are preserved.

### MINOR — Finite JSON pan values can corrupt transient pan with infinity

`apps/umbrello/src/qa/control.rs:495-506` accepts any finite `f64` and casts
it to `f32`. A valid JSON value such as `1e308` passes `is_finite()` but casts
to `f32::INFINITY`, after which the canvas transform and snapshot hold a
non-finite pan. This is a reachable malformed-client path that can make
rendering and response serialization unusable.

Smallest acceptable correction: validate the converted `f32` values are finite
before mutating pan (or retain pan as `f64` with bounded validation), and add a
QA-dispatch regression for an out-of-`f32` finite input.

## Validation observed

- `git diff --check HEAD` passed. Scope inspection found no unsafe additions,
  dependency/manifest changes, or C++ changes in the reviewed worktree diff.
- `cargo fmt --all --check` passed, with only the repository's existing stable
  rustfmt unstable-option warnings.
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
  passed.
- Focused tests passed: diagram zoom reader defaults/clamping, zoom writer
  round-trip, all five `viewport` tests, QA canvas pan validation test, and
  the `mcp_stdio` CLI test. The existing reader widget test passed, but it
  covers in-model `<XMI.extension>` rather than the writer's root-level
  `<XMI.extensions>` output and therefore does not cover the major finding.

## Runtime MCP attempt

Attempted the required native launch twice using:

```sh
cargo run -p umbrello -- --mcp-stdio tests/data/xmi/test-BVW.xmi
```

The process remained running with `DISPLAY=:0` and `WAYLAND_DISPLAY=wayland-0`.
An MCP initialize request was sent over the rmcp stdio newline framing, followed
by `tools/list` and `ui_inspect`, but no response arrived within 30 seconds.
On stdin EOF the process exited cleanly (status 0) and stderr reported
`MCP server stopped: connection closed: initialize request`. An earlier
Content-Length framing attempt likewise received no response. `xvfb-run` and
`xwininfo` are unavailable in this environment, so the availability of a
usable native viewport could not be established. Thus viewport target
activation, canvas pan, `ui_sync`, screenshot metadata/image observation, and
protocol-only stdout could not be completed; no screenshot was captured.

## Documentation and residual risk

`AGENTS.md` accurately describes the intended persisted-zoom/transient-pan
design and the stated 334-test accounting, but its claim that XMI zoom and
diagram behavior are complete is premature while writer-produced diagram
nodes are lost on read. Native MCP visual QA remains unverified because the
available display environment did not produce an MCP response.

## Verdict

CHANGES REQUIRED

---

## Re-review addendum — S1-F1, S3-F1, S3-F2

**Scope:** current integrated uncommitted `git diff HEAD` for S1/S2/S3 and
the three assigned fixes. This re-review revisited the complete integrated
diff, the reader/writer and QA-control fixes, runtime behavior, durable
documentation, and the prior findings.

### Findings

No blocking, major, or minor findings remain.

The former major is fixed: both Start and Empty reader paths now admit events
when `inside_xmi_extension` is true (`crates/uml-io/src/xmi/reader.rs:299-303`
and `435-449`), including the writer's root-level `<XMI.extensions>` output.
`write_diagram_zoom_and_round_trip_non_default_value` now writes a
self-closing node and verifies its restored element identity, bounds, and
rounded zoom (`crates/uml-io/src/xmi/writer.rs:1178-1214`). The former pan
overflow is fixed before mutation/revision: conversion to `f32` and accumulated
pan are both tested for finiteness (`apps/umbrello/src/qa/control.rs:495-526`),
with regressions for conversion and accumulation overflow
(`apps/umbrello/src/tests.rs:688-723`).

### Validation observed

- `git diff --check HEAD` passed; no unsafe, dependency/manifest, or C++
  changes are present in the reviewed diff.
- `cargo test -p uml-io write_diagram_zoom_and_round_trip_non_default_value
  -- --nocapture` passed.
- `cargo test -p umbrello qa_canvas_drag_pans_by_screen_delta_and_rejects_non_finite_values
  -- --nocapture` passed.
- `cargo test -p umbrello --test mcp_stdio -- --nocapture` passed.
- Independent `cargo test --workspace` passed: 334 tests by the documented
  suite accounting.
- The architect's reported format and all-features clippy commands were also
  independently rerun in the prior pass and passed (stable rustfmt emitted only
  the existing unsupported-option warnings).

### Native MCP runtime QA

The earlier client issue was diagnosed from rmcp 3.0.0 itself:
`AsyncRwTransport` reads one JSON-RPC message per newline
(`rmcp .../transport/async_rw.rs:125-160`), so Content-Length framing is not
valid for this adapter. A standards-conforming Python MCP `ClientSession` was
run through `uvx --from 'mcp>=1.0'`; its child-launch defaults initially strip
display variables, which reproduced winit's explicit no-display error. Passing
the existing environment (`DISPLAY` and `WAYLAND_DISPLAY`) allowed native QA to
complete.

Against `target/debug/umbrello --mcp-stdio tests/data/xmi/test-BVW.xmi`, the
client initialized at MCP `2025-11-25`, listed exactly seven tools, inspected
and activated the diagram target, then selected/clicked `viewport.zoom_in`,
`viewport.fit`, and `viewport.reset`. Observed state was respectively 25% zoom,
fit at the bounded 10% with pan `(-117.1875, 88.225006...)`, and reset at 100%
with zero pan. Selecting `canvas` and `ui_drag(31, -17)` produced pan
`(31, -17)` and revision 10. `ui_sync(after_revision=10)` returned rendered
revision 10. `ui_screenshot` returned a non-error PNG image block (419,800
base64 characters) and metadata `1741x1306`, state/rendered revision 10.
The conforming client parsed stdout exclusively as MCP messages; stdin EOF
cleanly terminated the child session. Thus the screenshot represents the
native viewport at the synchronized viewport revision.

### Documentation and residual risk

`AGENTS.md` accurately states the 334-test accounting and durable M22 facts.
The normal residual risk is platform-specific rendering variance, which is
appropriately outside this change's automated assertions; native Wayland MCP
QA completed in this environment.

## Final verdict

APPROVED
