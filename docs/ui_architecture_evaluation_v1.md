# UI Architecture Evaluation — v1

**Date:** 2026-06-23  
**Author:** Architecture Team  
**Status:** Draft  
**Scope:** Rust GUI framework selection for Umbrello-RS (heavy-canvas UML diagramming tool)

---

## 1. Requirements

Umbrello-RS needs a desktop GUI that can host a freeform 2D canvas for UML diagramming — a **heavy-canvas** application, not a standard form-based UI.

### 1.1 Window Layout

```
┌─────────────────────────────────────────────────────┐
│  Menu Bar (File, Edit, View, Code, Tools, Help)     │
├─────────────────────────────────────────────────────┤
│  Toolbar (Select, Line, Rectangle, Zoom, Undo/Redo) │
├──────────┬──────────────────────────┬───────────────┤
│          │                          │               │
│ Tree     │     Canvas               │  Properties   │
│ View     │     (scrollable,         │  Panel        │
│ (Model   │      zoomable,           │               │
│  Browser) │      paintable)         │               │
│          │                          │               │
│          │                          │               │
│          │                          │               │
├──────────┴──────────────────────────┴───────────────┤
│  Status Bar                                          │
└─────────────────────────────────────────────────────┘
```

### 1.2 Canvas Capabilities

| Requirement | Priority | Notes |
|-------------|----------|-------|
| Freeform painting (rect, ellipse, line, text at arbitrary coordinates) | **P0** | Every UML shape must be drawable |
| Pan (middle-button drag) | **P0** | Must be smooth |
| Zoom (scroll wheel, pinch) | **P0** | Must support 10%–500% |
| Click-to-select | **P0** | Hit-test against node bounds |
| Drag-to-move | **P0** | Dispatches `MoveNodeCommand` |
| Resize handles | **P1** | Selection handles at corners/edges |
| Rubber-band selection | **P2** | Drag on empty area → select multiple |
| Rulers and grid | **P3** | Snap-to-grid for alignment |

### 1.3 Architecture Constraints

| Constraint | Rationale |
|------------|-----------|
| GUI **must not own** UML data | The `UmlModel` is the single source of truth |
| All mutations go through Command/History | Undo/redo requires an audit trail |
| GUI reads model (immutable borrow), dispatches commands (mutable) | Thread safety via borrow model |
| Cross-platform: Linux, Windows, macOS | Must compile on all three |
| Rust toolchain: 1.92.0 | Minimum supported version |
| Minimize `unsafe` code | Safety guarantee for the rewrite |

### 1.4 Data Flow Contract

```
User input (mouse/keyboard)
       │
       ▼
┌──────────────────┐
│  UI Event Handler │  — reads model state, creates Command
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  History          │  — executes Command, records for undo
│  (Vec<Command>)   │
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  UmlModel         │  — single source of truth, updated in place
└────────┬─────────┘
         │
         ▼
┌──────────────────┐
│  Next frame       │  — GUI re-reads model, renders fresh
└──────────────────┘
```

No signals. No callbacks. No bindings. Every frame is a clean render from current model state.

---

## 2. Candidates

Four Rust GUI ecosystems were evaluated against the requirements above.

### 2.1 egui (Immediate Mode)

**Version:** 0.31.x  
**Rendering backend:** eframe (glow/wgpu)  
**License:** MIT/Apache-2.0  
**Repository:** <https://github.com/emilk/egui>

#### 2.1.1 Architecture

Immediate mode. Every frame, the UI tree is built from scratch by executing the `update()` callback. No retained widget tree exists between frames. Widget state (IDs, focus, drag) is tracked by egui internally via a `Context` that persists across frames.

