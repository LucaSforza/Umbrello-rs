# Milestone 23 G4 Final Independent Review

**Scope:** `0cf9eef^..e58eefb`, plus the uncommitted M23 plan and `AGENTS.md`,
covering S1–S4 and the surrounding command, property, XMI, and MCP paths.

## Findings

### MAJOR — XMI round trips discard Component/Node/Artifact common metadata

**Evidence: Direct contract violation.** The plan's C++ compatibility contract
states that all three types preserve common “documentation [and] flags”
(`docs/designs/milestone_23_component_node_artifact.md:35`), and the S2
acceptance criterion requires semantic read/write/read preservation of common
metadata (`:157`).  `build_base` unconditionally constructs
`documentation: String::new()` and `is_static: false`
(`crates/uml-io/src/xmi/reader.rs:900-903`).  The new Component and Artifact
writers emit self-closing tags containing `isAbstract` but no `isStatic` or
documentation representation (`crates/uml-io/src/xmi/writer.rs:315-339` and
`:342-372`); Node delegates to the same incomplete simple-element writer
(`:280`, `:289-313`).  Thus loading a Component, Node, or Artifact whose
`ElementBase.documentation` or `is_static` is populated and saving it loses
that user data.  The focused round-trip test only asserts IDs and new scalar
fields (`:1279-1352`), so it does not expose the loss.

Smallest acceptable correction: extend the established XMI common-metadata
path (including a suitable documentation/comment representation) so these
three variants preserve `documentation` and `is_static` on read/write/read;
add a synthetic round-trip regression that populates both fields for each
variant.  Do not solve this by adding C++-inheritance-style containment.

## Validation observed

- Reviewed the complete listed commit range, current plan, and `AGENTS.md`.
  `ObjectType::is_container()` still returns true for Component and Artifact
  (`crates/uml-core/src/types.rs:144-146`), while `ModelElement` treats them
  as non-package/non-classifier (`elements.rs:896-920`).  This is the explicit
  planned high-level-capability/repository-containment distinction, accurately
  documented in the plan and AGENTS; it is not a selection or containment
  defect.
- `git diff --check` passed. The reviewed range has no manifest/dependency or
  C++ changes and no added production `unsafe`.
- Independently passed `cargo fmt --all --check` (only existing stable-rustfmt
  unstable-option warnings), `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, and `cargo test --workspace`.
  Observed accounting is 350: core 168+8+9+2+4, IO 64+1, app 92+1, doctest 1.
- Focused component/node/artifact core, XMI, QA, and MCP-stdio tests also
  passed.

## Native MCP QA

Using `uvx --from 'mcp>=1.0'` with a conforming `ClientSession` and inherited
`DISPLAY=:0`/`WAYLAND_DISPLAY=wayland-0`, against
`target/debug/umbrello --mcp-stdio`:

- Initialize negotiated `2025-11-25`; `tools/list` returned exactly
  `ui_click`, `ui_drag`, `ui_inspect`, `ui_screenshot`, `ui_select`,
  `ui_set_text`, and `ui_sync`.
- Created and selected a diagram, then selected/clicked `tool.component`,
  `tool.node`, and `tool.artifact` on distinct canvas positions. Inspection
  returned durable node targets for `Component_1`, `Node_1`, and `Artifact_1`.
- Renamed the component, moved the node, then undid rename/move/artifact
  creation: Artifact target was absent after the third undo and restored by
  redo, confirming atomic creation history behavior.
- `ui_sync` reached state/rendered revision 33. `ui_screenshot` returned a
  valid PNG signature, `image/png`, 404,612 base64 characters, and metadata
  `1741x1306` at revision 33. This proves capture of the synchronized native
  frame containing the three durable nodes; the conforming client session has
  no image-display surface, so visual distinguishability is supported by the
  exercised native rendering dispatch and its smoke coverage rather than a
  manual pixel inspection. The conforming client parsed server stdout
  exclusively as MCP traffic; closing stdin terminated the child cleanly.

## Residual risks

The synthetic XMI coverage remains appropriate because the checked-in corpus
has no examples of these model/widget tags. Platform-specific visual metrics
remain end-to-end QA risk, not a reason to change the rendering design.

## Verdict

CHANGES REQUIRED

---

## Re-review addendum — S2-F1 and S2-F2

**Scope:** complete integrated range `0cf9eef^..2d8c1b9`, plus current
`AGENTS.md`, M23 plan, and this review report. The only production changes
after the prior review are the two persistence fixes in
`crates/uml-io/src/xmi/{reader,writer}.rs`.

### Findings

No blocking, major, or minor findings remain.

The former major is resolved. `build_base` now reads both legacy
`documentation` and C++ `comment`, and deterministically evaluates static
spellings in the required order: `static`, `ownerScope`, `isStatic`, then
`scope` (`reader.rs:896-907`). The shared simple-element writer emits C++
`comment` and `ownerScope="classifier"` (`writer.rs:343-358`) and is used by
Component, Node, Artifact, Actor, and UseCase. Therefore the repair covers
the original three types and does not regress Actor/UseCase's common simple
metadata path. The focused semantic round trip now populates and checks
documentation and static state for all three M23 variants; the parser test
covers defaults and contradictory static attributes.

### Validation observed

- Independently passed `cargo fmt --all --check` (only existing stable-rustfmt
  warnings), `cargo clippy --workspace --all-targets --all-features -- -D
  warnings`, and `cargo test --workspace`.
- Focused `common_metadata`, Actor round-trip, UseCase round-trip, and M23
  Component/Node/Artifact XMI round-trip tests passed.
- `git diff --check e58eefb..2d8c1b9` passed. The fixes add no dependencies,
  unsafe code, GUI/MCP changes, or C++ changes.
- `AGENTS.md` is accurate: it records 351 passing tests with core
  168+8+9+2+4, IO 65+1, app 92+1, and one doctest; it also retains the explicit
  Component/Artifact containment limitation.

### MCP disposition

Native MCP QA was not rerun. This is justified because S2-F1/S2-F2 alter only
the pure XMI reader/writer; the binary app, tool palette, canvas/rendering,
commands, QA bridge, MCP adapter, and their dependencies are byte-for-byte
outside the two commits. The prior conforming-client session already verified
the seven-tool surface, creation, durable targets, rename/move, atomic
undo/redo, synchronization, PNG capture, protocol-only stdout, and EOF
shutdown. Full workspace app and MCP tests pass again in this re-review.

### Residual risks

The corpus still lacks real Component/Node/Artifact examples, so synthetic
compatibility tests remain the appropriate evidence. Platform-specific visual
metrics remain ordinary end-to-end QA risk.

## Final verdict

APPROVED
