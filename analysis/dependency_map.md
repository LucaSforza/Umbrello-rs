# Umbrello Dependency Map

> Generated from comprehensive codebase analysis of the C++ Umbrello codebase.
> Purpose: Guide the Rust rewrite by mapping subsystem relationships, coupling points,
> cyclic dependencies, and recommended crate boundaries.

---

## 1. Subsystem Overview

The codebase lives in `umbrello/` (the static `libumbrello` library) plus `lib/` (external
parser libraries). The main binary is a thin `main.cpp` linked against `libumbrello`.

| # | Subsystem | Path | Role | Qt/KDE Dependency |
|---|-----------|------|------|-------------------|
| 1 | **UML Model** | `umbrello/umlmodel/` | Core domain objects (79 files) | QObject (base), QDom, QXmlStreamWriter |
| 2 | **UMLDoc** | `umbrello/umldoc.h/.cpp` | Document container, owns root folders, stereotypes, views | QObject, KConfig |
| 3 | **UMLScene/UMLView** | `umbrello/umlscene.*`, `umbrello/umlview.*` | QGraphicsScene/View for diagrams | QGraphicsScene, QGraphicsView |
| 4 | **UMLWidgets** | `umbrello/umlwidgets/` | All diagram widgets (94 files) | QGraphicsObject, depends on UML Model |
| 5 | **Code Generators** | `umbrello/codegenerators/` | 15+ language code generators | UML Model, CodeDocument model |
| 6 | **Code Importers** | `umbrello/codeimport/` | C++, Java, Python, etc. importers | UML Model, Import_Utils, external parsers |
| 7 | **Persistence (XMI)** | Cross-cutting | saveToXMI/loadFromXMI on every model object | QDom, QXmlStreamWriter |
| 8 | **Undo/Redo** | `umbrello/cmds/` | QUndoCommand wrappers | UML Model, UMLWidgets |
| 9 | **UI Layer** | `umbrello/umlapp.*`, `umbrello/dialogs/`, `umbrello/menus/` | Main window, dialogs, context menus | KXmlGuiWindow, QWidget, everything |
| 10 | **Settings** | `umbrello/optionstate.*`, `umbrello/umbrello.kcfg` | Application configuration | KConfig, KConfigXt |
| 11 | **Toolbar States** | `umbrello/toolbarstate*` | Interaction mode state machine | UMLScene, UMLWidgets |
| 12 | **Clipboard** | `umbrello/clipboard/` | Copy/paste of model+widget elements | UML Model, UMLWidgets |
| 13 | **Search/Find** | `umbrello/finder/` | Find UML objects across model+views | UML Model, UMLListView, UMLScene |
| 14 | **Refactoring** | `umbrello/refactoring/` | Refactoring assistant | UML Model |
| 15 | **Doc Generators** | `umbrello/docgenerators/` | DocBook → XHTML generation | XMI output, XSLT |
| 16 | **GraphViz/Layout** | `umbrello/layoutgenerator.*` | Auto-layout via GraphViz dot | UMLScene (reads positions) |
| 17 | **Dot Export** | `umbrello/dotgenerator.*` | DOT file export | UMLScene |
| 18 | **Debug** | `umbrello/debug/` | Logging, tracer | Qt (QDebug), UMLApp (for log routing) |
| 19 | **Model Utils** | `umbrello/model_utils.*` | Cross-cutting helpers | UML Model, UMLListView, WidgetBase |
| 20 | **Object Factory** | `umbrello/object_factory.*` | Creates any UMLObject by type enum | UML Model (all concrete types) |
| 21 | **Widget Factory** | `umbrello/umlwidgets/widget_factory.*` | Creates widgets from model objects | UMLWidgets (all concrete types) |
| 22 | **CodeGen Factory** | `umbrello/codegenerators/codegenfactory.*` | Creates code generators by language | All code generator classes |
| 23 | **Diagram Utils** | `umbrello/diagram_utils.*` | Diagram creation helpers | UML Model, UMLScene |
| 24 | **External: C++ Parser** | `lib/cppparser/` | Self-contained C++/C++11 parser | None (standalone) |
| 25 | **External: Interfaces** | `lib/interfaces/` | Shared code model interfaces | None |
| 26 | **External: PHP Parser** | `lib/kdev5-php/` | PHP parser | KDevPlatform |
| 27 | **External: KDevPlatform** | `lib/kdevplatform/` | KDevelop parser infrastructure | KDE |

---

## 2. Text-based Dependency Graph