```rust
impl eframe::App for UmbrelloApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Every frame: read model, paint, handle input
        egui::CentralPanel::default().show(ctx, |ui| {
            let diagram = self.model.get_diagram(self.active_diagram).unwrap();
            for (_, node) in &diagram.nodes {
                let rect = egui::Rect::from_min_size(
                    egui::pos2(node.bounds.x() as f32, node.bounds.y() as f32),
                    egui::Vec2::new(node.bounds.width() as f32, node.bounds.height() as f32),
                );
                // Paint
                ui.painter().rect_filled(rect, 0.0, fill_color);
                ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, &name, font, text_color);
                // Interact
                let response = ui.allocate_rect(rect, egui::Sense::drag());
                if let Some(delta) = response.drag_delta() {
                    let cmd = MoveNode::new(node.id, delta.x as f64, delta.y as f64);
                    self.history.execute(cmd, &mut self.model);
                }
            }
        });
    }
}
```

#### 2.1.2 Evaluation

| Criterion | Score | Notes |
|-----------|-------|-------|
| Canvas painting | ★★★★★ | `egui::Painter` — rect, circle, line, text, path, mesh. Arbitrary coordinates. |
| Pan & zoom | ★★★★★ | Built-in `ctx.zoom_factor()`, scroll-to-zoom, middle-button pan via `ui.input()`. |
| Mouse interaction | ★★★★★ | `Sense::drag()`, `Sense::click()`, `Sense::everything()`. Hit-testing is allocation-based. |
| Resize handles | ★★★★☆ | Manual implementation needed (allocate small rects at corners) but straightforward. |
| Dockable panels | ★★★★☆ | `egui_dock` crate provides tree + property panel docking. Production-quality. |
| Menus & toolbars | ★★★★☆ | `egui::menu::bar()`, toolbars via horizontal layout. Not native but functional. |
| Tree view | ★★★☆☆ | No built-in tree widget. `egui_tiles` or hand-rolled collapsible sections. Adequate for model browser. |
| State sync cost | ★★★★★ | **Zero.** The model is the source of truth. No bindings, no signals. |
| Performance (1000 nodes) | ★★★★☆ | Immediate mode repaints everything. Each rect is ~50–100 µs. 1000 nodes = 50–100 ms/frame. Acceptable. Scrolling may drop frames — use `ui.clip_rect()` culling. |
| Performance (10k nodes) | ★★★☆☆ | Would require spatial indexing (quadtree) and visibility culling. Not needed for typical UML diagrams (100–500 nodes). |
| Look & feel | ★★★☆☆ | egui's flat style is recognizable as non-native. Users who expect KDE/Qt look may notice. Skinnable via `ctx.set_style()`. |
| Native file dialogs | ★★★★☆ | `rfd` (Rust File Dialogs) crate integrates cleanly. |
| Accessibility | ★★☆☆☆ | No screen reader support. Not a priority for M14. |
| Maturity | ★★★★★ | 30k+ GitHub stars. Production use: Rerun, Bevy editor, TensorBoard. |
| Cross-platform | ★★★★★ | Linux (X11/Wayland), Windows, macOS, Web. |

#### 2.1.3 Pros

- **Zero state synchronization.** The GUI reads `UmlModel` every frame. No signal/slot, no bindings, no event loops to synchronize. This single property eliminates an entire class of bugs.
- **Natural fit for canvas.** The `Painter` API was designed for exactly this use case: arbitrary 2D shapes with text labels.
- **Input handling is trivial.** `Sense::drag()` returns the delta directly. No manual event tracking, no hit-test `switch` statements.
- **Pan and zoom are built-in.** Not an afterthought — they're first-class `Context` operations.
- **Fast prototyping.** No markup, no DSL, no code generation. Pure Rust.
- **Docking ecosystem.** `egui_dock` and `egui_tiles` cover the multi-panel layout requirement.

#### 2.1.4 Cons

- **Constant redraw.** Every frame paints everything visible. For >2000 nodes, optimization is required (spatial culling, dirty regions).
- **Not a native look.** egui's style is distinctive — it doesn't match GTK, Qt, or platform-native themes.
- **Tree view is not built-in.** Must implement or use `egui_tiles`. Acceptable for M14.
- **Text rendering uses own font atlas.** Not system fonts, but the default bundled fonts (Hack, Ubuntu, Noto Emoji) are adequate.
- **No built-in accessibility.** Not a blocker for initial release.

