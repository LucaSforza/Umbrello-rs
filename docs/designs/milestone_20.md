# Milestone 20 — Actor & UseCase Element Types

**Status:** Design v1  
**Target Milestone:** M20  
**Dependencies:** M19 (Interactive Edge Creation), M18 (Property Editor), M17 (Tool Palette & Node Creation), M15 (Rich Arrowhead Rendering), M10 (XMI Writer), M8 (XMI Reader), M3 (Domain Model)

---

## 1. Objective

M19 completed the core authoring loop for Class-diagram-only elements (Package, Class, Interface, Enum, Datatype) with all 6 relationship types. However, the editor cannot load or create **Use Case diagrams** — the second most fundamental UML diagram type — because the two essential element types, **Actor** and **UseCase**, have no Rust struct definitions, no `ModelElement` variants, no XMI parser/writer support, and no rendering or tool palette entry points.

The `ObjectType` enum already defines `Actor` and `UseCase` (30 variants total), and `DiagramKind::UseCase` already exists. The XMI reader silently skips `UML:Actor` and `UML:UseCase` via the wildcard arm (line ~399), despite several real test XMI files (`test-BVW.xmi`, `test-DCL2.xmi`, `test-DUC.xmi`) containing dozens of these elements — with associations and dependencies already connecting them to one another.

M20 closes this gap by providing:

1. Two new domain types: `Actor { base: ElementBase }` and `UseCase { base: ElementBase }` — structurally minimal non-classifier, non-container types.
2. Two new `ModelElement` variants with full match-arm propagation through `base()`, `base_mut()`, `object_type()`, `is_classifier()`, `is_container()`, `classifier_data()`, `NamedElement`, serde.
3. XMI reader support: `parse_actor()` / `parse_usecase()` methods dispatched from the `local_name` match arms (both `Event::Start` and `Event::Empty` paths). The XMI extension widget parser already handles `actorwidget` and `usecasewidget` tag names — no changes needed there.
4. XMI writer support: `write_element()` dispatch cases + a shared `write_simple_element()` helper for bare-`ElementBase` types. `guess_widget_type()` extended with Actor/UseCase→`actorwidget`/`usecasewidget`.
5. GUI rendering: `element_color()` cases (light orange for Actor, light coral for UseCase), `draw_partitioned_node()` with stick-figure icon for Actor and ellipse-with-centered-name for UseCase.
6. Tool palette: two new node-creation buttons (Actor, UseCase) in the `render_tool_palette()` panel; `create_element_for_tool()` cases; `ToolMode` variants with keyboard shortcuts (U=UseCase, T=Actor).
7. Undo/redo works automatically — existing `CreateElement` + `AddNodeToDiagram` commands handle Actor/UseCase because the `CreateElement` command operates on generic `ModelElement`.
8. XMI round-trip: load existing XMI files with Actor/UseCase, save, reload — all elements preserved.

**Out of scope:** Use-case-specific relationship types (Include, Extend — these are `Dependency` with stereotype in UML 1.2 and are already handled), Use Case diagram system boundary (box widget — the `boxwidget` tag is already parsed by the widget reader), Note widget type, Sequence diagram messages, State/Activity elements, Component/Node/Artifact/Entity types, Stereotype registry. These are deferred to future milestones.

---

## 2. Crates to Modify

| Crate | Changes | Rationale |
|-------|---------|-----------|
| `uml-core` | **Medium** — 2 new structs (`Actor`, `UseCase`) in `elements.rs`; 2 new `ModelElement` variants; match-arm propagation in 8 methods; serde derives; re-exports in `lib.rs` | Domain model must own the new types |
| `uml-io` | **Light** — reader: `parse_actor()` / `parse_usecase()` + dispatch cases in both `Event::Start` and `Event::Empty` match arms. writer: `write_element()` dispatch + `write_simple_element()` helper + `guess_widget_type()` extension | XMI compatibility with real Umbrello files |
| `apps/umbrello` | **Medium** — `rendering.rs`: `element_color()` cases, `draw_partitioned_node()` stick-figure + ellipse rendering. `tool_palette.rs`: 2 new `ToolMode` variants, `create_element_for_tool()` cases, `is_creation_tool()` extended, palette buttons. `app.rs`: 2 keyboard shortcuts. `tests.rs`: new app tests | All GUI interaction |
| `uml-codegen` | **Zero changes** | — |

**No new dependencies.** All types and rendering functions already exist. No undo command changes — the existing `CreateElement` and `AddNodeToDiagram` commands work generically with any `ModelElement`.

---

## 3. New Types, Structs, and Fields

### 3.1 `Actor` Struct (in `crates/uml-core/src/elements.rs`)

```rust
/// A UML Actor — represents a role played by a user or external system.
///
/// Actors are participants in Use Case diagrams. They are non-classifier,
/// non-container elements that carry only identity and metadata.
///
/// In UML 1.2 XMI, actors are `<UML:Actor>` elements with standard
/// Umbrello attributes (visibility, xmi.id, name, stereotype, etc.).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Actor {
    pub base: ElementBase,
}

impl Actor {
    /// Create a new Actor with the given name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            base: ElementBase {
                id: UmlId::new(),
                name: name.to_string(),
                visibility: Visibility::Public,
                stereotype_id: None,
                documentation: String::new(),
                is_abstract: false,
                is_static: false,
                original_xmi_id: None,
            },
        }
    }
}
```

