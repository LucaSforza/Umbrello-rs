# Diagram Engine Analysis

> **File:** `rust-rewrite/analysis/diagram_engine_analysis.md`
> **Date:** 2026-06-23
> **Scope:** Complete analysis of the Umbrello C++ diagram rendering, interaction, and editing engine.

---

## Table of Contents

1. [Scene/View Architecture](#1-sceneview-architecture)
2. [Widget Hierarchy and Composition](#2-widget-hierarchy-and-composition)
3. [Rendering Model](#3-rendering-model)
4. [Association/Line Drawing System](#4-associationline-drawing-system)
5. [Layout System](#5-layout-system)
6. [Interactive Editing](#6-interactive-editing)
7. [Model→View Propagation](#7-modelview-propagation)
8. [Rust Recommendations](#8-rust-recommendations)

---

## 1. Scene/View Architecture

### High-Level Design

Umbrello builds on Qt's **QGraphicsView / QGraphicsScene** framework:

```
UMLFolder (owns a list of UMLView instances)
  └── UMLView : QGraphicsView  (one per diagram)
        └── UMLScene : QGraphicsScene  (one per view, 1:1)
              ├── UMLWidgetList (all widgets on the diagram)
              ├── AssociationWidgetList (all lines/edges)
              └── MessageWidgetList (sequence message edges)
```

**Key files:**
- `umbrello/umlview.h/.cpp` — `UMLView : QGraphicsView`
- `umbrello/umlscene.h/.cpp` — `UMLScene : QGraphicsScene`
- `umbrello/umlmodel/umlfolder.h/.cpp` — `UMLFolder : UMLPackage`, owns `UMLViewList m_diagrams`

### Ownership and Lifetime

| Object | Owner | Count |
|--------|-------|-------|
| `UMLView` | `UMLFolder` | One per diagram |
| `UMLScene` | `UMLView` (constructed by `UMLView`) | One per diagram |
| `UMLWidget` | `UMLScene` | N per diagram |
| `AssociationWidget` | `UMLScene` | N per diagram |
| `FloatingTextWidget` | `UMLScene` (or owned by `AssociationWidget` role) | N per association |

- `UMLFolder::addView()` / `UMLFolder::removeView()` manages view lifetime.
- `UMLScene::addWidgetCmd()` / `UMLScene::removeWidget()` manages widget lifetime.
- `UMLFolder::m_diagrams` is a `UMLViewList` (the folder's list of views/scenes).

### View Details

`UMLView` (`QGraphicsView` subclass, 59 lines in header):
- **Zoom:** `qreal zoom()` / `void setZoom(qreal)` — applies a `QTransform` scale to the view.
- **Wheel event:** `wheelEvent()` — zoom in/out on Ctrl+scroll.
- **Resize event:** `resizeEvent()` — re-centers scene in view.
- **Show/hide events:** `showEvent()` / `hideEvent()` — forwarded to `UMLScene`.
- **Mouse events:** `mousePressEvent()` / `mouseReleaseEvent()` — forwarded to scene for state handling.

### Scene Details

`UMLScene` (`QGraphicsScene` subclass, 468 lines in header):
- **Central registry:** Holds `UMLWidgetList`, `AssociationWidgetList`, `MessageWidgetList`.
- **Diagram metadata:** `m_Type` (diagram type), `m_Name`, `m_Documentation`, `m_nID`.
- **Options:** `Settings::OptionState m_Options` — per-diagram rendering settings.
- **Grid:** `LayoutGrid* m_layoutGrid` — background dot/cross grid.
- **Alignment:** `AlignmentGuide* m_alignmentGuide` — live edge snapping.
- **Toolbar states:** Delegates to `ToolBarState` via `slotToolBarChanged()`.
- **Model sync:** Slots `slotObjectCreated()`, `slotObjectRemoved()` propagate model changes.
- **Selections:** `selectedWidgets()`, `selectedAssociationWidgets()`, `selectedMessageWidgets()`.
- **Hit testing:** `widgetAt()`, `associationAt()`, `messageAt()`, `collisions()`.

### Interaction Flow

```
User Input → QGraphicsView (native Qt) → QGraphicsScene (mouse*Event overrides)
  → UMLScene delegates to current ToolBarState
    → ToolBarState::mousePressWidget/Association/Empty
      → Widget/Association handles event or Scene creates new widget
```

### Observations

- **1:1 View:Scene** eliminates need for multiple views of same diagram (no `QGraphicsView` sharing).
- **QGraphicsView's coordinate system** is used heavily: `mapToScene()` / `mapFromScene()` for zoom coordinate transforms.
- **QGraphicsScene** provides built-in: collision detection, event routing, item indexing, selection model, z-ordering — all of which must be replaced in Rust.
- **QGraphicsItem's `itemChange()`** is used for selection synchronization (via `QGraphicsObjectWrapper`).

---

## 2. Widget Hierarchy and Composition

### Inheritance Chain

```
QObject
  └── QGraphicsItem                           (Qt paint/hit-test/transform primitive)
        └── QGraphicsObject                   (adds signals/slots)
              └── QGraphicsObjectWrapper       (Umbrello: virtual setSelected hack)
                    └── WidgetBase             (Umbrello: common base for all diagram items)
                          ├── UMLWidget        (Umbrello: main graphical widget base)
                          │     ├── ClassifierWidget (class/interface with visual compartments)
                          │     ├── ActorWidget
                          │     ├── UseCaseWidget
                          │     ├── ComponentWidget
                          │     ├── NodeWidget
                          │     ├── ArtifactWidget
                          │     ├── DatatypeWidget
                          │     ├── EnumWidget
                          │     ├── EntityWidget
                          │     ├── ObjectWidget
                          │     ├── CategoryWidget
                          │     ├── NoteWidget
                          │     ├── BoxWidget → ForkJoinWidget
                          │     ├── StateWidget (12 types)
                          │     ├── ActivityWidget (7 types)
                          │     ├── SignalWidget (3 types)
                          │     ├── ObjectNodeWidget (4 types)
                          │     ├── RegionWidget
                          │     ├── PreconditionWidget
                          │     ├── CombinedFragmentWidget (9 types)
                          │     ├── FloatingTextWidget
                          │     ├── FloatingDashLineWidget
                          │     ├── PinWidget / PortWidget
                          │     └── MessageWidget
                          └── AssociationWidget (line edges, also implements LinkWidget)
```

### Widget Type System

**`WidgetBase::WidgetType`** (enum, 29 values from `wt_Min` to `wt_Max`):
- Some have `UMLObject` backing (`wt_Class`, `wt_Actor`, `wt_UseCase`, etc.)
- Some are purely visual decorations (`wt_Note`, `wt_Box`, `wt_State`, `wt_Activity`, etc.)

**Type queries** on `WidgetBase`:
```cpp
bool isClassWidget()    const { return baseType() == wt_Class; }
bool isStateWidget()    const { return baseType() == wt_State; }
// ... 29 is*() methods total
```

**Downcast helpers:**
```cpp
ClassifierWidget* asClassifierWidget();
StateWidget*      asStateWidget();
// ... 29 as*() methods total, const + non-const variants = 58 methods
```

### Widget State and Properties (`WidgetBase`)

| Property | Type | Saved |
|----------|------|-------|
| `m_baseType` | `WidgetType` | Yes |
| `m_rect` | `QRectF` (always at 0,0 origin) | Size only |
| `m_nId` | `Uml::ID::Type` | Yes |
| `m_nLocalID` | `Uml::ID::Type` | Yes |
| `m_umlObject` | `QPointer<UMLObject>` | Via object ref |
| `m_textColor`, `m_lineColor`, `m_fillColor` | `QColor` | Yes |
| `m_lineWidth` | `uint` | Yes |
| `m_useFillColor` | `bool` | Yes |
| `m_usesDiagram*Color` | 4x `bool` | Yes |
| `m_autoResize` | `bool` | Yes |
| `m_changesShape` | `bool` | Yes |
| `m_highLighted` | `bool` | No |
| `m_Doc` | `QString` | Yes (if no UMLObject) |
| `m_Text` | `QString` | No |
| `m_font` | `QFont` | Yes |

### Additional Properties on `UMLWidget`

| Property | Type | Description |
|----------|------|-------------|
| `m_instanceName` | `QString` | Deployment diagram instance name |
| `m_isInstance` | `bool` | Is component instance |
| `m_showStereotype` | `Uml::ShowStereoType::Enum` | Stereotype display mode |
| `m_resizable` | `bool` | Can resize |
| `m_fixedAspectRatio` | `bool` | Lock aspect ratio |
| `m_minimumSize` / `m_maximumSize` | `QSizeF` | Size constraints |
| `m_Assocs` | `AssociationWidgetList` | Connected associations |
| `m_activated` | `bool` | Post-load init done |
| `m_ignoreSnapToGrid` | `bool` | Per-widget grid skip |

### Composition: `DiagramProxyWidget`

`UMLWidget` also inherits from `DiagramProxyWidget` (mixin class), which supports **embedded diagram rendering** — a diagram can be embedded inside another diagram's widget as a live preview.

### Concrete Widgets: Visual Variety

Each widget type uses pure `QPainter` 2D calls for rendering:

| Widget | Render Approach | Sub-enum |
|--------|----------------|----------|
| `ClassifierWidget` | `drawRect` + compartment lines + text rows | `VisualProperty` flags |
| `StateWidget` | `drawRoundedRect` / `drawEllipse` / `drawRect` | `StateType` (12) |
| `ActivityWidget` | `drawRoundedRect` / `drawEllipse` / `drawRect` | `ActivityType` (7) |
| `CombinedFragmentWidget` | `drawRect` + pentagon tab | `CombinedFragmentType` (9) |
| `ActorWidget` | Stick-figure: `drawEllipse` + `drawLine` | None |
| `UseCaseWidget` | `drawEllipse` + text | None |
| `NoteWidget` | Folded-corner rectangle | None |
| `ObjectWidget` | `drawRect` + underline | None |
| `FloatingTextWidget` | `drawText` only | `TextRole` enum |
| `MessageWidget` | Arrow lines (sync/async/lost/found/creation/destroy) | `SequenceMessage` |
| `PinWidget` / `PortWidget` | Small rect/square on parent boundary | None |

### Observations

- **Deep inheritance hierarchy** (5 levels deep) makes it hard to add new widget types.
- **`QGraphicsObjectWrapper`** exists purely to work around Qt's non-virtual `setSelected()`.
- **`as*()` downcast methods** are a code smell — one per widget type, repeated for const/non-const.
- **`WidgetType` enum** with 29 values is used for dispatch but forces switch/match chains.
- **Consolidated visual state** in `WidgetBase` works well (color, font, line width).
- **`m_rect`** is always stored with origin at (0,0) — position comes from QGraphicsItem's `pos()`.

---

## 3. Rendering Model

### Rendering Approach

**Pure `QPainter` 2D** — no SVG, no OpenGL, no compositing. Every widget uses:
- `painter->drawRect()`, `painter->drawRoundedRect()`, `painter->drawEllipse()`
- `painter->drawText()` (with `QFontMetrics` for text sizing)
- `painter->drawLine()`, `painter->drawPolyline()`
- `painter->setPen()` / `painter->setBrush()` for visual styles

### Paint Method Pattern

Every widget overrides:
```cpp
virtual void paint(QPainter *painter,
                   const QStyleOptionGraphicsItem *option,
                   QWidget *widget = nullptr);
```

`WidgetBase::paint()` provides the base that:
1. Sets pen from settings
2. Draws selection markers (8 small rectangles around bounds)
3. Calls derived `paintWidget()` (in some widgets) or custom paint logic

### Selection Markers

Drawn in `WidgetBase::paint()` — 8 small rectangles at corners and midpoints of bounding rect, filled yellow when selected. This is a common pattern used in Qt diagram editors.

### Text Rendering

- **`QFontMetrics`** is used for text measurement (pre-cached in `UMLWidget::m_pFontMetrics[FT_INVALID]` for 8 font types).
- **`forceUpdateFontMetrics()`** recalculates when font changes.
- Text is drawn with `painter->drawText()` within computed rects.

### Background Rendering

`UMLScene::drawBackground()`:
1. Fills with background color
2. Calls `LayoutGrid::paint()` for grid dots/crosses

`UMLScene::drawForeground()`:
- Draws alignment guide lines (via `AlignmentGuide::activeGuides()`)

### Export

- `UMLScene::getDiagram(QPainter&, QRectF)` renders to arbitrary QPainter (used for export).
- `UMLViewImageExporter` handles file export (PNG, etc.).

### Performance Considerations

- **No dirty rect / damage tracking** — `QGraphicsScene` provides this internally.
- **No GPU acceleration** — pure software `QPainter`.
- **No batching** — each widget draws independently.
- **`QGraphicsItem::paint()` is called per frame** for visible items.
- **`QGraphicsScene` uses BSP tree** for spatial indexing (now deprecated, replaced by `QGraphicsSceneIndex`).

### Observations

- The rendering model is simple and predictable — each widget draws its own shape from scratch.
- No batching, no GPU, no retained-mode scene graph.
- Font metrics are cached, which is important for performance during resize.
- Text layout is always simple single-line `drawText` within a rect — no rich text, no word wrap (except possibly in `ClassifierWidget` compartments).

---

## 4. Association/Line Drawing System

### Architecture

The association system has three main components:

```
AssociationWidget : WidgetBase, LinkWidget    (logical association: type, roles, endpoints)
  └── AssociationLine : QGraphicsObject       (geometry: points, symbols, layout)
        ├── Symbol : QGraphicsItem            (arrowheads: open, closed, crow, diamond, etc.)
        ├── Symbol (end symbol, via m_endSymbol)
        ├── Symbol (start symbol, via m_startSymbol)
        ├── Symbol (subset symbol, optional)
        └── QGraphicsLineItem (collaboration line, optional)
```

**Key files:**
- `umbrello/umlwidgets/associationwidget.h/.cpp` — AssociationWidget
- `umbrello/umlwidgets/associationline.h/.cpp` — AssociationLine
- `umbrello/umlwidgets/associationwidgetrole.h/.cpp` — AssociationWidgetRole
- `umbrello/umlwidgets/linkwidget.h/.cpp` — LinkWidget interface

### AssociationWidget

**Responsibilities:**
- Owns two `AssociationWidgetRole` objects (role A, role B).
- Owns `AssociationLine` for geometry.
- Manages `FloatingTextWidget` instances for name, role names, multiplicity, changeability.
- Handles association class line (for "association class" UML pattern).
- Syncs to/from `UMLAssociation` (the model object).
- Handles mouse/hover/context events on the association.

**AssociationWidgetRole** (per endpoint):
- `FloatingTextWidget* multiplicityWidget`
- `FloatingTextWidget* changeabilityWidget`
- `FloatingTextWidget* roleWidget`
- `QPointer<UMLWidget> umlWidget` — the connected widget
- `Uml::Region::Enum m_WidgetRegion` — which side of the widget (North/South/East/West)
- `int m_nIndex, m_nTotalCount` — position among multiple associations on same side

### AssociationLine

**QGraphicsObject** representing the geometric line:

Data:
- `QVector<QPointF> m_points` — the line path points
- `Uml::LayoutType::Enum m_layout` — one of:
  - `Direct` — straight line between endpoints
  - `Orthogonal` — axis-aligned segments
  - `Polyline` — user-placed intermediate points
  - `Spline` — bezier curve through points
- `Symbol* m_startSymbol, m_endSymbol` — arrowheads
- `Symbol* m_subsetSymbol` — optional subset notation
- `QGraphicsLineItem* m_collaborationLineItem` — parallel arrow for collaboration diagrams
- `int m_activePointIndex` — which point is being dragged
- `int m_activeSegmentIndex` — which segment is active

**Layout generation methods:**
- `createBezierCurve(QVector<QPointF>)` — returns `QPainterPath` with cubic bezier segments
- `createOrthogonalPath(QVector<QPointF>)` — returns axis-aligned `QPainterPath`
- `createSplinePoints()` — computes smooth spline through control points
- `optimizeLinePoints()` — removes redundant collinear points

**Symbol Types** (arrowheads):
- `None`, `OpenArrow`, `ClosedArrow`, `CrowFeet`, `Diamond`, `Subset`, `Circle`

Symbol is rendered with `painter->drawPath()` using a pre-computed `QPainterPath` stored in a shared `SymbolProperty` table.

### FloatingTextWidget

- `UMLWidget` subclass used for labels on associations and messages.
- Stored in `AssociationWidgetRole` (multiplicity, changeability, role name) and directly in `AssociationWidget` (name).
- Position is relative to the association line.
- Supports constraints: `constrainTextPos()` keeps text on the correct side of the line.

### LinkWidget Interface

Abstract interface implemented by both `AssociationWidget` and `MessageWidget`:
- `lwSetFont()`, `operation()`, `setOperation()`, `customOpText()`, `setCustomOpText()`
- `resetTextPositions()`, `setMessageText()`, `setText()`
- `constrainTextPos()`, `calculateNameTextSegment()`
- `sequenceNumber()` support

### Event Handling on Associations

`AssociationLine::mousePressEvent`:
1. Checks if click is near a point → starts point drag (`m_activePointIndex`)
2. Checks if click is near a segment midpoint → inserts new point
3. Otherwise → prepares line drag

`AssociationLine::mouseMoveEvent`:
- Drags active point or entire line

### Observations

- **Edge routing is complex:** 4 layout modes, each with different geometry.
- **Self-associations** have special path computation (`createPointsSelfAssociation()`).
- **Association class** adds a secondary line from the association midpoint to a classifier.
- **Symbols use static table** for performance (shared `SymbolProperty`).
- **`FloatingTextWidget`** positioning is non-trivial — must stay on correct side of line segments.
- **No label overlap avoidance** in the current implementation.
- **`AssociationLine`** is a `QGraphicsObject` separate from `AssociationWidget` — the `AssociationWidget` remains the logical owner, while `AssociationLine` handles geometry and paint.

---

## 5. Layout System

### Grid Layout

**`LayoutGrid`** (`umbrello/umlwidgets/layoutgrid.h/.cpp`):
- Drawn in `UMLScene::drawBackground()`.
- Configurable spacing (`m_gridSpacingX`, `m_gridSpacingY`), color, visibility.
- Two rendering modes: dots or crosses.
- Spacing is uniform in both axes.

### Alignment Guides

**`AlignmentGuide`** (`umbrello/umlwidgets/alignmentguide.h/.cpp`):
- Provides live snapping during drag.
- Compares widget edges/centers against all other widgets.
- Guide types: `LeftEdge`, `RightEdge`, `HorizontalCenter`, `TopEdge`, `BottomEdge`, `VerticalCenter`.
- `QPointF snapPosition(widget, proposedPos)` — returns snapped position.
- `m_activeGuides` — list of guide lines to render in foreground.
- `m_snapThreshold` — configurable distance for snap activation.
- Uses zoom-adjusted threshold (maintains consistent screen-space distance).

### Automatic Layout

**`LayoutGenerator : DotGenerator`** (`umbrello/layoutgenerator.h/.cpp`):
- External process: invokes **Graphviz `dot`** executable.
- Workflow:
  1. Write diagram state to temporary `.dot` file (format derived from `.desktop` config file per diagram type).
  2. Run `dot -Tplain` on the file.
  3. Parse output (positions, edge routing).
  4. Apply computed positions to widgets and simplify association line points.
- Config files stored in `umbrello/layouts/` resource directory.
- Diagram-type-specific config selection via `<DiagramType>.desktop` files.
- `QHash<QString, QRectF> m_nodes` — parsed node positions.
- `QHash<QString, EdgePoints> m_edges` — parsed edge points.

### Observations

- **Auto-layout requires external dependency** (graphviz `dot` binary at runtime).
- **No built-in layout algorithms** — purely dependent on Graphviz.
- **Layout config files** are `/desktop`-format INI-like files.
- **Snap-to-grid** operates on widget position and size (`snapComponentSizeToGrid`).
- **Alignment guides** are a recent addition (2025) and address a real UX need.
- **No force-directed layout, no tree layout, no layered layout** are implemented natively.

---

## 6. Interactive Editing

### Toolbar State Pattern

**State machine** (`umbrello/toolbarstate*.h/.cpp`):

```
ToolBarState (abstract base)
  ├── ToolBarStatePool (cached, shared)
  │     ├── ToolBarStateAssociation
  │     ├── ToolBarStateMessages
  │     ├── ToolBarStateOther
  │     └── ToolBarStateOneWidget
  └── ToolBarStateArrow
```

**`ToolBarState`** (base class):
- Pure virtual mouse event handlers:
  - `mousePress()` → dispatches to `mousePressWidget() / mousePressAssociation() / mousePressEmpty()`
  - `mouseRelease()` → dispatches similarly
  - `mouseDoubleClick()` → dispatches similarly
  - `mouseMove()` → dispatches similarly
- `setCurrentElement()` — determines which widget/association received the event.
- `changeTool()` — auto-switch back to arrow tool after operation.

**`ToolBarStateArrow`**:
- Default/selection state.
- Handles widget selection (click, rubber-band select).
- Widget move and resize.
- Association endpoint drag.
- Text editing (double-click on text).

**`ToolBarStateAssociation`**:
- Creates new associations between widgets.
- On `mousePressEmpty`: draws a temporary floating dash line from press point.
- On `mousePressWidget`: starts creating an association from that widget.
- On `mouseReleaseWidget`: completes the association, shows properties dialog.

**`ToolBarStateMessages`**:
- For sequence/collaboration diagram messages.
- Creates `MessageWidget` between `ObjectWidget` instances.

**`ToolBarStateOther`**:
- Creates a single widget (class, actor, use case, state, etc.) on mouse press.
- On `mouseReleaseEmpty`: creates widget at that position (if diagram-appropriate).

**`ToolBarStateOneWidget`**:
- Similar to `ToolBarStateOther` but for widgets that need a "rubber-band" size selection (e.g., boxes, combined fragments).
- On `mousePressEmpty`: starts rubber-band.
- On `mouseReleaseEmpty`: creates widget with selected size.

**`ToolBarStateFactory`**:
- Caches up to 5 state objects.
- `getState(toolbarButton, scene)` — maps from `WorkToolBar::ToolBar_Buttons` to the appropriate state.

### Widget Interaction (UMLWidget)

**Mouse events on widgets:**
- `mousePressEvent`: determines if in move area (`m_inMoveArea`) or resize area (`m_inResizeArea`), records offset.
- `mouseMoveEvent`: if moving, calls `moveWidgetBy()` which constrains movement and updates position. If resizing, calls `resize()`.
- `mouseReleaseEvent`: finalizes position/size, emits `sigWidgetMoved()`, notifies `AssociationWidget::widgetMoved()`.
- `mouseDoubleClickEvent`: opens properties dialog by default.

**Resize area** (`isInResizeArea`): checks if click is within `selectionMarkerSize` pixels of the bounding rect edge (hot zones at corners and edges exactly like standard UI resize handles).

**Selection:**
- `selectSingle()`: clear others, select this.
- `selectMultiple()`: toggle this widget's selection.
- `deselect()`: remove from selection.

### Scene-Level Mouse Events

`UMLScene` overrides `QGraphicsScene` mouse events to:
1. Apply coordinate transformation (inverse world matrix).
2. Delegate to current `ToolBarState`.
3. Handle context menus.

### Context Menus

- `WidgetBase::contextMenuEvent()` — creates a `ListPopupMenu` with options appropriate to widget type.
- `AssociationWidget::contextMenuEvent()` — association-specific menu.
- `UMLScene::contextMenuEvent()` — empty-space menu (paste, select all, options).

### Copy/Paste

- `UMLDragData` / `UMLClipboard` handle serialization of selected widgets + associations.
- XMI serialization is used as the clipboard format.
- Partial paste support via `beginPartialWidgetPaste()` / `endPartialWidgetPaste()`.

### Observations

- **State pattern is well-designed** but implemented with Qt's class-per-file pattern leading to many small files.
- **Event dispatching** uses the `mousePressWidget/Association/Empty` triple — clean but somewhat rigid.
- **No gesture recognition** — pure mouse press/move/release.
- **No keyboard navigation** for widget positioning (arrow keys nudge, but no tab-to-select).
- **`ToolBarStateArrow` is the default** — most time is spent in this state.
- **Factory caches 5 state instances** — reuse is important since state creation could be expensive.
- **The `ToolBarStatePool` base** is used for states that share a pool of sub-tools (association types, message types).

---

## 7. Model→View Propagation

### UML Model Layer

```
UMLDoc (document root)
  └── UMLFolder (logical / use case / component / deployment / ER folders)
        ├── UMLObject (model elements: classes, interfaces, attributes, operations, etc.)
        └── UMLView (diagram container)
              └── UMLScene (the actual diagram)
                    ├── UMLWidget (visual representation of UMLObject)
                    └── AssociationWidget (visual representation of UMLAssociation)
```

### Object Creation Flow

```
User clicks widget button in toolbar → creates object in model:
  1. ToolBarStateOther::mousePressEmpty()
  2. UMLScene::addObject(UMLObject*)  ← creates model object
  3. Widget_Factory::createWidget(scene, obj)  ← creates matching widget
  4. UMLScene::setupNewWidget(widget)  ← positions and adds to scene

User clicks association button → creates association in model:
  1. ToolBarStateAssociation::mousePressWidget()
  2. ToolBarStateAssociation::mouseReleaseWidget()
  3. UMLObject::addAssociationEnd(UMLAssociation*)
  4. AssociationWidget::create(scene, widgetA, type, widgetB, umlobject)
```

### Signal-Based Propagation

The model notifies the view via Qt signals:

```
UMLDoc / UMLObject signals:
  - objectCreated(UMLObject*) → UMLScene::slotObjectCreated()
  - objectRemoved(UMLObject*) → UMLScene::slotObjectRemoved()

UMLWidget signals:
  - sigWidgetMoved(id) → AssociationWidget::widgetMoved(), ObjectWidget::slotWidgetMoved(), etc.

AssociationWidget signals:
  - sigAssociationRemoved() → ToolBarState::slotAssociationRemoved()
```

### `slotObjectCreated` Handler (in `UMLScene`)

1. Checks if object type is valid for current diagram type.
2. Calls `Widget_Factory::createWidget(scene, obj)` to create widget.
3. Calls `setupNewWidget(widget)` to position and add to scene.
4. Creates auto-associations (for attribute types, foreign keys, etc.).

### `slotObjectRemoved` Handler (in `UMLScene`)

1. Finds the widget associated with the removed object.
2. Removes all associations connected to that widget.
3. Removes the widget from the scene.

### Widget Factory

`Widget_Factory::createWidget(UMLScene*, UMLObject*)`:
- Maps `UMLObject::ObjectType` + diagram type to `WidgetBase::WidgetType`.
- New widget instance via `new ClassifierWidget(scene, umlObj)` etc.
- Returns `UMLWidget*` base pointer.

### Activation (Post-Load Initialization)

After loading from XMI:
1. `UMLScene::activate()` is called.
2. Each widget's `activate()` is called to resolve cross-references (IDs → pointers).
3. `AssociationWidget::activate()` resolves role widget pointers.
4. `MessageWidget::activate()` resolves `ObjectWidget` pointers.

### Undo/Redo Commands

- `cmds/` directory contains QUndoCommand subclasses for all diagram operations.
- Commands wrap both model and view changes.
- Examples: `CmdCreateDiagram`, `CmdRemoveDiagram`, `CmdWidgetDelete`, `CmdResizeWidget`.

### Observations

- **Model-view coupling** is tight — widgets hold direct `QPointer<UMLObject>` references.
- **Signal-based propagation** works but creates implicit coupling.
- **Activation pattern** (two-phase init) is needed because XMI loading creates objects with resolved IDs before pointers are valid.
- **`Widget_Factory`** is a simple mapping function, not a true factory pattern.
- **Undo/redo** wraps both model and view changes in single commands — a good pattern.

---

## 8. Rust Recommendations

### 8.1 Rendering Backend

**Replace `QGraphicsView/QGraphicsScene`** with a retained-mode or immediate-mode 2D renderer.

**Recommended approach:** A custom retained-mode canvas using **`vello`** (GPU-accelerated 2D vector graphics, built on `wgpu`).

| Component | C++ (Qt) | Rust Replacement | Rationale |
|-----------|----------|-------------------|-----------|
| View/window | `QGraphicsView` | `winit` window + `wgpu` surface | Cross-platform, GPU-accelerated |
| Scene | `QGraphicsScene` | Custom `Scene` struct with arena | Full control, no Qt dependency |
| 2D rendering | `QPainter` | `vello` / `piet` / `tiny-skia` | GPU vector graphics, text |
| Text rendering | `QPainter::drawText` + `QFontMetrics` | `cosmic-text` / `parley` + `swash` | Modern text layout, BiDi, shaping |
| Image export | `QPainter` to QPixmap | `tiny-skia` `Surface` → PNG | Deterministic pixel output |

**Architecture:**
```rust
pub trait Renderer {
    fn begin_frame(&mut self, width: u32, height: u32);
    fn draw_shape(&mut self, shape: &Shape, style: &Style);
    fn draw_text(&mut self, text: &str, pos: Point, font: &Font, size: f32);
    fn end_frame(&mut self) -> FrameResult;
}

// Concrete implementations:
pub struct VelloRenderer;    // GPU via wgpu
pub struct TinySkiaRenderer; // CPU fallback
pub struct SvgRenderer;      // Export-only
```

### 8.2 Widget System

**Replace the class hierarchy** with trait-based composition.

```rust
/// Unique identifier for a diagram widget (generational arena index).
pub type WidgetId = generational_arena::Index;

/// Core widget trait — all diagram elements implement this.
pub trait DiagramWidget {
    fn id(&self) -> WidgetId;
    fn widget_type(&self) -> WidgetType;
    fn bounds(&self) -> Rect;
    fn set_bounds(&mut self, rect: Rect);
    fn render(&self, renderer: &mut dyn Renderer, state: &ViewState);
    fn hit_test(&self, point: Point) -> bool;
}

/// Storage: generational arena instead of parent-child tree.
pub struct WidgetStore {
    pub nodes: generational_arena::Arena<Box<dyn DiagramWidget>>,
    pub edges: generational_arena::Arena<EdgeWidget>,
}
```

**Widget type as enum, not class:**
```rust
pub enum WidgetType {
    Classifier(ClassifierData),
    Actor,
    UseCase,
    State(StateData),
    Activity(ActivityData),
    Note,
    Association(AssociationData),
    // ...
}
```

**Key decisions:**
- **No deep inheritance** — flatten the hierarchy, use `enum` dispatch.
- **No `QPointer<UMLObject>`** — store a `ModelId` (weak reference to model arena).
- **No `as*()` downcasts** — use `match` on `WidgetType` with `WidgetData` enum.
- **No `QGraphicsObjectWrapper`** — the `setSelected` workaround is not needed.

### 8.3 Association / Edge System

**Replace `QGraphicsObject` lines** with path-based geometry stored in a separate edge store.

```rust
pub struct EdgeStore {
    edges: generational_arena::Arena<EdgeWidget>,
}

pub struct EdgeWidget {
    pub id: EdgeId,
    pub source: WidgetId,
    pub target: WidgetId,
    pub points: Vec<Point>,
    pub layout: LayoutType,
    pub start_symbol: SymbolType,
    pub end_symbol: SymbolType,
    pub text_labels: Vec<TextLabel>,
}

pub enum LayoutType {
    Direct,
    Orthogonal,
    Polyline,
    Spline { c1: Point, c2: Point },
}

impl EdgeWidget {
    pub fn to_path(&self) -> BezierPath { /* compute from points + layout */ }
}
```

**Benefits:**
- Edges are data, not `QGraphicsObject` instances.
- Path computation is separate from rendering.
- Hit testing is done against the computed path, not `QPainterPath::contains()`.
- Labels (`FloatingTextWidget`) are simple data structs, not widget instances.

### 8.4 Layout System

**Implement layout natively in Rust** — no external process dependency.

**Recommended crates:**
- **Graph representation:** `petgraph` — `Graph<WidgetId, EdgeId>` with directed/undirected support.
- **Layout algorithms:**
  - Force-directed: `petgraph::graphmap` + custom force simulation (Barnes-Hut for N-body)
  - Layered (Sugiyama): for class diagrams, activity diagrams — implement DAG layering
  - Tree layout: for inheritance hierarchies
  - Orthogonal routing: for association lines (use planar embedding or simple routing)

```rust
pub trait LayoutEngine {
    fn layout(&self, graph: &DiGraph<WidgetId, EdgeId>,
              widget_sizes: &HashMap<WidgetId, Size>) -> HashMap<WidgetId, Point>;
}

pub struct ForceDirectedLayout {
    pub repulsion: f32,
    pub attraction: f32,
    pub damping: f32,
    pub max_iterations: u32,
}

pub struct LayeredLayout {
    pub layer_spacing: f32,
    pub node_spacing: f32,
}

pub struct TreeLayout {
    pub orientation: Orientation,
    pub sibling_spacing: f32,
}
```

**Implement as separate crate** (`umbrello-layout`) with the `petgraph` dependency.

### 8.5 Interaction System

**Replace `ToolBarState` class hierarchy** with an enum-based state machine.

```rust
pub enum InteractionMode {
    Arrow(ArrowState),
    CreateAssociation(AssocState),
    CreateMessage(MessageState),
    CreateWidget(WidgetCreationMode),
    CreateRubberBand(RubberBandState),
}

pub struct ToolState {
    pub mode: InteractionMode,
    pub scene: SceneId,
    pub hovered: Option<HitTarget>,
    pub selected: HashSet<WidgetId>,
    pub drag: Option<DragState>,
}

pub enum HitTarget {
    Widget(WidgetId),
    Edge(EdgeId),
    EdgePoint(EdgeId, usize),      // specific point on edge
    EdgeSegment(EdgeId, usize),    // edge segment midpoint (for insertion)
    Handle(WidgetId, HandleType),  // resize / rotate handle
    None,
}

impl ToolState {
    pub fn handle_event(&mut self, event: &InputEvent) -> Action {
        match (&mut self.mode, event) {
            (InteractionMode::Arrow(state), InputEvent::MousePress(pos)) => {
                // selection, drag start, resize start
            }
            (InteractionMode::CreateWidget(mode), InputEvent::MouseRelease(pos)) => {
                // create widget at position
            }
            // ...
        }
    }
}
```

**State machine benefits:**
- Single enum dispatch (no virtual methods).
- Data and logic co-located.
- Easy to serialize/replay.
- No factory caching needed.

**Event system:** Use `winit` for window events, route through a dedicated `EventHandler`:
```rust
pub trait EventHandler {
    fn on_mouse_press(&mut self, pos: Point, button: MouseButton);
    fn on_mouse_move(&mut self, pos: Point);
    fn on_mouse_release(&mut self, pos: Point, button: MouseButton);
    fn on_key(&mut self, key: VirtualKeyCode, state: ElementState);
}
```

### 8.6 Storage Architecture

**Replace Qt parent-child tree** with generational arenas.

```rust
pub struct Diagram {
    pub widgets: WidgetArena,
    pub edges: EdgeArena,
    pub model_refs: ModelRefMap,
    pub grid: GridConfig,
    pub guides: GuideState,
    pub tool: ToolState,
    pub selection: SelectionState,
}

// Generational arena prevents dangling pointer bugs.
type WidgetArena = generational_arena::Arena<WidgetData>;
type EdgeArena = generational_arena::Arena<EdgeData>;

// WidgetData uses enum dispatch instead of downcasting.
pub enum WidgetData {
    Classifier(ClassifierData),
    Actor(ActorData),
    State(StateData),
    // ...
}
```

**Why arenas:**
- Cache-friendly iteration.
- Stable indices (no dangling references after removal).
- Generational check prevents use-after-free.
- Easy to implement undo/redo (store snapshots of arena state).
- No `Arc<Mutex<...>>` required for single-threaded rendering.

### 8.7 Model↔View Separation

**Keep model and view in separate arenas** with weak reference IDs.

```rust
pub struct ModelId(u64);

pub struct ModelObject {
    pub id: ModelId,
    pub name: String,
    pub object_type: ObjectType,
    // ...
}

pub struct ViewWidget {
    pub widget_id: WidgetId,
    pub model_id: Option<ModelId>,  // None for purely visual widgets
    pub bounds: Rect,
    pub style: WidgetStyle,
    // ...
}

// Propagation via message passing, not signals/slots.
pub enum ModelEvent {
    ObjectCreated(ModelId),
    ObjectRemoved(ModelId),
    ObjectRenamed(ModelId, String),
    AssociationCreated(ModelId, ModelId, ModelId),
    AttributeAdded(ModelId, AttributeData),
    // ...
}

pub enum ViewEvent {
    WidgetMoved(WidgetId, Point),
    WidgetResized(WidgetId, Size),
    WidgetDeleted(WidgetId),
    AssociationDeleted(EdgeId),
    // ...
}
```

**Propagation loop:**
```rust
pub struct DiagramController {
    model: ModelStore,
    view: Diagram,
    event_queue: Vec<SystemEvent>,
}

impl DiagramController {
    pub fn sync(&mut self) {
        for event in self.event_queue.drain(..) {
            match event {
                SystemEvent::Model(ModelEvent::ObjectCreated(id)) => {
                    if let Some(obj) = self.model.get(id) {
                        let widget = WidgetFactory::create(&obj);
                        self.view.widgets.insert(widget);
                    }
                }
                SystemEvent::View(ViewEvent::WidgetMoved(id, pos)) => {
                    if let Some(widget) = self.view.widgets.get_mut(id) {
                        widget.bounds = Rect::new(pos, widget.bounds.size());
                    }
                }
                // ...
            }
        }
    }
}
```

**Benefits:**
- No signal/slot overhead.
- Events are data — can be logged, serialized, or replayed.
- Undo/redo works by reversing event operations.
- Thread-friendly (events can cross thread boundaries).

### 8.8 Recommended Project Structure

```
umbrello-core/
  src/
    scene/         — Scene, ViewState, camera
    widgets/       — WidgetData enum, widget types, rendering
    edges/         — EdgeData, path computation, symbols
    interaction/   — ToolState, gesture handling, selection
    layout/        — Grid, guides, auto-layout engines
    export/        — Image export, print
    model/         — Model objects (if not separate crate)
    render/        — Renderer trait + backends
  Cargo.toml       — depends on vello, tiny-skia, winit, cosmic-text

umbrello-layout/
  src/
    force_directed.rs
    layered.rs
    tree.rs
    orthogonal.rs
  Cargo.toml       — depends on petgraph, nalgebra

umbrello-model/
  src/
    objects.rs     — UMLObject, Class, Interface, etc.
    associations.rs
    package.rs
    document.rs
  Cargo.toml       — standalone, no rendering deps
```

### 8.9 Key Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| `vello` is experimental | Implement `tiny-skia` fallback from day one |
| Text layout complexity | Use `cosmic-text` for all text; avoid QPainter's simple `drawText` |
| Association line routing | Port Orthogonal + Spline algorithms; defer to Graphviz for complex cases initially |
| Performance of pure Rust layout | Use `petgraph` + simulation; benchmark against Graphviz `dot` |
| Missing Qt features (printing, QPicture, drag-drop) | Handle in platform layer; print via SVG/PDF export |
| Event handling correctness | Write exhaustive tests for ToolState state machine |
| Undo/redo complexity | Use event-log approach: every mutation produces a reversible event |

### 8.10 Migration Strategy

1. **Phase 1 — Core renderer:**
   - Implement `Renderer` trait with `tiny-skia` backend.
   - Implement `Window` with `winit`.
   - Implement basic scene with rect, ellipse, text drawing.

2. **Phase 2 — Widget system:**
   - Define `WidgetData` enum and `WidgetStore` arena.
   - Implement `ClassifierWidget`, `ActorWidget`, `StateWidget` rendering.
   - Implement hit testing.

3. **Phase 3 — Edges:**
   - Define `EdgeData` and `EdgeStore`.
   - Implement Direct, Orthogonal, Spline layout.
   - Implement arrowhead symbols.

4. **Phase 4 — Interaction:**
   - Implement `ToolState` state machine.
   - Implement selection, move, resize.
   - Implement association creation.

5. **Phase 5 — Model integration:**
   - Implement `ModelEvent` / `ViewEvent` propagation.
   - Implement two-phase activation.
   - Implement XMI load/save.

6. **Phase 6 — Layout:**
   - Implement grid, alignment guides.
   - Implement force-directed layout.
   - Port orthogonal routing.

7. **Phase 7 — Polish:**
   - Implement image export.
   - Implement undo/redo.
   - Performance optimization (dirty rect tracking, spatial indexing).

---

## Appendix A: Key File Index

| File | Purpose |
|------|---------|
| `umbrello/umlview.h/.cpp` | `UMLView : QGraphicsView` |
| `umbrello/umlscene.h/.cpp` | `UMLScene : QGraphicsScene` |
| `umbrello/umlwidgets/widgetbase.h/.cpp` | `WidgetBase : QGraphicsObjectWrapper` |
| `umbrello/umlwidgets/umlwidget.h/.cpp` | `UMLWidget : WidgetBase` |
| `umbrello/umlwidgets/classifierwidget.h/.cpp` | `ClassifierWidget : UMLWidget` |
| `umbrello/umlwidgets/associationwidget.h/.cpp` | `AssociationWidget : WidgetBase, LinkWidget` |
| `umbrello/umlwidgets/associationline.h/.cpp` | `AssociationLine : QGraphicsObject` |
| `umbrello/umlwidgets/associationwidgetrole.h/.cpp` | `AssociationWidgetRole` |
| `umbrello/umlwidgets/linkwidget.h/.cpp` | `LinkWidget` interface |
| `umbrello/umlwidgets/floatingtextwidget.h/.cpp` | `FloatingTextWidget : UMLWidget` |
| `umbrello/umlwidgets/messagewidget.h/.cpp` | `MessageWidget : UMLWidget, LinkWidget` |
| `umbrello/umlwidgets/widget_factory.h/.cpp` | `Widget_Factory` namespace |
| `umbrello/umlwidgets/layoutgrid.h/.cpp` | `LayoutGrid` |
| `umbrello/umlwidgets/alignmentguide.h/.cpp` | `AlignmentGuide` |
| `umbrello/umlwidgets/diagramproxywidget.h/.cpp` | `DiagramProxyWidget` mixin |
| `umbrello/umlwidgets/pinportbase.h/.cpp` | `PinPortBase` |
| `umbrello/toolbarstate.h/.cpp` | `ToolBarState` (base) |
| `umbrello/toolbarstatearrow.h/.cpp` | `ToolBarStateArrow` |
| `umbrello/toolbarstateassociation.h/.cpp` | `ToolBarStateAssociation` |
| `umbrello/toolbarstatemessages.h/.cpp` | `ToolBarStateMessages` |
| `umbrello/toolbarstateother.h/.cpp` | `ToolBarStateOther` |
| `umbrello/toolbarstateonewidget.h/.cpp` | `ToolBarStateOneWidget` |
| `umbrello/toolbarstatepool.h/.cpp` | `ToolBarStatePool` |
| `umbrello/toolbarstatefactory.h/.cpp` | `ToolBarStateFactory` |
| `umbrello/layoutgenerator.h/.cpp` | `LayoutGenerator : DotGenerator` |
| `umbrello/worktoolbar.h/.cpp` | `WorkToolBar : KToolBar` |
| `umbrello/umlmodel/umlfolder.h/.cpp` | `UMLFolder : UMLPackage` (owns views) |

## Appendix B: Data Flow Diagrams

### Widget Creation Flow
```
User clicks toolbar button
  → WorkToolBar emits sigButtonChanged(tbb_Class)
    → UMLScene::slotToolBarChanged()
      → ToolBarStateFactory::getState(tbb_Class) → ToolBarStateOther
      → Scene sets current state
User clicks on diagram
  → ToolBarStateOther::mousePressEmpty()
    → UMLScene::addObject(new UMLClassifier())
      → UMLDoc signals: objectCreated(UMLClassifier*)
        → UMLScene::slotObjectCreated(UMLClassifier*)
          → Widget_Factory::createWidget(scene, classifier) → new ClassifierWidget
          → UMLScene::setupNewWidget(widget)
```

### Association Creation Flow
```
User clicks association button
  → ToolBarStateAssociation active
User clicks widget A
  → ToolBarStateAssociation::mousePressWidget()
    → Creates FloatingDashLineWidget from click point
User moves mouse → draws temporary line
User releases on widget B
  → ToolBarStateAssociation::mouseReleaseWidget()
    → AssociationWidget::create(scene, widgetA, type, widgetB)
    → UMLScene::addAssociation(associationWidget)
    → Optionally: create UMLAssociation in model
    → Show properties dialog
```

### Paint/Render Flow
```
Qt frame cycle:
  → QGraphicsView::paintEvent()
    → QGraphicsScene::drawBackground()  (grid)
      → LayoutGrid::paint()
    → QGraphicsScene::drawItems()        (all widgets)
      → AssociationWidget::paint()
        → AssociationLine::paint()       (line + symbols)
      → ClassifierWidget::paint()        (rect + compartments + text)
      → StateWidget::paint()             (custom shape per state type)
      → FloatingTextWidget::paint()      (text)
    → QGraphicsScene::drawForeground()
      → AlignmentGuide lines
```