```
Legend:
  ───>  depends on (direction of arrow)
  ───x  cyclic dependency
  ───~  cross-cutting / pervasive dependency
  ┌────┐ subsystem boundary


                    ┌──────────────────────────────────────────────────┐
                    │                   UMLApp  (umlapp.*)             │
                    │  "God Object" — 175 files reference app()        │
                    │  Owns: m_doc, m_view, m_codegen, m_config ...    │
                    └────┬──────────┬──────────┬───────────┬───────────┘
                         │          │          │           │
               ┌─────────▼──┐  ┌────▼────┐ ┌──▼──────┐  ┌─▼──────────┐
               │  UMLDoc    │  │UMLScene │ │ Dialogs │  │  Menus     │
               │  (umldoc)  │  │(umlscene│ │ (80+)   │  │            │
               └──┬──┬──┬───┘  │ ,umlview)│ └─────────┘  └────────────┘
                  │  │  │      └──┬──┬────┘
       ┌──────────┘  │  │         │  │
       ▼              │  │         │  │
  ┌────────┐         │  │         │  │
  │UMLFolder│◄────────┘  │         │  │
  │(root x5)│          │         │  │
  └──┬─────┬┘          │         │  │
     │     │            │         │  │
     ▼     │            │         │  │
  ┌──────┐ │            │         │  │
  │UMLPkg│ │            │         │  │
  └──┬───┘ │            │         │  │
     │     │            │         │  │
     ▼     │            │         │  │
  ┌──────────┐          │         │  │
  │ UMLObject│◄─────────┼─────────┼──┘
  │ (base,   │          │         │
  │ 79 files)│          │         │
  └──┬───┬───┘          │         │
     │   │              │         │
     │   └──────────────────────┐ │
     │               │         │ │
     ▼               ▼         │ │
  ┌────────┐  ┌───────────┐    │ │
  │UMLAssoc│  │UMLClassif.│    │ │
  │+UMLRole│  │+subclasses│    │ │
  └────────┘  └───────────┘    │ │
                               │ │
         ┌─────────────────────┘ │
         │                       │
         ▼                       ▼
  ┌────────────────┐  ┌──────────────────────┐
  │  WidgetBase     │  │  AssociationWidget    │
  │  (QGraphicsObj) │  │  (WidgetBase+LinkW.)  │
  │  + UMLWidget    │  │                       │
  │  (94 files)     │  │  * has UMLScene        │
  │  * m_umlObject  │  │  * m_umlObject (opt)   │
  │  * m_scene      │  └──────────────────────┘
  └───────┬─────────┘
          │
          └──► UMLObject (via QPointer)

  ┌──────────────────────────────────────┐
  │          Code Generators             │
  │  ┌─────────┐  ┌──────────────────┐   │
  │  │CodeGen  │  │ CodeGenFactory    │   │
  │  │(base)   │  │ (switch on lang)  │   │
  │  └────┬────┘  └──────────────────┘   │
  │       │                              │
  │  ┌────▼──────────────────────────┐   │
  │  │ ClassifierCodeDocument        │   │
  │  │ + CodeOperation,CodeClassFld  │   │
  │  │ + TextBlock (hierarchy)       │   │
  │  └───────────┬──────────────────┘   │
  │              │                      │
  │              ▼                      │
  │  ┌────────────────────────┐         │
  │  │ CodeGenerationPolicy   │         │
  │  │ (settings)             │         │
  │  └────────────────────────┘         │
  └──────────────────────────────────────┘
                    │
                    ▼
           UML Model Objects

  ┌──────────────────────────────────────┐
  │          Code Importers               │
  │  ┌──────────┐  ┌──────────────────┐   │
  │  │ImportBase│  │ Import_Utils     │   │
  │  └────┬─────┘  └────────┬─────────┘   │
  │       │                 │             │
  │  ┌────▼─────┐    ┌──────▼──────────┐  │
  │  │CppImport │    │Object_Factory   │  │
  │  │JavaImport│    │UMLDoc           │  │
  │  │Python... │    │UMLFolder, etc.  │  │
  │  └──────────┘    └─────────────────┘  │
  └──────────────────────────────────────┘
                    │
                    ▼
           UML Model Objects

  ┌──────────────────────────────────────┐
  │          Persistence (XMI)            │
  │  saveToXMI/loadFromXMI:              │
  │  → UMLDoc::saveToXMI                 │
  │    → UMLFolder::saveToXMI            │
  │      → UMLObject::saveToXMI          │
  │        → specialized per subclass    │
  │                                      │
  │  ~ CROSS-CUTTING: 50+ classes        │
  │    each implement save/load          │
  └──────────────────────────────────────┘

  ┌──────────────────────────────────────┐
  │          Undo/Redo (cmds/)            │
  │  ┌─────────────────┐                 │
  │  │ cmd/ (generic)  │──► UML Object   │
  │  │ cmd/ (widget)   │──► UMLWidget    │
  │  │ CmdBaseObjCmd   │──► UndoStack    │
  │  └─────────────────┘                 │
  └──────────────────────────────────────┘

  ┌──────────────────────────────────────┐
  │          Settings/Config             │
  │  OptionState (Singleton)             │
  │  CodeGenerationPolicy                │
  │  kcfg → KConfig                      │
  │                                      │
  │  ~ PERVASIVE: optionState() called   │
  │    from widgets, model, generators   │
  └──────────────────────────────────────┘

  ┌──────────────────────────────────────┐
  │          External Parsers            │
  │                                      │
  │  lib/cppparser/  (standalone)        │
  │    ──► C++ source → AST              │
  │        No Umbrello dependency        │
  │                                      │
  │  lib/kdev5-php/  (KDE dependent)    │
  │  lib/kdevplatform/                   │
  │                                      │
  │  lib/interfaces/                     │
  │    Shared code model abstractions    │
  └──────────────────────────────────────┘
```