### 3.2 `UseCase` Struct (in `crates/uml-core/src/elements.rs`)

```rust
/// A UML UseCase — represents a unit of functionality provided by the system.
///
/// UseCases appear as ovals in Use Case diagrams. They are non-classifier,
/// non-container elements.
///
/// In UML 1.2 XMI, use cases are `<UML:UseCase>` elements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UseCase {
    pub base: ElementBase,
}

impl UseCase {
    /// Create a new UseCase with the given name.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            base: ElementBase {
                id: UmlId::new(),
                name: name.to_string(),
                visibility: Visibility::Public,
                stereotype_id: None,
                documentation: String::new(),
                is_abstract: false,
                is_static: false,
                original_xmi_id: None,
            },
        }
    }
}
```

### 3.3 `ModelElement` Variant Extension (in `crates/uml-core/src/elements.rs`)

Add two new variants to the `ModelElement` enum:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModelElement {
    // ── Existing variants (M3–M8) ──
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
    Datatype(Datatype),
    Relationship(Relationship),

    // ── New variants (M20) ──
    /// An Actor in a Use Case diagram.
    Actor(Actor),
    /// A UseCase in a Use Case diagram.
    UseCase(UseCase),
}
```

### 3.4 Match-Arm Propagation (in `crates/uml-core/src/elements.rs`)

All methods on `ModelElement` must be extended with `Actor` and `UseCase` arms. The affected methods:

| Method | Actor → | UseCase → |
|--------|---------|-----------|
| `base()` | `&a.base` | `&u.base` |
| `base_mut()` | `&mut a.base` | `&mut u.base` |
| `object_type()` | `ObjectType::Actor` | `ObjectType::UseCase` |
| `is_classifier()` | `false` (not in match) | `false` (not in match) |
| `is_container()` | `false` (not in match) | `false` (not in match) |
| `is_package()` | `false` (not in match) | `false` (not in match) |
| `classifier_data()` | `None` (not in match) | `None` (not in match) |
| `classifier_data_mut()` | `None` (not in match) | `None` (not in match) |

No changes needed for `Relationship`-related methods (`source_id()`, `target_id()`, `relationship_kind()`, `source_multiplicity()`, `target_multiplicity()`, etc.) — Actor and UseCase are not relationships.

### 3.5 `NamedElement` Trait Implementation

The `NamedElement` trait `impl` on `ModelElement` uses the same `base()` / `base_mut()` / `object_type()` methods, so adding the two new variants to those three methods automatically propagates through all provided trait methods (`id()`, `name()`, `set_name()`, `visibility()`, `set_visibility()`, `is_abstract()`, `set_abstract()`, `is_static()`, `documentation()`, `set_documentation()`).

### 3.6 Re-exports (in `crates/uml-core/src/lib.rs`)

```rust
pub use elements::{Actor, UseCase, /* ... existing ... */};
```

### 3.7 `ToolMode` Extension (in `apps/umbrello/src/tool_palette.rs`)

Add two new node-creation variants to `ToolMode`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolMode {
    // ── Existing variants ──
    Select,
    CreateClass,
    CreateInterface,
    CreateEnum,
    CreateDatatype,
    CreatePackage,
    // Edge tools (M19)
    CreateGeneralization,
    CreateRealization,
    CreateAssociation,
    CreateAggregation,
    CreateComposition,
    CreateDependency,

    // ── New node-creation variants (M20) ──
    /// Create a new Actor element on click.
    CreateActor,
    /// Create a new UseCase element on click.
    CreateUseCase,
}
```

#### New methods & updates on `ToolMode`:

```rust
impl ToolMode {
    pub(crate) fn label(&self) -> &'static str {
        match self {
            // ... existing ...
            Self::CreateActor => "🧑 Actor",
            Self::CreateUseCase => "⬭ UseCase",
        }
    }

    fn tooltip(&self) -> &'static str {
        match self {
            // ... existing ...
            Self::CreateActor => "Create an Actor (T)",
            Self::CreateUseCase => "Create a UseCase (U)",
        }
    }

    /// Extended: Actor and UseCase are creation tools.
    pub(crate) fn is_creation_tool(&self) -> bool {
        matches!(
            self,
            Self::CreateClass
                | Self::CreateInterface
                | Self::CreateEnum
                | Self::CreateDatatype
                | Self::CreatePackage
                | Self::CreateActor       // ← new
                | Self::CreateUseCase     // ← new
        )
    }
    // is_edge_tool() — unchanged (Actor/UseCase are not edge tools)
    // association_type() — unchanged
}
```

### 3.8 `create_element_for_tool()` Extension (in `tool_palette.rs`)

```rust
impl UmbrelloApp {
    pub(crate) fn create_element_for_tool(&self, tool: ToolMode) -> ModelElement {
        match tool {
            // ... existing cases ...
            ToolMode::CreateActor => {
                let name = self.generate_unique_name("Actor");
                ModelElement::Actor(Actor::new(&name))
            },
            ToolMode::CreateUseCase => {
                let name = self.generate_unique_name("UseCase");
                ModelElement::UseCase(UseCase::new(&name))
            },
            ToolMode::Select
            | ToolMode::CreateGeneralization
            | /* ... other edge tools ... */ => {
                unreachable!("Non-creation tools should never call create_element_for_tool")
            },
        }
    }
}
```

