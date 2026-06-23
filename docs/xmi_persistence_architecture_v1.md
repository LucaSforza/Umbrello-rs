# XMI Persistence Architecture v1 — XMI Reader Design

> **Document:** `rust-rewrite/docs/xmi_persistence_architecture_v1.md`
> **Status:** Active
> **Phase:** Milestone 8 (XMI Persistence — Reader Implementation)
> **Last updated:** 2026-06-23
>
> This document defines the architecture for reading legacy Umbrello C++ XMI
> files (UML 1.2 format) and populating our `UmlModel` repository. It covers
> format analysis, parser technology evaluation, the two-pass parsing strategy,
> the proposed API, error handling, and a staged implementation plan.
>
> **Scope:** XMI **reader** only. The XMI writer (serializing `UmlModel` back
> to XMI) is a separate task (M9+). This document covers Milestone 8 (M8):
> structural element parsing — `<UML:Class>`, `<UML:Package>`,
> `<UML:Interface>`, `<UML:Enum>`. Relationships, attributes, operations,
> stereotypes, and diagram data are deferred to M9+.

---

## Table of Contents

1. [Context: The XMI Format Landscape](#1-context-the-xmi-format-landscape)
2. [Target XMI Format (UML 1.2)](#2-target-xmi-format-uml-12)
   - [2.1 Document Structure](#21-document-structure)
   - [2.2 Element Syntax](#22-element-syntax)
   - [2.3 Attribute Reference Style](#23-attribute-reference-style)
   - [2.4 Containment Semantics](#24-containment-semantics)
   - [2.5 Stereotype System](#25-stereotype-system)
   - [2.6 Key Observations](#26-key-observations)
3. [Domain Model Mapping](#3-domain-model-mapping)
   - [3.1 Current Rust Domain Model](#31-current-rust-domain-model)
   - [3.2 XMI-to-Domain Mapping Table](#32-xmi-to-domain-mapping-table)
   - [3.3 Visibility Mapping](#33-visibility-mapping)
   - [3.4 Architectural Mismatches](#34-architectural-mismatches)
4. [ID Mapping Strategy](#4-id-mapping-strategy)
   - [4.1 Problem Statement](#41-problem-statement)
   - [4.2 Chosen Strategy: HashMap Bridge](#42-chosen-strategy-hashmap-bridge)
   - [4.3 The IdMap Type](#43-the-idmap-type)
   - [4.4 Forward Reference Handling](#44-forward-reference-handling)
5. [Parser Technology Evaluation](#5-parser-technology-evaluation)
   - [5.1 Candidates](#51-candidates)
   - [5.2 Evaluation Matrix](#52-evaluation-matrix)
   - [5.3 Recommendation: quick-xml](#53-recommendation-quick-xml)
6. [Two-Pass Parsing Strategy](#6-two-pass-parsing-strategy)
   - [6.1 Why Two Passes?](#61-why-two-passes)
   - [6.2 Pass 1: Structural Element Extraction](#62-pass-1-structural-element-extraction)
   - [6.3 Pass 2: Cross-Reference Resolution](#63-pass-2-cross-reference-resolution)
   - [6.4 M8 Scope vs Deferred Items](#64-m8-scope-vs-deferred-items)
7. [Proposed Module Structure](#7-proposed-module-structure)
   - [7.1 Current Status](#71-current-status)
   - [7.2 Proposed Structure](#72-proposed-structure)
   - [7.3 Rationale for Location](#73-rationale-for-location)
8. [Detailed Parsing Algorithm](#8-detailed-parsing-algorithm)
   - [8.1 Initialization](#81-initialization)
   - [8.2 XML Event Loop](#82-xml-event-loop)
   - [8.3 State Machine](#83-state-machine)
   - [8.4 Element Creation](#84-element-creation)
   - [8.5 Containment Tracking](#85-containment-tracking)
   - [8.6 Resolve References (Pass 2)](#86-resolve-references-pass-2)
   - [8.7 Pseudocode](#87-pseudocode)
9. [Proposed API](#9-proposed-api)
   - [9.1 Error Types](#91-error-types)
   - [9.2 XmiReader Struct](#92-xmireader-struct)
   - [9.3 XmiVersion Detection](#93-xmiversion-detection)
   - [9.4 Helper Traits](#94-helper-traits)
10. [Error Handling Strategy](#10-error-handling-strategy)
    - [10.1 Error Categories](#101-error-categories)
    - [10.2 Recovery Strategy](#102-recovery-strategy)
    - [10.3 Logging](#103-logging)
11. [Test Plan](#11-test-plan)
    - [11.1 Unit Tests (M8)](#111-unit-tests-m8)
    - [11.2 Integration Tests (M8)](#112-integration-tests-m8)
    - [11.3 Test Fixtures](#113-test-fixtures)
    - [11.4 Acceptance Criteria](#114-acceptance-criteria)
12. [Staged Implementation Plan](#12-staged-implementation-plan)
    - [12.1 Stage 1: Skeleton + XMI Detection](#121-stage-1-skeleton--xmi-detection)
    - [12.2 Stage 2: Pass 1 — Basic Elements](#122-stage-2-pass-1--basic-elements)
    - [12.3 Stage 3: Containment Hierarchy](#123-stage-3-containment-hierarchy)
    - [12.4 Stage 4: Pass 2 — Reference Resolution](#124-stage-4-pass-2--reference-resolution)
    - [12.5 Stage 5: Edge Cases & Real Files](#125-stage-5-edge-cases--real-files)
13. [What Is NOT in M8](#13-what-is-not-in-m8)
14. [Future Considerations (M9+)](#14-future-considerations-m9)
15. [References](#15-references)

---

## 1. Context: The XMI Format Landscape

The C++ Umbrello serialises its UML model to XMI (XML Metadata Interchange) —
the OMG standard for exchanging UML models. There are two major versions:

| Aspect | UML 1.2 (XMI 1.2) | UML 2.x (XMI 2.1) |
|--------|-------------------|-------------------|
| **Namespace** | `xmlns:UML="http://schema.omg.org/spec/UML/1.3"` | `xmlns:uml="http://schema.omg.org/spec/UML/2.1"` |
| **ID attribute** | `xmi.id` (dot notation) | `xmi:id` (colon notation) |
| **Containment** | `<UML:Namespace.ownedElement>` | `<packagedElement>` |
| **Stereotypes** | Inline `xmi.id` references | `xmi:value` / `instance` |
| **Extensions** | `<XMI.extensions>` with `xmi.extender="umbrello"` | Same pattern |
| **Umbrello default** | Produces UML 1.2 by default | Alternative export |

**The C++ codebase writes UML 1.2 XMI as its primary format.** All existing
`.xmi` files in the wild are UML 1.2. We MUST support reading this format as
our first priority. XMI 2.1 reading can be added later.

The C++ `saveToXMI` / `loadFromXMI` pattern places serialisation logic on every
model class — an anti-pattern we avoid by keeping `uml-core` format-agnostic.

---

## 2. Target XMI Format (UML 1.2)

### 2.1 Document Structure

```xml
<?xml version="1.0" encoding="UTF-8"?>
<XMI verified="false" xmi.version="1.2" xmlns:UML="http://schema.omg.org/spec/UML/1.3">
 <XMI.header>
  <XMI.documentation>
   <XMI.exporter>umbrello uml modeller http://umbrello.kde.org</XMI.exporter>
  </XMI.documentation>
 </XMI.header>
 <XMI.content>
  <!-- === MODEL HIERARCHY === -->
  <UML:Model xmi.id="m1" name="UML Model" isSpecification="false"
             isAbstract="false" isLeaf="false" isRoot="false">
   <UML:Namespace.ownedElement>
    <!-- Stereotype definitions -->
    <UML:Stereotype xmi.id="folder" name="folder" visibility="public"/>
    <!-- View-level models (Logical View, Use Case View, etc.) -->
    <UML:Model stereotype="folder" xmi.id="Logical View" name="Logical View"
               visibility="public">
     <UML:Namespace.ownedElement>
      <UML:Package stereotype="folder" xmi.id="Datatypes" name="Datatypes">
       <UML:Namespace.ownedElement>
        <UML:DataType xmi.id="wWhl5MFkeCF5" name="int"/>
       </UML:Namespace.ownedElement>
      </UML:Package>
      <UML:Class xmi.id="O0JJV24XoKdQ" name="ClassTopLeft"
                 visibility="public" isAbstract="false"/>
      <UML:Class xmi.id="M0AM2eGNh5ho" name="ClassTopMid"
                 visibility="public" isAbstract="false"/>
     </UML:Namespace.ownedElement>
    </UML:Model>
   </UML:Namespace.ownedElement>
  </UML:Model>
 </XMI.content>
 <!-- === UMBRELLO EXTENSIONS === -->
 <XMI.extensions xmi.extender="umbrello">
  <docsettings viewid="..." uniqueid="..." documentation=""/>
  <diagrams>
   <!-- Widget data, edges, positions — SKIP in M8 -->
  </diagrams>
 </XMI.extensions>
</XMI>
```

### 2.2 Element Syntax

Every model element is an XML element with the `UML:` namespace prefix:

```xml
<UML:Class
  xmi.id="O0JJV24XoKdQ"      <!-- REQUIRED: string ID -->
  name="ClassTopLeft"          <!-- REQUIRED: element name -->
  visibility="public"          <!-- OPTIONAL: default "public" -->
  isAbstract="false"           <!-- OPTIONAL: default "false" -->
  isLeaf="false"               <!-- OPTIONAL: default "false" -->
  isRoot="false"               <!-- OPTIONAL: default "false" -->
  isActive="false"             <!-- OPTIONAL: default "false" -->
  stereotype="folder"          <!-- OPTIONAL: reference to Stereotype xmi.id -->
/>
```

Elements may be self-closing (`/>`) or have child elements. Child elements
include `UML:Classifier.feature` (for attributes and operations) and
`UML:Namespace.ownedElement` (for nesting).

### 2.3 Attribute Reference Style

All cross-references in UML 1.2 use XMI string IDs as attribute values:

| Attribute | Type | Example |
|-----------|------|---------|
| `xmi.id` | Element's own ID | `xmi.id="O0JJV24XoKdQ"` |
| `stereotype` | Reference to a Stereotype's `xmi.id` | `stereotype="folder"` |
| `namespace` | Reference to parent element's `xmi.id` | `namespace="m1"` (rare in Umbrello) |
| `type` | Reference to a DataType's `xmi.id` | `type="wWhl5MFkeCF5"` (on attributes) |

These string IDs are **not** UUIDs. They are short, human-readable identifiers
generated by the C++ `UMLObject::createChildId()` method, which concatenates
a base ID with a counter.

### 2.4 Containment Semantics

Containment in UML 1.2 uses the `<UML:Namespace.ownedElement>` wrapper:

```xml
<UML:Package xmi.id="p1" name="Parent">
  <UML:Namespace.ownedElement>
    <UML:Class xmi.id="c1" name="Child"/>
    <UML:Class xmi.id="c2" name="Child2"/>
  </UML:Namespace.ownedElement>
</UML:Package>
```

The wrapping element `<UML:Namespace.ownedElement>` is a **structural feature**
of the UML metamodel, not a separate model element. It signals that the
contained elements are owned by the parent namespace. Our parser must:
1. Detect entering a `<UML:Package>` or `<UML:Model>` element.
2. Track it as the "current parent" on a stack.
3. When encountering child elements inside `<UML:Namespace.ownedElement>`,
   associate them with the parent.

### 2.5 Stereotype System

Stereotypes in Umbrello XMI files are **inline definitions** with their own
`xmi.id`, referenced by other elements via the `stereotype` attribute:

```xml
<UML:Stereotype xmi.id="folder" name="folder" visibility="public"/>
<UML:Package stereotype="folder" xmi.id="Datatypes" name="Datatypes"/>
```

The stereotype `xmi.id="folder"` is defined at the top of the file, and
multiple elements can reference it via `stereotype="folder"`.

In M8, we **capture the reference as a pending string** (`stereotype_xmi_id`)
and **defer resolution** to Pass 2. The actual `UML:Stereotype` element parsing
is deferred to M9.

### 2.6 Key Observations

1. **`xmi.id` not `xmi:id`** — UML 1.2 uses dot notation. This is the primary
   format we target.
2. **String IDs are concise** — `"O0JJV24XoKdQ"`, `"m1"`, `"folder"`. They are
   not UUIDs. Our domain model uses UUID-backed `UmlId`.
3. **Stereotypes are defined inline** — always before they are referenced
   (in practice), but we cannot rely on ordering.
4. **`<UML:Model>` elements represent views** — "Logical View", "Use Case View",
   etc. We treat them as `Package` elements in M8.
5. **`<UML:DataType>` elements represent primitive types** — "int", "bool",
   "string", etc. These are deferred to M9 (primitive type registry).
6. **`<XMI.extensions>` contains Umbrello-specific data** — `docsettings`,
   `diagrams`, `listview`, `codegeneration`. Entirely skipped in M8.
7. **Names are not unique** — XMI does not require unique names within a
   namespace. Our domain model does not enforce uniqueness either.

---

## 3. Domain Model Mapping

### 3.1 Current Rust Domain Model

```rust
// uml-core/src/elements.rs
pub struct ElementBase {
    pub id: UmlId,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype_id: Option<UmlId>,
    pub documentation: String,
    pub is_abstract: bool,
    pub is_static: bool,
}

pub struct Package {
    pub base: ElementBase,
    pub(crate) children: Vec<UmlId>,   // IDs of contained elements
}

pub struct Class {
    pub base: ElementBase,
    pub classifier: ClassifierData,    // attributes, operations, templates
}

pub struct Interface {
    pub base: ElementBase,
    pub classifier: ClassifierData,
}

pub struct Enum {
    pub base: ElementBase,
    pub classifier: ClassifierData,
    pub literals: Vec<EnumLiteral>,
}

pub enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
    Relationship(Relationship),
}

// uml-core/src/repository.rs
pub struct UmlModel {
    elements: IndexMap<UmlId, ModelElement>,
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

### 3.2 XMI-to-Domain Mapping Table

| XMI Tag | Domain Type | Notes |
|---------|-----------|-------|
| `<UML:Model>` (root) | `Package` | Root "UML Model" becomes top-level Package. Views ("Logical View") become nested Packages. |
| `<UML:Model>` (view) | `Package` | Always has `stereotype="folder"`. Treated as a folder/package. |
| `<UML:Package>` | `Package` | Direct 1:1 mapping. |
| `<UML:Class>` | `Class` | `isAbstract` maps to `ElementBase::is_abstract`. |
| `<UML:Interface>` | `Interface` | `isAbstract` forced to `true`. |
| `<UML:Enumeration>` | `Enum` | Enumeration name maps to `ElementBase::name`. Literals deferred to M9. |
| `<UML:DataType>` | **SKIP (M9)** | Primitive type registry in M9. |
| `<UML:Stereotype>` | **SKIP (M9)** | Stereotype registry in M9. |
| `<UML:Association>` | **SKIP (M9)** | Relationship parsing in M9. |
| `<UML:Generalization>` | **SKIP (M9)** | Relationship parsing in M9. |
| `<UML:Dependency>` | **SKIP (M9)** | Relationship parsing in M9. |
| `<UML:Realization>` | **SKIP (M9)** | Relationship parsing in M9. |
| `<XMI.extension>` | **SKIP** | Entirely skipped. |
| `<XMI.header>` | **SKIP** | Metadata only; exporter identity. |

### 3.3 Visibility Mapping

The `visibility` attribute is a string in XMI. We map it to our `Visibility` enum:

| XMI value | `Visibility` variant |
|-----------|---------------------|
| `"public"` | `Visibility::Public` (default) |
| `"protected"` | `Visibility::Protected` |
| `"private"` | `Visibility::Private` |
| `"implementation"` | `Visibility::Implementation` |
| missing / invalid | `Visibility::Public` (treated as default) |

### 3.4 Architectural Mismatches

| XMI Concept | Domain Concept | Mitigation |
|------------|---------------|------------|
| `xmi.id` (string) | `UmlId` (UUID) | `IdMap` bridge (section 4) |
| `stereotype` attribute (string) | `ElementBase::stereotype_id` (Option\<UmlId\>) | Two-pass resolution |
| `<UML:Namespace.ownedElement>` wrapper | `Package::children` (Vec\<UmlId\>) | Parent-stack tracking |
| `<UML:Model>` as view | Not a distinct type | Map to `Package` |
| No UUID in XMI | `UmlId::new()` generates UUID | Generated at registration time |
| `isSpecification`, `isLeaf`, `isRoot` | Not modelled | Silently ignored |

---

## 4. ID Mapping Strategy

### 4.1 Problem Statement

XMI uses string IDs like `"O0JJV24XoKdQ"` and `"m1"`. Our domain model uses
UUID-backed `UmlId`. Every XMI element with an `xmi.id` attribute must:

1. Be assigned a fresh `UmlId` during registration.
2. Be addressable by its XMI string ID during reference resolution
   (e.g., `stereotype="folder"` must resolve to the correct `UmlId`).

Since XMI string IDs are not UUIDs and have no predictable structure, we
**cannot** derive `UmlId` from the XMI string. We must maintain a mapping.

### 4.2 Chosen Strategy: HashMap Bridge

Maintain a `HashMap<String, UmlId>` during parsing. The lifecycle is:

1. **Registration:** When we encounter `xmi.id="O0JJV24XoKdQ"` on an element,
   we generate a new `UmlId` and insert the pair into the map.
2. **Lookup:** When we encounter `stereotype="folder"` (or any other reference),
   we look up `"folder"` in the map to obtain the `UmlId`.
3. **Consumption:** After Pass 2 (reference resolution), the ID map is no
   longer needed and can be dropped.

```rust
pub struct IdMap {
    /// XMI string ID → generated UmlId
    xmi_to_uml: HashMap<String, UmlId>,
}

impl IdMap {
    /// Register a new XMI ID, generating a fresh UmlId.
    /// Panics if the XMI ID is already registered (duplicate xmi.id).
    pub fn register(&mut self, xmi_id: &str) -> UmlId;

    /// Look up a previously registered XMI ID.
    /// Returns None if the XMI ID has not been registered yet (forward reference).
    pub fn resolve(&self, xmi_id: &str) -> Option<UmlId>;

    /// Returns true if the given XMI ID has been registered.
    pub fn contains(&self, xmi_id: &str) -> bool;
}
```

### 4.3 The IdMap Type

Full definition:

```rust
use std::collections::HashMap;
use uml_core::id::UmlId;

/// Mapping from XMI string IDs to generated UmlIds.
///
/// XMI files use short string identifiers like `"O0JJV24XoKdQ"` for
/// element cross-references. Our domain model uses UUID-backed `UmlId`.
/// This bridge type maps between the two during parsing.
///
/// # Invariants
///
/// - `register()` is called once per unique `xmi.id` value.
/// - `resolve()` is safe to call for forward references (returns `None`
///   if the target hasn't been registered yet).
#[derive(Debug, Clone)]
pub struct IdMap {
    xmi_to_uml: HashMap<String, UmlId>,
}

impl IdMap {
    /// Create an empty ID map.
    pub fn new() -> Self {
        Self {
            xmi_to_uml: HashMap::new(),
        }
    }

    /// Register a new XMI string ID.
    ///
    /// Generates a fresh `UmlId` and records the mapping. If the same
    /// `xmi_id` is registered twice, the second call overwrites the first
    /// (this should not happen in well-formed XMI but is not an error —
    /// the first element's `UmlId` wins in practice because of parsing order).
    pub fn register(&mut self, xmi_id: &str) -> UmlId {
        let uml_id = UmlId::new();
        self.xmi_to_uml.insert(xmi_id.to_string(), uml_id);
        uml_id
    }

    /// Look up the `UmlId` for a previously registered XMI string ID.
    pub fn resolve(&self, xmi_id: &str) -> Option<UmlId> {
        self.xmi_to_uml.get(xmi_id).copied()
    }

    /// Returns `true` if the XMI ID has been registered.
    pub fn contains(&self, xmi_id: &str) -> bool {
        self.xmi_to_uml.contains_key(xmi_id)
    }

    /// Number of registered mappings.
    pub fn len(&self) -> usize {
        self.xmi_to_uml.len()
    }

    /// Returns `true` if no mappings have been registered.
    pub fn is_empty(&self) -> bool {
        self.xmi_to_uml.is_empty()
    }
}

impl Default for IdMap {
    fn default() -> Self {
        Self::new()
    }
}
```

### 4.4 Forward Reference Handling

XMI files typically define stereotypes before they are referenced, but this
is not guaranteed by the format specification. We must handle the case where
a `stereotype="folder"` attribute is encountered before the
`<UML:Stereotype xmi.id="folder">` element.

**Strategy:** Deferred resolution via pending references.

```rust
/// A cross-reference that could not be resolved during Pass 1.
#[derive(Debug, Clone)]
struct PendingRef {
    /// The element's UmlId that has the unresolved reference.
    element_id: UmlId,
    /// The XMI string ID we need to resolve (e.g., "folder").
    target_xmi_id: String,
    /// Which field to set once resolved.
    field: PendingField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingField {
    Stereotype,
    // Future: TypeReference, RelationshipSource, RelationshipTarget, etc.
}
```

During Pass 1:
1. When we encounter `stereotype="folder"` on a class, we register a
   `PendingRef { element_id, target_xmi_id: "folder".into(), field: Stereotype }`.
2. We also register `<UML:Stereotype xmi.id="folder">` in the `IdMap` when we
   encounter it (if we ever do).

During Pass 2:
1. Iterate all pending references.
2. For each, look up `target_xmi_id` in the `IdMap`.
3. If found, set the corresponding field on the element.
4. If not found, emit a warning (dangling reference).

**This works because** XMI defines elements before referencing them in
practice. Even if a stereotype is referenced before it's defined (unlikely),
the `IdMap` will contain its entry by the time Pass 2 runs (assuming the
element appears somewhere in the file).

---

## 5. Parser Technology Evaluation

### 5.1 Candidates

#### Candidate 1: quick-xml (Event-based / StAX-like)

Already in workspace dependencies (`quick-xml = "0.37"`). Streaming parser
that reads events (`StartElement`, `EndElement`, `Text`, `Eof`) without
building a DOM tree.

**Pros:**
- **Memory efficient** — O(depth) rather than O(document size). Suitable for
  large XMI files (though typical files are <10 MB).
- **Already a workspace dependency** — no new crate to add.
- **Well-maintained** — actively developed, battle-tested, 2000+ stars.
- **Namespace-aware** — can distinguish `UML:Class` from other namespaced
  elements via `BytesStart::name()` with namespace resolution.
- **Ergonomic `Reader` API** — `Reader::from_reader()`, event loop with
  `read_event_into()`.
- **Configurable** — can trim text, expand empty elements, manage
  namespaces.

**Cons:**
- **Forward references require explicit state** — but we need this anyway
  for two-pass strategy.
- **No random access** — must track nesting depth and parent stack manually.
- **Slightly more verbose** than DOM-based APIs for hierarchical parsing.

#### Candidate 2: roxmltree (DOM-like, read-only tree)

Parses entire XML document into an immutable tree. Random access via
`parent()`, `children()`, `descendants()`, `attribute()`.

**Pros:**
- **Easy navigation** — `element.parent()`, `element.children()`,
  `element.descendants()`.
- **Forward references trivial** — just look up by `@xmi.id` anywhere in
  the tree.
- **Fast** — zero-copy where possible (borrows from the input string).

**Cons:**
- **Loads entire file into memory** — XMI files are small (<10 MB), so this
  is not a practical concern.
- **Not a workspace dependency** — would add `roxmltree = "0.20"`.
- **Read-only** — cannot build a tree to modify and write back (we don't
  need to, but worth noting).
- **No streaming** — entire file must fit in memory _and_ be valid UTF-8
  (XMI is always UTF-8 or can be transcoded).

#### Candidate 3: serde-xml-rs

Deserializes XML directly into Rust structs via serde derive macros.

**Pros:**
- **Zero boilerplate** for simple, fixed-schema XML.
- **Automatic field extraction** — `#[derive(Deserialize)]` on struct.

**Cons:**
- **Cannot handle XMI's complex nesting** — `<UML:Namespace.ownedElement>`
  is not a fixed type; the schema is flexible.
- **Cannot handle forward references** — serde expects a complete, self-
  contained document.
- **Cannot handle the `xmi.id` → `UmlId` mapping** — would require a
  custom deserializer.
- **Not suitable for XMI's flexible, extensible schema.**

### 5.2 Evaluation Matrix

| Criterion | quick-xml | roxmltree | serde-xml-rs |
|-----------|-----------|-----------|-------------|
| Memory footprint | O(depth) | O(document) | O(document) |
| Already in workspace | ✅ Yes | ❌ No | ❌ No |
| Namespace-aware | ✅ | ✅ | Partial |
| Forward ref handling | Manual (state machine) | Trivial (tree lookup) | Not supported |
| Hierarchical navigation | Manual (stack) | ✅ Built-in | ❌ |
| Streaming support | ✅ | ❌ | ❌ |
| Ergonomics for XMI | Medium | High | Low |
| Community/maintenance | Excellent | Excellent | Moderate |
| Learning curve | Medium | Low | Low |

### 5.3 Recommendation: quick-xml

**Verdict: Use `quick-xml`.**

The deciding factors are:

1. **Already in workspace dependencies.** Adding `roxmltree` would increase
   the dependency surface for marginal benefit. XMI files are simple enough
   that the streaming API is not a burden.

2. **Memory efficiency is a virtue.** While XMI files are small today,
   round-trip testing and code generation can produce files where streaming
   matters. Starting with `quick-xml` avoids a future migration.

3. **Two-pass strategy works well with streaming.** The state machine pattern
   (section 8) maps naturally to the event-based API. We maintain an explicit
   parent stack and dispatch to element-specific handlers.

4. **quick-xml's `Reader` API gives us fine-grained control** over how we
   handle namespaces, text events, and malformed input. DOM parsers abstract
   away the details we need to inspect.

**Counter-argument:** `roxmltree` would simplify forward reference resolution.
But our two-pass strategy already defers resolution to a separate phase. The
`IdMap` + `PendingRef` approach (section 4) works equally well with both
streaming and DOM parsers. The streaming parser forces us to be explicit about
state transitions, which reduces bugs in the long run.

---

## 6. Two-Pass Parsing Strategy

### 6.1 Why Two Passes?

XMI files contain cross-references between elements:
- `stereotype="folder"` references a `<UML:Stereotype>` element's `xmi.id`.
- Element A is defined before Element B but B references A.
- Containment is expressed via nesting, not via `UmlId` references.

A single-pass parser would need to handle forward references inline, which
complicates the state machine. A two-pass approach separates concerns:

| Pass | Responsibility | Outcome |
|------|---------------|---------|
| **Pass 1** | Extract all elements, register IDs, build hierarchy, collect pending refs | `UmlModel` populated with elements, `PendingRef` list filled |
| **Pass 2** | Resolve all pending cross-references | `UmlModel` fully connected |

### 6.2 Pass 1: Structural Element Extraction

Walk the XML tree depth-first using `quick-xml` events. For each model element:

1. **Extract** `xmi.id`, `name`, `visibility`, `isAbstract`, `stereotype`.
2. **Register** the `xmi.id` in `IdMap`, generating a fresh `UmlId`.
3. **Create** the corresponding `ModelElement` variant (`Package`, `Class`,
   `Interface`, `Enum`).
4. **Insert** into `UmlModel`.
5. **Track** the element as the current parent for containment.
6. **Collect** any `stereotype` reference as a `PendingRef`.

**Hierarchy tracking** via a stack:
```rust
// When we enter an element that can contain children (Package, Model):
parent_stack.push(current_element_id);

// When we exit the element's scope:
parent_stack.pop();
```

**Containment association** via `UmlModel::add_to_package()`:
```rust
// After creating a child element and inserting it:
if let Some(&parent_id) = parent_stack.last() {
    model.add_to_package(parent_id, child_id)?;
}
```

### 6.3 Pass 2: Cross-Reference Resolution

Iterate through all `PendingRef` entries collected during Pass 1:

```rust
for pending in self.pending_refs.drain(..) {
    if let Some(target_id) = self.id_map.resolve(&pending.target_xmi_id) {
        match pending.field {
            PendingField::Stereotype => {
                if let Some(elem) = model.get_mut(pending.element_id) {
                    elem.base_mut().stereotype_id = Some(target_id);
                }
            }
        }
    } else {
        tracing::warn!(
            "Dangling reference: element {} has unresolved stereotype '{}'",
            pending.element_id, pending.target_xmi_id
        );
    }
}
```

### 6.4 M8 Scope vs Deferred Items

| Item | M8 | M9+ |
|------|-----|-----|
| `<UML:Class>` parsing | ✅ Element + base attributes | ❌ Attributes/Operations |
| `<UML:Package>` parsing | ✅ Element + base attributes | ✅ |
| `<UML:Interface>` parsing | ✅ Element + base attributes | ❌ Attributes/Operations |
| `<UML:Enum>` parsing | ✅ Element + base attributes | ❌ Literals |
| `<UML:Model>` as Package | ✅ | ✅ |
| Containment hierarchy | ✅ Parent-stack tracking | ✅ |
| `IdMap` + pending refs | ✅ | ✅ |
| Stereotype references | ✅ Captured as pending | ✅ Resolved in Pass 2 |
| `<UML:Stereotype>` parsing | ❌ Skipped | ✅ Stereotype registry |
| `<UML:DataType>` parsing | ❌ Skipped | ✅ Primitive type registry |
| `<UML:Association>` etc. | ❌ Skipped | ✅ Relationship parsing |
| `<UML:Namespace.ownedElement>` | ✅ Container detection | ✅ |
| `<XMI.extensions>` / `<diagrams>` | ❌ Skipped | ✅ (separate task) |
| Attributes & Operations | ❌ | ✅ |
| Enumeration literals | ❌ | ✅ |
| Template parameters | ❌ | ✅ |

---

## 7. Proposed Module Structure

### 7.1 Current Status

The XMI reader currently exists as a stub in `uml-core/src/xmi/reader.rs`:

```
crates/uml-core/src/
├── xmi/
│   ├── mod.rs          # XmiVersion enum, module docs
│   ├── reader.rs       # Stub: pub struct XmiReader;
│   └── writer.rs       # Stub: pub struct XmiWriter;
```

The `uml-io` crate (at `crates/uml-io/`) exists for I/O operations but
currently only has a `StorageBackend` trait and file format enum.

### 7.2 Proposed Structure

Expand the existing `uml-core/src/xmi/` module with the reader implementation:

```
crates/uml-core/src/
├── xmi/
│   ├── mod.rs              # Module docs, XmiVersion enum
│   ├── reader.rs           # XmiReader struct + public API
│   ├── id_map.rs           # IdMap type
│   ├── parser.rs           # Core event-loop parsing logic
│   ├── elements.rs         # Element-specific parsers (parse_class, parse_package, ...)
│   ├── error.rs            # XmiError enum
│   └── writer.rs           # Stub (unchanged for now)
```

**Module responsibilities:**

| File | Responsibility | Public API |
|------|---------------|-----------|
| `reader.rs` | `XmiReader` struct, `read_from()`, `resolve_references()` | ✅ Public |
| `id_map.rs` | `IdMap` struct, `PendingRef` enum | `pub(crate)` |
| `parser.rs` | Event loop, state machine, dispatch to element parsers | `pub(crate)` |
| `elements.rs` | `parse_class()`, `parse_package()`, `parse_interface()`, `parse_enum()`, `parse_model_attrs()` helpers | `pub(crate)` |
| `error.rs` | `XmiError` enum | ✅ Public |

### 7.3 Rationale for Location

The XMI reader lives in `uml-core`, not `uml-io`, because:

1. **Historical continuity** — the stubs are already in `uml-core`. The planning
   documents envisioned a separate `uml-xmi` crate, but the workspace was
   consolidated (per `workspace_consolidation_v2.md`), merging small crates
   into `uml-core`.

2. **Tight coupling to domain types** — the reader creates `ModelElement`,
   `Package`, `Class`, `Interface`, `Enum` instances. These types are defined
   in `uml-core`. Putting the reader in a separate crate would require either:
   - Re-exporting everything from `uml-core` (extra public surface), or
   - Making model constructors `pub(crate)` to the whole workspace (weakening
     encapsulation).

3. **Build-time consideration** — `quick-xml` is the only additional dependency
   for XMI reading. The `uml-core` crate already depends on it (workspace dep).
   The added compile time is negligible.

The `uml-io` crate's role is **file I/O orchestration** (format detection,
compression, autosave). It will delegate to `uml-core::xmi::reader::XmiReader`
for actual parsing.

---

## 8. Detailed Parsing Algorithm

### 8.1 Initialization

```rust
fn parse_xmi<R: Read>(reader: R, model: &mut UmlModel) -> Result<XmiSummary, XmiError> {
    let mut xml_reader = Reader::from_reader(reader);
    xml_reader.config_mut().trim_text(true);
    xml_reader.config_mut().expand_empty_elements(true);

    let mut state = ParserState {
        id_map: IdMap::new(),
        pending_refs: Vec::new(),
        parent_stack: Vec::new(),
        element_count: 0,
        depth: 0,
    };

    // Detect XMI version
    let version = detect_xmi_version(&mut xml_reader)?;
    // ... proceed with version-specific parsing ...
}
```

### 8.2 XML Event Loop

The core loop reads `quick_xml::events::Event` variants and dispatches:

```rust
let mut buf = Vec::new();
loop {
    match xml_reader.read_event_into(&mut buf)? {
        Event::Start(ref elem) => {
            let tag = decode_name(&xml_reader, elem)?;
            let attrs = collect_attributes(elem)?;
            handle_start_tag(&tag, &attrs, &mut state, model)?;
            state.depth += 1;
        }
        Event::End(ref elem) => {
            let tag = decode_name(&xml_reader, elem)?;
            handle_end_tag(&tag, &mut state)?;
            state.depth -= 1;
        }
        Event::Text(ref text) => {
            // Not used in M8 (text content is minimal in XMI)
        }
        Event::Eof => break,
        _ => {} // Ignore comments, CDATA, PI, etc.
    }
    buf.clear();
}
```

### 8.3 State Machine

The parser maintains an explicit state that tracks what we are currently
parsing:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParseState {
    /// Outside any UML element (scanning for <XMI.content>)
    Root,
    /// Inside <XMI.content>
    Content,
    /// Inside a container element (<UML:Model>, <UML:Package>)
    InContainer { container_id: UmlId },
    /// Inside <UML:Namespace.ownedElement>
    InOwnedElement { parent_id: UmlId },
    /// Inside <XMI.extensions> — skip everything
    InExtensions,
    /// Inside a classifier element (<UML:Class>, <UML:Interface>, <UML:Enum>)
    InClassifier { classifier_id: UmlId },
}
```

However, in practice the parent stack + tag dispatch is simpler than a formal
state machine. We use a parent stack for containment and skip-lists for
extensions:

```rust
struct ParserState {
    id_map: IdMap,
    pending_refs: Vec<PendingRef>,
    parent_stack: Vec<UmlId>,
    extensions_depth: usize,   // >0 means we're inside <XMI.extensions>
    element_count: usize,
}
```

### 8.4 Element Creation

For each recognised element tag, we extract attributes and create the
corresponding `ModelElement`:

```rust
fn parse_class(
    attrs: &HashMap<String, String>,
    state: &mut ParserState,
) -> Result<ModelElement, XmiError> {
    let xmi_id = get_required_attr(attrs, "xmi.id", "UML:Class")?;
    let name = get_required_attr(attrs, "name", "UML:Class")?;
    let visibility = parse_visibility(attrs.get("visibility").map(String::as_str));
    let is_abstract = parse_bool(attrs.get("isAbstract"));
    let stereotype = attrs.get("stereotype").cloned();

    let uml_id = state.id_map.register(&xmi_id);
    let mut class = Class::new(&name);
    class.base.id = uml_id;
    class.base.visibility = visibility;
    class.base.is_abstract = is_abstract;

    // Capture stereotype reference as pending
    if let Some(stereo_xmi_id) = stereotype {
        state.pending_refs.push(PendingRef {
            element_id: uml_id,
            target_xmi_id: stereo_xmi_id,
            field: PendingField::Stereotype,
        });
    }

    Ok(ModelElement::Class(class))
}
```

### 8.5 Containment Tracking

```rust
fn handle_start_tag(
    tag: &str,
    attrs: &HashMap<String, String>,
    state: &mut ParserState,
    model: &mut UmlModel,
) -> Result<(), XmiError> {
    match tag {
        // Container elements — become the new "current parent"
        "UML:Model" | "UML:Package" => {
            let elem = parse_package_or_model(tag, attrs, state)?;
            let id = elem.id();
            model.insert(elem);
            state.parent_stack.push(id);
            state.element_count += 1;
        }

        // Classifier elements
        "UML:Class" => {
            let elem = parse_class(attrs, state)?;
            let id = elem.id();
            model.insert(elem);
            add_to_current_parent(id, &mut state, model);
            state.element_count += 1;
        }

        // ... Interface, Enum ...

        // Container wrapper — no new element, but signals that children
        // belong to the current parent (which is already on the stack).
        // No action needed: our parent_stack already holds the correct parent.
        "UML:Namespace.ownedElement" => {
            // Parent is already on the stack from the containing element.
            // We do NOT push anything here — the parent is the container.
        }

        // Extensions — skip entirely
        "XMI.extensions" => {
            state.extensions_depth += 1;
        }

        // Skip unknown elements
        _ => {
            tracing::debug!("Skipping unknown element: {tag}");
        }
    }
    Ok(())
}

fn handle_end_tag(
    tag: &str,
    state: &mut ParserState,
) -> Result<(), XmiError> {
    match tag {
        "UML:Model" | "UML:Package" => {
            state.parent_stack.pop();
        }
        "XMI.extensions" => {
            state.extensions_depth = state.extensions_depth.saturating_sub(1);
        }
        _ => {}
    }
    Ok(())
}

/// Add the given element as a child of the current parent on the stack.
fn add_to_current_parent(
    child_id: UmlId,
    state: &mut ParserState,
    model: &mut UmlModel,
) {
    // Skip if we're inside extensions
    if state.extensions_depth > 0 {
        return;
    }
    if let Some(&parent_id) = state.parent_stack.last() {
        // Ignore errors silently — the parent might not be a Package type
        // (e.g., if we're still in the root <XMI> element).
        let _ = model.add_to_package(parent_id, child_id);
    }
}
```

### 8.6 Resolve References (Pass 2)

```rust
pub fn resolve_references(
    pending_refs: &[PendingRef],
    id_map: &IdMap,
    model: &mut UmlModel,
) -> Result<(), XmiError> {
    let mut unresolved = 0;

    for pending in pending_refs {
        if let Some(target_id) = id_map.resolve(&pending.target_xmi_id) {
            match pending.field {
                PendingField::Stereotype => {
                    if let Some(elem) = model.get_mut(pending.element_id) {
                        elem.base_mut().stereotype_id = Some(target_id);
                    }
                }
            }
        } else {
            tracing::warn!(
                "Unresolved reference: element {} references '{}' ({:?})",
                pending.element_id,
                pending.target_xmi_id,
                pending.field,
            );
            unresolved += 1;
        }
    }

    if unresolved > 0 {
        return Err(XmiError::UnresolvedReferences { count: unresolved });
    }
    Ok(())
}
```

### 8.7 Pseudocode

```text
FUNCTION read_from(reader, model):
    xml = quick_xml::Reader::from_reader(reader)
    xml.trim_text(true)
    xml.expand_empty_elements(true)

    state = ParserState {
        id_map: IdMap::new(),
        pending_refs: [],
        parent_stack: [],
        extensions_depth: 0,
        element_count: 0,
    }

    buf = []

    LOOP:
        event = xml.read_event_into(&mut buf)
        IF event is Error: RETURN error
        IF event is Eof: BREAK

        MATCH event:
            Start(elem):
                tag = elem.name()
                attrs = elem.attributes()
                IF tag is "UML:Model" OR tag is "UML:Package":
                    elem = create_package(attrs, state)
                    model.insert(elem)
                    state.parent_stack.push(elem.id)
                    state.element_count++
                ELIF tag is "UML:Class":
                    elem = create_class(attrs, state)
                    model.insert(elem)
                    add_to_parent(elem.id, state, model)
                    state.element_count++
                ELIF tag is "UML:Interface":
                    ... similar to Class ...
                ELIF tag is "UML:Enumeration":
                    ... similar to Enum ...
                ELIF tag is "XMI.extensions":
                    state.extensions_depth++
                ELIF tag is "UML:Stereotype":
                    state.id_map.register(attrs["xmi.id"])
                    // Element itself not stored in M8
                ELIF tag is "UML:DataType":
                    state.id_map.register(attrs["xmi.id"])
                    // Element itself not stored in M8
                ELSE:
                    SKIP (trace log for unknown)

            End(tag):
                IF tag is "UML:Model" OR tag is "UML:Package":
                    state.parent_stack.pop()
                ELIF tag is "XMI.extensions":
                    state.extensions_depth--

    // PASS 2: Resolve cross-references
    FOR pending in state.pending_refs:
        target_id = state.id_map.resolve(pending.target_xmi_id)
        IF target_id is Some:
            SET pending.field on model[pending.element_id] = target_id
        ELSE:
            LOG warning: unresolved reference
            unresolved++

    IF unresolved > 0: RETURN error with count
    RETURN Ok(state.element_count)
```

---

## 9. Proposed API

### 9.1 Error Types

```rust
// uml-core/src/xmi/error.rs

use uml_core::id::UmlId;

/// Errors that can occur during XMI parsing.
#[derive(Debug, thiserror::Error)]
pub enum XmiError {
    /// XML parsing error from quick-xml.
    #[error("XML parse error at position {position}: {kind:?}")]
    Xml {
        /// The kind of XML error.
        kind: quick_xml::Error,
        /// Byte position in the input where the error occurred.
        position: usize,
    },

    /// I/O error reading the input.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Unknown XMI element tag encountered.
    #[error("Unknown XMI element at depth {depth}: '{tag}'")]
    UnknownElement {
        /// The element tag name (e.g., "UML:Foobar").
        tag: String,
        /// Nesting depth where the element was encountered.
        depth: usize,
    },

    /// A required attribute is missing from an element.
    #[error("Missing required attribute '{attr}' on element '{element}'")]
    MissingAttribute {
        /// The element tag name.
        element: &'static str,
        /// The attribute name.
        attr: &'static str,
    },

    /// Invalid attribute value (e.g., unparseable boolean or visibility).
    #[error("Invalid attribute value for '{attr}' on element '{element}': {value}")]
    InvalidAttributeValue {
        /// The element tag name.
        element: &'static str,
        /// The attribute name.
        attr: &'static str,
        /// The invalid value.
        value: String,
    },

    /// Some pending references could not be resolved in Pass 2.
    #[error("{count} unresolved cross-reference(s) remain after Pass 2")]
    UnresolvedReferences {
        /// Number of unresolved references.
        count: usize,
    },

    /// Unsupported XMI version.
    #[error("Unsupported XMI version: {version}")]
    UnsupportedVersion {
        /// The version string found in the document.
        version: String,
    },

    /// XMI version could not be detected.
    #[error("Could not detect XMI version — no 'xmi.version' or 'xmi:version' attribute found")]
    VersionDetectionFailed,

    /// Model operation error (element not found, cycle, etc.).
    #[error("Model operation error: {0}")]
    Model(#[from] crate::repository::ModelError),

    /// Invalid state — the parser encountered an unexpected sequence of events.
    #[error("Parser state error: {message}")]
    ParserState {
        /// Human-readable description of the state inconsistency.
        message: String,
    },
}

impl From<quick_xml::Error> for XmiError {
    fn from(err: quick_xml::Error) -> Self {
        // quick_xml::Error doesn't carry position info natively,
        // but we can extract it from the reader.
        // For now, a best-effort conversion:
        match err {
            quick_xml::Error::Io(io_err) => Self::Io(io_err),
            other => Self::Xml {
                kind: other,
                position: 0, // Caller should fill this in
            },
        }
    }
}
```

### 9.2 XmiReader Struct

```rust
// uml-core/src/xmi/reader.rs

use std::io::Read;
use crate::repository::UmlModel;
use super::error::XmiError;
use super::id_map::{IdMap, PendingRef, PendingField};

/// Summary of a successful XMI parse.
#[derive(Debug, Clone)]
pub struct XmiSummary {
    /// Number of model elements parsed.
    pub element_count: usize,
    /// Number of pending cross-references resolved in Pass 2.
    pub refs_resolved: usize,
    /// Number of pending cross-references that could NOT be resolved.
    pub refs_unresolved: usize,
}

/// Streaming XMI reader that populates a `UmlModel`.
///
/// Uses `quick-xml` for event-based parsing (no DOM tree). Implements a
/// two-pass strategy:
///
/// 1. **Pass 1 (structural):** Walk the XML tree depth-first, extracting
///    element data, registering IDs, building the containment hierarchy,
///    and collecting pending cross-references.
/// 2. **Pass 2 (resolution):** Iterate pending references and resolve them
///    against the registered ID map.
///
/// # Example
///
/// ```rust,ignore
/// use uml_core::xmi::reader::XmiReader;
/// use uml_core::repository::UmlModel;
///
/// let mut model = UmlModel::new();
/// let mut reader = XmiReader::new();
/// let summary = reader.read_from(std::fs::File::open("model.xmi")?, &mut model)?;
/// println!("Parsed {} elements", summary.element_count);
/// ```
#[derive(Debug)]
pub struct XmiReader {
    /// XMI string ID → UmlId mapping.
    id_map: IdMap,
    /// Pending cross-references collected during Pass 1.
    pending_refs: Vec<PendingRef>,
    /// Parent element stack for containment tracking.
    parent_stack: Vec<super::id_map::UmlId>,
    /// Nesting depth inside `<XMI.extensions>` (0 = outside).
    extensions_depth: usize,
    /// Total elements parsed.
    element_count: usize,
}

impl XmiReader {
    /// Create a new XMI reader.
    #[must_use]
    pub fn new() -> Self;

    /// Parse XMI from a reader and populate the given model.
    ///
    /// Executes Pass 1 (structural extraction) and Pass 2 (reference
    /// resolution) automatically.
    ///
    /// # Errors
    ///
    /// Returns `XmiError` if:
    /// - The XML is malformed or invalid.
    /// - Required attributes are missing on known elements.
    /// - Unresolved references remain after Pass 2.
    /// - An I/O error occurs.
    pub fn read_from<R: Read>(
        &mut self,
        reader: R,
        model: &mut UmlModel,
    ) -> Result<XmiSummary, XmiError>;

    /// Execute only Pass 2 (reference resolution) on the model.
    ///
    /// Useful if elements were added to the model after `read_from()`
    /// and need reference resolution against the existing ID map.
    /// Normally called automatically by `read_from()`.
    pub fn resolve_references(
        &mut self,
        model: &mut UmlModel,
    ) -> Result<XmiSummary, XmiError>;

    /// Reset all internal state, clearing the ID map and pending refs.
    /// Allows reusing the reader for a new file.
    pub fn reset(&mut self);
}

impl Default for XmiReader {
    fn default() -> Self {
        Self::new()
    }
}
```

### 9.3 XmiVersion Detection

```rust
// uml-core/src/xmi/mod.rs — already defined

/// XMI version selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmiVersion {
    /// XMI 1.2 — uses `xmi.id`, `UML:` namespace prefix.
    V1_2,
    /// XMI 2.1 — uses `xmi:id`, `uml:` namespace prefix, `<packagedElement>`.
    V2_1,
}
```

Detection logic (part of `reader.rs`):

```rust
fn detect_xmi_version<R: Read>(
    reader: &mut Reader<R>,
) -> Result<XmiVersion, XmiError> {
    // Peek at the root element by reading events until we find
    // the opening <XMI> tag, then check the xmi.version attribute.
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf)? {
            Event::Start(ref elem) if elem.name().as_ref() == b"XMI" => {
                let attrs = collect_attributes(elem)?;
                if let Some(version) = attrs.get("xmi.version") {
                    return match version.as_str() {
                        "1.2" | "1.1" | "1.0" => Ok(XmiVersion::V1_2),
                        "2.1" | "2.0" => Ok(XmiVersion::V2_1),
                        other => Err(XmiError::UnsupportedVersion {
                            version: other.to_string(),
                        }),
                    };
                }
                if let Some(version) = attrs.get("xmi:version") {
                    return match version.as_str() {
                        "2.1" | "2.0" => Ok(XmiVersion::V2_1),
                        other => Err(XmiError::UnsupportedVersion {
                            version: other.to_string(),
                        }),
                    };
                }
                return Err(XmiError::VersionDetectionFailed);
            }
            Event::Eof => return Err(XmiError::VersionDetectionFailed),
            _ => {} // Skip comments, PIs, DTD, etc.
        }
        buf.clear();
    }
}
```

**Note:** After version detection, the reader must be "rewound" to the start
of the document. Since `quick-xml`'s `Reader` is streaming, we cannot rewind
easily. Instead, we use `Reader::from_str()` with the full input loaded into
a `String` for version detection, or we buffer the initial events and replay
them. The practical approach is to load the entire XMI into a `String` first
(typical files are <10 MB), use `from_str()` for version detection and
parsing, so we can cheaply reconstruct the reader.

### 9.4 Helper Traits

```rust
// uml-core/src/xmi/elements.rs (pub(crate))

use crate::elements::Visibility;
use crate::xmi::error::XmiError;
use std::collections::HashMap;

/// Parse a boolean attribute value.
///
/// Accepts: "true", "1", "false", "0" (case-insensitive).
/// Returns `None` if the attribute is absent, `Some(true/false)` if present,
/// and `Err` if the value is invalid.
fn parse_bool(value: Option<&str>) -> Option<bool> {
    match value {
        None => None,
        Some("true") | Some("1") => Some(true),
        Some("false") | Some("0") => Some(false),
        Some(other) => {
            tracing::warn!("Invalid boolean value: '{other}', defaulting to false");
            Some(false)
        }
    }
}

/// Parse a visibility attribute value.
fn parse_visibility(value: Option<&str>) -> Visibility {
    match value {
        None | Some("public") => Visibility::Public,
        Some("protected") => Visibility::Protected,
        Some("private") => Visibility::Private,
        Some("implementation") => Visibility::Implementation,
        Some(other) => {
            tracing::warn!("Invalid visibility value: '{other}', defaulting to 'public'");
            Visibility::Public
        }
    }
}

/// Helper: collect element attributes into a HashMap.
fn collect_attributes(
    elem: &quick_xml::events::BytesStart<'_>,
) -> Result<HashMap<String, String>, XmiError> {
    let mut map = HashMap::new();
    for attr_result in elem.attributes() {
        let attr = attr_result?;
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = attr.unescape_value()?.to_string();
        map.insert(key, value);
    }
    Ok(map)
}

/// Helper: get a required attribute, returning an error if missing.
fn get_required_attr<'a>(
    attrs: &'a HashMap<String, String>,
    name: &'static str,
    element: &'static str,
) -> Result<&'a str, XmiError> {
    attrs.get(name).map(|s| s.as_str()).ok_or_else(|| {
        XmiError::MissingAttribute {
            element,
            attr: name,
        }
    })
}
```

---

## 10. Error Handling Strategy

### 10.1 Error Categories

| Category | Error Variant | Severity | Recovery |
|----------|--------------|----------|----------|
| **XML parse error** | `XmiError::Xml` | Fatal | Cannot proceed — malformed XML. |
| **I/O error** | `XmiError::Io` | Fatal | File read issue. |
| **Missing required attribute** | `XmiError::MissingAttribute` | Fatal | `xmi.id` or `name` is missing — element has no identity. |
| **Invalid attribute value** | `XmiError::InvalidAttributeValue` | Warning | Non-fatal for visibility/boolean; use defaults. Returned as error only for critical fields. |
| **Unknown element** | `XmiError::UnknownElement` | Warning | Skip with trace log. Not returned for unrecognised tags in production mode. |
| **Unresolved references** | `XmiError::UnresolvedReferences` | Warning | Elements are created but refer to unknown targets. Logged; callers can decide severity. |
| **Unsupported version** | `XmiError::UnsupportedVersion` | Fatal | Cannot parse an unknown XMI version. |
| **Version detection failed** | `XmiError::VersionDetectionFailed` | Fatal | Not an XMI file or root element malformed. |
| **Model error (cycle, not found)** | `XmiError::Model` | Fatal | Internal inconsistency — should not occur with well-formed XMI. |
| **Parser state error** | `XmiError::ParserState` | Fatal | Unexpected event sequence — indicates a bug in the parser. |

### 10.2 Recovery Strategy

- **Fatal errors** bubble up to the caller via `Result`. The model may be
  partially populated (elements parsed before the error occurred will exist
  in the model). Callers can inspect the partial model if desired.
- **Warnings** (invalid boolean, invalid visibility, unknown elements) are
  logged via `tracing::warn!()` and do not abort parsing. We use sensible
  defaults:
  - Missing visibility → `Visibility::Public`.
  - Invalid boolean → `false`.
  - Unknown element → skip silently (trace log at `DEBUG` level).
- **Unresolved references** in Pass 2 are logged at `WARN` and counted.
  If any remain, `read_from()` returns `Err(XmiError::UnresolvedReferences)`
  with the count. Callers can decide whether to treat dangling refs as
  an error or a warning.

### 10.3 Logging

We use `tracing` for structured logging throughout the parser:

| Level | When | Examples |
|-------|------|----------|
| `ERROR` | Fatal parse errors | Missing `xmi.id`, XML syntax error |
| `WARN` | Recoverable issues | Invalid boolean, dangling reference, unknown attribute |
| `INFO` | Progress indicators | "Parsed 42 elements, 3 stereotypes", "Pass 2 resolved 5/5 refs" |
| `DEBUG` | Detailed trace | Element creation, stack pushes/pops, attribute values |
| `TRACE` | Event-level logging | Every XML event dispatched |

---

## 11. Test Plan

### 11.1 Unit Tests (M8)

All tests are in `#[cfg(test)] mod tests { ... }` within each module.

| # | Test Name | Description | Input | Expected |
|---|-----------|-------------|-------|----------|
| 1 | `test_minimal_xmi` | Minimal XMI with one class | `<UML:Class xmi.id="c1" name="Foo"/>` | One `Class` element in model, name "Foo" |
| 2 | `test_package_with_class` | Package containing one class | `<UML:Package xmi.id="p1" name="P"><UML:Namespace.ownedElement><UML:Class xmi.id="c1" name="C"/></UML:Namespace.ownedElement></UML:Package>` | Package `P` has child `C` |
| 3 | `test_nested_packages` | Three levels of nesting | Package A → Package B → Class C | `A` contains `B`, `B` contains `C` |
| 4 | `test_model_as_package` | Root \<UML:Model\> and view models | Full document with two views | Root Package contains two child Packages |
| 5 | `test_interface` | Interface element | `<UML:Interface xmi.id="i1" name="MyInterface"/>` | Interface with `is_abstract: true` |
| 6 | `test_enum` | Enumeration element | `<UML:Enumeration xmi.id="e1" name="Color"/>` | Enum with name "Color" |
| 7 | `test_visibility_mapping` | All visibility values | Four classes with different visibilities | Each maps to correct `Visibility` variant |
| 8 | `test_is_abstract` | Abstract class | `<UML:Class xmi.id="c1" name="Abs" isAbstract="true"/>` | `ElementBase::is_abstract == true` |
| 9 | `test_stereotype_captured` | Class with stereotype attribute | `<UML:Class xmi.id="c1" name="Entity" stereotype="persist"/>` | PendingRef created with target "persist" |
| 10 | `test_extensions_skipped` | Extensions content ignored | `<XMI.extensions><diagrams>...</diagrams></XMI.extensions>` | No elements created from extension content |
| 11 | `test_unknown_elements_skipped` | Unknown tags ignored | `<UML:Foobar xmi.id="x" name="X"/>` | No element created, parsing continues |
| 12 | `test_missing_xmi_id` | Element without xmi.id | `<UML:Class name="NoID"/>` | `Err(XmiError::MissingAttribute)` |
| 13 | `test_missing_name` | Element without name | `<UML:Class xmi.id="c1"/>` | `Err(XmiError::MissingAttribute)` |
| 14 | `test_empty_xmi` | Empty document with root only | `<XMI xmi.version="1.2"><XMI.content/></XMI>` | Empty model (0 elements) |
| 15 | `test_non_xml` | Non-XML input | `"this is not xml"` | `Err(XmiError::Xml)` |
| 16 | `test_id_map_register_resolve` | IdMap round-trip | Register "abc", resolve "abc" | Matching UmlId returned |
| 17 | `test_id_map_forward_ref` | Forward reference returns None | Resolve before registration | `None` returned |
| 18 | `test_id_map_duplicate` | Duplicate registration | Register same ID twice | Second call overwrites but ded up with same first UmlId in practice |
| 19 | `test_pending_ref_resolve` | Stereotype resolved in Pass 2 | Stereotype "persist" defined and referenced | `stereotype_id` set on referencing element |
| 20 | `test_pending_ref_dangling` | Unresolved stereotype reference | Referenced stereotype never defined | Warning logged, `Err` returned if strict mode |

### 11.2 Integration Tests (M8)

| # | Test Name | Description | Input | Expected |
|---|-----------|-------------|-------|----------|
| 1 | `test_real_cpp_file` | Parse a real C++-generated XMI file | `test_1_2.xmi` from test data | Parses without error, >0 elements |
| 2 | `test_roundtrip_basic` | Parse → serialize → compare | Minimal XMI | Serialized output resembles input structure |
| 3 | `test_large_file` | Performance test | Generated XMI with 1000+ classes | Completes within 2 seconds |

### 11.3 Test Fixtures

Inline XML strings are used for unit tests (via `&str` + `reader::new()`
with `XmiReader`). Integration tests use files in `tests/data/`:

```
tests/data/
├── test_1_2.xmi              # Real C++ Umbrello output (classes, packages, views)
├── test_minimal.xmi           # Hand-crafted minimal XMI
├── test_stereotypes.xmi       # XMI with multiple stereotype definitions
├── test_empty.xmi             # Minimal valid XMI with no model elements
├── test_malformed.xml         # Unparseable XML
```

### 11.4 Acceptance Criteria

1. All unit and integration tests pass.
2. A real C++-generated `.xmi` file (e.g., `test_1_2.xmi`) is parsed without
   fatal errors, extracting classes and packages into `UmlModel`.
3. Containment hierarchy is preserved: package nesting matches the XMI
   document structure.
4. Stereotype references are captured as pending and resolved when the
   stereotype element is present in the file.
5. Unknown elements, `<XMI.extensions>`, and `<XMI.header>` are gracefully
   skipped.
6. No `unwrap()` or `expect()` calls in production code.
7. `cargo clippy` is clean.
8. `cargo test` passes.
9. Code coverage for new modules is >85%.

---

## 12. Staged Implementation Plan

### 12.1 Stage 1: Skeleton + XMI Detection

**Goal:** `XmiReader` struct compiles, detects XMI version, returns error
for non-XMI input.

**Files:**
- `uml-core/src/xmi/error.rs` — XmiError enum
- `uml-core/src/xmi/id_map.rs` — IdMap + PendingRef types
- `uml-core/src/xmi/reader.rs` — XmiReader struct with `new()`, `read_from()`
  (stub), `resolve_references()` (stub)
- `uml-core/src/xmi/elements.rs` — attribute parsing helpers

**Tests:**
- `test_empty_xmi` — returns empty model
- `test_non_xml` — returns parse error
- `test_id_map_register_resolve` — IdMap round-trip
- `test_id_map_forward_ref` — forward ref returns None

### 12.2 Stage 2: Pass 1 — Basic Elements

**Goal:** `<UML:Class>`, `<UML:Package>`, `<UML:Interface>`, `<UML:Enum>`
elements are parsed and inserted into `UmlModel`.

**Files:**
- `uml-core/src/xmi/parser.rs` — main event loop
- Extend `uml-core/src/xmi/elements.rs` — `parse_class()`, `parse_package()`,
  `parse_interface()`, `parse_enum()`

**Tests:**
- `test_minimal_xmi` — single class parsed
- `test_interface` — interface parsed with `is_abstract: true`
- `test_enum` — enumeration parsed
- `test_visibility_mapping` — all visibility values
- `test_is_abstract` — abstract flag parsed
- `test_missing_xmi_id` — error on missing ID
- `test_missing_name` — error on missing name

### 12.3 Stage 3: Containment Hierarchy

**Goal:** Package nesting tracked via parent stack. Children correctly
associated with parents via `UmlModel::add_to_package()`.

**Files:**
- Extend `uml-core/src/xmi/parser.rs` — parent stack, `add_to_current_parent()`
- Extend `uml-core/src/xmi/elements.rs` — `<UML:Model>` as Package

**Tests:**
- `test_package_with_class` — package contains class
- `test_nested_packages` — three-level nesting
- `test_model_as_package` — `UML:Model` elements become packages
- `test_extensions_skipped` — no containment inside extensions

### 12.4 Stage 4: Pass 2 — Reference Resolution

**Goal:** Stereotype references collected in Pass 1 are resolved in Pass 2.
Dangling references produce warnings.

**Files:**
- Extend `uml-core/src/xmi/reader.rs` — `resolve_references()` implementation
- Extend `uml-core/src/xmi/elements.rs` — stereotype attribute capture

**Tests:**
- `test_stereotype_captured` — pending ref created
- `test_pending_ref_resolve` — stereo resolved to UmlId
- `test_pending_ref_dangling` — dangling ref warning/error

### 12.5 Stage 5: Edge Cases & Real Files

**Goal:** Real-world XMI files parse correctly. Edge cases handled.

**Files:**
- Integration tests with real `.xmi` files
- Edge-case handling (empty elements, unusual whitespace, self-closing tags)

**Tests:**
- `test_real_cpp_file` — real Umbrello XMI file
- All acceptance criteria validated

---

## 13. What Is NOT in M8

The following are explicitly **out of scope** for M8 and deferred to M9+:

1. **Attribute and operation parsing** — `UML:Classifier.feature` elements
   that contain `<UML:Attribute>` and `<UML:Operation>`. The feature block
   appears inside `<UML:Class>` / `<UML:Interface>` / `<UML:Enum>` elements.

2. **Enumeration literals** — `<UML:EnumerationLiteral>` inside
   `<UML:Enumeration>`.

3. **Template parameters** — `<UML:TemplateParameter>` elements.

4. **Relationship parsing** — `<UML:Association>`, `<UML:Generalization>`,
   `<UML:Dependency>`, `<UML:Realization>`. These require resolving source
   and target element references.

5. **Stereotype registry** — creating `Stereotype` model elements from
   `<UML:Stereotype>` definitions. In M8, we only capture the `stereotype`
   attribute as a pending reference.

6. **Primitive type registry** — `<UML:DataType>` elements such as "int",
   "bool", "string". These will be resolved to `TypeReference::primitive()`.

7. **Diagram data** — `<diagrams>` section in `<XMI.extensions>` containing
   widget positions, sizes, edges, and visual properties.

8. **`<XMI.extensions>` content** — `docsettings`, `listview`,
   `codegeneration` sections.

9. **XMI 2.1 format** — the `xmi:id` / `packagedElement` format is not
   supported in M8. The reader detects the format and returns
   `Err(XmiError::UnsupportedVersion)` for non-1.2 files.

10. **XMI writing** — the `XmiWriter` remains a stub.

---

## 14. Future Considerations (M9+)

### 14.1 Attribute and Operation Parsing

In UML 1.2 XMI, attributes and operations appear inside
`<UML:Classifier.feature>`:

```xml
<UML:Class xmi.id="c1" name="Person">
  <UML:Classifier.feature>
    <UML:Attribute xmi.id="a1" name="name" visibility="private">
      <UML:StructuralFeature.type>
        <UML:DataType xmi.idref="int"/>
      </UML:StructuralFeature.type>
    </UML:Attribute>
    <UML:Operation xmi.id="o1" name="getName" visibility="public">
      <UML:BehavioralFeature.parameter>
        <UML:Parameter xmi.id="p1" name="return" visibility="public">
          <UML:Parameter.type>
            <UML:DataType xmi.idref="string"/>
          </UML:Parameter.type>
        </UML:Parameter>
      </UML:BehavioralFeature.parameter>
    </UML:Operation>
  </UML:Classifier.feature>
</UML:Class>
```

The parser must:
1. Enter `<UML:Classifier.feature>` and track the parent classifier.
2. Parse `<UML:Attribute>` → extract name, visibility, type reference.
3. Parse `<UML:Operation>` → extract name, visibility, return type, parameters.
4. Resolve type references (`xmi.idref`) against the `IdMap`.

### 14.2 Type Resolution

In M9, `<UML:DataType>` elements will be parsed and stored in a type registry.
The `TypeReference` struct supports both `model_id` (for classifier references)
and `type_name` (for primitive types). The parser will need to:

1. Build a mapping from `xmi.id` to type name for `<UML:DataType>` elements.
2. When encountering a type reference (e.g., `type="int"` or
   `<UML:DataType xmi.idref="wWhl5MFkeCF5"/>`), look up the mapping.
3. If the type is a primitive, create `TypeReference::primitive(name)`.
4. If the type is a classifier (class/interface), create `TypeReference::model(id)`.

### 14.3 Relationship Parsing

```xml
<UML:Association xmi.id="r1" name="" visibility="public">
  <UML:Association.connection>
    <UML:AssociationEnd xmi.id="end1" visibility="public"
                        multiplicity="1" name="" type="c1"
                        isNavigable="true"/>
    <UML:AssociationEnd xmi.id="end2" visibility="public"
                        multiplicity="0..*" name="" type="c2"
                        isNavigable="false"/>
  </UML:Association.connection>
</UML:Association>
```

Generalizations are simpler:

```xml
<UML:Generalization xmi.id="r2" visibility="public"
                    subtype="c1" supertype="c2"/>
```

### 14.4 Diagram Parsing

The `<diagrams>` section in `<XMI.extensions>` contains widget positions,
edges, and visual properties. This is a separate parsing pass that reads
Umbrello-specific XML embedded in the XMI extensions block.

### 14.5 XMI 2.1 Support

Adding XMI 2.1 reading involves:
1. Detecting `xmi:id` (colon) vs `xmi.id` (dot).
2. Using `uml:` namespace instead of `UML:`.
3. Handling `<packagedElement>` instead of `<UML:Namespace.ownedElement>`.
4. Mapping `xmi:type="uml:Class"` to our element types.
5. Handling the different stereotype syntax (`xmi:value`).

### 14.6 Writer Implementation

The XMI writer will:
1. Iterate `UmlModel` elements in insertion order.
2. Serialise each `ModelElement` variant to the appropriate XML tag.
3. Generate `xmi.id` values (not UUIDs — we want compact IDs for round-trip
   compatibility).
4. Write the containment hierarchy using `<UML:Namespace.ownedElement>`.
5. Write `docsettings`, `diagrams`, `listview`, `codegeneration` sections.
6. Support both UML 1.2 and UML 2.1 output formats.

---

## 15. References

| Reference | Description | File |
|-----------|-------------|------|
| UML 1.2 XMI DTD | OMG specification for UML 1.2 XMI interchange | `docs/specs/xmi-1.2-omg.pdf` |
| Domain Model v1 | Rust UML metamodel design | `docs/domain_model_v1.md` |
| Model Repository v1 | `UmlModel` storage design | `docs/model_repository_v1.md` |
| Relationships v1 | Relationship element design | `docs/relationships_v1.md` |
| Implementation Phases | Milestone breakdown | `planning/implementation_phases.md` |
| Crate Layout | Workspace structure | `planning/crate_layout.md` |
| quick-xml docs | Streaming XML parser API | `https://docs.rs/quick-xml/latest/quick_xml/` |
| C++ XMI saving code | Reference: `umbrello/umlmodel/umlobject.cpp` | C++ `saveToXMI()` / `loadFromXMI()` |
| C++ test files | Example XMI files | `tests/data/test_*.xmi` |

---

> **Document status:** Active for Milestone 8 implementation.
> **Next review:** Upon completion of M8 Stage 5 (real file parsing).
