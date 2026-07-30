# XMI Direct-Edge Round-Trip Regression

**Status:** implemented; integrated validation passed; preclosure review approved; final review pending
**Method:** strict red-green-regression TDD

## Goal

Saving and reopening a newly authored Class diagram containing two classes and one direct association must preserve the same semantic elements, node geometry, and direct edge path. Reopening must not route the edge through the canvas origin or introduce a synthetic user-visible `Package` for the document-level `<UML:Model>` wrapper.

## Verified Current Behavior

`ViewEdge::new` creates a direct edge with an empty `waypoints` vector. `XmiWriter::write_assoc_widget` nevertheless emits `<startpoint startx="0" starty="0"/>` and `<endpoint endx="0" endy="0"/>`. `XmiReader::finalize_assocwidget` converts those serialization placeholders into two real waypoints. `clipped_edge_path_points` then renders source → origin → target, producing the extra lines shown after reload.

When no root package exists, `XmiWriter::write_model_wrapper` emits a document-level `<UML:Model name="UML Model">`. Reader dispatch currently maps every `Model` and `Package` tag through `parse_package`, so this technical wrapper becomes an extra semantic `ModelElement::Package` after round-trip. Existing writer tests explicitly ignore this package-count mismatch and therefore conceal the regression.

## Scope

- Add a minimal in-memory save/reload regression matching the reported two-class, one-association project.
- Preserve an empty waypoint list for a direct edge across XMI round-trip.
- Keep explicit non-empty edge line points round-trippable.
- Treat only the outer document-level `<UML:Model>` container as transparent while retaining nested `UML:Model`/`UML:Package` containment semantics.
- Reconcile tests that currently accept a synthetic package-count mismatch.

## Explicit Non-goals

- New edge routing algorithms, waypoint editing, or canvas rendering changes.
- XMI 2.x or foreign dialect support.
- Changing relationship/widget shared-ID behavior.
- Changing `UmlId`, `ElementBase.original_xmi_id`, diagram geometry, or command history.
- Hiding arbitrary packages named `UML Model`; only the document container is transparent.
- Adding dependencies or modifying C++ reference sources.

## Architectural Decisions and Invariants

1. Empty `ViewEdge.waypoints` means a direct edge. Persistence must preserve that distinction rather than invent geometry.
2. The writer omits `<linepath>` when no explicit waypoints exist. The reader continues accepting legacy linepaths, including genuine coordinates.
3. The outer `<UML:Model>` is the XMI document container, not a semantic `Package` in `UmlModel`. Nested `UML:Model` elements remain package-like namespaces because `uml-core` has no separate UML Model type.
4. Reader containment state must represent a transparent root frame explicitly so nested model/package start/end events remain balanced. Structural children of the transparent root are unparented; children of nested namespaces retain ID-based package membership.
5. Existing collision-safe XMI ID allocation, deterministic ordering, shared association identity, and strict duplicate-ID rejection remain unchanged.

## Data and Control Flow

```text
direct ViewEdge { waypoints: [] }
  -> writer emits assocwidget without linepath
  -> reader finalizes ViewEdge { waypoints: [] }
  -> renderer clips one direct source-to-target segment

XMI.content -> outer UML:Model (transparent containment frame)
  -> classes/relationships remain wrapper-level semantic elements
  -> nested UML:Model or UML:Package creates Package + ID-based containment
```

## Ordered Subtasks

### S1 — Red save/reload regressions

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`
- `crates/uml-io/src/xmi/reader.rs`

**Dependencies:** none.

Add tests before production changes. The primary test constructs two Classes, one Association, a Class diagram with two nodes, and a direct `ViewEdge` with no waypoints; it writes and reads XMI and asserts exact semantic element-kind counts, node bounds/endpoints, one edge, and empty restored waypoints. Add focused reader coverage proving the outer document Model is transparent while a nested Model or Package remains semantic and retains containment. Run the focused tests and record the expected failures. Commit only red tests.

**Acceptance criteria:**

- The direct-edge regression fails pre-fix because restored waypoints contain origin placeholders.
- The semantic-count regression fails pre-fix because a synthetic Package appears.
- Nested namespace coverage passes or precisely identifies any containment behavior that the green fix must preserve.
- Tests inspect parsed structures, not brittle complete XML strings or screenshots.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io direct_edge_round_trip_preserves_empty_waypoints -- --nocapture
cargo test -p uml-io document_model_wrapper_is_transparent -- --nocapture
cargo test -p uml-io nested_model_remains_semantic_package -- --nocapture
```

