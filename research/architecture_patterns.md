# Architecture Pattern Evaluation for Umbrello-RS

> **Date:** 2026-06-23  
> **Scope:** Evaluate 12 architecture/design patterns against the Umbrello C++ codebase and recommend which to adopt, adapt, or avoid in the Rust rewrite.  
> **Based on:** Analysis of 79 UML model files, 94 widget files, 203 code-generator files, the diagram engine (68 files), undo system (35+ files), and cross-cutting subsystems.

---

## Table of Contents

1. [Evaluated Patterns](#evaluated-patterns)
   - [1. Entity-Component-System (ECS)](#1-entity-component-system-ecs)
   - [2. Document Model Architecture](#2-document-model-architecture)
   - [3. Domain-Driven Design (DDD)](#3-domain-driven-design-ddd)
   - [4. Plugin Architecture](#4-plugin-architecture)
   - [5. Event Sourcing](#5-event-sourcing)
   - [6. Command Pattern](#6-command-pattern)
   - [7. Observer Alternatives](#7-observer-alternatives)
   - [8. Generational Arena](#8-generational-arena)
   - [9. Builder Pattern](#9-builder-pattern)
   - [10. Visitor Pattern](#10-visitor-pattern)
   - [11. Strategy Pattern](#11-strategy-pattern)
   - [12. Adapter Pattern](#12-adapter-pattern)
2. [Overall Architecture Recommendation](#overall-architecture-recommendation)
3. [Pattern Interaction Diagram](#pattern-interaction-diagram)
4. [Anti-Patterns to Avoid](#anti-patterns-to-avoid)
5. [Crate Structure Mapping](#crate-structure-mapping)

---

## Evaluated Patterns

### 1. Entity-Component-System (ECS)

**Description in Rust terms:**
Model each UML element as an **Entity** (an ID/handle) with a dynamic set of **Components** attached. Systems iterate entities by component signature to perform queries, validation, or rendering. Crates: `bevy_ecs`, `hecs`, `specs`, `shipyard`.

```rust
// Using hecs
use hecs::*;

// Components
struct Name(String);
struct Stereotype(Option<String>);
struct Attributes(Vec<AttributeData>);
struct Operations(Vec<OperationData>);
struct Position { x: f32, y: f32 };

// Creating a class entity
let uml_class = world.spawn((
    Name("Customer".into()),
    Stereotype(Some("Entity".into())),
    Attributes(vec![]),
    Operations(vec![]),
));

// System: validate all entities with Name + Stereotype
fn validate_stereotypes(query: &Query<(&Name, &Stereotype)>) {
    for (name, stereo) in query.iter() {
        if stereo.0.is_some() && !KNOWN_STEREOTYPES.contains(stereo.0.as_deref().unwrap()) {
            warn!("Unknown stereotype on {}", name.0);
        }
    }
}
```

**Current C++ usage:** Not used. The C++ codebase uses deep inheritance (`UMLObject` → `UMLCanvasObject` → `UMLPackage` → `UMLClassifier` → ...). ECS would replace this entirely.

**Pros for Umbrello-RS:**
- Maximum flexibility: entities can have arbitrary component combinations
- Data-oriented iteration is cache-friendly
- Adding new UML element types doesn't require class hierarchy changes
- Well-suited for schema-less extensions (tagged values, stereotypes)

**Cons for Umbrello-RS:**
- **Overkill for a fixed-domain model.** UML has ~28 concrete element types with well-defined, stable properties. Inheritance maps naturally.
- **Harder to enforce UML constraints.** An `Actor` should never have `Attributes`; a `Class` always needs a `Name` and can have `Operations`. ECS makes such invariants runtime-checked rather than compile-time enforced.
- **No natural place for per-type methods.** Where does `UMLClassifier::findOperation(QString)` live? In a system? A trait? This scatters behavior.
- **Query overhead** for what is fundamentally hierarchical navigation (package → classes → attributes).
- **Ecosystem immaturity** — `bevy_ecs` is tied to Bevy game engine; `hecs`/`specs` lack the maturity guarantees needed for a document-oriented desktop application.

**Recommendation: AVOID** for primary domain model. The UML metamodel is a textbook case for algebraic data types (enums + structs), not ECS. ECS excels when entities are heterogeneous and composition is unpredictable — the opposite of UML's fixed metamodel.

> **Consider ECS only for** the **diagram layer**: widget entities can dynamically compose visual components (`Renderable`, `Selectable`, `Draggable`, `Resizable`) — a genuine cross-cutting concern where ECS outperforms widget class hierarchies.

---

### 2. Document Model Architecture

**Description in Rust terms:**
A central `Document` struct (analogous to `UMLDoc`) owns the model tree, stereotype registry, and view list. Views (diagram windows, tree views, property panels) hold a reference to the document and subscribe to change notifications. Mutation goes through the document, which emits events.

```rust
pub struct Document {
    // Single-ownership tree: the document roots the entire model
    root_folder: Folder,
    stereotypes: StereotypeRegistry,
    // Observers are push-based
    subscribers: Vec<Box<dyn Fn(&DocumentEvent)>>,
    // Generational arena for O(1) lookup by ID
    objects: Arena<UmlObject>,
    undo_stack: UndoStack,
}

impl Document {
    pub fn add_class(&mut self, name: &str, parent_id: ObjectId) -> ObjectId {
        let id = self.objects.insert(UmlObject::Class(name.into(), vec![]));
        self.emit(DocumentEvent::ObjectCreated(id));
        id
    }

    pub fn emit(&self, event: &DocumentEvent) {
        for sub in &self.subscribers {
            sub(event);
        }
    }
}
```

**Current C++ usage:** `UMLDoc` (umbrello/umldoc.h) is the central document. The C++ version has 175+ files calling `UMLApp::app()` to reach it — a god-object anti-pattern. `UMLDoc` itself owns folders, stereotypes, views, and the undo stack.

**Pros for Umbrello-RS:**
- Natural fit for a document-oriented UML modeling tool
- Clear ownership hierarchy (document → folders → classifiers → attributes)
- Single source of truth for the model
- Makes save/load straightforward (serialize the document tree)

**Cons for Umbrello-RS:**
- Risk of creating another god object if everything routes through Document
- C++ `UMLApp` demonstrates the danger: global `app()` accessor, 175+ dependents
- Without disciplined interface segregation, `Document` grows unbounded

**Recommendation: ADAPT** — use a Document model **but** with strict interface segregation:

```
Document (owns model)
  ├── ModelApi (read-only queries)
  ├── MutationApi (write operations with event emission)
  ├── QueryApi (findByID, findByType, full-text search)
  └── UndoManager (command stack)
```

Each subsystem receives only the API it needs:
- Diagram renderer gets `&ModelApi` (read-only)
- Property editor gets `&MutationApi` + subscribes to events
- Code generator gets `&ModelApi`
- File I/O gets `&Document` (full access, but isolated in save/load pipeline)

> **Key insight:** In C++, every subsystem called `UMLApp::app()->getDoc()`. In Rust, inject only the API trait each subsystem needs. This prevents the document from becoming a god object.

---

### 3. Domain-Driven Design (DDD)

**Description in Rust terms:**
Apply tactical DDD patterns selectively. The UML domain vocabulary (class, attribute, association, generalization) maps directly to DDD concepts.

```rust
// Aggregate: UMLPackage owns its children
pub struct Package {
    id: PackageId,
    name: String,
    // Children are entities within the aggregate boundary
    owned_elements: Vec<ElementId>,
}

// Entity: has identity and lifecycle
pub struct Class {
    id: ClassId,  // Identity: UML ID (UUID)
    name: String,
    attributes: Vec<Attribute>,        // Value objects owned by entity
    operations: Vec<Operation>,
    // References to other aggregates via ID only
    generalizations: Vec<ClassId>,
    associations: Vec<AssociationId>,
}

// Value object: immutable, equality by value
#[derive(Clone, PartialEq)]
pub struct Multiplicity {
    lower: Bound,  // Bound::Exact(1), Bound::Unbounded, etc.
    upper: Bound,
}

#[derive(Clone, PartialEq)]
pub struct Position {
    x: f64,
    y: f64,
}

// Domain event
#[derive(Clone)]
pub enum DomainEvent {
    ObjectCreated { id: ObjectId, type_: ObjectType },
    AttributeAdded { class_id: ObjectId, attr: Attribute },
    AssociationCreated { from: ObjectId, to: ObjectId, type_: AssocType },
    ObjectRemoved { id: ObjectId },
}
```

**Current C++ usage:** Not explicitly DDD, but many DDD concepts appear naturally:
- **Aggregates:** `UMLPackage` (owns `m_objects`), `UMLFolder` (owns `m_diagrams`)
- **Entities:** `UMLObject` (has `m_nId` as identifier)
- **Value objects:** Implicit — `UMLRole` has `m_Multiplicity` (QString, not a typed value object)
- **Repositories:** Not formalized; lookup is ad-hoc via `UMLDoc::findObjectById()`
- **Domain events:** Not used; Qt signals replace them

**Pros for Umbrello-RS:**
- Natural vocabulary: the UML specification is already a domain model
- Aggregate boundaries clarify ownership (what gets saved/deleted together)
- ID-based references prevent dangling pointer issues
- Domain events provide clean integration points (e.g., "when class is created, add widget to diagram")
- Clear separation between entities (identity matters) and value objects (equality matters)

**Cons for Umbrello-RS:**
- Can over-complicate simple operations (e.g., renaming a class involves event emission, repository lookup)
- Aggregate boundaries in UML are not completely clean: attributes are owned by classifier, but associations reference two classifiers by ID — is the association a separate aggregate?
- Purely DDD repositories add indirection for what is often a flat lookup

**Recommendation: ADAPT selectively** — use DDD concepts where they clarify, avoid where they add ceremony.

| DDD Concept | Apply? | Rationale |
|---|---|---|
| Entities (ID-based) | **YES** | Every UML object has a unique ID; this is core |
| Value objects | **YES** | `Position`, `Multiplicity`, `Color`, `Stereotype` — immutability by value |
| Aggregates | **YES, loosely** | `Package`/`Folder` as ownership boundary; `Association` as separate aggregate referencing classifiers by ID |
| Domain events | **YES** | Replace Qt signals for model → view/subsystem notifications |
| Repositories | **NO** | Prefer direct `Document` queries; add repository traits only if needed for testability |
| Domain services | **CAUTIOUSLY** | Use for cross-aggregate operations (e.g., "merge classes", "apply design pattern") |
| Factories | **YES** | `ObjectFactory` becomes typed constructors or `Builder` (see below) |

---

### 4. Plugin Architecture

**Description in Rust terms:**
A plugin trait + registry pattern for swappable code generators, importers, and export formats.

```rust
// Compile-time plugin (Cargo feature gates)
#[cfg(feature = "codegen-cpp")]
mod cpp_generator;

#[cfg(feature = "codegen-python")]
mod python_generator;

// Trait-based dispatch
pub trait CodeGenerator: Send + Sync {
    fn language_name(&self) -> &'static str;
    fn file_extension(&self) -> &'static str;
    fn generate(&self, model: &ModelApi, writer: &mut dyn Write) -> Result<(), GenError>;
}

// Registry populated at startup
pub struct PluginRegistry {
    generators: HashMap<&'static str, Box<dyn CodeGenerator>>,
    importers: HashMap<&'static str, Box<dyn CodeImporter>>,
}

// Dynamic loading via WASM (optional, for community contributions)
pub trait WasmPlugin: CodeGenerator {
    fn instantiate(wasm_bytes: &[u8]) -> Result<Box<dyn CodeGenerator>>;
}
```

**Current C++ usage:** `CodeGenFactory::getObject(language)` is a `switch` statement over ~22 languages. No actual plugin system — adding a language means modifying the factory. Code importers likewise use a factory `Import_Utils::importFile()` with `if/else if` chain.

**Pros for Umbrello-RS:**
- **Compile-time plugins:** Cargo feature flags (`codegen-cpp`, `codegen-python`, etc.) let users build minimal binaries
- **Trait-based dispatch** is idiomatic Rust, no inheritance pitfalls
- **WASM plugins** for community contributions without compromising sandboxing
- **Separation** between core and contributed generators

**Cons for Umbrello-RS:**
- Dynamic loading (libloading) is unsafe and platform-dependent
- WASM runtime adds ~10MB to binary size (wasmtime)
- Trait objects have `dyn` dispatch overhead (though negligible for codegen)
- Language-specific codegen settings need careful API design

**Recommendation: ADAPT — compile-time hybrid approach.**

```
Layer 1: Trait definition (core)
  └── trait CodeGenerator { fn generate(...) }
  └── trait CodeImporter { fn import(...) }

Layer 2: Default implementations (optional features)
  └── feature = "codegen-cpp" → CppGenerator impl CodeGenerator
  └── feature = "codegen-py"  → PythonGenerator impl CodeGenerator
  └── ...

Layer 3: WASM plugin host (optional feature)
  └── feature = "wasm-plugins" → loads .wasm files from plugin dir
  └── Each WASM plugin exports a CodeGenerator-compatible interface
```

> **Recommendation:** Start with compile-time + feature gates. Add WASM plugin support only if the community demands it and the complexity budget allows.

**Code sketch — generator with feature gates:**

```rust
// src/codegen/registry.rs
pub struct GeneratorRegistry {
    generators: Vec<Box<dyn CodeGenerator>>,
}

impl GeneratorRegistry {
    pub fn builtin() -> Self {
        let mut reg = Self { generators: vec![] };

        #[cfg(feature = "codegen-cpp")]
        reg.register(cpp::CppGenerator);

        #[cfg(feature = "codegen-python")]
        reg.register(python::PythonGenerator);

        #[cfg(feature = "codegen-rust")]
        reg.register(rust::RustGenerator);

        reg
    }

    pub fn register(&mut self, gen: impl CodeGenerator + 'static) {
        self.generators.push(Box::new(gen));
    }
}
```

---

### 5. Event Sourcing

**Description in Rust terms:**
Every mutation to the model is recorded as an append-only event log. The current state is derived by replaying events. Undo = replay all events except the last. Full history = the entire event log.

```rust
#[derive(Serialize, Deserialize)]
pub enum ModelEvent {
    V1 {
        timestamp: i64,
        event: ModelEventV1,
    },
}

pub struct EventStore {
    events: Vec<ModelEvent>,
    snapshot_interval: usize,
}

impl EventStore {
    pub fn append(&mut self, event: ModelEvent) { self.events.push(event); }

    // Rebuild state from events
    pub fn replay(&self) -> Document {
        let mut doc = Document::empty();
        for event in &self.events {
            doc.apply(event);
        }
        doc
    }

    // Snapshot for performance (replay from snapshot, not beginning of time)
    pub fn snapshot(&self) -> Document { ... }

    pub fn undo(&mut self) -> Option<Document> {
        self.events.pop();
        Some(self.replay())
    }
}
```

**Current C++ usage:** Not used. Undo is via `QUndoCommand` (Command pattern). There is no audit trail.

**Pros for Umbrello-RS:**
- Perfect undo/redo (no command composition issues)
- Built-in audit trail (who changed what, when)
- Enables collaborative editing (send events over network)
- Snapshot + replay enables time-travel debugging
- Serialization becomes trivial: dump the event log

**Cons for Umbrello-RS:**
- **Massive complexity for a single-user desktop application**
- Storage overhead: saving a project serializes millions of events vs. a single snapshot
- Views must replay events to reconstruct state — startup time increases with project history
- Event schema migration is hard (events are immutable once written)
- UML operations (e.g., "delete class and all its widgets") become composite events requiring careful design
- Undo is not simply "pop and replay" — some operations have side effects (file I/O, clipboard) that can't be undone

**Recommendation: AVOID** as primary persistence strategy. Consider **event-sourced undo stack** only:

```rust
/// Lightweight event-sourced undo for a single editing session:
/// keep a log of mutations since the last save, persist on save as snapshot.
pub struct SessionHistory {
    since_save: Vec<Mutation>,
    saved_snapshot: Document,
}

impl SessionHistory {
    pub fn current_state(&self) -> Document {
        let mut doc = self.saved_snapshot.clone();
        for mutation in &self.since_save {
            doc.apply(mutation);
        }
        doc
    }

    pub fn undo(&mut self) -> Document {
        self.since_save.pop();
        self.current_state()
    }
}
```

> This gives "undo to last save" for free without the full Event Sourcing overhead. True persistent Event Sourcing is more complexity than this application warrants.

---

### 6. Command Pattern

**Description in Rust terms:**
Encapsulate each mutation as a `Command` trait object. Commands are pushed onto a stack for undo/redo. Each command knows how to execute, undo, and describe itself.

```rust
pub trait Command: Send {
    fn execute(&mut self, doc: &mut Document) -> Result<(), CmdError>;
    fn undo(&mut self, doc: &mut Document) -> Result<(), CmdError>;
    fn description(&self) -> String;
    fn merge(&mut self, other: &dyn Command) -> Option<Box<dyn Command>>;
}

pub struct UndoStack {
    done: Vec<Box<dyn Command>>,
    undone: Vec<Box<dyn Command>>,
    // Generational counter for invalidation
    generation: u64,
    max_depth: usize,
}

impl UndoStack {
    pub fn execute(&mut self, mut cmd: Box<dyn Command>, doc: &mut Document) -> Result<(), CmdError> {
        cmd.execute(doc)?;
        self.done.push(cmd);
        self.undone.clear();  // new action invalidates redo
        self.generation += 1;
        Ok(())
    }

    pub fn undo(&mut self, doc: &mut Document) -> Result<(), CmdError> {
        if let Some(mut cmd) = self.done.pop() {
            cmd.undo(doc)?;
            self.undone.push(cmd);
        }
        Ok(())
    }
}

// Concrete command
pub struct CmdCreateClass {
    name: String,
    parent_id: ObjectId,
    created_id: Option<ObjectId>,  // populated on execute
}

impl Command for CmdCreateClass {
    fn execute(&mut self, doc: &mut Document) -> Result<(), CmdError> {
        let id = doc.add_class(&self.name, self.parent_id);
        self.created_id = Some(id);
        Ok(())
    }

    fn undo(&mut self, doc: &mut Document) -> Result<(), CmdError> {
        if let Some(id) = self.created_id.take() {
            doc.remove_object(id)?;
        }
        Ok(())
    }

    fn description(&self) -> String { format!("Create class {}", self.name) }

    fn merge(&mut self, other: &dyn Command) -> Option<Box<dyn Command>> {
        // e.g., consecutive rename commands merge into one
        None
    }
}
```

**Current C++ usage:** 30+ command classes in `umbrello/cmds/`, inheriting `QUndoCommand`. The C++ commands are split into `generic/` (model operations) and `widget/` (diagram widget operations). The C++ `CmdBaseObjectCommand` wraps an `UMLObject` pointer; the widget commands wrap `WidgetBase`.

**Pros for Umbrello-RS:**
- Well-understood, straightforward implementation in Rust
- `Box<dyn Command>` + generational stack provides clean undo/redo
- Composition: compound command can group atomic commands (e.g., "paste" = create objects + create widgets + create associations)
- Each command is testable in isolation
- Merging consecutive same-type commands (e.g., rename, move) reduces stack clutter

**Cons for Umbrello-RS:**
- Must ensure commands capture all state needed for undo — Rust's move semantics help here (command takes ownership of old values)
- Commands that cross subsystems (e.g., "create class + add to diagram") need coordination between model and widget command stacks
- `dyn Command` has a vtable call overhead per execute/undo (negligible for user-initiated operations)

**Recommendation: ADOPT** with these design decisions:

1. **Two stacks** — one for model commands, one for widget commands — to keep concerns separated. The diagram undo commands operate on widget state, while model undo operates on the document.

2. **Make `Command` a trait** with `execute`, `undo`, `description`, and optional `merge`.

3. **Generational invalidation** — a counter on the stack so stale command references can be detected.

4. **Compound command** as a `Vec<Box<dyn Command>>` that implements `Command` by delegating to children.

5. **Store old values by value** (clone/copy) rather than by reference — Rust's borrow checker enforces this naturally.

```rust
// Compound command example
pub struct CompoundCommand {
    commands: Vec<Box<dyn Command>>,
    description: String,
}

impl Command for CompoundCommand {
    fn execute(&mut self, doc: &mut Document) -> Result<(), CmdError> {
        for cmd in &mut self.commands {
            cmd.execute(doc)?;
        }
        Ok(())
    }

    fn undo(&mut self, doc: &mut Document) -> Result<(), CmdError> {
        // Undo in reverse order
        for cmd in self.commands.iter_mut().rev() {
            cmd.undo(doc)?;
        }
        Ok(())
    }
    // ...
}
```

---

### 7. Observer Alternatives

**Description in Rust terms:**
Replace Qt's signal/slot mechanism with idiomatic Rust notification patterns. Several options exist:

```rust
// Option A: Event bus (cross-cutting concerns)
use event_bus::{EventBus, EventHandler};

#[derive(Clone)]
pub enum ModelEvent {
    ObjectCreated(ObjectId),
    ObjectRemoved(ObjectId),
    AttributeChanged(ObjectId, String),  // object_id, attribute_name
}

// Views subscribe:
struct DiagramView {
    bus: Subscriber<ModelEvent>,
}

impl EventHandler<ModelEvent> for DiagramView {
    fn on_event(&mut self, event: &ModelEvent) {
        match event {
            ModelEvent::ObjectCreated(id) => self.add_widget_for(*id),
            ModelEvent::ObjectRemoved(id) => self.remove_widget_for(*id),
            _ => {}
        }
    }
}

// Option B: Direct callback (tight coupling, fine for 1:1)
pub struct Document {
    on_class_added: Vec<Box<dyn FnMut(ObjectId)>>,
}

impl Document {
    pub fn add_on_class_added(&mut self, cb: Box<dyn FnMut(ObjectId)>) { ... }

    pub fn add_class(&mut self, name: &str) -> ObjectId {
        let id = ...;
        for cb in &mut self.on_class_added {
            cb(id);
        }
        id
    }
}

// Option C: Immediate direct calls (simplest, least flexible)
pub struct Document {
    diagram_views: Vec<Weak<RefCell<DiagramView>>>,
}

impl Document {
    pub fn add_class(&mut self, name: &str) -> ObjectId {
        let id = ...;
        for view in &self.diagram_views {
            if let Some(view) = view.upgrade() {
                view.borrow_mut().on_object_created(id);
            }
        }
        id
    }
}
```

**Current C++ usage:** Qt signals/slots everywhere:
- `UMLDoc` emits `sigObjectCreated(UMLObject*)` → scene creates widget
- `UMLScene` emits `sigAssociationRemoved()` → toolbar state reacts
- Option state changes via `KConfig::notify()` / signals

**Pros for Umbrello-RS:**
- **Event bus** (flume, tokio::sync::broadcast) provides decoupled pub/sub
- **Direct callbacks** are simple and composable
- No macro-based signal/slot system (Qt MOC dependency eliminated)
- Rust's ownership model forces us to be explicit about observer lifetimes (preventing dangling observers)

**Cons for Umbrello-RS:**
- No single dominant Rust pattern = team must agree on approach
- Event bus can make data flow hard to trace (the "where does this event come from?" problem)
- Callbacks require careful lifetime management (use `Weak` references to prevent cycles)
- No built-in queued connections (Qt's `Qt::QueuedConnection` for cross-thread)

**Recommendation: HYBRID APPROACH**

| Communication Pattern | Use For | Mechanism |
|---|---|---|
| **Tight 1:1** (model → dedicated view) | Widget knows its specific model object | Direct method call with `&mut` access |
| **Broadcast events** (model → multiple views) | Changes that affect multiple widgets/views | `flume` broadcast channel or custom `EventBus` |
| **Command completion** | After command executes, notify interested parties | Return value from command; observers poll `[cmd]` |
| **Cross-cutting** (settings changes, toolbar changes) | Global configuration updates | `tokio::sync::watch` channel (one value, many watchers) |
| **Undo stack changes** | Enable/disable undo/redo buttons | Direct callback from `UndoStack::execute/undo` |

```rust
/// Lightweight typed event bus
pub struct EventBus<E> {
    subscribers: Vec<Box<dyn FnMut(&E)>>,
}

impl<E: Clone> EventBus<E> {
    pub fn new() -> Self { Self { subscribers: vec![] } }

    pub fn subscribe(&mut self, sub: Box<dyn FnMut(&E)>) {
        self.subscribers.push(sub);
    }

    pub fn emit(&mut self, event: &E) {
        for sub in &mut self.subscribers {
            sub(event);
        }
    }
}
```

> **Key principle:** Prefer direct calls for known 1:1 relationships; use event bus only for 1:N broadcast or when the producer and consumer are in different subsystems that should not directly depend on each other.

---

### 8. Generational Arena

**Description in Rust terms:**
Store all UML objects in a generational arena (slotmap) instead of directly or with `Box<RefCell<...>>` pointers. Objects are referenced by `Key<UmlObject>` (a generational index) rather than by pointer.

```rust
use slotmap::{SlotMap, Key};

// Define a new key type for UML objects
new_key_type! { pub struct ObjectKey; }

// The arena owns all objects
pub struct ObjectStore {
    objects: SlotMap<ObjectKey, UmlObject>,
}

// Each UmlObject variant stores keys to other objects
pub enum UmlObject {
    Class {
        name: String,
        attributes: Vec<AttributeKey>,
        operations: Vec<OperationKey>,
        generalizations: Vec<ObjectKey>,
    },
    Association {
        name: String,
        from: ObjectKey,  // references another object
        to: ObjectKey,
        type_: AssocType,
    },
    // ...
}

impl ObjectStore {
    pub fn insert(&mut self, obj: UmlObject) -> ObjectKey {
        self.objects.insert(obj)
    }

    pub fn get(&self, key: ObjectKey) -> Option<&UmlObject> {
        self.objects.get(key)  // returns None if key was invalidated
    }

    pub fn get_mut(&mut self, key: ObjectKey) -> Option<&mut UmlObject> {
        self.objects.get_mut(key)
    }

    pub fn remove(&mut self, key: ObjectKey) -> Option<UmlObject> {
        self.objects.remove(key)  // subsequent get(key) returns None
    }
}
```

**Current C++ usage:** Raw pointers everywhere:
- `UMLObject*` passed between subsystems
- `QPointer` for guarded pointers (Qt's weak pointer)
- No systematic protection against dangling pointers
- `UMLDoc::findObjectById()` returns `UMLObject*` that may have been deleted

**Pros for Umbrello-RS:**
- **No use-after-free:** accessing a removed key returns `None` instead of UB
- **Stable iteration:** arena allows inserting while iterating (no iterator invalidation)
- **Compact storage:** objects are stored contiguously in memory (cache-friendly)
- **Interchangeable references:** keys are Copy, can be freely passed around
- **Makes ownership clear:** arena owns objects; keys are non-owning references
- **Easy serialization:** keys are integers; serialize as `<object_id>` in XMI

**Cons for Umbrello-RS:**
- One level of indirection on every object access (arena lookup)
- Cannot hold `&mut` references into arena while also mutating other entries (borrow checker)
- Removing an object leaves "dangling keys" in other objects (must handle gracefully)
- Not idiomatic for hierarchical trees (parent-child relationships)

**Recommendation: ADOPT** as the primary storage mechanism for UML model objects. Specifically, use `slotmap::SlotMap` or `generational_arena::GenerationalArena`.

```rust
// Handling dangling references gracefully
impl ObjectStore {
    /// Clean all references to a removed key
    pub fn remove_with_fixup(&mut self, key: ObjectKey) {
        // Remove the object and fix up references in other objects
        if let Some(obj) = self.objects.remove(key) {
            match obj {
                UmlObject::Association { from, to, .. } => {
                    // Notify from/to objects that association was removed
                }
                UmlObject::Package { children, .. } => {
                    // Recursively remove children or reparent them
                }
                _ => {}
            }
        }
    }
}
```

> **Implementation note:** Use `SlotMap` for general-purpose storage. Use `HopSlotMap` if you need stable keys but also fast iteration. The key type should be `ObjectKey` — new-type around `DefaultKey` for type safety.

**Relationship to diagram widgets:** Diagram widgets also benefit from arena storage:

```rust
new_key_type! { pub struct WidgetKey; }

pub struct WidgetStore {
    widgets: SlotMap<WidgetKey, Widget>,
}
```

This gives the diagram layer the same safety guarantees as the model layer.

---

### 9. Builder Pattern

**Description in Rust terms:**
Provide a builder API for constructing complex UML model objects with validation at construction time.

```rust
pub struct Class {
    pub id: ObjectId,
    pub name: String,
    pub stereotype: Option<String>,
    pub attributes: Vec<Attribute>,
    pub operations: Vec<Operation>,
    pub visibility: Visibility,
    pub is_abstract: bool,
    pub parent_id: ObjectKey,
}

pub struct ClassBuilder {
    name: String,
    stereotype: Option<String>,
    attributes: Vec<Attribute>,
    operations: Vec<Operation>,
    visibility: Visibility,
    is_abstract: bool,
}

impl ClassBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            stereotype: None,
            attributes: vec![],
            operations: vec![],
            visibility: Visibility::Public,
            is_abstract: false,
        }
    }

    pub fn stereotype(mut self, s: &str) -> Self {
        self.stereotype = Some(s.to_string());
        self
    }

    pub fn add_attr(mut self, attr: Attribute) -> Self {
        self.attributes.push(attr);
        self
    }

    pub fn visibility(mut self, v: Visibility) -> Self {
        self.visibility = v;
        self
    }

    pub fn build(self) -> Result<Class, BuildError> {
        if self.name.is_empty() {
            return Err(BuildError::EmptyName);
        }
        if self.attributes.iter().any(|a| a.name.is_empty()) {
            return Err(BuildError::AttributeWithoutName);
        }
        Ok(Class {
            id: ObjectId::new(),
            name: self.name,
            stereotype: self.stereotype,
            attributes: self.attributes,
            operations: self.operations,
            visibility: self.visibility,
            is_abstract: self.is_abstract,
            parent_id: ObjectKey::default(), // set later by document
        })
    }
}

// Usage
let my_class = ClassBuilder::new("Customer")
    .stereotype("Entity")
    .add_attr(Attribute::new("id", "int"))
    .add_attr(Attribute::new("name", "String"))
    .visibility(Visibility::Public)
    .build()?;
```

**Current C++ usage:** No builder pattern. Objects are constructed via:
- Constructor with many parameters (e.g., `UMLAssociation(type, umlobject, umlobject, ...)`)
- Default constructors + setter calls: `obj = new UMLClass(); obj->setName("Foo"); obj->addAttribute(...)`
- `Object_Factory::createUMLObject(type, name, ...)` — a factory that constructs and registers

**Pros for Umbrello-RS:**
- Self-documenting construction API
- Validation at build time (not at runtime after half the setters have been called)
- Can enforce invariants (e.g., a class must have at least a name)
- Rust's `Result` return forces callers to handle construction failures
- Named parameters (via builder methods) approximate C++'s lack of named arguments

**Cons for Umbrello-RS:**
- Verbose: a builder struct for every complex type
- Adds boilerplate for simple types (use direct struct construction with `..Default::default()`)
- Builders require `Copy`/`Clone` on intermediate types or take ownership (which is fine in Rust)

**Recommendation: ADOPT selectively** — use builders for types with:

- 5+ fields (Class, Operation, Association)
- Complex validation rules (Constraints, Entity with PrimaryKey)
- Multiple optional fields (Stereotype, Documentation, TaggedValues)

For simple types (1–3 fields), prefer direct struct initialization:

```rust
// Simple: no builder needed
let attr = Attribute {
    name: "id".into(),
    type_: TypeRef::new("int"),
    visibility: Visibility::Private,
};
```

---

### 10. Visitor Pattern

**Description in Rust terms:**
Traverse the UML model tree with a visitor that dispatches on object type. Two Rust approaches:

**Approach A: Enum-based (recommended for Umbrello-RS)**

```rust
#[derive(Clone)]
pub enum UmlObject {
    Package(PackageData),
    Class(ClassData),
    Interface(InterfaceData),
    Enum(EnumData),
    Attribute(AttributeData),
    Operation(OperationData),
    Association(AssociationData),
    // ... ~20 variants
}

pub trait UmlVisitor {
    fn visit_package(&mut self, data: &PackageData) -> VisitFlow { VisitFlow::Continue }
    fn visit_class(&mut self, data: &ClassData) -> VisitFlow { VisitFlow::Continue }
    fn visit_interface(&mut self, data: &InterfaceData) -> VisitFlow { VisitFlow::Continue }
    // ... default impls for each variant, defaulting to Continue
}

pub enum VisitFlow {
    Continue,    // visit children
    SkipChildren,
    Stop,        // abort traversal
}

impl UmlObject {
    pub fn accept(&self, visitor: &mut dyn UmlVisitor) -> VisitFlow {
        match self {
            UmlObject::Package(d) => visitor.visit_package(d),
            UmlObject::Class(d) => visitor.visit_class(d),
            UmlObject::Interface(d) => visitor.visit_interface(d),
            // ...
        }
    }
}
```

**Approach B: Trait-based (more idiomatic for small hierarchies)**

```rust
pub trait UmlVisitable {
    fn accept(&self, visitor: &mut dyn UmlVisitor);
}

impl UmlVisitable for ClassData {
    fn accept(&self, visitor: &mut dyn UmlVisitor) {
        visitor.visit_class(self);
        for attr in &self.attributes {
            attr.accept(visitor);
        }
    }
}
```

**Current C++ usage:** **Not strictly a visitor pattern** — each UML object class implements `saveToXMI(QXmlStreamWriter&)` and `loadFromXMI(QDomElement&)` as virtual methods. This is more Template Method than Visitor. The `switch` on `ObjectType` in factories and `UMLWidget::loadFromXMI()` is closer to a visitor dispatch.

**Pros for Umbrello-RS:**
- **Enum-based dispatch** is zero-cost (match compiled to jump table), no vtable overhead
- Single `UmlObject` enum prevents "forgotten case" bugs — the compiler forces exhaustiveness
- XMI serialization, code generation, and validation can all be visitors
- Adding a new visitor doesn't modify model types (open for extension)

**Cons for Umbrello-RS:**
- Enum with 28+ variants is large; match statements become long
- The enum forces single ownership: you can't have a `Vec<Attribute>` and a reference to it simultaneously without careful API design
- Adding a new UML type means adding a variant to the enum (modifies the type definition), whereas a trait-based approach lets new types implement the trait externally

**Recommendation: ADOPT** — use enum-based visitor for serialization and code generation, where exhaustive matching is valuable. Use a trait-based approach for the core model hierarchy if it grows beyond the enum's manageability.

```rust
// XMI Serialization Visitor
pub struct XmiWriter<'w> {
    writer: &'w mut dyn Write,
    version: XmiVersion,
}

impl UmlVisitor for XmiWriter<'_> {
    fn visit_class(&mut self, data: &ClassData) -> VisitFlow {
        writeln!(self.writer, r#"<UML:Class name="{}">"#, data.name);
        for attr in &data.attributes {
            // inline attribute serialization
        }
        writeln!(self.writer, "</UML:Class>");
        VisitFlow::Continue
    }
    // ...
}

// Usage
let visitor = &mut XmiWriter { writer: &mut output, version: XmiVersion::V21 };
root_object.accept(visitor);
```

**Alternative: `enum_dispatch` crate** — generates an enum-based vtable from a trait, combining trait ergonomics with enum dispatch speed:

```rust
#[enum_dispatch]
pub trait Serializable {
    fn serialize(&self, writer: &mut dyn Write) -> Result<(), SerError>;
}

#[enum_dispatch(Serializable)]
pub enum UmlObject {
    Package(PackageData),
    Class(ClassData),
    // ...
}
```

---

### 11. Strategy Pattern

**Description in Rust terms:**
Define a family of algorithms (layout engines, export formats, rendering backends) behind a trait, and inject different implementations at runtime.

```rust
pub trait LayoutEngine: Send + Sync {
    fn name(&self) -> &'static str;
    fn layout(&self, diagram: &DiagramData, objects: &ObjectStore) -> Result<LayoutResult, LayoutError>;
}

/// Graphviz-based layout
pub struct DotLayoutEngine;

impl LayoutEngine for DotLayoutEngine {
    fn layout(&self, diagram: &DiagramData, objects: &ObjectStore) -> Result<LayoutResult, LayoutError> {
        // Generate DOT input, call `dot` binary, parse coordinates back
        Ok(LayoutResult { positions: vec![] })
    }
}

/// Simple grid layout (fallback, no external dependency)
pub struct GridLayoutEngine;

impl LayoutEngine for GridLayoutEngine {
    fn layout(&self, diagram: &DiagramData, objects: &ObjectStore) -> Result<LayoutResult, LayoutError> {
        // Arrange widgets in a grid
        Ok(LayoutResult { positions: vec![] })
    }
}

// Usage
pub struct Diagram {
    layout_engine: Box<dyn LayoutEngine>,
}

impl Diagram {
    pub fn set_layout_engine(&mut self, engine: Box<dyn LayoutEngine>) {
        self.layout_engine = engine;
    }

    pub fn auto_layout(&self) -> Result<(), LayoutError> {
        let result = self.layout_engine.layout(&self.data, &self.objects)?;
        self.apply_positions(result);
        Ok(())
    }
}
```

**Current C++ usage:**
- **Layout:** `LayoutGenerator` hard-codes GraphViz invocation (no strategy pluggability)
- **Export:** `UMLApp::slotFileExport()` uses `diagram_utils::exportPDF()` / `exportImage()` — not strategy-based
- **Code generation:** Switch-based factory (see Plugin Architecture above)
- **Rendering:** Not pluggable (direct QPainter)

**Pros for Umbrello-RS:**
- Clean separation between algorithm and consumer
- Testable in isolation (mock layout engines for diagram tests)
- Users can choose between built-in layout and GraphViz layout
- New export formats (SVG, PNG, PDF) can be added without modifying core

**Cons for Umbrello-RS:**
- `Box<dyn Strategy>` has runtime dispatch overhead
- Stateless strategies are simple; stateful ones need lifetime management
- Strategy selection must be exposed in the UI (settings dialog, etc.)

**Recommendation: ADOPT** for:

| Domain | Strategy Trait | Implementations |
|---|---|---|
| Auto-layout | `LayoutEngine` | `DotLayout`, `GridLayout`, `HierarchicalLayout` |
| Export | `ExportFormat` | `SvgExport`, `PngExport`, `PdfExport`, `DotExport` |
| Rendering | `Renderer` | `CpuRenderer`, `GpuRenderer` (if needed) |
| Diagram style | `StyleStrategy` | `UmlStyle`, `CleanStyle`, `HandDrawnStyle` |

```rust
// Export strategy example
pub trait ExportFormat {
    fn file_extension(&self) -> &'static str;
    fn mime_type(&self) -> &'static str;
    fn export(&self, scene: &SceneData, writer: &mut dyn Write) -> Result<(), ExportError>;
}

pub struct SvgExport;
impl ExportFormat for SvgExport { /* ... */ }

pub struct PngExport { resolution: u32 }
impl ExportFormat for PngExport { /* ... */ }
```

---

### 12. Adapter Pattern

**Description in Rust terms:**
Translate between external representations (XMI 1.2, XMI 2.1, ArgoUML `.zargo`, Rational Rose `.mdl`) and the internal model. Each adapter implements a common trait.

```rust
pub trait XmiImport {
    fn can_parse(&self, header: &str) -> bool;
    fn import(&self, reader: &mut dyn BufRead) -> Result<Document, ImportError>;
}

pub trait XmiExport {
    fn export(&self, doc: &Document, writer: &mut dyn Write) -> Result<(), ExportError>;
    fn version(&self) -> &'static str;
}

// Adapter for XMI 1.2
pub struct Xmi12Import;
impl XmiImport for Xmi12Import {
    fn can_parse(&self, header: &str) -> bool {
        header.contains("xmi.version=\"1.2\"") || header.contains("XMI.version=1.2")
    }

    fn import(&self, reader: &mut dyn BufRead) -> Result<Document, ImportError> {
        // XMI 1.2 specific parsing logic
        Ok(Document::empty())
    }
}

// Adapter for XMI 2.1
pub struct Xmi21Import;
impl XmiImport for Xmi21Import {
    fn can_parse(&self, header: &str) -> bool {
        header.contains("xmi:version=\"2.1\"") || header.contains("xmi.version=2.1")
    }

    fn import(&self, reader: &mut dyn BufRead) -> Result<Document, ImportError> {
        // XMI 2.1 specific parsing logic
        Ok(Document::empty())
    }
}

// Adapter resolution
pub struct ImportResolver {
    adapters: Vec<Box<dyn XmiImport>>,
}

impl ImportResolver {
    pub fn new() -> Self {
        Self {
            adapters: vec![
                Box::new(Xmi21Import),
                Box::new(Xmi12Import),
                Box::new(ArgoUmlImport),
                Box::new(RationalRoseImport),
            ],
        }
    }

    pub fn import(&self, data: &str) -> Result<Document, ImportError> {
        // Peek header to find the right adapter
        let header = data.lines().next().unwrap_or("");
        for adapter in &self.adapters {
            if adapter.can_parse(header) {
                return adapter.import(&mut data.as_bytes());
            }
        }
        Err(ImportError::UnrecognizedFormat)
    }
}
```

**Current C++ usage:** Ad-hoc:
- `UMLDoc::loadFromXMI()` handles both XMI 1.2 and 2.1 via `if (uml2)` branching (not separate adapters)
- `Import_Rose` for Rational Rose `.mdl` files
- `Import_Argo` for ArgoUML `.zargo` files
- The branching within `loadFromXMI` results in a 1000+ line function

**Pros for Umbrello-RS:**
- Each format adapter is isolated — easier to test, maintain, and evolve independently
- Adding a new import/export format doesn't modify existing code (Open/Closed Principle)
- Peek-and-dispatch pattern enables auto-detection of file format
- Common trait enables format-agnostic file opening (just resolve the adapter)

**Cons for Umbrello-RS:**
- Trait dispatch indirection per adapter (acceptable for file I/O, which is already slow)
- Duplication if formats are mostly similar (XMI 1.2 vs 2.1 share ~80% structure)
- Must define a common intermediate representation to avoid adapter-to-adapter coupling

**Recommendation: ADOPT** — this is essential for backward compatibility. The C++ codebase supports 4 different file formats; the Rust rewrite must match or exceed this.

```rust
// Common intermediate representation for all adapters
#[derive(Default)]
pub struct RawModel {
    pub objects: Vec<RawObject>,
    pub associations: Vec<RawAssociation>,
    pub diagrams: Vec<RawDiagram>,
    pub stereotypes: Vec<RawStereotype>,
}

// Each adapter produces a RawModel, which is then normalized into a Document
pub trait ModelImporter {
    fn import(&self, source: &mut dyn Read) -> Result<RawModel, ImportError>;
}

// Normalizer converts RawModel → Document (shared across all importers)
pub struct ModelNormalizer;
impl ModelNormalizer {
    pub fn normalize(raw: RawModel) -> Result<Document, NormalizationError> {
        // Deduplicate, validate cross-references, build arena
    }
}
```

---

## Overall Architecture Recommendation

The following patterns work together to form the recommended architecture for Umbrello-RS:

### Primary Patterns (ADOPT)

| Pattern | Role | Why |
|---|---|---|
| **Document Model** | Top-level organizational pattern | UML tool is inherently document-oriented; single model tree with views |
| **Command** | All mutation paths | Clean undo/redo; testable; composable |
| **Generational Arena** | Object storage | Safety; no dangling pointers; stable iteration |
| **Visitor (enum)** | Serialization, code generation | Exhaustive type matching; zero-cost dispatch |
| **Adapter** | XMI versioning, file import/export | Backward compatibility; isolated format support |
| **Builder** | Complex object construction | Validation at build time; self-documenting API |

### Supporting Patterns (ADAPT)

| Pattern | Role | Adaptation Notes |
|---|---|---|
| **Observer (Event Bus)** | Cross-cutting notifications | Use typed event bus for 1:N; direct calls for 1:1 |
| **Strategy** | Layout, export, rendering | Trait-based dispatch for swappable algorithms |
| **DDD (selective)** | Aggregate boundaries, domain events | Apply vocabulary, avoid over-engineering |
| **Plugin (compile-time)** | Code generators, importers | Cargo feature gates; WASM later if needed |

### Patterns to Avoid

| Pattern | Reason |
|---|---|
| **ECS** | Overkill for fixed metamodel; use for diagram layer only if needed |
| **Event Sourcing** | Too complex for single-user desktop app; lightweight session history only |
| **Deep Inheritance** | The C++ codebase demonstrates the cost (28 virtual methods, diamond problems, hard to test) |

### Rust-Specific Architecture Principles

1. **Prefer enum over trait object** for closed type hierarchies (UML model types are closed — UML 2.x spec defines them exhaustively)
2. **Prefer `Box<dyn Trait>`** for open hierarchies (code generators, importers, layout engines)
3. **Use `slotmap`** instead of `Vec<Option<T>>` for sparse ID-based storage
4. **Use `flume`** for event-passing (bounded channels prevent memory leaks from slow consumers)
5. **Avoid `RefCell`** in the model layer — design the document API to accept `&mut self` for mutations and `&self` for queries (borrow checker enforces discipline)
6. **Use `thiserror` + `anyhow`** for error handling — every fallible operation returns a typed error

---

## Pattern Interaction Diagram

```
 ┌─────────────────────────────────────────────────────────────────────────┐
 │                        Document (central model)                         │
 │  ┌──────────────────────────────────────────────────────────────────┐   │
 │  │  ObjectStore (Generational Arena: SlotMap<ObjectKey, UmlObject>)  │   │
 │  │  ┌──────────────────────────────────────────────────────────┐    │   │
 │  │  │ UmlObject (enum with 28 variants) — owned by arena       │    │   │
 │  │  │ Package → Class/Interface → Attribute/Operation          │    │   │
 │  │  │ Association → references Class via ObjectKey             │    │   │
 │  │  └──────────────────────────────────────────────────────────┘    │   │
 │  │                                                                  │   │
 │  │  ┌──────────────────────────────────────────┐                     │   │
 │  │  │ EventBus<DomainEvent>                     │                    │   │
 │  │  │  → ObjectCreated, ObjectRemoved, ...      │                    │   │
 │  │  └──────────────────────────────────────────┘                    │   │
 │  └──────────────────────────────────────────────────────────────────┘   │
 └──────────────────────────────┬──────────────────────────────────────────┘
                                │
          ┌─────────────────────┼──────────────────────────┐
          │                     │                          │
          ▼                     ▼                          ▼
 ┌─────────────────┐  ┌──────────────────┐  ┌──────────────────────────┐
 │   DiagramView   │  │   PropertyPanel  │  │    TreeView (dock)       │
 │   (Scene/View)  │  │   (inspector)    │  │    (UMLListView analog)  │
 │                 │  │                  │  │                          │
 │ ┌─────────────┐ │  │ Subscribes to    │  │ Subscribes to            │
 │ │WidgetStore  │ │  │ ObjectSelected,  │  │ ObjectCreated/Removed    │
 │ │(Arena)      │ │  │ AttributeChanged │  │                          │
 │ └─────────────┘ │  └──────────────────┘  └──────────────────────────┘
 └────────┬────────┘
          │
          ▼
 ┌──────────────────────────────────────────────────────────────────────┐
 │                     Mutation Pipeline                                 │
 │                                                                       │
 │  UI Action → Command (Box<dyn Command>)→ UndoStack → Document::mut() │
 │                  ↕                          ↕                          │
 │            CompoundCommand              generational                  │
 │            (macro or Vec)               invalidation                  │
 └──────────────────────────────────────────────────────────────────────┘

 ┌──────────────────────────────────────────────────────────────────────┐
 │                     File I/O Pipeline                                 │
 │                                                                       │
 │  Load: BufRead → XmiImport (Adapter) → RawModel → Normalizer → Doc   │
 │  Save: Document → XmiExport (Adapter) → BufWrite                     │
 │                                                                       │
 │  Adapters: Xmi12Import, Xmi21Import, ArgoUmlImport, RoseImport       │
 │                                                                       │
 │  Internally: Visitor (UmlVisitor) traverses arena for serialization  │
 └──────────────────────────────────────────────────────────────────────┘

 ┌──────────────────────────────────────────────────────────────────────┐
 │                     Plugin / Extension Layer                          │
 │                                                                       │
 │  CodeGenerator (trait) ←── Cargo feature gates ───→ impl per lang    │
 │  CodeImporter  (trait) ←── Cargo feature gates ───→ impl per lang    │
 │  LayoutEngine  (trait) ←── Grid, Dot, Hierarchical                   │
 │  ExportFormat  (trait) ←── SVG, PNG, PDF                            │
 └──────────────────────────────────────────────────────────────────────┘
```

---

## Anti-Patterns to Avoid

### 1. God Object (`UMLApp` in C++)
**What it looks like:** A single struct that every other module imports and calls methods on. In the C++ codebase, 175 files reference `UMLApp::app()`.
**How to avoid:** Inject only the API traits each subsystem needs. The document should expose narrow interfaces (`ModelApi`, `MutationApi`), and subsystems receive only the interfaces they require.

```rust
// WRONG — recreating UMLApp's global access
pub static DOCUMENT: Lazy<Mutex<Document>> = Lazy::new(|| Mutex::new(Document::new()));

// RIGHT — pass references through the construction chain
pub struct DiagramView {
    // Only needs read access to models and ability to subscribe to events
    model: Box<dyn ModelApi>,
    events: Box<dyn EventSubscriber>,
}

impl DiagramView {
    pub fn new(model: Box<dyn ModelApi>, events: Box<dyn EventSubscriber>) -> Self { ... }
}
```

### 2. Deep Inheritance Chains
**What it looks like:** `QObject → UMLObject → UMLCanvasObject → UMLPackage → UMLClassifier → UMLClass`. Each level adds fields and virtual methods, making the class hard to understand, test, and modify.
**How to avoid:** Use composition (struct fields) and enum dispatch instead of inheritance. A `UmlObject::Class(ClassData)` has exactly the fields it needs, not those inherited from 5 levels up.

### 3. Global Mutable State
**What it looks like:** `OptionState` singleton, `UMLApp::app()` returning a mutable pointer from anywhere.
**How to avoid:** Rust's type system makes this difficult (good!). Pass configuration explicitly through the dependency chain. Use `Arc<Config>` for immutable shared config; use message passing for mutable state changes.

### 4. Switch-on-Type in Domain Code
**What it looks like:** `if (obj->isUMLClass()) { ... } else if (obj->isUMLInterface()) { ... }`  scattered across 50+ locations.
**How to avoid:** Use the Visitor pattern for type-dispatching operations. `obj.accept(visitor)` keeps type-specific logic in one place. For simple checks, use `match` on the enum variant directly.

### 5. Raw Pointer Passing
**What it looks like:** `UMLObject*` passed around without ownership semantics. Who deletes? When is it valid?
**How to avoid:** Use `ObjectKey` (generational index) for references. The `ObjectStore` manages all lifetimes. Access returns `Option<&T>`.

### 6. Overly Abstract Code
**What it looks like:** Trait hierarchies with a single implementation, factory methods for every object, "strategy" for everything.
**How to avoid:** YAGNI — don't abstract until you have a second implementation. A concrete struct with a few methods is better than a trait with `Box<dyn>` overhead and three layers of indirection.

### 7. Callback Hell (deeply nested observers)
**What it looks like:** Observer A triggers change B, which triggers observer C, which triggers change D, creating cascading updates.
**How to avoid:** Use a **deferred event bus** — collect events during mutation, then emit them in a batch after the mutation is complete. This prevents cascading and makes data flow predictable.

```rust
impl Document {
    pub fn add_class(&mut self, name: &str) -> ObjectKey {
        let key = self.objects.insert(UmlObject::Class(...));
        // Collect event for later emission (not immediate)
        self.pending_events.push(DomainEvent::ObjectCreated(key));
        key
    }

    pub fn flush_events(&mut self) {
        // Emit all pending events after mutation is complete
        for event in self.pending_events.drain(..) {
            self.event_bus.emit(&event);
        }
    }
}
```

---

## Crate Structure Mapping

How the recommended patterns map to the proposed crate decomposition (from `dependency_map.md`):

```
umbrello-rs/
│
├── umbrello-core/                      ← PATTERNS: Document Model, Arena, DDD
│   ├── src/
│   │   ├── document/                   ← Document (central), UndoStack (Command)
│   │   │   ├── mod.rs
│   │   │   ├── document.rs             ← Document struct, model API traits
│   │   │   ├── undo.rs                 ← Command trait, CompoundCommand, UndoStack
│   │   │   └── commands/              ← Concrete command implementations
│   │   │       ├── create_object.rs
│   │   │       ├── delete_object.rs
│   │   │       ├── rename_object.rs
│   │   │       └── modify_attribute.rs
│   │   ├── model/                      ← UmlObject enum + data structs (Visitor target)
│   │   │   ├── mod.rs                  ← UmlObject enum (28 variants)
│   │   │   ├── object.rs               ← ObjectId, ObjectKey, ObjectStore (Arena)
│   │   │   ├── class.rs                ← ClassData
│   │   │   ├── attribute.rs            ← AttributeData
│   │   │   ├── operation.rs            ← OperationData
│   │   │   ├── association.rs          ← AssociationData
│   │   │   ├── interface.rs
│   │   │   ├── package.rs              ← PackageData
│   │   │   ├── stereotype.rs
│   │   │   ├── constraint.rs
│   │   │   └── ... (remaining types)
│   │   ├── visitor.rs                  ← UmlVisitor trait + VisitFlow enum
│   │   ├── builder.rs                  ← ClassBuilder, AssociationBuilder, etc.
│   │   ├── events.rs                   ← DomainEvent enum, EventBus
│   │   └── error.rs                    ← thiserror enums
│   └── Cargo.toml
│
├── umbrello-diagram/                   ← PATTERNS: Arena (widgets), Observer
│   ├── src/
│   │   ├── scene.rs                    ← Scene, View, WidgetStore (Arena)
│   │   ├── widgets/                    ← Widget enum, concrete WidgetData per type
│   │   ├── layout/                     ← Strategy: LayoutEngine trait + impls
│   │   ├── rendering/                  ← Strategy: Renderer trait + impls
│   │   ├── interaction/                ← State: ToolState enum (replaces ToolBarState hierarchy)
│   │   └── export/                     ← Strategy: ExportFormat trait + impls
│   └── Cargo.toml
│
├── umbrello-codegen/                   ← PATTERNS: Plugin (compile-time), Visitor
│   ├── src/
│   │   ├── registry.rs                 ← GeneratorRegistry (compile-time plugin registry)
│   │   ├── trait.rs                    ← CodeGenerator trait
│   │   ├── cpp/                        ← Feature-gated: #[cfg(feature="codegen-cpp")]
│   │   ├── python/                     ← Feature-gated
│   │   ├── java/                       ← Feature-gated
│   │   ├── rust/                       ← Feature-gated
│   │   └── ... (remaining languages)
│   └── Cargo.toml
│
├── umbrello-codeimport/                ← PATTERNS: Plugin, Adapter
│   ├── src/
│   │   ├── registry.rs                 ← ImporterRegistry
│   │   ├── trait.rs                    ← CodeImporter trait
│   │   ├── cpp_import/
│   │   ├── java_import/
│   │   ├── python_import/
│   │   └── ...
│   └── Cargo.toml
│
├── umbrello-persistence/               ← PATTERNS: Adapter, Visitor, Builder
│   ├── src/
│   │   ├── import/                     ← XmiImport trait + adapters
│   │   │   ├── xmi12.rs               ← Adapter for XMI 1.2
│   │   │   ├── xmi21.rs               ← Adapter for XMI 2.1
│   │   │   ├── argouml.rs             ← Adapter for ArgoUML
│   │   │   └── rose.rs                ← Adapter for Rational Rose
│   │   ├── export/                     ← XmiExport trait + adapters
│   │   ├── raw_model.rs                ← RawModel (common intermediate representation)
│   │   ├── normalizer.rs               ← RawModel → Document conversion
│   │   └── format_detector.rs          ← Peek-and-dispatch format resolver
│   └── Cargo.toml
│
├── umbrello-settings/                  ← PATTERNS: Observer, Strategy
│   ├── src/
│   │   ├── config.rs                   ← Configuration struct (Arc<Config>)
│   │   ├── option_state.rs             ← OptionState (per-diagram settings)
│   │   └── ...
│   └── Cargo.toml
│
├── umbrello-ui/                        ← PATTERNS: Observer (EventBus), Strategy
│   ├── src/
│   │   ├── app.rs                      ← Application shell, window management
│   │   ├── main_window.rs              ← Main window, dock areas, menu/toolbar
│   │   ├── dialogs/                    ← Editors, wizards, dialogs
│   │   ├── panels/                     ← Dock panels (tree, properties, docs)
│   │   └── widgets/                    ← Custom UI widgets (tree view, etc.)
│   └── Cargo.toml
│
├── umbrello-rs/                        ← Binary crate (thin entry point)
│   ├── src/main.rs                     ← CLI handling, app startup
│   └── Cargo.toml
│
└── Cargo.toml                          ← Workspace root
```

### Mapping Summary

| Crate | Primary Patterns |
|---|---|
| `umbrello-core` | Document Model, Command, Arena, DDD, Visitor, Builder, Observer |
| `umbrello-diagram` | Arena (widgets), Strategy (layout/rendering/export), State, Observer |
| `umbrello-codegen` | Plugin, Visitor, Strategy, Builder |
| `umbrello-codeimport` | Plugin, Adapter, Builder |
| `umbrello-persistence` | Adapter, Visitor, Builder |
| `umbrello-settings` | Observer, Strategy |
| `umbrello-ui` | Observer, Strategy |

---

## Decision Matrix Summary

| Pattern | Adopt? | Rationale |
|---|---|---|
| **ECS** | ❌ Avoid | Overkill for fixed UML metamodel; use only for diagram layer if needed |
| **Document Model** | ✅ Adopt | Core architectural pattern; use with trait-based interface segregation |
| **DDD** | 🔶 Adapt selectively | Entities (yes), Value objects (yes), Aggregates (loosely), Repository (no) |
| **Plugin** | ✅ Adopt (compile-time) | Feature gates + trait dispatch; WASM later if needed |
| **Event Sourcing** | ❌ Avoid | Too complex; session-history undo only |
| **Command** | ✅ Adopt | Core mutation pattern; two stacks (model + widget); compound commands |
| **Observer** | 🔶 Adapt | Event bus for 1:N; direct calls for 1:1; avoid deep chains |
| **Arena** | ✅ Adopt | `slotmap::SlotMap` for model objects and widgets |
| **Builder** | ✅ Adopt | For types with 5+ fields or complex validation |
| **Visitor** | ✅ Adopt | Enum-based for serialization/codegen; `enum_dispatch` for speed |
| **Strategy** | ✅ Adopt | Layout, export, rendering, style algorithms |
| **Adapter** | ✅ Adopt | XMI versioning, multiple import formats, backward compat |

---

*This document is a living reference. As the rewrite progresses, patterns should be revisited if new evidence (e.g., performance bottlenecks, ergonomic friction) emerges.*
