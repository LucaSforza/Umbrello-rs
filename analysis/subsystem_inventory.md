# Umbrello Subsystem Inventory

> Version: 26.07.70 (development, patch ≥ 70 enables unstable features)
> Build: C++17, Qt5/KF5 or Qt6/KF6, output `umbrello5`/`umbrello6`
> Last updated: 2026-06-23

---

## 1. Build System & Project Topology

### Purpose
Configure, build, and link the Umbrello binary and its libraries across two Qt/KF generations.

### Responsibilities
- Detect Qt5/KF5 vs Qt6/KF6 and select compatible flags, sources, and dependencies.
- Build the thin `umbrello5`/`umbrello6` entry-point executable linked against `libumbrello`.
- Build the secondary static library `codeimport` for code-import functionality.
- Guard unstable feature flags behind `UMBRELLO_VERSION_PATCH >= 70`.
- Conditionally compile optional dependencies (KDevPlatform for PHP import, LLVM for tests).

### Key Files
| File | Role |
|---|---|
| `CMakeLists.txt` (root) | Project definition, version, Qt/KF detection, executable targets |
| `umbrello/CMakeLists.txt` | `libumbrello` static library target and all its sources |
| `lib/cppparser/CMakeLists.txt` | C++ parser static lib target |
| `lib/interfaces/CMakeLists.txt` | Shared interface stubs |

### Dependencies
- **Required Qt**: Core, Gui, Widgets, Xml, PrintSupport, Svg, Test
- **Required KF**: Archive, Completion, Config, CoreAddons, Crash, I18n, IconThemes, KIO, TextEditor, WidgetsAddons, WindowSystem, XmlGui
- **System**: LibXml2, LibXslt
- **Optional**: KDevPlatform, LLVM

### Unstable Feature Flags
| Flag | Enables |
|---|---|
| `WIDGET_SHOW_DOC` | Documentation display on widgets |
| `NEW_CODE_GENERATORS` | Experimental alternative code generators |
| `UML_OBJECTS_WINDOW` | Dedicated UML objects dock |
| `XMIRESOLUTION` | XMI forward-reference resolution improvements |
| `COMBINED_STATE_DIRECT_EDIT` | Inline editing of combined states |
| `OBJECT_DIAGRAM` | Object diagram support |

### Key Observations
- Dual-generational Qt/KF support adds significant `#ifdef` complexity.
- Unstable features are always on in the development branch (patch >= 70) — effectively always enabled.
- No built-in plugin system; all code generation is compiled into the monolithic `libumbrello`.
- Exactly one executable; the binary is a thin wrapper around the static library.

---

## 2. UML Model (`umbrello/umlmodel/` — 69 source files)

### Purpose
The in-memory representation of all UML model elements — classes, interfaces, relationships, diagrams, stereotypes — and the operations that create, query, and modify them.

### Responsibilities
- Define the `ObjectType` enum (30+ types: `ot_Class`, `ot_Interface`, `ot_Actor`, `ot_UseCase`, etc.).
- Provide the class hierarchy: `UMLObject` → `UMLCanvasObject` → `UMLPackage` → `UMLClassifier` / `UMLFolder`.
- Own and manage all UML model objects via `UMLDoc` (singleton root document).
- Manage relationships (`UMLAssociation` with 2 `UMLRole` objects) for 25+ association types.
- Provide stereotype objects (`UMLStereotype`, reference-counted, owned by `UMLDoc::m_stereoList`).
- Provide 12+ list-type specializations for child management.

### Class Hierarchy

```
UMLObject (QObject)
 ├── UMLCanvasObject
 │    ├── UMLPackage
 │    │    ├── UMLClassifier
 │    │    │    ├── UMLClass
 │    │    │    ├── UMLInterface
 │    │    │    ├── UMLEnum
 │    │    │    ├── UMLDatatype
 │    │    │    └── UMLEntity
 │    │    ├── UMLFolder
 │    │    │    ├── Logical Folder
 │    │    │    ├── UseCase Folder
 │    │    │    ├── Component Folder
 │    │    │    ├── Deployment Folder
 │    │    │    └── EntityRelationship Folder
 │    │    └── UMLComponent
 │    ├── UMLActor
 │    ├── UMLUseCase
 │    ├── UMLNode
 │    ├── UMLPort
 │    ├── UMLArtifact
 │    └── UMLInstance
 └── UMLStereotype (special, ref-counted)
```