#### 2.1.5 Conclusion

**Best fit.** The immediate-mode architecture eliminates the hardest problem in GUI-for-model applications: keeping UI state in sync with model state. The canvas APIs are mature and well-documented.

---

### 2.2 Slint (Declarative/Retained)

**Version:** 1.x  
**License:** GPL-3.0 or commercial  
**Repository:** <https://github.com/slint-ui/slint>

#### 2.2.1 Architecture

Declarative markup (`.slint` files) compiled to Rust. Retained widget tree — the framework owns the widget hierarchy and diffs updates.

```slint
// model.slint
export component Canvas {
    // How would we express "for each diagram node, draw a Rect at node.bounds"?
    // Slint has `for item in model: Rectangle { ... }` but the model must be a
    // SlintModel, not our UmlModel.
}
```

#### 2.2.2 Evaluation

| Criterion | Score | Notes |
|-----------|-------|-------|
| Canvas painting | ★★☆☆☆ | `Flickable` with manually painted shapes via `Canvas` element. No immediate-mode paint API. |
| Pan & zoom | ★★★★☆ | `Flickable` provides smooth scrolling. Zoom requires manual transform on paint. |
| Mouse interaction | ★★★☆☆ | Touch areas on each element. Works but verbose for 100s of elements. |
| State sync cost | ★★☆☆☆ | Must manually copy `UmlModel` data into Slint models or use `slint::Model`. Two representations to keep in sync. |
| Looking native | ★★★★★ | Excellent native integration (Qt, GTK, or platform renderer). |
| License | ★★☆☆☆ | GPL-3.0 requires commercial license for proprietary distribution. |
| Ecosystem | ★★★☆☆ | Smaller community, fewer examples for canvas-heavy apps. |

#### 2.2.3 Pros

- **Native look and feel.** Slint renders using platform-native widgets or a polished custom style.
- **Excellent for forms and panels.** Property panel, dialogs, settings — Slint excels here.
- **Strong compile-time validation.** `.slint` syntax checked at compile time.

#### 2.2.4 Cons

- **Freeform canvas is a poor fit.** Slint is designed for widget-based UIs with structured layouts. Arbitrary rectangles at arbitrary positions require fighting the layout system.
- **Two representations.** The `.slint` model is separate from `UmlModel`. Must keep them synchronized — the exact problem we want to avoid.
- **GPL-3.0 license.** May be acceptable for GPL projects but restricts downstream use.
- **Markup + Rust split.** Adds cognitive load — some logic in `.slint`, some in Rust.

#### 2.2.5 Conclusion

**Not recommended for the canvas.** Slint could be a good choice for standalone property panels or dialogs in a hybrid architecture, but as the primary GUI framework it introduces state synchronization problems that egui eliminates entirely.

---

### 2.3 Iced (Reactive/Elm-like)

**Version:** 0.13.x  
**License:** MIT  
**Repository:** <https://github.com/iced-rs/iced>

#### 2.3.1 Architecture

Elm Architecture — pure functional update cycle: `Model → View → Update → (Model, Command<Message>)`.

```rust
// Iced's update function is pure — it returns a new model
fn update(state: &mut State, msg: Message) -> Command<Message> {
    match msg {
        Message::NodeDragged { id, dx, dy } => {
            let cmd = MoveNode::new(id, dx, dy);
            state.history.execute(cmd, &mut state.model);
            Command::none()
        }
    }
}
```

#### 2.3.2 Evaluation

| Criterion | Score | Notes |
|-----------|-------|-------|
| Canvas painting | ★★★☆☆ | `iced_widget::Canvas` with custom `Program` trait. Less ergonomic than egui's `Painter`. |
| Pan & zoom | ★★★☆☆ | Manual implementation via `Canvas` transform and `MouseScroll` events. |
| Mouse interaction | ★★★☆☆ | Hit-testing is manual in the `Canvas::draw()` callback. |
| State sync cost | ★★★★☆ | Elm architecture naturally separates model (UmlModel) from UI (Widget tree). Good architectural match. |
| Ecosystem | ★★☆☆☆ | Breaking changes between minor versions. Few production canvas examples. |
| Maturity | ★★★☆☆ | Still stabilizing APIs. 0.13 indicates pre-1.0 volatility. |