### Summary Flow

```
CLI ──► UMLApp ──► UMLDoc ──► UMLFolder ──► UMLPackage ──► UMLObject
                        │                                          │
                        ▼                                          ▼
                   UMLScene ◄──► UMLWidget ──► WidgetBase ──► QGraphicsObject
                        │
                        ▼
                   CodeImporter / CodeGenerator / Persistence (XMI)
```

---

## 3. Coupling Analysis

### 3.1 Tight Coupling Points

#### 3.1.1 UMLApp God Object (CRITICAL)

**What:** Singleton `UMLApp::app()` accessed from **175 files** across every subsystem.

**Why it exists:**
- KDE's `KXmlGuiWindow` is the natural application root
- Every subsystem needs access to: `document()`, `currentView()`, `generator()`, `config()`, `activeLanguage()`, `undoStack()`, logging
- No dependency injection — subsystems reach for the global

**Call-site distribution:**
| Subsystem | Approximate count |
|-----------|------------------|
| Debug macros (all callers transitively) | 40+ macros |
| UMLWidgets | ~20 files |
| Dialogs | ~25 files |
| Code Generators | ~15 files |
| Code Importers | ~10 files |
| Find/Search | ~5 files |
| Menus | ~10 files |
| Other | ~50 files |

**Decoupling strategy for Rust:**
1. Replace singleton with an **AppContext** struct injected into subsystems via trait bounds
2. Define fine-grained traits that subsystems actually need:
   ```rust
   trait ModelProvider { fn document(&self) -> &UmlDoc; }
   trait ActiveViewProvider { fn current_view(&self) -> Option<&UmlScene>; }
   trait Logger { fn log(&self, level: Level, msg: &str); }
   trait ConfigProvider { fn config(&self) -> &Settings; }
   trait UndoProvider { fn undo_stack(&self) -> &UndoStack; }
   ```
3. Main function assembles the context and passes references downward
4. Only the UI shell (main window) should know about all pieces

#### 3.1.2 XMI Serialization Scattered (CROSS-CUTTING)

**What:** Every UMLObject subclass implements `saveToXMI()`/`loadFromXMI()`. The serialization logic is scattered across 50+ classes.

**Why it exists:**
- XMI format requires type-specific element/attribute handling
- No central serialization registry — each class knows its own XMI schema
- Loading uses `Object_Factory::makeObjectFromXMI()` which switches on XMI tag

**Decoupling strategy for Rust:**
1. Define a `Serialize` / `Deserialize` trait separate from domain objects
2. Use Serde's derive macros where possible
3. Keep serializer/deserializer as separate modules that know about model types
4. Consider a visitor pattern or type-erased serialization registry:
   ```rust
   trait XmiSerializable {
       fn xmi_tag(&self) -> &'static str;
       fn serialize(&self, writer: &mut XmlWriter) -> Result<()>;
   }
   
   // Registry maps tag → factory function
   type Deserializer = fn(&mut XmlReader) -> Result<Box<dyn UmlObject>>;
   ```

#### 3.1.3 Factory Switch Statements

**What:** Three factories switch on type enums to create concrete instances:
- `Object_Factory::createUMLObject()` (all 30+ UMLObject types)
- `Widget_Factory::createWidget()` (all 25+ widget types)
- `CodeGenFactory::createObject()` (all 15+ languages)

**Why it exists:**
- C++ lacks a type-safe way to map enum → constructor
- Adding any new subtype requires modifying the factory
- Violates Open/Closed Principle

**Decoupling strategy for Rust:**
1. Use a `Registry` pattern with `dyn Fn()` or `TypedBuilder`:
   ```rust
   #[derive(Default)]
   struct ObjectFactory {
       creators: HashMap<ObjectType, Box<dyn Fn() -> Box<dyn UmlObject>>>,
   }
   
   impl ObjectFactory {
       fn register<T: UmlObject + 'static>(&mut self, ot: ObjectType) {
           self.creators.insert(ot, Box::new(|| Box::new(T::default())));
       }
   }
   ```
2. Each model crate registers its types at startup or via `ctor`/`linkme`
3. No central switch statement — extensible by design

#### 3.1.4 OptionState Singleton (Settings)

**What:** `Settings::OptionState` is a singleton accessed via `optionState()` / `setOptionState()` across the entire codebase.

**Why it exists:**
- KConfig-based settings loaded at startup
- Every subsystem reads settings (colors, fonts, toggles)
- Settings are scattered in `optionstate.h` as plain structs

**Decoupling strategy for Rust:**
1. Replace with a `Settings` struct that is passed as `&Settings` to functions that need it
2. Use fine-grained setting groups rather than one monolithic struct
3. Use `serde` for serialization instead of `KConfig`
4. Consider an event bus for notification of setting changes

#### 3.1.5 QObject Base Class (PERVASIVE)

**What:** Every UMLObject and WidgetBase inherits from QObject/QGraphicsObject.