The `unreachable!()` arm must be updated to include `CreateActor` and `CreateUseCase` in the `Select` arm (they are not non-creation tools; they are creation tools):

```rust
ToolMode::Select
| ToolMode::CreateGeneralization
| ToolMode::CreateRealization
| ToolMode::CreateAssociation
| ToolMode::CreateAggregation
| ToolMode::CreateComposition
| ToolMode::CreateDependency => {
    unreachable!(...)
},
```

### 3.9 `render_tool_palette()` Extension (in `tool_palette.rs`)

Add Actor and UseCase to the node-creation tools array:

```rust
for tool in &[
    ToolMode::Select,
    ToolMode::CreateClass,
    ToolMode::CreateInterface,
    ToolMode::CreateEnum,
    ToolMode::CreateDatatype,
    ToolMode::CreatePackage,
    ToolMode::CreateActor,       // ← new
    ToolMode::CreateUseCase,     // ← new
] {
    // ... existing button rendering logic ...
}
```

---

## 4. XMI Changes

### 4.1 Reader — `parse_actor()` and `parse_usecase()` (in `crates/uml-io/src/xmi/reader.rs`)

Both Actor and UseCase are structurally identical from the parser's perspective: they use `build_base()` with a tag name and produce an `ElementBase`. A shared `parse_simple_element()` helper handles both:

```rust
/// Parse a simple element (Actor, UseCase) that only has an `ElementBase`.
fn parse_simple_element(
    &mut self,
    e: &quick_xml::events::BytesStart,
    element_name: &str,
    make_elem: impl FnOnce(ElementBase) -> ModelElement,
) -> Result<Option<ModelElement>, XmiParseError> {
    let base = self.build_base(e, element_name)?;
    let elem_id = base.id;
    let stereo = Self::attr_value(e, "stereotype");
    self.defer_stereotype(elem_id, stereo);
    Ok(Some(make_elem(base)))
}

/// Parse a `<UML:Actor>` element.
fn parse_actor(
    &mut self,
    e: &quick_xml::events::BytesStart,
) -> Result<Option<ModelElement>, XmiParseError> {
    self.parse_simple_element(e, "Actor", |base| {
        ModelElement::Actor(Actor { base })
    })
}

/// Parse a `<UML:UseCase>` element.
fn parse_usecase(
    &mut self,
    e: &quick_xml::events::BytesStart,
) -> Result<Option<ModelElement>, XmiParseError> {
    self.parse_simple_element(e, "UseCase", |base| {
        ModelElement::UseCase(UseCase { base })
    })
}
```

#### Dispatch cases in `Event::Start` match (add after "DataType" arm, around line 349):

```rust
"Actor" => {
    if let Some(elem) = self.parse_actor(e)? {
        model.insert(elem);
        count += 1;
    }
},
"UseCase" => {
    if let Some(elem) = self.parse_usecase(e)? {
        model.insert(elem);
        count += 1;
    }
},
```

#### Dispatch cases in `Event::Empty` match (add after "DataType" arm, around line 470):

```rust
"Actor" => {
    if let Some(elem) = self.parse_actor(e)? {
        model.insert(elem);
        count += 1;
    }
},
"UseCase" => {
    if let Some(elem) = self.parse_usecase(e)? {
        model.insert(elem);
        count += 1;
    }
},
```

**Widget parsing:** The existing `handle_xmi_extension_start()` at line 1330 already includes `"usecasewidget"` and `"actorwidget"` in its match pattern on line 1331. The generic `parse_xmi_widget()` at line 1383 parses all widget types using `xmi.id`, `x`, `y`, `width`, `height` attributes — no changes needed for widget parsing.

### 4.2 Writer — Element Dispatch (in `crates/uml-io/src/xmi/writer.rs`)

#### `write_element()` dispatch extension (lines 265–279):

```rust
fn write_element(&mut self, elem: &ModelElement, model: &UmlModel) -> Result<(), XmiWriteError> {
    match elem {
        // ... existing cases ...
        ModelElement::Actor(actor) => self.write_simple_element("UML:Actor", &actor.base),
        ModelElement::UseCase(uc) => self.write_simple_element("UML:UseCase", &uc.base),
    }
}
```

#### `write_simple_element()` helper — writes a self-closing UML element with only `ElementBase` metadata:

```rust
/// Write a simple UML element (Actor, UseCase) as a self-closing tag.
fn write_simple_element(
    &mut self,
    tag_name: &str,
    base: &uml_core::ElementBase,
) -> Result<(), XmiWriteError> {
    let xmi_id = self.lookup_xmi_id(base.id);
    let mut tag = BytesStart::new(tag_name);
    tag.push_attribute(("xmi.id", xmi_id.as_str()));
    tag.push_attribute(("name", base.name.as_str()));
    tag.push_attribute(("visibility", base.visibility.as_str()));
    tag.push_attribute(("isSpecification", "false"));
    tag.push_attribute(("isAbstract", if base.is_abstract { "true" } else { "false" }));
    tag.push_attribute(("isLeaf", "false"));
    tag.push_attribute(("isRoot", "false"));

    // Write stereotype reference if set
    if let Some(st_id) = base.stereotype_id {
        let st_xmi = self.lookup_xmi_id(st_id);
        tag.push_attribute(("stereotype", st_xmi.as_str()));
    }

    self.writer.write_event(Event::Empty(tag))?;
    Ok(())
}
```