### Key Classes
| Class | Lines | Role |
|---|---|---|
| `UMLObject` | ~337 header | Base: ID, name, visibility, stereotype, documentation, abstract/static. 28 type-check methods (god object). |
| `UMLCanvasObject` | — | Adds `m_List` of subordinates (owned objects) and association-end management. |
| `UMLPackage` | — | Adds `m_objects` — list of contained standalone objects. |
| `UMLClassifier` | — | Adds attributes, operations, templates. |
| `UMLFolder` | — | Root-level container for diagrams (`m_diagrams`). |
| `UMLAssociation` | — | Owns 2 `UMLRole` objects; 25+ association types. |
| `UMLRole` | — | Multiplicity, changeability, pointer to participating `UMLObject`. |
| `UMLStereotype` | — | Reference-counted stereotype object. |
| `UMLDoc` | — | Singleton root: owns model trees, diagrams, stereotypes, autosave, XMI I/O. |

### Key Observations
- **God object**: `UMLObject` is a 337-line header with 28 type-check methods (`isClass()`, `isInterface()`, etc.) and a large union of fields.
- **Dual ownership**: Objects are simultaneously QObject children (parent hierarchy) and children of a `UMLPackage` (semantic hierarchy). This creates ambiguity.
- **Relationships stored in `m_List`**: Associations are tracked by the container package, not purely by role.
- **12+ typed lists**: `UMLObjectList`, `UMLClassifierList`, `UMLAssociationList`, etc., each with custom `copyInto()` / `clone()` patterns — significant code duplication.
- **Extended enum inheritance**: `ObjectType` is a flat enum, not hierarchical — type safety via `isX()` methods, not the type system.
- **Root UMLDoc** owns 5 `UMLFolder` singletons (Logical, UseCase, Component, Deployment, EntityRelationship), each a tree root.

---

## 3. Diagram & Widget System (`umbrello/umlwidgets/` — 94 source files)

### Purpose
Render UML diagrams as interactive 2D scenes, manage widget lifecycles, handle user interaction (selection, drag, resize, association drawing), and support multiple diagram types.

### Responsibilities
- Implement the `QGraphicsView` / `QGraphicsScene` pair: `UMLView` + `UMLScene`.
- Define the widget hierarchy: `WidgetBase` → `UMLWidget` → 29 concrete widget types.
- Render all diagram shapes using pure `QPainter` 2D (no SVG, no OpenGL).
- Handle association drawing (lines, labels, floating text) via `AssociationWidget` / `AssociationLine`.
- Manage interaction state via the ToolBarState pattern (7 state types).
- Support auto-layout via Graphviz dot (`LayoutGenerator`), grid snapping (`LayoutGrid`), and alignment guides (`AlignmentGuide`).
- Apply a state pattern for toolbar modes: `ToolBarStateArrow`, `ToolBarStateOneWidget`, etc.
- Serialize and deserialize widget state to/from XMI.

### Widget Hierarchy

```
QGraphicsObject
 └── WidgetBase
      └── UMLWidget
           ├── ClassifierWidget
           ├── ActorWidget
           ├── UseCaseWidget
           ├── PackageWidget
           ├── ComponentWidget / NodeWidget / ArtifactWidget
           ├── DatatypeWidget / EnumWidget / EntityWidget
           ├── ObjectWidget (sequence diagram)
           ├── NoteWidget / BoxWidget
           ├── StateWidget (12 state types)
           ├── ActivityWidget / SignalWidget / ObjectNodeWidget
           └── CombinedFragmentWidget (9 fragment types)
```

### Association Architecture
| Class | Role |
|---|---|
| `AssociationWidget` | Top-level association object (owns widget-level data) |
| `AssociationWidgetRole` | Per-endpoint data |
| `AssociationLine` | Geometry calculation (4 layouts: Direct, Orthogonal, Polyline, Spline) |
| `FloatingTextWidget` | Labels on associations |

### Diagram Types
| Diagram Type | Key Widgets |
|---|---|
| Class | `ClassifierWidget`, `AssociationWidget` |
| Use Case | `ActorWidget`, `UseCaseWidget` |
| Sequence | `ObjectWidget`, `MessageWidget`, `CombinedFragmentWidget`, `PreconditionWidget` |
| Collaboration | `ObjectWidget`, `MessageWidget` |
| State | `StateWidget` (12 types) |
| Activity | `ActivityWidget`, `SignalWidget`, `ObjectNodeWidget` |
| Component | `ComponentWidget`, `InterfaceWidget` |
| Deployment | `NodeWidget`, `ArtifactWidget` |
| Entity Relationship | `EntityWidget` |
| Object | `ObjectWidget` |