**Why it exists:**
- Qt memory management (parent-child tree)
- Qt signals/slots for model-view communication
- Q_PROPERTY for introspection

**Decoupling strategy for Rust:**
1. **No signal/slot replacement needed**: Use Rust channels, `tokio::sync`, `event_listener`, or a dedicated event bus
2. **Memory management**: Use `Rc<RefCell<>>` or `Arc<Mutex<>>` only when needed; prefer `Box` with owned parent references
3. **Object identification**: Replace QObject parent tree with explicit references
4. **Q_PROPERTY**: Not needed — use regular Rust fields

### 3.2 Coupling Metrics

| Dependency Pair | Direction | Tightness | Reason |
|----------------|-----------|-----------|--------|
| UMLApp → Everything | Outgoing | ★★★★★ | Singleton, 175 call sites |
| UMLWidget → UMLObject | Outgoing | ★★★★☆ | m_umlObject QPointer, signals |
| UMLDoc → UMLFolder | Mutual | ★★★★☆ | Owns folders, folders reference doc |
| UMLScene → UMLWidget | Mutual | ★★★★☆ | Scene owns widgets, widgets reference scene |
| Object_Factory → All model types | Outgoing | ★★★☆☆ | Must know all types |
| CodeGenFactory → All generators | Outgoing | ★★★☆☆ | Must know all languages |
| XMI Code → All model types | Outgoing | ★★★☆☆ | Every class implements save/load |
| Widget_Factory → All widget types | Outgoing | ★★★☆☆ | Must know all widget types |
| Settings → All subsystems | Outgoing | ★★★☆☆ | Global state read everywhere |
| Dialogs → UMLApp, Model, Widgets | Outgoing | ★★★★☆ | UI depends on everything |
| Import_Utils → Object_Factory | Outgoing | ★★★☆☆ | Creates objects during import |
| Undo/Redo → Model + Widgets | Outgoing | ★★☆☆☆ | Commands wrap model/widget operations |
| Debug → UMLApp | Outgoing | ★★☆☆☆ | Macro-based log routing |
| Clipboard → Model + Widgets | Outgoing | ★★★☆☆ | Serializes/deserializes both |

---

## 4. Cyclic Dependencies

### 4.1 Identified Cycles

```
Cycle 1: UMLDoc ↔ UMLFolder
  UMLDoc owns UMLFolder[5 root folders]
  UMLFolder has UMLDoc as friend class, references m_doc (indirectly)
  UMLFolder::load1() calls back to UMLDoc
  UMLDoc::rootFolder() returns UMLFolder
  Break: Make folders unaware of their containing document.
          Pass a &UmlDoc reference when needed as a parameter.

Cycle 2: UMLScene ↔ UMLWidget
  UMLScene owns: m_WidgetList (UMLWidgetList)
  UMLWidget has: m_scene (UMLScene*)
  UMLScene calls: widget->setScene(), widget->update()
  UMLWidget calls: scene()->removeWidget(), scene()->addWidget()
  Break: Use a scene registry / interior mutability.
          Widget stores scene ID; queries scene via context.
          Or make scene a parameter passed to widget operations.

Cycle 3: UMLWidget ↔ AssociationWidget
  UMLWidget has: m_Assocs (AssociationWidgetList)
  AssociationWidget has: m_widget[2] (UMLWidget* for endpoints)
  AssociationWidget calls widget->addAssoc(this)
  UMLWidget calls association->setScene()
  Break: Store association relationship in Scene, not in widgets.
          Scene is the natural owner of all connections.

Cycle 4: UMLObject ↔ UMLAssociation (via UMLRole)
  UMLObject may have associations in m_List (via UMLCanvasObject)
  UMLAssociation references UMLObject[2] via UMLRole[2]
  UMLAssociation::resolveRef() resolves objects by ID
  Break: Move association lists out of objects into a central
          AssociationRegistry owned by the document.

Cycle 5: UI ↔ Settings ↔ UMLApp
  UI modifies settings → config changes → UI re-reads settings
  Settings singleton → UMLApp::config() → KConfig
  UMLApp owns SettingsDialog which reads/writes Settings
  Break: Use a channel for settings change notification.
          Settings is a value type, passed explicitly.

Cycle 6: UMLDoc ↔ UMLView ↔ UMLScene
  UMLDoc owns UMLView list
  UMLView references UMLFolder (parent) → UMLDoc
  UMLScene has UMLDoc* and UMLView*
  UMLScene calls doc->removeView(), doc->addAssociation()
  Break: Scene stores a weak document reference.
          Use events for cross-document communication.
```

### 4.2 Cycle Breaking Strategy