#### `guess_widget_type()` extension (line 843–855):

```rust
fn guess_widget_type(&self, model: &UmlModel, element_id: UmlId) -> &'static str {
    if let Some(elem) = model.get(element_id) {
        return match elem {
            ModelElement::Package(_) => "packagewidget",
            ModelElement::Class(_) => "classwidget",
            ModelElement::Interface(_) => "interfacewidget",
            ModelElement::Enum(_) => "enumwidget",
            ModelElement::Datatype(_) => "datatypewidget",
            ModelElement::Actor(_) => "actorwidget",        // ← new
            ModelElement::UseCase(_) => "usecasewidget",    // ← new
            ModelElement::Relationship(_) => "classwidget",
        };
    }
    "classwidget"
}
```

### 4.3 XMI Format Compatibility

Both Actor and UseCase elements in Umbrello XMI use the **exact same attribute set** as other self-closing elements (Class, Interface, etc.): `visibility`, `isSpecification`, `namespace`, `isAbstract`, `isLeaf`, `isRoot`, `xmi.id`, `name`, and optionally `stereotype` and `comment`. Our `build_base()` already handles `xmi.id`, `name`, `visibility`, and `isAbstract`. The other attributes (`isSpecification`, `isLeaf`, `isRoot`, `namespace`) are standard boilerplate in Umbrello's XMI output but not meaningful for the Rust model — we write them to the output for compatibility but skip them during parsing (the writer writes them as literal `"false"`).

The `comment` attribute that appears on some `UML:UseCase` elements (e.g., `comment="asfs"` in test-DUC.xmi) is not currently parsed by `build_base()`. For M20, it is **skipped during reading** for consistency with the existing reader behavior (it is a minor feature used for diagram annotation, not structural model data). A future milestone can add a `comment` field to `ElementBase` if needed.

---

## 5. UI Changes (all in `apps/umbrello/src/`)

### 5.1 `element_color()` — New Colors (in `rendering.rs`)

```rust
pub(crate) fn element_color(elem: Option<&ModelElement>) -> egui::Color32 {
    match elem {
        Some(ModelElement::Class(_)) => egui::Color32::from_rgb(180, 210, 255),
        Some(ModelElement::Interface(_)) => egui::Color32::from_rgb(180, 255, 210),
        Some(ModelElement::Enum(_)) => egui::Color32::from_rgb(255, 210, 180),
        Some(ModelElement::Datatype(_)) => egui::Color32::from_rgb(210, 180, 255),
        Some(ModelElement::Package(_)) => egui::Color32::from_rgb(255, 255, 200),
        // ── New (M20) ──
        Some(ModelElement::Actor(_)) => egui::Color32::from_rgb(255, 200, 170),    // light orange
        Some(ModelElement::UseCase(_)) => egui::Color32::from_rgb(255, 180, 180),  // light coral
        _ => egui::Color32::from_rgb(220, 220, 220),
    }
}
```

**Color rationale:** Light orange (salmon) for Actor evokes the human/user-role connotation. Light coral for UseCase distinguishes it from Class elements while keeping a warm, readable fill.

### 5.2 `draw_partitioned_node()` — Actor Stick Figure (in `canvas.rs`)

Add a new match arm after the Package arm (before the `_` wildcard):

```rust
Some(ModelElement::Actor(actor)) => {
    // ── Stick-figure icon ──
    // Draw a simple stick-figure person icon centered in the node,
    // with the actor name below it.
    let cx = full_rect.center().x;
    let top = full_rect.top() + 4.0;
    let stick_color = egui::Color32::from_gray(60);

    // Head (circle, radius ~5)
    let head_center = egui::pos2(cx, top + 6.0);
    painter.circle_filled(head_center, 5.0, stick_color);

    // Body (vertical line from below head to ~2/3 down)
    let body_top = egui::pos2(cx, top + 12.0);
    let body_bottom = egui::pos2(cx, top + 28.0);
    painter.line_segment([body_top, body_bottom], egui::Stroke::new(1.5, stick_color));

    // Arms (horizontal line at shoulder level)
    let shoulder_y = top + 16.0;
    painter.line_segment(
        [egui::pos2(cx - 8.0, shoulder_y), egui::pos2(cx + 8.0, shoulder_y)],
        egui::Stroke::new(1.5, stick_color),
    );

    // Left leg (diagonal)
    painter.line_segment(
        [body_bottom, egui::pos2(cx - 6.0, top + 36.0)],
        egui::Stroke::new(1.5, stick_color),
    );
    // Right leg (diagonal)
    painter.line_segment(
        [body_bottom, egui::pos2(cx + 6.0, top + 36.0)],
        egui::Stroke::new(1.5, stick_color),
    );

    // Name below the stick figure
    let name_y = top + 40.0;
    painter.text(
        egui::pos2(cx, name_y),
        egui::Align2::CENTER_TOP,
        &actor.base.name,
        name_font.clone(),
        egui::Color32::BLACK,
    );
},
Some(ModelElement::UseCase(uc)) => {
    // ── Ellipse (oval) with centered name ──
    // Draw an ellipse inscribed in the node rectangle.
    let ellipse_color = egui::Color32::from_gray(60);
    let stroke = egui::Stroke::new(1.5, ellipse_color);
    let inset = egui::vec2(6.0, 8.0);
    let ellipse_rect = full_rect.shrink2(inset);
    // Approximate ellipse with a rounded rectangle (corner_radius = half height)
    let corner_radius = (ellipse_rect.height() / 2.0).min(ellipse_rect.width() / 2.0);
    painter.rect_stroke(ellipse_rect, corner_radius, stroke, egui::StrokeKind::Inside);

    // Name centered inside the ellipse
    painter.text(
        ellipse_rect.center(),
        egui::Align2::CENTER_CENTER,
        &uc.base.name,
        name_font.clone(),
        egui::Color32::BLACK,
    );
},
```