### State Pattern (Toolbar Modes)
- `ToolBarState` (abstract base)
- `ToolBarStatePool` (manage state transitions)
- `ToolBarStateArrow` (selection/view)
- `ToolBarStateOther` (non-association widgets)
- `ToolBarStateAssociation` (drawing associations)
- `ToolBarStateMessages` (sequence message creation)

### Key Observations
- **Widget_Factory** acts as the creation hub: maps `ObjectType` → `WidgetType` and constructs widgets from UML model objects or XMI tags.
- **Association widget** is one of the most complex classes in the system: geometry, routing, labels, endpoint management.
- **Sequence diagrams** have their own set of specialized interactions (lifelines, messages, combined fragments) layered on top of the general widget system.
- **No GPU rendering**: all drawing is `QPainter`-based; performance on large diagrams is a known concern.
- **`LayoutGenerator`** shell-executes Graphviz dot — this is a runtime dependency on an external tool.

---

## 4. Code Generators (`umbrello/codegenerators/` — ~203 files, 22 languages)

### Purpose
Generate source code from the UML model across 22 programming languages, with two distinct implementation strategies.

### Responsibilities
- Define `CodeGenerator` abstract base with two concrete styles: `SimpleCodeGenerator` (direct QTextStream) and `AdvancedCodeGenerator` (in-memory document tree).
- Provide per-language implementations for all 22 target languages.
- Build and manage an in-memory `CodeDocument` / `TextBlock` tree for the 4 advanced languages (C++, D, Java, Ruby).
- Enable code editing and viewer integration for advanced-language code.
- Maintain consistency between model and generated code via `syncCodeToDocument()`.
- Provide factory dispatch (`CodeGenFactory` namespace) by `ProgrammingLanguage` enum.

### Supported Languages
| Strategy | Languages |
|---|---|
| **Simple** (stream-based) | Ada, ActionScript, C#, D, IDL, JavaScript, Pascal, Perl, PHP4, PHP5, Python, Ruby, SQL, MySQL, PostgreSQL, Tcl, Vala, XML Schema |
| **Advanced** (document model) | C++, D, Java, Ruby |

### Document Model Architecture (Advanced)
```
TextBlock
 └── CodeBlock
      └── CodeBlockWithComments
           └── HierarchicalCodeBlock
                └── OwnedCodeBlock
                     └── CodeMethodBlock
                          ├── CodeOperation (methods/functions)
                          └── CodeAccessorMethod (getter/setter)

CodeParameter → CodeClassField (attributes/associations in code)
ClassifierCodeDocument (per-classifier document)
```

### Key Classes
| Class | Role |
|---|---|
| `CodeGenerator` | Abstract base: `writeClass()`, `newClassifierCodeDocument()` |
| `SimpleCodeGenerator` | Template method pattern: each language implements `writeClass()` |
| `AdvancedCodeGenerator` | Manages document tree, enables editing |
| `ClassifierCodeDocument` | Per-classifier code document with operations and class fields |
| `CodeGenFactory` | Switch-based dispatch by `ProgrammingLanguage` enum |
| `CodeGenerationWizard` | 3-page wizard UI for generation |
| `CodeGenPolicyExt` | Language-specific policy (overwrite, indentation, accessors) |

### Heading Templates
17 template files for file-level comment blocks (license headers, auto-generated warnings).

### Key Observations
- **Massive duplication**: ~203 files for 22 languages, but only 4 have the advanced document-model path.
- **C++ special-casing**: C++ gets disproportionate treatment across the codebase (default importer, most complex generator).
- **No template engine**: Each language manually writes streams / builds document trees — no template-based generation ("like Jinja/Mustache").
- **No plugin architecture**: Adding a new language requires modifying the central factory switch and adding files to the build.
- **`syncCodeToDocument()`** is the bidirectional sync between model and generated code — a challenging invariant to maintain.
- **CodeGenerationWizard** is the only UI for generation; there is no batch/headless mode.

---

## 5. Code Importers (`umbrello/codeimport/` + `lib/cppparser/` — 30+ files, 10+ languages)