```
Target architecture: All cycles broken by introducing a "context" or "registry"

┌─────────────────────────────────────────────┐
│              Document Context               │
│  ┌──────────┐ ┌──────────┐ ┌────────────┐  │
│  │ModelRepo │ │SceneRepo │ │AssocRepo   │  │
│  │(owns all │ │(owns all │ │(registry)  │  │
│  │ objects) │ │ scenes)  │ │            │  │
│  └──────────┘ └──────────┘ └────────────┘  │
│         ▲           ▲            ▲         │
│         │           │            │         │
│  ┌──────┴───┐ ┌────┴────┐ ┌─────┴──────┐  │
│  │UMLObject │ │UMLScene │ │UMLAssoc    │  │
│  │(no doc   │ │(no doc  │ │(no widget  │  │
│  │ pointer) │ │pointer) │ │ pointer)   │  │
│  └──────────┘ └─────────┘ └────────────┘  │
│                                            │
│  Everything communicates via Context:      │
│  fn do_something(ctx: &Context, obj: &Obj) │
└─────────────────────────────────────────────┘
```

**General principles for cycle elimination:**
1. Objects point to registries, not to each other
2. Use IDs (u64) instead of pointers for cross-references
3. Document is a collection of registries with query methods
4. Events propagate changes, not direct method calls

---

## 5. Recommended Rust Crate Boundaries

### 5.1 Crate Hierarchy

```
umbrello/
├── Cargo.toml              # Root workspace
├── core/                   # ──► NO QT/KDE DEPENDENCY
│   ├── model/              #     Pure UML model types
│   ├── types/              #     Enums: DiagramType, AssociationType, etc.
│   └── ids/                #     ID types, UniqueId generator
├── persistence/            # ──► serde, quick-xml
│   ├── xmi/                #     XMI serializer/deserializer
│   └── export/             #     Image export abstractions
├── application/            # ──► Cross-cutting infrastructure
│   ├── context/            #     AppContext (replaces UMLApp singleton)
│   ├── settings/           #     OptionState replacement
│   └── undo/               #     Undo/redo with Command pattern
├── diagram/                # ──► GUI-neutral diagram operations
│   ├── scene/              #     UMLScene logic (layout, containment)
│   └── layout/             #     GraphViz integration
├── codegen/                # ──► Pure Rust, no GUI
│   ├── core/               #     CodeDocument, TextBlock hierarchy
│   ├── cpp/                #     C++ code generator
│   ├── java/               #     Java code generator
│   ├── python/             #     Python code generator
│   └── .../                #     Other language generators
├── codeimport/             # ──► External parser integration
│   ├── cpp/                #     Wraps cppparser
│   ├── java/               #     Java importer
│   └── python/             #     Python importer
├── widget/                 # ──► GUI (Qt via cxx-qt or egui)
│   ├── core/               #     WidgetBase, UMLWidget base
│   ├── class/              #     ClassifierWidget
│   ├── sequence/           #     MessageWidget, ObjectWidget
│   └── .../                #     Other widget types
├── gui/                    # ──► Qt/KDE bindings, main window
│   ├── app/                #     Application shell (thin)
│   ├── dialogs/            #     Property dialogs
│   ├── menus/              #     Context menus
│   └── clipboard/          #     Copy/paste
├── find/                   #     Search across model + diagram
├── docgen/                 #     DocBook/XHTML generator
├── debug/                  #     Logging, tracing
└── parsers/                # ──► External, no Umbrello dependency
    └── cpp/                #     lib/cppparser/ rewrite in Rust
```

### 5.2 Dependency Rules Between Crates

```
                  ┌──────────────────────────────────────┐
                  │          gui/app (binary)             │
                  │  Thin shell, wires everything         │
                  └─────────┬──────────┬──────────────────┘
                            │          │
          ┌─────────────────▼──┐  ┌────▼──────────────┐
          │      gui/          │  │  application/       │
          │  Qt-dependent      │  │  Widget-system      │
          │  drawing, dialogs  │  │  agnostic           │
          └─────────┬──────────┘  └────────┬────────────┘
                    │                      │
                    ▼                      ▼
          ┌──────────────────────────────────┐
          │       widget/                     │
          │  Diagram widgets, scene logic     │
          │  GUI-drawing capabilities         │
          └──────┬───────────────────────────┘
                 │
                 ▼
   ┌───────────────────────────────────────────┐
   │         diagram/                           │
   │  Scene logic (non-GUI: containment, layout)│
   └──────┬────────────────────────────────────┘
          │
          ▼
   ┌───────────────────────────────────────────────┐
   │               core/                            │
   │  ┌─────────┐ ┌──────────┐ ┌────────────────┐  │
   │  │ model/  │ │ types/   │ │     ids/       │  │
   │  │UMLObj   │ │ enums    │ │  UniqueId      │  │
   │  │UMLAssoc │ │ basic    │ │  ID types      │  │
   │  │UMLClass │ │ types    │ │                │  │
   │  └─────────┘ └──────────┘ └────────────────┘  │
   │   NO external dependencies (except serde)      │
   └────────────────────────────────────────────────┘

  codegen/ ───► core/model/
  codeimport/ ───► core/model/
  persistence/ ───► core/model/
  find/ ───► core/model/, diagram/
  docgen/ ───► persistence/xmi/
  gui/ ───► widget/, codegen/, codeimport/, find/, docgen/
  application/context/ ───► core/, persistence/
  parsers/cpp/ ───► NO DEPENDENCY on Umbrello
```

### 5.3 Interface Traits at Crate Boundaries