#### 2.3.3 Pros

- **Architecturally clean.** The Elm model/view/update cycle maps cleanly to our Command pattern.
- **Pure functions.** Easy to test, easy to reason about.

#### 2.3.4 Cons

- **Canvas API is immature.** Custom `Program` trait for drawing requires boilerplate.
- **Ecosystem churn.** Breaking changes between 0.12 → 0.13 would require significant porting effort.
- **Smaller community.** Fewer examples, fewer blog posts, fewer Stack Overflow answers for diagram/canvas problems.
- **Text rendering overhead.** Custom font setup is more involved than egui.

#### 2.3.5 Conclusion

**Promising, but not yet production-ready for a diagram canvas.** If Iced stabilizes its Canvas API and reaches 1.0, it could become a strong contender. For 2026, the API volatility risk is too high.

---

### 2.4 Tauri + Web Frontend

**Architecture:** Rust backend (Tauri) + Web frontend (HTML5 Canvas, SVG).

#### 2.4.1 Evaluation

| Criterion | Score | Notes |
|-----------|-------|-------|
| Canvas painting | ★★★★★ | HTML5 Canvas is the most capable 2D API available. |
| Pan & zoom | ★★★★★ | Trivial with Canvas 2D transforms. |
| Mouse interaction | ★★★★★ | Full mouse event API. |
| State sync cost | ★★☆☆☆ | JS ↔ Rust bridge via serialized messages. Every interaction serializes/deserializes. |
| Bundle size | ★★☆☆☆ | ~200 MB for minimal Electron-like bundle (WebView runtime). |
| Complexity | ★★☆☆☆ | Two codebases (JS, Rust), two toolchains. Build pipeline complexity. |

#### 2.4.2 Pros

- **The best canvas rendering.** HTML5 Canvas 2D is battle-tested at Google Maps, Figma, Excalidraw scale.
- **Huge ecosystem.** Any JS/TS library is available.

#### 2.4.3 Cons

- **JS bridge overhead.** Every user interaction crosses the Rust–JS boundary. Serialization cost adds latency to drag operations.
- **Two codebases.** Increases maintenance burden. Bug fixes may require changes in both.
- **Bundle size.** Even with Tauri's smaller WebView approach, the app is 10–50× larger than a native Rust binary.
- **Not idiomatic Rust.** Our core architecture is Rust — the GUI should be Rust too, not HTML/JS.

#### 2.4.4 Conclusion

**Overkill.** The web bridge overhead and dual-codebase complexity outweigh the canvas rendering benefits. Only recommended if cross-platform web deployment is required.

---

## 3. Recommendation: egui

**egui is the recommended GUI framework for Umbrello-RS.**

The single deciding factor is the **immediate-mode architecture**. Umbrello-RS's core design principle is that `UmlModel` is the single source of truth and all mutations go through `Command`/`History`. egui's immediate mode aligns perfectly with this:

| Requirement | egui alignment |
|-------------|----------------|
| GUI reads model (immutable) | `update()` reads model every frame. No sync needed. |
| GUI dispatches commands (mutable) | Event handler calls `history.execute(cmd, &mut model)`. |
| Undo changes model, GUI updates next frame | Undo modifies model → next frame re-renders. Automatic. |
| No signals/callbacks/bindings | Not needed. The frame loop replaces all of them. |

The alternatives either require maintaining a parallel GUI state (Slint), have immature canvas APIs (Iced), or introduce needless complexity (Tauri).

### 3.1 Qualification

egui is not a perfect fit for every aspect of Umbrello-RS:

- **Tree view (Model Browser):** egui has no tree widget. We will implement a minimal tree using `egui_tiles` or a custom collapsible list. This is ~200 lines of Rust — not a blocker.
- **Property panel (Table-like):** egui's `Grid` layout is sufficient. For complex property editing (multi-field, type-specific controls), a thin wrapper layer will be built.
- **Look and feel:** egui is not native. For a KDE/Qt-replacement audience, this may cause friction. Mitigation: provide a `umi` (Unified Modeling Interface) skin that approximates a professional diagramming tool appearance.

### 3.2 Prioritised Adoption Path

| Phase | Components | egui constructs |
|-------|-----------|----------------|
| M14 prototype | Window, canvas, drag, undo/redo | `eframe`, `CentralPanel`, `Painter`, `Sense::drag()` |
| M15 | Menu bar, toolbar, tree view | `menu::bar()`, `SidePanel`, custom tree widget |
| M16 | Property panel, resize handles | `SidePanel`, `Grid`, `Sense::everything()` |
| M17 | Zoom, pan, rulers, grid | `ctx.zoom_factor()`, `ctx.input().scroll_delta`, custom overlay |
| M18 | Rubber-band selection, multi-select | `Sense::click()`, `Painter::rect_stroke()` for selection rect |
| M19+ | Dockable panels, themes, preferences | `egui_dock`, `ctx.set_style()`, `egui::Style` |

---

## 4. Architecture: State Bridge

The `UmbrelloApp` struct is the single bridge between egui and the Umbrello-RS backend.

```rust
/// Top-level application state.
///
/// This is the *only* struct that owns both the backend (model, history)
/// and the ephemeral UI state (zoom, pan, drag). It is passed to egui's
/// `update()` callback every frame.
pub struct UmbrelloApp {
    // ── Backend (source of truth) ──────────────────────────────────
    /// The UML model — contains all diagrams, elements, relationships.
    /// GUI reads from this every frame. GUI never writes to it directly.
    model: UmlModel,

    /// Command history for undo/redo.
    /// GUI dispatches commands via `history.execute()`.
    history: History,

    // ── Ephemeral UI state (not persisted, not part of model) ──────
    /// Which diagram is currently shown in the canvas.
    active_diagram: Option<DiagramId>,

    /// Currently selected nodes (one or more).
    selected_nodes: Vec<UmlNodeId>,

    /// Active drag operation, if any.
    drag_state: Option<DragState>,

    /// Canvas viewport.
    zoom: f32,
    pan_offset: egui::Vec2,
}

/// Tracks an in-progress drag operation.
enum DragState {
    /// Dragging one or more selected nodes.
    Moving {
        node_ids: Vec<UmlNodeId>,
        /// Accumulated delta to dispatch as a single command on release.
        accumulated_delta: egui::Vec2,
    },
    /// Resizing a single selected node.
    Resizing {
        node_id: UmlNodeId,
        handle: ResizeHandle,
    },
    /// Rubber-band selecting.
    Selecting {
        start: egui::Pos2,
        current: egui::Pos2,
    },
}

impl eframe::App for UmbrelloApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ── 1. Top-level layout ────────────────────────────────────
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.render_menu(ui);
        });
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.render_toolbar(ui);
        });

        // ── 2. Left panel: model browser (tree view) ──────────────
        egui::SidePanel::left("tree_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                self.render_tree(ui);
            });

        // ── 3. Right panel: property editor ────────────────────────
        egui::SidePanel::right("property_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                self.render_properties(ui);
            });

        // ── 4. Center: the diagram canvas ──────────────────────────
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });

        // ── 5. Handle keyboard shortcuts ───────────────────────────
        self.handle_keyboard(ctx);
    }
}
```

### 4.1 Canvas Rendering

