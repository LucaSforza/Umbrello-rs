# Umbrello-RS: Comprehensive Risk Assessment

> **Project**: Rust rewrite of Umbrello UML Modeller  
> **Date**: 2026-06-23  
> **Status**: Pre-implementation risk analysis  
> **Scope**: All subsystems identified in the codebase analysis

---

## Table of Contents

1. [Risk Matrix Overview](#1-risk-matrix-overview)
2. [Risk Categories](#2-risk-categories)
   - [R01 — XMI Compatibility](#r01-xmi-compatibility-critical)
   - [R02 — Rendering Quality](#r02-rendering-quality-critical)
   - [R03 — Feature Parity](#r03-feature-parity-high)
   - [R04 — GUI Framework](#r04-gui-framework-high)
   - [R05 — Performance](#r05-performance-medium)
   - [R06 — Plugin Architecture](#r06-plugin-architecture-medium)
   - [R07 — Test Coverage](#r07-test-coverage-medium)
   - [R08 — Community and Momentum](#r08-community-and-momentum-medium)
   - [R09 — Complexity](#r09-complexity-medium)
   - [R10 — Knowledge Retention](#r10-knowledge-retention-medium)
   - [R11 — Dependency](#r11-dependency-low-medium)
   - [R12 — Maintenance During Transition](#r12-maintenance-during-transition-low)
3. [Top 5 Risk Summary](#3-top-5-risk-summary)
4. [Risk Tracking Recommendations](#4-risk-tracking-recommendations)
5. [Risk Acceptance Decisions](#5-risk-acceptance-decisions)
6. [Contingency Budget](#6-contingency-budget)
7. [Appendices](#7-appendices)

---

## 1. Risk Matrix Overview

### 1.1 Assessment Scale

**Likelihood** (probability the risk materializes):

| Level | Label          | Range      |
|-------|----------------|------------|
| 1     | Very Low       | 0–10%      |
| 2     | Low            | 10–30%     |
| 3     | Medium         | 30–60%     |
| 4     | High           | 60–80%     |
| 5     | Very High      | 80–100%    |

**Impact** (consequence if the risk occurs):

| Level | Label      | Description |
|-------|------------|-------------|
| 1     | Negligible | No noticeable effect on timeline or quality |
| 2     | Minor      | Minor delay (< 1 month) or cosmetic defects |
| 3     | Moderate   | Significant delay (1–3 months) or feature gaps |
| 4     | Major      | Major delay (3–6 months) or serious defects |
| 5     | Critical   | Project failure or unusable product |

**Risk Rating** = Likelihood × Impact:

| Likelihood ↓ \ Impact → | 1 Negligible | 2 Minor | 3 Moderate | 4 Major | 5 Critical |
|-------------------------|:------------:|:-------:|:----------:|:-------:|:----------:|
| 5 Very High (80–100%)   | Medium (5)   | High (10)  | **Critical (15)** | **Critical (20)** | **Critical (25)** |
| 4 High (60–80%)         | Low (4)      | Medium (8) | High (12)  | **Critical (16)** | **Critical (20)** |
| 3 Medium (30–60%)       | Low (3)      | Medium (6) | High (9)   | High (12)  | **Critical (15)** |
| 2 Low (10–30%)          | Very Low (2) | Low (4)    | Medium (6) | Medium (8) | High (10)  |
| 1 Very Low (0–10%)      | Very Low (1) | Very Low (2) | Low (3)  | Low (4)    | Medium (5) |

### 1.2 Risk Summary Table

| ID   | Risk Category              | Likelihood | Impact  | Rating   | Direction |
|------|----------------------------|:----------:|:-------:|:--------:|:---------:|
| R01  | XMI Compatibility          | Very High  | Critical| **Critical (25)** | ⬆ Stable |
| R02  | Rendering Quality          | High       | Critical| **Critical (20)** | ⬆ Increasing |
| R03  | Feature Parity             | High       | Major   | **Critical (16)** | ⬆ Increasing |
| R04  | GUI Framework              | High       | Major   | **Critical (16)** | ⬇ Decreasing |
| R05  | Performance                | Medium     | Major   | High (12) | ➡ Stable |
| R06  | Plugin Architecture        | Medium     | Major   | High (12) | ➡ Stable |
| R07  | Test Coverage              | Medium     | Major   | High (12) | ⬇ Decreasing |
| R08  | Community & Momentum       | Medium     | Major   | High (12) | ⬆ Increasing |
| R09  | Complexity                 | Medium     | Major   | High (12) | ➡ Stable |
| R10  | Knowledge Retention        | Medium     | Major   | High (12) | ⬆ Increasing |
| R11  | Dependency                 | Low        | Major   | Medium (8) | ⬇ Decreasing |
| R12  | Maintenance (dual)         | Low        | Moderate| Medium (6) | ⬆ Increasing |

### 1.3 Visual Risk Matrix

```
Impact →
  Critical  │    R01          R02
            │
  Major     │           R03, R04     R05, R06, R07
            │                        R08, R09, R10
  Moderate  │                             R12
            │
  Minor     │                    R11
            │
  Negligible│
            └────────────────────────────────────
                  Very    Low  Medium  High  Very
                  Low                          High
                              Likelihood →
```

---

## 2. Risk Categories

---

### R01 — XMI Compatibility (Critical)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Very High (90%)         |
| **Impact**      | Critical (5)            |
| **Rating**      | **25 of 25**            |
| **Direction**   | Stable — well understood, but inherently difficult |

#### Description

The C++ Umbrello has accumulated **20+ years** of XMI format knowledge embedded in its
serialization code. Every UML model class implements its own `saveToXMI()` / `loadFromXMI()`
methods — scattered across **50+ classes**. There is no formal XMI specification document for
the Umbrello dialect; the C++ source code *is* the specification.

XMI round-trip is the **most fundamental capability** of the application. If a user saves a
diagram in the C++ version and cannot open it in the Rust version (or vice versa), the entire
project loses credibility.

Key complexity factors:

- **Two XMI versions**: UML 1.2 (default, legacy) and UML 2.1 (optional). Different element
  names, attribute conventions (`xmi.id` vs `xmi:id`), and containment models (`UML:` vs `uml:`
  namespace prefixes, `packagedElement` vs direct children).
- **Foreign XMI dialects**: The C++ version can read XMI produced by NSUML, Unisys,
  Embarcadero, Rational Rose, and ArgoUML. Each has idiosyncratic element names, attribute
  positions, and extension mechanisms.
- **Mixed streaming vs DOM**: The C++ save path uses `QXmlStreamWriter` (streaming, efficient),
  but the load path uses `QDomDocument` (full DOM). Replicating DOM-based loading in Rust
  without matching edge-case behavior is non-trivial.
- **Forward references**: XMI uses ID-based cross-references (`xmi.id` / `xmi:idref`), with
  `resolveRef()` deferred post-processing. Objects can reference other objects not yet loaded.
- **Extension namespaces**: Diagram state is stored in `<XMI.extension>` elements with
  Umbrello-specific tags. Widget positions, sizes, colors, fonts — hundreds of attributes.
- **DTD validation**: Four DTD files must be matched on output and optionally checked on input.
  Exact output formatting (attribute order, whitespace) may affect interop.
- **Multiple archive formats**: `.xmi`, `.xmi.tgz`, `.xmi.tar.bz2`, `.zargo` (ArgoUML legacy).
  Each requires different I/O plumbing.
- **File version constants**: `XMI1_FILE_VERSION = "1.7.6"`, `XMI2_FILE_VERSION = "2.0.4"`.
  The Rust version must maintain these or handle version migration.

#### Likelihood Assessment

Near-certain. Any non-trivial XMI implementation will have incompatibilities with the C++
version on the first attempt. The C++ version itself has had bugs in XMI serialization that
were fixed over years of user reports.

#### Impact Assessment

**Critical**. If XMI round-trip is broken:
- Users cannot migrate their existing models to the Rust version.
- Users cannot collaborate across C++ and Rust versions during transition.
- The project loses its most important compatibility guarantee.
- Testing becomes impossible (test suite relies on XMI save/load).

#### Mitigation Strategies

1. **Golden file test suite**: Collect every available XMI file — from Umbrello's own test
   directory (`test/test-*.xmi`), from user reports, from foreign tools. Store them as golden
   reference files. The Rust XMI parser must load all of them without error and produce
   semantically equivalent model objects.

2. **Byte-for-byte round-trip testing**: After loading a C++-produced XMI file with the Rust
   implementation, serialize it back to XMI and compare the output byte-for-byte with the
   original (after normalizing formatting-only differences). Any difference is a bug.

3. **Property-based testing with `proptest`**: Generate random UML models in Rust, serialize
   to XMI, deserialize, and verify the in-memory model matches the original. This catches
   serialization asymmetry and field omissions.

4. **Fuzz testing the XMI parser**: Use `cargo-fuzz` to feed the Rust XMI parser with
   malformed, truncated, and edge-case XMI input. Ensure it never panics and produces
   meaningful error messages.

5. **Bidirectional compatibility CI**: On every CI run, produce XMI from both C++ and Rust
   versions and verify that each can read the other's output. Run this automatically.

6. **Comprehensive error handling**: Use `Result<T, XmiError>` throughout the XMI parser
   with structured error types that include file, line, and element context.

7. **XMI schema documentation**: Produce explicit documentation of the Umbrello XMI dialect
   as a reference. This is also a risk reduction for future contributors.

8. **Archive format support early**: Implement `.xmi.tgz`, `.xmi.tar.bz2`, and `.zargo`
   reading in the first persistence milestone. Don't defer archive support.

#### Contingency Plan

If XMI compatibility cannot be achieved within the allocated time budget:

- **Phase 1 fallback**: Use a JSON intermediate format. The C++ version exports JSON,
  the Rust version reads it. This provides a narrow compatibility bridge while the XMI
  implementation is refined.
- **Phase 2 fallback**: Ship the Rust version with a bundled C++ XMI converter binary.
  Users run the converter once to migrate their files.
- **Emergency fallback**: Accept that the Rust version starts with a breaking XMI format
  change (documented, version-bumped) and provide an explicit migration tool.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| Golden file test failures > 10% | Red | Stop and fix parser before proceeding |
| Round-trip byte differences found | Yellow | Investigate and fix before release |
| Fuzz test crashes | Red | Blocking issue, fix immediately |
| Foreign XMI load failures | Yellow | Log as known limitation, fix per priority |
| Missing XMI 2.1 support gap | Yellow | Add to sprint backlog |

---

### R02 — Rendering Quality (Critical)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | High (75%)              |
| **Impact**      | Critical (5)            |
| **Rating**      | **20 of 25**            |
| **Direction**   | Increasing — Rust GPU rendering ecosystem is rapidly evolving but immature |

#### Description

The C++ version renders all diagrams using `QPainter` — a battle-tested, pixel-perfect 2D
rendering engine developed over 30+ years. It handles sub-pixel positioning, font metrics,
anti-aliasing, dashed lines, gradient fills, and complex clipping with well-defined behavior.

Rust's rendering ecosystem offers several alternatives — **vello** (GPU compute shader-based),
**tiny-skia** (CPU software rasterizer), **piet** (cross-platform 2D), **cosmic-text**
(text layout) — but none have the maturity of `QPainter`.

The rendering task involves:
- 29 widget types, each with its own visual appearance (rounded rects, compartments, icons,
  stereotype decorations).
- Association lines with 4 layout types (Direct, Orthogonal, Polyline, Spline) and multiple
  arrow styles (UML-compliant: filled, open, dashed, etc.).
- Text rendering with correct metrics, word wrapping, and alignment for:
  - Class names, attribute lists, operation signatures
  - Stereotype labels («guillemet» delimiters)
  - Multiplicity labels on association endpoints
  - Role names, constraint text
- Zoom levels, anti-aliasing at different scales.
- Print/export to SVG, PNG, PDF (QPainter provides all these natively).
- Selection handles, resize indicators, grid snapping visualization.

Even small differences in font metrics (1–2px discrepancies) cause visible misalignment in
diagrams — text overflowing compartments, labels overlapping lines, elements misaligned
after zoom.

#### Likelihood Assessment

High. Visual differences between `QPainter` and any Rust 2D renderer are near-certain on the
first attempt. The question is whether the differences are acceptable or jarring.

#### Impact Assessment

**Critical**. If rendering quality is noticeably worse than the C++ version:
- Users perceive the Rust version as "unprofessional" or "broken."
- Diagrams become hard to read (text overflow, overlapping elements).
- Exported images (PNG, SVG) look different from C++ output.
- Print output quality suffers.
- Users lose trust in the tool.

#### Mitigation Strategies

1. **Abstract rendering behind a trait**: Define a `Renderer` trait that all drawing code
   targets. Multiple backends can be swapped:
   ```rust
   pub trait Renderer {
       fn draw_rect(&mut self, rect: Rect, fill: &Fill, stroke: &Stroke);
       fn draw_text(&mut self, pos: Point, text: &str, font: &FontDef, align: Alignment);
       fn draw_line(&mut self, from: Point, to: Point, style: &LineStyle);
       fn draw_polyline(&mut self, points: &[Point], style: &LineStyle);
       fn draw_arc(&mut self, center: Point, r: f64, start: f64, end: f64);
       fn measure_text(&self, text: &str, font: &FontDef) -> Size;
       // ... etc
   }
   ```

2. **Start with tiny-skia (CPU)**: Begin with a software rasterizer for correctness.
   tiny-skia is a well-tested Skia subset, deterministic output, easy to debug.
   Add vello (GPU) later for performance.

3. **Screenshot diffing CI pipeline**: Render known diagrams in both C++ and Rust, then
   compare pixel-by-pixel. Use `perceptualdiff` or similar for human-visual comparison.
   Check into CI as a gate.

4. **Use cosmic-text for text**: cosmic-text provides high-quality font layout with correct
   metrics, shaping, and line breaking. It handles the complexities that QPainter's text
   engine manages.

5. **Pixel tolerance baseline**: Establish an acceptable pixel-difference threshold early.
   Not every pixel will match QPainter exactly. Document the tolerance and the reasons.

6. **Widget rendering reference renders**: For each of the 29 widget types, capture a
   reference PNG from the C++ version. Write tests that compare Rust rendering of the same
   widget against the reference.

7. **Use SVG as an intermediate format**: QPainter can produce SVG (via `QSvgGenerator`).
   Compare SVG output from C++ and Rust as a text-based diff (normalized). This catches
   structural rendering differences without pixel comparison.

8. **Accessibility-aware rendering**: Ensure contrast ratios, focus indicators, and
   screen-reader compatibility are not lost in the transition.

#### Contingency Plan

If Rust rendering cannot match C++ quality within project timeline:

- **Hybrid approach**: Keep C++ QPainter rendering for the diagram canvas via FFI
  (or via cxx-qt bridging). The Rust implementation handles everything else.
- **C++ rendering server**: Run a lightweight C++ process that renders via QPainter and
  returns pixel buffers or SVG. The Rust GUI calls it over IPC.
- **Accept visual regression**: Document known visual differences. Prioritize functional
  correctness over pixel-perfect rendering in v1.0.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| Screenshot diff > 5% pixel difference | Yellow | Investigate root cause |
| Screenshot diff > 20% pixel difference | Red | Blocking, switch to hybrid approach |
| Text overflow in any widget type | Red | Fix text layout before release |
| Font metrics mismatch > 1px on average | Yellow | Tune or switch text engine |
| SVG output structural mismatch | Yellow | Logged, may accept |
| User complaints about rendering quality | Red | Prioritize fixes |

---

### R03 — Feature Parity (Critical)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | High (75%)              |
| **Impact**      | Major (4)               |
| **Rating**      | **16 of 25**            |
| **Direction**   | Increasing — the gap grows as C++ version continues development |

#### Description

The C++ Umbrello has been developed over **20+ years** and contains hundreds of features —
many undocumented, some known only to long-time users. The Rust rewrite must either match
these features or consciously drop them.

Key feature counts:

| Feature Area | Count | Complexity |
|-------------|:-----:|:----------:|
| Code generators | 22 languages | 2 strategies (simple streaming + advanced document-tree) |
| Code importers | 10+ languages | C++ (50-node AST), line-scanning for others |
| Widget types for diagrams | 29 widget types | Different visual styles, interactions, XMI state |
| Diagram types | 10 types | Class, Use Case, Sequence, Collaboration, State, Activity, Component, Deployment, Entity Relationship, Object |
| Association types | 25+ types | Generalization, dependency, association, aggregation, composition, etc. |
| Dock widgets | 8+ | Tree view, documentation, undo history, log, birdview, stereotypes, diagrams, objects |
| Dialog pages | 40+ | Properties (per element type), settings (8 pages), wizards |
| Menu types | 260+ | Context menu entries varying by selection and state |
| Commands (undo/redo) | 20+ | Model and widget operations |
| CLI flags | 8 | Export, import, language selection |
| Diagram layout | 4 layouts | Direct, Orthogonal, Polyline, Spline |
| Find/search | 3 finders | Document, scene, list view |

Additionally, there are many small "quality of life" features that users depend on:
- Auto-save with configurable interval.
- Recent files list.
- Grid snap and alignment guides.
- Copy/paste across diagrams.
- Drag-and-drop from tree view to diagram.
- Stereotype editing.
- Tagged values.
- Template parameters on classifiers.
- Autosave recovery on crash.
- Print with page setup.
- Export to image formats.

#### Likelihood Assessment

High. It is extremely unlikely that every feature will be implemented in the initial Rust
release. Some features may never be ported if usage is low.

#### Impact Assessment

**Major**. If too many features are missing:
- Existing Umbrello users cannot switch — they depend on specific functionality.
- "The Rust version doesn't have X" becomes a common refrain in the community.
- Feature requests overwhelm the development team.
- New users may choose alternatives that have broader feature sets.

#### Mitigation Strategies

1. **Feature parity checklist**: Create and maintain a living document listing every feature
   of the C++ version with priority (P0–P4), implementation status, and notes. This is the
   single source of truth for feature tracking.

2. **Usage-frequency prioritization**: Instrument the C++ version (if possible) or survey
   users to determine which features are most used. The Pareto principle applies: 80% of
   users use 20% of features.

3. **Minimal viable product (MVP) definition**: Define a clear v1.0 feature cut line.
   Communicate it publicly so expectations are managed.

4. **Plugin architecture for code generators**: The 22 code generators are highly repetitive
   (18 share the same streaming pattern). A plugin architecture + template engine reduces
   the per-language implementation cost from ~10 files to ~1 template + config.

5. **Community contribution paths**: Document how contributors can add features — especially
   code generators for "their" language. Make it easy to add a new language without touching
   core code.

6. **Feature gating**: Use Cargo features to compile optional subsystems. Not all
   languages/importers need to be compiled by default. Users who need a specific language
   enable it.

7. **Quantitative feature coverage metric**: Track "percent of C++ functionality available
   in Rust" as a measurable KPI. Set a target of 80% for v1.0.

#### Contingency Plan

If feature parity progress is too slow:

- **Target 80/20**: Accept that ~20% of features (least used) will not be implemented in
  the first release. Document them as known limitations.
- **Phase 2 features**: Move less-common features to a post-1.0 roadmap.
- **C++ feature bridge**: For very complex features (e.g., the advanced code document model),
  expose C++ functionality via FFI during the transition period.
- **Community drives**: Heavily promote community-driven feature development for
  language-specific features like code generators and importers.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| P0 features incomplete at v1.0 target | Red | Delay release or cut scope |
| P1 features < 60% complete at v1.0 | Yellow | Evaluate what to defer to v1.1 |
| Community feature requests > 50% of open issues | Yellow | Need more contributor documentation |
| Code generator coverage < 12 of 22 languages | Yellow | Acceptable for v1.0 if most-used are covered |
| User survey shows low feature satisfaction | Red | Prioritize survey-identified gaps |

---

### R04 — GUI Framework (Critical)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | High (70%)              |
| **Impact**      | Major (4)               |
| **Rating**      | **16 of 25**            |
| **Direction**   | Decreasing — Rust GUI ecosystem is maturing, but still risky |

#### Description

The C++ Umbrello is built on **Qt** and **KDE Frameworks** — one of the most mature,
feature-rich GUI toolkits in existence. It provides:

- Main window with dock widgets, toolbars, status bar (via `KXmlGuiWindow`).
- 8+ dock widgets: tree view, documentation editor, undo history, log, bird view,
  stereotypes, diagrams list, objects window.
- 40+ dialogs with multi-page property editors (via `KPageWidget`).
- Rich text editing for documentation (via `QTextEdit` / `KTextEditor`).
- Full internationalization (via `KLocalizedString`).
- Configuration system (via `KConfig`).
- Printing, clipboard, drag-and-drop.
- Accessibility infrastructure.

The Rust GUI ecosystem is fragmented and less mature:

| Framework | Maturity | Dock Widgets | Property Grids | Text Editing | Canvas Performance |
|-----------|:--------:|:------------:|:--------------:|:------------:|:-----------------:|
| **egui** | High | Poor (not designed for MDI) | Basic | Minimal | Good |
| **iced** | Medium | Limited | Limited | Basic | Good |
| **slint** | Medium | Limited | Limited | Basic | Good (GPU) |
| **tauri** (web) | High | HTML/CSS (flexible) | Rich (web libs) | Rich (web) | HTML Canvas |
| **druid** | Low | Ended | N/A | N/A | N/A |
| **xilem** | Experimental | N/A | N/A | N/A | Future |

No Rust GUI framework offers:
- A mature dock widget system comparable to Qt's QDockWidget.
- A property grid widget (for UML element properties).
- Rich text editing with font/style/paragraph controls.
- Full accessibility support.
- Mature printing infrastructure.

#### Likelihood Assessment

High. Building a professional-quality GUI in Rust today is significantly harder than doing
so with C++/Qt. The team will face unexpected challenges.

#### Impact Assessment

**Major**. If the GUI quality is insufficient:
- Users perceive the application as amateurish.
- Productive use is hindered by poor UX.
- Complex workflows (multi-diagram editing, property editing) become frustrating.
- Accessibility requirements may prevent adoption in regulated environments.

#### Mitigation Strategies

1. **Model-first strategy**: Build all non-GUI subsystems first (model, XMI, code generators,
   code importers, scene logic). Verify them with CLI tools and tests. Add the GUI last.
   This reduces GUI risk because the core functionality exists regardless of GUI framework
   maturity.

2. **Prototype with 2–3 frameworks**: Dedicate 2–3 weeks to building the same minimal GUI
   (main window + diagram canvas + property panel) in the top candidate frameworks. Evaluate
   on: dock widgets, canvas performance, text editing, ease of integration,
   cross-platform quality, documentation quality.

3. **Dual-layer architecture**: Separate the diagram rendering canvas from the application
   shell UI. The canvas uses `vello` (GPU) or `tiny-skia` (CPU) for rendering; the shell
   uses a different framework (egui, iced, or slint). This is possible if the renderer trait
   (see R02) is well-designed.

4. **Consider hybrid approach**: Use egui for panels (tree view, properties, settings) and
   vello for the diagram canvas. egui has immediate-mode simplicity; vello has GPU rendering
   quality.

5. **Consider cxx-qt bridging**: If Rust GUI is insufficient, bridge to C++ Qt via `cxx-qt`.
   This allows reusing the mature Qt infrastructure while writing new code in Rust.
   This is a significant contingency option.

6. **Defer advanced GUI features**: Complex dialogs (multi-page property editors, code
   generation wizard) can be built after the core GUI is stable. Start with simple dialogs.

7. **Accessibility as a separate track**: Don't attempt full accessibility in v1.0.
   Document it as a post-1.0 focus area.

8. **Printing**: Start with export to SVG/PDF via the rendering backend. Printing via
   system print dialog can be deferred.

#### Contingency Plan

If the Rust-native GUI approach proves unacceptably slow or low-quality:

- **Contingency A (cxx-qt)**: Build the GUI with Qt via `cxx-qt`. The core logic (model,
  XMI, scene) is in Rust; the Qt GUI is a thin shell. This is the safest but least
  "pure Rust" option.
- **Contingency B (Tauri)**: Build the GUI as a web application using Tauri. The Rust
  code is the backend; the UI is HTML/CSS/TypeScript. This gives rich UI capabilities
  at the cost of complexity.
- **Contingency C (CLI-first release)**: Release v1.0 as a CLI tool with all the core
  functionality (model, XMI, code gen, code import). GUI is v2.0. This defers the risk
  entirely but reduces adoption.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| Prototype evaluation shows framework deficiency | Yellow | Switch framework, or go hybrid |
| Dock widget implementation > 3 months | Yellow | Simplify dock widget design |
| Property grid implementation > 2 months | Yellow | Simplify property editor (form-based) |
| Canvas rendering performance < 30 fps on reference hardware | Yellow | Optimize or switch renderer |
| Accessibility requirements raised by users | Info | Document as roadmap item |
| Text editing quality insufficient for documentation | Yellow | Consider embedding a web-based editor |

---

### R05 — Performance (High)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Medium (50%)            |
| **Impact**      | Major (4)               |
| **Rating**      | **12 of 25**            |
| **Direction**   | Stable — performance is predictable but must be validated |

#### Description

The C++ version is fast — it's compiled native code with Qt's well-optimized rendering.
However, the Rust version should be **comparable or better** due to:
- Rust's zero-cost abstractions.
- Potential for GPU-accelerated rendering (vello).
- Efficient generational arena for object storage.
- Streaming XMI parsing instead of DOM.

Potential performance regressions:

| Area | C++ Approach | Rust Approach | Risk |
|------|-------------|---------------|------|
| XMI loading | `QDomDocument` (full DOM) | `quick-xml` (streaming) | **Lower memory** — Rust wins |
| XMI saving | `QXmlStreamWriter` (streaming) | `quick-xml` writer | **Comparable** |
| Object access | Raw pointer from `QObject` parent tree | Generational arena lookups | **Comparable** — arena is O(1) |
| Rendering | `QPainter` (CPU) | vello (GPU compute) | **Different profile** — GPU may be slower for small diagrams |
| Text layout | `QFontMetrics` (native) | cosmic-text (Rust) | **Slower initially** — caching needed |
| Large models (1000+ classes) | `QObject` tree, linear scans | Arena + indexed queries | **Faster** — indexed access |
| Undo stack | `QUndoStack` (single-thread) | Custom command stack | **Comparable** |
| Auto-layout | Shell exec `dot` (external) | Same dependency | **Same** — no improvement |
| Startup time | Load XMI + build widgets | Load XMI + lazy widget build | **Faster** — lazy loading |

#### Likelihood Assessment

Medium. Performance is unlikely to be worse across the board, but specific areas
(rendering, text layout, initial model creation) may regress.

#### Impact Assessment

**Major**. If performance is noticeably worse:
- Users dealing with large models (enterprise-scale) hit slowdowns.
- User experience degrades — lag when scrolling, zooming, selecting.
- Rust version perceived as academically interesting but practically unusable.
- C++ version remains the performance baseline.

#### Mitigation Strategies

1. **Benchmark early and often with `criterion.rs`**: Establish a performance benchmark
   suite from day one. Test XMI parsing throughput, model query operations, widget rendering
   time, text layout throughput. Track against C++ baseline.

2. **Profile with `flamegraph-rs`** during development to identify hot spots early.

3. **Generational arena for all domain objects**: Use `generational-arena` or `slotmap`
   for O(1) access and cache-friendly iteration.

4. **Lazy widget instantiation**: Don't create all widgets when loading a model. Create
   widgets only when their containing diagram is first viewed. For large models with many
   diagrams, this drastically reduces startup time.

5. **Viewport-based rendering**: Only render widgets visible in the current viewport.
   For large diagrams, this is essential.

6. **Text layout caching**: Cache font metrics and laid-out text per (text, font,
   width) tuple. Invalidation on font change only.

7. **Streaming XMI parser**: Use `quick-xml` with a pull-based (event-driven) parser.
   Avoid building the full DOM tree. Process elements as they are encountered.

8. **Multi-threaded XMI loading**: Split the XMI file into sections (stereotypes, model
   elements, diagrams) and load them in parallel where dependencies allow.

9. **Memory budget**: Set a target: loading a 10MB XMI file should consume no more than
   2x the file size in RAM. Streaming parsing helps meet this target.

#### Contingency Plan

If performance targets are not met:

- **Render backend switch**: If vello (GPU) is slow, switch to tiny-skia (CPU). CPU
  rendering is slower for complex scenes but more predictable.
- **Eager vs lazy trade-off**: If lazy loading adds complexity without benefit, switch
  to eager loading.
- **Simplify text layout**: If cosmic-text is too slow, consider a simpler text layout
  approach (monospace-only layout, or cached bitmaps for common labels).
- **Late-mitigation optimization sprint**: Dedicate one full sprint (2 weeks) to
  performance optimization before release.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| XMI load time > 1.5x C++ time | Yellow | Profile, optimize parser |
| XMI load time > 3x C++ time | Red | Blocking, investigate fundamental issue |
| Rendering < 30 fps on reference hardware | Yellow | Optimize or switch renderer |
| Memory > 3x XMI file size during load | Yellow | Investigate memory leaks or over-retention |
| Text layout > 5ms per label | Yellow | Implement caching |
| Startup time > 5 seconds cold | Yellow | Implement lazy loading |

---

### R06 — Plugin Architecture (High)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Medium (40%)            |
| **Impact**      | Major (4)               |
| **Rating**      | **12 of 25**            |
| **Direction**   | Stable — technology is improving, but complexity is high |

#### Description

The C++ version has **no working plugin system** — the abandoned `Plugin`/`PluginLoader`
classes in `_unused/` were never completed. All code generators, importers, and widgets
are compiled into the monolithic `libumbrello` static library.

The Rust version should provide a proper plugin system to:
- Allow community-contributed code generators without modifying core.
- Allow third-party extensions (new diagram types, importers, exporters).
- Enable independent development by separate teams.

**Dynamic loading in Rust is significantly harder than in C++**:
- C++ `dlopen` + dlsym for function pointers is straightforward (though ABI-sensitive).
- Rust has no stable ABI — dynamic loading across compiler versions is unsupported.
- The `abi_stable` crate provides a solution but adds complexity and boilerplate.
- WASM-based plugins are an alternative but introduce a different execution model.

#### Likelihood Assessment

Medium. A plugin system may work well but will take significant effort, and the risk of ABI
incompatibilities or performance overhead is real.

#### Impact Assessment

**Major**. Without a plugin system:
- The monolithic approach limits community contributions.
- Adding a new language requires editing core code and rebuilding.
- The C++ problem (everything compiled in) is replicated.
- Long-term maintenance burden is higher.

#### Mitigation Strategies

1. **Start with compile-time plugins**: Use Cargo features + trait objects. Users who want
   Python code generation add `--features codegen-python` to their build. This requires no
   dynamic loading and is fully supported by Rust's build system.

2. **Defer dynamic loading**: Don't implement dynamic plugin loading in v1.0. The
   compile-time approach is simpler and sufficient for the initial release. Add dynamic
   loading in v2.0 if there is demand.

3. **Design for plugin interface from day one**: Even without dynamic loading, define the
   plugin trait interfaces:
   ```rust
   pub trait CodeGeneratorPlugin: Send + Sync {
       fn name(&self) -> &'static str;
       fn supported_languages(&self) -> &[ProgrammingLanguage];
       fn generate(&self, model: &ModelRepository, config: &CodeGenConfig) -> Result<GeneratedFiles>;
   }

   pub trait ImportPlugin: Send + Sync {
       fn name(&self) -> &'static str;
       fn file_extensions(&self) -> &[&'static str];
       fn import(&self, source: &str) -> Result<Vec<UmlObject>>;
   }
   ```

4. **If dynamic loading is needed**: Use `abi_stable` crate for stable Rust ABI.
   Provide extensive documentation and examples for plugin authors.

5. **WASM plugin option**: For sandboxed plugins (especially code generators), WASM
   provides a safe, cross-platform execution environment. Evaluate `wasmtime` for this.

6. **Plugin registry pattern**: Make the registry extensible at runtime even for
   compile-time plugins:
   ```rust
   let mut registry = CodeGenRegistry::new();
   registry.register(Box::new(CppGenerator::new()));
   registry.register(Box::new(JavaGenerator::new()));
   // User adds:
   registry.register(Box::new(PythonGenerator::new()));
   ```

#### Contingency Plan

If plugin system development stalls:

- **Pragmatic approach**: Keep all plugins compile-time. Document "how to add a language"
   as a guide that involves modifying `Cargo.toml` and adding a module. This is the same
   workflow as adding code to the C++ version.

- **Re-evaluate demand**: If only 2–3 people contribute external plugins in the first
   year, the compile-time approach may be sufficient permanently.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| External contribution attempts fail | Yellow | Improve plugin documentation |
| Complaints about rebuild time for single-language changes | Yellow | Consider breaking into separate crates |
| More than 5 languages contributed externally | Yellow | Evaluate dynamic loading |
| Plugin system development > 2 months | Yellow | Cut scope — use compile-time only |

---

### R07 — Test Coverage (High)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Medium (50%)            |
| **Impact**      | Major (4)               |
| **Rating**      | **12 of 25**            |
| **Direction**   | Decreasing — test infrastructure is being established |

#### Description

The C++ version has a **limited test suite** — 13 standard tests (plus 2 LLVM-dependent).
These focus mainly on model round-trip (save/load XMI for individual objects). There are no
tests for:
- Code generator output correctness.
- Code importer correctness.
- Widget rendering.
- Association geometry.
- Diagram layout algorithms.
- UI interaction workflows.
- Performance regression.

The Rust rewrite can and must do better, but testing a complex UML tool is hard:
- **GUI testing**: Hard in any framework, harder in Rust's nascent GUI ecosystem.
- **Rendering correctness**: Pixel-based comparison is fragile; structural comparison (SVG)
  is better but still incomplete.
- **XMI compatibility**: Requires access to real-world XMI files and their expected
  in-memory model representation.
- **Code generation**: Generated code must be syntactically and semantically correct across
  22 languages.

#### Likelihood Assessment

Medium. The team recognizes the importance of testing and has planned comprehensive
strategies (property-based, snapshot, golden file). The risk is in execution — testing
sophisticated software is inherently time-consuming.

#### Impact Assessment

**Major**. Inadequate testing leads to:
- Regressions in XMI round-trip compatibility.
- Silent rendering defects.
- Incorrect code generation (users get broken source files).
- Crashes on unusual inputs.
- Low confidence for releases.

#### Mitigation Strategies

1. **Property-based testing with `proptest`**: Generate random UML models and test
   invariants: serialization round-trip, association integrity, constraint enforcement.

2. **Snapshot testing with `insta`**: For code generation output. Generate code with the
   Rust version, compare against expected output stored as snapshots. Review and update
   snapshots on intentional changes.

3. **Golden file testing for XMI**: Store a curated set of XMI files (from C++ Umbrello's
   test directory, from foreign tools, from real user models). The Rust parser must load
   each one and produce the expected in-memory model.

4. **Fuzz testing with `cargo-fuzz`**: Fuzz the XMI parser with malformed input, edge
   cases, and large files.

5. **Render comparison**: See R02 — screenshot diffing for widget rendering.

6. **Code generation validation**: For each of the 22 languages, compile/lint the generated
   code to verify syntax. For a subset (C++, Java, Python), run unit tests on generated code.

7. **80%+ code coverage target**: Use `tarpaulin` or `cargo-llvm-cov` to measure line and
   branch coverage. Set the target at 80% for core model/XMI code, 60% for GUI code.

8. **CI with testing gating**: All tests must pass before merge. Fuzz tests run on a
   schedule (not per-commit, due to time).

9. **Test the C++ version first**: Before writing the Rust implementation, write tests that
   verify the C++ version's behavior. These become the specification for the Rust version.

#### Contingency Plan

If comprehensive testing threatens the timeline:

- **Risk-based test prioritization**: Focus tests on XMI compatibility (most critical),
  model invariants, and code generation correctness. Defer rendering and GUI tests.
- **Property-based test timeout**: Limit `proptest` runs to 30 seconds per test in CI.
- **Manual testing acceptance**: Accept that some areas (GUI workflows) will rely on
  manual testing for v1.0.
- **Outsource test development**: If testing is a bottleneck, consider dedicated QA
  resources or community testing days.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| Code coverage < 50% for core model at v1.0 | Red | Increase testing effort |
| Coverage < 30% for XMI parser at v1.0 | Red | Blocking — XMI bugs are critical |
| Property-based test failures > 1 per week | Yellow | Investigate, may indicate logic errors |
| Fuzz test crashes > 0 | Red | Fix all crashes before release |
| Snapshot review backlog > 50 un-reviewed | Yellow | Schedule snapshot review session |

---

### R08 — Community and Momentum (High)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Medium (50%)            |
| **Impact**      | Major (4)               |
| **Rating**      | **12 of 25**            |
| **Direction**   | Increasing — risk grows if no visible progress is made early |

#### Description

The C++ Umbrello is an **actively maintained** project (30+ year history, still receiving
updates). The Rust rewrite is a new effort that must:
- Attract contributors.
- Build user trust.
- Deliver value quickly enough to maintain momentum.
- Avoid the "second system effect" — over-engineering in the rewrite.

**Rewrites have a well-known reputation for failing** (Netscape-to-Firefox, Lotus-to-Eclipse,
countless others). The key failure mode is taking too long to deliver value, causing
the team to lose motivation and the community to lose interest.

#### Likelihood Assessment

Medium. Many rewrites fail, but this one has advantages: clear scope, strong analysis phase,
and a supportive community.

#### Impact Assessment

**Major**. If the project loses momentum:
- Contributors stop showing up.
- The C++ version remains the canonical version.
- The Rust code becomes abandoned.
- Future UML tooling in Rust is set back.

#### Mitigation Strategies

1. **Release early and often**: The first release should be a **CLI tool** that can read
   XMI files, display model information, and generate code. This ships before any GUI.
   Users get immediate value:
   - "I can use Umbrello-RS from my CI pipeline to generate C++ code."
   - "I can validate my XMI files with `umbrello check model.xmi`."
   - "I can convert my XMI files to JSON for custom tooling."

2. **Milestone plan with measurable deliverables**:

   | Milestone | Deliverable | Timeline |
   |-----------|-------------|----------|
   | M1: Core types | Enums, IDs, basic types | Month 1 |
   | M2: XMI persistence | XMI load/save round-trip | Month 3 |
   | M3: CLI model browser | `umbrello info`, `umbrello check` | Month 4 |
   | M4: Code generation | Top 5 languages | Month 6 |
   | M5: Code import | Top 3 importers | Month 8 |
   | M6: Minimal GUI | Read-only diagram canvas | Month 10 |
   | M7: Interactive GUI | Full editing | Month 12 |
   | M8: Feature complete | v1.0 release | Month 18 |

3. **Community involvement**: Involve the existing Umbrello community early:
   - Publish the risk assessment and architecture documents for feedback.
   - Ask for XMI test files from real users.
   - Conduct a survey about feature priorities.
   - Host a community call / AMA.

4. **XMI compatibility guarantee**: Communicate clearly: "Your existing XMI files will work
   with Umbrello-RS." This is the most important trust-building message.

5. **Show progress visually**: Maintain a public dashboard showing feature parity progress,
   test coverage, and performance metrics.

6. **Avoid over-engineering**: The analysis phase identified many improvements (arena-based
   storage, events instead of signals, trait-based dispatch). Implement these step by step.
   Don't block the first working version on perfect architecture.

7. **Written community charter**: Define how contributions are accepted, what the review
   process is, and how decisions are made. Transparency builds trust.

8. **Regular release cadence**: Ship a new version monthly during the first year. Even if
   the changes are small, regular releases demonstrate progress.

#### Contingency Plan

If the project loses momentum:

- **Narrow scope aggressively**: Drop all but the most-used features. Ship a focused
  "Class Diagram Editor + C++ Code Generator" tool.
- **Find champions**: Actively recruit 2–3 community members who are passionate about
  making the Rust version succeed.
- **Corporate sponsorship**: If the project is strategically important, seek sponsorship
  (KDE e.V., a company using Umbrello, etc.).
- **Honest sunset**: If the project truly cannot succeed, state it publicly and help
  community members migrate to alternatives.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| GitHub stars / contributors flatline | Yellow | Increase outreach, evaluate barriers |
| No community contributions after 6 months | Yellow | Improve onboarding documentation |
| Negative community sentiment in forums | Red | Engage directly, address concerns |
| Development slows to < 10 commits/month | Red | Assess team health, cut scope |
| C++ version gains major new features | Info | Evaluate whether to add same features |
| No new XMI test files submitted by community | Info | Proactively ask for test files |

---

### R09 — Complexity (High)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Medium (50%)            |
| **Impact**      | Major (4)               |
| **Rating**      | **12 of 25**            |
| **Direction**   | Stable — domain complexity is constant |

#### Description

UML is a **vast domain** governed by OMG specifications. The full UML 2.5 specification
is over 800 pages. Umbrello implements a subset of UML — but even that subset is complex:

| Dimension | Complexity Factor |
|-----------|-------------------|
| Model types | 30+ object types with varying properties |
| Relationships | 25+ association types, each with roles, multiplicities, constraints |
| Diagrams | 10 types, each with different widget types and layout rules |
| Constraints | OCL (Object Constraint Language) support, tagged values |
| Templates | Template parameters, template bindings, bound elements |
| Stereotypes | User-defined extension of UML metaclasses |
| Code generation | Mapping UML constructs to 22 programming language idioms |
| XMI | Complex XML schema with two versions and multiple extensions |

Beyond the OMG specification, the C++ version has its own:
- Internal consistency rules (what relationships are valid between which elements).
- Diagram layout heuristics (auto-layout uses Graphviz, but manual layout rules exist).
- Conflict resolution for duplicate names, circular dependencies, etc.
- Migration code for older file formats.

**Diagram layout is NP-hard** in general. The C++ version uses Graphviz (external) for
auto-layout. The manual layout algorithms (orthogonal routing, spline computation) are
complex geometric code.

#### Likelihood Assessment

Medium. The complexity is well-understood, but some edge cases will inevitably be
overlooked until discovered by users.

#### Impact Assessment

**Major**. Underestimating complexity leads to:
- Missed deadlines (the "last 10%" takes 90% of the time).
- Bugs in edge cases that frustrate users.
- Design decisions that work for common cases but fail for uncommon ones.

#### Mitigation Strategies

1. **Start with class diagrams**: Class diagrams are the most-used diagram type (estimated
   80% of all usage). Sequence diagrams, activity diagrams, and state diagrams have
   10× the complexity with 1/10th the usage. Prioritize class diagrams first.

2. **Exotic diagram types as optional modules**: State, activity, collaboration, object,
   deployment, component, entity-relationship diagrams are important but less-used.
   Make them optional Cargo features.

3. **C++ behavior as reference**: For every complex behavior (e.g., "what happens when you
   drag an association endpoint"), check the C++ version's behavior first. This is the
   specification.

4. **Simplified constraint system**: Full OCL constraint parsing is very complex.
   Consider storing constraints as structured strings with a simple parser for v1.0,
   deferring full OCL to v2.0.

5. **Manual layout with smart defaults**: Don't attempt to build a competitive auto-layout
   system. Use Graphviz (as C++ does) as an external dependency. Focus on manual layout
   tools (snap-to-grid, alignment, distribution) that are more commonly used.

6. **Version your file format**: The Rust version should produce XMI that declares its
   Rust version (`umbrello-rs 1.0`). If incompatibilities exist, the version tag enables
   migration.

7. **Complexity budget**: Track the number of model types, widget types, and test cases
   as a complexity metric. If they grow too fast, evaluate scope reduction.

#### Contingency Plan

If a specific area proves too complex:

- **Cut feature**: For v1.0, remove support for a diagram type or association type.
  Document the removal and add it to the post-1.0 roadmap.
- **Simplified alternative**: Use a simpler but less functional alternative. For example,
  use straight lines instead of spline routing for associations.
- **External tool bridge**: For very complex features (OCL, advanced layout), provide
  a hook to call an external tool.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| Time to implement one diagram type > 2 months | Yellow | Cut scope of that diagram type |
| Association routing (spline) > 1 month | Yellow | Simplify routing (direct/orthogonal only) |
| Unhandled edge cases found in user testing | Info | Log, prioritize by frequency |
| Size of core model code > 10,000 lines | Yellow | Assess if simplification is possible |
| New XMI edge case discovered weekly | Info | Build up golden file corpus |

---

### R10 — Knowledge Retention (High)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Medium (50%)            |
| **Impact**      | Major (4)               |
| **Rating**      | **12 of 25**            |
| **Direction**   | Increasing — loses value as C++ maintainers drift away |

#### Description

The C++ Umbrello codebase contains **20+ years of accumulated knowledge** about:
- UML modeling conventions and edge cases.
- XMI format quirks and workarounds.
- Foreign tool compatibility (Rose, ArgoUML, NSUML, Embarcadero).
- Code generation patterns for 22 languages.
- UI workflows that users expect.

This knowledge is embedded in:
- ~550 source files of C++ code.
- DTD files.
- XMI test files.
- Build system configuration.
- KDE integration patterns.
- Bug tracker history (thousands of issues, many documenting edge cases).

**Single points of knowledge loss**:
- The C++ maintainers may not have time to answer questions.
- C++ code is not always self-documenting.
- "Why is this done this way?" is often answered by "because that's what the XMI parser
  from 2005 expected."

#### Likelihood Assessment

Medium. Some knowledge will inevitably be lost, but the analysis phase has already
documented much of it. The risk is in the undocumented "tribal knowledge."

#### Impact Assessment

**Major**. If knowledge is lost:
- The Rust version may repeat bugs that were fixed in C++ years ago.
- Foreign XMI compatibility may be incorrect (subtle attribute handling).
- Code generation may produce incorrect output for edge cases.
- Users encounter bugs that the C++ version resolved in 2010.

#### Mitigation Strategies

1. **Write tests against C++ behavior first**: For every feature, write tests that verify
   the C++ version's behavior before implementing it in Rust. This captures the expected
   behavior as executable specifications.

2. **Document discoveries during exploration**: Maintain a "notes" directory alongside
   the code with findings about edge cases, undocumented behaviors, and design decisions.

3. **Use C++ as oracle in testing**: In the test suite, have a mode that runs both C++
   and Rust implementations of a function and compares outputs. This catches regressions
   in both directions.

4. **Study bug tracker history**: Mine the KDE Bugzilla for Umbrello bugs, especially
   those labeled "fixed" with XMI or code generation tags. These represent knowledge about
   edge cases.

5. **Record architecture decision records (ADRs)**: For every non-trivial design decision
   in the Rust version, write an ADR explaining the alternatives considered and the
   rationale. This prevents "why was this done this way?" confusion in the future.

6. **Conduct pair analysis sessions**: Have the Rust team and C++ maintainers walk through
   complex code together. Record these sessions (or at least take notes).

7. **Publish a C++-to-Rust migration guide**: Document patterns for translating C++ code
   to Rust for this specific codebase. This serves as both knowledge transfer and
   contributor onboarding.

#### Contingency Plan

If knowledge gaps become blocking:

- **Black-box testing**: Treat the C++ version as a black box. Write tests that observe
   C++ behavior (input XMI → output XMI, input model → generated code) and use them to
   validate the Rust version. No understanding of internal C++ logic is required.
- **Reach out to C++ maintainers**: Make a specific, clear request for help on a narrow
   issue. Maintainers are more likely to respond to specific questions than vague requests.
- **Accept known unknowns**: Document areas where knowledge is incomplete. Mark them as
   "experimental" or "may have edge case issues" in the release notes.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| Knowledge blocker > 1 week (can't proceed without answer) | Yellow | Escalate: ask C++ maintainers or reverse engineer |
| Same bug fixed in C++ re-appears in Rust | Yellow | Audit C++ bug tracker for similar patterns |
| No C++ maintainer response within 2 weeks | Yellow | Fall back to black-box testing approach |
| ADRs not written for > 5 major decisions | Yellow | Schedule ADR writing session |
| New contributor asks "why is this done this way?" | Info | Ensure answer is documented |

---

### R11 — Dependency (Medium)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Low (20%)               |
| **Impact**      | Major (4)               |
| **Rating**      | **8 of 25**             |
| **Direction**   | Decreasing — can be mitigated with good practices |

#### Description

The Rust version will depend on **third-party crates** for:
- XMI parsing: `quick-xml`, `serde`, `serde-xml-rs`.
- Rendering: `vello`, `tiny-skia`, `cosmic-text`.
- GUI: egui / iced / slint / tauri.
- Code generation: templates, string formatting.
- Tree-sitter grammars for code import.
- Archive handling: `tar`, `flate2`, `bzip2`.
- Command-line: `clap`.
- Testing: `proptest`, `insta`, `criterion`.
- IDs: `uuid`.
- Arenas: `generational-arena` or `slotmap`.

**Risks**:
- **Crate abandonment**: A key dependency may become unmaintained.
- **Breaking changes**: Semver violations or major version bumps requiring significant work.
- **Security vulnerabilities**: A crate may have a vulnerability requiring urgent update.
- **License incompatibility**: A crate's license may conflict with project goals.
- **Quality variance**: Especially for tree-sitter grammars — some languages have excellent
  grammars, others have poor ones.
- **Rendering crates under active development**: Vello is at v0.1.x — API may change.

#### Likelihood Assessment

Low. The Rust ecosystem is mature enough that most crates are well-maintained. Breaking
changes are manageable with good pinning practices. The main risk is in rendering crates
(vello) which are under active development.

#### Impact Assessment

**Major**. A critical dependency problem could require:
- Rewriting parts of the codebase to use a different crate.
- Delayed releases while waiting for fixes upstream.
- Dropping support for a language (if its tree-sitter grammar is abandoned).

#### Mitigation Strategies

1. **Pin dependencies with `Cargo.lock` in repo**: All builds use exact versions recorded
   in the lockfile. Update intentionally, not accidentally.

2. **Prefer mature, widely-used crates**: For critical infrastructure:
   - `quick-xml` over less-established XML parsers.
   - `serde` for serialization (the de facto standard).
   - `clap` for CLI (battle-tested).
   - `tokio` for async (if needed).
   - `tracing` for logging.

3. **Minimize dependency count**: Each dependency is a risk. Audit dependencies regularly.
   Use `cargo-deny` to check for license issues and security vulnerabilities.

4. **Tree-sitter grammar quality assessment**: Before depending on a tree-sitter grammar for
   code import, evaluate its test coverage, number of open issues, and last commit date.
   For languages with poor grammars, use the simpler line-scanning approach (like the C++
   version's `NativeImportBase`).

5. **Fallback plans for critical dependencies**:

   | Dependency | Risk | Fallback |
   |------------|------|----------|
   | vello | Immature, API changes | tiny-skia (CPU backend) |
   | cosmic-text | Maturity | `fontdue` + custom layout |
   | specific tree-sitter grammar | Abandonment | Line-scanning parser |
   | egui/iced/slint | Breaking changes | Use stable version, defer upgrade |

6. **Separate rendering crate behind trait**: See R02. If vello becomes problematic,
   swap to tiny-skia without affecting other code.

7. **Vendoring as last resort**: If a critical crate is abandoned and no replacement
   exists, vendor the source into the repository and maintain it.

#### Contingency Plan

If a dependency becomes critical-blocking:

- **Fork and fix**: Fork the crate, apply necessary fixes, publish as `umbrello-<name>`.
  This requires commitment to ongoing maintenance.
- **Reimplement the minimal subset**: For small crates, reimplement the needed
  functionality in-house.
- **Switch to alternative**: For most concerns, there is an alternative crate.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| Crate vulnerability advisory published | Yellow | Evaluate impact, update promptly |
| Key crate unmaintained > 6 months | Yellow | Evaluate alternatives, prepare migration |
| Tree-sitter grammar test failures > 10% | Yellow | Tighten grammar or switch approach |
| Vello API breaking changes | Yellow | Pin version, defer upgrade |
| Dependency count > 50 direct dependencies | Yellow | Audit for unnecessary dependencies |

---

### R12 — Maintenance During Transition (Medium)

| Attribute       | Rating                  |
|-----------------|-------------------------|
| **Likelihood**  | Low (25%)               |
| **Impact**      | Moderate (3)            |
| **Rating**      | **6 of 25**             |
| **Direction**   | Increasing — grows the longer dual maintenance is needed |

#### Description

During the transition period, both the C++ and Rust versions must be **maintained in
parallel**. This creates:

- **Dual bug fixes**: A bug found in C++ must be fixed in both versions. A bug found in
  Rust must also be checked against the C++ version.
- **XMI format divergence**: If the C++ version changes its XMI output (new fields,
  different ordering), the Rust version must be updated to remain compatible.
- **New features in C++**: If the C++ version adds features during the transition, the
  Rust team must decide whether to match them, defer, or skip.
- **Community confusion**: Users may not know which version to use for which purpose.

The transition timeline estimate is **12–18 months for v1.0** (see R08). During this
period, both codebases exist.

#### Likelihood Assessment

Low. Dual maintenance is manageable if the teams are coordinated. The risk increases
with the length of the transition period.

#### Impact Assessment

**Moderate**. Dual maintenance is a cost, not a blocker. It requires coordination but
is a solved problem in the software industry.

#### Mitigation Strategies

1. **Shared XMI compliance testing**: Write tests that run against both versions. Any
   change that breaks XMI round-trip is caught immediately.

2. **Clear sunset plan for C++ version**: Communicate that the C++ version will receive
   critical bug fixes only after the Rust version reaches feature parity. Publish a
   timeline:
   - Phase 1 (months 1–6): Both versions active, C++ is primary.
   - Phase 2 (months 7–12): Rust is primary for new development, C++ receives bug
     fixes only.
   - Phase 3 (months 13–18): C++ enters maintenance mode, community encouraged to
     migrate.
   - Phase 4 (month 19+): C++ receives only security fixes.

3. **Automated XMI compliance**: In CI, generate XMI from both versions and verify
   they produce equivalent output. Any difference triggers an alert.

4. **Feature flags in Rust for C++-only features**: When a C++ feature is not yet
   implemented in Rust, and the Rust version is asked to load a model using that feature,
   provide a clear error message: "This model uses X feature not yet available.
   Please use Umbrello C++ v26.x to edit this model."

5. **Separate issue trackers with cross-references**: Link related issues in both
   repositories so no fix is forgotten.

6. **Dedicated transition maintainer**: If budget allows, assign one person (or a
   shared role) to ensure alignment between the two versions during the transition.

#### Contingency Plan

If dual maintenance becomes unsustainable:

- **Accelerate sunset**: Move the C++ sunset date forward. Accept that some features will
  be lost in the Rust version.
- **Freeze C++ development**: Stop new features in C++ sooner than planned. Bug fixes only.
- **Automated porting**: For simple bug fixes (e.g., a missing field in serialization),
  write a script that generates the Rust fix from the C++ fix.

#### Indicators to Watch

| Indicator | Trigger | Action |
|-----------|---------|--------|
| C++ bug fixes not ported to Rust within 2 weeks | Yellow | Improve cross-notification |
| XMI format divergence > 1 attribute | Yellow | Investigate and align |
| C++ version releases new major feature | Yellow | Evaluate: port to Rust or skip? |
| Community reports "which version should I use?" frequently | Yellow | Improve communication |
| Dual maintenance budget > 30% of total dev effort | Yellow | Accelerate sunset plan |

---

## 3. Top 5 Risk Summary

### Top 5 Risks — Recommended Focus Areas

| Rank | ID | Risk | Rating | Why #1 Priority | Critical Decisions Required |
|:----:|:--:|------|:------:|-----------------|-----------------------------|
| **1** | R01 | XMI Compatibility | 25/25 | Without XMI round-trip, the project cannot exist. This is the non-negotiable foundation. | - Accept byte-for-byte equivalence vs semantic equivalence?<br>- Support all foreign XMI dialects from day one? |
| **2** | R02 | Rendering Quality | 20/25 | Visual quality is the most visible aspect of the application. Users judge the book by its cover. | - tiny-skia or vello?<br>- Accept pixel differences vs wait for perfection?<br>- SVG comparison or pixel comparison? |
| **3** | R03 | Feature Parity | 16/25 | Missing features block user migration. The longer the gap, the less likely users switch. | - Which 20% of features are dropped for v1.0?<br>- How many code generator languages are P0?<br>- Plugin architecture vs monolthic? |
| **4** | R04 | GUI Framework | 16/25 | The GUI framework decision is the most consequential technology choice. A wrong choice costs months. | - Which GUI framework to prototype?<br>- Hybrid approach (native shell + custom canvas)?<br>- Defer GUI to v2.0? |
| **5** | R08 | Community & Momentum | 12/25 | Without momentum, the project dies. Early deliverables build trust and attract contributors. | - What is the first public release? (CLI? GUI? Both?)<br>- How to involve existing Umbrello community? |

### Recommended Action Plan for Top 5 Risks

```
Immediate (Sprint 1-2):
├── R01: Collect golden XMI files from C++ repo and community
├── R01: Build XMI fuzz harness
├── R02: Render a single class widget with tiny-skia → compare with C++ PNG
├── R03: Create feature parity checklist with all 22 languages + all diagram types
├── R04: Select 2-3 GUI frameworks for prototyping
└── R08: Publish roadmap on KDE forums, request XMI test files

Short-term (Sprint 3-6):
├── R01: Implement full XMI round-trip with golden file CI
├── R01: Add property-based XMI tests
├── R02: Implement all 29 widget types in tiny-skia
├── R02: Set up screenshot diffing CI pipeline
├── R03: Implement top 5 code generators (C++, Java, Python, PHP, C#)
├── R04: Complete GUI framework prototype evaluation → commit to one
└── R08: Ship CLI v0.1 (XMI reader + model info)

Medium-term (Sprint 7-12):
├── R01: Add foreign XMI dialect support
├── R02: GPU backend (vello) as optional alternative
├── R03: Implement remaining code generators + top 3 importers
├── R04: Ship minimal GUI (read-only diagram canvas)
└── R08: Monthly releases, growing community

Long-term (Sprint 13+):
├── R01: XMI format version migration (if needed)
├── R03: All features — dynamic plugin system
├── R04: Full GUI (editing, multi-diagram, dock widgets)
└── R08: C++ sunset plan execution
```

---

## 4. Risk Tracking Recommendations

### 4.1 Risk Register

Maintain a **living risk register** as a Markdown table in the repository
(`rust-rewrite/risks/register.md`). Update it at least monthly.

| ID | Risk | Rating | Owner | Status | Last Updated | Action Items |
|:--:|------|:------:|:-----:|:------:|:------------:|-------------|
| R01 | XMI Compatibility | 25 | TBD | Active | 2026-06-23 | Collect golden files, build fuzz |
| R02 | Rendering Quality | 20 | TBD | Active | 2026-06-23 | Prototype tiny-skia, set up CI diff |
| ... | ... | ... | ... | ... | ... | ... |

### 4.2 Risk Review Cadence

| Cadence | Activity | Participants |
|---------|----------|-------------|
| **Weekly** | Quick risk check-in (5 min in standup) | Whole team |
| **Monthly** | Formal risk review — update likelihood/impact, review indicators | Tech lead + architect |
| **Quarterly** | Deep risk reassessment — review all 12 categories, update mitigation strategies | Full team + stakeholders |

### 4.3 Risk Dashboard

Publish a simple dashboard visible to the team and community:

- **Traffic light** per risk (Green/Yellow/Red).
- **Trend** arrows (stable / improving / worsening).
- **Top 3 active risks** at the top.
- **Burndown** of risk count (number of red risks over time, targeting 0 red at v1.0).

Host this as a simple HTML page rendered from the risk register in CI.

### 4.4 Risk Owner Responsibilities

Each risk should have a named **owner** who:

- Monitors the risk indicators.
- Executes mitigation actions.
- Reports status at the monthly review.
- Escalates if the risk crosses a threshold (Yellow → Red).

---

## 5. Risk Acceptance Decisions

These risks are **consciously accepted** — the team will not dedicate specific
effort to mitigating them, accepting the potential consequences.

### Accepted Risks

#### A01 — Long-tail code generators (accepted within R03)

**Decision**: For the 22 code generators, implement the **top 8–10 languages** for v1.0.
The remaining 12–14 languages will only be implemented if there is community demand.

**Rationale**: The Pareto principle applies — 80% of users use the top 4–5 languages
(C++, Java, Python, PHP, C#). The remaining languages have very few users and represent
a significant implementation effort (~5–10 days per language).

**Downside**: Users of niche languages (Ada, Pascal, Tcl, Vala, etc.) cannot use the
Rust version for code generation in v1.0.

**Trigger to re-evaluate**: > 5 community requests for a specific omitted language.

#### A02 — Accessibility (accepted within R04)

**Decision**: Full accessibility (screen reader support, keyboard navigation, high-contrast
modes) will not be a v1.0 requirement.

**Rationale**: Rust GUI frameworks have limited accessibility infrastructure. Implementing
accessibility would require significant custom work with uncertain outcomes. Accessibility
will be addressed in v2.0 or when the GUI framework ecosystem matures.

**Downside**: Users with accessibility needs cannot use v1.0. This excludes some
educational and enterprise environments.

**Trigger to re-evaluate**: Regulatory requirement or funding mandate emerges.

#### A03 — Full OCL constraint support (accepted within R09)

**Decision**: The Rust version will store constraints as structured strings with a basic
built-in parser for simple expressions. Full OCL parsing and evaluation is deferred.

**Rationale**: OCL is a complex language to parse and evaluate. The C++ version's OCL
support is also limited. Full OCL is rarely used by most users. Storing constraints as
structured strings preserves them for future implementation.

**Downside**: Users who rely on OCL for model validation cannot use the Rust version for
this purpose in v1.0.

**Trigger to re-evaluate**: User demand for constraint evaluation exceeds 5 requests.

#### A04 — Dynamic plugin system (accepted within R06)

**Decision**: v1.0 ships with a compile-time plugin system (Cargo features). Dynamic
loading is deferred to v2.0.

**Rationale**: Dynamic plugins add significant complexity (ABI stability, WASM runtime,
documentation, plugin SDK) for limited initial benefit. The compile-time approach serves
the same purpose with less risk.

**Downside**: Adding a new code generator requires modifying `Cargo.toml` and rebuilding.
Plugin development requires access to the full Rust toolchain.

**Trigger to re-evaluate**: > 3 distinct community-developed external plugins requested.

#### A05 — Perfection over pragmatism in GUI polish (accepted within R04)

**Decision**: The v1.0 GUI will prioritize **functionality over polish**. Simple
single-document interface, basic property panels, minimal animation. The polished
multi-document dock-widget experience comes in v2.0.

**Rationale**: Building an advanced GUI shell in Rust takes months. The model-first
strategy means the core functionality works even with a basic GUI. Users who need
advanced GUI features can continue using the C++ version.

**Downside**: First impressions may be "this looks like a prototype." Some users may
dismiss the application based on GUI quality.

**Trigger to re-evaluate**: User survey shows GUI quality as the #1 blocker to adoption.

---

## 6. Contingency Budget

Recommend allocating **30% of total project contingency budget** for risk response.
This is separate from the development time budget and is reserved for unplanned work.

### Contingency Allocation by Risk

| Risk | Contingency Reserve (% of total) | Expected Use |
|:----:|:--------------------------------:|-------------|
| R01 — XMI | 10% | Extra testing, edge case handling, foreign dialect support |
| R02 — Rendering | 5% | Hybrid approach if native Rust rendering doesn't meet quality |
| R03 — Feature Parity | 5% | Implementing missing P0 features discovered late |
| R04 — GUI Framework | 5% | Framework switch or cxx-qt contingency |
| R05 — Performance | 2% | Optimization sprint |
| R06–R12 | 3% (combined) | Minor surprises |

**Total contingency reserve**: 30% of total project budget.

### Releasing Contingency

Contingency is released when:
- The risk's status has been "Green" for 3 consecutive monthly reviews.
- The risk owner confirms the indicator thresholds have not been triggered.
- The project lead approves the release.

Released contingency can be reprioritized to other risks or returned to the
feature development budget.

---

## 7. Appendices

### Appendix A: Risk Assessment Methodology

This assessment was conducted using the following methodology:

1. **Codebase analysis**: Exhaustive review of the C++ Umbrello source tree (~550 files,
   analysis documents generated for each subsystem).
2. **Expert knowledge**: Insights from the Umbrello C++ maintainers (via codebase analysis,
   bug tracker, and documentation).
3. **Prior rewrite experience**: Lessons from known software rewrites (both successful and
   failed), including the Netscape rewrite, the GIMP V2 rewrite, and KDE 4 transition.
4. **Rust ecosystem evaluation**: Assessment of current state of key Rust crates (rendering,
   GUI, XML, serialization, parsing).

### Appendix B: Risk Glossary

| Term | Definition |
|------|------------|
| **Likelihood** | The probability that the risk event will occur, expressed as a percentage |
| **Impact** | The severity of consequences if the risk occurs, rated 1–5 |
| **Rating** | Likelihood × Impact, normalized to a 1–25 scale |
| **Critical Risk** | Rating 15–25 — must be addressed before release |
| **High Risk** | Rating 10–14 — must have active mitigation plan |
| **Medium Risk** | Rating 6–9 — monitored, mitigation as needed |
| **Low Risk** | Rating 1–5 — accepted, monitored quarterly |
| **Direction** | Whether the risk is increasing (⬆), decreasing (⬇), or stable (➡) |

### Appendix C: Key Assumptions

This risk assessment makes the following assumptions:

1. The Rust version aims for **XMI compatibility** with the C++ version (not a new file format).
2. The Rust version supports **at least** the diagram types and code generators of the C++ version.
3. The development team has **Rust expertise** but may need to learn UML domain concepts.
4. The C++ version continues to be maintained during the transition.
5. The KDE community supports the Rust rewrite (or is at least neutral).
6. No disruptive changes occur in the Rust GUI/rendering ecosystem during development.
7. The project has a **dedicated team** of at least 2–3 core developers.

### Appendix D: Risk Escalation Process

```
Risk Indicator triggers threshold
        │
        ▼
Risk Owner assesses situation
        │
        ├── Can fix within sprint? ──► Fix, update register
        │
        └── Cannot fix within sprint?
                │
                ▼
        Escalate to tech lead
        │
        ├── Needs decision? ──► Decision made, communicate
        ├── Needs resources? ──► Draw from contingency budget
        └── Changes timeline? ──► Update roadmap, communicate
                │
                ▼
        Update risk register with outcome
```

---

*This is a living document. Update as risks evolve, new risks emerge, and mitigations
succeed or fail. The next scheduled review is 2026-07-23.*