```rust
// core/model/src/lib.rs
pub trait UmlObject: std::fmt::Debug + Send + Sync {
    fn id(&self) -> UmlId;
    fn name(&self) -> &str;
    fn object_type(&self) -> ObjectType;
    fn set_name(&mut self, name: String);
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

// persistence/src/xmi.rs
pub trait XmiSerializable {
    fn xmi_tag(&self) -> &'static str;
    fn serialize<W: std::io::Write>(&self, writer: &mut XmlWriter<W>) -> Result<()>;
    fn deserialize<R: std::io::Read>(reader: &mut XmlReader<R>) -> Result<Box<dyn UmlObject>>
    where Self: Sized;
}

// application/src/context.rs
pub trait ModelProvider { fn model(&self) -> &ModelRepository; }
pub trait SceneProvider { fn scene(&self, id: SceneId) -> Option<&SceneData>; }
pub trait Logger { fn log(&self, level: LogLevel, msg: &str); }
pub trait UndoStack { fn push(&self, command: Box<dyn UndoCommand>); }
pub trait SettingsProvider { fn settings(&self) -> &Settings; }

// widget/core/src/lib.rs
pub trait DiagramWidget: std::fmt::Debug {
    fn widget_type(&self) -> WidgetType;
    fn bounding_rect(&self) -> Rect;
    fn set_position(&mut self, pos: Point);
    fn associated_object_id(&self) -> Option<UmlId>;
}
```

---

## 6. Migration Dependencies

### 6.1 Extraction Order

Each phase extracts a crate that can be compiled and tested independently.

```
Phase 1: Foundation (no dependencies on old code)
─────────────────────────────────────────────────────
  core/types/        ──  Enums: ObjectType, DiagramType, AssociationType, etc.
  core/ids/          ──  UmlId, UniqueId generator
  Result: Pure types, replaces basictypes.h, umlobjectlist.h

Phase 2: Model (depends only on Phase 1)
─────────────────────────────────────────────────────
  core/model/        ──  UmlObject, UmlAssociation, UmlClassifier, etc.
  Result: All domain objects, no Qt, no GUI. Can load/save via serde.
         Parallel to C++ UMLObject hierarchy but in Rust.

Phase 3: Persistence (depends on Phase 2)
─────────────────────────────────────────────────────
  persistence/xmi/   ──  XMI serializer/deserializer using serde + quick-xml
  Result: Can read/write XMI files independent of old codebase.
         Used for migration bridge phase.

Phase 4: Application Context (depends on Phase 2+3)
─────────────────────────────────────────────────────
  application/       ──  AppContext, Settings, UndoStack
  Result: Application infrastructure without GUI.

Phase 5: Code Import (depends on Phase 2)
─────────────────────────────────────────────────────
  parsers/cpp/       ──  Rewrite of lib/cppparser/ in Rust
  codeimport/        ──  Importers using Phase 2 model
  Result: Can import code and produce Rust model objects.

Phase 6: Code Generation (depends on Phase 2)
─────────────────────────────────────────────────────
  codegen/           ──  All language code generators
  Result: Can generate code from Rust model objects.

Phase 7: Scene/Diagram Logic (depends on Phase 2+4)
─────────────────────────────────────────────────────
  diagram/           ──  Scene, layout, containment logic
  Result: Diagram operations without a GUI renderer.

Phase 8: Widget System (depends on Phase 7)
─────────────────────────────────────────────────────
  widget/            ──  WidgetBase, UMLWidget, AssociationWidget
  Result: Widget logic independent of GUI framework.

Phase 9: GUI Layer (depends on Phase 8)
─────────────────────────────────────────────────────
  gui/app/           ──  Main window (cxx-qt, egui, or KF6 bindings)
  gui/dialogs/       ──  Property dialogs
  gui/menus/         ──  Context menus
  gui/clipboard/     ──  Copy/paste
  Result: Complete application.

Phase 10: Remaining (depends on Phase 7+9)
─────────────────────────────────────────────────────
  find/              ──  Search across model + diagram
  docgen/            ──  DocBook/XHTML generation
  debug/             ──  Logging, tracing
```

### 6.2 Migration Strategy

```
Old C++ Process                  New Rust Process
─────────────────                ─────────────────
main()                           1. Rust loads C++ .xmi file
  └─ UMLApp                         via Phase 3 persistence
      └─ UMLDoc                  2. In-memory model → Rust model
          └─ UMLFolder           3. Code gen, code import, etc.
              └─ UMLObject          (Phases 5-6) run in Rust
                                 4. GUI (Phase 9) visualizes
                                    Rust model directly
                                 5. XMI round-trip tests
                                    verify equivalence

Bridge Phase:
  - C++ app reads .xmi → exports JSON
  - Rust app reads JSON → builds model
  - Compare in-memory representation
  - Switch to Rust XMI writer when mature

Coexistence Phase:
  - Rust is the source of truth
  - C++ code accesses Rust model via C FFI
  - Gradual replacement: C++ widget → Rust widget
  - Final cutover: Rust main() only
```

### 6.3 What Can Be Extracted Immediately

