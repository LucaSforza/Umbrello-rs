# Umbrello UI Architecture Analysis

> **Version:** 26.07.70 (development)
> **Last updated:** 2026-06-23
> **Purpose:** Guide the Rust rewrite by documenting every aspect of the existing C++ UI architecture,
> identifying KDE/Qt coupling depth, and providing concrete recommendations for Rust-native replacement.

---

## Table of Contents

1. [UI Component Hierarchy and Layout](#1-ui-component-hierarchy-and-layout)
2. [KDE Integration Analysis](#2-kde-integration-analysis)
3. [Qt Module Usage Analysis](#3-qt-module-usage-analysis)
4. [Dialog and Window Management Patterns](#4-dialog-and-window-management-patterns)
5. [Settings/Configuration Architecture](#5-settingsconfiguration-architecture)
6. [Menu/Toolbar/Action Architecture](#6-menutoolbaraction-architecture)
7. [Dock Widget Management](#7-dock-widget-management)
8. [Model-View Binding Patterns](#8-model-view-binding-patterns)
9. [Event Handling and Routing](#9-event-handling-and-routing)
10. [Rust Recommendations](#10-rust-recommendations)
11. [Proposed UI Architecture for Umbrello-RS](#11-proposed-ui-architecture-for-umbrello-rs)
12. [Migration Strategy for UI Layer](#12-migration-strategy-for-ui-layer)

---

## 1. UI Component Hierarchy and Layout

### 1.1 Physical Layout

```
 ┌──────────────────────────────────────────────────────┐
 │  Menu Bar (XMLGUI from umbrelloui.rc)                │
 │  File | Edit | View | Diagram | Code | Settings      │
 ├──────────────────────────────────────────────────────┤
 │  Main Toolbar (KToolBar) — standard file/undo actions │
 ├──────────────────────────────────────────────────────┤
 │ ┌──────────┬──────────────────────────┬──────────────┐ │
 │ │  Dock    │   Central Widget         │  Dock        │ │
 │ │  Area    │                         │  Area        │ │
 │ │ (Left)   │  TabWidget / StackedWidget             │ │
 │ │          │  ┌────────────────────┐  │              │ │
 │ │ TreeView │  │   KTabWidget       │  │ BirdView     │ │
 │ │ ListView │  │   ┌──────────────┐ │  │ (Minimap)    │ │
 │ │          │  │   │  UMLView     │ │  │              │ │
 │ │ Diagrams │  │   │ (QGraphicsView)│ │  │ Welcome     │ │
 │ │          │  │   │  ┌─────────┐  │ │  │              │ │
 │ │ Doc      │  │   │  │UMLScene│  │ │  │              │ │
 │ │ CmdHist  │  │   │  │(QGraphics│ │  │              │ │
 │ │ Log      │  │   │  │ Scene)  │ │  │              │ │
 │ │ Stereotypes │  │   │  └─────────┘  │  │              │ │
 │ │ Objects  │  │   └──────────────┘  │  │              │ │
 │ │ Debug    │  │                     │  │              │ │
 │ └──────────┴──────────────────────────┴──────────────┘ │
 ├──────────────────────────────────────────────────────┤
 │  Status Bar                                           │
 │  [Message Label]  [Zoom:-][Slider][Zoom:+][Fit][100%] │
 ├──────────────────────────────────────────────────────┤
 │  WorkToolBar (KToolBar) — diagram drawing tools        │
 │  [Arrow][Class][Interface][Association][Note]...      │
 └──────────────────────────────────────────────────────┘
```

### 1.2 Key Classes and Their Qt Bases

| Class | Inherits From | Role | File Count |
|---|---|---|---|
| `UMLApp` | `KXmlGuiWindow` | Singleton main window | 2 (h/cpp) |
| `UMLAppPrivate` | `QObject` | PIMPL holding dock widgets | 2 |
| `UMLView` | `QGraphicsView` | Viewport onto one diagram | 2 |
| `UMLScene` | `QGraphicsScene` | One diagram's content | 2 |
| `UMLWidget` | `QGraphicsObject` | Base class for all diagram widgets | 94 files |
| `AssociationWidget` | `QGraphicsObject` | Association lines between widgets | 1 |
| `MessageWidget` | `QGraphicsObject` | Sequence/collaboration messages | 1 |
| `WorkToolBar` | `KToolBar` | Context-sensitive drawing toolbar | 2 |
| `UMLListView` | `QTreeWidget` | Model tree view (dock) | 2 |
| `DocWindow` | `QTextEdit` | Documentation editor (dock) | 2 |
| `BirdView` | `QFrame` | Minimap (dock) | 2 |
| `ListPopupMenu` | `QMenu` | 260+ entry context menu factory | 2 |
| `SettingsDialog` | `MultiPageDialogBase` | 8-page tabbed settings | 2 |

### 1.3 Layout Management

- **Central widget**: A `QVBoxLayout` containing either a `QStackedWidget` (stacked mode) or `QTabWidget` (tabbed mode), controlled by `Settings::optionState().generalState.tabdiagrams`.
- **Dock widgets**: placed in `Qt::LeftDockWidgetArea` (tree view, diagrams, doc, cmd history, log, stereotypes, objects, debug) and `Qt::RightDockWidgetArea` (bird view, welcome).
- **WorkToolBar**: positioned at `Qt::TopToolBarArea`, after the main toolbar. This is NOT a dock — it is a `KToolBar` managed by KXMLGUI.
- **Status bar**: custom widgets added via `statusBar()->addWidget()` and `statusBar()->addPermanentWidget()`.

### 1.4 Diagram Representation

Two modes (configurable via settings):
1. **Stacked mode** (`m_viewStack`): single diagram visible at a time, switched programmatically.
2. **Tabbed mode** (`m_tabWidget`): each diagram gets a tab with close button, drag-to-reorder.

Both modes share the same underlying `UMLView`/`UMLScene` instances. The active diagram is tracked via `m_view`.

---

## 2. KDE Integration Analysis

### 2.1 Deeply Embedded (Hard to Replace — Requires Equivalent in Rust)

| Integration | Usage | Files Affected | Severity |
|---|---|---|---|
| `KXmlGuiWindow` | Main window base class | `umlapp.h/cpp` | Critical |
| `KActionCollection` | All menu/toolbar actions | `umlapp.cpp` (+300 actions) | Critical |
| `XMLGUI` (umbrelloui.rc) | Menu structure definition | `umbrelloui.rc.cmake` | Critical |
| `KStandardAction` | Standardized actions (file/open/save/quit/undo/redo...) | `umlapp.cpp` | High |
| `KRecentFilesAction` | Recent files menu | `umlapp.h/cpp` | High |
| `KToggleAction` | Toggle actions (snap-to-grid, show-grid) | `umlapp.cpp` | High |
| `KActionMenu` | Submenu actions (new diagram menu) | `umlapp.cpp` | High |
| `KSharedConfig` / `KConfig` | Application configuration | `umlapp.cpp`, many others | Critical |
| `KConfig XT` (kcfg) | Auto-generated settings from XML | `umbrello.kcfg`, auto-generated code | High |
| `KPageWidget` / `KPageDialog` | Tabbed settings dialog | `settingsdialog.*`, `multipagedialogbase.*` | High |
| `KLocalizedString` / `i18n` | All user-facing strings | 1000+ calls across codebase | Critical |

### 2.2 Moderate (Replaceable with Reasonable Effort)

| Integration | Usage | Files Affected | Notes |
|---|---|---|---|
| `KToolBar` | Main toolbar + WorkToolBar | `worktoolbar.*` | Simple toolbar API |
| `KTextEditor` | Embedded code viewer | `umlappprivate.*` | Optional; fallback to QTextEdit possible |
| `KColorButton` | Color picker in dialogs | `settingsdialog.*` | Tiny wrapper |
| `KFontChooser` | Font selection dialog | `multipagedialogbase.*` | Replacement exists |
| `KLineEdit` | Line edit with clear button | `settingsdialog.*` | Minor usability |
| `KComboBox` | Combo boxes | `settingsdialog.*` | Minor differences |
| `KMessageBox` | Standard dialogs (yes/no/info) | Many | `QMessageBox` equivalent |
| `KCursor` | Cursor helper | `umlapp.cpp` | Minor |
| `KActionCategory` | Action group for dock visibility | `umlappprivate.*` | Minor |
| `QUndoView` | Undo history dock | `umlappprivate.*` | Qt provides this directly! |
| `KXMLGUIFactory` | Find menus by XML name | `umlapp.cpp` | Only 2 call sites |

### 2.3 Light (Trivially Replaceable)

| Integration | Usage | Notes |
|---|---|---|
| `KConfigGroup` | Save/restore toolbar/dock state | Simple key-value |
| `KMainWindow::saveProperties` | Session management | Optional |
| `KMainWindow::applyMainWindowSettings` | Window state restore | Optional |
| `KIconLoader` / `Icon_Utils` | Icon theme loading | Custom icon loading possible |
| `KAboutData` / `KAboutApplicationDialog` | About dialog | Trivial replacement |
| `KCrash` | Crash handler | Optional |
| `KEMailSettings` | Email integration | Rarely used |

### 2.4 Key Observation

**KDE is not an add-on layer — it is deeply structural.** The entire action/menu architecture, configuration system, and main window lifecycle are built on KDE frameworks. A Rust rewrite cannot "peel off" KDE; it must replace the entire stack bottom-up. The most deeply embedded components are:

1. `KXmlGuiWindow` → main window base class
2. `KActionCollection` + XMLGUI → entire action/menu infrastructure  
3. `KConfig` + KConfig XT → settings persistence
4. `i18n` / `KLocalizedString` → all user-facing text

---

## 3. Qt Module Usage Analysis

### 3.1 Qt Modules and Their Roles

| Qt Module | Usage | Depth | Can Replace? |
|---|---|---|---|
| **QtCore** | QObject, signals/slots, QString, QUrl, QFile, QTimer, QPointF, etc. | Everywhere | The Rust framework replaces |
| **QtGui** | QColor, QFont, QPixmap, QPainter, QCursor, QKeyEvent | Everywhere | The Rust framework replaces |
| **QtWidgets** | QMainWindow, QDockWidget, QTabWidget, QStackedWidget, QTreeWidget, QGraphicsView, QGraphicsScene, QGraphicsItem, QMenu, QToolBar, QStatusBar, QLabel, QSlider, QListWidget, QDialog, QFileDialog, QMessageBox, QTextEdit, QPrinter, QPrintDialog, QPrintPreviewDialog | Entire UI | The Rust framework replaces |
| **QtPrintSupport** | QPrinter, QPrintDialog, QPrintPreviewDialog | Print/export diagrams | Major effort |
| **QtSvg** | SVG rendering for icons/diagrams | Icon loading, diagram export | resvg/usvg |
| **QtXml** | QDomDocument, QXmlStreamWriter | XMI persistence (also in model layer) | serde + quick-xml |
| **QtNetwork** | (Not used in UI directly) | - | - |

### 3.2 Critical Qt Patterns in Use

1. **QObject hierarchy**: Every significant class inherits QObject. The destructor tree, signal/slot connections, and `parent`-based ownership are fundamental.

2. **QGraphicsView framework**: The entire diagram system (UMLScene/UMLView/UMLWidget) is built on QGraphicsView. This is the **single most framework-coupled subsystem**.
   - `UMLScene` : `QGraphicsScene`
   - `UMLView` : `QGraphicsView`
   - `UMLWidget` : `QGraphicsObject` (94 widget types)
   - `AssociationWidget` : `QGraphicsObject`
   - `MessageWidget` : `QGraphicsObject`

3. **Signals and slots**: Everywhere. The old-style `SIGNAL()/SLOT()` macro syntax is used in the vast majority of cases (not the new functor-based syntax). Example:
   ```cpp
   connect(m_pUndoStack, SIGNAL(canRedoChanged(bool)), editRedo, SLOT(setEnabled(bool)));
   ```

4. **Qt's meta-type system**: `Q_ENUMS`, `Q_PROPERTY`, `Q_DECLARE_METATYPE` used for enum reflection and variant data.

5. **QPointer**: Used for safe nullable references (e.g., `QPointer<UMLView> m_view`).

6. **QDomDocument / QXmlStreamWriter**: XMI serialization interleaves with UI objects (save/load widget positions, colors, etc.).

### 3.3 QGraphicsView — The Largest Obstacle

The QGraphicsView framework provides:
- Scene/view separation with coordinate transforms
- Item-based rendering with z-ordering
- Mouse/event propagation to items
- Rubber-band selection
- Drag-and-drop between items
- Collision detection

**Every diagram widget and association line uses QGraphicsItem-derived classes.** There are 94 files in `umbrello/umlwidgets/`. Replacing QGraphicsView in Rust requires building an equivalent scene graph with:
- Hit testing
- Mouse event routing
- Transform (zoom)
- Selection tracking
- Z-ordering
- Grid snapping
- Alignment guides

---

## 4. Dialog and Window Management Patterns

### 4.1 Dialog Base Classes

#### `MultiPageDialogBase` (wraps `KPageWidget`)

Used for:
- `SettingsDialog` (8 pages: General, Font, UI, Class, Code Import, Code Gen, Code Viewer, Auto Layout)
- Property dialogs for: Class, Association, Message Widget, Note, etc.

Pattern:
```
MultiPageDialogBase (QWidget)
 └── KPageDialog (hidden unless m_useDialog=true)
     └── KPageWidget
         ├── Page 1: GeneralPage
         ├── Page 2: FontPage
         └── Page 3: StylePage...
```

Support for `setupFontPage()`, `setupStylePage()`, `setupGeneralPage()`, `setupAssociationRolePage()` as reusable components.

#### `SinglePageDialogBase` (wraps `QDialog`)

Used for single-property dialogs (attribute, operation, template, parameter, enum literal, etc.).

Pattern:
```
SinglePageDialogBase (QDialog)
 ├── QDialogButtonBox (OK/Cancel/Apply)
 └── m_mainWidget (user-provided content)
```

### 4.2 Common Dialogs

| Dialog | Base | Purpose | Page Count |
|---|---|---|---|
| `SettingsDialog` | `MultiPageDialogBase` | App preferences | 8 |
| `ClassPropertiesDialog` | `MultiPageDialogBase` | Class/widget properties | 3-6 |
| `AssociationPropertiesDialog` | `MultiPageDialogBase` | Association properties | 3 |
| `UMLAttributeDialog` | `SinglePageDialogBase` | Attribute editing | 1 |
| `UMLOperationDialog` | `SinglePageDialogBase` | Operation editing | 1 |
| `UMLTemplateDialog` | `SinglePageDialogBase` | Template parameter editing | 1 |
| `CodeViewerDialog` | Custom `QDialog` | Read-only code view | 1 |
| `ClassWizard` | `QWizard` | Step-by-step class creation | 2 |
| `CodeGenerationWizard` | Custom `QDialog` | Code generation configuration | Multi-step |
| `CodeImportingWizard` | Custom `QDialog` | Import configuration | Multi-step |
| `FindDialog` | `QDialog` | Search/find | 1 |
| `DiagramPrintPage` | `QWidget` | Print settings page | 1 |
| `ExportAllViewsDialog` | `QDialog` | Batch export configuration | 1 |

### 4.3 Dialog Pattern Summary

```
                  ┌──────────────────────────────────────┐
                  │  MultiPageDialogBase                  │
                  │  ┌─ KPageDialog ────────────────────┐ │
                  │  │ ┌─ KPageWidget ────────────────┐ │ │
                  │  │ │ [Tab1] [Tab2] [Tab3] ...     │ │ │
                  │  │ │                               │ │ │
                  │  │ │  (User pages via createPage)  │ │ │
                  │  │ └───────────────────────────────┘ │ │
                  │  │ [OK] [Apply] [Cancel] [Default]   │ │
                  │  └────────────────────────────────────┘ │
                  └──────────────────────────────────────┘

                  ┌──────────────────────────────────────┐
                  │  SinglePageDialogBase                  │
                  │  ┌─ m_mainWidget ────────────────────┐ │
                  │  │  (User-provided content widget)    │ │
                  │  └────────────────────────────────────┘ │
                  │  [OK] [Apply] [Cancel]                  │
                  └──────────────────────────────────────┘
```

---

## 5. Settings/Configuration Architecture

### 5.1 Three-Layer Architecture

```
Layer 1: UmbrelloSettings (auto-generated from umbrello.kcfg)
┌────────────────────────────────────────────┐
│ KConfig XT code generation                 │
│ umbrello.kcfg → UmbrelloSettings class     │
│ Methods: setGeometry(), geometry(),         │
│          setImageMimeType(), etc.           │
│ Storage: KSharedConfig ("umbrellorc")       │
└────────────────────────────────────────────┘

Layer 2: Settings::OptionState (in-memory runtime state)
┌────────────────────────────────────────────┐
│ Singleton: Settings::OptionState::instance()│
│ Categories:                                 │
│   - GeneralState (undo, tabdiagrams, etc.) │
│   - UIState (colors, fonts, line width)    │
│   - ClassState (visibility defaults)       │
│   - CodeGenerationState (per-language opts)│
│   - CodeImportState                        │
│   - AutoLayoutState                        │
│   - LayoutTypeState                        │
│ Methods: load(), save() read/write KConfig │
└────────────────────────────────────────────┘

Layer 3: KSharedConfig + KConfigGroup (manual)
┌────────────────────────────────────────────┐
│ Direct KConfigGroup access for:             │
│   - Toolbar state                           │
│   - Recent files list                       │
│   - Window geometry + dock positions        │
│   - Keyboard shortcuts                      │
└────────────────────────────────────────────┘
```

### 5.2 Per-Diagram Settings

Each `UMLScene` has its **own** `Settings::OptionState m_Options` which overrides the global defaults. Settings flow:

```
UmbrelloSettings (disk)
       ↓
Settings::OptionState (global)
       ↓
UMLScene::m_Options (per-diagram override)
```

### 5.3 Settings Dialog Pages

| Page | Backing State | KDE Widgets Used |
|---|---|---|
| General | `GeneralState` | Checkboxes, spinboxes |
| Font | `UIState.font` | `KFontChooser` |
| UI (User Interface) | `UIState` | `KColorButton`, color checkboxes |
| Class | `ClassState` | Checkboxes, `KComboBox` |
| Code Import | `CodeImportState` | Checkboxes |
| Code Generation | `CodeGenerationState` | `KComboBox`, `KLineEdit` |
| Code Viewer | `CodeViewerState` | `KColorButton`, `KFontChooser` |
| Auto Layout | `AutoLayoutState` | Checkboxes, `KLineEdit` |

### 5.4 Config File Location

`~/.config/umbrellorc` (handled by KConfig). Contains INI-style sections matching the kcfg groups.

---

## 6. Menu/Toolbar/Action Architecture

### 6.1 The Action Universe

```
KActionCollection (on UMLApp)
 ├── KStandardAction: openNew, open, save, saveAs, close, print, 
 │                    printPreview, quit, undo, redo, cut, copy, paste,
 │                    selectAll, preferences, find, findNext, findPrev
 ├── KRecentFilesAction: fileOpenRecent (~10 items)
 ├── KToggleAction: viewSnapToGrid, viewShowGrid
 ├── KActionMenu: new_view (submenu with 8+ diagram types)
 ├── Custom actions (50+): delete_selected, align_*, zoom*, 
 │                          file_export_docbook, class_wizard, etc.
 ├── Language actions (20): setLang_actionscript .. setLang_none
 └── DOCK ACTIONS (via KActionCategory): view_show_tree, view_show_doc,
                                         view_show_undo, view_show_bird, etc.
```

**Total: ~100 actions**

### 6.2 XMLGUI Menu Structure (umbrelloui.rc)

```
MenuBar
 ├── File
 │   ├── Export model
 │   │   ├── Export model to DocBook
 │   │   └── Export model to XHTML
 │   └── Export Diagrams as Pictures
 ├── Edit
 │   └── Delete Selected
 ├── View
 │   └── Show/hide window
 │       ├── Tree View
 │       ├── Documentation
 │       ├── Command history
 │       ├── Bird's eye view
 │       ├── Stereotypes
 │       ├── Diagrams
 │       ├── UML Objects
 │       └── Welcome
 ├── Diagram
 │   ├── New (submenu: Class, Object, Sequence, ...)
 │   ├── Clear Diagram
 │   ├── Delete Diagram
 │   ├── Export as Picture
 │   ├── Show (dynamic diagram list)
 │   ├── Zoom (dynamic zoom menu)
 │   ├── Align (8 alignment actions)
 │   ├── Snap to Grid
 │   ├── Show Grid
 │   └── Properties
 ├── Code
 │   ├── Import Class
 │   ├── Import from Directory
 │   ├── Code Importing Wizard
 │   ├── Code Generation Wizard
 │   ├── Generate All Code
 │   ├── Active Language (submenu, 20 languages)
 │   ├── Add Default Datatypes
 │   └── New Class Wizard
 └── Settings
     └── [preferences action]
```

### 6.3 WorkToolBar (Per-Diagram Context Toolbar)

The `WorkToolBar` (a `KToolBar` placed at the top) changes its buttons based on diagram type:

- **Class/UseCase/Component/Deployment diagrams**: Arrow, Class, Interface, Enum, Actor, UseCase, Package, Component, Node, Artifact, Association types (Generalization, Aggregation, Composition, Dependency, etc.), Note, Box, Text, Anchor
- **Sequence diagrams**: Arrow, Message types (Synchronous, Asynchronous, Creation, Destroy, Found, Lost), Combined Fragment, Precondition, Object, Note
- **State/Activity diagrams**: Arrow, State/Activity types, Transitions, Fork/Join, Initial/Final, etc.
- **Entity Relationship diagrams**: Arrow, Entity, Relationship types

**Total: ~75 tool buttons** enumerated in `WorkToolBar::ToolBar_Buttons` (tbb_*).

### 6.4 Context Menus (ListPopupMenu)

`ListPopupMenu` is a `QMenu` with **260+ MenuType entries** (`mt_*`). It serves as a factory that constructs context menus based on the type of item clicked:

- **Model items** (in `UMLListView` tree): create/rename/delete/diagrams for each UML object type
- **Diagram items** (in `UMLScene`): widget properties, visual toggles, alignment, layers, cut/copy/paste, delete
- **Associations**: line style, role rename, properties
- **Diagram background**: new widget placement, paste, select all, diagram properties

Each `MenuType` maps to an enum, and `ListPopupMenu::insert()` methods build the QMenu dynamically. The `getMenuType()` method identifies which action was triggered.

### 6.5 Keyboard Shortcuts

Shortcuts are defined via:
```cpp
actionCollection()->setDefaultShortcut(deleteSelectedWidget, QKeySequence(Qt::Key_Delete));
```

And persisted in `KConfigGroup("Shortcuts")`.

---

## 7. Dock Widget Management

### 7.1 Dock Widget Inventory

All dock widgets are created and managed in `UMLAppPrivate`:

| Dock Widget | Variable | Widget Class | Object Name | Area | Toggle Action |
|---|---|---|---|---|---|
| Tree View | `listDock` | `UMLListView` (QTreeWidget) | `TreeViewDock` | Left | `view_show_tree` |
| Documentation | `documentationDock` | `DocWindow` (QTextEdit) | `DocumentationDock` | Left | `view_show_doc` |
| Command History | `cmdHistoryDock` | `QUndoView` | `CmdHistoryDock` | Left | `view_show_undo` |
| Log | `logDock` | `QListWidget` | `LogDock` | Left | `view_show_log` |
| Stereotypes | (see `StereotypesWindow`) | `QTableView` | — | Left | `view_show_stereotypes` |
| Diagrams | (see `DiagramsWindow`) | `QTableView` | — | Left | `view_show_diagrams` |
| UML Objects | (see `ObjectsWindow`) | `QTableView` | — | Left | `view_show_objects` |
| Debug | `debugDock` | `Tracer` (QTreeWidget) | `DebugDock` | Left | `view_show_debug` |
| Bird's Eye View | `birdViewDock` | `BirdView` (QFrame) | `BirdViewDock` | Right | `view_show_bird` |
| Welcome | `welcomeWindow` | `QWebView`/`QTextBrowser` | `WelcomeDock` | Right | `view_show_welcome` |

### 7.2 Dock Tabification

Docks are tabified (grouped into tabbed stacks) in `UMLAppPrivate::initActions()`:
```
Documentation | Command History | Log     (tab group 1)
Tree View     | Stereotypes     | Diagrams | (UML Objects) (tab group 2)
Welcome       | Bird's Eye View                        (tab group 3)
Debug         | Log (tabified together when shown)
```

### 7.3 Dock Visibility

Visibility is controlled by:
1. Toggle actions via `dockCategory->addAction(name, dockWidget->toggleViewAction())`
2. Settings persistence via `KConfigGroup` saving dock positions/visibility
3. Debug windows shown only when `optionState.generalState.showDebugWindows` is true

---

## 8. Model-View Binding Patterns

### 8.1 Primary Data Flow

```
UMLDoc (Document Model)
  ├── Owns root UMLFolder hierarchy
  ├── Each UMLFolder contains UMLObjects
  │     └── UMLObjects have properties (name, visibility, etc.)
  ├── Each UMLFolder owns UMLViews (diagrams)
  │     └── Each UMLView owns a UMLScene
  │           └── UMLScene renders UMLObjects via UMLWidgets

Signals:
  UMLDoc::sigObjectCreated → UMLListView::slotObjectCreated
  UMLDoc::sigObjectCreated → UMLScene::slotObjectCreated
  UMLDoc::sigObjectRemoved → UMLListView::slotObjectRemoved
  UMLDoc::sigObjectRemoved → UMLScene::slotObjectRemoved
  UMLDoc::sigWriteToStatusBar → UMLApp::slotStatusMsg
```

### 8.2 Binding Patterns by Component

| UI Component | Binds To | Update Mechanism |
|---|---|---|
| `UMLListView` (tree) | `UMLDoc` model tree | Signal/slot: `sigObjectCreated`, `sigObjectRemoved`, `sigDiagramCreated` |
| `UMLScene` (diagram canvas) | `UMLObjects` via `UMLWidget` | Signal/slot + direct method calls |
| `DocWindow` (documentation) | Currently selected `UMLObject` | Explicit `setCurrentObject()` calls |
| `DiagramsWindow` (table) | `UMLDoc` diagram list | Custom model, signals on changes |
| `StereotypesWindow` (table) | Stereotype list in `UMLDoc` | Custom model |
| Code viewer | `CodeDocument` from code generators | Direct document access |
| Status bar | `UMLDoc` + `UMLView` | Signals for messages, direct for zoom |

### 8.3 Key Observation

**There is no formal MVC/MVP framework.** The binding is done through:
- Direct method calls (tight coupling, e.g., `UMLApp::currentView()` → `UMLView`)
- Qt signal/slot connections (looser, but still class-specific)
- Singleton access (`UMLApp::app()` used everywhere)

This tight coupling means the UI layer cannot be replaced independently of the model layer.

---

## 9. Event Handling and Routing

### 9.1 Event Flow Diagram

```
User Input
    │
    ▼
┌────────────────┐
│  UMLApp        │  keyPressEvent, keyReleaseEvent, customEvent
│  (Main Window) │  → delegates to current UMLView
└────────────────┘
    │
    ▼
┌────────────────┐
│  UMLView       │  wheelEvent, mousePressEvent, mouseReleaseEvent,
│  (QGraphicsView)│  showEvent, hideEvent, resizeEvent
│                │  → forwards to UMLScene
└────────────────┘
    │
    ▼
┌────────────────────────────────────────────────┐
│  UMLScene                                      │
│  (QGraphicsScene)                               │
│  mousePressEvent → ToolBarState::mousePress()  │
│  mouseMoveEvent  → ToolBarState::mouseMove()   │
│  mouseReleaseEvent → ToolBarState::mouseRelease│
│  contextMenuEvent → ListPopupMenu (QMenu)      │
│  drag/drop events → UMLScene handling          │
└────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────┐
│  ToolBarState (State Machine)                 │
│  ├── ToolBarStateArrow (default/selection)    │
│  ├── ToolBarStateAssociation (draw assocs)    │
│  ├── ToolBarStateMessages (sequence msgs)     │
│  ├── ToolBarStatePool (lifeline creation)     │
│  └── ToolBarStateOther (widget creation)      │
│                                               │
│  Each state handles: mousePress[Association  │
│  |Widget|Empty], mouseRelease*, mouseMove*,  │
│  mouseDoubleClick*                            │
└──────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────┐
│  UMLWidget / AssociationWidget / MessageWidget│
│  (QGraphicsItems)                              │
│  → paint(), mouse events via scene,            │
│    context menus via scene                     │
└──────────────────────────────────────────────┘
```

### 9.2 Toolbar State Machine Detail

The `ToolBarState` hierarchy implements a **state pattern**:

| State Class | Purpose | Subclass of |
|---|---|---|
| `ToolBarState` | Base: init, clean, press/release/move/doubleClick | — |
| `ToolBarStateArrow` | Selection, move, resize widgets | `ToolBarState` |
| `ToolBarStateAssociation` | Draw association lines | `ToolBarState` |
| `ToolBarStateMessages` | Sequence message arrows | `ToolBarState` |
| `ToolBarStatePool` | Lifeline/destruction box creation | `ToolBarState` |
| `ToolBarStateOther` | Create single-click widgets | `ToolBarState` |

Each state subdivides events:
- `mousePressAssociation()` — click on existing association
- `mousePressWidget()` — click on existing widget
- `mousePressEmpty()` — click on empty canvas space

This routing enables different behaviors per current tool.

### 9.3 Keyboard Event Handling

```
UMLApp::keyPressEvent → currentView() → currentView()->umlScene()
  → ToolBarState::mousePress (if applicable)
  → UMLScene falls through to default QGraphicsScene handling
  → UMLApp also handles cursor keys (handleCursorKeyReleaseEvent)
```

### 9.4 Context Menu Triggers

Three separate context menu entry points:
1. **`UMLScene::contextMenuEvent`** → Diagram/Widget/Association right-click
2. **`UMLListView::contextMenuEvent`** → Tree view right-click
3. **`UMLApp::slotDiagramPopupMenu`** → Tab bar right-click

All delegate to `ListPopupMenu` or specific popup menu classes in `umbrello/menus/`:
- `ListPopupMenu` — general purpose (260+ MenuTypes)
- `UMLScenePopupMenu` — scene-specific actions
- `UMLListViewPopupMenu` — list view items
- `WidgetBasePopupMenu` — widget base actions
- `AssociationWidgetPopupMenu` — association-specific

---

## 10. Rust Recommendations

### 10.1 GUI Framework Options

| Framework | Paradigm | Widget Set | Native Feel | Canvas | Ecosystem | Recommendation |
|---|---|---|---|---|---|---|
| **Slint** | Declarative (own language) | Built-in (~40 widgets) | Excellent (native rendering) | Custom (Rust) | Growing | ★★★★ **Primary candidate** |
| **Egui** | Immediate mode | Built-in (good) | Less native/looks unique | Built-in canvas | Large | ★★★ Good for prototyping |
| **Iced** | Elm architecture | Expanding (~20 widgets) | Decent | WGPU via widgets | Moderate | ★★ Still maturing |
| **Tauri + web** | VDOM (React/Vue/...) | Full web UI | Themed possible | Canvas/SVG | Massive | ★★ Heavy stack |
| **FLTK-rs** | Retained/simple | Full native | Ugly but native | Only via custom | Small | ★ Too old-school |
| **Relm4/GTK4-rs** | GTK4 idiom | Full mature | Native GTK | GTK DrawingArea | Mature | ★★★ GTK dependency |

### 10.2 Recommendation: Slint (Primary) with Egui (Diagram Canvas)

**Rationale:**
- Slint provides a declarative markup language that maps well to the XMLGUI approach
- Native rendering on all platforms (Qt-independent)
- Growing widget set includes: Button, CheckBox, ComboBox, ListView, TabWidget, Slider, SpinBox, TextEdit, LineEdit, TreeView (Basic), TableView, Dialog, etc.
- Supports custom canvas elements for the diagram area
- Built-in signal/slot mechanism similar to Qt's
- Active development and increasing Rust ecosystem integration

**Fallback: Egui** — if Slint proves insufficient for complex diagram editing, use Egui for the diagram canvas while keeping Slint for standard UI (menus, dialogs, dock widgets).

### 10.3 Component-by-Component Recommendations

| UI Component | C++ Implementation | Rust Recommendation | Notes |
|---|---|---|---|
| Main Window | `KXmlGuiWindow` | Slint `Window + NavigationLayout` | Custom implementation needed |
| Menu Bar | XMLGUI + `KActionCollection` | Slint `MenuBar` + custom action registry | Hand-code menus (no XMLGUI equivalent) |
| Toolbars | `KToolBar` | Slint `HorizontalBox` with buttons | Map button clicks to commands |
| Dock Widgets | `QDockWidget` (Qt built-in) | Custom dock system or `egui_dock` | Slint lacks native dock support |
| Diagram Canvas | `QGraphicsView`/`QGraphicsScene` | **Custom `wgpu`/`vello` canvas** | The hard part — see §10.4 |
| Tree View | `QTreeWidget` (= `UMLListView`) | Slint `TreeView` (Basic) or custom | Will need custom item rendering |
| Tab Widget | `QTabWidget` | Slint `TabWidget` | Good mapping |
| Stack Widget | `QStackedWidget` | Slint `Navigator` | Good mapping |
| Status Bar | Custom widgets in `QStatusBar` | Slint horizontal layout | Simple |
| Dialogs | `QDialog`, `KPageDialog` | Slint `Dialog` | Will need custom page system |
| Text Edit | `QTextEdit`, `KTextEditor` | Slint `TextEdit` (basic) or embedding | Basic editing OK |
| List/Table Widgets | `QListWidget`, `QTableView` | Slint `ListView`, `TableView` | Standard mappings |
| Slider | `QSlider` | Slint `Slider` | Direct mapping |
| Undo View | `QUndoView` | Custom (list of undo descriptions) | Simple |
| Code Editor | `KTextEditor` | `syntect` for highlighting + plain TextEdit | No embedded full editor needed |
| Print Preview | `QPrintPreviewDialog` | `printpdf` crate + custom dialog | Simpler path |
| SVG rendering | `QtSvg` | `resvg` + `usvg` | Excellent Rust replacements |

### 10.4 Diagram Canvas Strategy

The diagram canvas is the **most complex UI component**. The QGraphicsView framework provides:
- Scene graph with item hierarchy
- View transforms (zoom, pan)
- Event dispatching to individual items
- Collision detection
- Rubber-band selection
- Drag-and-drop

**Recommended approach for Rust:**

```
Layer 1: wgpu (GPU backend)
Layer 2: vello (2D rendering/compositing) or piet-gpu
Layer 3: Custom scene graph
  ├── SceneNode trait (position, bounds, hit-test, paint)
  │   ├── WidgetNode (for UMLWidget types)
  │   ├── AssociationNode (for connection lines)
  │   ├── TextNode (for labels)
  │   └── GridNode (for background grid)
  ├── SceneManager (scene graph management)
  ├── EventRouter (mouse/keyboard → items)
  ├── SelectionManager (track selected items)
  └── TransformController (zoom/pan)
```

**Alternative: Use Egui for the canvas layer.** Egui provides superior immediate-mode drawing with built-in:
- Mouse event handling
- Zoom/pan transforms
- Grid layout
- Text rendering

### 10.5 Dock Widget System

**Option A: Custom dock widget implementation (recommended)**
- Implement a `DockArea` widget in Slint or Egui
- Support for: tabification, drag-to-reorder, resize splitters, hide/show
- Model the behavior on `QDockWidget` but lighter
- Reference: `egui_dock` crate (Egui-based)

**Option B: Use Egui + egui_dock**
- Already provides tabification, resizable panels
- Limited customization but functional

### 10.6 Supporting Libraries

| Need | Rust Crate | Notes |
|---|---|---|
| Configuration | `serde` + `toml` or `json` | TOML for human readability |
| i18n/L10n | `fluent-rs` (Project Fluent) | Modern, ergonomic, supports plural/context |
| Undo/Redo | Custom `UndoStack` wrapper | Simple trait + VecStack |
| Keyboard shortcuts | Custom registry | Key → Action mapping |
| SVG Icons | `resvg` + `usvg` | Load, render, theme SVGs |
| Image export | `image` crate + custom rendering | PNG, JPEG, SVG export |
| Print | `printpdf` | PDF generation |
| Syntax highlighting | `syntect` | For code viewer/highlighting |
| Clipboard | `arboard` | Cross-platform clipboard |
| File dialogs | `rfd` (Rusty File Dialogs) | Native file picker |
| Tree model | Custom (no ready crate) | Will need recursive data structure |
| Auto-layout | `graphviz` bindings or custom | Dot layout engine |
| Text editing | Slint `TextEdit` or custom | Basic editing for documentation |

### 10.7 Code Editor Strategy

The existing `KTextEditor` integration is **light** (used for viewing only, not full editing). Rust options:
- **`syntect`** for syntax highlighting + Slint `TextEdit` for editing — sufficient for code viewing
- Embed `xi-editor` or its successor if full code editing is needed (lower priority)

---

## 11. Proposed UI Architecture for Umbrello-RS

### 11.1 High-Level Architecture

```
umbrello-ui crate
 ├── main_window/           (Slint + custom dock system)
 │   ├── MainWindow         — app singleton, owns all sub-components
 │   ├── ActionRegistry     — central action manager (menus, toolbars, shortcuts)
 │   ├── DockContainer      — manages all dock widgets
 │   ├── MenuBar            — Slint MenuBar bound to ActionRegistry
 │   ├── MainToolBar        — standard actions toolbar
 │   └── StatusBar          — message label + zoom controls
 │
 ├── docks/                 (each is a DockContainer child)
 │   ├── TreeDock           — model tree view (replaces UMLListView)
 │   ├── DocumentationDock  — documentation text editor
 │   ├── CommandHistoryDock — undo/redo list
 │   ├── LogDock            — log messages list
 │   ├── BirdViewDock       — diagram minimap
 │   ├── DiagramsDock       — diagram list table
 │   ├── StereotypesDock    — stereotype table
 │   └── DebugDock          — tracer/debug output (optional)
 │
 ├── diagram/               (the critical component)
 │   ├── DiagramCanvas      — wgpu/vello or Egui canvas area
 │   ├── SceneManager       — owns the scene graph
 │   ├── SceneNode          — trait for all diagram items
 │   ├── WidgetNode         — UMLWidget behavioral types
 │   ├── AssociationNode    — association lines
 │   ├── MessageNode        — sequence message arrows
 │   ├── InteractionState   — state machine (replaces ToolBarState)
 │   │   ├── SelectState          (was ToolBarStateArrow)
 │   │   ├── AssociationDrawState (was ToolBarStateAssociation)
 │   │   ├── MessageDrawState     (was ToolBarStateMessages)
 │   │   └── CreateWidgetState    (was ToolBarStateOther)
 │   ├── SelectionManager   — tracks selected items
 │   ├── GridManager        — background grid, snap
 │   ├── AlignmentGuide     — alignment snapping
 │   └── ZoomController     — zoom/pan transforms
 │
 ├── dialogs/               (Slint Dialog + custom page system)
 │   ├── SettingsDialog     — 8-page settings
 │   ├── ClassWizard        — step-by-step class creation
 │   ├── PropertyDialogs    — per-type property editors
 │   ├── FindDialog         — search tool
 │   ├── CodeViewer         — syntax-highlighted code display
 │   ├── ExportDialog       — image/PDF export configuration
 │   └── AboutDialog        — version info
 │
 ├── popup_menus/           (context menu factories)
 │   ├── ContextMenu        — central menu builder
 │   └── MenuRegistry       — maps element types to menu actions
 │
 ├── toolbar/               (diagram drawing toolbar)
 │   └── WorkToolBar        — per-diagram-type tool palette
 │
 ├── settings/              (configuration)
 │   ├── SettingsStore      — serde + TOML persistence
 │   ├── OptionState        — in-memory runtime state (modeled on C++)
 │   └── CodeViewerConfig   — syntax highlighting config
 │
 └── action/                (centralized action system)
     ├── Action             — struct: id, label, shortcut, icon, callback
     ├── ActionRegistry     — singleton registry of all actions
     └── ShortcutManager    — key binding management
```

### 11.2 Data Flow

```
                     ┌──────────────────────┐
                     │    umbrello-model     │
                     │  (UMLObject, UMLDoc,  │
                     │   UMLFolder, etc.)    │
                     └──────────┬───────────┘
                                │ Observer pattern (EventBus/Channel)
                                ▼
┌─────────────────────────────────────────────────────┐
│                  umbrello-ui                         │
│                                                      │
│  ActionRegistry ←── Slint UI (menus, toolbars)      │
│       │                                              │
│       │ command dispatch                             │
│       ▼                                              │
│  MainWindow ──→ DiagramCanvas ──→ SceneManager       │
│       │                            │        │        │
│       │                            ▼        ▼        │
│       │                     WidgetNodes  Interaction │
│       │                                  State      │
│       │                                              │
│       └──→ DockContainer                             │
│                ├── TreeDock ←── model events         │
│                ├── CommandHistoryDock ←── undo stack │
│                └── DocumentationDock ←── selection   │
│                                                      │
│  SettingsStore ←── SettingsDialog                    │
└─────────────────────────────────────────────────────┘
```

### 11.3 Action System Design (Replaces KActionCollection)

```rust
struct Action {
    id: &'static str,           // e.g. "file_save"
    label: String,              // i18n'd user label
    tooltip: String,
    shortcut: Option<KeyBinding>,
    icon: Option<Icon>,          // SVG icon
    enabled: bool,
    callback: Box<dyn Fn()>,
}

struct ActionRegistry {
    actions: HashMap<String, Action>,
}

impl ActionRegistry {
    fn register(action: Action);
    fn trigger(id: &str);
    fn set_enabled(id: &str, enabled: bool);
    fn action(id: &str) -> Option<&Action>;
}
```

### 11.4 Interaction State Machine (Replaces ToolBarState)

```rust
enum InteractionState {
    Select(SelectState),
    CreateAssociation(AssociationState),
    CreateMessage(MessageState),
    CreateWidget(WidgetCreationState),
    DrawSignal(SignalState),
}

trait InteractionStateHandler {
    fn on_press(&mut self, event: PointerEvent, scene: &SceneManager);
    fn on_move(&mut self, event: PointerEvent, scene: &SceneManager);
    fn on_release(&mut self, event: PointerEvent, scene: &SceneManager);
    fn on_double_click(&mut self, event: PointerEvent, scene: &SceneManager);
    fn cursor(&self) -> Cursor;
}
```

---

## 12. Migration Strategy for UI Layer

### 12.1 Order of Implementation

The UI layer should be implemented **last**, after the model, persistence, and code generation layers are stable. This follows the principle of building from the "inside out":

```
Phase 1: Model + Persistence (stable data layer)
Phase 2: Code Generation + Import (business logic, testable without UI)
Phase 3: Core Application Shell (main window, menus, actions)
Phase 4: Diagram Canvas (the hardest component)
Phase 5: Dialogs + Dock Widgets
Phase 6: Integration + Polish
```

### 12.2 Phase 3: Application Shell

**Goal**: Minimal functional window that can open/save files, display a tree, and show a placeholder canvas.

Deliverables:
- `MainWindow` skeleton (title bar, menu bar, status bar)
- `ActionRegistry` with all actions defined
- File menu working (New, Open, Save, Save As, Quit)
- `SettingsStore` with TOML persistence
- Tree dock showing loaded model
- Empty diagram canvas placeholder

**Not yet implemented**:
- Diagram rendering (just a blank rectangle)
- Most dock widgets (placeholder or hidden)
- Context menus
- Keyboard shortcuts

### 12.3 Phase 4: Diagram Canvas

**Goal**: Functional diagram editor with the major widget types.

Deliverables:
- `DiagramCanvas` with `wgpu`/`vello` or Egui rendering
- `SceneManager` with basic scene graph
- `InteractionState` machine with `SelectState`
- 5-10 most common widget types: Class, Interface, Association, Note, Package
- Grid rendering + snap-to-grid
- Zoom/pan controls
- Selection (click, rubber-band, multi-select)
- Drag to move widgets
- Save/load diagram positions

**Not yet implemented**:
- All 60+ widget types
- Sequence/state/activity message arrows
- Alignment guides
- Auto-layout

### 12.4 Phase 5: Dialogs + Dock Widgets

**Goal**: Complete the UI with all dialogs and dock windows.

Deliverables:
- `SettingsDialog` with 8 pages
- Property dialogs for all widget types
- All dock widgets functional
- `WorkToolBar` with per-diagram-type tool selection
- Context menus for all element types
- Undo/redo stack with dock
- Clipboard (cut/copy/paste)

### 12.5 Phase 6: Integration + Polish

**Goal**: Feature parity with the C++ version.

Deliverables:
- All 60+ UMLWidget types rendered
- Sequence/state/activity diagrams fully working
- Code generation invoked from UI
- Code import invoked from UI
- Print/export (image, PDF, SVG)
- Keyboard shortcuts configurable
- Full i18n via fluent-rs
- Theme support (SVG icons)
- Dock layout persistence
- Session management

### 12.6 Testing Strategy

| Component | Testing Approach |
|---|---|
| Model binding | Unit tests with mock model |
| Action registry | Unit tests for action dispatch |
| Menu/toolbar | Integration test (click → action fires) |
| Dialog logic | Unit test (dialog state → model update) |
| Diagram canvas | Snapshot/render tests, hit-testing tests |
| Widget creation | Property-based tests for positioning |
| State machine | Sequence-based tests (click→drag→release) |

### 12.7 Risk Mitigation

| Risk | Mitigation |
|---|---|
| Slint widget set insufficient | Fall back to Egui for advanced components; keep architecture framework-agnostic via trait abstractions |
| Canvas rendering performance | Use `wgpu`/`vello` from start; benchmark against C++ QGraphicsView |
| Dock widget complexity | Start with simple non-tabified docks; add tabification as opt-in feature |
| Missing QGraphicsView features | Audit all QGraphicsView APIs used; prioritize in custom implementation |
| Print/export quality drop | Keep PDF/print for last phase; compare output against C++ version |
| i18n coverage loss | Use `fluent-rs` from start with `.ftl` files extracted alongside development |

### 12.8 Key Architectural Decisions

1. **Framework abstraction via traits**: The UI should not depend on Slint concrete types for business logic. Use traits like `CanvasRenderer`, `DialogHost`, `DockHost` to abstract the framework.

2. **Event bus for model→UI updates**: Instead of direct signal/slot coupling, use a channel-based event bus (`tokio::sync::broadcast` or custom). The model publishes events; UI components subscribe.

3. **Command pattern for undo/redo**: Every user action should produce a `Command` object (as in the C++ `QUndoCommand` pattern). This makes undo, redo, and macro recording straightforward.

4. **Off-thread model loading**: XMI parsing and code generation should happen on background threads, with the UI showing progress via the event bus.

5. **No plugins, no dynamic loading**: Like the C++ version, the Rust rewrite should compile all code generators and importers into the binary.

---

## Appendix A: File Count by UI Subsystem

| Subsystem | Directory | Files |
|---|---|---|
| Main window | `umbrello/umlapp.*` + `umlappprivate.*` | 4 |
| Workspace | `umbrello/umlview.*` + `umlscene.*` | 4 |
| Diagram widgets | `umbrello/umlwidgets/` | 94 |
| Toolbar state machine | `umbrello/toolbarstate*.h/.cpp` | 12 |
| Dialogs | `umbrello/dialogs/` | ~150 (82 .h + matching .cpp) |
| Menu system | `umbrello/menus/` | 12 |
| Dock widgets | `umbrello/*window.*`, `birdview.*`, etc. | 16 |
| Settings | `umbrello/optionstate.*`, `umbrello.kcfg` | 3 |
| Work toolbar | `umbrello/worktoolbar.*` | 2 |
| Model tree | `umbrello/umllistview.*` + `umllistviewitem.*` | 4 |
| Debug/Log | `umbrello/debug/` | 2 |
| Icon utils | `umbrello/icon_utils.*` | 2 |
| Finder | `umbrello/finder/` | ~4 |
| **Total UI** | | **~300 files** |

## Appendix B: Key Metrics

| Metric | Value |
|---|---|
| Total action count | ~100 |
| Context menu entry types | 260+ (`ListPopupMenu::MenuType`) |
| Toolbar button types | 75+ (`WorkToolBar::ToolBar_Buttons`) |
| Diagram widget types | 60+ (in `umlwidgets/`) |
| Settings pages | 8 |
| Dock widgets | 10 |
| Dialog classes | 30+ |
| Languages in code menu | 20 |
| Diagram types | 9 (Class, Sequence, Collaboration, Use Case, State, Activity, Component, Deployment, Entity Relationship) |