### Purpose
Parse source-code files and reverse-engineer them into UML model objects.

### Responsibilities
- Provide a unified import API via `ClassImport` abstract base.
- Dispatch by file extension: `createImporterByFileExt()` → returns appropriate importer.
- Support 10+ languages: C++, C#, Java, Python, Ada, Pascal, IDL, SQL, PHP, Vala.
- Implement C++ import via a self-contained external parser library (`lib/cppparser/`).
- Provide `NativeImportBase` for line-based parsing (Python, Ada, Pascal, IDL, SQL).
- Bridge parsed constructs to UML model via `Import_Utils`.

### Architecture

```
ClassImport (abstract)
 ├── CppImport          ─── uses ──→ CppParser (lib/cppparser/)
 ├── PHPImport          ─── uses ──→ KDevelop PHP parser (optional)
 └── NativeImportBase (line-scanning)
      ├── PythonImport   (indentation → synthetic braces)
      ├── AdaImport
      ├── PascalImport
      ├── IDLImport      (calls external C preprocessor)
      ├── SQLImport
      └── JavaCsValaImportBase
           ├── JavaImport
           ├── CsImport
           └── ValaImport
```

### C++ Parser (`lib/cppparser/`)
| Component | Role |
|---|---|
| Lexer | Tokenizer with preprocessor (handles `#include`, `#define`, `#ifdef`) |
| Parser | Recursive-descent parser |
| AST | ~50 node types representing C++ constructs |
| Driver | Include resolution, file management |
| `CppTree2Uml` | Visitor: maps C++ AST → UML model objects |

### NativeImportBase (Line-Scanning)
| Method | Role |
|---|---|
| `preprocess()` | Strip comments, normalize whitespace |
| `split()` | Tokenize into words / symbols |
| `fillSource()` | Language-specific token transformation |
| `parseStmt()` | Recognize statement types (class, method, field, enum) |

### Import Pipeline
```
CodeImpSelectPage (UI) → CodeImpThread (background) → ClassImport::importFiles()
  → initialize() → parseFile() → Import_Utils creates UML objects
```

### Key Observations
- **C++ is the default fallback**: if no extension matches, C++ is assumed.
- **PHP import is optional**: only compiled when KDevPlatform is available — a significant extra dependency for one language.
- **IDL import depends on external C preprocessor** at runtime.
- **Python import** transforms indentation into synthetic braces — a clever but fragile approach.
- **`CppTree2Uml`** is a single visitor mapping ~50 AST node types — tightly coupled to both the parser and UML model.
- **`Import_Utils`** is the central bridge — any importer ultimately calls `createUMLObject()`, `insertAttribute()`, `makeOperation()`, `createGeneralization()` on it.
- C++ and PHP use external parser libraries; all other languages use the simpler line-scanning approach.

---

## 6. Persistence / XMI (`umldoc.cpp`, `umlobject.cpp`, `umlscene.cpp`, and per-class XMI methods)

### Purpose
Save and load the full UML model (objects, diagrams, relationships, stereotypes) to/from the XMI interchange format.

### Responsibilities
- Serialize the entire `UMLDoc` model tree to XML via `QXmlStreamWriter`.
- Deserialize XMI via `QDomDocument` (full DOM tree).
- Support two XMI versions: UML 1.2 (legacy, default) and UML 2.1 (optional).
- Handle forward-reference resolution via `resolveRef()` deferred pass.
- Store diagrams as part of `UMLFolder` serialization.
- Provide autosave (QTimer-based to `~/autosave.xmi`).
- Support multiple file formats (`.xmi`, `.xmi.tgz`, `.xmi.tar.bz2`, `.zargo`).

### Key Methods
| Method | Location | Role |
|---|---|---|
| `UMLDoc::saveToXMI()` | `umldoc.cpp` | Streaming serialization of entire document |
| `UMLDoc::loadFromXMI()` | `umldoc.cpp` | DOM-based parsing of entire document |
| `UMLObject::saveToXMI()` | `umlobject.cpp` | Base object serialization |
| `UMLObject::loadFromXMI()` | `umlobject.cpp` | Base object deserialization |
| `UMLScene::saveToXMI()` | `umlscene.cpp` | Widget/diagram state serialization |
| `resolveRef()` | Various | Deferred forward-reference resolution pass |

### XMI Versions
| Version | Features |
|---|---|
| **UML 1.2** (default) | Uses `xmi.id` attribute, `UML:` namespace prefix |
| **UML 2.1** (optional) | Uses `xmi:id` attribute, `packagedElement` containment |