**Design note for Actor:** The stick-figure icon is ~38px tall + name (~16px). The default node size from `place_element()` (160×60) provides enough space. The actor's name appears below the stick figure, centered horizontally.

**Design note for UseCase:** The ellipse uses `rect_stroke()` with a large corner radius to approximate an ellipse within the available rectangle. The name is centered inside the ellipse. The 160×60 node size works well — the ellipse spans ~148×44 with 6px/8px insets, providing a natural oval shape.

### 5.3 Keyboard Shortcuts (in `app.rs` `update()`)

Add two new keyboard shortcuts, only active when `!ctx.wants_keyboard_input()`:

```rust
// ── Actor & UseCase tool keyboard shortcuts ──
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::T)) {
    // 'T' is free (Ctrl+T not used)
    self.current_tool = crate::tool_palette::ToolMode::CreateActor;
    self.preview_position = None;
    self.drag_source_node_id = None;
}
if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::U)) {
    // 'U' is free (Ctrl+U not used)
    self.current_tool = crate::tool_palette::ToolMode::CreateUseCase;
    self.preview_position = None;
    self.drag_source_node_id = None;
}
```

**Shortcut rationale:**
- `T` → Ac**t**or (free — no conflict with existing Ctrl+N/O/S/Q)
- `U` → **U**seCase (free — the only other use is Ctrl+U for "underline" in rich text, not relevant)
- Escape behavior unchanged — resets to Select

### 5.4 Property Editor (in `property_editor.rs`)

The property editor panel displays element details generically via the `NamedElement` trait. Since Actor and UseCase implement `NamedElement` through `base()`/`name()`/etc., the property editor **works automatically** — name editing, visibility dropdown, abstract/static checkboxes, documentation, and the element type/ID display all function without code changes.

The classifier details section (attributes/operations list) is only shown when `is_classifier()` returns true, which it does not for Actor/UseCase. No changes needed.

### 5.5 Selection & Edge Connection

Since Actor and UseCase can be sources and targets of relationships (e.g., Actor ↔ UseCase associations in real XMI files), they must work with the edge-creation tools. The existing edge-creation code operates on `UmlId` generically — `place_edge()` calls `CreateEdge::new()` which works with any two node IDs regardless of element type. No changes needed.

---

## 6. Test Plan

### 6.1 `uml-core` — Domain Model Tests (in `crates/uml-core/src/elements.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| DM-20 | `actor_creation` | `Actor::new("user")` creates an Actor with correct name, generated UmlId, default visibility (Public) |
| DM-21 | `usecase_creation` | `UseCase::new("Login")` creates a UseCase with correct name, generated UmlId |
| DM-22 | `actor_model_element_insert` | Inserting an Actor into UmlModel, retrieving by ID, verifying `object_type()` returns `ObjectType::Actor` |
| DM-23 | `usecase_model_element_insert` | Same for UseCase → `ObjectType::UseCase` |
| DM-24 | `actor_not_classifier` | `ModelElement::Actor(a).is_classifier()` returns `false` |
| DM-25 | `usecase_not_classifier` | `ModelElement::UseCase(u).is_classifier()` returns `false` |
| DM-26 | `actor_not_container` | `ModelElement::Actor(a).is_package()` returns `false` |
| DM-27 | `agent_serde_roundtrip` | `Actor` serializes/deserializes via JSON (serde tagged format) |

### 6.2 `uml-core` — Serde Round-Trip Tests (in `crates/uml-core/tests/serde_roundtrip.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| SRT-8 | `serde_roundtrip_actor` | Actor round-trips JSON: serialize → deserialize → structural equality |
| SRT-9 | `serde_roundtrip_usecase` | UseCase round-trips JSON |

### 6.3 `uml-io` — XMI Reader Tests (in `crates/uml-io/src/xmi/reader.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| XMI-20 | `parse_actor_from_xmi` | Parse `<UML:Actor xmi.id="A1" name="User" visibility="public"/>` → correct Actor with name, XMI ID |
| XMI-21 | `parse_usecase_from_xmi` | Parse `<UML:UseCase xmi.id="UC1" name="Login" visibility="public"/>` → correct UseCase |
| XMI-22 | `parse_actor_with_stereotype` | Actor with `stereotype="actor"` attribute → `stereotype_id` deferred for Pass 2 |
| XMI-23 | `parse_usecase_in_package` | UseCase inside `<UML:Namespace.ownedElement>` within a Model → registered correctly |
| XMI-24 | `parse_usecase_with_comment` | UseCase with `comment="note"` attribute → parsed without error (comment attribute skipped) |

