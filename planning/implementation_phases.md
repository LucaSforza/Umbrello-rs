# Umbrello-RS: Implementation Phases

> **Document:** `rust-rewrite/planning/implementation_phases.md`
> **Status:** Planning
> **Last updated:** 2026-06-23
>
> This document breaks down the Rust rewrite into 24 concrete, independently
> implementable milestones. Each phase is designed for delegation to worker
> agents and produces testable, working code.
>
> All phase estimates assume a single experienced Rust developer. Parallel
> tracks are noted where phases can overlap.

---

## Phase Dependency Graph

```
Phase 0  (Project Scaffolding)
   │
   ▼
Phase 1  (Core Types & Enums)
   │
   ▼
Phase 2  (UML Model Data Structures)
   │
   ├──────────────────────────────────────┐
   ▼                                      ▼
Phase 3  (Model Mutation & Events)    Phase 4  (XMI Serialization)
   │                                      │
   ├──────────┬───────────┬───────────────┘
   ▼          ▼           ▼
Phase 5    Phase 6    Phase 7
(File      (Undo/     (CLI
Persist.)   Redo)      App)
   │          │           │
   ├──────────┼───────────┼────────────────────────────────────────────────
   ▼          ▼           ▼
Phase 8  (Code Import Framework)
   │
   ├──────────────────────────────────┐
   ▼                                  ▼
Phase 9  (C++ Import)           Phase 12  (Code Gen Framework)
Phase 10 (Java Import)              │
Phase 11 (Python Import)            ├──────────────┬──────────────┐
   │                                ▼              ▼              ▼
   │                           Phase 13        Phase 14       Phase 15
   │                           (C++ Codegen)   (Java Codegen) (Python Codegen)
   │
   └──────────────────────────────────────────────────────────────────────┐
                                                                          ▼
                               Phase 16 (Diagram Model) ◄────────────────┘
                                   │
                                   ▼
                               Phase 17 (Layout Algorithms)
                                   │
                                   ▼
                               Phase 18 (Diagram Rendering)
                                   │
                                   ▼
                               Phase 19 (Interactive Editing)
                                   │
                                   ▼
                               Phase 20 (Desktop GUI Application)
                                   │
                   ┌───────────────┼───────────────┐
                   ▼               ▼               ▼
               Phase 21        Phase 22        Phase 23
               (Addl           (Addl           (Foreign Format
                Importers)      Codegens)        Import)
                   │               │               │
                   └───────────────┼───────────────┘
                                   ▼
                               Phase 24 (Polish & QA)
```

---

## Conventions Used

**Crate layout convention:**
```
crates/
├── uml-core/          # Pure data: enums, model types, repository
├── uml-xmi/           # XMI serialization (reader/writer)
├── uml-persistence/   # File I/O, compression, storage backends
├── uml-undo/          # Undo/redo command system
├── uml-cli/           # Command-line application
├── uml-import/        # Code import framework (traits, registry)
├── uml-import-cpp/    # C++ code import (tree-sitter)
├── uml-import-java/   # Java code import
├── uml-import-python/ # Python code import
├── uml-codegen/       # Code generation framework (traits, CodeWriter, registry)
├── uml-codegen-cpp/   # C++ code generator
├── uml-codegen-java/  # Java code generator
├── uml-codegen-python/# Python code generator
├── uml-diagram/       # Diagram model (widget data, edges, store)
├── uml-layout/        # Layout algorithms (grid, alignment, auto-layout)
├── uml-render/        # Diagram rendering (Renderer trait, backends)
├── umbrello-desktop/  # Desktop GUI application (egui, winit)
└── xtask/             # Developer workflow automation
```

**Testing convention:**
- Every public function has unit tests
- `#[cfg(test)] mod tests { ... }` in every module
- Integration tests in `tests/` directory at crate level
- Test fixtures in `tests/data/` and `tests/import/`

**Error handling convention:**
- Each crate defines its own error type using `thiserror`
- `Result<T, CrateError>` for all fallible operations
- No `unwrap()` or `expect()` in production code

---

## PHASE 0: PROJECT SCAFFOLDING (1 week)

**Crates involved:** Workspace manifest, `xtask`
**Dependencies:** None
**Risk:** Low
**Parallelizable with:** Nothing (foundation)

### Task Breakdown

#### 0.1 Cargo Workspace Initialization

Create root `Cargo.toml` workspace manifest:

```toml
[workspace]
resolver = "2"
members = [
    "crates/uml-core",
    "crates/uml-xmi",
    "crates/uml-persistence",
    "crates/uml-undo",
    "crates/uml-cli",
    "crates/uml-import",
    "crates/uml-import-cpp",
    "crates/uml-import-java",
    "crates/uml-import-python",
    "crates/uml-codegen",
    "crates/uml-codegen-cpp",
    "crates/uml-codegen-java",
    "crates/uml-codegen-python",
    "crates/uml-diagram",
    "crates/uml-layout",
    "crates/uml-render",
    "crates/umbrello-desktop",
    "crates/xtask",
]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "GPL-2.0-or-later"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
thiserror = "2"
quick-xml = "0.37"
slotmap = "1"
bitflags = "2"
uuid = { version = "1", features = ["v4", "serde"] }
tracing = "0.1"
tracing-subscriber = "0.3"
petgraph = "0.7"
clap = { version = "4", features = ["derive"] }
```

#### 0.2 Empty Crate Stubs

For each member crate, create `Cargo.toml` with correct dependencies and
`src/lib.rs` with module structure. Example for `uml-core`:

```rust
//! Core UML model types and definitions.
#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

pub mod types;
pub mod model;
pub mod id;
pub mod error;
```

#### 0.3 CI/CD Setup

`.github/workflows/ci.yml`:
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo build --workspace`
- `cargo test --workspace`

#### 0.4 Developer Tooling

- `rust-toolchain.toml` — pin stable
- `rustfmt.toml` — 4-space indent, match project style
- `clippy.toml` — allowed lints
- `deny.toml` — license audit (GPL-2.0-or-later), advisory DB

#### 0.5 xtask Crate

```rust
// Usage: cargo xtask <command>
// Commands:
//   build        — cargo build --workspace
//   test         — cargo test --workspace
//   docs         — build mdBook documentation
//   init-test-data — copy XMI files from C++ repo
//   check-xmi <path> — validate XMI file structure
fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("build") => run("cargo build --workspace"),
        Some("test") => run("cargo test --workspace"),
        Some("docs") => run("mdbook build docs"),
        Some("init-test-data") => copy_test_data(),
        Some("check-xmi") => check_xmi(args.get(2).expect("path required")),
        _ => eprintln!("Usage: cargo xtask <command>"),
    }
}
```

#### 0.6 Documentation Framework

`docs/` with mdBook skeleton:
```
docs/
├── book.toml
├── src/
│   ├── SUMMARY.md
│   ├── introduction.md
│   ├── architecture.md
│   ├── crate-reference.md
│   └── development-guide.md
```

#### 0.7 Test Data

`tests/data/` copied from C++ repository XMI test files:
```
tests/data/
├── test-1.2.xmi              # UML 1.2: classes, attributes, operations
├── test-2.1.xmi              # UML 2.1: packagedElement style
├── test-associations.xmi     # All association types
├── test-components.xmi       # Component/Deployment diagrams
├── test-statemachine.xmi     # State machine diagram
├── test-usecase.xmi          # Use case diagram
├── test-entity.xmi           # Entity relationship
├── test-foreign-dialect.xmi  # NSUML dialect
├── test-roundtrip.xmi        # Comprehensive round-trip test
├── argo-example.zargo        # ArgoUML import test
└── rose-example.mdl          # Rational Rose import test
```

### Acceptance Criteria

- `cargo build --workspace` succeeds
- `cargo test --workspace` runs zero tests and passes
- `cargo clippy --workspace --all-targets -- -D warnings` passes
- `cargo fmt --all --check` passes
- CI pipeline shows green build on first push

### Deliverables

- Complete workspace structure with 18 empty crate stubs
- CI pipeline (GitHub Actions)
- Developer tooling configuration (rustfmt, clippy, deny)
- Documentation skeleton (mdBook)
- XMI test files ready for Phase 4

---

## PHASE 1: CORE TYPES AND ENUMS (2-3 weeks)

**Crates involved:** `uml-core`
**Dependencies:** Phase 0
**Risk:** Low
**Parallelizable with:** Nothing (this is the foundation)

### Task Breakdown

#### 1.1 Identity Types (`uml-core/src/id.rs`)

```rust
/// Unique identifier for UML model objects.
/// Uses UUID internally, constructable from XMI ID strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UmlId(uuid::Uuid);

impl UmlId {
    pub fn new() -> Self { Self(uuid::Uuid::new_v4()) }
    pub fn from_xmi(s: &str) -> Result<Self, IdParseError> { /* parse UUID or hash */ }
    pub fn to_xmi(&self) -> String { self.0.to_string().to_uppercase() }
}

impl Default for UmlId { fn default() -> Self { Self::new() } }

/// Generational index for arena-based storage within a session.
pub type ObjectIndex = slotmap::DefaultKey;
```

**Tests:**
```rust
#[test]
fn test_umlid_roundtrip() {
    let id = UmlId::new();
    let s = id.to_xmi();
    let parsed = UmlId::from_xmi(&s).unwrap();
    assert_eq!(id, parsed);
}
#[test]
fn test_umlid_from_xmi_invalid() {
    assert!(UmlId::from_xmi("not-a-uuid").is_err());
}
```

#### 1.2 Enum Types (`uml-core/src/types.rs`)

All enums implement `Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize`.

**ObjectType** (28 variants):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ObjectType {
    Class, Interface, Enum, Datatype, Entity,
    Package, Folder, Component, Artifact, Node, Port,
    Actor, UseCase, Instance, Association, Role,
    Attribute, EntityAttribute, Operation, Template, EnumLiteral,
    Stereotype, InstanceAttribute, Category,
    UniqueConstraint, ForeignKeyConstraint, CheckConstraint, EntityConstraint,
}
impl ObjectType {
    pub fn from_xmi(s: &str) -> Result<Self, ParseError> { /* ... */ }
    pub fn to_xmi_tag(self, version: XmiVersion) -> &'static str { /* ... */ }
}
```