### File Formats
| Extension | Format |
|---|---|
| `.xmi` | Plain XML |
| `.xmi.tgz` | GNU zip tar archive |
| `.xmi.tar.bz2` | Bzip2 tar archive |
| `.zargo` | ArgoUML legacy ZIP format |

### Version Constants
- `XMI1_FILE_VERSION = "1.7.6"`
- `XMI2_FILE_VERSION = "2.0.4"`

### DTDs
- `uml-1.4-umbrello.dtd`
- `umbrello-diagrams.dtd`
- `umbrello-misc.dtd`
- `uml241.dtd`

### Key Observations
- **Mixed streaming vs DOM**: Save uses streaming (`QXmlStreamWriter`, efficient); load uses full DOM (`QDomDocument`, memory-intensive). This is a historical inconsistency.
- **Deferred loading**: `resolveRef()` requires a post-processing pass because forward references are common in XMI.
- **Diagrams in folders**: Since v1.5.5, diagrams are serialized inside `UMLFolder`; previously they were in `XMI.extensions` — backwards-compatible loading handles both.
- **Autosave** is a raw timer-based write to a fixed path — no debouncing, no incremental save.
- **Every model class** implements `saveToXMI()` / `loadFromXMI()` — serialization is scattered across the class hierarchy, not centralized.

---

## 7. Undo / Redo (`umbrello/cmds/` — 20 command classes)

### Purpose
Provide comprehensive undo/redo for all user-visible model and diagram operations using Qt's `QUndoStack` framework.

### Responsibilities
- Define a command hierarchy rooted in `QUndoCommand`.
- Implement per-operation commands: create/remove/move/resize/rename/color/font/text changes.
- Support macro grouping for compound operations.
- Disable undo during file loading.

### Command Hierarchy

```
QUndoCommand
 ├── CmdBaseObjectCommand
 │    ├── CmdSetVisibility
 │    ├── CmdSetStereotype
 │    └── (model property changes)
 ├── CmdBaseWidgetCommand (14 widget commands)
 │    ├── CmdCreateWidget / CmdRemoveWidget
 │    ├── CmdMoveWidget / CmdResizeWidget
 │    ├── CmdChangeColor / CmdChangeFont
 │    ├── CmdChangeText
 │    └── (additional widget properties)
 ├── CmdCreateUMLObject
 ├── CmdRemoveUMLObject
 ├── CmdRenameUMLObject
 ├── CmdCreateDiagram
 ├── CmdRemoveDiagram
 └── CmdHandleRename
```

### Key Class
| Class | Role |
|---|---|
| `UMLApp::executeCommand()` | Central push method: `m_pUndoStack->push(cmd)` |
| `QUndoStack` | Qt framework undo stack (not custom) |
| `beginMacro()` / `endMacro()` | Group multiple operations into one undo step |

### Key Observations
- **Clean separation**: model commands (`CmdBaseObjectCommand`) vs widget commands (`CmdBaseWidgetCommand`) follow the model/view split.
- **Qt framework reuse**: `QUndoStack` is used directly — no custom undo infrastructure.
- **`executeCommand()`** is the sole gateway for undoable operations, providing a consistent enforcement point.
- **Undo disabled during load** via a simple boolean flag to avoid recording initialization as undoable events.
- **No redo limit** — unlimited undo stack could become a memory issue on very large sessions.

---

## 8. User Interface Architecture

### Purpose
Deliver the complete desktop application UI: main window, menus, toolbars, dock widgets, dialogs, and deep KDE platform integration.

### Responsibilities
- Manage the main window (`UMLApp`, singleton `KXmlGuiWindow`): menus, toolbars, status bar.
- Host multiple diagram editors in a tabbed or stacked central area.
- Provide dock widgets: tree view, documentation viewer, undo history, log, birdview, stereotypes, diagrams list, objects window.
- Implement all dialogs (properties, settings, wizards) via KDE dialog infrastructure.
- Provide context-sensitive menus via `ListPopupMenu` (260+ `MenuType` entries).
- Manage application settings through a 3-layer settings architecture.

### Main Window Architecture