### 6.4 `uml-io` — XMI Writer Tests (in `crates/uml-io/src/xmi/writer.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| XMI-25 | `write_actor_to_xmi` | Actor → `<UML:Actor .../>` self-closing tag with correct attributes |
| XMI-26 | `write_usecase_to_xmi` | UseCase → `<UML:UseCase .../>` self-closing tag |
| XMI-27 | `actor_roundtrip` | Actor: insert into model → write XMI → read XMI → element preserved with same name + visibility |
| XMI-28 | `usecase_roundtrip` | Same round-trip test for UseCase |
| XMI-29 | `actor_widget_in_diagram` | Actor in diagram → `guess_widget_type()` returns `"actorwidget"`, written correctly in XMI extension |
| XMI-30 | `usecase_widget_in_diagram` | UseCase in diagram → `guess_widget_type()` returns `"usecasewidget"` |

### 6.5 `uml-io` — Real Corpus Test Update

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| CORP-1 | `load_real_xmi_with_actor` | Load `test-DUC.xmi` → at least 4 actors and 9 use cases parsed (currently skipped) |

The existing `load_test_xmi` corpus test loads only `test-COG.xmi` which has no Actor/UseCase elements. A new test or extension should load `test-DUC.xmi` (the Restaurant use case diagram) to verify real-world Actor/UseCase parsing.

### 6.6 `apps/umbrello` — App Tests (in `tests.rs`)

| Test ID | Name | What It Verifies |
|---------|------|------------------|
| APP-28 | `tool_actor_is_creation` | `ToolMode::CreateActor.is_creation_tool()` returns `true` |
| APP-29 | `tool_usecase_is_creation` | `ToolMode::CreateUseCase.is_creation_tool()` returns `true` |
| APP-30 | `tool_actor_not_edge` | `ToolMode::CreateActor.is_edge_tool()` returns `false` |
| APP-31 | `tool_usecase_not_edge` | `ToolMode::CreateUseCase.is_edge_tool()` returns `false` |
| APP-32 | `create_element_for_actor` | `create_element_for_tool(CreateActor)` returns `ModelElement::Actor` with name "Actor_1" |
| APP-33 | `create_element_for_usecase` | `create_element_for_tool(CreateUseCase)` returns `ModelElement::UseCase` with name "UseCase_1" |
| APP-34 | `place_actor_dirty_flag` | Placing an Actor sets `is_dirty` to `true` |
| APP-35 | `place_usecase_dirty_flag` | Placing a UseCase sets `is_dirty` |
| APP-36 | `actor_unique_naming` | Placing two actors produces "Actor_1" and "Actor_2" |
| APP-37 | `usecase_unique_naming` | Placing two use cases produces "UseCase_1" and "UseCase_2" |
| APP-38 | `actor_undo_redo` | Undo after placing Actor removes element + node; redo restores both |
| APP-39 | `actor_color` | `element_color(Some(ModelElement::Actor(...)))` returns correct orange color |
| APP-40 | `usecase_color` | `element_color(Some(ModelElement::UseCase(...)))` returns correct coral color |

### 6.7 Manual / Visual Tests

| Test ID | What to Verify |
|---------|---------------|
| VIS-11 | Select Actor tool → click canvas → stick-figure icon appears with name below it |
| VIS-12 | Select UseCase tool → click canvas → ellipse with centered name appears |
| VIS-13 | Tool palette shows Actor (🧑 Actor) and UseCase (⬭ UseCase) buttons in node-creation section |
| VIS-14 | Press `T` → Actor tool selected; ghost preview shows at cursor |
| VIS-15 | Press `U` → UseCase tool selected |
| VIS-16 | Drag edge from Actor to UseCase → rubber-band preview shows, release creates Association |
| VIS-17 | Save → close → reopen `test-DUC.xmi` → actors and use cases are preserved in XMI round-trip |
| VIS-18 | Select Actor node → property editor shows Actor type, editable name, visibility dropdown |
| VIS-19 | Undo after creating Actor → element disappears from canvas; redo → reappears |

### 6.8 Verification Commands

```sh
# Unit tests for new domain types
cargo test -p uml-core actor
cargo test -p uml-core usecase

# Serde round-trip tests
cargo test -p uml-core serde_roundtrip_actor
cargo test -p uml-core serde_roundtrip_usecase

# XMI reader tests
cargo test -p uml-io parse_actor
cargo test -p uml-io parse_usecase

# XMI writer tests
cargo test -p uml-io write_actor
cargo test -p uml-io write_usecase
cargo test -p uml-io actor_roundtrip
cargo test -p uml-io usecase_roundtrip

# App-level tests
cargo test -p umbrello tool_actor
cargo test -p umbrello tool_usecase
cargo test -p umbrello create_element_for_actor
cargo test -p umbrello actor_color
cargo test -p umbrello usecase_color

# Full suite
cargo test --workspace

# Lint
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
```

### 6.9 Expected Test Count (Post-M20)