**AssociationType** (structural only — message types in separate enum):
```rust
pub enum AssociationType {
    Generalization, Aggregation, Composition, Association,
    AssociationSelf, DirectedAssociation, Dependency, Realization,
    Containment, Anchor, Relationship, CategoryToParent,
    ChildToCategory, Exception,
}
```

**DiagramType:**
```rust
pub enum DiagramType {
    Class, UseCase, Sequence, Collaboration, State,
    Activity, Component, Deployment, EntityRelationship, Object,
}
```

**Other enums:**
```rust
pub enum Visibility { Public, Protected, Private, Implementation }
pub enum ParameterDirection { In, Out, InOut, Return }
pub enum Changeability { Changeable, Frozen, AddOnly }
pub enum LayoutType { Direct, Orthogonal, Polyline, Spline }
pub enum ProgrammingLanguage {
    Ada, ActionScript, Cpp, CSharp, D, Go, Idl, Java,
    JavaScript, MySQL, Pascal, Perl, Php4, Php5, PostgreSQL,
    Python, Ruby, Rust, Sql, Tcl, Vala, XmlSchema,
}
pub enum IndexType { None, Primary, Index, Unique }
pub enum ReferentialAction { NoAction, Restrict, Cascade, SetNull, SetDefault }
pub enum StereotypeDisplayMode { None, Text, Icon, Attribute }
```

#### 1.3 Bitflag Types (`uml-core/src/types/flags.rs`)

```rust
bitflags! {
    pub struct OperationFlags: u8 {
        const CONST = 0b0001; const OVERRIDE = 0b0010;
        const FINAL = 0b0100; const VIRTUAL = 0b1000;
        const INLINE = 0b0001_0000;
    }
}
bitflags! {
    pub struct AttributeFlags: u8 {
        const STATIC = 0b0001; const DERIVED = 0b0010;
        const READ_ONLY = 0b0100; const CONST = 0b1000;
    }
}
bitflags! {
    pub struct ClassFlags: u8 {
        const ABSTRACT = 0b0001; const LEAF = 0b0010;
        const ROOT = 0b0100; const ACTIVE = 0b1000;
    }
}
```

#### 1.4 Value Types (`uml-core/src/types/value_types.rs`)

```rust
pub struct Position { pub x: f64, pub y: f64 }
pub struct Size { pub width: f64, pub height: f64 }
pub struct Rect { pub position: Position, pub size: Size }
pub struct Color { pub r: u8, pub g: u8, pub b: u8, pub a: u8 }
pub struct Font { pub family: String, pub size: f64, pub bold: bool, pub italic: bool, pub underline: bool }
pub struct Multiplicity { pub lower: String, pub upper: String }
```

All implement `Debug, Clone, Copy, PartialEq, Serialize, Deserialize`.

#### 1.5 Error Types (`uml-core/src/error.rs`)

```rust
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("Invalid XMI ID: {0}")] InvalidId(String),
    #[error("Type mismatch: expected {expected}, got {actual}")]
    TypeMismatch { expected: ObjectType, actual: ObjectType },
    #[error("Validation error: {0}")] Validation(String),
    #[error("Object not found: {0}")] NotFound(UmlId),
    #[error("Parse error: {0}")] Parse(String),
}
```

### Acceptance Criteria

- All 28 `ObjectType` variants defined
- All 14 `AssociationType` variants defined (structural only)
- All 10 `DiagramType` variants defined
- All 4 `Visibility`, 4 `ParameterDirection`, 3 `Changeability`
- 22 `ProgrammingLanguage` variants
- All enums implement serde round-trip (serialize → deserialize → assert_eq)
- `UmlId` round-trips through string representation
- All bitflags compile and support bitwise operations
- All value types implement `PartialEq` and can be constructed
- 100% line coverage on all types in this phase

---

## PHASE 2: UML MODEL DATA STRUCTURES (3-4 weeks)

**Crates involved:** `uml-core`
**Dependencies:** Phase 1
**Risk:** Medium
**Parallelizable with:** Phase 3 later, Phase 4 independently

### Task Breakdown

#### 2.1 ModelElement Enum (`uml-core/src/model/element.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelElement {
    Class(Box<UmlClass>), Interface(Box<UmlInterface>),
    Enum(Box<UmlEnum>), Datatype(Box<UmlDatatype>),
    Entity(Box<UmlEntity>), Package(Box<UmlPackage>),
    Folder(Box<UmlFolder>), Component(Box<UmlComponent>),
    Artifact(Box<UmlArtifact>), Node(Box<UmlNode>),
    Port(Box<UmlPort>), Actor(Box<UmlActor>),
    UseCase(Box<UmlUseCase>), Instance(Box<UmlInstance>),
    Association(Box<UmlAssociation>), Role(Box<UmlRole>),
    Attribute(Box<UmlAttribute>), EntityAttribute(Box<UmlEntityAttribute>),
    Operation(Box<UmlOperation>), Template(Box<UmlTemplate>),
    EnumLiteral(Box<UmlEnumLiteral>), Stereotype(Box<UmlStereotype>),
    InstanceAttribute(Box<UmlInstanceAttribute>),
    Category(Box<UmlCategory>),
    UniqueConstraint(Box<UmlUniqueConstraint>),
    ForeignKeyConstraint(Box<UmlForeignKeyConstraint>),
    CheckConstraint(Box<UmlCheckConstraint>),
}

impl ModelElement {
    pub fn id(&self) -> UmlId { /* match on variant, return .id */ }
    pub fn name(&self) -> &str { /* ... */ }
    pub fn object_type(&self) -> ObjectType { /* ... */ }
    pub fn visibility(&self) -> Visibility { /* ... */ }
    pub fn documentation(&self) -> &str { /* ... */ }
    pub fn set_name(&mut self, name: String) { /* ... */ }
}
```

#### 2.2 Concrete Structs (`uml-core/src/model/`)

One file per type, all implement `Debug, Clone, PartialEq, Serialize, Deserialize`:

| File | Structs | Key Fields |
|------|---------|------------|
| `class.rs` | `UmlClass` | id, name, visibility, stereotype, documentation, flags (abstract/leaf/root/active), attributes, operations, templates, owned_objects |
| `interface.rs` | `UmlInterface` | Same as class |
| `enumeration.rs` | `UmlEnum`, `UmlEnumLiteral` | + literals: Vec<UmlId> |
| `datatype.rs` | `UmlDatatype` | is_reference, is_active, origin_type |
| `entity.rs` | `UmlEntity` | + entity_attributes, constraints, primary_key |
| `package.rs` | `UmlPackage` | + owned_objects: Vec<UmlId> |
| `folder.rs` | `UmlFolder` | + diagrams: Vec<UmlId>, local_name, folder_file |
| `component.rs` | `UmlComponent` | + executable: bool, owned_objects |
| `artifact.rs` | `UmlArtifact` | + draw_as_type: ArtifactDrawType |
| `node.rs` | `UmlNode` | Simple leaf |
| `port.rs` | `UmlPort` | + parent: Option<UmlId> |
| `actor.rs` | `UmlActor` | Simple leaf |
| `usecase.rs` | `UmlUseCase` | Simple leaf |
| `category.rs` | `UmlCategory` | + category_type: CategoryType |
| `instance.rs` | `UmlInstance`, `UmlInstanceAttribute` | + classifier_ref, attribute_values |
| `association.rs` | `UmlAssociation`, `UmlRole` | assoc_type, role_a/b, multiplicity, changeability |
| `attribute.rs` | `UmlAttribute`, `UmlEntityAttribute` | type_ref, initial_value, flags, index_type, auto_increment |
| `operation.rs` | `UmlOperation` | return_type, parameters, flags, source_code |
| `template.rs` | `UmlTemplate` | type_ref |
| `stereotype.rs` | `UmlStereotype` | attribute_defs, ref_count |
| `constraint.rs` | `UmlUniqueConstraint`, `UmlForeignKeyConstraint`, `UmlCheckConstraint` | entity_attributes, referenced_entity, condition |

#### 2.3 ObjectRepository (`uml-core/src/model/repository.rs`)

```rust
/// Generational-index-based storage for all UML model objects.
pub struct ObjectRepository {
    objects: SlotMap<ObjectIndex, ModelElement>,
    // Type-filtered iteration caches
    by_type: SecondaryMap<ObjectIndex, ObjectType>,
    // Parent-child tracking
    parent: SecondaryMap<ObjectIndex, ObjectIndex>,
    children: SecondaryMap<ObjectIndex, Vec<ObjectIndex>>,
}

impl ObjectRepository {
    pub fn new() -> Self;
    pub fn insert(&mut self, element: ModelElement) -> ObjectIndex;
    pub fn get(&self, key: ObjectIndex) -> Option<&ModelElement>;
    pub fn get_mut(&mut self, key: ObjectIndex) -> Option<&mut ModelElement>;
    pub fn remove(&mut self, key: ObjectIndex) -> Option<ModelElement>;
    pub fn contains(&self, key: ObjectIndex) -> bool;

    // Filtered iteration
    pub fn iter(&self) -> impl Iterator<Item = (ObjectIndex, &ModelElement)>;
    pub fn iter_by_type(&self, ty: ObjectType) -> impl Iterator<Item = (ObjectIndex, &ModelElement)>;

    // Hierarchy
    pub fn children_of(&self, parent: ObjectIndex) -> &[ObjectIndex];
    pub fn parent_of(&self, child: ObjectIndex) -> Option<ObjectIndex>;
    pub fn set_parent(&mut self, child: ObjectIndex, parent: Option<ObjectIndex>);