```
UMLApp (KXmlGuiWindow)
 ├── Central Area: QTabWidget / QStackedWidget
 │    └── UMLView (QGraphicsView) + UMLScene (QGraphicsScene) per open diagram
 ├── Dock Widgets:
 │    ├── UMLListView (QTreeWidget) — model tree
 │    ├── DocWindow — documentation editor
 │    ├── QUndoView — undo history
 │    ├── QListWidget — log
 │    ├── BirdView — miniature overview
 │    ├── Stereotypes dock
 │    ├── Diagrams dock
 │    └── UML Objects dock (unstable)
 ├── Toolbar: KToolBar
 └── Status bar
```

### Dialog Architecture
| Pattern | Base Class | Examples |
|---|---|---|
| Multi-page | `MultiPageDialogBase` (KPageWidget) | Properties dialogs, Settings (8 pages) |
| Single-page | `SinglePageDialogBase` | Simple input/confirmation dialogs |
| Wizards | CodeGenerationWizard (3 pages), CodeImportWizard | Multi-step guided workflows |

### Settings Architecture (3-layer)
| Layer | Mechanism | Scope |
|---|---|---|
| 1. Persistent storage | KConfig XT (`UmbrelloSettings` singleton) | File-based ini-style config |
| 2. In-memory snapshot | `Settings::OptionState` | Cached runtime settings |
| 3. Manual groups | `KSharedConfig` | Direct KConfig group access |

### Context Menus
- `ListPopupMenu`: 260+ `MenuType` enumeration entries.
- Menu content varies by widget type, selection count, and application state.

### Key Observations
- **Deep KDE integration**: `KXmlGuiWindow`, `KActionCollection`, `KToolBar`, `KConfig`, `KTextEditor`, `KPageDialog`, `KLocalizedString` — the application is inseparable from KDE frameworks.
- **Singleton main window**: `UMLApp` is accessed via `UMLApp::app()` throughout the codebase.
- **Dead plugin system**: `_unused/` contains `Plugin`, `PluginLoader`, and `Configurable` classes that were never completed.
- **Single `ListPopupMenu`**: All context menus flow through one file — 260+ menu types creates a maintenance burden.

---

## 9. CLI & Headless Interface

### Purpose
Enable non-interactive use of Umbrello for export, import, and language configuration.

### Responsibilities
- Parse command-line options in `main.cpp`.
- Operate in headless mode when `--export` is specified.
- Support export to multiple image/doc formats.
- Support batch import of source files.
- List available languages and export formats.

### CLI Options
| Option | Behavior |
|---|---|
| `--export <ext>` | Export diagrams and exit (headless) |
| `--export-formats` | List available export formats |
| `--directory <url>` | Export target directory |
| `--use-folders` | Preserve tree structure on export |
| `--import-files <files...>` | Batch import source files |
| `--import-directory <dir>` | Import all files from directory |
| `--languages` | List supported programming languages |
| `--set-language <lang>` | Set active code generation language |

### Export Formats
SVG, EPS, PNG, BMP, JPEG, DOT, and more (driven by `QImageWriter` supported formats).

### Key Observations
- **Headless mode** disables the GUI and runs the event loop only for the export/import pipeline.
- **No batch generation**: there is no `--generate <language>` flag — code generation is GUI-only (CodeGenerationWizard).
- **Import is available headless**, but generation is not — an asymmetry.
- **Format list** comes from `QImageWriter::supportedImageFormats()` at runtime.

---

## 10. Test Infrastructure

### Purpose
Provide unit, integration, and regression testing for all subsystems.

### Framework
- Qt Test (`QObject` + `Q_SLOTS`), **not** Google Test.
- Test templates: `TestUML<T,N>` and `TestWidget<T,N>` for save/load round-trip testing.
- 11 standard tests + 2 optional LLVM-dependent tests.

### Test Organization
| Directory | Contents |
|---|---|
| `unittests/` | Test sources and CMakeLists |
| `test/import/` | Sample source files per language for import testing |
| `test/test-*.xmi` | 9 XMI test files for save/load round-trips |

### Test Execution
```sh
cmake --build build && ctest --test-dir build -VV
# Single test:
./build/unittests/testbasictypes
```

### Runtime Requirements
- Display required: set `QT_QPA_PLATFORM=offscreen` or use `xvfb-run`.
- Environment: `LANG=C.UTF-8 QT_LOGGING_RULES=umbrello.debug=false`.