```rust
fn render_canvas(&mut self, ui: &mut egui::Ui) {
    // Get the diagram from the model (immutable borrow).
    let diagram = match self.model.get_diagram(self.active_diagram) {
        Some(d) => d,
        None => {
            ui.label("No diagram selected.");
            return;
        }
    };

    // Allow the canvas area to scroll and zoom.
    let (response, painter) = ui.output_painter(|ui| {
        // Apply pan and zoom to the painter's transform.
        ui.painter().zoom(self.zoom);
        ui.painter().translate(self.pan_offset);

        // Draw each UML node.
        for node in diagram.nodes() {
            let bounds = node.bounds();
            let rect = egui::Rect::from_min_size(
                egui::pos2(bounds.x() as f32, bounds.y() as f32),
                egui::Vec2::new(bounds.width() as f32, bounds.height() as f32),
            );

            // ── Fill and border ────────────────────────────────────
            let fill = node.color().unwrap_or(egui::Color32::WHITE);
            painter.rect_filled(rect, 0.0, fill);
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, egui::Color32::BLACK));

            // ── Label ──────────────────────────────────────────────
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                node.name(),
                egui::FontId::proportional(14.0),
                egui::Color32::BLACK,
            );

            // ── Selection handles ──────────────────────────────────
            if self.selected_nodes.contains(&node.id()) {
                for corner in rect.corners() {
                    let handle_rect = egui::Rect::from_center_size(corner, egui::Vec2::splat(6.0));
                    painter.rect_filled(handle_rect, 0.0, egui::Color32::BLUE);
                }
            }
        }

        // ── Draw relationships (lines) ─────────────────────────────
        for rel in diagram.relationships() {
            let start = /* ... resolve endpoint positions from model */;
            let end = /* ... */;
            painter.line_segment([start, end], egui::Stroke::new(1.0, egui::Color32::BLACK));
        }

        ui
    });

    // ── Handle mouse events on the canvas ─────────────────────────
    self.handle_canvas_input(&response, ui);
}
```

### 4.2 Mouse Interaction

```rust
fn handle_canvas_input(&mut self, response: &egui::Response, ui: &egui::Ui) {
    let ctx = ui.ctx();

    // ── Zoom (scroll wheel) ───────────────────────────────────────
    let scroll = ctx.input(|i| i.scroll_delta);
    if scroll.y != 0.0 {
        let factor = 1.0 + scroll.y * 0.001;
        self.zoom = (self.zoom * factor).clamp(0.1, 5.0);
        ctx.request_repaint();
    }

    // ── Pan (middle button drag) ──────────────────────────────────
    if response.middle_button().clicked_or_dragged() {
        let delta = ctx.input(|i| i.pointer.delta());
        self.pan_offset += delta;
        ctx.request_repaint();
    }

    // ── Node selection (left click) ───────────────────────────────
    if response.clicked() {
        let pos = ctx.input(|i| i.pointer.interact_pos()).unwrap_or_default();
        // Hit-test: find node at pos (iterate, or use spatial index)
        if let Some(node_id) = self.hit_test_node(pos) {
            self.selected_nodes.clear();
            self.selected_nodes.push(node_id);
        } else {
            self.selected_nodes.clear();
        }
        ctx.request_repaint();
    }

    // ── Node drag (left button drag on a node) ────────────────────
    if let Some(delta) = response.drag_delta() {
        if let Some(node_id) = self.selected_nodes.first() {
            // Accumulate delta; we'll dispatch one command on release.
            let entry = self.drag_state.get_or_insert_with(|| {
                DragState::Moving {
                    node_ids: self.selected_nodes.clone(),
                    accumulated_delta: egui::Vec2::ZERO,
                }
            });
            if let DragState::Moving { accumulated_delta, .. } = entry {
                *accumulated_delta += delta;
            }
            ctx.request_repaint();
        }
    }

    // ── Drag release ──────────────────────────────────────────────
    if response.drag_released() {
        if let Some(DragState::Moving { node_ids, accumulated_delta }) = self.drag_state.take() {
            if accumulated_delta.length_sq() > 0.0 {
                let cmd = MoveNode::new(
                    node_ids,
                    accumulated_delta.x as f64,
                    accumulated_delta.y as f64,
                );
                self.history.execute(cmd, &mut self.model);
            }
        }
        ctx.request_repaint();
    }
}
```

