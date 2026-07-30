# G4 Final Integrated Review — Property Authoring and Edge Interaction Fixes

**Plan:** `docs/designs/ui_property_and_edge_interaction_fixes.md`
**Gate / scope:** G4; S1–S9 and accepted fixes S2F1–S2F4, S3F1–S3F2, S4F1–S4F2, S5F1, S7F1–S7F2, S8F1
**Range reviewed:** `617ee45..0a97493`

## Findings

No blocking, major, or minor findings remain.

## Resolved findings

- **MCP attribute visibility dispatch:** `qa_classifier_attr_dispatch` now splits the numeric index on the first dot. The four visibility values, invalid index safety, static/delete, and adjacent operation visibility dispatch are covered by focused tests.
- **Parameter default-value data loss:** the writer emits non-empty `Parameter.default_value` as canonical `UML:Parameter@value`; the reader prefers `value` and accepts legacy `initialValue`. The fresh native project contains `value="seed"` at `integrated-final.xmi:13`.
- **Relationship duplication on reload:** newly written assocwidgets share their semantic relationship XMI ID. The reader resolves shared IDs first and deterministically handles old separate widget IDs. Fresh and old-format native reloads each exposed exactly four semantic relationships and four edge targets after activating the diagram.
- **Properties consuming the canvas:** the right panel is bounded to 200–400 logical pixels and vertically scrollable; compact parameter rows retain a usable canvas. Focused full-app layout tests cover 1024×768 and 1741×1306.

## Validation observed

- Architect-provided integrated validation reviewed: `git diff 617ee45..HEAD --check`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace` (464), and `cargo build -p umbrello` passed.
- Independently run:
  - `cargo test -p umbrello native_property_panel_click_preserves_selection` — passed.
  - `cargo test -p uml-io xmi` — 90 passed, including canonical/legacy parameter defaults, shared IDs, old separate IDs for all six kinds, parallel relationships, multi-diagram reuse, widget-only input, and repeated-save relationship-count stability.
  - `cargo fmt --all --check` — passed (only known stable-toolchain notices for unstable rustfmt options).
  - `cargo clippy --workspace --all-targets --all-features -- -D warnings` — passed.
  - `cargo build -p umbrello` — passed.
  - `git diff --check 617ee45..0a97493` — passed.
- The range changes only planned application/core-I/O/docs files. `uml-core` remains free of GUI/I/O dependencies and retains `#![forbid(unsafe_code)]`.

## Native MCP QA

Loaded the project-local `umbrello-mcp-qa` skill and exercised the actual `target/debug/umbrello --mcp-stdio` with `DISPLAY=:0`, using the versioned reusable `mcp_client.py` API. Target IDs were dynamically discovered; no disposable client file or repository production/test file was created.

The fresh single-process scenario verified exactly seven tools (`ui_inspect`, `ui_select`, `ui_click`, `ui_set_text`, `ui_drag`, `ui_sync`, `ui_screenshot`) and completed:

1. writable project, Class diagram, and five class nodes;
2. attribute `age: i32` with MCP-applied `private` visibility; operation `compute() -> bool`; parameter `input: String`, `out`, default `seed`; Apply, synchronization, and enabled Undo history;
3. Generalization, Realization, Composition, and Dependency to separate target nodes;
4. connected-node `gesture: true` drag, synchronized revision, Undo, and synchronized screenshot;
5. save/relaunch with five node targets, exactly four semantic relationship targets, and exactly four corresponding edge targets after diagram activation;
6. reload of the prior separate-ID file at `/tmp/opencode/umbrello-property-edge-qa/integrated/integrated-project.xmi`, again with exactly four semantic relationships and four edge targets.

Fresh artifacts under `/tmp/opencode/umbrello-property-edge-qa/integrated-final`:

- `transcript.json` — SHA-256 `466ec750ca4b5893fc85405e1310f9884b6a7a2ff33409fcf951edafeadaa90d`
- `relaunch-transcript.json` — SHA-256 `b7fb576c5be7a514cc5576b19e037eff0babe6b0b2343ce2d2da8d71fbc112e2`
- `old-format-reload.json` — SHA-256 `9d6d77022729b82cb35669532750b4cc001d7b9505941b0bd500e4810d4991b6`
- `integrated-final.xmi` — SHA-256 `47b3819e6a4cd25496c40e17606cda5e795b52f1d541223efb6cf2695f301d4c`
- `relationships.png` — SHA-256 `38b824f6413a6aa8ca024c0b54cad740a4440a73179a42396eb56c639a31adf7`
- `after-undo.png` — SHA-256 `b97a7de38631de9277a7de8c82652c4eabb789dee869370d60615f8c4d81c478`
- `relationships-fit.png` — SHA-256 `1c0476e2937c1a84dfdb2f4ce1fe062c584f1d6fabafdc13b3612c0725c313dd`
- `fit-transcript.json` — SHA-256 `0350790028f7aff003db76f364ce1e20282bd5ce81ffc920cac57d886b8af8b3`

Visual inspection of the synchronized fit screenshot confirms a usable populated Properties inspector (including `private`, `out`, and `seed`) beside the canvas. It also visibly distinguishes solid Generalization with hollow target triangle, dashed Realization with hollow target triangle, solid Composition with filled source diamond, and dashed Dependency with open target arrow. The post-Undo screenshot retains the populated inspector and canvas. The full-app pointer regression passed, and the live screenshot/state shows the selected classifier and retained draft, providing the strongest available evidence that Properties interaction does not clear semantic selection while true canvas-background clicks still do.

## AGENTS.md

`AGENTS.md:397-408` accurately records the 464-test count, canonical parameter value persistence with legacy fallback, shared/new and deterministic-old association-widget identity behavior, bounded scrollable Properties panel, and current implementation locations.

## Residual risks

Pixel-golden rendering remains intentionally deferred. The native scenario used Fit Diagram to bring all four target-end notations into one screenshot; this is normal viewport use rather than a correctness limitation.

APPROVED