| Test Suite | From (M19) | New | Expected |
|------------|-----------|-----|----------|
| `uml-core` elements | 140 | +7 (DM-20..DM-27) | ~147 |
| `uml-core` serde_roundtrip | 6 | +2 (SRT-8, SRT-9) | ~8 |
| `uml-io` XMI reader | 46 | +5 (XMI-20..XMI-24) | ~51 |
| `uml-io` XMI writer | included above | +6 (XMI-25..XMI-30) | ~57 |
| `uml-io` real corpus | 1 | +1 (CORP-1) | ~2 |
| `apps/umbrello` tests | 58 | +13 (APP-28..APP-40) | ~71 |
| **Total** | **275** | **+34** | **~309** |

---

## 7. Implementation Sequence

The `@implementer` MUST follow this order:

### Phase 1: Domain Model — Actor & UseCase Types (Commit 1)

1. Add `Actor` struct + `impl Actor { pub fn new() }` to `crates/uml-core/src/elements.rs`.
2. Add `UseCase` struct + `impl UseCase { pub fn new() }` to `crates/uml-core/src/elements.rs`.
3. Add `Actor(Actor)` and `UseCase(UseCase)` variants to `ModelElement` enum.
4. Propagate match arms in: `base()`, `base_mut()`, `object_type()`. Verify `is_classifier()`, `is_container()`, `is_package()`, `classifier_data()`, `classifier_data_mut()` do NOT need changes (they use wildcard/`None` patterns).
5. Update `NamedElement` impl on `ModelElement` (propagated automatically via `base()`/`base_mut()`/`object_type()` — verify no additional changes needed).
6. Re-export `Actor`, `UseCase` from `lib.rs`.
7. Write DM-20 through DM-27 tests in `elements.rs`.
8. Write SRT-8, SRT-9 tests in `tests/serde_roundtrip.rs`.
9. Verify: `cargo test -p uml-core && cargo clippy -p uml-core -- -D warnings`
10. **Commit** with message: `"feat(uml-core): add Actor and UseCase domain types with ModelElement variants"`

### Phase 2: XMI Reader & Writer (Commit 2)

1. Add `parse_simple_element()` helper to `reader.rs`.
2. Add `parse_actor()` and `parse_usecase()` thin wrappers.
3. Add dispatch cases in `Event::Start` match (after "DataType" arm).
4. Add dispatch cases in `Event::Empty` match (after "DataType" arm).
5. In `writer.rs`: Add `write_simple_element()` helper.
6. Extend `write_element()` match with `Actor` and `UseCase` dispatch.
7. Extend `guess_widget_type()` with `actorwidget` and `usecasewidget`.
8. Write XMI-20 through XMI-30 tests (reader: 5, writer: 6).
9. Write CORP-1 test (load `test-DUC.xmi` and verify Actor/UseCase count).
10. Verify: `cargo test -p uml-io && cargo clippy -p uml-io -- -D warnings`
11. **Commit** with message: `"feat(uml-io): add Actor and UseCase XMI reader/writer support"`

### Phase 3: GUI — Rendering, Tool Palette, Shortcuts (Commit 3)

1. Add `Actor` and `UseCase` match arms to `element_color()` in `rendering.rs`.
2. Add `Actor` stick-figure and `UseCase` ellipse rendering to `draw_partitioned_node()` in `canvas.rs`.
3. Add `CreateActor` and `CreateUseCase` variants to `ToolMode` in `tool_palette.rs`.
4. Update `label()`, `tooltip()`, `is_creation_tool()`.
5. Add `create_element_for_tool()` cases for Actor and UseCase.
6. Update `render_tool_palette()` to include Actor and UseCase buttons.
7. Add keyboard shortcuts (T for Actor, U for UseCase) in `app.rs` `update()`.
8. Write APP-28 through APP-40 tests in `tests.rs`.
9. Verify: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
10. **Commit** with message: `"feat(app): add Actor and UseCase rendering, tool palette, and keyboard shortcuts"`

### Final Verification

```sh
cargo test --workspace            # All tests pass (expected: ~309)
cargo clippy --workspace --all-targets -- -D warnings  # Zero warnings
cargo fmt --all --check           # No formatting diffs
```

---

## 8. File Summary (After M20 — Changed Files Only)

```
Modified files:
  crates/uml-core/src/elements.rs          # +Actor, +UseCase structs, +2 enum variants, +match arms (~70 lines)
  crates/uml-core/src/lib.rs               # +2 re-exports
  crates/uml-core/tests/serde_roundtrip.rs # +2 round-trip tests (~30 lines)
  crates/uml-io/src/xmi/reader.rs          # +parse_simple_element, +parse_actor, +parse_usecase,
                                            #   +4 dispatch cases (~65 lines, 2416→2480 total)
  crates/uml-io/src/xmi/writer.rs          # +write_simple_element, +2 dispatch cases,
                                            #   +2 guess_widget_type cases (~55 lines, 1273→1328 total)
  apps/umbrello/src/rendering.rs           # +2 element_color cases (~6 lines)
  apps/umbrello/src/canvas.rs              # +2 draw_partitioned_node cases (~70 lines)
  apps/umbrello/src/tool_palette.rs        # +2 ToolMode vars, +label/tooltip/is_creation_tool updates,
                                            #   +create_element_for_tool cases, +palette buttons (~45 lines)
  apps/umbrello/src/app.rs                 # +2 keyboard shortcuts (~10 lines)
  apps/umbrello/src/tests.rs               # +13 app tests (~160 lines)
```

