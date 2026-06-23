# Rich UML Canvas Rendering — Umbrello-RS Milestone 15

**Status:** Draft v1  
**Target Milestone:** M15  
**Dependencies:** M14 (basic canvas with ViewNode/ViewEdge rendering)

---

## 1. Current State (M14)

The canvas in `apps/umbrello/src/app.rs` currently:

- Draws a solid‑colored rectangle per `ViewNode` with the element name centred
- Draws straight line segments from node centre to node centre
- Supports click‑to‑select and drag‑to‑move
- Undo/Redo via `Command` history

It does **not**:

- Show attributes or operations inside class boxes
- Draw compartment separator lines
- Draw UML‑specific arrowheads (hollow triangle, filled diamond, open arrow, etc.)
- Dynamically size nodes based on content
- Use font metrics for layout calculation

---

## 2. Partitioned Class Widget — Node Layout Anatomy

A UML **Class** box has three vertical zones separated by horizontal divider lines:

```
  ┌──────────────────────────────────┐
  │           <<stereotype>>          │  Zone 0 — Stereotype (optional, centred, guillemets)
  │           ClassName               │  Zone 1 — Name (bold, centred)
  ├──────────────────────────────────┤  ──── Divider line (1 px, dark gray)
  │  - privateAttr : int              │  Zone 2 — Attributes (left‑aligned)
  │  + publicAttr  : String           │      each line: {visibility} {name} : {type}
  │  # protectedAttr : double         │
  ├──────────────────────────────────┤  ──── Divider line (1 px, dark gray)
  │  + method1() : void               │  Zone 3 — Operations (left‑aligned)
  │  - method2(x: int) : bool         │      each line: {visibility} {name}({params}) : {return}
  └──────────────────────────────────┘
```

### 2.1 Zone 0 — Stereotype

If the element has a stereotype (`stereotype_id` is `Some`), render the stereotype name centred in guillemets:

```
<<stereotype_name>>
```

| Property      | Value                     |
|---------------|---------------------------|
| Font          | Proportional 11 pt        |
| Weight        | Normal (not bold)         |
| Colour        | Gray (`Color32::from_gray(140)`) |
| Height        | ~16 px                    |
| Alignment     | Horizontal centre         |

When there is **no stereotype**, the zone is omitted entirely (0 px contribution to total height).

### 2.2 Zone 1 — Name

The element name, centred, bold:

| Property      | Value                     |
|---------------|---------------------------|
| Font          | Proportional 14 pt        |
| Weight        | Bold                      |
| Colour        | Black (`Color32::BLACK`)  |
| Height        | ~20 px                    |
| Alignment     | Horizontal centre         |

### 2.3 Zone 2 — Attributes

Each attribute occupies one line, left‑aligned with **4 px** left padding. Each line contributes ~16 px.

**Format:**

```
{visibility_symbol} {name} : {type_name}
```

**Visibility symbols:**

| Symbol | Visibility |
|--------|------------|
| `+`    | Public     |
| `#`    | Protected  |
| `-`    | Private    |
| `~`    | Implementation (package) |

**`type_name` resolution:**
- If the `TypeReference` contains a direct `type_name`, use it.
- If the `TypeReference` contains a `model_id`, look up the referenced `UmlElement` in the model and use its `name`.

**Zero attributes:** When the classifier has no attributes, this zone is omitted (saves space — contributes 0 px).

### 2.4 Zone 3 — Operations

Each operation occupies one line, left‑aligned with **4 px** left padding. Each line contributes ~16 px.

**Format:**

```
{visibility_symbol} {name}({params}) : {return_type}
```

**`{params}` expansion:**

```
param1_name : param1_type, param2_name : param2_type, …
```

Comma‑separated, no trailing comma. Each parameter's `type_name` follows the same `TypeReference` resolution rules as attributes.

**Zero operations:** When the classifier has no operations, this zone is omitted.

### 2.5 Interface Rendering

An Interface node reuses the same three‑zone layout as Class, with these differences:

- The name zone is **italic** (not bold).
- The stereotype zone always shows `<<interface>>` as its first line (stereotype of the model element or hard‑coded if absent).
- The name is rendered in **italic**.

```
  ┌──────────────────────────────────┐
  │          <<interface>>            │  Zone 0 — forced <<interface>>
  │          *Paintable*              │  Zone 1 — Name (italic, centred)
  ├──────────────────────────────────┤
  │  + draw(g: Graphics) : void       │  Zone 2 — Operations only (typically no attributes)
  └──────────────────────────────────┘
```

> UML interfaces conventionally have operations but no attributes; the renderer respects whatever the model contains.