### 4.3 Keyboard Handling

```rust
fn handle_keyboard(&mut self, ctx: &egui::Context) {
    ctx.input_mut(|i| {
        use egui::Key;

        // Ctrl+Z: Undo
        if i.modifiers.ctrl && i.key_pressed(Key::Z) && !i.modifiers.shift {
            if let Ok(()) = self.history.undo(&mut self.model) {
                ctx.request_repaint();
            }
        }

        // Ctrl+Shift+Z or Ctrl+Y: Redo
        if (i.modifiers.ctrl && i.modifiers.shift && i.key_pressed(Key::Z))
            || (i.modifiers.ctrl && i.key_pressed(Key::Y))
        {
            if let Ok(()) = self.history.redo(&mut self.model) {
                ctx.request_repaint();
            }
        }

        // Delete: remove selected nodes
        if i.key_pressed(Key::Delete) {
            if !self.selected_nodes.is_empty() {
                let ids = self.selected_nodes.clone();
                let cmd = RemoveNodes::new(ids);
                self.history.execute(cmd, &mut self.model);
                self.selected_nodes.clear();
                ctx.request_repaint();
            }
        }
    });
}
```

---

## 5. Dependency Injection

egui is added to `Cargo.toml` as:

```toml
[dependencies]
# Core GUI framework
egui = "0.31"
eframe = { version = "0.31", features = ["default"] }

# Docking/tabs for multi-panel layout
egui_dock = "0.12"

# Native file dialogs
rfd = "0.15"

# Backend (already present in workspace)
umbrello-uml-model = { path = "../crates/uml-model" }
umbrello-command = { path = "../crates/umbrello-command" }
umbrello-history = { path = "../crates/umbrello-history" }
```

No additional feature flags or native dependencies. egui is a pure Rust crate.

---

## 6. Prototype Scope: M14

The M14 milestone delivers an interactive proof-of-concept:

### 6.1 Deliverables

1. **Executable** that launches a native window (800×600, resizable).
2. **XMI import**: loads `test-COG.xmi` via `uml-io` into `UmlModel`.
3. **Canvas rendering**: UML nodes displayed as colored rectangles with element names.
4. **Drag-to-move**: click-and-drag a rectangle to reposition it.
5. **Undo/Redo**: toolbar buttons and `Ctrl+Z`/`Ctrl+Shift+Z` revert/restore positions.
6. **Verification**: after Undo, the rectangle returns to its pre-drag position. Confirmed via both visual inspection and unit test.

### 6.2 Non-goals

- Relationship lines (M15).
- Tree view / model browser (M15).
- Property panel (M16).
- Zoom and pan (M17).
- Resize handles (M16).
- Rubber-band selection (M18).