No new files. No changes to `uml-codegen`.

---

## 9. Design Decisions

| Decision | Rationale |
|----------|-----------|
| **`Actor` and `UseCase` are bare `ElementBase`** | Neither has attributes, operations, children, or literals in UML. They are identity-only elements. No `ClassifierData`, no `children: Vec<UmlId>`. This is the simplest possible element pattern, and it's correct per UML semantics. |
| **Shared `parse_simple_element()` helper** | Actor and UseCase share identical XMI parsing logic. The helper takes a closure to construct the appropriate `ModelElement` variant, avoiding code duplication while keeping the parser extensible. |
| **Shared `write_simple_element()` helper** | Same rationale for writing. Both element types produce self-closing `<UML:Actor .../>` / `<UML:UseCase .../>` tags with the same attribute set. |
| **Stick-figure rendering for Actor** | The standard UML notation for an actor is a stick figure (head circle + body line + arms + legs). This is instantly recognizable and follows UML conventions. The implementation uses simple `painter.line_segment()` and `painter.circle_filled()` calls — no custom meshes needed. |
| **Rounded-rectangle ellipse for UseCase** | egui has no native ellipse primitive. Using `rect_stroke()` with `corner_radius = height/2` produces a visually convincing oval. The inset margins (6px horizontal, 8px vertical) prevent the ellipse from touching the selection highlight border. |
| **Light orange for Actor, light coral for UseCase** | Distinct from all existing element colors (blue, green, peach, lavender, yellow) while maintaining pastel readability on white canvas. Actor orange evokes human/user-role association. |
| **`T` and `U` keyboard shortcuts** | Neither conflicts with existing shortcuts. `'T'` without Ctrl is free (Ctrl+T is not used). `'U'` without Ctrl is free (Ctrl+U is not used in our app). |
| **No changes to undo commands** | `CreateElement` + `AddNodeToDiagram` operate generically on `ModelElement`. The `CreateEdge` command also works generically with any two node IDs. Actor and UseCase benefit from all existing undo/redo support for free. |
| **Widget parsing unchanged** | The XMI extension widget parser already matches `actorwidget` and `usecasewidget` tags (line 1331). The generic `parse_xmi_widget()` reads position/size from `x`, `y`, `width`, `height` attributes regardless of widget type. |
| **Skip `comment` attribute during XMI read** | The `comment` attribute appears on some `UML:UseCase` elements in real Umbrello files but is not part of `ElementBase`. Including it would require adding a `comment: String` field to `ElementBase` — a schema change affecting all element types. Deferred to a future milestone with proper `comment` tracking. |

---

## 10. Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| **Compiler errors from non-exhaustive match** | Adding `Actor` and `UseCase` to `ModelElement` will cause compiler errors at every `match` on `ModelElement` throughout the codebase | The exhaustive match check is Rust's safety net — it forces us to handle every new variant. All existing matches have a `_` wildcard arm that will silently absorb the new variants. The `is_classifier()` / `is_container()` methods use `matches!()` macros with explicit variant lists — since Actor/UseCase are not classifiers, they are correctly excluded. |
| **Stick-figure rendering too tall for default 60px node height** | The stick figure is ~36px + name text ~16px = ~52px total. With top padding of 4px, it fits within the 60px node height. | Verified: stick figure from `top + 6` to `top + 36` = 30px, plus name at `top + 40` = 16px. Total 56px. Within 60px with 2px bottom margin. |
| **UseCase ellipse clipped by node bounds** | If the ellipse exceeds the node rect, part of it won't be visible. | The ellipse uses `full_rect.shrink2(inset)` with 6px/8px insets, leaving ~148×44 within a 160×60 node. The ellipse is fully contained. |
| **Real XMI files use `boxwidget` for system boundary** | The `test-DUC.xmi` has a `<boxwidget>` surrounding actors and use cases. This widget type is already parsed (line 1331 includes it as matching — wait, no it doesn't. Let me re-check.) | [UPDATE after review: The `handle_xmi_extension_start()` at line 1330-1333 does NOT include `boxwidget` in its match pattern. The `boxwidget` tag falls into the `_` wildcard at line 1351-1356 and is silently skipped. This is pre-existing behavior — system boundaries were never parsed. Not in scope for M20.] |
| **`guess_widget_type()` match becoming large** | With 7 element types + fallback, the match has 8 arms. | This is acceptable — each arm returns a simple `&'static str`. No logic, no branches. If it exceeds ~15 arms in future milestones, it can be refactored to a method on `ModelElement`. |
| **Actor/UseCase as edge source/target in XMI** | Real XMI files have AssociationEnd `type` attributes pointing to actor/usecase XMI IDs. If these elements are parsed, the relationship resolution in Pass 2 must handle them. | The relationship resolution code at line 727-742 uses `id_map.get(&pr.source_xmi)` which works generically for any registered ID. Since `parse_actor()` and `parse_usecase()` both call `register_id()`, the XMI ID → UmlId mapping will be correct. No changes to relationship resolution needed. |

---

*Last updated: 2026-06-26 · Umbrello-RS Milestone 20 Design v1*