### 2.6 Enum Rendering

An Enum node shows:

```
  ┌──────────────────────────────────┐
  │          <<enumeration>>          │  Zone 0 — Stereotype (optional)
  │            Status                 │  Zone 1 — Name (bold, centred)
  ├──────────────────────────────────┤
  │  PENDING                          │  Zone 2 — Enumeration literals
  │  ACTIVE  = 1                      │      each line: {name} [= {value}]
  │  COMPLETED                        │
  └──────────────────────────────────┘
```

**Rules:**
- `<<enumeration>>` stereotype is shown if the element has a stereotype, or if none is set it may be hard‑coded (product decision).
- Zone 2 contains **enumeration literals** (from `enum_literals`), *not* attributes or operations.
- Each literal is one line, left‑aligned with 4 px padding: `{name}` or `{name} = {value}` if the literal has a value.
- If both attributes *and* literals exist (unusual but valid UML), prefer literals in the middle zone and operations in the bottom zone.

### 2.7 Datatype Rendering

A Datatype node shows:

```
  ┌──────────────────────────────────┐
  │           <<datatype>>            │  Zone 0 — Stereotype (hard‑coded or from model)
  │            DateTime               │  Zone 1 — Name (bold, centred)
  └──────────────────────────────────┘
```

- No compartments (datatypes rarely carry attributes/operations in practice).
- If attributes or operations *are* present, they are rendered in zones 2 and 3 identically to Class.

### 2.8 Package Rendering

A Package node uses a **tabbed** shape:

```
  ┌──────────┐
  │ PackageName  ─────────────────────┐
  │                                    │
  │   (contained elements — future)    │
  │                                    │
  └────────────────────────────────────┘
```

| Element      | Style                              |
|--------------|------------------------------------|
| Top tab      | Name centred in a smaller rectangle (~20 px tall) protruding from the main body |
| Main body    | Larger rectangle below the tab      |
| Stereotype   | Rendered inside the tab if present  |
| Compartments | Not rendered for M15 (future nested rendering) |

**Tab geometry:**

```
tab_width  = text_width(PackageName) + 16 px padding
tab_x      = node_bounds.min.x
tab_y      = node_bounds.min.y
tab_height = 20 px

body_x      = node_bounds.min.x
body_y      = node_bounds.min.y + tab_height
body_width  = max(tab_width, node_bounds.width())
body_height = node_bounds.height() - tab_height
```

The tab blends into the body (no vertical separator between tab and body). The outer outline encompasses both tab and body.

### 2.9 Divider Lines

Horizontal lines drawn between compartments:

| Property      | Value                        |
|---------------|------------------------------|
| Primitive     | `painter.line_segment()`     |
| Width         | 1 px                         |
| Colour        | Dark gray (`Color32::from_gray(150)`) |
| Padding above | 2 px                         |
| Padding below | 2 px                         |
| Total height  | 6 px per divider (2 + 1 + 2 + 1 spare) |

In the layout calculation each divider contributes **6 px** to the total height.

---

## 3. Dynamic Node Sizing

In M14, nodes use fixed dimensions from the XMI file (or defaults). In M15, node height is calculated dynamically based on content.

### 3.1 Height Formula

```
node_height = top_padding
            + stereotype_height    (0 if absent)
            + name_height
            + bottom_padding
            + divider              (6 px  — omitted if zones 2+3 are both absent)
            + attributes_height    (0 if absent)
            + divider              (6 px  — omitted if zone 3 is absent)
            + operations_height    (0 if absent)
```

Where numeric values are:

| Component            | Value     |
|----------------------|-----------|
| `top_padding`        | 8 px      |
| `stereotype_height`  | 16 px     |
| `name_height`        | 20 px     |
| `bottom_padding`     | 4 px      |
| `divider`            | 6 px      |
| `attributes_height`  | 16 × N    |
| `operations_height`  | 16 × N    |

For **Enum** the formula is the same but `attributes_height` is replaced by `literals_height = 16 × N` and `operations_height` is shown only if non‑empty.