### 6.3 Acceptance Criteria

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// After executing a MoveNode command and then undoing it,
    /// the node position is restored to the original value.
    #[test]
    fn test_undo_restores_node_position() {
        let mut app = UmbrelloApp::new(load_test_model());

        // Record initial position
        let diagram_id = app.active_diagram.unwrap();
        let node_id = app.model.get_diagram(diagram_id).unwrap().nodes()[0].id();
        let initial_pos = app.model.get_node(node_id).unwrap().bounds().origin();

        // Drag by (50, 30)
        let cmd = MoveNode::new(vec![node_id], 50.0, 30.0);
        app.history.execute(cmd, &mut app.model);

        let moved_pos = app.model.get_node(node_id).unwrap().bounds().origin();
        assert_ne!(initial_pos, moved_pos, "Node must move after drag");

        // Undo
        app.history.undo(&mut app.model).unwrap();

        let restored_pos = app.model.get_node(node_id).unwrap().bounds().origin();
        assert_eq!(
            initial_pos, restored_pos,
            "Node must return to original position after undo"
        );
    }
}
```

### 6.4 Estimated Effort

| Task | Est. (person-days) |
|------|--------------------|
| Create `umbrello-ui` crate with eframe boilerplate | 1 |
| Bind `UmlModel` and `History` to `UmbrelloApp` | 0.5 |
| Implement canvas rendering (rectangles + labels) | 1.5 |
| Implement drag-to-move with command dispatch | 1 |
| Implement undo/redo (keyboard + toolbar buttons) | 1 |
| XMI import integration | 0.5 |
| Tests and verification | 1 |
| **Total** | **6.5** |

---

## 7. Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| egui cannot handle >500 nodes at 60fps | Low | Medium | Spatial culling (only paint visible rects). Quadtree for hit-testing. |
| `egui_dock` is abandoned or incompatible | Low | Medium | Fall back to manual `SidePanel` + `CentralPanel` layout. |
| egui 0.32 introduces breaking changes | Medium | Low | Pin to 0.31.x. Migrate when convenient. |
| Users reject non-native look | Medium | Medium | Provide a `umi` theme with `ctx.set_visuals()`. Add Qt-like styling presets. |
| Accessibility requirement emerges | Low | High | egui has no accessibility story. Would need a separate a11y layer. |
| Mouse input feels laggy on X11/Wayland | Low | Medium | egui uses wgpu/glow which bypasses X11 input latency issues. Monitor in testing. |

---

## 8. Appendix: Test Plan (M14)

### 8.1 Unit Tests (Rust)

```
tests/
├── app_construction.rs    — UmbrelloApp::new() loads model, no crash
├── canvas_rendering.rs    — render_canvas() produces correct painter calls
├── drag_command.rs        — drag dispatches MoveNodeCommand with correct delta
├── undo_redo.rs           — undo restores position, redo reapplies
├── keyboard_shortcuts.rs  — Ctrl+Z triggers undo, Ctrl+Shift+Z triggers redo
└── hit_test.rs            — click on empty space deselects, click on node selects
```

### 8.2 Integration Test (Manual)

1. Launch `cargo run --bin umbrello-ui`
2. Verify: window appears with grey canvas area
3. File → Open → select `test-COG.xmi`
4. Verify: diagram nodes appear as colored rectangles with text labels
5. Drag a rectangle to a new position
6. Verify: rectangle follows cursor, moves smoothly
7. Ctrl+Z: rectangle returns to original position
8. Ctrl+Shift+Z: rectangle returns to dragged position
9. Close window: app exits cleanly

---

## 9. Appendix: Alternative Hybrid Architecture

If egui's look-and-feel proves unacceptable to users, a **hybrid architecture** is possible:

```
┌───────────────────────────────────────────┐
│  Slint (or native Qt via CXX)            │
│  ┌─────────────────────────────────────┐  │
│  │  Menu, toolbar, tree, property panel │  │
│  └─────────────────────────────────────┘  │
│  ┌─────────────────────────────────────┐  │
│  │  egui embedded in a Slint widget    │  │
│  │  (via slint::egui::EguiRenderer)    │  │
│  │  ┌───────────────────────────────┐  │  │
│  │  │  Canvas (UML diagram)         │  │  │
│  │  └───────────────────────────────┘  │  │
│  └─────────────────────────────────────┘  │
└───────────────────────────────────────────┘
```

This approach embeds an egui canvas inside a Slint or Qt window, giving native look for menus/panels while preserving immediate-mode canvas rendering. **Not recommended for M14** due to increased complexity, but tracked for future consideration.

---

## 10. References

- [egui documentation](https://docs.rs/egui/0.31.0/egui/)
- [eframe tutorial](https://docs.rs/eframe/0.31.0/eframe/)
- [egui_dock crate](https://crates.io/crates/egui_dock)
- [Slint canvas example](https://slint-ui.com/releases/1.3.0/docs/slint/src/recipes/custom_control)
- [Iced canvas example](https://docs.rs/iced/0.13.0/iced/canvas/index.html)
- [Tauri + Svelte example](https://tauri.app/v1/guides/getting-started/setup/svelte)
- [Umbrello-RS command architecture](./command_architecture_v1.md)
- [Umbrello-RS model repository](./model_repository_v1.md)