### Key Observations
- **Round-trip testing**: `TestUML<T,N>` / `TestWidget<T,N>` automatically save and reload model elements, a valuable pattern for ensuring XIM serialization correctness.
- **Qt Test framework**: Test functions are slots named `testX()`; no assertions framework like Google Test — `QCOMPARE`, `QVERIFY` etc.
- **No mocking framework**: Tests create real model objects.
- **LLVM-dependent tests**: Two tests (likely C++ import related) require LLVM and are optional.

---

## 11. DocBook / XHTML Generation

### Purpose
Generate documentation from the UML model in DocBook XML format, then transform to XHTML.

### Pipeline
```
UML Model → XMI → [XSLT] → DocBook → [XSLT] → XHTML
```

### Architecture
| Component | Role |
|---|---|
| `XMI → DocBook XSLT` | Transform XMI to DocBook intermediate format |
| `DocBook → XHTML XSLT` | Transform DocBook to final XHTML |
| Threaded jobs | XSLT transformations run in background threads |

### Key Observations
- **Dual XSLT pipeline**: Two separate transformations with different stylesheets.
- **LibXslt dependency**: XSLT processing via `libxslt` (system dependency).
- **Threaded**: Running XSLT in a separate thread avoids blocking the UI.
- **No direct HTML generation**: Everything goes through DocBook as an intermediate; no programmatic HTML builder.

---

## 12. Graphviz Integration

### Purpose
Provide automatic diagram layout using Graphviz's `dot` layout engine.

### Components
| Class | Role |
|---|---|
| `DotGenerator` | Generate `.dot` file from UML model / diagram |
| `LayoutGenerator` | Execute `dot` process, parse output, apply coordinates |

### Pipeline
```
DotGenerator: UML Diagram → DOT file
                   ↓ (shell exec: `dot -Tplain`)
LayoutGenerator: DOT output → parsed coordinates → widget positions
```

### Key Observations
- **External dependency**: Requires Graphviz `dot` binary at runtime.
- **`LayoutGenerator`** parses `dot -Tplain` output format — a text-based protocol.
- **No fallback**: If `dot` is not installed, auto-layout is unavailable (no built-in layout algorithm).
- **Used on demand**: Layout is triggered manually by the user, not automatically.
- **Shell execution**: The `QProcess` call to `dot` is synchronous.

---

## 13. Foreign Format Import (Rose / ArgoUML)

### Purpose
Import models from legacy CASE tools: IBM Rational Rose (`.mdl`) and ArgoUML (`.zargo` / `.xmi`).

### Architecture
| Component | Role |
|---|---|
| `Import_Rose` | Parse Rational Rose `.mdl` files |
| `Import_Argo` | Parse ArgoUML `.zargo` / `.xmi` files |
| `PetalNode` | Internal tree representation for Rose `.mdl` format |

### Key Observations
- **PetalNode** is a recursive tree structure representing the Rose MDL format.
- **ArgoUML import** reuses the standard XMI loading path for `.xmi` files but handles ArgoUML-specific extensions.
- **Rose import** is line-by-line parsing of a proprietary format — a standalone parser, not shared with anything else.
- **Both are one-shot conversion tools**: they create UML model objects and the import is complete; no round-trip or sync.

---

## 14. Search / Find

### Purpose
Search for model elements by name across the document, scene, and list view.

### Architecture
| Class | Role |
|---|---|
| `UMLFinder` | Abstract base for find operations |
| `UMLDocFinder` | Search the entire UML document model |
| `UMLSceneFinder` | Search the active diagram scene widgets |
| `UMLListViewFinder` | Search the tree view items |
| `FindDialog` | UI for search input and results |

### Key Observations
- **Multiple finders**: Different subsystems each get their own finder, but all share the `UMLFinder` interface.
- **Simple string matching**: No regex or advanced search patterns — case-insensitive substring match on names.
- **Modal dialog**: `FindDialog` is modal and provides find-next / find-all semantics.
- **No replace**: This is search-only, no search-and-replace.

---

## 15. Clipboard & Drag-and-Drop

### Purpose
Support copy/paste and drag-and-drop of UML model elements within and between diagrams.

### Architecture
| Class | Role |
|---|---|
| `UMLClipboard` | Manages 5 clip types (object, widget, diagram, association, image) |
| `UMLDragData` | QMimeData subclass for drag-and-drop |

### Clip Types
| Type | Content |
|---|---|
| Object | UML model object reference |
| Widget | Diagram widget (includes underlying model object) |
| Diagram | Entire diagram |
| Association | Relationship between objects |
| Image | Rendered diagram as image |