For **Package** the height is *not* auto‑computed in M15 (the user's stored bounds are kept). Future milestones may introduce containment‑driven sizing.

### 3.2 Width

```
node_width = max(120 px, stored_width_from_xmi)
```

If the stored width is larger than 120 px, the stored value is retained. Texts that exceed the width are **clipped** by the painter clip rectangle (see §5).

### 3.3 Precedence Rule

> **For XMI‑loaded diagrams:** the stored bounds from the XMI file take precedence over auto‑calculated sizes. Auto‑calculation is applied **only** for newly created diagrams or when a node is first added to a diagram. This preserves the user's carefully arranged layout.

Implementation sketch:

```rust
fn decide_node_bounds(node: &ViewNode, element: &UmlElement) -> Rect {
    if diagram_was_loaded_from_xmi {
        // trust the stored geometry
        node.bounds
    } else {
        // auto‑compute based on content
        let auto_height = compute_node_height(element);
        Rect::from_min_size(node.bounds.min, vec2(node.bounds.width().max(120.0), auto_height))
    }
}
```

### 3.4 Recalculation Trigger

The height is recalculated whenever:

1. The node's `element_id` changes (new element assigned).
2. The node is first rendered after creation.
3. The diagram is exported / printed (re‑layout for target medium).

Height is **not** recalculated on every frame — the result is cached in the node's `bounds` and only updated when the model changes.

---

## 4. Semantic Edge Engine — Arrowhead Mathematics

Each relationship type has a distinct visual representation. The edge connects the source node to the target node, and arrowheads are drawn at the appropriate end(s).

### 4.1 Line from Source to Target

```
dir = target_centre - source_centre
len = dir.length()
unit_dir = dir / len
perp = vec2(-unit_dir.y, unit_dir.x)
```

The arrow tip is placed at the **intersection** of the line with the target node's rectangle boundary, not at the centre. For M15 we approximate:

```
tip = target_centre - unit_dir * (target_half_diagonal + 5 px)
```

Where `target_half_diagonal = max(target_width, target_height) / 2`. This ensures the tip touches the rectangle edge visually.

### 4.2 Computing Rectangle‑Edge Intersection (Reference)

For a more precise implementation (future), compute the intersection of a ray with an axis‑aligned rectangle:

```rust
/// Returns the point on `rect` boundary along the ray from `inside` toward `outside`.
fn rect_border_point(rect: Rect, inside: Pos2, outside: Pos2) -> Pos2 {
    let dir = outside - inside;
    // t values for intersection with each of the 4 edges
    let mut t_min = f32::INFINITY;
    // left edge
    if dir.x != 0.0 { let t = (rect.left() - inside.x) / dir.x; if t > 0.0 { t_min = t_min.min(t); } }
    // right edge
    if dir.x != 0.0 { let t = (rect.right() - inside.x) / dir.x; if t > 0.0 { t_min = t_min.min(t); } }
    // top edge
    if dir.y != 0.0 { let t = (rect.top() - inside.y) / dir.y; if t > 0.0 { t_min = t_min.min(t); } }
    // bottom edge
    if dir.y != 0.0 { let t = (rect.bottom() - inside.y) / dir.y; if t > 0.0 { t_min = t_min.min(t); } }
    inside + dir * t_min
}
```

M15 may use this or the simpler approximation above — either is acceptable provided the arrowhead visibly attaches to the node border.

### 4.3 Perpendicular Vector Helper

```rust
fn perpendicular(dir: egui::Vec2) -> egui::Vec2 {
    egui::vec2(-dir.y, dir.x)
}
```

### 4.4 Arrow Type Constants

| Constant             | Value  |
|----------------------|--------|
| `ARROW_LENGTH`       | 14.0   |
| `ARROW_HALF_WIDTH`   | 7.0    |
| `DIAMOND_SIZE`       | 16.0   |
| `DIAMOND_HALF_WIDTH` | 8.0    |

### 4.5 Hollow Triangle (Generalization)

```
     triangle_apex = tip

     triangle_left  = tip - dir * ARROW_LENGTH + perp * ARROW_HALF_WIDTH
     triangle_right = tip - dir * ARROW_LENGTH - perp * ARROW_HALF_WIDTH
```

```
                            ──── target node ────
                              │                │
                              │                │
                    ┌─────────▼                │
                   ╱           ╲               │
                  ╱             ╲              │
                 ╱               ╲             │
                ╱                 ╲            │
               ╱                   ╲           │
              ╱                     ╲          │
             ╱                       ╲         │
            ╱                         ╲        │
           ╱                           ╲       │
          ╱                             ╲      │
         ╱                               ╲     │
        ╱                                 ╲    │
       ╱                                   ╲   │
      ╱                                     ╲  │
     ╱                                       ╲ │
    ╱                                         ╲│
   ╱                                           │
  ╱                                           ╱
 ╱                                           ╱
╱                                           ╱
───────────────────────────────────────────
                    ▲
                    │
          source node
```

**Drawing:** Three `line_segment` calls, **no fill** (hollow). Stroke: solid black, 1.5 px.

```
painter.line_segment([triangle_left,  triangle_apex], stroke);
painter.line_segment([triangle_right, triangle_apex], stroke);
painter.line_segment([triangle_left,  triangle_right], stroke);
```

### 4.6 Hollow Triangle, Dashed Main Line (Realization)

Same triangle geometry as §4.5. The **main line** from source → target is dashed (see §4.11). The triangle itself is drawn solid (not dashed). Stroke: solid black, 1.5 px.

### 4.7 Hollow Diamond (Aggregation)

Drawn at the **source** end of the line (not the target).

```
                              ──── target node ────
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                ▼                │
                        ──── target node ────


                                    ◇          ── hollow diamond at source
                                   ╱ ╲
                                  ╱   ╲
                                 ╱     ╲
                                ╱       ╲
                               ╱         ╲
                              ╱           ╲
                             ╱             ╲
                            ╱               ╲
                           ╱                 ╲
                          ╱                   ╲
                         ╱                     ╲
                        ╱                       ╲
                       ╱                         ╲
                      ╱                           ╲
                     ╱                             ╲
                    ╱                               ╲
 ──── source node ─▶                                 ╲
   │                │  diamond_front                  ╲
   │                │                                   ╲
   │                │                                    ╲
   ──── source node ──────────────────────────────────────
```

**Diamond vertices:**

```
diamond_front = source_centre + unit_dir * DIAMOND_SIZE * 0.5
diamond_back  = source_centre - unit_dir * DIAMOND_SIZE * 0.5
diamond_left  = source_centre + perp * DIAMOND_HALF_WIDTH
diamond_right = source_centre - perp * DIAMOND_HALF_WIDTH
```

**Drawing:** Four line segments in a loop, **no fill** (hollow). Stroke: solid black, 1.5 px.

```
for [a, b] in [
    [diamond_front, diamond_left],
    [diamond_left, diamond_back],
    [diamond_back, diamond_right],
    [diamond_right, diamond_front],
] {
    painter.line_segment([a, b], stroke);
}
```

### 4.8 Filled Diamond (Composition)

Same diamond geometry as §4.7. The diamond is **filled** with black (or the foreground colour) instead of being hollow. The main line is solid.

```
painter.add(egui::Shape::convex_polygon(
    vec![diamond_front, diamond_left, diamond_back, diamond_right],
    Color32::BLACK,   // fill
    Stroke::new(1.5, Color32::BLACK),  // outline
));
```

```
                              ──── target node ────
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                │                │
                                ▼                │
                        ──── target node ────

                                    ◆          ── filled diamond at source
                                   ╱ ╲
                                  ╱   ╲
                                 ╱     ╲
                                ╱       ╲
                               ╱         ╲
                              ╱           ╲
                             ╱             ╲
                            ╱               ╲
                           ╱                 ╲
                          ╱                   ╲
                         ╱                     ╲
                        ╱                       ╲
                       ╱                         ╲
                      ╱                           ╲
                     ╱                             ╲
                    ╱                               ╲
 ──── source node ─▶                                 ╲
   │                │                                   ╲
   │                │                                    ╲
   ──── source node ──────────────────────────────────────
```

### 4.9 Open Arrow (Dependency)

Drawn at the **target** end. The main line is dashed. The arrowhead is two line segments forming an open V (no base line):

```
     open_left  = tip - dir * ARROW_LENGTH + perp * ARROW_HALF_WIDTH
     open_right = tip - dir * ARROW_LENGTH - perp * ARROW_HALF_WIDTH
```

```
                              ──── target node ────
                                │                │
                                │              ╱ │
                                │             ╱  │
                                │            ╱   │
                                │           ╱    │
                                │          ╱     │
                                │         ╱      │
                                │        ╱       │
                                │       ╱        │
                                │      ╱         │
                                │     ╱          │
                                │    ╱           │
                                │   ╱            │
                                │  ╱             │
                                │ ╱              │
                                │╱               │
                                ╱                │
                               ╱                 │
                              ╱                  │
                             ╱                   │
                            ╱                    │
                           ╱                     │
                          ╱          (dashed     │
                         ╱            main line) │
                        ╱                        │
                       ╱                         │
                      ╱                          │
                     ╱                           │
                    ╱                            │
                   ╱                             │
                  ╱                              │
                 ╱                               │
                ╱                                │
               ╱                                 │
              ╱                                  │
             ╱                                   │
            ╱                                    │
           ╱                                     │
          ╱                                      │
         ╱                                       │
        ╱                                        │
       ╱                                         │
      ╱                                          │
     ╱                                           │
    ╱                                            │
   ╱                                             │
  ╱                                              │
 ╱                                               │
╱                                                │
──────────────────────────────────────────────────
                        ▲
                        │
                  source node
```

**Drawing:** Two `line_segment` calls (no base segment):

```
painter.line_segment([tip, open_left],  stroke);
painter.line_segment([tip, open_right], stroke);
```

Stroke: dashed gray, 1.0 px for the main line; solid gray, 1.0 px for the arrow wings.

### 4.10 Plain Line (Association)

No arrowhead. Just a solid line from `source_border_point` to `target_border_point`. No decoration.

```
painter.line_segment([source_pt, target_pt], stroke);
```

Stroke: solid gray, 1.0 px.

### 4.11 Dashed Line Helper

egui does not expose a built‑in dashed line primitive. Implement:

```rust
fn draw_dashed_line(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    stroke: egui::Stroke,
) {
    let dir = end - start;
    let len = dir.length();
    if len < 0.001 {
        return;
    }
    let unit = dir / len;
    let dash_len = 8.0;
    let gap_len = 4.0;
    let mut pos = 0.0;
    while pos < len {
        let seg_end = (pos + dash_len).min(len);
        painter.line_segment(
            [start + unit * pos, start + unit * seg_end],
            stroke,
        );
        pos += dash_len + gap_len;
    }
}
```

### 4.12 Determining the Relationship Type

For each `ViewEdge`:

```rust
fn edge_relationship(edge: &ViewEdge, model: &UmlModel) -> Option<&Relationship> {
    model.relationships.get(edge.relationship_id)
}
```

If the relationship is not found, fall back to `AssociationType::Association` (plain line).

The edge's visual is then chosen based on the relationship's `kind`:

| `AssociationType`     | Main Line  | Arrowhead              | Arrow End   | Stroke              |
|-----------------------|------------|------------------------|-------------|---------------------|
| `Generalization`      | Solid      | Hollow triangle        | Target      | 1.5 px black        |
| `Realization`         | Dashed     | Hollow triangle        | Target      | 1.5 px black        |
| `Aggregation`         | Solid      | Hollow diamond         | Source      | 1.5 px black        |
| `Composition`         | Solid      | Filled diamond         | Source      | 1.5 px black        |
| `Dependency`          | Dashed     | Open arrow (V)         | Target      | 1.0 px gray         |
| `Association`         | Solid      | None                   | —           | 1.0 px gray         |

### 4.13 Edge Summary Table with ASCII

| Type            | Source End                  | Line Style   | Target End                   |
|-----------------|-----------------------------|--------------|------------------------------|
| Generalization  | —                           | Solid        | ◁ (hollow triangle)          |
| Realization     | —                           | ╌╌╌ dashed   | ◁ (hollow triangle)          |
| Aggregation     | ◇ (hollow diamond)          | Solid        | —                            |
| Composition     | ◆ (filled diamond)          | Solid        | —                            |
| Dependency      | —                           | ╌╌╌ dashed   | < (open V, no base)          |
| Association     | —                           | Solid        | —                            |

---

## 5. Text Truncation Strategy

Attribute and operation signatures can be long. For M15, use a **clip rect** approach:

```rust
// Before drawing text into a node compartment:
painter.set_clip_rect(node_bounds);

// Draw text — egui will clip anything outside the bounds
painter.text(/* … */);

// Restore clip rect to full canvas after drawing (or use scoped clip).
```

Alternatively, if the egui `Painter` does not support `set_clip_rect` directly, use `painter.add(shape.clipped(clip_rect, shape))`.

**Simplest M15 approach:** Use `Label::new(text).truncate()` within `ui.put(rect, …)` or pre‑truncate by measuring with a `Fonts` handle:

```rust
fn elide_text(painter: &egui::Painter, text: &str, max_width: f32) -> String {
    let galley = painter.layout_no_wrap(text.to_string(), egui::FontId::proportional(14.0), Color32::BLACK);
    if galley.rect.width() > max_width {
        // binary search for truncation with "…"
        // or use galley.elide()
    }
    text.to_string()
}
```

For M15 the clip‑rect approach is preferred — it is simple, works with any text length, and avoids the complexity of manual elision. The only downside is that truncated text does not show "…", but this is acceptable for an initial implementation.

---

## 6. Implementation Plan

### 6.1 Files Modified

| File                          | Change                                                       |
|-------------------------------|--------------------------------------------------------------|
| `apps/umbrello/src/app.rs`    | Main `render_canvas()` rewritten; helpers added              |
| `apps/umbrello/src/lib.rs`    | Possibly re‑export new public helpers (if extracted)         |
| `apps/umbrello/src/canvas.rs` | **(new)** Optional extraction of canvas drawing helpers      |

### 6.2 New Helper Functions in `app.rs`

All functions live in `app.rs` to minimise refactoring. If the file grows beyond ~1000 lines, extraction into a `canvas.rs` module should be considered.

```rust
// ─── Node rendering ──────────────────────────────────────────

/// Draw a single partitioned UML node (class, interface, enum, datatype, package).
fn draw_partitioned_node(
    painter: &egui::Painter,
    node: &ViewNode,
    element: &UmlElement,
    is_selected: bool,
) { … }

/// Format an attribute as a single display line.
fn format_attribute_line(attr: &UmlAttribute) -> String { … }

/// Format an operation as a single display line.
fn format_operation_line(op: &UmlOperation) -> String { … }

/// Return the font-advance width of a string (for elision decisions).
fn text_width(painter: &egui::Painter, text: &str, font_id: &egui::FontId) -> f32 { … }

// ─── Edge rendering ──────────────────────────────────────────

/// Draw a dashed line segment.
fn draw_dashed_line(painter: &egui::Painter, start: Pos2, end: Pos2, stroke: Stroke) { … }

/// Draw the appropriate arrowhead at a given line end.
fn draw_arrowhead(
    painter: &egui::Painter,
    tip: Pos2,
    dir: Vec2,
    assoc_type: AssociationType,
    at_source: bool,  // true for aggregation/composition diamond at source
) { … }

/// Vector perpendicular to `v`.
fn perpendicular(v: egui::Vec2) -> egui::Vec2 { … }

/// Compute where the ray from `inside` to `outside` exits `rect`.
fn rect_border_point(rect: Rect, inside: Pos2, outside: Pos2) -> Pos2 { … }
```

### 6.3 Updated `render_canvas()` Pseudocode

```
fn render_canvas(&self, ctx: &egui::Context, model: &UmlModel) {
    let (response, painter) = /* allocate canvas area */;

    // 1. Draw all nodes
    for view_node in &model.view_nodes {
        if let Some(element) = model.elements.find(view_node.element_id) {
            let bounds = decide_node_bounds(view_node, element);
            // Update bounds in view_node (if dynamic sizing says so)
            // Draw the partitioned node:
            draw_partitioned_node(&painter, view_node, element, is_selected);
        }
    }

    // 2. Draw all edges
    for view_edge in &model.view_edges {
        let source_node = /* lookup by view_edge.source_id */;
        let target_node = /* lookup by view_edge.target_id */;
        let rel = edge_relationship(view_edge, model);

        let source_pt = source_node.bounds.center();
        let target_pt = target_node.bounds.center();
        let dir = (target_pt - source_pt).normalized();
        let perp = perpendicular(dir);

        // Choose line style and colours
        let (line_style, arrow_kind, stroke) = style_for(rel.kind());

        // Draw main line
        match line_style {
            LineStyle::Solid => painter.line_segment([source_pt, target_pt], stroke),
            LineStyle::Dashed => draw_dashed_line(painter, source_pt, target_pt, stroke),
        }

        // Draw arrowhead
        if arrow_kind.has_target_decoration() {
            let tip = rect_border_point(target_node.bounds, source_pt, target_pt);
            draw_arrowhead(painter, tip, dir, rel.kind(), false);
        }
        if arrow_kind.has_source_decoration() {
            let source_tip = rect_border_point(source_node.bounds, target_pt, source_pt);
            draw_arrowhead(painter, source_tip, -dir, rel.kind(), true);
        }
    }

    // 3. Handle selection & dragging (unchanged from M14)
}
```

### 6.4 No Changes to `uml-core`

All rendering logic stays in `apps/umbrello`. The domain model in `uml-core` (elements, relationships, view nodes/edges) is treated as read‑only data. No new fields or methods are required in the domain layer.

### 6.5 Rendering Order

Items are drawn in this order (back to front):

1. Fill background of node boxes (solid colour per element type).
2. Divider lines.
3. Text (stereotype, name, attributes, operations).
4. Border outline of node boxes.
5. Selection highlight (dashed blue border if selected; re‑use M14 behaviour).
6. Edge lines (solid or dashed).
7. Arrowheads / diamonds on top of edge lines.

This order ensures arrowheads sit cleanly on top of lines and selection feedback is above everything.

---

## 7. Test Plan

### 7.1 Manual Visual Inspection

| Test Case                       | Procedure                                                     | Expected Outcome                                               |
|---------------------------------|---------------------------------------------------------------|----------------------------------------------------------------|
| Class with attributes           | Load `test-RFA.xmi` (has classes with attributes/operations). | Attributes appear in middle compartment, ops in bottom.        |
| Interface rendering             | Load diagram with `<<interface>>` stereotype.                 | Name is italic; stereotype shows `<<interface>>`.              |
| Enum rendering                  | Add an Enum from the toolbox.                                 | Literals appear in middle compartment; no attributes zone.     |
| Generalization arrow            | Load diagram with Generalization edge.                        | Hollow triangle at target end.                                 |
| Realization arrow               | Load diagram with Realization edge.                           | Dashed line + hollow triangle at target.                       |
| Aggregation diamond             | Load diagram with Aggregation edge.                           | Hollow diamond at source end.                                  |
| Composition diamond             | Load diagram with Composition edge.                           | Filled diamond at source end.                                  |
| Dependency arrow                | Load diagram with Dependency edge.                            | Dashed line + open V arrow at target.                          |
| Association plain line          | Load diagram with Association edge.                           | Solid line, no arrowhead.                                      |
| Undo/redo after drag            | Drag a class, undo.                                           | Node snaps back; compartments redraw correctly.                |
| Dynamic height on new diagram   | Create new class diagram, add class with 5 attrs.             | Node height accommodates all 5 attribute lines + dividers.     |
| XMI layout preserved            | Load XMI with manually positioned tall class.                 | Height from XMI is retained, not overwritten by auto‑sizing.   |

### 7.2 Unit Tests (when helpers are extracted into tested modules)

If a `canvas.rs` module is extracted, unit‑test the pure functions:

| Function                    | Test                                              |
|-----------------------------|---------------------------------------------------|
| `format_attribute_line()`   | Visibility symbol mapping, type resolution.         |
| `format_operation_line()`   | Parameter list formatting, return type omission.    |
| `rect_border_point()`       | Ray hits each of the 4 edges.                      |
| `perpendicular()`           | Dot product with original is zero.                  |
| `decide_node_bounds()`      | XMI precedence over auto‑calc.                     |
| `draw_dashed_line()`        | No panic on zero‑length segment.                   |

### 7.3 CI Integration

The existing test infrastructure (`unittests/`) is C++ based and does not cover Rust modules. M15 should add a Rust test harness if not already present:

```shell
cd rust-rewrite && cargo test
```

At minimum, the `apps/umbrello` crate should have doc‑tests on the helper functions.

---

## Appendix A — Complete ASCII Gallery

### A.1 Class (fully populated)

```
  ┌─────────────────────────────────────┐
  │            <<entity>>                │  16 px  stereotype
  │            Customer                  │  20 px  name (bold)
  ├─────────────────────────────────────┤   6 px  divider
  │  - customerId : int                  │
  │  + firstName : String                │  16×4 = 64 px  attributes
  │  + lastName : String                 │
  │  # email : String                    │
  ├─────────────────────────────────────┤   6 px  divider
  │  + Customer(id: int, name: String)   │
  │  + getName() : String                │  16×3 = 48 px  operations
  │  - validateEmail() : bool            │
  └─────────────────────────────────────┘
  Total: 8 + 16 + 20 + 4 + 6 + 64 + 6 + 48 = 172 px
```

### A.2 Interface

```
  ┌─────────────────────────────────────┐
  │           <<interface>>              │
  │            *Paintable*               │  italic name
  ├─────────────────────────────────────┤
  │  + paint(g: Graphics) : void         │
  │  + getSize() : Dimension             │
  └─────────────────────────────────────┘
```

### A.3 Enum

```
  ┌─────────────────────────────────────┐
  │          <<enumeration>>             │
  │             Status                   │
  ├─────────────────────────────────────┤
  │  PENDING                             │
  │  ACTIVE     = 1                      │
  │  COMPLETED  = 2                      │
  │  FAILED                              │
  └─────────────────────────────────────┘
```

### A.4 Datatype

```
  ┌─────────────────────────────────────┐
  │           <<datatype>>               │
  │            DateTime                  │
  └─────────────────────────────────────┘
  (no compartments)
```

### A.5 Package

```
  ┌──────────┐
  │   model  │────────────────────────────┐
  │                                       │
  │   (nested contents — future)          │
  │                                       │
  └───────────────────────────────────────┘
```

### A.6 Generalization

```
  ┌──────────┐                    ┌──────────┐
  │  Source   │───────────────────▷│  Target   │
  │  (child)  │                    │ (parent)  │
  └──────────┘                    └──────────┘
                               hollow triangle
                               (open, no fill)
```

### A.7 Realization

```
  ┌──────────┐                    ┌──────────┐
  │  Source   │═══════════════════▷│  Target   │
  │          │     (dashed)       │          │
  └──────────┘                    └──────────┘
                               hollow triangle
```

### A.8 Association

```
  ┌──────────┐                    ┌──────────┐
  │  Source   │───────────────────│  Target   │
  │          │                    │          │
  └──────────┘                    └──────────┘
                              (no arrowhead)
```

### A.9 Aggregation

```
  ┌──────────┐                    ┌──────────┐
  │  Source   │◇──────────────────│  Target   │
  │ (whole)   │   hollow diamond  │  (part)   │
  └──────────┘                    └──────────┘
               at source end
```

### A.10 Composition

```
  ┌──────────┐                    ┌──────────┐
  │  Source   │◆──────────────────│  Target   │
  │ (whole)   │   filled diamond  │  (part)   │
  └──────────┘                    └──────────┘
               at source end
```

### A.11 Dependency

```
  ┌──────────┐                    ┌──────────┐
  │  Source   │══════════════════╲│  Target   │
  │ (client)  │     (dashed)      ╲│(supplier)│
  └──────────┘                    └──────────┘
                                  open V arrow
```

---

## Appendix B — Colour Reference

| Element                      | Colour / Stroke                                         |
|------------------------------|---------------------------------------------------------|
| Class fill                   | Light yellow (`Color32::from_rgb(255, 255, 230)`)       |
| Interface fill               | Light green (`Color32::from_rgb(230, 255, 230)`)        |
| Enum fill                    | Light orange (`Color32::from_rgb(255, 240, 210)`)       |
| Datatype fill                | Light blue (`Color32::from_rgb(210, 230, 255)`)         |
| Package fill                 | Light gray (`Color32::from_gray(235)`)                  |
| Node border                  | Dark gray (`Color32::from_gray(80)`), 1.0 px             |
| Compartment divider line     | `Color32::from_gray(150)`, 1.0 px                        |
| Generalization stroke        | Black (`Color32::BLACK`), 1.5 px                         |
| Realization stroke           | Black, 1.5 px (dashed)                                  |
| Aggregation stroke           | Black, 1.5 px                                           |
| Composition stroke           | Black, 1.5 px                                           |
| Composition fill             | Black (`Color32::BLACK`)                                 |
| Dependency stroke            | Gray (`Color32::from_gray(120)`), 1.0 px (dashed)       |
| Association stroke           | Gray (`Color32::from_gray(120)`), 1.0 px                |
| Selection highlight          | Blue (`Color32::from_rgb(50, 130, 255)`), 2.0 px dashed |
| Stereotype text              | `Color32::from_gray(140)`                                |
| Class/Interface name text    | `Color32::BLACK`                                         |
| Attribute/operation text     | `Color32::BLACK`                                         |

---

## Appendix C — Edge Cases

| Scenario                           | Expected Behaviour                                        |
|------------------------------------|-----------------------------------------------------------|
| Node has zero attributes **and** zero operations | Name only; no dividers; height = 8 + stereo + name + 4    |
| Node has attributes but no operations | One divider below name; attributes zone; no bottom divider |
| Edge length is zero (overlapping nodes) | Line is omitted; arrowhead is omitted                    |
| Relationship ID is dangling (deleted relationship) | Fall back to solid Association (plain line, no arrowhead) |
| Unrecognised `AssociationType`     | Fall back to solid Association                            |
| Node width less than text width    | Text is clipped (see §5)                                  |
| Enum with no literals              | Middle compartment is omitted                              |
| Package with no name               | Tab shows "(unnamed)"                                     |
| Very tall node (many attributes)   | Node grows dynamically; may exceed canvas — scroll or zoom to see |

---

## Appendix D — Open Questions (M15)

1. **Performance:** If a diagram has 200+ nodes and 400+ edges, does per‑frame text layout (e.g., `painter.layout_no_wrap`) cause frame drops? Mitigation: cache galleys alongside `ViewNode` and invalidate only on content change.

2. **Scaled rendering:** When the user zooms out, should compartment text be legible (minimum font size) or scale down naturally? M15: scale naturally; legibility at small zoom is M16.

3. **Nested nodes (Packages):** Should package compartments visually clip their child nodes? M15: no nesting rendering — packages are just a tabbed box. Child containment is M16+.

4. **Colour scheme:** Are the proposed fills accessible enough for users with colour‑vision deficiency? Mitigation: M15 uses fills but also relies on stereotype text and shape to distinguish element types.

---

*End of specification.*
