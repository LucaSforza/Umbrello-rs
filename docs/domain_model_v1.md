# Domain Model v1 — Umbrello-RS UML Metamodel

> **Document:** `rust-rewrite/docs/domain_model_v1.md`
> **Status:** Active
> **Phase:** Milestone 3 (uml-core model types)
> **Last updated:** 2026-06-23
>
> This document defines the Rust-native UML domain model that replaces the C++
> inheritance hierarchy. It covers the design principles, type architecture,
> ownership model, and comparison with the legacy approach.

---

## Table of Contents

1. [Context: The C++ Inheritance Problem](#1-context-the-c-inheritance-problem)
2. [Rust Design Principles](#2-rust-design-principles)
3. [The Design](#3-the-design)
   - [3.1 UmlId](#31-umlid)
   - [3.2 ElementBase](#32-elementbase)
   - [3.3 The NamedElement Trait](#33-the-namedelement-trait)
   - [3.4 ClassifierData — Composition over Inheritance](#34-classifierdata--composition-over-inheritance)
   - [3.5 Concrete Element Types](#35-concrete-element-types)
   - [3.6 The ModelElement Enum](#36-the-modelelement-enum)
   - [3.7 Package as Container](#37-package-as-container)
   - [3.8 ModelRepository — The Arena](#38-modelrepository--the-arena)
4. [Comparison with C++](#4-comparison-with-c)
5. [What This Design Enables](#5-what-this-design-enables)
6. [What Is NOT in v1](#6-what-is-not-in-v1)
7. [Future Considerations](#7-future-considerations)

---

## 1. Context: The C++ Inheritance Problem

### 1.1 The Legacy Hierarchy

The C++ codebase models UML elements using a deep single-inheritance tree rooted at
`QObject`:

```
QObject
 └── UMLObject                              # root: id, name, visibility,
     │                                      #   stereotype, documentation,
     │                                      #   abstract/static flags
     ├── UMLCanvasObject                    # adds m_List for subordinates,
     │   │                                  #   association-end management
     │   └── UMLPackage                     # adds m_objects for contained
     │       │                              #   standalone objects
     │       └── UMLClassifier              # adds attributes, operations,
     │           │                          #   templates, ClassifierType enum
     │           └── UMLEnum                # adds enum literals
     │
     ├── UMLClassifierListItem              # abstract: parent of attribute,
     │   │                                  #   operation, template, enum literal
     │   ├── UMLAttribute
     │   ├── UMLOperation
     │   ├── UMLTemplate
     │   └── UMLEnumLiteral
     │
     ├── UMLAssociation                     # owns 2 UMLRole objects
     │
     └── UMLStereotype
```

### 1.2 Known Problems

This design suffers from five structural issues that the Rust rewrite must avoid:

**1. Fragile base class problem (5 levels of inheritance).**

A change to `UMLObject` (adding a field, changing a constructor signature)
ripples through all 20+ subclasses. `UMLCanvasObject::m_List` is a `QList<UMLObject*>`
that every canvas-aware subclass inherits, even those that never use it.

```cpp
// UMLObject has 28 isUML*()/asUML*() methods — manual RTTI.
// Adding a new element type requires adding two new methods to the root class.
class UMLObject {
    virtual bool isUMLClass() const;     // + 27 more like this
    virtual UMLClass* asUMLClass();      // + 27 more like this
};
```

**2. Twenty-eight `isUML*()` / `asUML*()` manual RTTI methods on `UMLObject`.**

Every concrete type gets a boolean query and a cast method on the root base class.
This is the Visitor pattern implemented badly — every new type requires modifying
the root.

```cpp
// A sampling of the 28 is/as methods on UMLObject:
virtual bool isUMLClass() const;
virtual bool isUMLInterface() const;
virtual bool isUMLEnum() const;
virtual bool isUMLPackage() const;
// ... 24 more ...
virtual UMLClass* asUMLClass();
virtual UMLInterface* asUMLInterface();
// ...
```

**3. Dual ownership hierarchy (QObject parent vs UMLPackage containment).**

Every `UMLObject` is simultaneously owned by:
- A `QObject` parent (Qt memory management tree)
- A `UMLPackage::m_objects` list (model containment)

This dual ownership makes it unclear which reference owns the object's lifetime.
Destruction order bugs are common.

```cpp
// Both of these manage lifetime — conflicting:
class UMLObject : public QObject { /* QObject parent */ };
class UMLPackage : public UMLCanvasObject {
    UMLObjectList m_objects;  // also owns these objects
};
```

**4. `m_pSecondary` field — "only used by a few classes" (source comment).**

The base class `UMLObject` carries a `UMLObject* m_pSecondary` field that is
only meaningful for `UMLRole` (to point at the `UMLAssociation`). The source
code acknowledges this is a design smell:

```cpp
// umlobject.h, line ~145:
// m_pSecondary: This member variable is used by some classes
// (not all).  Maybe it could be moved into each specific class
// that uses it.
UMLObject* m_pSecondary;
```

**5. Association stored via `m_List` on `UMLCanvasObject` instead of role pointers.**

Associations are stored as a flat list on every canvas object, not as dedicated
role references on the association itself. The source code has an explicit TODO:

```cpp
// TODO: Move the list of Associations to the UMLAssociation class itself.
// It is stored here in UMLCanvasObject only for historical reasons.
UMLAssociationList m_List;
```

**6. Twelve separate list-type headers for `QList<T*>`.**

Each concrete type gets its own typedef header file:

```
umbrello/umlassociationlist.h   → typedef QList<UMLAssociation*>
umbrello/umlattributelist.h     → typedef QList<UMLAttribute*>
umbrello/umlclassifierlist.h    → typedef QList<UMLClassifier*>
umbrello/umlentitylist.h        → typedef QList<UMLEntity*>
umbrello/umlenumlist.h          → typedef QList<UMLEnum*>
umbrello/umloperationlist.h     → typedef QList<UMLOperation*>
umbrello/umlpackagelist.h       → typedef QList<UMLPackage*>
umbrello/umlrolelist.h          → typedef QList<UMLRole*>
umbrello/uml stereotyyypelist.h → typedef QList<UMLStereotype*>
umbrello/umltemplatelist.h      → typedef QList<UMLTemplate*>
umbrello/umlwidgetlist.h        → typedef QList<UMLWidget*>
umbrello/umlentityattributelist.h → etc.
```

---

## 2. Rust Design Principles

The domain model v1 follows five core principles:

### Principle 1: No Inheritance Emulation

Rust does not have class inheritance, and we do not emulate it. Instead:

- **Traits** provide shared behaviour across unrelated types (`NamedElement`).
- **Enums** provide type dispatch without casting (`ModelElement`).
- **Composition** embeds shared data directly in structs (`ClassifierData`).

```rust
// BAD — Rust does not have inheritance, and we don't want it:
// struct Package { base: ElementBase }  // ✅ composition
// NOT: impl Package: ElementBase        // ❌ inheritance emulation
```

### Principle 2: Flat Type Hierarchy

Instead of a 5-deep class tree, we have a single enum with flat variants:

```rust
enum ModelElement {
    Package(Package),
    Class(Class),
    // ... more variants at the same level
}
```

There is exactly one level of nesting. Adding a new type means adding a variant
to `ModelElement` and a new struct — not extending a class hierarchy.

### Principle 3: Shared Data via Composition

Common metadata lives in `ElementBase`, a plain struct embedded in each
concrete type:

```rust
struct Class {
    base: ElementBase,       // embedded, not inherited
    classifier: ClassifierData,  // reused by Interface, Enum, Datatype
    // type-specific fields...
}
```

### Principle 4: Type-Safe Dispatch

Rust's `match` replaces both `dynamic_cast` and the manual `isUML*()` / `asUML*()`
method family:

```rust
// C++: if (obj->isUMLClass()) { UMLClass* c = obj->asUMLClass(); ... }
// Rust:
match element {
    ModelElement::Class(c) => { /* c is &Class, statically known */ }
    ModelElement::Interface(i) => { /* i is &Interface */ }
    _ => {} // exhaustive — compiler checks all variants
}
```

### Principle 5: ID-Based References

Elements reference each other by `UmlId`, not raw pointers. This enables:

- No dangling pointers (IDs are validated on lookup)
- Easy serialization (IDs are plain UUIDs, not memory addresses)
- Generational index safety (via `SlotMap` in `ModelRepository`)

```rust
struct Package {
    base: ElementBase,
    children: Vec<UmlId>,  // references to child elements
}
```

---

## 3. The Design

### 3.1 UmlId

Every model element has a unique, immutable identifier. `UmlId` wraps a UUID v4
and provides:

- `new()` — generate a new unique ID
- `default()` — same as `new()` (every default is unique)
- `Serialize` / `Deserialize` — serde support for JSON and XMI
- `Display` / `FromStr` — UUID string format (e.g., `"550e8400-e29b-41d4-a716-446655440000"`)
- `Eq`, `Ord`, `Hash` — value semantics

```rust
/// A unique identifier for a model element.
///
/// Wraps a UUID v4. Every `UmlId` is globally unique.
/// IDs are never reused — even after an element is deleted, its ID is retired.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UmlId(#[serde(with = "uuid::serde::urn")] uuid::Uuid);
```

IDs are the backbone of the reference system. No element stores a Rust reference
to another element — only `UmlId` values.

```
┌─────────────────────┐
│      Package        │
│  ┌───────────────┐  │
│  │ ElementBase   │  │
│  │  id: UmlId    │──│──── "550e8400-..."
│  │  name: "Root" │  │
│  └───────────────┘  │
│  children:          │
│    [UmlId A,        │── references to child elements
│     UmlId B,        │
│     UmlId C]        │
└─────────────────────┘
```

### 3.2 ElementBase

All model elements share a common set of metadata fields. In C++ these live on
`UMLObject` and are inherited by every subclass. In Rust they are a standalone
struct embedded in each concrete element:

```rust
/// Common metadata shared by all model elements.
///
/// Embedded (not inherited) into every concrete element type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementBase {
    /// Globally unique identifier (never changes after creation).
    pub id: UmlId,

    /// Human-readable name (e.g., "Person", "Customer", "main()").
    pub name: String,

    /// UML visibility: public, protected, private, implementation.
    pub visibility: Visibility,

    /// Optional reference to a stereotype element.
    pub stereotype_id: Option<UmlId>,

    /// Free-form documentation text.
    pub documentation: String,

    /// Whether this element is abstract (applies to classes, operations).
    pub is_abstract: bool,

    /// Whether this element is static (applies to attributes, operations).
    pub is_static: bool,
}
```

Every concrete element type embeds this struct as its first field. In Rust,
embedding means the struct contains an `ElementBase` value directly — there is
no indirection, no allocation, and no virtual dispatch:

```
Memory layout (no indirection):
┌──────────────────────────────────────┐
│ Class                                 │
│  ├── ElementBase      (inline)        │
│  │    ├── id: UmlId                  │
│  │    ├── name: String               │
│  │    ├── visibility: Visibility     │
│  │    ├── stereotype_id: Option<Id>  │
│  │    ├── documentation: String      │
│  │    ├── is_abstract: bool          │
│  │    └── is_static: bool            │
│  ├── ClassifierData  (inline)        │
│  │    ├── attributes: Vec<Attribute> │
│  │    ├── operations: Vec<Operation> │
│  │    └── templates: Vec<Template>   │
│  └── (type-specific fields...)       │
└──────────────────────────────────────┘
```

### 3.3 The NamedElement Trait

To provide uniform access to common properties without inheritance, a trait
defines the accessor protocol:

```rust
/// Trait providing uniform access to common element properties.
///
/// Implemented by `ModelElement` (the enum) and delegating to the inner `ElementBase`.
pub trait NamedElement {
    /// Borrow the embedded `ElementBase`.
    fn base(&self) -> &ElementBase;

    /// Mutably borrow the embedded `ElementBase`.
    fn base_mut(&mut self) -> &mut ElementBase;

    /// The UML object type discriminator.
    fn object_type(&self) -> ObjectType;

    // --- Provided methods (derived from base()) ---

    fn id(&self) -> UmlId {
        self.base().id
    }

    fn name(&self) -> &str {
        &self.base().name
    }

    fn set_name(&mut self, name: String) {
        self.base_mut().name = name;
    }

    fn visibility(&self) -> Visibility {
        self.base().visibility
    }

    fn set_visibility(&mut self, v: Visibility) {
        self.base_mut().visibility = v;
    }

    fn is_abstract(&self) -> bool {
        self.base().is_abstract
    }

    fn is_static(&self) -> bool {
        self.base().is_static
    }

    fn documentation(&self) -> &str {
        &self.base().documentation
    }
}
```

This trait is implemented on `ModelElement` (the enum) via a single match:

```rust
impl NamedElement for ModelElement {
    fn base(&self) -> &ElementBase {
        match self {
            ModelElement::Package(p) => &p.base,
            ModelElement::Class(c) => &c.base,
            ModelElement::Interface(i) => &i.base,
            ModelElement::Enum(e) => &e.base,
        }
    }

    fn base_mut(&mut self) -> &mut ElementBase {
        match self {
            ModelElement::Package(p) => &mut p.base,
            ModelElement::Class(c) => &mut c.base,
            ModelElement::Interface(i) => &mut i.base,
            ModelElement::Enum(e) => &mut e.base,
        }
    }

    fn object_type(&self) -> ObjectType {
        match self {
            ModelElement::Package(_) => ObjectType::Package,
            ModelElement::Class(_) => ObjectType::Class,
            ModelElement::Interface(_) => ObjectType::Interface,
            ModelElement::Enum(_) => ObjectType::Enumeration,
        }
    }
}
```

This replaces all 28 `isUML*()` / `asUML*()` methods with a single match.

```
C++:
  obj->isUMLClass()      → true/false
  obj->asUMLClass()      → UMLClass* (or nullptr)
  obj->getDocumentation()→ QString
  Need 28 methods on root.

Rust:
  element.object_type() == ObjectType::Class  → true/false
  if let ModelElement::Class(c) = element     → &Class
  element.documentation()                     → &str
  One match on ModelElement. No root methods.
```

### 3.4 ClassifierData — Composition over Inheritance

In C++, `UMLClassifier` is a base class between `UMLPackage` and concrete types
like `UMLClass`, `UMLInterface`, `UMLEnum`. It exists solely to share the
attributes list, operations list, and template parameters.

In Rust, this shared data becomes a standalone struct:

```rust
/// Data shared by classifier-like elements.
///
/// Embedded (not inherited) by Class, Interface, Enum, and Datatype.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassifierData {
    /// Attributes owned by this classifier.
    pub attributes: Vec<Attribute>,

    /// Operations owned by this classifier.
    pub operations: Vec<Operation>,

    /// Template parameters (generic type parameters).
    pub templates: Vec<TemplateParameter>,
}
```

Elements that are classifiers embed this struct. Elements that are not classifiers
do not carry this data — no wasted memory, no irrelevant fields.

```
Composition diagram:

    Class            Interface          Enum
  ┌────────┐       ┌────────┐       ┌────────┐
  │ base   │       │ base   │       │ base   │
  ├────────┤       ├────────┤       ├────────┤
  │ cd:    │       │ cd:    │       │ cd:    │
  │ Class- │       │ Class- │       │ Class- │
  │ ifier  │       │ ifier  │       │ ifier  │
  │ Data   │       │ Data   │       │ Data   │
  ├────────┤       ├────────┤       ├────────┤
  │        │       │        │       │literals│
  │        │       │        │       │ Vec<   │
  │        │       │        │       │ Enum-  │
  │        │       │        │       │ Literal│
  └────────┘       └────────┘       └────────┘

    Package           Attribute        Operation
  ┌────────┐       ┌────────┐       ┌────────┐
  │ base   │       │ base   │       │ base   │
  ├────────┤       ├────────┤       ├────────┤
  │children│       │ type_  │       │ return_ │
  │Vec<Id> │       │ id     │       │ type   │
  │        │       │        │       │ params │
  └────────┘       └────────┘       └────────┘
  (no ClassifierData)                (no ClassifierData)
```

### 3.5 Concrete Element Types

Each element type is a standalone struct. They share no base class — only common
fields via embedded `ElementBase`.

```rust
/// A UML Package (namespace/container for model elements).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Package {
    pub base: ElementBase,
    pub children: Vec<UmlId>,
}

/// A UML Class.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Class {
    pub base: ElementBase,
    pub classifier: ClassifierData,
}

/// A UML Interface.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Interface {
    pub base: ElementBase,
    pub classifier: ClassifierData,
}

/// A UML Enumeration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enum {
    pub base: ElementBase,
    pub classifier: ClassifierData,
    /// Enum-specific literals (additional to any attributes).
    pub literals: Vec<EnumLiteral>,
}

/// A classifier attribute (field / member variable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name.
    pub name: String,
    /// Type of the attribute (reference to a UML type by ID).
    pub type_id: Option<UmlId>,
    /// Type name (fallback when the type is not a UML model element).
    pub type_name: Option<String>,
    /// Visibility.
    pub visibility: Visibility,
    /// Initial value expression.
    pub initial_value: Option<String>,
    /// Whether the attribute is static (class-level).
    pub is_static: bool,
}

/// A classifier operation (method).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    /// Operation name.
    pub name: String,
    /// Return type (reference to a UML type by ID).
    pub return_type_id: Option<UmlId>,
    /// Return type name (fallback).
    pub return_type_name: Option<String>,
    /// Formal parameters.
    pub parameters: Vec<Parameter>,
    /// Visibility.
    pub visibility: Visibility,
    /// Whether the operation is static (class-level).
    pub is_static: bool,
    /// Whether the operation has no implementation.
    pub is_abstract: bool,
    /// Whether the operation is virtual / overridable.
    pub is_virtual: bool,
}

/// An operation parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name.
    pub name: String,
    /// Parameter type (reference to a UML type by ID).
    pub type_id: Option<UmlId>,
    /// Parameter type name (fallback).
    pub type_name: Option<String>,
    /// Parameter direction (in, out, inout, return).
    pub direction: ParameterDirection,
    /// Default value expression.
    pub default_value: Option<String>,
}

/// A template / generic type parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateParameter {
    /// Parameter name (e.g. `T`, `K`, `V`).
    pub name: String,
    /// Type constraint (e.g. `class`, `Comparable`).
    pub constraint: Option<String>,
}

/// An enumeration literal value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnumLiteral {
    /// Literal name.
    pub name: String,
    /// Optional explicit value (e.g. `= 42`).
    pub value: Option<String>,
}
```

Key observations:

- **No `m_pSecondary` equivalent.** Each type only has the fields it needs.
  If a field is needed by multiple types, it goes into `ElementBase` or
  `ClassifierData`. If it is only needed by one type, it stays on that type.
- **Pure value types for subordinates.** `Attribute`, `Operation`,
  `Parameter`, `TemplateParameter`, and `EnumLiteral` do **not** embed
  `ElementBase` — they are pure value types owned by their classifier.
  They have no independent `UmlId` and are not addressable from the
  `ModelRepository` arena. This matches UML semantics (features of a
  classifier, not standalone model elements) and avoids unnecessary
  repository overhead.
- **No `m_List` on canvas objects.** There is no canvas object layer in v1.
  The arena handles all storage.
- **No list-type headers.** `Vec<Attribute>`, `Vec<Operation>`, etc. are
  just `Vec` — no typedefs needed.

#### Dual-Reference Type Pattern

`Attribute`, `Operation`, and `Parameter` use a dual-reference pattern
for their types:

- **`type_id: Option<UmlId>`** — references a UML model element (class,
  interface, enum, datatype) when the type is a modeled type.
- **`type_name: Option<String>`** — a string fallback for primitive types
  (`int`, `String`, `bool`, `float`) and external types that have no
  corresponding UML model element. When `type_id` is `Some`, it takes
  precedence for UML-aware tooling.

This pattern mirrors the C++ codebase where `UMLClassifierListItem` stores
a plain `QString` type name alongside the optional UML classifier reference.

Both fields can be `None` when the type is unspecified (e.g., a forward
reference or unresolved import). At least one should be set when the type
is known.

### 3.6 The ModelElement Enum

All concrete element types are variants of a single enum. This enum is the
primary handle for element storage, dispatch, and serialization.

```rust
/// All UML model element types in a single tagged union.
///
/// Enables:
/// - Flat storage in a single arena (no pointer chasing)
/// - Pattern matching for type-safe dispatch
/// - Serialization as a tagged union (serde adjacently tagged)
/// - Iteration without dynamic dispatch
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),

    // Future Milestones:
    // Datatype(Datatype),
    // Actor(Actor),
    // UseCase(UseCase),
    // Component(Component),
    // Node(Node),
    // Artifact(Artifact),
    // Instance(Instance),
    // Association(Association),
    // ...
}
```

Serde serialization produces a tagged JSON format:

```json
{
    "type": "Class",
    "base": { "id": "...", "name": "Person", ... },
    "classifier": { "attributes": [...], "operations": [...] }
}
```

The enum is the **sole dispatch mechanism**. No `dyn NamedElement` trait objects
are needed. Every operation on model elements is a `match`:

```rust
impl ModelElement {
    /// Return the `ObjectType` discriminant for this element.
    pub fn object_type(&self) -> ObjectType {
        match self {
            Self::Package(_) => ObjectType::Package,
            Self::Class(_) => ObjectType::Class,
            Self::Interface(_) => ObjectType::Interface,
            Self::Enum(_) => ObjectType::Enumeration,
        }
    }

    /// Return a shared reference to the element's base metadata.
    pub fn base(&self) -> &ElementBase {
        match self {
            Self::Package(p) => &p.base,
            Self::Class(c) => &c.base,
            Self::Interface(i) => &i.base,
            Self::Enum(e) => &e.base,
        }
    }

    /// Return a mutable reference to the element's base metadata.
    pub fn base_mut(&mut self) -> &mut ElementBase {
        match self {
            Self::Package(p) => &mut p.base,
            Self::Class(c) => &mut c.base,
            Self::Interface(i) => &mut i.base,
            Self::Enum(e) => &mut e.base,
        }
    }

    /// The unique identifier of this element.
    pub fn id(&self) -> UmlId {
        self.base().id
    }

    /// The human-readable name.
    pub fn name(&self) -> &str {
        &self.base().name
    }

    /// Set the name.
    pub fn set_name(&mut self, name: String) {
        self.base_mut().name = name;
    }

    /// Check if this element is a classifier (has attributes/operations).
    pub fn is_classifier(&self) -> bool {
        matches!(self, Self::Class(_) | Self::Interface(_) | Self::Enum(_))
    }

    /// Check if this element is a package.
    pub fn is_package(&self) -> bool {
        matches!(self, Self::Package(_))
    }

    /// Return a reference to the classifier data, if this is a classifier.
    pub fn classifier_data(&self) -> Option<&ClassifierData> {
        match self {
            Self::Class(c) => Some(&c.classifier),
            Self::Interface(i) => Some(&i.classifier),
            Self::Enum(e) => Some(&e.classifier),
            Self::Package(_) => None,
        }
    }

    /// Return a mutable reference to the classifier data, if this is a classifier.
    pub fn classifier_data_mut(&mut self) -> Option<&mut ClassifierData> {
        match self {
            Self::Class(c) => Some(&mut c.classifier),
            Self::Interface(i) => Some(&mut i.classifier),
            Self::Enum(e) => Some(&mut e.classifier),
            Self::Package(_) => None,
        }
    }
}
```

### 3.7 Package as Container

A `Package` owns its children by storing their `UmlId`s. The actual element data
lives in the `ModelRepository` arena — the package only knows the IDs.

```rust
impl Package {
    /// Create a new package with the given name.
    pub fn new(name: String) -> Self {
        Self {
            base: ElementBase {
                id: UmlId::new(),
                name,
                visibility: Visibility::Public,
                stereotype_id: None,
                documentation: String::new(),
                is_abstract: false,
                is_static: false,
            },
            children: Vec::new(),
        }
    }

    /// Add a child element by ID.
    pub fn add_child(&mut self, child_id: UmlId) {
        self.children.push(child_id);
    }

    /// Remove a child element by ID. Returns true if found and removed.
    pub fn remove_child(&mut self, child_id: UmlId) -> bool {
        let idx = self.children.iter().position(|&id| id == child_id);
        if let Some(i) = idx {
            self.children.swap_remove(i);
            true
        } else {
            false
        }
    }

    /// Iterate over child IDs.
    pub fn children(&self) -> &[UmlId] {
        &self.children
    }

    /// Find a child by name (linear search over children, resolved via repository).
    pub fn find_child_by_name<'a>(
        &self,
        name: &str,
        repo: &'a ModelRepository,
    ) -> Option<&'a ModelElement> {
        self.children
            .iter()
            .filter_map(|id| repo.get(*id))
            .find(|elem| elem.name() == name)
    }

    /// Check if a child with the given ID exists in this package.
    pub fn contains(&self, child_id: UmlId) -> bool {
        self.children.contains(&child_id)
    }
}
```

The package does not store elements inline — it only stores references by ID.
This means:

- Removing an element from a package does not deallocate it (the repository
  still holds it until explicitly removed).
- An element can belong to multiple packages (shared containment, e.g., via
  imported elements).
- The package is lightweight — it can be cloned cheaply (IDs are `Copy`).

```
ModelRepository (SlotMap<UmlId, ModelElement>)
┌──────────────────────────────────────────────────┐
│  UmlId A  →  Package { name: "Root",             │
│                       children: [B, C, D] }       │
│                                                   │
│  UmlId B  →  Class { name: "Person",             │
│                      attributes: [E, F] }          │
│                                                   │
│  UmlId C  →  Class { name: "Address",            │
│                      attributes: [G] }             │
│                                                   │
│  UmlId D  →  Interface { name: "Serializable"    │
│                          operations: [H] }         │
│                                                   │
│  UmlId E  →  Attribute { name: "name",           │
│                          type_id: Some(...) }      │
│  UmlId F  →  Attribute { name: "age", ... }       │
│  ...                                              │
└──────────────────────────────────────────────────┘
          ▲                    ▲
          │                    │
          └── Package.children ┘  (Vec<UmlId>)
```

### 3.8 ModelRepository — The Arena

The `ModelRepository` is a generational slot map that owns all model elements.
It provides O(1) access by `UmlId`, safe iteration, and lifecycle management.

```rust
/// The central storage for all model elements.
///
/// Uses a `SlotMap` (generational index) internally for:
/// - O(1) insert / get / remove
/// - No dangling references (generational keys detect use-after-free)
/// - Cache-friendly iteration (elements stored contiguously)
#[derive(Debug, Clone)]
pub struct ModelRepository {
    elements: SlotMap<UmlId, ModelElement>,
}

impl ModelRepository {
    /// Create an empty repository.
    pub fn new() -> Self { /* ... */ }

    /// Insert an element, returning its assigned key.
    pub fn insert(&mut self, element: ModelElement) -> UmlId { /* ... */ }

    /// Get a reference to an element by ID.
    pub fn get(&self, id: UmlId) -> Option<&ModelElement> { /* ... */ }

    /// Get a mutable reference to an element by ID.
    pub fn get_mut(&mut self, id: UmlId) -> Option<&mut ModelElement> { /* ... */ }

    /// Remove an element by ID. Returns the element if it existed.
    pub fn remove(&mut self, id: UmlId) -> Option<ModelElement> { /* ... */ }

    /// Iterate over all elements.
    pub fn iter(&self) -> impl Iterator<Item = (UmlId, &ModelElement)> { /* ... */ }

    /// Number of elements in the repository.
    pub fn len(&self) -> usize { /* ... */ }

    /// Returns true if the repository is empty.
    pub fn is_empty(&self) -> bool { /* ... */ }
}
```

Key properties:

- **Elements own their children.** A `Package` owns `Vec<UmlId>`, but the
  actual `ModelElement` data is owned by the repository. The package does not
  need `Arc` or `Rc` — it just names IDs.
- **Removal is safe.** Because IDs are generational, a removed element's ID
  will never collide with a new element's ID.
- **No circular references in v1.** There are no associations (cross-references)
  in v1, so we avoid the problem of reference cycles. When associations arrive
  in Milestone 4, they will use `UmlId` references (weak by nature — no cycle
  in ownership).

---

## 4. Comparison with C++

### 4.1 Table of Patterns

| C++ Pattern | Rust Equivalent |
|---|---|
| 5-deep class inheritance | Single enum with 4 variants (v1) |
| 28 `isUML*()` / `asUML*()` methods | One `match` on `ModelElement` |
| `m_pSecondary` on root class (used by few) | Explicit `Option<UmlId>` on specific types |
| `QObject` parent for memory management | `SlotMap` arena + generational IDs |
| `Clone()` / `copyInto()` virtual methods | `#[derive(Clone)]` on all structs |
| `QList<T*>` with manual ownership | `Vec<T>` with clear ownership |
| 12 list-type headers (`umlassociationlist.h`, ...) | Plain `Vec<Attribute>`, `Vec<Operation>`, ... |
| `saveToXMI()` / `loadFromXMI()` on every class | External `XmiSerializer` (separate crate) |
| `UMLClassifier` base class for shared data | Embedded `ClassifierData` struct |
| `UMLObjectList` (QList<UMLObject*>) with heterogeneous types | `Vec<ModelElement>` (homogeneous enum) |
| Manual RTTI via `isUMLClass()` / `dynamic_cast` | `matches!(elem, ModelElement::Class(_))` |
| `m_List` on `UMLCanvasObject` for all associations | Association model in future milestone |

### 4.2 Structural Comparison

```
C++ class tree (simplified):        Rust composition (simplified):

QObject                              (no root object)
 └── UMLObject (28 RTTI methods)     ElementBase (plain struct)
     ├── UMLCanvasObject             (no canvas object in v1)
     │   ├── add UMLPackage          Package { base, children }
     │   │   └── UMLClassifier        (no classifier base)
     │   │       ├── UMLClass        Class { base, cd }
     │   │       ├── UMLInterface    Interface { base, cd }
     │   │       └── UMLEnum         Enum { base, cd, literals }
     │   ├── UMLAttribute            Attribute { base, type_id }
     │   ├── UMLOperation            Operation { base, return_type_id, params }
     │   └── UMLTemplate             TemplateParameter { base, type_id }
     └── UMLAssociation              (Milestone 4)
       UMLRole                       (Milestone 4)

Count of isUML*() methods: 28       match arms: 4 (grows 1:1 with variants)
List types (typedef headers): 12    Vec specializations: 0 (built-in)
Fields on root class: ~15           Fields on ElementBase: 7
```

### 4.3 Ownership Comparison

```
C++ ownership (dual):

  UMLPackage::m_objects
    │
    ├── UMLClass*  ──────────────── QObject parent (Qt tree)
    ├── UMLInterface* ───────────── QObject parent (Qt tree)
    └── ...
          │
          └── Also reachable via UMLObject::parent() — dual path

  Problem: Who owns the memory? Two valid answers. Destructor order bug.

Rust ownership (single):

  ModelRepository (SlotMap)
    │
    ├── ModelElement::Class  ────── owned by slot map
    ├── ModelElement::Interface ─── owned by slot map
    └── ...
          │
          └── Referenced by Package::children via UmlId — not ownership

  Problem: None. Arena owns everything. IDs are just references.
  Removal: repo.remove(id) — the SlotMap frees the slot.
  Safety: generational key prevents use-after-free.
```

### 4.4 Adding a New Element Type

```
C++: Steps to add UMLFoo:

  1. Add UMLFoo to class hierarchy (choose parent)
  2. Create umlfoo.h with virtual method declarations
  3. Create umlfoo.cpp with implementations
  4. Add isUMLFoo() and asUMLFoo() to UMLObject (root class)
  5. Add UMLFoo to all relevant switch/if-else chains
  6. Create umlfoolist.h (QList<UMLFoo*> typedef)
  7. Add XMI serialization to umlfoolist.cpp
  8. Add to any factory methods
  9. Add to any visitor/traversal code
  Risk: Forgetting step 4 breaks root class contract.

Rust: Steps to add Foo:

  1. Create struct Foo { base: ElementBase, ... }
  2. Add Foo(Foo) variant to ModelElement
  3. Add match arm to NamedElement impl (base/base_mut)
  4. Add match arm to ModelElement::object_type()
  Risk: Compiler errors on every non-exhaustive match — cannot forget.
```

### 4.5 Error Handling

```
C++:
  UMLObject* obj = factory(type);
  if (!obj) { /* handle error — but no error type */ }
  // Factory returns nullptr on failure — callers must check.

  // Many operations are void:
  void UMLObject::setName(const QString& name);
  // No validation feedback.

Rust:
  fn try_set_name(&mut self, name: String) -> Result<(), ValidationError> {
      if name.is_empty() {
          return Err(ValidationError::EmptyName);
      }
      self.base.name = name;
      Ok(())
  }

  // Construction always succeeds (no nullptr):
  let cls = Class::new("Person".to_string());
  // Builder pattern for validation:
  let cls = Class::builder()
      .name("Person")
      .build()?;  // Returns Result
```

### 4.6 Summary of Changes

| Concern | C++ | Rust |
|---|---|---|
| Type dispatch | 28 virtual methods on root | One `match` on enum |
| Shared data | Base class inheritance | Embedded struct composition |
| Ownership | Dual (QObject + UMLPackage) | Single (SlotMap arena) |
| References | Raw pointers | ID-based (UmlId) |
| Memory safety | Manual (QObject parent tree) | Compiler-enforced (borrow checker + arena) |
| Serialization | Hand-written XML on each class | Derive + external XMI crate |
| Adding types | Modify root class, create 3+ files | Add variant + 2 match arms |
| Testing | Integration/functional only | Unit tests on every type + property tests |
| Cloning | Virtual clone() method | `#[derive(Clone)]` |

---

## 5. What This Design Enables

### Fast Iteration

`ModelRepository` uses a `SlotMap` (generational index-based storage) that
provides O(1) access by key, cache-friendly iteration, and safe removal:

```rust
// O(1) insert
let id = repo.insert(ModelElement::Class(my_class));

// O(1) get
let elem = repo.get(id).unwrap();

// O(1) remove
let removed = repo.remove(id).unwrap();

// Cache-friendly iteration over all elements
for (id, elem) in repo.iter() {
    // ...
}
```

### Safe References

Generational indices prevent use-after-free. If code retains an `UmlId` for an
element that has been removed, `repo.get(id)` returns `None` instead of a
dangling pointer. The SlotMap's generation counter increments on each removal,
so even if a new element occupies the same slot, the old key will not match.

### Serialization

Serde derive on all types means JSON round-tripping is free. XMI serialization
will be handled by a separate `uml-xmi` crate that maps between XMI XML format
and the serde model — this is explicit, testable, and decoupled from the domain
types.

```rust
// JSON round-trip (free with derive):
let json = serde_json::to_string_pretty(&element).unwrap();
let deserialized: ModelElement = serde_json::from_str(&json).unwrap();
assert_eq!(element, deserialized);
```

### Pattern Matching

Rust's `match` is exhaustive — adding a new variant to `ModelElement` will
produce compiler errors at every `match` that handles `ModelElement`. This
replaces the fragile C++ pattern where adding a type requires auditing every
`if/else if` chain and every switch statement in the codebase.

### Clear Ownership

- The `ModelRepository` owns all element data (single source of truth).
- `Package` stores `Vec<UmlId>` — references, not ownership.
- `ClassifierData`'s `attributes`, `operations`, `templates` are `Vec<T>` —
  inline owned data.
- No `Rc<RefCell<T>>`, no `Arc<Mutex<T>>`, no shared ownership in v1.

### Extensibility

Adding a new element type in v2+ is mechanical:

1. Define the struct
2. Add variant to `ModelElement`
3. Add two match arms (`NamedElement::base` and `object_type`)
4. Done

```
ModelElement v1 (4 variants):          ModelElement v2 (+3 variants):
┌────────────────────┐                ┌──────────────────────┐
│ Package            │                │ Package              │
│ Class              │                │ Class                │
│ Interface          │                │ Interface            │
│ Enum               │                │ Enum                 │
└────────────────────┘                │ Datatype             │ ← new
                                       │ Actor                │ ← new
                                       │ UseCase              │ ← new
                                       └──────────────────────┘
```

Each addition is a local change (one file) with compiler-checked propagation.

---

## 6. What Is NOT in v1

The following are deliberately excluded from domain model v1. They will be added
in future milestones once the core infrastructure is solid.

| Feature | Milestone | Rationale |
|---------|-----------|-----------|
| Associations and relationships | 4 | Requires bidirectional role references, constraint validation, and XMI compatibility testing. Adds significant complexity. |
| Stereotypes as first-class entities | 5 | Stereotypes need a registry/ontology pattern. In v1, `stereotype_id: Option<UmlId>` is a placeholder. |
| XMI serialization | 4 | Provided by `uml-xmi` crate. Domain model does not depend on XMI format. |
| Diagram model | 16 | `uml-diagram` crate. Diagrams reference model elements but are not model elements themselves. |
| Undo/redo system | 6 | `uml-undo` crate. Commands operate on `ModelRepository`, not on individual elements. |
| Folder (specialized Package with diagram ownership) | 17 | Folder is a Package that also owns diagrams. Requires diagram model to exist first. |
| Datatype | 3 (minor) | Could easily be added in v1.2 — it is structurally identical to `Class` but with `ObjectType::Datatype`. Deferred to keep v1 scope minimal. |
| Entity (ERD) | 8+ | Entity-Relationship diagram support. Adds `Entity`, `EntityAttribute`, constraints. |
| Actor | 9+ | Use case diagrams. Structurally lightweight — just `ElementBase`. |
| UseCase | 9+ | Use case diagrams. Like Actor, structurally simple. |
| Component | 10+ | Component/diagram support. |
| Node | 10+ | Deployment diagram. |
| Artifact | 10+ | Deployment diagram. |
| Instance | 11+ | Object diagram. |
| InstanceAttribute | 11+ | Object diagram — attribute values on instances. |
| Constraint types | 8+ | `UniqueConstraint`, `ForeignKeyConstraint`, `CheckConstraint` for ERD. |
| Association types | 4 | 14+ association subtypes (aggregation, composition, generalization, dependency, etc.). |

### Why keep v1 small

1. **Validation of architecture.** The composition + enum + arena design needs
   to be validated with a small set of types before scaling up to 27+
   element types and 14+ association types.

2. **Package and Class cover 80% of use cases.** Most UML models are
   `Package` → `Class` → `Attribute` / `Operation`. If this core works
   correctly, the remaining types are mechanical additions.

3. **XMI compatibility driven by Class.** The critical round-trip test is
   loading a C++ XMI file with classes, attributes, and operations. Once that
   works, adding new types is incremental.

4. **Avoid premature abstraction.** Types like `Actor` and `UseCase` are
   structurally identical to `Class` with a different `ObjectType`. Rather
   than building a generic "element factory" system in v1, we add each type
   explicitly and let the pattern emerge.

---

## 7. Future Considerations

### 7.1 Associations (Milestone 4)

Associations introduce bidirectional references between elements. The v1 design
handles this naturally with `UmlId`:

```rust
// Future: association model (Milestone 4)
struct Association {
    base: ElementBase,
    association_type: AssociationType,
    role_a: Role,
    role_b: Role,
}

struct Role {
    base: ElementBase,
    /// The element at this end of the association (class, interface, etc.).
    participant_id: UmlId,
    /// Multiplicity string (e.g., "0..*", "1", "0..1").
    multiplicity: String,
    /// Whether navigation from this end is supported.
    is_navigable: bool,
    /// Aggregation kind: none, shared (aggregation), composite.
    aggregation: AggregationKind,
}
```

No circular ownership — `Role` references `participant_id` by `UmlId`, not by
pointer. The repository owns everything.

### 7.2 Stereotype Registry (Milestone 5)

In v1, stereotypes are referenced by `Option<UmlId>` in `ElementBase`. A future
`StereotypeRegistry` will manage stereotype definitions (profiles, tag
definitions, icon paths):

```rust
// Future: stereotype registry (Milestone 5)
struct Stereotype {
    base: ElementBase,
    /// Tag definitions (key-value pairs) for this stereotype.
    tags: Vec<TagDefinition>,
    /// Optional reference to a base stereotype (profile hierarchy).
    base_stereotype_id: Option<UmlId>,
}

struct TagDefinition {
    name: String,
    value_type: String,  // "String", "Integer", "Boolean", etc.
    default_value: String,
}
```

### 7.3 Diagram Model (Phase 16)

Diagrams reference model elements but are not themselves model elements:

```rust
// Future: diagram model (Phase 16, separate crate)
struct Diagram {
    name: String,
    diagram_type: DiagramType,
    widgets: Vec<Widget>,
}

enum Widget {
    ClassWidget { element_id: UmlId, position: Point, size: Size },
    NoteWidget { text: String, position: Point },
    // ...
}
```

### 7.4 Query API

As the model grows, a query API layer on top of `ModelRepository` will provide
convenience methods without bloating the core types:

```rust
// Future: query extension trait
trait ModelQuery {
    fn find_classes_by_name(&self, pattern: &str) -> Vec<UmlId>;
    fn find_all_associations(&self, element_id: UmlId) -> Vec<UmlId>;
    fn find_all_children_recursive(&self, package_id: UmlId) -> Vec<UmlId>;
    fn count_elements_by_type(&self) -> HashMap<ObjectType, usize>;
}
```

### 7.5 Event System (Phase 2/3)

Model mutations should emit events so that the undo system and UI can react:

```rust
// Future: model modification events
enum ModelEvent {
    ElementInserted { id: UmlId },
    ElementRemoved { id: UmlId },
    ElementModified { id: UmlId, changed_fields: Vec<String> },
    ChildAdded { parent_id: UmlId, child_id: UmlId },
    ChildRemoved { parent_id: UmlId, child_id: UmlId },
}
```

The v1 design keeps events out of the core types — `ModelRepository` can be
wrapped in an event-emitting layer without changing the domain model.

---

## References

- [C++ class hierarchy](https://invent.kde.org/system/umbrello/-/blob/master/umbrello/umlobject.h) — `UMLObject` root class
- [C++ basic types](https://invent.kde.org/system/umbrello/-/blob/master/umbrello/basictypes.h) — `ObjectType`, `Visibility`, etc.
- [Testing strategy](./testing_strategy.md) — Umbrello-RS testing approach
- [Crate boundary review](./crate_boundary_review.md) — Workspace organization