| Subsystem | Can extract now? | Reason |
|-----------|-----------------|--------|
| `lib/cppparser/` | **Yes** | Standalone, no Umbrello deps |
| `core/types/` | **Yes** | Just enums, no deps |
| `core/ids/` | **Yes** | Just ID generation |
| `codegen/core/` | **Partial** | CodeDocument needs model interfaces |
| `debug/` | **Yes** | Just logging abstraction |
| `persistence/xmi/` | **No** | Needs model types first |

---

## 7. Recommendations for Breaking UMLApp God Object

### 7.1 Current Pattern

```cpp
// C++ — everywhere in codebase:
UMLApp::app()->document()->findUMLObject(...);
UMLApp::app()->currentView()->umlScene()->...;
UMLApp::app()->generator()->...;
UMLApp::app()->activeLanguage();
UMLApp::app()->logDebug("message");
```

### 7.2 Target Pattern (Rust)

```rust
// Service Locator Pattern (controlled, not global)
#[derive(Clone)]
struct AppContext {
    model: Arc<ModelRepository>,
    scenes: Arc<SceneRepository>,
    settings: Arc<RwLock<Settings>>,
    undo_stack: Arc<UndoStack>,
    logger: Arc<dyn Logger>,
    codegen_registry: Arc<CodeGenRegistry>,
    active_language: Cell<ProgrammingLanguage>,
}

// Functions receive only what they need:
fn create_attribute(
    model: &ModelRepository,
    parent: &UmlClassifier,
    name: &str,
) -> Result<UmlAttribute> { ... }

fn render_widget(
    ctx: &RenderContext,  // contains only scene, settings, logger
    widget: &dyn DiagramWidget,
) -> Result<()> { ... }
```

### 7.3 Decomposition of UMLApp Responsibilities

| Responsibility | New Owner | Accessed Via |
|---------------|-----------|-------------|
| Document storage | `ModelRepository` | `context.model()` |
| View management | `SceneRepository` | `context.scenes()` |
| Undo/redo | `UndoStack` | `context.undo()` |
| Code generation | `CodeGenRegistry` | Trait-based dispatch |
| Settings | `Settings` | `context.settings()` |
| Logging | `Logger` trait | `context.logger()` |
| Language selection | `AppState` | `context.active_language()` |
| File I/O | `DocumentService` | `service.open(path)` |
| Clipboard | `ClipboardService` | `service.clipboard()` |
| Finding | `SearchService` | `service.search()` |
| Config persistence | `SettingsStore` trait | `settings_store.load()` |
| Bird view | `SceneRepository` | Part of scene management |

### 7.4 The `Context` Struct — Final Design

```rust
/// The application context replaces UMLApp::app().
/// NOT a singleton — created in main() and threaded through.
/// Each subsystem takes only the sub-context it needs.
#[derive(Clone)]
pub struct AppContext {
    // Internal implementations
    model: Arc<ModelRepository>,
    scenes: Arc<SceneRepository>,
    settings: Arc<RwLock<Settings>>,
    undo: Arc<UndoStack>,
    logger: Arc<dyn Logger>,
    codegen: Arc<CodeGenRegistry>,
    active_language: Cell<ProgrammingLanguage>,
}

impl AppContext {
    pub fn new(
        model: ModelRepository,
        settings: Settings,
        logger: impl Logger + 'static,
    ) -> Self { ... }
    
    /// Factory that wires together all components
    pub fn bootstrap(config: Config) -> Result<Self> { ... }
}

// Fine-grained access traits — implement on &AppContext
impl ModelProvider for AppContext {
    fn model(&self) -> &ModelRepository { &self.model }
}

impl Logger for AppContext {
    fn log(&self, level: LogLevel, msg: &str) {
        self.logger.log(level, msg);
    }
}
```

---

## 8. Avoiding Recreating C++ Coupling in Rust

### 8.1 Anti-patterns to Avoid

```
❌ Global mutable state (static mut, lazy_static)
   → Prefer: Dependency injection through parameter passing

❌ God struct with everything accessible
   → Prefer: Trait-based access, each function gets what it needs

❌ Subsystems reaching across boundaries
   → Prefer: Layer-based access, no back-edges

❌ Reference cycles with Rc/Arc
   → Prefer: Weak references, ID-based lookups

❌ Deep inheritance hierarchy (UMLObject has 30+ subclasses)
   → Prefer: Enum-based discriminated union for model types

❌ Qt signals/slots everywhere
   → Prefer: Channel-based event bus (tokio::sync::broadcast)

❌ Raw pointer casts (asUMLClassifier())
   → Prefer: TryFrom / TryInto for type-safe conversion

❌ Switch-on-type anti-patterns
   → Prefer: Trait dispatch, visitor pattern

❌ God factories (Object_Factory, CodeGenFactory)
   → Prefer: Registry pattern with typed builders
```

### 8.2 Where Rust Helps Naturally