    // Deep clone
    pub fn deep_clone(&self, root: ObjectIndex) -> Result<(Self, HashMap<ObjectIndex, ObjectIndex>), CoreError>;
}
```

#### 2.4 Builder Pattern (`uml-core/src/model/builder.rs`)

```rust
pub struct ClassBuilder { name: Option<String>, visibility: Option<Visibility>, /* ... */ }
impl ClassBuilder {
    pub fn name(mut self, name: &str) -> Self;
    pub fn visibility(mut self, v: Visibility) -> Self;
    pub fn abstract_(mut self) -> Self;
    pub fn add_attribute(mut self, attr: AttributeBuilder) -> Self;
    pub fn add_operation(mut self, op: OperationBuilder) -> Self;
    pub fn build(self) -> Result<UmlClass, CoreError>;
}
pub struct AttributeBuilder { /* ... */ }
pub struct OperationBuilder { /* ... */ }
pub struct AssociationBuilder { /* ... */ }
pub struct PackageBuilder { /* ... */ }
```

### Acceptance Criteria

- Can construct a complete UML model programmatically
- All struct types implement Debug, Clone, PartialEq, Serialize, Deserialize
- ObjectRepository: insert, lookup, mutate, remove work correctly
- Filtered iteration by type works
- Builder pattern validates required fields
- Deep clone produces independent copy
- Serde round-trip on all types (JSON format for test)

### Test Example

```rust
#[test]
fn test_build_class_model() {
    let mut repo = ObjectRepository::new();
    let pkg = PackageBuilder::new().name("com.example").build().unwrap();
    let pkg_key = repo.insert(ModelElement::Package(Box::new(pkg)));
    let cls = ClassBuilder::new()
        .name("Person").visibility(Visibility::Public)
        .add_attribute(AttributeBuilder::new().name("name").type_name("String").build().unwrap())
        .add_attribute(AttributeBuilder::new().name("age").type_name("int").build().unwrap())
        .add_operation(OperationBuilder::new().name("say_hello").build().unwrap())
        .build().unwrap();
    let cls_key = repo.insert(ModelElement::Class(Box::new(cls)));
    repo.set_parent(cls_key, Some(pkg_key));
    let person = repo.get(cls_key).unwrap();
    assert_eq!(person.name(), "Person");
}
```

---

## PHASE 3: MODEL MUTATION AND EVENTS (2-3 weeks)

**Crates involved:** `uml-core`
**Dependencies:** Phase 2
**Risk:** Medium
**Parallelizable with:** Phase 4 (XMI serialization)

### Task Breakdown

#### 3.1 UmlChange Enum (`uml-core/src/model/change.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UmlChange {
    // Object lifecycle
    ObjectCreated { id: UmlId, object_type: ObjectType, parent: Option<UmlId> },
    ObjectRemoved { id: UmlId, snapshot: Box<ModelElement> },
    ObjectRenamed { id: UmlId, old_name: String, new_name: String },
    // Property changes
    VisibilityChanged { id: UmlId, old: Visibility, new: Visibility },
    StereotypeChanged { id: UmlId, old: Option<UmlId>, new: Option<UmlId> },
    DocumentationChanged { id: UmlId, old: String, new: String },
    AbstractChanged { id: UmlId, old: bool, new: bool },
    StaticFlagChanged { id: UmlId, old: bool, new: bool },
    // Attribute operations
    AttributeAdded { classifier_id: UmlId, attribute: Box<UmlAttribute> },
    AttributeRemoved { classifier_id: UmlId, attribute_id: UmlId, snapshot: Box<UmlAttribute> },
    // Operation operations
    OperationAdded { classifier_id: UmlId, operation: Box<UmlOperation> },
    OperationRemoved { classifier_id: UmlId, operation_id: UmlId, snapshot: Box<UmlOperation> },
    // Association operations
    AssociationCreated { association: Box<UmlAssociation> },
    AssociationRemoved { association_id: UmlId, snapshot: Box<UmlAssociation> },
    AssociationTypeChanged { association_id: UmlId, old: AssociationType, new: AssociationType },
    // Template operations
    TemplateAdded { classifier_id: UmlId, template: Box<UmlTemplate> },
    TemplateRemoved { classifier_id: UmlId, template_id: UmlId },
    // Enum literal operations
    EnumLiteralAdded { enum_id: UmlId, literal: Box<UmlEnumLiteral> },
    EnumLiteralRemoved { enum_id: UmlId, literal_id: UmlId },
    // Package containment
    PackageObjectAdded { package_id: UmlId, object_id: UmlId },
    PackageObjectRemoved { package_id: UmlId, object_id: UmlId },
    // Diagram lifecycle
    DiagramCreated { diagram_id: UmlId, diagram_type: DiagramType, name: String },
    DiagramRemoved { diagram_id: UmlId },
}
```

#### 3.2 Mutation API (`uml-core/src/model/mutation.rs`)

```rust
impl ObjectRepository {
    pub fn apply(&mut self, change: &UmlChange) -> Result<UmlChange, CoreError>;
    pub fn apply_batch(&mut self, changes: &[UmlChange]) -> Result<Vec<UmlChange>, CoreError>;

    pub fn create_object(&mut self, ty: ObjectType, name: &str, parent: Option<ObjectIndex>) -> Result<ObjectIndex, CoreError>;
    pub fn remove_object(&mut self, key: ObjectIndex) -> Result<ModelElement, CoreError>;
    pub fn rename_object(&mut self, key: ObjectIndex, new_name: &str) -> Result<String, CoreError>;

    pub fn add_attribute(&mut self, classifier: ObjectIndex, attr: UmlAttribute) -> Result<ObjectIndex, CoreError>;
    pub fn remove_attribute(&mut self, classifier: ObjectIndex, attr_key: ObjectIndex) -> Result<UmlAttribute, CoreError>;

    pub fn add_operation(&mut self, classifier: ObjectIndex, op: UmlOperation) -> Result<ObjectIndex, CoreError>;
    pub fn remove_operation(&mut self, classifier: ObjectIndex, op_key: ObjectIndex) -> Result<UmlOperation, CoreError>;

    pub fn create_association(&mut self, assoc_type: AssociationType, role_a: ObjectIndex, role_b: ObjectIndex) -> Result<ObjectIndex, CoreError>;
    pub fn remove_association(&mut self, assoc_key: ObjectIndex) -> Result<UmlAssociation, CoreError>;
}
```

#### 3.3 Event Bus (`uml-core/src/model/event.rs`)

```rust
#[derive(Clone)]
pub struct EventBus { sender: flume::Sender<UmlChange> }

impl EventBus {
    pub fn new() -> (Self, flume::Receiver<UmlChange>);
    pub fn publish(&self, change: &UmlChange);
}

impl ObjectRepository {
    pub fn with_event_bus(bus: EventBus) -> Self;
    pub fn set_event_bus(&mut self, bus: EventBus);
}
```

#### 3.4 Validation (`uml-core/src/model/validation.rs`)

```rust
impl ObjectRepository {
    pub fn validate(&self) -> Vec<ValidationError>;
}

pub enum ValidationError {
    DanglingReference { referrer: UmlId, target: UmlId },
    OrphanedRole { role_id: UmlId },
    ContainmentCycle(UmlId),
    MissingRequiredField { object_id: UmlId, field: &'static str },
}
```

### Acceptance Criteria

- All 20+ `UmlChange` variants defined
- `apply()` transforms model and returns inverse change
- `apply_batch()` succeeds or fully rolls back
- Event bus delivers change notifications to subscribers
- Validation catches: dangling references, containment cycles, orphaned roles
- Undo round-trip: apply(change) → apply(inverse) → original state
- No panics in any mutation path

---

## PHASE 4: XMI SERIALIZATION (3-4 weeks)

**Crates involved:** `uml-xmi`
**Dependencies:** Phase 1, Phase 2
**Risk:** High (format correctness critical)
**Parallelizable with:** Phase 3

### Task Breakdown

#### 4.1 XmiWriter (`uml-xmi/src/writer.rs`)

```rust
pub struct XmiWriter<W: Write> { writer: Writer<W>, version: XmiVersion, dialect: XmiDialect }
pub enum XmiVersion { V1_2, V2_1 }

impl<W: Write> XmiWriter<W> {
    pub fn new(writer: W, version: XmiVersion) -> Self;

    pub fn write_document(&mut self, doc: &UmlModelDocument) -> Result<(), XmiError> {
        self.write_xml_declaration()?;
        self.write_xmi_root_open()?;
        if self.version == XmiVersion::V1_2 { self.write_header()?; self.start_content()?; }
        self.write_model_element(&doc.model)?;
        self.write_stereotypes(&doc.stereotypes)?;
        self.write_root_folders(&doc.root_folders)?;
        self.write_doc_settings(&doc.doc_settings)?;
        self.write_diagrams(&doc.diagrams)?;
        self.write_listview(&doc.listview)?;
        self.write_codegen(&doc.codegen_state)?;
        if self.version == XmiVersion::V1_2 { self.end_content()?; }
        self.close_xmi_root()?;
        Ok(())
    }
}
```

**XMI 1.2 Output:**
```xml
<XMI xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.4">
  <XMI.header>...</XMI.header>
  <XMI.content>
    <UML:Model xmi.id="m1" name="..." isSpecification="false"
               isAbstract="false" isRoot="false" isLeaf="false">
      <UML:Namespace.ownedElement>
        <UML:Class xmi.id="GUID-..." name="Person" visibility="public" isAbstract="false"/>
      </UML:Namespace.ownedElement>
    </UML:Model>
  </XMI.content>
  <XMI.extensions xmi.extender="umbrello">
    <docsettings viewid="..." documentation="..." uniqueid="..."/>
    <diagrams>...</diagrams>
    <listview>...</listview>
    <codegeneration>...</codegeneration>
  </XMI.extensions>
</XMI>
```

**XMI 2.1 Output:**
```xml
<xmi:XMI xmi:version="2.1" xmlns:uml="http://schema.omg.org/spec/UML/2.1">
  <uml:Model xmi:id="m1" name="...">
    <packagedElement xmi:type="uml:Class" xmi:id="GUID-..." name="Person"/>
  </uml:Model>
  <xmi:Extension extender="umbrello">
    <docsettings viewid="..." documentation="..." uniqueid="..."/>
  </xmi:Extension>
</xmi:XMI>
```

#### 4.2 XmiReader (`uml-xmi/src/reader.rs`)

Streaming (SAX-style) via quick-xml — no DOM tree built:

```rust
pub struct XmiReader<R: Read> {
    reader: Reader<R>,
    version: XmiVersion,
    registry: ObjectRegistry,
}

impl<R: Read> XmiReader<R> {
    pub fn new(reader: R) -> Result<Self, XmiError>;

    pub fn read_document(&mut self) -> Result<UmlModelDocument, XmiError> {
        self.detect_version()?;
        let mut model = self.read_model()?;
        self.resolve_all_refs(&mut model)?;
        self.read_diagrams(&mut model)?;
        Ok(model)
    }

