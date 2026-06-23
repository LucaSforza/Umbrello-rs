# Milestone 15 Architectural Review — Rich Canvas Rendering

> **Document:** `rust-rewrite/docs/milestone15_review.md`  
> **Review date:** 2026-06-23  
> **Reviewer:** Umbrello-RS Reviewer  
> **Proposal under review:** Rich UML canvas rendering (partitioned nodes, semantic edge engine, dynamic sizing)  
> **Proposal document:** `rust-rewrite/docs/ui_rich_rendering_spec_v1.md` (1067 lines)  
> **Codebase verified against:** `rust-rewrite/` as of 2026-06-23; M14 canvas is 296 lines in `apps/umbrello/src/app.rs`; `uml-core` element types, `ClassiferData`, `Relationship`, `AssociationType`, `ViewNode`, `ViewEdge` all fully implemented

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture: Is Rendering Logic in the Right Place?](#2-architecture-is-rendering-logic-in-the-right-place)
3. [Data Access: Does the Renderer Correctly Access Model Data?](#3-data-access-does-the-renderer-correctly-access-model-data)
4. [Dynamic Sizing: Correct Formula and Precedence?](#4-dynamic-sizing-correct-formula-and-precedence)
5. [Arrowhead Geometry: Mathematically Sound?](#5-arrowhead-geometry-mathematically-sound)
6. [Partitioned Node Layout: Zone-by-Zone Review](#6-partitioned-node-layout-zone-by-zone-review)
7. [Package Rendering: Ready for M15?](#7-package-rendering-ready-for-m15)
8. [Performance: Will This Scale?](#8-performance-will-this-scale)
9. [Text Truncation: Sufficient Approach?](#9-text-truncation-sufficient-approach)
10. [Implementation Complexity: Manageable Scope?](#10-implementation-complexity-manageable-scope)
11. [Implementation Discrepancies and Gaps](#11-implementation-discrepancies-and-gaps)
12. [Test Coverage Adequacy](#12-test-coverage-adequacy)
13. [Summary of Required Conditions](#13-summary-of-required-conditions)
14. [Final Recommendation](#14-final-recommendation)

---

## 1. Executive Summary

This review evaluates the M15 rich rendering specification against:

- The **current M14 codebase** — `UmbrelloApp::render_canvas()` (296 lines): solid-coloured rectangles, centre-aligned name text, straight centroid-to-centroid edges, click+drag
- The **uml-core data model** — `ModelElement` enum, `ClassifierData` with `Attribute`/`Operation`/`TemplateParameter`, `Enum.literals`, `Relationship` with `AssociationType`, `TypeReference::display_name()`
- The **diagram types** — `ViewNode` (bounds, z-order, visibility), `ViewEdge` (relationship_id, source/target node IDs, waypoints, routing)
- The **specification's own test plan** — 12 manual visual tests, 6 unit test functions

**Overall finding:** The specification is **thorough, well-researched, and architecturally sound**. It correctly separates rendering (app crate) from domain model (uml-core), uses the correct data access APIs, defines mathematically correct arrowhead geometry, and sets an appropriate scope for a single milestone. The zone layout anatomy for Class, Interface, Enum, Datatype, and Package is complete with correct typographic properties.

Five implementation gaps were identified, all minor and resolvable during implementation:

1. **XMI precedence** — the spec states the rule correctly but the implementation needs a stored `model_loaded_from_xmi: bool` flag that does not currently exist
2. **API mismatch** — the pseudo-code references non-existent methods (`model.elements.find()`, `edge_relationship()`) that need translation to actual APIs (`model.get()`, `model.iter()`)
3. **`TypeReference::display_name` requires model access** — not noted in the spec but essential for attribute/operation formatting
4. **Enum literal field name** — spec says `enum_literals` but the actual field is `Enum.literals`
5. **Render function signature** — pseudo-code uses `&self` but the actual M14 signature is `&mut self` (drag state requires mutability)

**Verdict: APPROVE WITH CONDITIONS — 5 mandatory conditions, 3 recommendations.**

---

## 2. Architecture: Is Rendering Logic in the Right Place?

### 2.1 Claim in Spec

> All rendering code is in `apps/umbrello/src/app.rs` — the GUI application crate. No changes to `uml-core` — the domain model stays pure.

### 2.2 Verification

**Verified.** The spec places all new rendering in `app.rs` (+ optional `canvas.rs` extraction at ~1000 lines). The domain model (`uml-core`) serves as a read-only data source. The six new helper functions (`draw_partitioned_node`, `format_attribute_line`, `format_operation_line`, `draw_dashed_line`, `draw_arrowhead`, etc.) are pure rendering functions that consume model data.

The current M14 implementation already follows this pattern:

```rust
// M14: reads model, draws rectangles (app.rs line 163-206)
node_visuals.push((rect, fill, name.clone()));
// ...
painter.rect_filled(*rect, 4.0, *fill);
painter.text(rect.center(), Align2::CENTER_CENTER, name, ...);
```

M15 extends this with compartment rendering but does not cross the crate boundary.

### 2.3 Verdict: ✅ CORRECT SEPARATION

No architectural boundary violations identified. The spec is consistent with the existing separation between `uml-core` (pure data) and `apps/umbrello` (interactive GUI).

---

## 3. Data Access: Does the Renderer Correctly Access Model Data?

### 3.1 Claim in Spec

> ClassifierData (attributes, operations) accessed via `model.get(element_id).classifier_data()`. Relationship type accessed via `model.get(edge.relationship_id)` matching on `ModelElement::Relationship`. TypeReferences resolved via `display_name()` method.

### 3.2 Verification

**Partially verified — minor API name discrepancies found.**

The spec's pseudo-code in §4.12 and §6.3 uses `model.elements.find(view_node.element_id)`. The actual API is `model.get(view_node.element_id)` which returns `Option<&ModelElement>`.

The spec's §2.3 says `TypeReference::display_name()`. The actual method signature is:

```rust
pub fn display_name(&self, model: Option<&UmlModel>) -> String
```

This is correct, but **not noted in the spec** that the renderer must pass `Some(&model)` to resolve model-ID-based types. The attribute and operation formatters will need model access for type name resolution (e.g., an attribute typed `Customer` where `Customer` is a UML class). The current M14 renderer already has model access (it looks up `self.model.get(...)`), so this is not a new requirement — just an omission from the spec.

The spec's `edge_relationship()` helper does not match any existing API. The correct lookup is:

```rust
// Actual API: iterate model elements, find matching Relationship
fn find_relationship<'a>(model: &'a UmlModel, edge: &ViewEdge) -> Option<&'a Relationship> {
    model.get(edge.relationship_id).and_then(|elem| {
        if let ModelElement::Relationship(rel) = elem {
            Some(rel)
        } else {
            None
        }
    })
}
```

Alternatively, `UmlModel::relationships_of()` could be used, though it filters by participant element, not by relationship ID.

### 3.3 Verdict: ✅ CORRECT ACCESS PATTERNS (with implementation notes)

The conceptual access patterns are correct. The implementation must translate the pseudo-code's shorthand to actual `uml-core` APIs.

---

## 4. Dynamic Sizing: Correct Formula and Precedence?

### 4.1 Claim in Spec

Two-tier behaviour:
1. **XMI-loaded diagrams** — stored `ViewNode.bounds` are authoritative; no auto-sizing
2. **New diagrams** — auto-compute height from content counts, with `max(120, stored_width)` for width

### 4.2 Formula Verification

The height formula in §3.1 is verified:

```
height = top_padding(8) + stereotype(16 or 0) + name(20) + bottom_padding(4)
       + divider(6, only if zones 2+3 present)
       + attributes(N×16, 0 if empty)
       + divider(6, only if zone 3 present)
       + operations(M×16, 0 if empty)
```

For a fully-populated class with stereotype, 4 attributes, 3 operations:
- 8 + 16 + 20 + 4 + 6 + 64 + 6 + 48 = **172 px** ✅

This matches the spec's Appendix A.1 example exactly.

For an empty class (no attributes, no operations, no stereotype):
- 8 + 0 + 20 + 4 = **32 px** ✅

The width formula `max(120, stored_width_from_xmi)` is a reasonable minimum. **Potential issue:** For XMI-loaded diagrams, the stored width may be less than 120 px. The spec says "XMI bounds take precedence" but also says `max(120, stored_width)`. This is inconsistent — if XMI takes full precedence, the width should be `stored_width` (even if < 120 px). The implementation should use `max(120, stored_width)` only for new diagrams; for loaded diagrams, use `stored_width` as-is.

### 4.3 XMI Precedence Implementation Gap

The spec's pseudo-code (§3.3):

```rust
fn decide_node_bounds(node: &ViewNode, element: &UmlElement) -> Rect {
    if diagram_was_loaded_from_xmi {
        node.bounds  // trust stored
    } else {
        // auto-compute
    }
}
```

**Gap identified:** The `diagram_was_loaded_from_xmi` boolean does not exist in the current codebase. `UmbrelloApp::new()` receives a `loaded: bool` parameter but does **not** store it (only uses it for status message construction). The implementation must:

1. Add `model_loaded_from_xmi: bool` to `UmbrelloApp`
2. Check bounds validity — a freshly-created `ViewNode` with default bounds `Rect::new(50, 50, 120, 60)` should be auto-sized, while a node from XMI with custom bounds `Rect::new(100, 200, 250, 80)` should keep its stored size

The simplest heuristic: track `model_loaded_from_xmi` in the app. If true, all nodes keep their XMI bounds. If false, auto-compute.

### 4.4 Cache Invalidation

The spec says height is recalculated when "element_id changes" or "first rendered after creation." **Missing:** height invalidation when classifier data changes (attributes/operations added or removed). The spec anticipates this in Appendix D question 1 (caching galleys) but doesn't address when to invalidate the cached bounds. For M15, this is acceptable since attributes/operations are static after model load. Future milestones will need explicit invalidation hooks.

### 4.5 Verdict: ✅ CORRECT FORMULA — CLARIFICATION REQUIRED

| Issue | Severity | Resolution |
|-------|----------|------------|
| `model_loaded_from_xmi` not stored | **Condition 1** | Add field to `UmbrelloApp` |
| Width formula conflicts with "XMI takes precedence" | **Condition 2** | For loaded diagrams, use stored width as-is (even if <120px) |
| No invalidation on classifier data change | **Acceptable** | Defer to M16+ when editing is implemented |

---

## 5. Arrowhead Geometry: Mathematically Sound?

### 5.1 Vector Math Verification

The spec uses standard vector operations:

```rust
dir = target_centre - source_centre           // direction vector
unit_dir = dir / dir.length()                  // normalized
perp = vec2(-unit_dir.y, unit_dir.x)           // perpendicular (rotated 90° CCW)
```

The perpendicular is correct: `dot(unit_dir, perp) = unit_dir.x*(-unit_dir.y) + unit_dir.y*unit_dir.x = 0`. ✅

### 5.2 Arrowhead Vertex Computation

**Hollow triangle (Generalization):** placed at target end.

```
tip = target_border_point
left  = tip - unit_dir * 14.0 + perp * 7.0
right = tip - unit_dir * 14.0 - perp * 7.0
```

This creates an isosceles triangle with base length 14.0 (ARROW_HALF_WIDTH × 2) and height 14.0 (ARROW_LENGTH). The base is perpendicular to the direction vector. **Correct.** ✅

**Hollow diamond (Aggregation):** placed at source end.

```
front = source_centre + unit_dir * 8.0   // toward target
back  = source_centre - unit_dir * 8.0   // away from target
left  = source_centre + perp * 8.0
right = source_centre - perp * 8.0
```

This creates a diamond (rotated square) with diagonal lengths 16.0 each. The vertices form a convex polygon: front → left → back → right → front. **Correct.** ✅

**Filled diamond (Composition):** same geometry, filled with `Color32::BLACK`. Uses `egui::Shape::convex_polygon`. **Correct.** ✅

**Open arrow (Dependency):** two lines from tip, no base:

```
left  = tip - unit_dir * 14.0 + perp * 7.0
right = tip - unit_dir * 14.0 - perp * 7.0
// Draw: tip → left, tip → right (no line between left and right)
```

This is the open-V shape. **Correct.** ✅

### 5.3 Border Intersection Accuracy

The spec acknowledges that arrowheads should touch the rectangle **border**, not the centre. It provides two approaches:

1. **Simplified (M15):** `tip = target_centre - unit_dir * (target_half_diagonal + 5px)`
2. **Precise (future):** `rect_border_point()` solving ray-rectangle intersection

**Assessment:** The simplified approach is acceptable for M15. It works well for rectangular nodes at moderate angles. The spec correctly identifies limitations (large nodes at odd angles may show a visible gap between arrow tip and border). The future `rect_border_point()` implementation is provided as reference code and is mathematically correct.

### 5.4 Dashed Line Implementation

The `draw_dashed_line()` helper (§4.11) is correct:

```
dash_length = 8.0 px, gap = 4.0 px
Loop: draw dash segment, advance position by 8+4=12px
Terminates when accumulated position >= line length
```

**Edge case noted but handled:** zero-length segment → early return. ✅

One subtle issue: the dashed line helper ignores arrowhead placement. For edges with arrowheads (Generalization, Dependency), the dashed main line should stop before the arrow tip, not at the tip. The current approach draws dash segments all the way to the target and then draws the arrowhead on top. This works visually (the arrowhead covers the final dash segment's end) but may look slightly messy at high zoom. **Acceptable for M15.**

### 5.5 Edge Summary Table (Cross-Check)

Spec §4.13 defines six combinations. Verified against `AssociationType` enum (exactly 6 variants):

| `AssociationType` variant | Line style | Arrow at | Decoration | Stroke |
|---------------------------|------------|----------|------------|--------|
| `Generalization` | Solid | Target | Hollow △ | 1.5 px BLACK |
| `Realization` | Dashed | Target | Hollow △ | 1.5 px BLACK |
| `Aggregation` | Solid | Source | Hollow ◇ | 1.5 px BLACK |
| `Composition` | Solid | Source | Filled ◆ | 1.5 px BLACK |
| `Dependency` | Dashed | Target | Open V | 1.0 px gray(120) |
| `Association` | Solid | — | None | 1.0 px gray(120) |

**All six `AssociationType` variants are accounted for.** ✅

### 5.6 Missing: Realization/Composition Hybrids

In standard UML, a Realization can have a composition diamond at the source end (a class implementing an interface while containing it). The C++ Umbrello uses separate association type constants for these hybrids. The M7 review explicitly reduced the enum from 12 variants to 6, explicitly rejecting hybrids. The spec correctly follows the simplified model. **Acceptable — aligns with M7 decision.**

### 5.7 Verdict: ✅ MATHEMATICALLY SOUND

The vector math, vertex computation, and dashed line algorithm are all correct. The simplified border intersection is acceptable for M15. The `rect_border_point()` reference implementation is provided for future precision.

---

## 6. Partitioned Node Layout: Zone-by-Zone Review

### 6.1 Zone 0 — Stereotype

**Spec:** Centred, `<<name>>`, 11pt proportional, gray(140), 16px, omitted when absent.  
**Data source:** `element.base().stereotype_id` — resolve stereotype name from model.  
**Verdict:** ✅ Correct. The ElementBase has `stereotype_id: Option<UmlId>`.

**Implementation note:** Resolving a stereotype name requires looking up the stereotype element by ID in the model. The spec does not explicitly describe this lookup, but it is implicit in the data model. For stereotypes hard-coded in the renderer (e.g., `<<interface>>`, `<<enumeration>>`, `<<datatype>>`), the element's `ObjectType` can be used directly without a model lookup.

### 6.2 Zone 1 — Name

**Spec:** Centred, 14pt proportional, bold, BLACK, 20px.  
**Data source:** `element.name()`.  
**Verdict:** ✅ Correct.

### 6.3 Zone 2 — Attributes

**Spec:** Left-aligned, 4px padding, 16px per line. Format: `{visibility_symbol} {name} : {type_name}`.  
**Data source:** `element.classifier_data().attributes`.  
**Verdict:** ✅ Correct.

**Visibility symbol mapping** (verified against `Visibility` enum):

| Spec symbol | `Visibility` variant | Match |
|-------------|---------------------|-------|
| `+` | `Public` | ✅ |
| `#` | `Protected` | ✅ |
| `-` | `Private` | ✅ |
| `~` | `Implementation` | ✅ |

### 6.4 Zone 3 — Operations

**Spec:** Left-aligned, 4px padding, 16px per line. Format: `{visibility_symbol} {name}({params}) : {return_type}`.  
**Data source:** `element.classifier_data().operations`.  
**Verdict:** ✅ Correct.

**Parameter formatting:** `param1_name : param1_type, param2_name : param2_type, ...`  
**Data source:** `operation.parameters`.  
**Verdict:** ✅ Correct. Each `Parameter` has `name` and `type_ref`.

**Return type:** When `return_type` is `unspecified()` (neither `model_id` nor `type_name`), `display_name()` returns `"void"`. The spec should clarify whether `: void` is displayed or suppressed. UML convention is to suppress `: void` return types.

### 6.5 Interface Rendering

**Spec:** Name in italic, forced `<<interface>>` stereotype, operations only (typically no attributes).  
**Verdict:** ✅ Correct. The renderer distinguishes `ModelElement::Interface(_)` from other classifiers.

### 6.6 Enum Rendering

**Spec:** Zone 2 contains literals (not attributes), format `{name}` or `{name} = {value}`.  
**Data source:** `Enum.literals` (the spec says `enum_literals` — minor field name discrepancy).  
**Verdict:** ✅ Correct approach, but **Condition 3**: the implementation must use `Enum.literals` (actual field name), not `enum_literals`.

### 6.7 Datatype Rendering

**Spec:** No compartments by default. If attributes/operations present, render identically to Class.  
**Verdict:** ✅ Correct. Matches UML convention.

### 6.8 Divider Lines

**Spec:** 1px, gray(150), 2px padding above and below, 6px total per divider. Conditionally rendered when zones 2 or 3 are non-empty.  
**Verdict:** ✅ Correct.

### 6.9 Verdict: ✅ LAYOUT ANATOMY CORRECT

All five node types (Class, Interface, Enum, Datatype, Package) have correctly specified zone layouts. The conditional divider rendering is well-defined.

---

## 7. Package Rendering: Ready for M15?

### 7.1 Spec Description

The spec defines a **tabbed** shape for packages:

```
  ┌──────────┐
  │ PackageName  ─────────────────────┐
  │                                    │
  │   (contained elements — future)    │
  │                                    │
  └────────────────────────────────────┘
```

### 7.2 Assessment

**Three concerns:**

1. **Tab geometry blending** — the spec says "The tab blends into the body (no vertical separator between tab and body)." Drawing this correctly requires careful polygon construction: the outer outline must trace the tab and body as a single path. The spec does not provide the exact polygon vertices. **Recommendation:** Implement as a filled polygon + stroke, or use two overlapping rectangles with same-colour fill to visually blend.

2. **Containment rendering deferred** — the spec correctly defers nested element rendering to M16+. For M15, packages show only their name in the tab. The body rectangle is empty.

3. **Dynamic sizing for packages** — the spec says "For Package the height is not auto-computed in M15." This is correct. Packages are containers whose size is determined by their contents (future), not by attribute/operation count.

### 7.3 Verdict: ✅ ACCEPTABLE — DEFERRED COMPLEXITY

The tab rendering is visually important for distinguishing packages from classes. The implementation should use a single rounded polygon or two overlapping rects. The spec's deferral of containment rendering is appropriate.

---

## 8. Performance: Will This Scale?

### 8.1 Spec Claim

> Each frame iterates all nodes and edges, looking up model data. For <100 nodes and <100 edges, this is well under 1ms per frame.

### 8.2 Analysis

**Current M14 hot path** (per frame):
- 1 clone of the diagram struct
- 1 iteration over `diagram.nodes` (IndexMap lookup per node)
- 1 iteration over `diagram.edges` (IndexMap lookup per edge)
- Text layout for each node (name only)

**M15 additional work** (per frame):
- For each node: text layout for stereotype, name, N attributes, M operations (N+M lines)
- For each edge: dashed line segmentation, arrowhead polygon construction
- Text measurement for width decisions

**With caching (§3.4):**
- Node height is cached in `ViewNode.bounds.height` and not recalculated every frame
- Text galleys could be cached alongside ViewNode (Appendix D question 1)

**Quantitative estimate for 50 nodes, 80 edges:**
- M14: ~50 text layouts (names), ~80 line segments → ~0.3ms
- M15 (no caching): ~200+ text layouts (names + 3 attrs + 2 ops avg per node), ~80 arrowhead constructions → ~1.2ms
- M15 (with height caching, galley caching noted as future): ~50 text layouts, ~80 line segments + arrowheads → ~0.5ms

At 60 FPS, the budget is 16.67ms per frame. M15 is well within budget even without caching. With caching, it is negligible (<5% of frame budget).

### 8.3 Memory

The spec does not add significant memory pressure. Arrowhead geometry is ephemeral (constructed per frame). The only persistent addition is cached text galleys if implemented (Appendix D recommendation) — at ~100 bytes per attribute/operation line, this is <10 KB for a 100-node diagram.

### 8.4 Verdict: ✅ PERFORMANCE IS FINE FOR M15

Even the worst-case (no caching, 200 nodes × 5 lines each = 1000 text layouts) falls under 5ms. Caching (recommended in Appendix D) makes this trivial.

---

## 9. Text Truncation: Sufficient Approach?

### 9.1 Spec Approach

Two approaches presented:
1. **Clip rect** (primary) — set egui clip rect to node bounds; text is visually clipped
2. **Manual elision** (secondary) — measure text width, binary search for truncation with "…"

The spec prefers clip rect for M15 as it is simpler.

### 9.2 Assessment

The clip rect approach is **correct but crude**. It will:
- ✅ Prevent text from overflowing the node box
- ❌ Cut text mid-character, not at word boundary
- ❌ Not show "…" to indicate truncation

For M15 this is acceptable. The spec acknowledges the limitations.

**Implementation concern:** egui's `Painter` does not expose a `set_clip_rect()` method directly on the painter. The spec suggests `painter.add(shape.clipped(clip_rect, shape))` as an alternative. The implementation will need to verify which egui API is available in the version used by umbrello-rs.

### 9.3 Recommendation (Non-blocking)

Use the `Galley::elide()` method if available in the egui version:

```rust
let galley = painter.layout_no_wrap(...);
let elided = galley.elide(max_width);
```

If not available, clip rect is sufficient for M15.

### 9.4 Verdict: ✅ ACCEPTABLE FOR M15

---

## 10. Implementation Complexity: Manageable Scope?

### 10.1 Spec Estimate

> ~300 additional lines in app.rs (from 296 to ~600), 6 helper functions, all changes additive.

### 10.2 Line Count Verification

| Component | Estimated lines | Basis |
|-----------|----------------|-------|
| `draw_partitioned_node()` | ~80 | 5 node types × switch arms + zone layout |
| `format_attribute_line()` | ~15 | Visibility mapping + type resolution |
| `format_operation_line()` | ~20 | Parameter list formatting |
| `text_width()` | ~10 | Delegate to painter layout |
| `draw_dashed_line()` | ~20 | §4.11 pseudocode |
| `draw_arrowhead()` | ~60 | 6 association type cases × 2 ends |
| `perpendicular()` | ~3 | One-line expression |
| `rect_border_point()` | ~20 | §4.2 pseudocode |
| `decide_node_bounds()` | ~25 | XMI precedence logic |
| Updated `render_canvas()` | ~50 | Replace simple loop with partition logic |
| **Total** | **~303** | |

The 300-line estimate is **reasonable**. With the optional `canvas.rs` extraction, the total could be split into `app.rs` (~400 lines) and `canvas.rs` (~200 lines).

### 10.3 Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| egui API limitations (clip rect, dashed lines) | Low | Medium | Fallback approaches already described |
| Font metrics inaccuracies | Low | Low | 1-2px errors acceptable for diagram rendering |
| Arrowhead positioning errors at extreme angles | Low | Low | Simplified approach works for rectangular nodes |
| Borrow checker issues with `self.model` + `ui.painter()` | Medium | Medium | M14 already solved this with two-phase collection (data first, paint second) |

The M14 implementation already handles the borrow checker split (`node_visuals` vec collected before `painter` scope). M15 extends this pattern with additional data collection.

### 10.4 Verdict: ✅ SCOPE IS APPROPRIATE

The ~300 lines of additive changes is well within a single milestone. No refactoring of M14 code is required — only replacement of the simple rectangle rendering with partitioned rendering.

---

## 11. Implementation Discrepancies and Gaps

### 11.1 Pseudo-Code vs Actual APIs

| Spec pseudo-code | Actual API | Severity |
|-----------------|------------|----------|
| `model.elements.find(node.element_id)` | `model.get(node.element_id)` → `Option<&ModelElement>` | Minor — different method name |
| `model.relationships.get(edge.relationship_id)` | `model.get(edge.relationship_id)` then match on `ModelElement::Relationship` | Minor — relationships are in `elements`, not a separate map |
| `edge_relationship(edge, model)` | Must be implemented; no existing method | Minor — new helper |
| `diagram_was_loaded_from_xmi` | Must be added to `UmbrelloApp` | **Condition 1** |
| `enum_literals` | `Enum.literals` (actual field name) | **Condition 3** |

### 11.2 Function Signature Issue

The spec's pseudo-code for `render_canvas`:

```rust
fn render_canvas(&self, ctx: &egui::Context, model: &UmlModel) {
```

The actual M14 signature is:

```rust
fn render_canvas(&mut self, ui: &mut egui::Ui) {
```

The `&mut self` is required because drag state (`drag_node_id`, `drag_start_pos`) is mutated during rendering (drag start/stop detection). The `model` is accessed via `self.model`, not passed as a parameter. **The implementation should keep the existing signature** and access `self.model` internally.

### 11.3 Rendering Order Clarification

The spec specifies rendering order (§6.5): fill → dividers → text → border → selection → edges → arrowheads. This is correct for visual layering. The current M14 draws fill + border + text together, then edges. M15 extends this with compartment-specific text and dividers between fill and border.

### 11.4 Colour Consistency

The spec's colour reference (Appendix B) uses different fill colours than M14:

| Element | M14 fill | Spec fill |
|---------|----------|-----------|
| Class | `from_rgb(180, 210, 255)` (blue) | `from_rgb(255, 255, 230)` (light yellow) |
| Interface | `from_rgb(180, 255, 210)` (green) | `from_rgb(230, 255, 230)` (light green) |
| Enum | `from_rgb(255, 210, 180)` (orange) | `from_rgb(255, 240, 210)` (light orange) |
| Datatype | `from_rgb(210, 180, 255)` (purple) | `from_rgb(210, 230, 255)` (light blue) |

The spec colours are **more standard UML** (class=yellow, interface=green, enum=orange, datatype=light blue). M14 used placeholder colours. **Recommendation:** Use spec colours. This is a breaking visual change but appropriate.

---

## 12. Test Coverage Adequacy

### 12.1 Spec Test Plan

The spec defines 12 manual visual test cases (§7.1), 6 unit test functions (§7.2), and one CI integration step (§7.3).

### 12.2 Manual Test Assessment

The 12 visual test cases cover all six edge types, all four node types, undo/redo, dynamic height, and XMI layout preservation. **Coverage is adequate for a visual rendering feature.**

### 12.3 Unit Test Assessment

The 6 unit test functions target pure helper functions. However:

1. **`format_attribute_line()`** — testable (pure string formatting) ✅
2. **`format_operation_line()`** — testable (pure string formatting) ✅
3. **`rect_border_point()`** — testable (pure math) ✅
4. **`perpendicular()`** — testable (pure math) ✅
5. **`decide_node_bounds()`** — testable only if `model_loaded_from_xmi` is parameterized ✅
6. **`draw_dashed_line()`** — **not testable without an egui context** ❌

The `draw_dashed_line()` function depends on `egui::Painter` which requires an active egui context. It cannot be unit tested in a pure Rust test. The spec acknowledges this limitation in §7.2 "no panic on zero-length segment" — this is a minimal test that can be done with a mock painter, but egui does not provide mock painters.

### 12.4 Missing Test: Arrowhead Drawing Verification

Drawing functions (`draw_arrowhead()`, `draw_dashed_line()`) are inherently visual and cannot be unit tested without a rendering context. This is acceptable — visual correctness is tested through the manual test cases.

### 12.5 Rust Test Harness

The spec notes that CI integration should include `cargo test` in the `rust-rewrite` directory. The current project has:
- C++ tests in `unittests/` (Qt Test framework)
- Rust tests in `rust-rewrite/` (cargo test)

The M15 implementation should add `#[cfg(test)] mod tests` to `app.rs` (or `canvas.rs` if extracted) for the testable pure functions.

### 12.6 Verdict: ✅ ADEQUATE FOR M15 — with one condition

| Issue | Condition |
|-------|-----------|
| `draw_dashed_line` not unit-testable | Acceptable — visual testing covers this |
| No rendering regression tests | Acceptable — screenshots are M16+ work |
| `#[cfg(test)]` tests should be added for pure helpers | **Condition 4**: Add unit tests for `format_attribute_line`, `format_operation_line`, `rect_border_point`, `perpendicular`, `decide_node_bounds` |

---

## 13. Summary of Required Conditions

### Mandatory Conditions (must be satisfied before approval)

| # | Condition | Rationale |
|---|-----------|-----------|
| **C1** | Add `model_loaded_from_xmi: bool` field to `UmbrelloApp` and set it from the existing `loaded` constructor parameter | `decide_node_bounds()` needs this flag to implement XMI precedence. Currently the constructor receives `loaded: bool` but discards it after status message construction. |
| **C2** | For XMI-loaded diagrams, use stored width as-is (do not enforce `max(120, stored_width)` minimum) | The spec's XMI precedence rule conflicts with the `max(120, stored_width)` width formula. The minimum width should apply only to new diagrams. |
| **C3** | Use `Enum.literals` (actual field name) not `enum_literals` (spec's name) | The actual struct field is `pub literals: Vec<EnumLiteral>`. The spec incorrectly names it `enum_literals` in §2.6. |
| **C4** | Add `#[cfg(test)]` unit tests for pure helper functions | Testable functions: `format_attribute_line`, `format_operation_line`, `rect_border_point`, `perpendicular`, `decide_node_bounds`. |
| **C5** | Pass `Option<&UmlModel>` to `TypeReference::display_name()` in attribute/operation formatters | The `display_name` method requires model access to resolve `model_id`-based type references to names. The formatters must receive the model reference. |

### Recommendations (non-blocking improvements)

| # | Recommendation | Rationale |
|---|---------------|-----------|
| **R1** | Extract rendering helpers into `apps/umbrello/src/canvas.rs` when `app.rs` exceeds ~600 lines | Better code organization; aligns with spec §6.2 which plans this extraction. |
| **R2** | Cache text galleys alongside `ViewNode` to avoid per-frame text layout | Addressed in Appendix D question 1; reduces frame time for diagrams > 50 nodes. |
| **R3** | Use `Galley::elide()` for text truncation instead of clip rect if available | Shows "…" for truncated text, improving user experience. Fall back to clip rect if egui version doesn't support it. |

---

## 14. Final Recommendation

### APPROVE WITH CONDITIONS

The specification `ui_rich_rendering_spec_v1.md` is **architecturally sound, mathematically correct, and appropriately scoped** for Milestone 15.

**Strengths:**
- Complete zone layout anatomy for all five node types (Class, Interface, Enum, Datatype, Package)
- Correct vector math for all six arrowhead types, matching the six `AssociationType` variants
- Clear XMI precedence rule preserving user layouts from loaded files
- Proper separation of rendering (app crate) from domain model (uml-core)
- Realistic line-count estimates (~300 lines) for the scope
- Appropriate deferral of complex features (nesting, zoom scaling, galley caching) to M16+

**Five mandatory conditions** (C1–C5) must be satisfied during implementation. These are minor integration adjustments — they do not require changes to the specification document itself.

**Three recommendations** (R1–R3) are quality-of-life improvements for the implementation.

**Implementation should proceed** with these conditions checked during implementation, not in a revised specification document.

---

*End of review.*