### S2 — Green persistence fix

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`
- `crates/uml-io/src/xmi/reader.rs`

**Dependencies:** S1.

Make empty direct-edge geometry serialize without synthetic `(0,0)` line points. Introduce balanced transparent-root containment handling in the reader and remove test assumptions that tolerate a generated wrapper Package. Preserve explicit non-empty point parsing and nested package/model containment. Commit the production fix and any necessary test refinements.

**Acceptance criteria:**

- The reported two-class association round-trip restores exactly two Classes, one Relationship, two diagram nodes, one edge, unchanged bounds/endpoints, and no waypoints.
- No synthetic wrapper Package is added.
- Explicit non-empty line points still parse and round-trip according to existing behavior.
- Nested packages/models still populate `Package.children` and `parent_index` deterministically.
- No unrelated source or test files change.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io direct_edge_round_trip_preserves_empty_waypoints
cargo test -p uml-io document_model_wrapper_is_transparent
cargo test -p uml-io nested_model_remains_semantic_package
cargo test -p uml-io xmi
```

### S3 — Integrated validation and durable record

**Owned files:**

- `AGENTS.md`
- `docs/designs/xmi_direct_edge_round_trip.md`

**Dependencies:** S2F2, integrated validation, and independent review.

Record the corrected direct-edge and transparent-wrapper round-trip guarantees, exact source locations, validation evidence, review verdict, and any residual compatibility limitation. Commit the closure documentation after final review evidence is available.

**Validation:**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

### S2F1 — Preserve semantic root Packages

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** S2 inspection.

Add a red regression before further production changes proving that a model whose first or only Package is a user-authored semantic package preserves that Package, its `original_xmi_id`, and child containment after write/read/resolve. The S2 implementation made the outer `UML:Model` transparent, but the writer still co-opts the first Package as that wrapper via `find_root_model_id`; the test must expose the resulting Package loss. Commit only the red test.

**Acceptance criteria:**

- The new test fails on S2 because the semantic root Package is absent or containment is lost.
- The test asserts exact Package count/name/original ID, child membership, and `parents_of`.
- No production code changes are included.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io semantic_root_package_survives_transparent_wrapper -- --nocapture
```

### S2F2 — Separate wrapper from semantic Packages

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`

**Dependencies:** S2F1.

Make the writer always allocate its document-level `UML:Model` as a synthetic transparent container and emit every semantic Package as a real nested `UML:Package`, including a package named `UML Model`. Select deterministic wrapper-level structural roots from canonical package containment so descendants are emitted exactly once by their semantic package. Do not change reader behavior, IDs of semantic elements, relationship placement, or nested multi-parent determinism.

**Acceptance criteria:**

- Semantic root Packages survive round-trip with name, unique `original_xmi_id`, children, and `parent_index` intact.
- The two-class no-package scenario still reloads without a synthetic Package.
- Nested and multiply-parented package tests retain deterministic single-definition emission.
- Generated wrapper IDs remain document-wide collision-safe.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io semantic_root_package_survives_transparent_wrapper
cargo test -p uml-io direct_edge_round_trip_preserves_empty_waypoints
cargo test -p uml-io nested_package_definitions_are_emitted_once
cargo test -p uml-io package_containment
cargo test -p uml-io xmi
```

### S2F3 — Reconcile compatibility documentation and legacy-point coverage

**Owned files:**

- `crates/uml-io/src/xmi/writer.rs`
- `crates/uml-io/src/xmi/reader.rs`

**Dependencies:** S2F2; G3-preclosure reviewer findings.

Update stale test names/comments that still describe a semantic root Package as the document wrapper or claim a removed root-wins policy. Add a focused reader assertion proving an explicitly serialized legacy `(0,0)` linepoint remains a real waypoint; do not treat explicit zero geometry as the omitted direct-edge sentinel. No production behavior changes.

**Acceptance criteria:**

- Writer comments and containment test naming describe synthetic wrapper plus deterministic first-parent ownership.
- A legacy assocwidget fixture with an explicit `(0,0)` startpoint asserts that exact restored waypoint.
- Direct-edge empty-waypoint regression remains green.

**Validation:**

```sh
cargo fmt --all --check
cargo test -p uml-io legacy_explicit_origin_linepoint_is_preserved
cargo test -p uml-io direct_edge_round_trip_preserves_empty_waypoints
cargo test -p uml-io xmi
```

## Integration and Review Gates

- **G1 — Red evidence:** inspect the S1 commit and confirm failures correspond to the two verified causes, with no production changes.
- **G2 — Green inspection:** inspect the S2 diff, surrounding reader state machine, generated XMI, focused tests, and commit ownership. Confirm the transparent wrapper does not consume a semantic Package; otherwise complete S2F1/S2F2 before proceeding.
- **G3 — Integrated validation:** run formatting, all-feature workspace clippy, and all workspace tests.
- **G4 — Independent final review:** reviewer checks S1–S3 against this plan, the integrated commit range, regression quality, reader compatibility, and validation evidence. Any blocking or major finding becomes a new implementer fix subtask followed by repeated validation/review.

## Error Handling and Compatibility Effects

No public error type changes are expected. Malformed or duplicate defining IDs remain errors. The reader remains lenient toward missing linepaths and continues to accept explicit legacy linepoints. The transparent-root rule is structural, not name- or generated-ID-based, so legitimate nested packages named `UML Model` are preserved.