    fn detect_version(&mut self) -> Result<(), XmiError> {
        // Peek root element for xmi.version or xmi:version attribute
    }
}
```

#### 4.3 ObjectRegistry (`uml-xmi/src/registry.rs`)

```rust
pub struct ObjectRegistry {
    by_xmi_id: HashMap<String, ObjectIndex>,
    pending_refs: Vec<PendingRef>,
}

struct PendingRef {
    owner_index: ObjectIndex,
    field_path: String,         // dot-separated: "type_ref", "stereotype"
    target_xmi_id: String,
}

impl ObjectRegistry {
    pub fn register(&mut self, xmi_id: &str, index: ObjectIndex);
    pub fn collect_refs(&mut self, repo: &ObjectRepository);
    pub fn resolve_all(&mut self, repo: &mut ObjectRepository) -> Result<(), XmiError>;
}
```

#### 4.4 Foreign Dialects (`uml-xmi/src/dialect.rs`)

```rust
pub enum XmiDialect { Native, Nsuul, Unisys, Embarcadero, ArgoUML }

impl XmiDialect {
    pub fn detect(root_tag: &str, attrs: &HashMap<String, String>) -> Self;
}
```

#### 4.5 Model Document (`uml-xmi/src/document.rs`)

```rust
pub struct UmlModelDocument {
    pub model_id: UmlId,
    pub name: String,
    pub version: String,
    pub stereotypes: Vec<ModelElement>,
    pub root_folders: Vec<ModelElement>,
    pub diagrams: Vec<DiagramData>,
    pub doc_settings: DocSettings,
    pub listview: Option<ListViewState>,
    pub codegen_state: Option<CodeGenState>,
}

pub struct DiagramData {
    pub id: UmlId, pub name: String, pub diagram_type: DiagramType,
    pub widgets: Vec<WidgetData>, pub edges: Vec<EdgeData>,
}

pub struct WidgetData {
    pub id: UmlId, pub local_id: UmlId, pub widget_type: WidgetType,
    pub position: Position, pub size: Size,
    pub visual: WidgetVisual, pub uml_object_id: Option<UmlId>,
}

pub struct EdgeData {
    pub id: UmlId, pub widget_a_id: UmlId, pub widget_b_id: UmlId,
    pub uml_association_id: Option<UmlId>,
    pub points: Vec<Position>, pub layout_type: LayoutType,
    pub labels: Vec<EdgeLabel>,
}
```

### Acceptance Criteria

- All C++ test XMI files load successfully (tests/data/*.xmi)
- XMI 1.2 and XMI 2.1 both read and write correctly
- Round-trip: load → save → load produces identical model
- Forward references resolved correctly (no dangling refs)
- Foreign XMI dialects detected and handled
- Streaming reader uses <100MB for large files
- Writer output matches C++ Umbrello XML structure

### Test Example

```rust
#[test]
fn test_xmi_roundtrip_12() {
    let input = include_str!("../../tests/data/test-1.2.xmi");
    let mut reader = XmiReader::new(input.as_bytes()).unwrap();
    let model = reader.read_document().unwrap();

    let mut output = Vec::new();
    XmiWriter::new(&mut output, XmiVersion::V1_2).unwrap()
        .write_document(&model).unwrap();

    let mut reader2 = XmiReader::new(output.as_slice()).unwrap();
    let model2 = reader2.read_document().unwrap();
    assert_eq!(model, model2); // Structural equality
}

#[test]
fn test_load_all_test_files() {
    let files = ["test-associations.xmi", "test-components.xmi",
                  "test-statemachine.xmi", "test-usecase.xmi",
                  "test-entity.xmi", "test-foreign-dialect.xmi"];
    for fname in &files {
        let path = format!("../../tests/data/{}", fname);
        let input = std::fs::read_to_string(path).unwrap();
        let mut reader = XmiReader::new(input.as_bytes()).unwrap();
        let model = reader.read_document().unwrap();
        assert!(model.object_count() > 0, "Empty model from {}", fname);
    }
}
```

---

## PHASE 5: FILE PERSISTENCE (2 weeks)

**Crates involved:** `uml-persistence`
**Dependencies:** Phase 4
**Risk:** Low
**Parallelizable with:** Phase 6, Phase 7

### Task Breakdown

#### 5.1 StorageBackend Trait (`uml-persistence/src/backend.rs`)

```rust
pub trait StorageBackend: Debug + Send + Sync {
    fn extensions(&self) -> &[&str];
    fn can_handle(&self, path: &Path, magic: Option<&[u8]>) -> bool;
    fn load(&self, reader: &mut dyn Read) -> Result<UmlModelDocument, PersistenceError>;
    fn save(&self, writer: &mut dyn Write, model: &UmlModelDocument) -> Result<(), PersistenceError>;
}
```

#### 5.2 Backend Implementations

- `XmiStorage` — `.xmi` plain XML
- `CompressedStorage` — `.xmi.tgz` (gzip), `.xmi.tar.bz2` (bzip2)
- Backend selection by extension, fallback to magic bytes

#### 5.3 Atomic Save (`uml-persistence/src/atomic.rs`)

```rust
pub fn atomic_save(path: &Path, model: &UmlModelDocument) -> Result<(), PersistenceError> {
    let tmp = path.with_extension("tmp");
    {
        let file = File::create(&tmp)?;
        let backend = select_backend(path)?;
        backend.save(&mut file, model)?;
    }
    std::fs::rename(&tmp, path)?;
    Ok(())
}
```

#### 5.4 Autosave (`uml-persistence/src/autosave.rs`)

```rust
pub struct AutosaveConfig {
    pub interval: Duration,
    pub path: PathBuf,
    pub max_versions: usize,
}
```

### Acceptance Criteria

- Load/save round-trip for `.xmi`, `.xmi.tgz`, `.xmi.tar.bz2`
- Storage registry detects correct backend by extension
- Atomic save does not corrupt on crash
- Autosave configuration compiles

---

## PHASE 6: UNDO/REDO SYSTEM (2 weeks)

**Crates involved:** `uml-undo`
**Dependencies:** Phase 2, Phase 3
**Risk:** Medium
**Parallelizable with:** Phase 5, Phase 7

### Task Breakdown

#### 6.1 Command Trait (`uml-undo/src/command.rs`)

```rust
pub trait Command: Debug + Send {
    fn execute(&mut self, model: &mut ObjectRepository) -> Result<(), UndoError>;
    fn undo(&mut self, model: &mut ObjectRepository) -> Result<(), UndoError>;
    fn description(&self) -> Cow<'_, str>;
    fn merge_with(&mut self, _other: &dyn Command) -> bool { false }
}
```

#### 6.2 Concrete Commands

20+ command types in `uml-undo/src/cmd_*.rs`:
- `CmdCreateObject`, `CmdRemoveObject`, `CmdRenameObject`
- `CmdSetVisibility`, `CmdSetStereotype`, `CmdSetDocumentation`
- `CmdAddAttribute`, `CmdRemoveAttribute`
- `CmdAddOperation`, `CmdRemoveOperation`
- `CmdCreateAssociation`, `CmdRemoveAssociation`, `CmdSetAssociationType`
- `CmdAddEnumLiteral`, `CmdRemoveEnumLiteral`
- `CmdAddTemplate`, `CmdRemoveTemplate`
- `CmdPackageObjectAdd`, `CmdPackageObjectRemove`

#### 6.3 UndoStack (`uml-undo/src/stack.rs`)

```rust
pub struct UndoStack {
    stack: Vec<Box<dyn Command>>,
    position: usize,
    saved_position: usize,
    enabled: bool,
    macro_group: Option<(String, Vec<Box<dyn Command>>)>,
    max_size: usize,
}

impl UndoStack {
    pub fn push(&mut self, cmd: Box<dyn Command>, model: &mut ObjectRepository) -> Result<(), UndoError>;
    pub fn undo(&mut self, model: &mut ObjectRepository) -> Result<(), UndoError>;
    pub fn redo(&mut self, model: &mut ObjectRepository) -> Result<(), UndoError>;
    pub fn can_undo(&self) -> bool;
    pub fn can_redo(&self) -> bool;
    pub fn begin_macro(&mut self, description: &str);
    pub fn end_macro(&mut self, model: &mut ObjectRepository) -> Result<(), UndoError>;
    pub fn clear(&mut self);
    pub fn set_enabled(&mut self, enabled: bool);
    pub fn is_clean(&self) -> bool;
    pub fn mark_saved(&mut self);
}
```

### Acceptance Criteria

- push → undo → redo → undo sequence works correctly
- Macro grouping composites multiple commands as one undo step
- Disabled during load: execute but don't record
- Clean/dirty tracking works with undo/redo
- Max stack size enforced

---

## PHASE 7: CLI APPLICATION (2-3 weeks)

**Crates involved:** `uml-cli`
**Dependencies:** Phase 5, Phase 1, Phase 2
**Risk:** Low
**Parallelizable with:** Phase 5, Phase 6

### Task Breakdown

#### 7.1 Argument Parsing

```rust
#[derive(Parser, Debug)]
#[command(name = "umbrello", version, about = "UML Modeller")]
pub struct CliArgs {
    pub file: Option<PathBuf>,
    #[arg(long)] pub export: Option<String>,
    #[arg(long)] pub export_formats: bool,
    #[arg(long)] pub directory: Option<PathBuf>,
    #[arg(long)] pub use_folders: bool,
    #[arg(long, num_args = 1..)] pub import_files: Vec<PathBuf>,
    #[arg(long)] pub import_directory: Option<PathBuf>,
    #[arg(long)] pub languages: bool,
    #[arg(long)] pub set_language: Option<String>,
    #[arg(long)] pub validate: bool,
}
```

#### 7.2 Command Dispatch

```rust
fn run() -> Result<(), CliError> {
    let args = CliArgs::parse();
    if args.languages { return list_languages(); }
    if args.export_formats { return list_export_formats(); }
    if let Some(lang) = &args.set_language { return set_language(lang); }
    if let Some(file) = &args.file {
        if args.validate { return validate_xmi(file); }
        let model = load_model(file)?;
        if let Some(format) = &args.export { return export_diagrams(&model, format, ...); }
        return print_summary(&model);
    }
    Ok(())
}
```

#### 7.3 Model Summary

```rust
fn print_summary(model: &UmlModelDocument) {
    println!("Model: {} ({})", model.name, model.model_id);
    println!("  Objects: {}", count_objects(model));
    println!("  Diagrams: {}", model.diagrams.len());
    for diagram in &model.diagrams {
        println!("  Diagram: {} ({:?})", diagram.name, diagram.diagram_type);
    }
}
```

### Acceptance Criteria

- `umbrello --languages` lists 22 languages
- `umbrello --export-formats` lists formats
- `umbrello file.xmi` loads and prints summary
- `umbrello file.xmi --validate` checks XMI validity
- Error handling: nonexistent file → user-friendly error

---

## PHASE 8: CODE IMPORT FRAMEWORK (3-4 weeks)

**Crates involved:** `uml-import`
**Dependencies:** Phase 2, Phase 3
**Risk:** Medium
**Parallelizable with:** Phase 9, 10, 11 (they build on this)

### Task Breakdown

#### 8.1 LanguageImporter Trait (`uml-import/src/importer.rs`)

```rust
pub trait LanguageImporter: Debug + Send + Sync {
    fn name(&self) -> &'static str;
    fn extensions(&self) -> &[&'static str];
    fn import_file(&self, path: &Path, source: &str) -> Result<Vec<UmlChange>, ImportError>;
    fn initialize(&mut self, repo: &ObjectRepository) -> Result<(), ImportError> { Ok(()) }
    fn finalize(&mut self, repo: &mut ObjectRepository) -> Result<Vec<UmlChange>, ImportError> { Ok(()) }
}
```

#### 8.2 ImportRegistry (`uml-import/src/registry.rs`)

```rust
pub struct ImportRegistry { importers: HashMap<String, Box<dyn LanguageImporter>> }

impl ImportRegistry {
    pub fn register(&mut self, importer: impl LanguageImporter + 'static);
    pub fn importer_for(&self, path: &Path) -> Option<&dyn LanguageImporter>;
}
```

#### 8.3 ImportHelpers (`uml-import/src/helpers.rs`)

```rust
pub struct ImportHelpers<'a> { repo: &'a mut ObjectRepository }

impl<'a> ImportHelpers<'a> {
    pub fn ensure_package(&mut self, name: &str, parent: Option<ObjectIndex>) -> Result<ObjectIndex, ImportError>;
    pub fn create_classifier(&mut self, name: &str, ty: ObjectType, parent: ObjectIndex) -> Result<ObjectIndex, ImportError>;
    pub fn add_attribute(&mut self, classifier: ObjectIndex, attr: &ImportAttribute) -> Result<ObjectIndex, ImportError>;
    pub fn add_operation(&mut self, classifier: ObjectIndex, op: &ImportOperation) -> Result<ObjectIndex, ImportError>;
    pub fn add_generalization(&mut self, child: ObjectIndex, parent: ObjectIndex) -> Result<ObjectIndex, ImportError>;
    pub fn add_association(&mut self, from: ObjectIndex, to: ObjectIndex, assoc_type: AssociationType) -> Result<ObjectIndex, ImportError>;
}
```

#### 8.4 TreeSitterImporter Base (`uml-import/src/tree_sitter.rs`)

```rust
pub struct TreeSitterImporter {
    language: tree_sitter::Language,
    parser: tree_sitter::Parser,
}

impl TreeSitterImporter {
    pub fn new(language: tree_sitter::Language) -> Result<Self, ImportError>;
    pub fn parse_source(&mut self, source: &str) -> Result<tree_sitter::Tree, ImportError>;
}
```

### Acceptance Criteria

- `LanguageImporter` trait complete
- `ImportRegistry` dispatches correctly by extension
- `ImportHelpers` creates classifiers, attributes, operations, generalizations
- `TreeSitterImporter` base can parse a source file

---

## PHASE 9: C++ CODE IMPORT (3-4 weeks)

**Crates involved:** `uml-import-cpp`
**Dependencies:** Phase 8
**Risk:** High (C++ complexity)
**Parallelizable with:** Phase 10, Phase 11

### Task Breakdown

#### 9.1 CppImporter

```rust
pub struct CppImporter {
    base: TreeSitterImporter,
    config: CppImportConfig,
    current_access: Visibility,
}

pub struct CppImportConfig {
    pub include_paths: Vec<PathBuf>,
    pub defines: HashMap<String, Option<String>>,
    pub follow_includes: bool,
}
```

#### 9.2 AST Node Mappings

| C++ Construct | UML Element |
|---------------|-------------|
| `class_specifier` | `UmlClass` + `Generalization` per base |
| `function_definition` | `UmlOperation` |
| `field_declaration` | `UmlAttribute` |
| `enum_specifier` | `UmlEnum` + `UmlEnumLiteral` |
| `namespace_definition` | `UmlPackage` |
| `template_parameter_list` | `UmlTemplate` per parameter |
| `access_specifier` | State tracking for visibility |
| `#include` | File dependency tracking |

#### 9.3 Sub-Modules

- `class.rs` — Class/struct parsing
- `function.rs` — Function/method parsing
- `field.rs` — Field/variable parsing
- `enum_parse.rs` — Enum parsing
- `namespace.rs` — Namespace parsing
- `template.rs` — Template parameter parsing
- `access.rs` — Access specifier tracking
- `preprocessor.rs` — #include/#define handling

### Acceptance Criteria

- Parses: classes, inheritance, attributes, methods, enums, namespaces, templates
- Handles: `public`/`protected`/`private` access specifiers
- Handles: forward declarations, `#include`, `#define`
- Generates correct `UmlChange` events
- Test with real C++ headers in `tests/import/cpp/`
- Compare output with C++ Umbrello import

---

## PHASE 10: JAVA CODE IMPORT (2-3 weeks)

**Crates involved:** `uml-import-java`
**Dependencies:** Phase 8
**Risk:** Medium
**Parallelizable with:** Phase 9, 11

### Task Breakdown

#### 10.1 JavaImporter

```rust
pub struct JavaImporter {
    base: TreeSitterImporter,
    package_name: Option<String>,
    imports: Vec<String>,
}
```

#### 10.2 AST Node Mappings

| Java Construct | UML Element |
|---------------|-------------|
| `package_declaration` | `UmlPackage` |
| `class_declaration` | `UmlClass` |
| `interface_declaration` | `UmlInterface` |
| `enum_declaration` | `UmlEnum` |
| `field_declaration` | `UmlAttribute` |
| `method_declaration` | `UmlOperation` |
| `type_parameters` | `UmlTemplate` |
| `extends` | `Generalization` |
| `implements` | `Realization` |
| `annotation` | `UmlStereotype` (for known annotations) |

### Acceptance Criteria

- Parses Java 8-21 classes, interfaces, enums, records
- Handles: packages, imports, generics, annotations
- Test with sample Java files

---

## PHASE 11: PYTHON CODE IMPORT (2-3 weeks)

**Crates involved:** `uml-import-python`
**Dependencies:** Phase 8
**Risk:** Medium
**Parallelizable with:** Phase 9, 10

### Task Breakdown

#### 11.1 PythonImporter

```rust
pub struct PythonImporter {
    base: TreeSitterImporter,
    current_module: Option<String>,
}
```

#### 11.2 AST Node Mappings

| Python Construct | UML Element |
|-----------------|-------------|
| `module` | `UmlPackage` |
| `class_definition` | `UmlClass` |
| `function_definition` (in class) | `UmlOperation` |
| `assignment` (class-level) | `UmlAttribute` |
| `decorator` | `UmlStereotype` |
| `base_class` | `Generalization` |
| `type` annotation | Type reference |

### Acceptance Criteria

- Parses Python 3.8+ classes, methods, attributes, decorators
- Handles: type hints, module structure, inheritance
- Test with sample Python files

---

## PHASE 12: CODE GENERATION FRAMEWORK (2-3 weeks)

**Crates involved:** `uml-codegen`
**Dependencies:** Phase 2
**Risk:** Low
**Parallelizable with:** Phase 13, 14, 15

### Task Breakdown

#### 12.1 CodeGenerator Trait

```rust
pub trait CodeGenerator: Debug + Send + Sync {
    fn language(&self) -> ProgrammingLanguage;
    fn generate(&self, model: &ObjectRepository, config: &GenerationConfig) -> Result<Vec<GeneratedFile>, CodegenError>;
    fn generate_classifier(&self, classifier: ObjectIndex, repo: &ObjectRepository, config: &GenerationConfig)
        -> Result<Vec<GeneratedFile>, CodegenError>;
}

pub struct GeneratedFile { pub path: PathBuf, pub content: String }
pub struct GenerationConfig {
    pub output_dir: PathBuf, pub overwrite: bool,
    pub tab_size: usize, pub line_ending: LineEnding,
    pub header_comment: String,
}
```

#### 12.2 CodeWriter (`uml-codegen/src/code_writer.rs`)

```rust
pub struct CodeWriter { output: String, indent_level: usize, indent_str: String }

impl CodeWriter {
    pub fn new(indent_size: usize) -> Self;
    pub fn write(&mut self, text: &str) -> &mut Self;
    pub fn writeln(&mut self, text: &str) -> &mut Self;
    pub fn indent(&mut self);
    pub fn dedent(&mut self);
    pub fn newline(&mut self) -> &mut Self;
    pub fn block(&mut self, open: &str, close: &str, f: impl FnOnce(&mut Self));
    pub fn into_string(self) -> String;
}
```

#### 12.3 GeneratorRegistry

```rust
pub struct GeneratorRegistry { generators: HashMap<ProgrammingLanguage, Box<dyn CodeGenerator>> }

impl GeneratorRegistry {
    pub fn register(&mut self, gen: impl CodeGenerator + 'static);
    pub fn for_language(&self, lang: ProgrammingLanguage) -> Option<&dyn CodeGenerator>;
    pub fn languages(&self) -> Vec<ProgrammingLanguage>;
}
```

### Acceptance Criteria

- `CodeGenerator` trait complete
- `CodeWriter` produces correctly indented output
- `GeneratorRegistry` dispatches by language

---

## PHASE 13: C++ CODE GENERATION (3-4 weeks)

**Crates involved:** `uml-codegen-cpp`
**Dependencies:** Phase 12
**Risk:** High (C++ generation complexity)
**Parallelizable with:** Phase 14, 15

### Task Breakdown

#### 13.1 CppCodeGenerator

```rust
pub struct CppCodeGenerator { config: CppConfig }

pub struct CppConfig {
    pub header_extension: String, pub source_extension: String,
    pub generate_getters_setters: bool, pub generate_virtual_destructor: bool,
    pub include_guard_style: IncludeGuardStyle,
    pub use_smart_pointers: bool, pub generate_qt_meta: bool,
    pub tab_size: usize,
}
```

#### 13.2 Sub-Modules

- `header.rs` — `.h` file generation (class declaration, includes, forward decls)
- `source.rs` — `.cpp` file generation (constructor, destructor, method bodies)
- `associations.rs` — Association member variables and accessors
- `type_mapping.rs` — UML type → C++ type mapping
- `name_mangling.rs` — Reserved word escaping

### Acceptance Criteria

- Generates valid C++ headers and sources
- Handles: inheritance, attributes (public/private/protected), operations
- Handles: const, virtual, static, pure virtual, templates, enums, namespaces
- Associations: aggregation → unique_ptr, composition → value member
- Output matches C++ Umbrello CppWriter output
- Generated code compiles with g++/clang++

---

## PHASE 14: JAVA CODE GENERATION (2-3 weeks)

**Crates involved:** `uml-codegen-java`
**Dependencies:** Phase 12
**Risk:** Medium
**Parallelizable with:** Phase 13, 15

### Task Breakdown

#### 14.1 JavaCodeGenerator

```rust
pub struct JavaCodeGenerator { config: JavaConfig }

pub struct JavaConfig {
    pub package_prefix: String,
    pub generate_getters_setters: bool,
    pub use_lombok: bool,
    pub generate_javadoc: bool,
}
```

Capabilities: `.java` files for classes, interfaces, enums; generics; annotations;
Lombok `@Data`; JavaDoc; `extends`/`implements`.

### Acceptance Criteria

- Generates valid Java files that compile with `javac`
- Handles: packages, interfaces, generics, annotations
- Lombok annotation generation

---

## PHASE 15: PYTHON CODE GENERATION (1-2 weeks)

**Crates involved:** `uml-codegen-python`
**Dependencies:** Phase 12
**Risk:** Low
**Parallelizable with:** Phase 13, 14

### Task Breakdown

#### 15.1 PythonCodeGenerator

```rust
pub struct PythonCodeGenerator { config: PythonConfig }

pub struct PythonConfig {
    pub generate_type_hints: bool,
    pub use_dataclasses: bool,
    pub generate_docstrings: bool,
}
```

Capabilities: Python classes with proper indentation; type hints (PEP 484);
`@dataclass`; ABC + `@abstractmethod`; Enum; decorators.

### Acceptance Criteria

- Generates valid Python 3.8+ code (4-space indent, PEP 8)
- Type hints match annotation syntax
- `@dataclass` generation correct
- Generated code passes `py_compile` and `mypy --strict`

---

## PHASE 16: DIAGRAM MODEL (3-4 weeks)

**Crates involved:** `uml-diagram`
**Dependencies:** Phase 1, Phase 2
**Risk:** Medium
**Parallelizable with:** Phase 17

### Task Breakdown

#### 16.1 Diagram Metadata

```rust
pub struct Diagram {
    pub id: UmlId, pub name: String, pub diagram_type: DiagramType,
    pub zoom: f64, pub grid_settings: GridSettings,
    pub canvas_size: Size, pub background_color: Color,
}
pub struct GridSettings { pub spacing_x: f64, pub spacing_y: f64, pub visible: bool, pub snap_to_grid: bool }
pub enum GridStyle { Dots, Crosses, Lines }
```

#### 16.2 Widget Data

```rust
pub struct Widget {
    pub id: UmlId, pub local_id: UmlId, pub widget_type: WidgetType,
    pub position: Position, pub size: Size,
    pub visual: WidgetVisual, pub model_object_id: Option<UmlId>,
    pub properties: HashMap<String, String>,
}

pub enum WidgetType {
    Class, Interface, Enum, Datatype, Entity, Actor, UseCase,
    Package, Component, Node, Artifact, Instance, Object, Category, Port,
    Note, Box, State, Activity, Signal, ObjectNode,
    Region, Precondition, CombinedFragment, ForkJoin, Pin,
    FloatingText, FloatingDashLine, Message, Association,
}
```

#### 16.3 Edge Data

```rust
pub struct Edge {
    pub id: UmlId, pub source_id: UmlId, pub target_id: UmlId,
    pub uml_association_id: Option<UmlId>,
    pub points: Vec<Position>, pub layout_type: LayoutType,
    pub labels: Vec<EdgeLabel>,
}
pub struct EdgeLabel { pub text: String, pub position: Position, pub role: LabelRole }
```

#### 16.4 DiagramStore

```rust
pub struct DiagramStore {
    pub diagrams: HashMap<UmlId, Diagram>,
    pub widgets: HashMap<UmlId, Widget>,
    pub edges: HashMap<UmlId, Edge>,
}
impl DiagramStore { /* add/remove/query methods */ }
```

#### 16.5 WidgetFactory

Maps `(object_type, diagram_type) → WidgetType` with default visual properties.

### Acceptance Criteria

- All 29 widget types represented as data structs
- Widget factory creates correct types
- Edges store line points, labels, layout type
- Serialize/deserialize from XMI-compatible format
- Can reconstruct diagram from XMI data

---

## PHASE 17: LAYOUT ALGORITHMS (2-3 weeks)

**Crates involved:** `uml-layout`
**Dependencies:** Phase 16
**Risk:** Medium
**Parallelizable with:** Phase 18 (rendering with fixed positions)

### Task Breakdown

#### 17.1 Grid Snapping

```rust
pub struct GridSnapper { spacing_x: f64, spacing_y: f64 }
impl GridSnapper { fn snap_position(&self, pos: Position) -> Position; fn snap_size(&self, size: Size) -> Size; }
```

#### 17.2 Widget Alignment

```rust
pub struct AlignmentEngine;
impl AlignmentEngine {
    pub fn align_left(widgets: &mut [&mut Widget], target: Position);
    pub fn align_right(widgets: &mut [&mut Widget], target: Position);
    pub fn align_top(widgets: &mut [&mut Widget], target: Position);
    pub fn align_center_horizontal(widgets: &mut [&mut Widget], target_x: f64);
    pub fn distribute_horizontally(widgets: &mut [&mut Widget], left: f64, right: f64);
}
```

#### 17.3 Edge Snapping (Alignment Guides)

```rust
pub struct AlignmentGuide { threshold: f64 }
impl AlignmentGuide {
    pub fn snap_during_drag(&self, widget: &Widget, proposed: Position, others: &[&Widget]) -> (Position, Vec<GuideLine>);
}
```

#### 17.4 Auto-Layout (petgraph)

```rust
pub trait LayoutEngine {
    fn name(&self) -> &'static str;
    fn layout(&self, graph: &Graph<UmlId, AssociationType>, sizes: &HashMap<UmlId, Size>)
        -> Result<HashMap<UmlId, Position>, LayoutError>;
}
pub struct ForceDirectedLayout;  // Fruchterman-Reingold
pub struct LayeredLayout;        // Sugiyama for class diagrams
pub struct TreeLayout;           // For inheritance hierarchies
pub struct GraphvizBackend;      // External `dot` process
```

### Acceptance Criteria

- Grid snap rounds to nearest grid point
- Alignment engine works for all edges/centers
- Alignment guides detect and snap during drag
- Auto-layout produces reasonable arrangements
- Handles empty graphs, single nodes, disconnected components

---

## PHASE 18: DIAGRAM RENDERING (4-6 weeks)

**Crates involved:** `uml-render`
**Dependencies:** Phase 16, Phase 17
**Risk:** Very High (most complex single phase)
**Parallelizable with:** Nothing foundational

### Task Breakdown

#### 18.1 Renderer Trait

```rust
pub trait Renderer {
    fn begin_frame(&mut self, size: Size);
    fn end_frame(&mut self) -> Result<(), RenderError>;

    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, line_width: f32);
    fn fill_rounded_rect(&mut self, rect: Rect, radius: f64, color: Color);
    fn stroke_rounded_rect(&mut self, rect: Rect, radius: f64, color: Color, line_width: f32);
    fn fill_ellipse(&mut self, rect: Rect, color: Color);
    fn stroke_ellipse(&mut self, rect: Rect, color: Color, line_width: f32);
    fn fill_polygon(&mut self, points: &[Position], color: Color);
    fn stroke_polygon(&mut self, points: &[Position], color: Color, line_width: f32);
    fn stroke_line(&mut self, from: Position, to: Position, color: Color, line_width: f32);
    fn stroke_path(&mut self, path: &Path, color: Color, line_width: f32);
    fn fill_path(&mut self, path: &Path, color: Color);

    fn draw_text(&mut self, text: &str, pos: Position, font: &Font, color: Color, size: f64);
    fn text_size(&mut self, text: &str, font: &Font, size: f64) -> Size;

    fn set_transform(&mut self, transform: &AffineTransform);
    fn reset_transform(&mut self);
}
```

#### 18.2 Backends

- `TinySkiaRenderer` — CPU (always available)
- `VelloRenderer` — GPU (feature-gated, `wgpu` + `vello`)
- `SvgRenderer` — SVG export

#### 18.3 Canvas Widget

```rust
pub struct DiagramCanvas {
    pub diagram_id: UmlId,
    pub store: DiagramStoreRef,
    pub viewport: Viewport,
    pub renderer: Box<dyn Renderer>,
}

impl DiagramCanvas {
    pub fn render(&mut self) -> Result<(), RenderError>;
    // Individual widget renderers:
    fn render_classifier(&mut self, widget: &Widget);
    fn render_actor(&mut self, widget: &Widget);
    fn render_usecase(&mut self, widget: &Widget);
    fn render_note(&mut self, widget: &Widget);
    fn render_state(&mut self, widget: &Widget);
    fn render_activity(&mut self, widget: &Widget);
    // ...
    fn render_edge(&mut self, edge: &Edge);
    fn render_background(&mut self);
    fn render_grid(&mut self);
}
```

#### 18.4 Edge Rendering

```rust
impl DiagramCanvas {
    fn render_edge(&mut self, edge: &Edge) {
        let path = match edge.layout_type {
            LayoutType::Direct => Path::line(edge.points[0], edge.points[1]),
            LayoutType::Orthogonal => build_orthogonal_path(&edge.points),
            LayoutType::Polyline => Path::polyline(&edge.points),
            LayoutType::Spline => build_bezier_path(&edge.points),
        };
        self.renderer.stroke_path(&path, Color::BLACK, 2.0);
        self.render_arrowhead(edge.points.last(), edge.points[edge.points.len()-2]);
        for label in &edge.labels {
            self.renderer.draw_text(&label.text, label.position, &Font::default(), Color::BLACK, 10.0);
        }
    }
}
```

### Acceptance Criteria

- All widget types render with correct shapes
- Class boxes show compartments (name, attributes, operations)
- Actor stick figures, use case ellipses, note widgets
- Association lines render: straight, orthogonal, polyline, spline
- Arrowheads render at endpoints
- Grid background renders
- Zoom and pan with transform matrix
- Tiny-Skia backend produces pixel-identical output
- Vello GPU backend (optional) matches CPU output

---

## PHASE 19: INTERACTIVE EDITING (3-4 weeks)

**Crates involved:** `uml-render` (or `uml-interaction`)
**Dependencies:** Phase 18
**Risk:** High

### Task Breakdown

#### 19.1 Hit Testing

```rust
pub enum HitTarget {
    Widget(UmlId),
    Edge(UmlId),
    EdgePoint(UmlId, usize),
    EdgeSegment(UmlId, usize),
    ResizeHandle(UmlId, HandleType),
    None,
}

impl DiagramCanvas {
    pub fn hit_test(&self, pos: Position, threshold: f64) -> HitTarget;
}
```

#### 19.2 Selection State

```rust
pub struct SelectionState {
    pub selected_widgets: HashSet<UmlId>,
    pub selected_edges: HashSet<UmlId>,
    pub hovered: Option<HitTarget>,
    pub drag: Option<DragState>,
}

pub enum DragState {
    WidgetMove { widget_ids: HashSet<UmlId>, start_positions: Vec<Position>, offset: Position },
    WidgetResize { widget_id: UmlId, handle: HandleType, start_rect: Rect },
    EdgePoint { edge_id: UmlId, point_index: usize },
    RubberBand { origin: Position, current: Position },
}
```

#### 19.3 Mouse Event Handling

```rust
impl DiagramCanvas {
    pub fn handle_mouse_press(&mut self, pos: Position, button: MouseButton);
    pub fn handle_mouse_move(&mut self, pos: Position);
    pub fn handle_mouse_release(&mut self, pos: Position, button: MouseButton);
    pub fn handle_mouse_double_click(&mut self, pos: Position);
    pub fn handle_key(&mut self, key: KeyCode, state: ElementState);
}
```

### Acceptance Criteria

- Click selects widget (with visual highlight)
- Drag moves selected widgets
- Resize handles appear on selection, drag to resize
- Rubber-band selection (multi-select via drag on empty space)
- Edge point drag to reposition
- Double-click opens properties (stub for now, real dialog in Phase 20)
- Keyboard: Delete removes selection, arrows nudge

---

## PHASE 20: DESKTOP GUI APPLICATION (6-8 weeks)

**Crates involved:** `umbrello-desktop`
**Dependencies:** Phase 19, Phase 12-15, Phase 8-11
**Risk:** Very High (largest phase)
**Parallelizable with:** Nothing

### Task Breakdown

#### 20.1 Application Shell

Window using `winit` + `egui`:

```rust
pub struct App {
    context: AppContext,
    window: winit::window::Window,
    state: AppState,
}

pub struct AppContext {
    pub model: Arc<Mutex<ObjectRepository>>,
    pub diagram_store: Arc<Mutex<DiagramStore>>,
    pub undo_stack: Arc<Mutex<UndoStack>>,
    pub settings: Arc<RwLock<Settings>>,
    pub event_bus: EventBus,
    pub import_registry: ImportRegistry,
    pub generator_registry: GeneratorRegistry,
}
```

#### 20.2 UI Panels

- **Menu bar:** File, Edit, View, Diagram, Code, Settings, Help
- **Main toolbar:** New, Open, Save, Undo, Redo, Zoom controls
- **WorkToolBar:** Diagram element palette (context-sensitive)
- **Tree view** (`egui::SidePanel`): Model browser
- **Diagram tabs** (`egui::CentralPanel`): Tabbed diagram editor
- **Property editor** (`egui::SidePanel`): Widget/object properties
- **Documentation panel:** Text editor for documentation
- **Undo history** (`egui::SidePanel`): List of undoable actions
- **BirdView** (`egui::SidePanel`): Minimap
- **Log panel** (`egui::BottomPanel`): Event log
- **Status bar:** Messages, zoom level

#### 20.3 Dialogs

- Settings dialog (8 pages: General, UI, Diagram, Code Import, Code Gen, C++, Java, Python)
- Class properties dialog
- Association properties dialog
- Diagram properties dialog
- About dialog
- Code generation wizard

#### 20.4 Context Menus

Right-click on canvas/widgets with type-appropriate options:
- Canvas: Paste, Select All, Zoom, Layout
- Widget: Edit, Cut, Copy, Delete, Move to Front/Back
- Association: Edit, Delete, Change Type

### Acceptance Criteria

- File menu: New, Open, Save, Save As, Export (all working)
- Edit menu: Undo, Redo, Cut, Copy, Paste, Delete
- Model browser shows tree of packages/classifiers
- Diagram renders with all widgets and associations
- Drag & drop widget creation from toolbar
- Selection, move, resize, delete on widgets
- Properties dialog for classifiers
- Context menus functional
- Code generation from menu
- File import from menu (stub until Phase 21)

---

## PHASE 21: ADDITIONAL IMPORTERS (3-4 weeks ongoing)

**Crates involved:** `uml-import-{ada,pascal,idl,sql,cs,vala,php,js}` (individual crates)
**Dependencies:** Phase 8
**Risk:** Low-Medium
**Parallelizable with:** Each other, Phase 22

### Language Roadmap (priority order)

| Language | Crate | tree-sitter grammar | Est. time |
|----------|-------|---------------------|-----------|
| C# | `uml-import-cs` | `tree-sitter-c-sharp` | 2 days |
| SQL | `uml-import-sql` | `tree-sitter-sql` | 2 days |
| IDL | `uml-import-idl` | Manual lexer | 2 days |
| Ada | `uml-import-ada` | `tree-sitter-ada` | 2 days |
| Pascal | `uml-import-pascal` | `tree-sitter-pascal` | 2 days |
| Vala | `uml-import-vala` | `tree-sitter-vala` | 2 days |
| PHP | `uml-import-php` | `tree-sitter-php` | 2 days |
| JavaScript/TS | `uml-import-js` | `tree-sitter-typescript` | 2 days |
| Rust | `uml-import-rust` | `tree-sitter-rust` | 1 day |

Each follows the same pattern: `LanguageImporter` impl with tree-sitter grammar,
CST walker mapping constructs to UML model changes. Each is a separate crate
registering itself for its file extensions.

---

## PHASE 22: ADDITIONAL CODE GENERATORS (3-4 weeks ongoing)

**Crates involved:** `uml-codegen-{d,go,ruby,rust,js,...}` (individual crates)
**Dependencies:** Phase 12
**Risk:** Low-Medium
**Parallelizable with:** Each other, Phase 21

### Language Roadmap (priority order)

| Language | Crate | Est. time |
|----------|-------|-----------|
| D | `uml-codegen-d` | 2 days |
| Go | `uml-codegen-go` | 2 days |
| Ruby | `uml-codegen-ruby` | 2 days |
| JavaScript | `uml-codegen-js` | 2 days |
| Rust | `uml-codegen-rust` | 2 days |
| C# | `uml-codegen-cs` | 2 days |
| SQL | `uml-codegen-sql` | 1 day |
| PHP | `uml-codegen-php` | 1 day |
| Ada | `uml-codegen-ada` | 1 day |
| Pascal | `uml-codegen-pascal` | 1 day |

Each follows the same pattern: `CodeGenerator` impl using `CodeWriter`,
mapping UML classifiers to language-specific syntax, handling:
- Class/interface/enum declarations
- Attribute declarations
- Method/function declarations
- Inheritance/extensions
- Association member variables
- Reserved word escaping

---

## PHASE 23: FOREIGN FORMAT IMPORT (2-3 weeks)

**Crates involved:** `uml-persistence` (extensions)
**Dependencies:** Phase 4, Phase 3
**Risk:** Medium

### Task Breakdown

#### 23.1 ArgoUML Import

```rust
pub struct ArgoStorage;
impl StorageBackend for ArgoStorage { /* .zargo → extract ZIP → parse PGML → XMI */ }
```

#### 23.2 Rational Rose Import

```rust
pub struct RoseStorage;
impl StorageBackend for RoseStorage {
    /* .mdl → tokenize petal format → build UML model */
}
```

### Acceptance Criteria

- `.zargo` files load correctly
- `.mdl` files load correctly (basic constructs)
- Foreign format importers are registered in StorageRegistry

---

## PHASE 24: POLISH AND QUALITY (4-6 weeks)

**Crates involved:** All
**Dependencies:** All phases
**Risk:** Low (but time-consuming)

### Task Breakdown

#### 24.1 Performance Optimization

- Profile model operations with criterion benchmarks
- Profile XMI loading with large files (10MB+)
- Profile diagram rendering with 1000+ widgets
- Profile code generation for large models
- Optimize hot paths: arena lookups, text rendering, layout loops

#### 24.2 Memory Profiling

- Reduce memory footprint of large models
- Implement arena compression for infrequently accessed data
- Profile with dhat/heaptrack

#### 24.3 Rendering Quality

- Anti-aliasing improvements
- Font rendering consistency across platforms
- Color and style matching with C++ Umbrello output
- High-DPI display support

#### 24.4 Accessibility

- Keyboard navigation
- Screen reader support (aria labels on canvas elements)
- Color contrast for color-blind users
- Configurable font sizes

#### 24.5 Documentation

- **Developer docs:** Architecture guide, crate reference, contribution guide
- **User docs:** Feature overview, CLI reference, FAQ
- **API docs:** Complete rustdoc with examples

#### 24.6 Packaging

- Linux: `.deb` (Debian/Ubuntu), `.rpm` (Fedora), AppImage
- Windows: MSI installer (WiX), portable .exe
- macOS: `.dmg`

#### 24.7 Bug Fixes

- Address all issues found during beta testing
- Fix edge cases in XMI parsing
- Fix code generation for unusual UML models
- Fix rendering artifacts
- Fix undo/redo interactions

### Acceptance Criteria

- All clippy warnings eliminated
- Benchmark suite runs with published results
- 1000+ widget model loads in <2 seconds
- 1000+ widget diagram renders at >30 FPS
- Documentation builds without warnings
- Packaging scripts produce installable artifacts
- Zero known critical bugs

---

## Risk Summary Table

| Phase | Risk | Key Risk Factors | Mitigation |
|-------|------|------------------|------------|
| 0 | Low | None | — |
| 1 | Low | Enum variant naming | Serde rename attributes |
| 2 | Medium | Circular refs, large scope | ID-based refs, slotmap |
| 3 | Medium | Inverse change correctness | Exhaustive unit tests |
| 4 | High | XMI format compliance | Round-trip tests, C++ comparison |
| 5 | Low | File format edge cases | Test with malformed files |
| 6 | Medium | Inverse correctness | Snapshot-based deletion undos |
| 7 | Low | Simple dispatch | Clap validation |
| 8 | Medium | Tree-sitter API stability | Version pin in Cargo.lock |
| 9 | High | C++ syntax complexity | Skip complex constructs first |
| 10 | Medium | Modern Java features | Handle records/sealed classes |
| 11 | Medium | Dynamic typing | Graceful type hint degradation |
| 12 | Low | Simple framework | — |
| 13 | High | C++ generation complexity | Compare char-by-char with C++ |
| 14 | Medium | Lombok features | Feature-gate Lombok support |
| 15 | Low | Simple syntax | — |
| 16 | Medium | 29 widget types | Systematic test per type |
| 17 | Medium | Convergence sensitivity | Parameter tuning, Graphviz fallback |
| 18 | Very High | GPU rendering, text layout | TinySkia CPU fallback, cosmic-text |
| 19 | High | Event state machine correctness | Exhaustive mouse event tests |
| 20 | Very High | Largest phase, many components | Modularize, feature-gate |
| 21 | Low-Medium | Grammar quality | Test with real-world files |
| 22 | Low-Medium | Language syntax differences | CodeWriter abstraction |
| 23 | Medium | Proprietary format docs | Reverse-engineer from C++ code |
| 24 | Low | Time-consuming but safe | Prioritize by user impact |

---

## Parallelization Strategy

The following phases can be parallelized across independent agents:

```
Agent 1: Phases 0 → 1 → 2 → 3 → 6 (model foundation + undo)
Agent 2: Phases 0 → 1 → 2 → 4 → 5 (persistence)
Agent 3: Phase 7 (CLI, after phase 5)
Agent 4: Phases 0 → 1 → 2 → 8 → 9 → 10 → 11 (importers)
Agent 5: Phases 0 → 1 → 2 → 12 → 13 → 14 → 15 (generators)
Agent 6: Phases 0 → 1 → 2 → 16 → 17 (diagram data + layout)
Agent 7: Phases 18 → 19 → 20 (rendering, interaction, GUI)
Agent 8: Phases 21, 22 (additional importers/generators)
Agent 9: Phase 23 (foreign import)
Agent 10: Phase 24 (polish)
```

Optimal team size: 3-4 developers (model+persistence, importers, generators,
rendering+GUI). With 4 developers, estimated completion: 9-12 months.

---

## Testing Strategy Per Phase

| Phase | Unit Tests | Integration Tests | Round-trip Tests | Comparison Tests |
|-------|-----------|------------------|------------------|------------------|
| 0 | — | Build succeeds | — | — |
| 1 | Serde round-trip per enum | — | — | — |
| 2 | Builder pattern, repo CRUD | Programmatic model | Serde JSON round-trip | — |
| 3 | Mutation → inverse → original | Event emission | — | — |
| 4 | XMI reader per dialect | Load all test files | Load → Save → Load | XML structure diff |
| 5 | Storage backend per format | File save → load | — | — |
| 6 | push→undo→redo sequence | Macro grouping | — | — |
| 7 | CLI argument parsing | Load file via CLI | — | — |
| 8 | Tree-sitter parse | — | — | — |
| 9 | Parse C++ per construct | Import real project | — | C++ Umbrello output |
| 10 | Parse Java per construct | Import real project | — | C++ Umbrello output |
| 11 | Parse Python per construct | Import real project | — | C++ Umbrello output |
| 12 | CodeWriter indentation | — | — | — |
| 13 | Gen per construct | Generate full model | — | C++ Umbrello output |
| 14 | Gen per construct | Generate full model | — | C++ Umbrello output |
| 15 | Gen per construct | Generate full model | — | C++ Umbrello output |
| 16 | Widget factory, store CRUD | — | — | — |
| 17 | Grid snap, alignment, layout | Auto-layout diagrams | — | Graphviz output |
| 18 | Renderer per shape | Render all widget types | — | C++ screenshot comparison |
| 19 | Hit test, selection, drag | Mouse event sequence | — | — |
| 20 | Dialog creation, menu dispatch | Full app launch | — | C++ feature parity |
| 21 | Parse per construct | Import real project | — | C++ Umbrello output |
| 22 | Gen per construct | Generate full model | — | C++ Umbrello output |
| 23 | Parse foreign format | Load test files | — | C++ Umbrello output |
| 24 | — | Full regression | All XMI round-trip | All output diff |

---

## Appendices

### A. Reference: C++ ↔ Rust Type Mapping

| C++ UMLObject Subclass | Rust Struct | ObjectType | File |
|------------------------|-------------|------------|------|
| `UMLClass` | `UmlClass` | `Class` | `class.rs` |
| `UMLInterface` | `UmlInterface` | `Interface` | `interface.rs` |
| `UMLEnum` | `UmlEnum` | `Enum` | `enumeration.rs` |
| `UMLDatatype` | `UmlDatatype` | `Datatype` | `datatype.rs` |
| `UMLEntity` | `UmlEntity` | `Entity` | `entity.rs` |
| `UMLPackage` | `UmlPackage` | `Package` | `package.rs` |
| `UMLFolder` | `UmlFolder` | `Folder` | `folder.rs` |
| `UMLComponent` | `UmlComponent` | `Component` | `component.rs` |
| `UMLArtifact` | `UmlArtifact` | `Artifact` | `artifact.rs` |
| `UMLNode` | `UmlNode` | `Node` | `node.rs` |
| `UMLPort` | `UmlPort` | `Port` | `port.rs` |
| `UMLActor` | `UmlActor` | `Actor` | `actor.rs` |
| `UMLUseCase` | `UmlUseCase` | `UseCase` | `usecase.rs` |
| `UMLCategory` | `UmlCategory` | `Category` | `category.rs` |
| `UMLInstance` | `UmlInstance` | `Instance` | `instance.rs` |
| `UMLAssociation` | `UmlAssociation` | `Association` | `association.rs` |
| `UMLRole` | `UmlRole` | `Role` | `association.rs` |
| `UMLAttribute` | `UmlAttribute` | `Attribute` | `attribute.rs` |
| `UMLEntityAttribute` | `UmlEntityAttribute` | `EntityAttribute` | `attribute.rs` |
| `UMLOperation` | `UmlOperation` | `Operation` | `operation.rs` |
| `UMLTemplate` | `UmlTemplate` | `Template` | `template.rs` |
| `UMLEnumLiteral` | `UmlEnumLiteral` | `EnumLiteral` | `enumeration.rs` |
| `UMLStereotype` | `UmlStereotype` | `Stereotype` | `stereotype.rs` |
| `UMLInstanceAttribute` | `UmlInstanceAttribute` | `InstanceAttribute` | `instance.rs` |
| `UMLUniqueConstraint` | `UmlUniqueConstraint` | `UniqueConstraint` | `constraint.rs` |
| `UMLForeignKeyConstraint` | `UmlForeignKeyConstraint` | `ForeignKeyConstraint` | `constraint.rs` |
| `UMLCheckConstraint` | `UmlCheckConstraint` | `CheckConstraint` | `constraint.rs` |

### B. Crate Dependency Graph

```
uml-core         — no deps (except serde, thiserror, bitflags, slotmap, uuid)
uml-xmi          — uml-core (+ quick-xml)
uml-persistence  — uml-xmi (+ flate2, tar)
uml-undo         — uml-core
uml-cli          — uml-persistence (+ clap)
uml-import       — uml-core (+ tree-sitter)
uml-import-cpp   — uml-import (+ tree-sitter-cpp)
uml-import-java  — uml-import (+ tree-sitter-java)
uml-import-python— uml-import (+ tree-sitter-python)
uml-codegen      — uml-core (+ tera/askama optional)
uml-codegen-cpp  — uml-codegen
uml-codegen-java — uml-codegen
uml-codegen-python— uml-codegen
uml-diagram      — uml-core
uml-layout       — uml-diagram (+ petgraph)
uml-render       — uml-diagram (+ tiny-skia, cosmic-text, wgpu/vello optional)
umbrello-desktop — everything (+ winit, egui)
xtask            — no deps
```

### C. Key Design Decisions

1. **Enum-based dispatch over trait objects** for ModelElement. The enum
   approach gives serialization (serde), exhaustive pattern matching, and
   avoids downcasting.

2. **ID-based references over pointers.** All cross-object references use
   `UmlId`. This breaks cycles, enables serialization, and simplifies
   undo/redo (snapshots don't need pointer invalidation).

3. **SlotMap arena over Vec.** SlotMap provides O(1) with generational
   safety — stale keys are detected, preventing dangling reference bugs.

4. **Streaming XML over DOM.** XMI reader uses quick-xml Events (SAX-style)
   to avoid loading entire documents into memory.

5. **Command pattern for undo/redo** (not event sourcing). Simpler to
   implement and reason about. Event sourcing can be added later if needed.

6. **Renderer trait with multiple backends.** CPU (tiny-skia) always
   available; GPU (vello) as optional feature. SVG export as a third backend.

7. **CodeWriter for code generation** (not template engine). Offers full
   control over formatting. Templates (tera) can be added as an alternative
   path.

8. **Tree-sitter for all importers.** Avoids hand-written parsers. Each
   language gets a consistent CST walking pattern.

9. **egui for GUI** (not QT bindings). Pure Rust, immediate mode, cross-
   platform, no C++ compilation needed. Faster iteration than cxx-qt.

10. **AppContext replacing UMLApp singleton.** Dependency injection through
    `AppContext` struct with trait-based accessors. No global state.