### Key Observations
- **5 distinct clip types**: Each serialized differently; some carry model references, others carry serialized data.
- **`UMLDragData`** implements custom MIME types for UML element drag-and-drop.
- **Copy/paste uses XMI**: Objects are serialized to XMI for clipboard transport — reuses the XMI serialization path.
- **Cross-diagram paste**: Widgets can be pasted into any compatible diagram type.

---

## 16. BirdView

### Purpose
Provide a miniature overview of the active diagram for navigation.

### Architecture
| Class | Role |
|---|---|
| `BirdView` (dock widget) | Minimap of the active `UMLScene` |
| Connection to `UMLView` | Tracks viewport position and zoom level |

### Key Observations
- **Live update**: The birdview updates in real time as the diagram changes.
- **Navigation**: Clicking/dragging in the birdview repositions the main viewport.
- **Own diagram renderer**: The birdview renders the scene at a fixed scale, not a scaled copy of the main view.
- **Small, self-contained**: One of the simpler subsystems.

---

## 17. Debug / Logging

### Purpose
Provide developer-oriented debugging and tracing capabilities, including per-class logging control.

### Architecture
| Component | Role |
|---|---|
| `Tracer` (singleton) | Per-class trace-enable/disable |
| Log macros | Conditional debug output controlled by `Tracer` |
| Log dock widget | `QListWidget` for runtime log display |

### Key Observations
- **Per-class debug control**: Each class can enable/disable its own debug output independently.
- **Global singleton**: `Tracer` is accessed globally.
- **No structured logging**: Debug output is unstructured text strings; no log levels beyond enabled/disabled.
- **Log widget**: The in-application log display is a simple `QListWidget` — useful for debugging but not for production diagnostics.

---

## 18. Refactoring Assistant

### Purpose
Provide a UI for reviewing and managing classifier relationships.

### Architecture
| Class | Role |
|---|---|
| `RefactoringAssistant` | `QTreeWidget`-based dialog for relationship review |

### Key Observations
- **Simple UI**: Tree widget displays relationships (associations, generalizations, dependencies).
- **No actual refactoring**: Despite the name, this is a review/visualization tool, not a refactoring engine.
- **Classifier-focused**: Only works with `UMLClassifier` objects and their relationships.
- **Limited scope**: Does not support operations like rename-through, extract interface, or pull-up/push-down members.

---

## Appendix: Subsystem Dependency Graph

```
                  ┌──────────────┐
                  │   UML Model   │
                  │   (69 files)  │
                  └──────┬───────┘
                         │
          ┌──────────────┼──────────────┐
          │              │              │
          ▼              ▼              ▼
   ┌──────────┐   ┌──────────┐   ┌──────────┐
   │ Widgets  │   │   XMI    │   │  Code    │
   │(94 files)│   │ Persist. │   │Gen(203)  │
   └────┬─────┘   └──────────┘   └────┬─────┘
        │                              │
        ▼                              ▼
   ┌──────────┐                  ┌──────────┐
   │ BirdView │                  │  Code    │
   │ Clipboard│                  │ Import   │
   │  Search  │                  │(30 files)│
   │  Undo    │                  └────┬─────┘
   │ Graphviz │                       │
   └──────────┘                       ▼
                               ┌──────────┐
                               │C++ Parser│
                               │(lib/     │
                               │ cppparser)│
                               └──────────┘

   ┌─────────────────────────────────────────┐
   │              UI Architecture            │
   │  UMLApp · UAVikimView · Dialogs · Menus │
   │  Settings · KDE Integration · CLI       │
   └─────────────────────────────────────────┘
```

---

## Appendix: File Count by Subsystem

| Subsystem | Approximate File Count |
|---|---|
| UML Model (`umlmodel/`) | 69 |
| Diagram Widgets (`umlwidgets/`) | 94 |
| Code Generators (`codegenerators/`) | ~203 |
| Code Import (`codeimport/`) | 30+ |
| C++ Parser (`lib/cppparser/`) | ~20 |
| Dialogs | ~40 |
| Commands (`cmds/`) | 20+ |
| UI / Main Window | ~15 |
| Tests (`unittests/`) | 13+ |
| Doc Generators (`docgenerators/`) | ~10 |
| Rose/Argo Import | ~5 |
| Other | ~20 |
| **Total (approx.)** | **~550+** |