| C++ Problem | Rust Solution |
|-------------|---------------|
| No ownership tracking | Ownership & borrowing enforced by compiler |
| Global singleton state | `&mut` prevents accidental sharing |
| Raw pointer casts | Enum dispatch + `TryFrom` |
| Missing interface contracts | Traits with required methods |
| Qt memory management | `Box`, `Rc`, `Arc` — explicit ownership |
| Null pointers | `Option<T>` |
| Uninitialized state | All fields must be initialized |
| Undefined behavior from cycles | Weak references, arena-based storage |
| Thread safety (manual) | `Send` + `Sync` auto-checked |
| Inconsistent error handling | `Result<T, E>` everywhere |
| Implicit side effects | Pure functions where possible |
| Scattered serialization | Serde derives + visitor |

### 8.3 Model Type Design (Avoiding Inheritance Pain)

```rust
// Instead of 30+ subclasses of UMLObject, use an enum:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UmlObject {
    Actor(UmlActor),
    Class(UmlClassifier),
    Interface(UmlClassifier),
    Enum(UmlEnum),
    Attribute(UmlAttribute),
    Operation(UmlOperation),
    Association(UmlAssociation),
    Package(UmlPackage),
    Entity(UMLEntity),
    // ... etc
}

// Common fields in a separate struct:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmlObjectBase {
    pub id: UmlId,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype: Option<StereotypeRef>,
    pub documentation: String,
    pub tags: Vec<TaggedValue>,
}

// Each variant has its own struct:
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmlClassifier {
    pub base: UmlObjectBase,
    pub is_abstract: bool,
    pub attributes: Vec<UmlAttributeId>,
    pub operations: Vec<UmlOperationId>,
    pub templates: Vec<UmlTemplate>,
    pub parent: Option<UmlPackageId>,
}

// Type-safe conversions:
impl TryFrom<UmlObject> for UmlClassifier {
    type Error = TypeError;
    fn try_from(obj: UmlObject) -> Result<Self> {
        match obj {
            UmlObject::Class(c) | UmlObject::Interface(c) => Ok(c),
            _ => Err(TypeError::new("not a classifier")),
        }
    }
}
```

### 8.4 ID-Based References (Breaking Cycles)

```rust
// Strongly-typed IDs instead of raw pointers:
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UmlId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SceneId(u64);

// Repository pattern — central ownership:
pub struct ModelRepository {
    objects: Arena<UmlObject>,
    associations: Vec<UmlAssociation>,
    // A → B references use IDs, not handles:
    // attribute.owner() returns UmlId, not &UmlObject
}

impl ModelRepository {
    pub fn get(&self, id: UmlId) -> Option<&UmlObject> { ... }
    pub fn get_mut(&mut self, id: UmlId) -> Option<&mut UmlObject> { ... }
    
    // Associations stored outside the objects:
    pub fn associations_for(&self, id: UmlId) -> Vec<&UmlAssociation> { ... }
}
```

### 8.5 Event Bus for Model-View Communication

```rust
// Replace Qt signals/slots with a typed event bus:
#[derive(Clone, Debug)]
pub enum ModelEvent {
    ObjectCreated(UmlId),
    ObjectModified(UmlId),
    ObjectRemoved(UmlId),
    AssociationCreated(UmlId),
    AssociationRemoved(UmlId),
    DiagramChanged(SceneId),
}

#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<ModelEvent>,
}

impl EventBus {
    pub fn new() -> (Self, broadcast::Receiver<ModelEvent>) { ... }
    pub fn publish(&self, event: ModelEvent) { ... }
}

// Scene subscribes:
impl UmlScene {
    pub fn new(bus: broadcast::Receiver<ModelEvent>) -> Self {
        // Spawn task: listen for events, update widget state
    }
}
```

---

## 9. Summary

| Metric | C++ Value | Rust Target |
|--------|-----------|-------------|
| Files referencing app singleton | 175 | 0 |
| Cyclic dependencies | 6 | 0 |
| Inheritance depth (max) | 5 (UMLObj→UMLCanvas→UMLPkg→UMLFolder) | 1 (composition) |
| Switch-on-type factories | 3 (Obj, Widget, CodeGen) | 0 (registry) |
| Qt signal connections | ~200 | 0 (channel-based) |
| Classes with XMI serialization | 50+ | serde on all model types |
| Settings singleton accesses | ~100+ | Explicit parameter passing |
| Header include coupling | Tight (flat includes) | Crate-level `pub use` |

### Key Principles for the Rust Rewrite

1. **No singleton. Ever.** Pass context explicitly or use DI.
2. **Model is pure data.** No GUI, no I/O, no QObject.
3. **IDs over pointers.** Break cycles, enable serialization.
4. **Traits at boundaries.** Each crate knows only what it needs.
5. **Bottom-up extraction.** Start with core/types, end with GUI.
6. **Serde for persistence.** One derive, not 50+ saveToXMI.
7. **Registry not switch.** Register implementations, don't switch on types.
8. **Channel not signal.** Use typed event buses, not string-based signals.
9. **Repository not owning-tree.** Objects stored in arenas, looked up by ID.
10. **Value types for settings.** Not global mutable state.

---

*Generated from analysis of Umbrello C++ codebase. For questions about specific
dependencies, see the subsystem files referenced in Section 1.*
