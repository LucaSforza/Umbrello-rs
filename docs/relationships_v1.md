# UML Relationships v1 — Architecture Evaluation

> **Document:** `rust-rewrite/docs/relationships_v1.md`
> **Status:** Active
> **Phase:** Milestone 5 (uml-core relationships)
> **Last updated:** 2026-06-23
>
> This document evaluates three alternative approaches for representing UML
> relationships (Generalization, Association, Aggregation, Composition,
> Dependency, Realization) in the Rust model. It examines XMI compatibility,
> architectural consistency, implementation complexity, query efficiency, and
> code generation readiness before recommending a design.

---

## Table of Contents

1. [Context](#1-context)
2. [Requirements](#2-requirements)
3. [Current State Assessment](#3-current-state-assessment)
4. [Design Alternatives](#4-design-alternatives)
   - [Alternative A: Relationships as ModelElement Variants](#alternative-a-relationships-as-modelelement-variants)
   - [Alternative B: Relationships as a Separate Edge Store](#alternative-b-relationships-as-a-separate-edge-store)
   - [Alternative C: Graph Library Integration (petgraph)](#alternative-c-graph-library-integration-petgraph)
5. [Comparison Matrix](#5-comparison-matrix)
6. [XMI Compatibility Analysis](#6-xmi-compatibility-analysis)
7. [Query Efficiency Analysis](#7-query-efficiency-analysis)
8. [Rejected Alternatives — Deep Analysis](#8-rejected-alternatives--deep-analysis)
9. [Recommendation: Alternative A](#9-recommendation-alternative-a)
10. [Proposed Implementation](#10-proposed-implementation)
11. [API Design](#11-api-design)
12. [What This Enables](#12-what-this-enables)

---

## 1. Context

### 1.1 The UML Association Problem

UML relationships connect model elements. They describe how classes, interfaces,
and other classifiers interact:

- **Generalization** — inheritance between a subclass and a superclass.
- **Association** — a structural relationship (e.g., `Employee` works for `Company`).
- **Aggregation** — whole-part relationship with shared lifecycle (e.g., `Team`
  contains `Player`s — players can exist without the team).
- **Composition** — whole-part relationship with exclusive lifecycle (e.g.,
  `Order` contains `OrderLine`s — lines are destroyed with the order).
- **Dependency** — a usage relationship (e.g., `Report` depends on `DataStore`).
- **Realization** — interface implementation (e.g., `ArrayList` implements `List`).

In the C++ codebase, associations are first-class `UMLObject` subclasses with
their own identity, name, stereotype, and documentation. They own two `UMLRole`
objects that reference the participant elements. The C++ source acknowledges
this is stored in the wrong place:

```cpp
// UMLCanvasObject.h, TODO comment:
// "Move the list of Associations to the UMLAssociation class itself.
//  It is stored here in UMLCanvasObject only for historical reasons."
UMLAssociationList m_List;
```

The Rust rewrite must avoid this historical baggage while correctly representing
the relationship concepts.

### 1.2 The Rust Domain Model (Current State)

The existing Rust domain model (`domain_model_v1.md`) defines elements using a
flat enum with embedded `ElementBase`:

```rust
pub enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
}

pub struct ElementBase {
    pub id: UmlId,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype_id: Option<UmlId>,
    pub documentation: String,
    pub is_abstract: bool,
    pub is_static: bool,
}
```

Elements are stored in `UmlModel` (in `repository.rs`) which uses
`IndexMap<UmlId, ModelElement>` for O(1) lookup by ID and deterministic
insertion-order iteration.

There is currently no representation of relationships. The domain_model_v1.md
§7.1 sketches a potential design:

```rust
// Sketch from domain_model_v1.md §7.1
struct Association {
    base: ElementBase,
    association_type: AssociationType,
    role_a: Role,
    role_b: Role,
}

struct Role {
    base: ElementBase,
    participant_id: UmlId,
    multiplicity: String,
    is_navigable: bool,
    aggregation: AggregationKind,
}
```

This document evaluates that sketch against alternatives and produces a final
design.

### 1.3 Existing Type Infrastructure

The `AssociationType` enum in `types.rs` already exists with 12 variants derived
from the C++ codebase:

```rust
/// The kind of UML association between two model elements.
///
/// Maps to the C++ `UMLAssociation::AssociationType` enum (minus unused variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssociationType {
    Association,           // Plain structural link
    DirectedAssociation,   // Uni-directional navigation
    Generalization,        // Inheritance ("is-a")
    Realization,           // Interface implementation
    Aggregation,           // Shared whole-part ("has-a")
    Composition,           // Exclusive whole-part ("owns-a")
    Dependency,            // Usage relationship ("uses-a")
    Anchor,                // Note/comment anchor
    Containment,           // UI containment (diagram-only)
    Exception,             // Exception specification
    Category2Parent,       // Categories (UML extension)
    Child2Category,        // Categories (UML extension)
}
```

The six relationship types required for Milestone 5 map directly to existing
enum values:

| Milestone 5 Relationship | `AssociationType` Variant |
|--------------------------|---------------------------|
| Generalization           | `AssociationType::Generalization` |
| Association              | `AssociationType::Association` |
| Aggregation              | `AssociationType::Aggregation` |
| Composition              | `AssociationType::Composition` |
| Dependency               | `AssociationType::Dependency` |
| Realization              | `AssociationType::Realization` |

The remaining variants (`DirectedAssociation`, `Anchor`, `Containment`,
`Exception`, `Category2Parent`, `Child2Category`) are deferred to later
milestones.

---

## 2. Requirements

### 2.1 Functional Requirements

| # | Requirement | Description |
|---|-------------|-------------|
| R1 | **Relationship as element** | Each relationship must reference a source element and a target element by `UmlId`. |
| R2 | **Query by participant** | Must support "find all relationships where element X participates". |
| R3 | **Type-specific query** | Must support "find all generalizations of element X" and "find all realizations of element X". |
| R4 | **Validation** | Relationship references must be validated against dangling element references. |
| R5 | **Repository integration** | Relationships must be storable and retrievable from `UmlModel`. |
| R6 | **Serde serialization** | Full serde `Serialize` / `Deserialize` support via derive. |

### 2.2 Non-Functional Requirements

| # | Requirement | Target |
|---|-------------|--------|
| NFR1 | **Model size** | 100–5,000 elements, up to 2,000 relationships per model. |
| NFR2 | **Query latency** | Sub-millisecond for "all relationships of element X" at 2,000 relationships. |
| NFR3 | **Validation latency** | Full model validation in <10ms for 5,000 elements + 2,000 relationships. |
| NFR4 | **XMI compatibility** | Must map cleanly to `<UML:Association>`, `<UML:Generalization>`, etc. in XMI format. |
| NFR5 | **Implementation budget** | Minimal new code — prefer extending existing patterns over new concepts. |
| NFR6 | **Backward compatible** | Existing `ModelElement`, `UmlModel`, `NamedElement` APIs must not break. |

### 2.3 Key Architectural Invariant

> **The UmlModel repository owns all elements. Relationships reference
> participants by `UmlId` — they do not own them.**

This is the same invariant that governs packages and their children.
`UmlId` references are weak by nature — no circular ownership.

---

## 3. Current State Assessment

### 3.1 What Exists Now

The current codebase has these relevant components:

```
crates/uml-core/src/
├── id.rs              — UmlId (UUID wrapper)
├── types.rs           — AssociationType, ObjectType, Visibility, etc.
├── elements.rs        — ModelElement enum (4 variants), Package, Class, etc.
├── repository.rs      — UmlModel with IndexMap<UmlId, ModelElement>
├── traits.rs          — NamedElement trait
└── lib.rs             — Public exports
```

### 3.2 What Is Missing

| Component | Status |
|-----------|--------|
| Relationship struct | Not defined |
| Relationship variant on ModelElement | Not added |
| AssociationType→Relationship mapping | AssociationType exists but not wired into element storage |
| Relationship query API on UmlModel | Not implemented |
| validate_references() for relationships | Not implemented |
| Serde derive for relationships | Blocked on struct definition |

### 3.3 Key Design Decisions Already Made

1. **Flat enum dispatch.** `ModelElement` is a tagged union, not a class hierarchy.
   Adding a variant is a local change.

2. **ID-based references.** All cross-element references use `UmlId`, not pointers.
   This avoids circular ownership and enables safe validation.

3. **ElementBase provides metadata.** Common fields (name, documentation,
   stereotype, visibility) are embedded in every model element.

4. **IndexMap storage.** `UmlModel` uses `IndexMap<UmlId, ModelElement>` for
   O(1) lookup and deterministic iteration.

These decisions constrain the relationship design — which is good. A design that
fits the existing patterns will be simpler, safer, and faster to implement.

---

## 4. Design Alternatives

### Alternative A: Relationships as ModelElement Variants

Add `Relationship` as a new `ModelElement` variant. Each relationship is a
first-class element with its own `UmlId` and `ElementBase`.

```rust
/// A UML relationship between two model elements.
///
/// Relationships are first-class model elements with their own UmlId,
/// name, stereotype, and documentation. They connect a source element
/// to a target element via UmlId references.
///
/// # UML Mapping
///
/// | AssociationType    | UML Element           | Semantics                              |
/// |--------------------|-----------------------|----------------------------------------|
/// | Generalization     | `<UML:Generalization>` | "is-a" — class to superclass          |
/// | Association        | `<UML:Association>`    | "knows-a" — structural reference      |
/// | Aggregation        | `<UML:Association>`    | "has-a" — shared whole-part           |
/// | Composition        | `<UML:Association>`    | "owns-a" — exclusive whole-part       |
/// | Dependency         | `<UML:Dependency>`     | "uses-a" — usage relationship         |
/// | Realization        | `<UML:Realization>`    | "implements" — interface realization  |
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Common element metadata (id, name, documentation, stereotype, etc.).
    pub base: ElementBase,

    /// The kind of relationship.
    pub kind: AssociationType,

    /// The source element (e.g., the subclass in a generalization).
    pub source_id: UmlId,

    /// The target element (e.g., the superclass in a generalization).
    pub target_id: UmlId,

    /// Multiplicity at the source end (e.g., "1", "0..*", "1..*").
    pub source_multiplicity: Option<String>,

    /// Multiplicity at the target end.
    pub target_multiplicity: Option<String>,

    /// Role name at the source end (e.g., "employee" in a "works for" association).
    pub source_role_name: Option<String>,

    /// Role name at the target end (e.g., "employer" in a "works for" association).
    pub target_role_name: Option<String>,

    /// Whether navigation from source to target is supported.
    pub source_to_target_navigable: bool,

    /// Whether navigation from target to source is supported.
    pub target_to_source_navigable: bool,
}
```

Add a single variant to `ModelElement`:

```rust
pub enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
    Relationship(Relationship),  // NEW
}
```

**Pros:**

- **Consistent with existing architecture.** `Relationship` is just another
  element type. It embeds `ElementBase`, has its own `UmlId`, and lives in the
  same `IndexMap` as packages and classes.

- **Automatic repository integration.** `UmlModel::insert()` / `get()` /
  `remove()` work without changes. The existing `IndexMap<UmlId, ModelElement>`
  accepts the new variant natively.

- **Automatic reference validation.** `validate_references()` already checks
  `Package::children`, `Attribute::type_id`, `Operation::return_type_id`. Adding
  checks for `source_id` and `target_id` is a natural extension — same pattern,
  same `ReferenceField` enum extension.

- **XMI compatibility.** UML associations are serialized as sibling elements
  at the same level as classes and packages:
  ```xml
  <UML:Namespace.ownedElement>
    <UML:Class xmi.id="1" name="Person"/>
    <UML:Class xmi.id="2" name="Company"/>
    <UML:Association xmi.id="3" name="works for">  <!-- sibling -->
      <UML:Association.connection>
        <UML:AssociationEnd participant="1"/>
        <UML:AssociationEnd participant="2"/>
      </UML:Association.connection>
    </UML:Association>
  </UML:Namespace.ownedElement>
  ```
  With Alternative A, this maps 1:1: `Relationship` variant → `<UML:Association>`
  element. No adapter layer needed.

- **Association metadata is free.** `ElementBase` provides name, documentation,
  stereotype, visibility — all of which are valid on UML associations. UML allows
  associations to have names (e.g., "works for"), documentation, and stereotypes
  (e.g., `<<friend>>`, `<<create>>`).

- **Simple query API.** `model.iter()` can filter by `ObjectType::Relationship`:
  ```rust
  fn relationships_of(&self, element_id: UmlId) -> Vec<&Relationship> {
      self.iter()
          .filter_map(|(_, e)| match e {
              ModelElement::Relationship(r) => Some(r),
              _ => None,
          })
          .filter(|r| r.source_id == element_id || r.target_id == element_id)
          .collect()
  }
  ```

- **Maps to C++ pattern.** The C++ codebase treats `UMLAssociation` as a
  `UMLObject` subclass. Alternative A follows the same conceptual model without
  inheriting the bugs (the misplaced `m_List` is eliminated by design).

**Cons:**

- **Larger ModelElement enum.** Adding a variant is a local change — the
  compiler enforces exhaustiveness. This is a feature, not a bug.

- **Nodes and edges mixed.** Relationships and element nodes share the same
  flat storage. They are semantically different (`Package` is a container;
  `Relationship` is a connection). However, the storage is just a map — the
  semantic difference is captured by the variant tag.

- **O(n) query for participant lookup.** Finding all relationships for element X
  requires scanning all relationships. For <10,000 relationships, this is
  sub-millisecond (see §7).

- **Name is sometimes meaningless.** Association names are often empty in typical
  UML models. `ElementBase::name` exists, which is fine — it just happens to be
  empty for many associations. No different from the C++ approach.

---

### Alternative B: Relationships as a Separate Edge Store

Keep relationships out of `ModelElement`. Instead, store them in a dedicated
`Vec<Relationship>` in `UmlModel`.

```rust
pub struct UmlModel {
    /// Node storage: model elements that are not relationships.
    elements: IndexMap<UmlId, ModelElement>,
    /// Edge storage: relationships between elements.
    relationships: Vec<Relationship>,
    /// Optional: adjacency index for fast relationship queries.
    adjacency_index: Option<HashMap<UmlId, Vec<usize>>>,
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}

/// A relationship stored separately from model elements.
///
/// NOT a ModelElement variant. Relationships live in a dedicated edge store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Unique identifier (also used in XMI for xmi.id references).
    pub id: UmlId,
    /// The kind of relationship.
    pub kind: RelationshipType,
    /// The source element.
    pub source_id: UmlId,
    /// The target element.
    pub target_id: UmlId,
    /// Optional name (derived from role names, typically).
    pub name: Option<String>,
    /// Optional stereotype reference.
    pub stereotype_id: Option<UmlId>,
    /// Documentation text.
    pub documentation: String,
    /// Multiplicity at the source end.
    pub source_multiplicity: Option<String>,
    /// Multiplicity at the target end.
    pub target_multiplicity: Option<String>,
    /// Role name at the source end.
    pub source_role_name: Option<String>,
    /// Role name at the target end.
    pub target_role_name: Option<String>,
    /// Navigation flags.
    pub source_to_target_navigable: bool,
    pub target_to_source_navigable: bool,
}

/// A subset of AssociationType for edges only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationshipType {
    Generalization,
    Association,
    Aggregation,
    Composition,
    Dependency,
    Realization,
}
```

**Pros:**

- **Clean graph model.** Nodes and edges are separate concepts. `ModelElement`
  stays focused on "things" (classes, packages, interfaces).

- **Efficient indexing.** The adjacency index can be maintained incrementally:
  ```rust
  fn add_relationship(&mut self, rel: Relationship) {
      let source = rel.source_id;
      let target = rel.target_id;
      self.relationships.push(rel);
      let idx = self.relationships.len() - 1;
      self.adjacency_index.entry(source).or_default().push(idx);
      self.adjacency_index.entry(target).or_default().push(idx);
  }
  ```

- **Better for code generation.** "Generate member variables from associations"
  becomes a direct adjacency query rather than a filtered scan.

- **Diagram rendering.** Edge data is naturally separated from node data.

- **RelationshipType is focused.** Only the six needed relationship types
  (no `Anchor`, `Containment`, `Exception`, etc.).

**Cons:**

- **Second storage container.** Requires a `Vec<Relationship>` in addition to
  `IndexMap<UmlId, ModelElement>`. Two containers = two APIs.

- **Second ID space.** Relationships need `UmlId` values (for XMI references)
  but aren't in the same map as elements. This means some code paths use
  `model.get(id)` and others use a hypothetical `model.get_relationship(id)`.

- **XMI adapter required.** XMI serializes associations as sibling elements in
  `UML:Namespace.ownedElement`. With Alternative B, the XMI writer must
  interleave elements from two containers into a single output stream. The
  XMI reader must split elements into two containers. This is error-prone.

- **Reference validation is dual-source.** `validate_references()` must check
  `elements` for node references AND `relationships` for source/target
  references. Two loops, two code paths.

- **Field duplication.** The `Relationship` struct duplicates fields that
  `ElementBase` already provides (name, documentation, stereotype_id,
  visibility). This violates DRY — if `ElementBase` changes, `Relationship`
  must be updated in parallel.

- **New RelationshipType enum.** Partially duplicates `AssociationType`. If
  we reuse `AssociationType`, we need to decide which variants are valid for
  edges vs. other purposes. If we create a new enum, we have two enums that
  overlap. Neither choice is clean.

---

### Alternative C: Graph Library Integration (petgraph)

Use `petgraph` as the underlying storage for both elements and relationships.

```rust
use petgraph::stable_graph::{StableGraph, NodeIndex, EdgeIndex};

pub struct UmlModel {
    /// Graph with ModelElement as nodes.
    graph: StableGraph<ModelElement, RelationshipData>,
    /// Maps UmlId to NodeIndex within the graph.
    id_to_node: HashMap<UmlId, NodeIndex>,
    /// Reverse: NodeIndex → UmlId.
    node_to_id: HashMap<NodeIndex, UmlId>,
    /// Parent index (same as current).
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipData {
    /// The kind of relationship.
    pub kind: AssociationType,
    /// Multiplicity at the source end.
    pub source_multiplicity: Option<String>,
    /// Multiplicity at the target end.
    pub target_multiplicity: Option<String>,
    /// Role names.
    pub source_role_name: Option<String>,
    pub target_role_name: Option<String>,
    /// Navigation flags.
    pub source_to_target_navigable: bool,
    pub target_to_source_navigable: bool,
}
```

**Pros:**

- **Built-in graph algorithms.** `petgraph` provides shortest paths, cycle
  detection (critical for containment hierarchy validation), topological sort
  (useful for code generation ordering), and graph traversal.

- **Efficient adjacency queries.** `graph.neighbors(node_index)` returns all
  connected nodes in O(degree) — the natural operation for "find all
  relationships for element X".

- **Enforced separation.** The library separates nodes and edges at the type
  level. There's no way to accidentally store a relationship as a node.

- **Already a workspace dependency.** `petgraph` is listed in the workspace
  `Cargo.toml`.

**Cons:**

- **Two parallel ID systems.** External code uses `UmlId` (UUID), internal
  storage uses `NodeIndex` (usize). Every lookup requires going through
  `id_to_node`: `graph.node_weight(id_to_node[&umlid])`. This is the same
  double-lookup problem identified in the repository evaluation for SlotMap.

- **NodeIndex instability.** Removing a node shifts indices of subsequent
  nodes — unless using `StableGraph`. The design uses `StableGraph`, which
  solves this but uses more memory (tombstone slots for removed nodes).

- **Serialization friction.** `petgraph` graphs are not directly serializable
  with serde. Node indices are session-local. Custom serialization code is
  required to convert `NodeIndex` ↔ `UmlId` on every save/load cycle.

- **XMI disconnect.** XMI format is a tree-oriented XML structure, not a
  graph. Converting a `StableGraph` to XMI requires traversing the graph and
  emitting elements in XMI-compatible order. Conversely, parsing XMI into a
  graph requires inserting nodes, then adding edges after all nodes are known.

- **Three-way index management.** Need to keep `id_to_node`, `node_to_id`,
  and `parent_index` in sync with the graph — four data structures that must
  agree. A bug in any one update produces silent corruption.

- **Over-engineering.** UML models typically have at most hundreds of
  relationships. A linear scan over a `Vec<Relationship>` (Alternative A or B)
  completes in microseconds. Adding a full graph library for O(degree) queries
  that are already O(100) with a simple scan is premature optimization.

- **Removes the simple `model.get(id)` pattern.** Currently, any element is
  one lookup away. With petgraph, element access becomes:
  ```rust
  model.graph.node_weight(*model.id_to_node.get(&id)?)
  ```
  This is harder to read and write correctly.

---

## 5. Comparison Matrix

| Criterion | Weight | A (ModelElement) | B (Edge Store) | C (petgraph) |
|-----------|--------|------------------|----------------|--------------|
| **Consistency with existing arch** | Critical | ✅ Natural fit | ⚠️ New container concept | ❌ Two ID systems |
| **XMI compatibility** | Critical | ✅ 1:1 mapping to XMI elements | ⚠️ Needs mapping layer | ❌ Graph→XMI adapter required |
| **Repository integration** | Critical | ✅ Automatic (same IndexMap) | ⚠️ New Vec + new API | ❌ New API + parallel indices |
| **Reference validation** | Critical | ✅ Extends existing logic | ⚠️ Dual-source check | ❌ Triple-source (graph + 2 maps) |
| **Serde serialization** | Critical | ✅ Derive on struct | ✅ Derive on struct | ⚠️ Custom serialization |
| **Association metadata** | Critical | ✅ ElementBase (name, doc, stereotype, visibility) | ⚠️ Fields duplicated on Relationship | ⚠️ Fields duplicated on RelationshipData |
| **Query efficiency** | High | ⚠️ O(n) scan (n ≤ 2,000) | ✅ O(degree) with index | ✅ O(degree) |
| **Code generation readiness** | High | ⚠️ Filter by type | ✅ Dedicated query API | ✅ Graph walks |
| **Diagram rendering** | High | ⚠️ Filter by Relationship type | ✅ Separate layer | ✅ Separate layer |
| **Implementation complexity** | High | ✅ ~80 lines + 2 match arms | ⚠️ ~150 lines + new concepts | ❌ ~200 lines + petgraph semantics |
| **Memory** | Medium | ✅ Same as any element | ✅ Lightweight (Vec + optional index) | ⚠️ StableGraph tombstone overhead |
| **Maintainability** | High | ✅ One pattern (ElementBase + variant) | ⚠️ Split across two containers | ❌ Four containers to sync |
| **Cycle detection** | Low | ✅ Manual (algorithmic) | ✅ Manual (algorithmic) | ✅ Built-in (petgraph::algo) |

**Weighted assessment:**

| Alternative | Critical ✅ | Critical ⚠️ | Critical ❌ | Verdict |
|-------------|-------------|--------------|-------------|---------|
| **A (ModelElement)** | 4 | 1 | 0 | **Strongest** — passes all critical criteria |
| **B (Edge Store)** | 1 | 4 | 0 | Mixed — fails XMI and metadata criteria |
| **C (petgraph)** | 0 | 2 | 3 | **Rejected** — fails 3/5 critical criteria |

---

## 6. XMI Compatibility Analysis

This section is critical because XMI round-tripping (save → load → compare) is
the primary integration test for the Umbrello-RS project.

### 6.1 XMI Structure for UML Associations

In standard XMI (UML 1.4 / XMI 1.2, used by Umbrello C++), associations,
generalizations, and dependencies are serialized as first-class elements within
`UML:Namespace.ownedElement`:

```xml
<UML:Model xmi.id="model1" name="MyModel">
  <UML:Namespace.ownedElement>
    <!-- Model elements (classes, interfaces, etc.) -->
    <UML:Class xmi.id="1" name="Person"/>
    <UML:Class xmi.id="2" name="Company"/>

    <!-- Association — same level as Class, not nested -->
    <UML:Association xmi.id="3" name="works for">
      <UML:Association.connection>
        <UML:AssociationEnd
          xmi.id="4"
          visibility="public"
          participant="1"               <!-- references Person by UmlId -->
          multiplicity="1"
          name="employee"
          isNavigable="true"/>
        <UML:AssociationEnd
          xmi.id="5"
          visibility="public"
          participant="2"               <!-- references Company by UmlId -->
          multiplicity="1..*"
          name="employer"
          isNavigable="true"/>
      </UML:Association.connection>
    </UML:Association>

    <!-- Generalization — also same level -->
    <UML:Generalization xmi.id="6" name="">
      <UML:Generalization.child>1</UML:Generalization.child>
      <UML:Generalization.parent>3</UML:Generalization.parent>
    </UML:Generalization>

    <!-- Realization — also same level -->
    <UML:Realization xmi.id="7" name="">
      <UML:Realization.client>2</UML:Realization.client>
      <UML:Realization.supplier>3</UML:Realization.supplier>
    </UML:Realization>

    <!-- Dependency — also same level -->
    <UML:Dependency xmi.id="8" name="">
      <UML:Dependency.client>1</UML:Dependency.client>
      <UML:Dependency.supplier>2</UML:Dependency.supplier>
    </UML:Dependency>
  </UML:Namespace.ownedElement>
</UML:Model>
```

Key observations:

1. **Associations are sibling elements.** They appear at the same nesting level
   as `UML:Class`, `UML:Interface`, etc. There is no separate "edge section"
   in XMI.

2. **Associations have their own `xmi.id`.** This ID is used in other elements'
   references (e.g., a diagram widget might reference the association by ID).

3. **Associations can have metadata.** `name`, `visibility`,
   `stereotype`, and documentation are all valid attributes on
   `<UML:Association>`, `<UML:Generalization>`, `<UML:Dependency>`, etc.

4. **Association ends are nested.** Each association has connection ends
   (`UML:AssociationEnd`) as child elements. These are not standalone elements
   — they are owned by the association.

### 6.2 Mapping Analysis

| XMI Element | Alternative A (ModelElement) | Alternative B (Edge Store) | Alternative C (petgraph) |
|---|---|---|---|
| `<UML:Association>` | `ModelElement::Relationship` with `AssociationType::Association` | `Relationship` in edge store | Edge in `StableGraph` |
| `<UML:Generalization>` | Same as above, `AssociationType::Generalization` | Same | Same |
| `<UML:Realization>` | Same as above, `AssociationType::Realization` | Same | Same |
| `<UML:Dependency>` | Same as above, `AssociationType::Dependency` | Same | Same |
| `<UML:AssociationEnd>` | Fields on `Relationship` (multiplicity, role, navigability) | Fields on `Relationship` | Fields on `RelationshipData` |
| List of all elements in `ownedElement` | `model.iter()` — single container, single loop | Must merge `elements` + `relationships` into a single interleaved stream | Must extract all elements from graph nodes + edges |
| Element ordering in XMI | Insertion order = emit order | Must define merge strategy (elements first? interleaved?) | Must define traversal order |
| Reference via `xmi.id` | `model.get(xmi_id)` — same API for all elements | Two APIs: `model.get()` for nodes, `model.get_relationship()` for edges | `graph.node_weight(id_to_node[&xmi_id])` — indirect |

### 6.3 Verdict

**Alternative A is the only approach that maps 1:1 to XMI without an adapter
layer.** The XMI writer iterates `model.iter()` and emits each element
according to its variant — exactly as it would for classes and packages. The
XMI reader deserializes `<UML:Association>` elements into
`ModelElement::Relationship`, same as it deserializes `<UML:Class>` into
`ModelElement::Class`. No merging, no splitting, no interleaving.

Alternative B requires the XMI writer to merge two containers into a single
output stream. The XMI reader must split the input into two containers. This
is a constant source of bugs — "did an association end up in the wrong
container?" is a question that should not exist.

Alternative C requires converting between graph indices and UmlIds for every
element during both read and write.

### 6.4 AssociationEnd ID Strategy

XMI 1.2 requires each `<UML:AssociationEnd>` element to have its own
`xmi.id` attribute. The mapping strategy is:

- **Association `xmi.id`:** The `Relationship`'s own `UmlId` is used as the
  parent `<UML:Association>` `xmi.id`.
- **AssociationEnd `xmi.id` values:** Derived from the `Relationship`'s `UmlId`
  with a deterministic suffix — e.g., `{uuid}-end-a` for the source end and
  `{uuid}-end-b` for the target end.

This derivation is handled entirely in the `uml-xmi` crate during serialization
and deserialization. The domain model (`Relationship` struct) stores only
`source_id` and `target_id` — it has no knowledge of AssociationEnd identifiers.
The suffix convention guarantees uniqueness across the model because each
`UmlId` is globally unique.

---

## 7. Query Efficiency Analysis

### 7.1 The O(n) Concern

Alternative A requires scanning all relationships to find those for a given
element:

```rust
fn relationships_of(&self, element_id: UmlId) -> Vec<&Relationship> {
    self.iter()
        .filter_map(|(_, e)| match e {
            ModelElement::Relationship(r) => Some(r),
            _ => None,
        })
        .filter(|r| r.source_id == element_id || r.target_id == element_id)
        .collect()
}
```

Complexity: **O(r)** where r = total number of relationships in the model.

### 7.2 Real-World Performance

| Model size | Relationships (r) | Scan time (estimated) | Acceptable? |
|------------|-------------------|-----------------------|-------------|
| Small (10 classes) | 15 | ~0.1 µs | ✅ Instant |
| Medium (100 classes) | 150 | ~1 µs | ✅ Instant |
| Large (500 classes) | 1,000 | ~8 µs | ✅ Sub-frame |
| Very large (2,000 classes) | 5,000 | ~40 µs | ✅ Sub-frame |
| Extreme (10,000 classes) | 20,000 | ~160 µs | ⚠️ Still sub-millisecond |

Even at 20,000 relationships, a scan takes ~0.16 ms — well under the 16 ms
budget for 60 fps rendering and far below 1 second for a code generation pass.

### 7.3 Comparison Across Alternatives

| Operation | A (ModelElement) | B (Edge Store) | C (petgraph) |
|-----------|------------------|----------------|--------------|
| All relationships of element X | O(r) scan | O(degree) via index | O(degree) |
| Generalizations of element X | O(r) scan + type filter | O(degree) via index + type filter | O(degree) |
| All participants connected to element X | O(r) scan | O(degree) | O(degree) |
| Cycles in generalization hierarchy | O(r + n) DFS | O(r + n) DFS | O(r + n) built-in |
| Topological sort for code gen | O(r + n) Kahn's | O(r + n) Kahn's | O(r + n) built-in |
| Insert relationship | O(1) | O(1) + index update | O(1) + 2 map updates |
| Remove relationship | O(1) removal + O(r) for index rebuild | O(1) removal + index update | O(1) + 2 map updates |

### 7.4 Mitigation: Lazy Adjacency Index

If profiling ever shows that O(r) scan is a bottleneck, a lazy adjacency index
can be added to `UmlModel` **without changing the data model**:

```rust
impl UmlModel {
    /// Ensure the adjacency index is built.
    /// Called automatically on first query that needs it.
    fn ensure_adjacency_index(&mut self) {
        if self.adjacency_index.is_some() {
            return;
        }
        let mut index: HashMap<UmlId, Vec<UmlId>> = HashMap::new();
        for (id, element) in self.iter() {
            if let ModelElement::Relationship(r) = element {
                index.entry(r.source_id).or_default().push(id);
                index.entry(r.target_id).or_default().push(id);
            }
        }
        self.adjacency_index = Some(index);
    }

    /// Invalidate the index when a relationship is added or removed.
    fn invalidate_adjacency_index(&mut self) {
        self.adjacency_index = None;
    }
}
```

This is a zero-risk optimisation:
- The data model (ModelElement enum + Relationship struct) doesn't change.
- The public API (`relationships_of()`) doesn't change — only its internal
  implementation.
- The index is rebuilt lazily — wasted work if queries are never called.
- Building the index is O(r) — same as one full scan — and happens at most
  once between mutations.

**Alternative A provides the simplest path to the correct design, with a
clear, optional optimisation path if needed.**

---

## 8. Rejected Alternatives — Deep Analysis

### 8.1 Why NOT Alternative B (Separate Edge Store)

Alternative B was rejected for four decisive reasons:

**Reason 1: XMI Adapter Complexity**

XMI serialization must produce a single ordered list of elements in
`UML:Namespace.ownedElement`. With Alternative B, the serializer must:

```rust
// Necessary but error-prone merge logic for Alternative B
fn serialize_owned_elements(
    elements: &IndexMap<UmlId, ModelElement>,
    relationships: &[Relationship],
    writer: &mut XmiWriter,
) {
    // Problem: in what order should elements and relationships appear?
    // Option 1: All elements first, then all relationships
    //   → This may violate tool expectations (Umbrello C++ emits in
    //     insertion order, interleaved)
    // Option 2: Interleave by insertion timestamp (but relationships
    //     don't have insertion order in a Vec)
    // Option 3: Sort by UmlId (UUID) — arbitrary, meaningless order
    //
    // Worst case: a third tool expects a specific interleaving pattern,
    // and our merged output is unparseable by it.
}
```

With Alternative A, there is no merge step:

```rust
fn serialize_owned_elements(
    model: &UmlModel,
    writer: &mut XmiWriter,
) {
    for (_id, element) in model.iter() {  // Single loop
        writer.write_element(element);     // Single dispatch
    }
}
```

**Reason 2: Dual API Surface**

Alternative B introduces a separate API for relationships:

```rust
// Alternative B: two sets of methods
impl UmlModel {
    // For nodes (existing):
    pub fn insert(&mut self, element: ModelElement) -> Option<ModelElement>;
    pub fn get(&self, id: UmlId) -> Option<&ModelElement>;

    // For edges (new, separate):
    pub fn add_relationship(&mut self, rel: Relationship);
    pub fn get_relationship(&self, id: UmlId) -> Option<&Relationship>;
    pub fn relationships_of(&self, element_id: UmlId) -> Vec<&Relationship>;
    pub fn remove_relationship(&mut self, id: UmlId) -> Option<Relationship>;
}
```

Every user of the model must learn which API applies to which kind of data.
This is a conceptual tax that lasts for the lifetime of the project.

Alternative A uses one API for everything:

```rust
// Alternative A: unified API (existing, no changes)
impl UmlModel {
    pub fn insert(&mut self, element: ModelElement) -> Option<ModelElement>;
    pub fn get(&self, id: UmlId) -> Option<&ModelElement>;
    pub fn relationships_of(&self, element_id: UmlId) -> Vec<&Relationship>;
    // remove() already handles all element types
}
```

**Reason 3: Field Duplication**

The `Relationship` struct in Alternative B must duplicate fields from
`ElementBase`:

```rust
// Alternative B — duplicated fields (violates DRY)
pub struct Relationship {
    pub id: UmlId,
    pub name: Option<String>,
    pub stereotype_id: Option<UmlId>,
    pub documentation: String,
    // ... relationship-specific fields ...
}

// Alternative A — no duplication, ElementBase provides all metadata
pub struct Relationship {
    pub base: ElementBase,  // id, name, documentation, stereotype, etc.
    pub kind: AssociationType,
    pub source_id: UmlId,
    pub target_id: UmlId,
    // ... relationship-specific fields only ...
}
```

If a future milestone adds a field to `ElementBase` (e.g., `keywords`, `url`,
`applied_profile`), Alternative B's `Relationship` must be updated manually.
Alternative A gets the field for free via `ElementBase`.

**Reason 4: New RelationshipType Enum**

Alternative B requires a new `RelationshipType` enum because the existing
`AssociationType` includes variants that don't make sense for edges
(`Anchor`, `Containment`, `Exception`, etc.):

```rust
// Alternative B: must define this enum (partial duplicate)
pub enum RelationshipType {
    Generalization,
    Association,
    Aggregation,
    Composition,
    Dependency,
    Realization,
}
```

This means either:
- We have two enums that overlap semantically (maintenance burden), or
- We use `AssociationType` directly but must document which variants are valid
  for edges (runtime errors if invalid variants are used).

Alternative A uses `AssociationType` directly because relationships ARE
associations — the enum name matches the domain concept:

```rust
// Alternative A: direct use of existing AssociationType
pub struct Relationship {
    pub base: ElementBase,
    pub kind: AssociationType,  // Only the 6 valid variants
    // ...
}
```

The only downside is that `AssociationType` includes 12 variants but only 6
are used for relationships. This is a minor documentation concern, not a
correctness issue — the unused variants simply never appear in a Relationship.

---

### 8.2 Why NOT Alternative C (petgraph)

Alternative C was rejected for five decisive reasons:

**Reason 1: Four Data Structures to Synchronize**

```rust
// Alternative C — four containers that must agree
pub struct UmlModel {
    graph: StableGraph<ModelElement, RelationshipData>,
    id_to_node: HashMap<UmlId, NodeIndex>,     // Must match graph node set
    node_to_id: HashMap<NodeIndex, UmlId>,     // Inverse of id_to_node
    parent_index: HashMap<UmlId, Vec<UmlId>>,  // Independent index
}
```

Every mutation must update all four containers consistently:

```rust
fn insert(&mut self, element: ModelElement) -> UmlId {
    let id = element.id();
    let node_idx = self.graph.add_node(element);

    // These must ALL succeed:
    assert!(self.id_to_node.insert(id, node_idx).is_none(),
        "ID collision");
    assert!(self.node_to_id.insert(node_idx, id).is_none(),
        "node index collision");

    // parent_index is maintained separately
    id
}
```

A bug in any single update produces silent data corruption. Testing must
verify that all four containers are consistent after every mutation.

**Reason 2: The Lookup Tax**

Every `model.get(id)` requires:

```rust
fn get(&self, id: UmlId) -> Option<&ModelElement> {
    let node_idx = self.id_to_node.get(&id)?;  // HashMap lookup #1
    self.graph.node_weight(*node_idx)           // StableGraph lookup #2
}
```

This is the same double-lookup problem identified in the repository evaluation
for SlotMap. For a diagram rendering 100 widgets at 60fps, that's 12,000 extra
HashMap lookups per second.

**Reason 3: Serialization Friction**

`NodeIndex` and `EdgeIndex` are not serializable across sessions — they are
memory addresses (slot positions) that change between process runs. Every
serialization must convert indices to `UmlId` values. Every deserialization
must insert nodes in correct order and rebuild indices.

This is fundamentally at odds with the workspace's approach of `#[derive(Serialize, Deserialize)]`
on all domain types.

**Reason 4: Over-engineering for the Use Case**

petgraph is valuable when you need:
- **Shortest path** between two classes (not useful in UML modeling)
- **Minimum spanning tree** over the inheritance hierarchy (not useful)
- **Maximum flow** / min cut (not useful)
- **Graph isomorphism** (not useful — UML model comparison is done by ID)

What UML modeling actually needs:
- **Topological sort** for code generation (manual Kahn's is ~15 lines)
- **Cycle detection** for inheritance (manual DFS is ~20 lines)
- **Find all relationships of element X** (O(degree) with index, O(r) without)

None of these require a graph library. The algorithms are trivial to implement
and the pay-off from a dependency is negative when the dependency creates
parallel data structures.

**Reason 5: Architectural Mismatch**

The entire Umbrello-RS architecture uses `UmlId` as the universal identity.
petgraph uses `NodeIndex` / `EdgeIndex`. Every interaction between the model
and external code (diagrams, XMI serialization, code generation, CLI) uses
`UmlId`. Introducing a second identity system creates translation
opportunities for bugs throughout the stack.

---

### 8.3 Considered but not Evaluated: Hybrid Approach

An intermediate approach was briefly considered: store relationships as
`ModelElement` variants (Alternative A) **and** maintain a separate adjacency
cache for performance (Alternative B's index). This is effectively Alternative
A with lazy adjacency indexing (as described in §7.4).

This is not a separate alternative — it is Alternative A with the optional
optimisation path that the recommendation already includes. It was considered
and folded into the recommendation.

---

## 9. Recommendation: Alternative A

### 9.1 Decisive Factors

**Factor 1: XMI Compatibility (Critical)**

XMI serializes associations as sibling elements alongside classes and packages.
Alternative A maps 1:1 to XMI — no merge step, no split step, no order
consistency problem. Alternative B requires merging two containers. Alternative C
requires converting between two ID systems. For a project where XMI round-tripping
is the primary integration test, Alternative A is the only viable choice.

**Factor 2: Architectural Consistency (Critical)**

The existing pattern is `ElementBase` + `ModelElement` variant + `UmlModel`
storage. Alternative A extends this pattern without introducing any new concepts.
Alternative B introduces a second container, a second ID scope, and a second
API surface. Alternative C introduces a graph library with four data structures
to synchronize.

**Factor 3: Association Metadata (Critical)**

UML associations can have names, documentation, stereotypes, and visibility.
Alternative A provides all of these for free via `ElementBase`. Alternative B
must duplicate these fields on the `Relationship` struct. Alternative C must
duplicate them on `RelationshipData`.

**Factor 4: Implementation Simplicity (High)**

| Aspect | A (ModelElement) | B (Edge Store) | C (petgraph) |
|--------|------------------|----------------|--------------|
| New struct definitions | 1 (`Relationship`) | 2 (`Relationship` + `RelationshipType`) | 1 (`RelationshipData`) |
| ModelElement changes | +1 variant | 0 | 0 |
| Match arm additions | +2 (base, object_type) | 0 | 0 |
| UmlModel changes | +3 query methods | +4 methods + storage | +4 methods + 3 maps |
| Code generation changes | Filter by variant | Filter by store type | Filter by edge type |
| Total estimated LOC | ~80 | ~150 | ~200 |

### 9.2 What About Query Performance?

The O(r) scan for participant queries is acceptable for realistic model sizes.
If profiling shows it's a bottleneck, the lazy adjacency index (§7.4) is a
transparent, zero-risk optimisation that does not change the data model.

**The correct sequence is:**
1. Implement Alternative A (simple, correct, no optimisation).
2. Ship Milestone 5 with the simple implementation.
3. Profile if and when performance problems arise.
4. Add the adjacency index optimisation as needed (an internal detail).

### 9.3 Summary

| Criteria | Assessment |
|----------|------------|
| **XMI compatibility** | ✅ 1:1 mapping — no adapter layer needed |
| **Architectural fit** | ✅ Extends existing ModelElement + ElementBase pattern |
| **Metadata support** | ✅ Free via ElementBase |
| **Serde support** | ✅ Derive on Relationship + existing derives on ModelElement |
| **Query efficiency** | ⚠️ O(r) scan — acceptable; mitigation path exists |
| **Implementation cost** | ✅ ~80 LOC — cheapest alternative |
| **Long-term maintenance** | ✅ One pattern for all elements — lowest cognitive load |

---

## 10. Proposed Implementation

### 10.1 Relationship Struct (in `elements.rs`)

```rust
/// A UML relationship between two model elements.
///
/// Relationships are first-class model elements with their own `UmlId`,
/// name, stereotype, and documentation. They connect a source element
/// to a target element via `UmlId` references.
///
/// The `kind` field determines the UML element kind:
///
/// | AssociationType    | XMI Element           | Semantics                            |
/// |--------------------|-----------------------|--------------------------------------|
/// | Generalization     | `<UML:Generalization>` | "is-a" — subclass to superclass     |
/// | Association        | `<UML:Association>`    | "knows-a" — structural link         |
/// | Aggregation        | `<UML:Association>`    | "has-a" — shared whole-part         |
/// | Composition        | `<UML:Association>`    | "owns-a" — exclusive whole-part     |
/// | Dependency         | `<UML:Dependency>`     | "uses-a" — usage relationship       |
/// | Realization        | `<UML:Realization>`    | "implements" — interface realization|
///
/// # Examples
///
/// ```
/// use uml_core::{Relationship, AssociationType};
///
/// // A generalization: Employee → Person
/// let gen = Relationship::generalization(employee_id, person_id);
///
/// // An association: Employee → Company
/// let assoc = Relationship::association(
///     employee_id, company_id,
///     Some("1".into()), Some("1..*".into()),
///     Some("employee".into()), Some("employer".into()),
///     true, false,
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Relationship {
    /// Common element metadata (id, name, documentation, stereotype, visibility).
    pub base: ElementBase,

    /// The kind of relationship.
    pub kind: AssociationType,

    /// The source element (e.g., the subclass in a generalization).
    pub source_id: UmlId,

    /// The target element (e.g., the superclass in a generalization).
    pub target_id: UmlId,

    /// Multiplicity at the source end (e.g., "1", "0..*", "1..*").
    ///
    /// Meaningful for Association, Aggregation, and Composition.
    /// Typically empty for Generalization, Dependency, and Realization.
    pub source_multiplicity: Option<String>,

    /// Multiplicity at the target end.
    pub target_multiplicity: Option<String>,

    /// Role name at the source end (e.g., "employee").
    ///
    /// Meaningful for Association, Aggregation, and Composition.
    pub source_role_name: Option<String>,

    /// Role name at the target end (e.g., "employer").
    pub target_role_name: Option<String>,

    /// Whether navigation from source to target is supported.
    ///
    /// `true` means code generation should create a reference from
    /// source to target (e.g., a pointer, getter method).
    pub source_to_target_navigable: bool,

    /// Whether navigation from target to source is supported.
    pub target_to_source_navigable: bool,
}
```

### 10.2 Constructors

```rust
impl Relationship {
    /// Create a new generalization (inheritance) relationship.
    ///
    /// `subclass_id` is the child (source), `superclass_id` is the parent (target).
    pub fn generalization(subclass_id: UmlId, superclass_id: UmlId) -> Self {
        Self {
            base: ElementBase {
                id: UmlId::new(),
                name: String::new(),
                visibility: Visibility::Public,
                stereotype_id: None,
                documentation: String::new(),
                is_abstract: false,
                is_static: false,
            },
            kind: AssociationType::Generalization,
            source_id: subclass_id,
            target_id: superclass_id,
            source_multiplicity: None,
            target_multiplicity: None,
            source_role_name: None,
            target_role_name: None,
            source_to_target_navigable: true,
            target_to_source_navigable: false,
        }
    }

    /// Create a new association relationship.
    #[allow(clippy::too_many_arguments)]
    pub fn association(
        source_id: UmlId,
        target_id: UmlId,
        source_multiplicity: Option<String>,
        target_multiplicity: Option<String>,
        source_role_name: Option<String>,
        target_role_name: Option<String>,
        source_to_target_navigable: bool,
        target_to_source_navigable: bool,
    ) -> Self { /* ... */ }

    /// Create a new aggregation relationship.
    pub fn aggregation(
        whole_id: UmlId,
        part_id: UmlId,
        /* ... */
    ) -> Self { /* ... */ }

    /// Create a new composition relationship.
    pub fn composition(
        whole_id: UmlId,
        part_id: UmlId,
        /* ... */
    ) -> Self { /* ... */ }

    /// Create a new dependency relationship.
    pub fn dependency(client_id: UmlId, supplier_id: UmlId) -> Self { /* ... */ }

    /// Create a new realization (interface implementation) relationship.
    pub fn realization(implementor_id: UmlId, interface_id: UmlId) -> Self { /* ... */ }
}
```

### 10.3 ModelElement Changes

```rust
/// All UML model element types in a single tagged union.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
    Relationship(Relationship),  // NEW
}
```

### 10.4 NamedElement Implementation (New Match Arms)

Two existing match blocks gain one arm each:

```rust
impl NamedElement for ModelElement {
    fn base(&self) -> &ElementBase {
        match self {
            ModelElement::Package(p) => &p.base,
            ModelElement::Class(c) => &c.base,
            ModelElement::Interface(i) => &i.base,
            ModelElement::Enum(e) => &e.base,
            ModelElement::Relationship(r) => &r.base,  // NEW
        }
    }

    fn base_mut(&mut self) -> &mut ElementBase {
        match self {
            ModelElement::Package(p) => &mut p.base,
            ModelElement::Class(c) => &mut c.base,
            ModelElement::Interface(i) => &mut i.base,
            ModelElement::Enum(e) => &mut e.base,
            ModelElement::Relationship(r) => &mut r.base,  // NEW
        }
    }

    fn object_type(&self) -> ObjectType {
        match self {
            ModelElement::Package(_) => ObjectType::Package,
            ModelElement::Class(_) => ObjectType::Class,
            ModelElement::Interface(_) => ObjectType::Interface,
            ModelElement::Enum(_) => ObjectType::Enumeration,
            ModelElement::Relationship(r) => {
                // Map AssociationType to ObjectType
                match r.kind {
                    AssociationType::Association
                    | AssociationType::Aggregation
                    | AssociationType::Composition
                    | AssociationType::DirectedAssociation => ObjectType::Association,
                    AssociationType::Generalization => ObjectType::Generalization,
                    AssociationType::Realization => ObjectType::Realization,
                    AssociationType::Dependency => ObjectType::Dependency,
                    _ => ObjectType::Association,  // Fallback for deferred types
                }
            }
        }
    }
}
```

### 10.5 Query Methods on UmlModel

```rust
impl UmlModel {
    /// Find all relationships where the given element participates
    /// (as either source or target).
    #[must_use]
    pub fn relationships_of(&self, element_id: UmlId) -> Vec<&Relationship> {
        self.iter()
            .filter_map(|(_, e)| match e {
                ModelElement::Relationship(r) => Some(r),
                _ => None,
            })
            .filter(|r| r.source_id == element_id || r.target_id == element_id)
            .collect()
    }

    /// Find all generalizations (superclasses) of the given element.
    ///
    /// Returns the generalization relationships where `element_id` is the source
    /// (subclass). The target element of each returned relationship is the
    /// superclass.
    #[must_use]
    pub fn generalizations_of(&self, element_id: UmlId) -> Vec<&Relationship> {
        self.iter()
            .filter_map(|(_, e)| match e {
                ModelElement::Relationship(r)
                    if r.kind == AssociationType::Generalization
                        && r.source_id == element_id =>
                {
                    Some(r)
                }
                _ => None,
            })
            .collect()
    }

    /// Find all specializations (subclasses) of the given element.
    ///
    /// Returns the generalization relationships where `element_id` is the target
    /// (superclass). The source element of each returned relationship is a
    /// subclass.
    #[must_use]
    pub fn specializations_of(&self, element_id: UmlId) -> Vec<&Relationship> {
        self.iter()
            .filter_map(|(_, e)| match e {
                ModelElement::Relationship(r)
                    if r.kind == AssociationType::Generalization
                        && r.target_id == element_id =>
                {
                    Some(r)
                }
                _ => None,
            })
            .collect()
    }

    /// Find all realizations (interfaces implemented) of the given element.
    ///
    /// Returns the realization relationships where `element_id` is the source
    /// (implementor). The target element of each returned relationship is the
    /// realized interface.
    #[must_use]
    pub fn realizations_of(&self, element_id: UmlId) -> Vec<&Relationship> {
        self.iter()
            .filter_map(|(_, e)| match e {
                ModelElement::Relationship(r)
                    if r.kind == AssociationType::Realization
                        && r.source_id == element_id =>
                {
                    Some(r)
                }
                _ => None,
            })
            .collect()
    }

    /// Find all associations (plain, aggregation, composition) of the given element.
    #[must_use]
    pub fn associations_of(&self, element_id: UmlId) -> Vec<&Relationship> {
        self.iter()
            .filter_map(|(_, e)| match e {
                ModelElement::Relationship(r)
                    if matches!(
                        r.kind,
                        AssociationType::Association
                        | AssociationType::Aggregation
                        | AssociationType::Composition
                    ) && (r.source_id == element_id || r.target_id == element_id) =>
                {
                    Some(r)
                }
                _ => None,
            })
            .collect()
    }

    /// Find all relationships of a specific type connected to the given element.
    #[must_use]
    pub fn relationships_of_type(
        &self,
        element_id: UmlId,
        kind: AssociationType,
    ) -> Vec<&Relationship> {
        self.iter()
            .filter_map(|(_, e)| match e {
                ModelElement::Relationship(r)
                    if r.kind == kind
                        && (r.source_id == element_id || r.target_id == element_id) =>
                {
                    Some(r)
                }
                _ => None,
            })
            .collect()
    }
}
```

### 10.6 Reference Validation Extension

```rust
/// Extend the ReferenceField enum:
pub enum ReferenceField {
    PackageChild,
    AttributeType,
    OperationReturnType,
    ParameterType,
    Stereotype,
    RelationshipSource,   // NEW
    RelationshipTarget,   // NEW
}

impl UmlModel {
    fn validate_references(&self) -> Vec<ReferenceError> {
        let mut errors = Vec::new();
        // ... existing checks ...

        // NEW: Check relationship source/target references
        for (id, element) in self.iter() {
            if let ModelElement::Relationship(r) = element {
                if !self.contains(r.source_id) {
                    errors.push(ReferenceError {
                        source_id: id,
                        field: ReferenceField::RelationshipSource,
                        target_id: r.source_id,
                    });
                }
                if !self.contains(r.target_id) {
                    errors.push(ReferenceError {
                        source_id: id,
                        field: ReferenceField::RelationshipTarget,
                        target_id: r.target_id,
                    });
                }
            }
        }
        errors
    }
}
```

### 10.7 ObjectType Enum Extensions

The `ObjectType` enum must be extended with three new variants to represent
relationship types that are not associations:

```rust
pub enum ObjectType {
    // ... existing variants ...
    Association,        // Maps to Association, Aggregation, Composition
    Generalization,     // NEW — maps to AssociationType::Generalization
    Realization,        // NEW — maps to AssociationType::Realization
    Dependency,         // NEW — maps to AssociationType::Dependency
}
```

These variants are already referenced in the `object_type()` match arm in §10.4.
They should be added to the `ObjectType` enum in `types.rs` alongside the
existing `Association` variant (which covers Association, Aggregation, and
Composition).

### 10.8 Cascading Relationship Cleanup in `remove()`

The `UmlModel::remove()` method must cascade-delete all relationships where the
removed element participates (as source or target). This extends the existing
cascading cleanup pattern (already removes from `parent_index` and
`package.children`).

```rust
/// Remove an element by ID.
///
/// Performs cascading cleanup:
/// 1. Removes all relationships where this element is source or target.
/// 2. Removes the element from `parent_index`.
/// 3. Removes the element's ID from every package's `children` list.
/// 4. Removes the element from the elements map.
pub fn remove(&mut self, id: UmlId) -> Option<ModelElement>;
```

Implementation sketch:

```rust
impl UmlModel {
    pub fn remove(&mut self, id: UmlId) -> Option<ModelElement> {
        // Step 1: Remove all relationships involving this element.
        let rel_ids: Vec<UmlId> = self.relationships_of(id)
            .iter()
            .map(|r| r.base.id)
            .collect();
        for rel_id in rel_ids {
            self.remove(rel_id);
        }

        // Step 2: Remove from parent_index (existing logic).
        // Step 3: Remove from package children (existing logic).
        // Step 4: Remove from elements map (existing logic).
        self.elements.remove(&id)
    }
}
```

Note that the recursive call to `self.remove(rel_id)` for each related
relationship will not trigger infinite recursion: a `Relationship` has no
`source_id`/`target_id` pointing to another `Relationship` (relationships only
reference non-relationship elements), so step 1 is a no-op when called on a
relationship. The cascade is exactly one level deep.

### 10.9 Required Match Arm Updates

When adding the `Relationship` variant to `ModelElement`, the following match
arms must be updated:

| Location | Arm to Add | Notes |
|----------|-----------|-------|
| `ModelElement::object_type()` | `ModelElement::Relationship(r) => object_type_from_association_type(r.kind)` | Maps via `AssociationType` |
| `ModelElement::base()` | `ModelElement::Relationship(r) => &r.base` | Standard pattern |
| `ModelElement::base_mut()` | `ModelElement::Relationship(r) => &mut r.base` | Standard pattern |
| `ModelElement::classifier_data()` | `ModelElement::Relationship(_) => None` | Relationships are not classifiers |
| `ModelElement::is_classifier()` | `ModelElement::Relationship(_) => false` | Relationships are not classifiers |
| `NamedElement for ModelElement` — three methods | Add `Relationship` arm in `base()`, `base_mut()`, `object_type()` | Already shown in §10.4 |
| `UmlModel::validate_references()` | Check `source_id` and `target_id` exist | Already shown in §10.6 |

### 10.10 Test Plan

The following minimum tests are required for the `Relationship` implementation:

| # | Test Category | Description |
|---|--------------|-------------|
| 1 | **Construction** | Create each relationship type (Generalization, Association, Aggregation, Composition, Dependency, Realization) with valid source/target IDs. Verify field values. |
| 2 | **Serde round-trip** | Serialize a `Relationship` to JSON and deserialize it back. Verify all fields match. |
| 3 | **ModelElement integration** | Insert a `Relationship` via `UmlModel::insert()`, retrieve it via `get()`, verify `is_relationship()` and `as_relationship()`. Remove and confirm `None`. |
| 4 | **Reference validation** | Insert a `Relationship` with a dangling `source_id`. Verify `validate_references()` returns a `ReferenceError` with `ReferenceField::RelationshipSource`. Repeat for dangling `target_id`. |
| 5 | **Cascading remove** | Insert elements A, B, and a Relationship(A, B). Remove A. Verify the relationship is also removed from the model. |
| 6 | **Query methods** | Test `relationships_of()`, `generalizations_of()`, `realizations_of()`, `associations_of()`, `specializations_of()`, `relationships_of_type()`. Verify correct filtering by participant and type. |
| 7 | **Self-relationship** | Create a relationship where `source_id == target_id`. Verify it is allowed and queries return it correctly. |
| 8 | **Duplicate protection** | Insert two identical relationships (same source, target, type). Verify the model allows it (no uniqueness constraint). |
| 9 | **Edge cases** | Relationship with all `Option` fields set to `None`. Relationship with an empty `name` string. Relationship with maximum multiplicity strings (e.g., `"2147483647"`). |

---

## 11. API Design

### 11.1 Complete Usage Example

```rust
use uml_core::{
    UmlModel, ModelElement, Relationship, AssociationType,
    Package, Class, Visibility, ElementBase, UmlId,
};

// Create a model.
let mut model = UmlModel::new();

// Create two classes.
let person_class = ModelElement::Class(Class::new("Person"));
let address_class = ModelElement::Class(Class::new("Address"));
let person_id = person_class.id();
let address_id = address_class.id();

model.insert(person_class);
model.insert(address_class);

// Create a package and add both classes.
let pkg = ModelElement::Package(Package::new("com.example"));
let pkg_id = pkg.id();
model.insert(pkg);
model.add_to_package(pkg_id, person_id).unwrap();
model.add_to_package(pkg_id, address_id).unwrap();

// Create an association: Person —address→ Address.
let assoc = Relationship::association(
    person_id,           // source
    address_id,          // target
    Some("1".into()),    // source multiplicity (each person has one address)
    Some("0..*".into()), // target multiplicity (an address can have many people)
    None,                // source role name
    Some("address".into()), // target role name
    true,                // navigable from Person to Address
    false,               // not navigable from Address to Person
);
let assoc_id = assoc.id();

model.insert(ModelElement::Relationship(assoc));

// Query: find all relationships for Person.
let rels = model.relationships_of(person_id);
assert_eq!(rels.len(), 1);
assert_eq!(rels[0].target_id, address_id);

// Query: find all associations for Person.
let assocs = model.associations_of(person_id);
assert_eq!(assocs.len(), 1);
assert_eq!(assocs[0].kind, AssociationType::Association);

// Validate references — all references are valid.
assert!(model.validate_references().is_empty());

// Serialize the association.
let json = serde_json::to_string_pretty(
    &ModelElement::Relationship(
        *model.get(assoc_id).unwrap().as_relationship().unwrap()
    )
).unwrap();
assert!(json.contains("\"type\": \"Relationship\""));
assert!(json.contains("\"source_id\""));

// Remove the association by ID (same API as any element).
let removed = model.remove(assoc_id).unwrap();
assert!(removed.is_relationship());
```

### 11.2 Serialization Format

```json
{
    "type": "Relationship",
    "base": {
        "id": "550e8400-e29b-41d4-a716-446655440003",
        "name": "",
        "visibility": "public",
        "stereotype_id": null,
        "documentation": "",
        "is_abstract": false,
        "is_static": false
    },
    "kind": "Association",
    "source_id": "550e8400-e29b-41d4-a716-446655440001",
    "target_id": "550e8400-e29b-41d4-a716-446655440002",
    "source_multiplicity": "1",
    "target_multiplicity": "0..*",
    "source_role_name": null,
    "target_role_name": "address",
    "source_to_target_navigable": true,
    "target_to_source_navigable": false
}
```

### 11.3 Integration with Existing ModelElement Methods

```rust
impl ModelElement {
    /// Returns `true` if this element is a relationship.
    pub fn is_relationship(&self) -> bool {
        matches!(self, Self::Relationship(_))
    }

    /// Returns a reference to the inner `Relationship`, if this is one.
    pub fn as_relationship(&self) -> Option<&Relationship> {
        match self {
            Self::Relationship(r) => Some(r),
            _ => None,
        }
    }

    /// Returns a mutable reference to the inner `Relationship`, if this is one.
    pub fn as_relationship_mut(&mut self) -> Option<&mut Relationship> {
        match self {
            Self::Relationship(r) => Some(r),
            _ => None,
        }
    }

    /// Returns the `AssociationType` if this is a relationship, otherwise `None`.
    pub fn kind(&self) -> Option<AssociationType> {
        match self {
            Self::Relationship(r) => Some(r.kind),
            _ => None,
        }
    }
    }
}
```

---

## 12. What This Enables

### 12.1 Milestone 5 Features

With relationships implemented, Milestone 5 delivers:

- **Relationship creation API.** Consistent with element creation:
  ```rust
  model.insert(ModelElement::Relationship(
      Relationship::generalization(subclass_id, superclass_id)
  ));
  ```

- **Relationship lookup by ID.** Same API as any element:
  ```rust
  let rel = model.get(rel_id);
  ```

- **Participant queries.** Bidirectional "what connects to element X":
  ```rust
  let all = model.relationships_of(element_id);
  let supers = model.generalizations_of(class_id);
  let ifaces = model.realizations_of(class_id);
  let assocs = model.associations_of(class_id);
  ```

- **Reference validation.** Dangling source/target references are detected:
  ```rust
  let errors = model.validate_references();
  ```

- **Serde support.** Automatic JSON round-tripping for tests and tooling.

### 12.2 Future Milestones

This design supports features needed in later milestones:

- **Code generation.** Generate member variables from associations, superclass
  references from generalizations, interface stubs from realizations:
  ```rust
  for rel in model.associations_of(class_id) {
      if rel.source_to_target_navigable {
          // emit: let target_name = HashMap::<String, Target>::new();
      }
  }
  ```

- **Diagram rendering.** Edges reference relationships by `UmlId`:
  ```rust
  struct DiagramEdge {
      relationship_id: UmlId,
      widget_a: WidgetId,
      widget_b: WidgetId,
  }
  // Resolution:
  let rel = model.get(edge.relationship_id).unwrap().as_relationship().unwrap();
  ```

- **XMI round-trip.** Single-loop serialization:
  ```rust
  for (_id, element) in model.iter() {
      match element {
          ModelElement::Relationship(r) => {
              // emit <UML:Association> or <UML:Generalization> etc.
          }
          ModelElement::Class(c) => {
              // emit <UML:Class>
          }
          // ...
      }
  }
  ```

- **Lazy adjacency index.** If profiling shows need, add transparently:
  ```rust
  impl UmlModel {
      fn ensure_adjacency_index(&mut self) {
          if self.adjacency_index.is_some() {
              return;
          }
          // Build index from ModelElement::Relationship variants
          // No change to public API or data model
      }
  }
  ```

- **Additional relationship types.** The six deferred `AssociationType`
  variants can be added without structural changes:
  - DirectedAssociation — already a field (navigability flags)
  - Anchor — used for note/comment connections
  - Containment — diagram-only, may stay in the diagram crate
  - Exception — language-specific (C++, Java)
  - Category2Parent / Child2Category — UML extension

---

## References

- [Domain Model v1](./domain_model_v1.md) — The Rust-native UML metamodel.
- [Model Repository v1](./model_repository_v1.md) — `UmlModel` storage design.
- [UML 1.4 Specification §2.5.3](https://www.omg.org/spec/UML/1.4/) — Association, Generalization, Dependency metamodel.
- [C++ `UMLAssociation`](https://invent.kde.org/system/umbrello/-/blob/master/umbrello/umlassociation.h) — Current C++ association implementation.
- [C++ `UMLRole`](https://invent.kde.org/system/umbrello/-/blob/master/umbrello/umlrole.h) — Role (association end) implementation.
- [Crate boundary review](./crate_boundary_review.md) — Umbrello-RS workspace organisation.
- `uml-core/src/types.rs` — `AssociationType` enum definition.
- `uml-core/src/elements.rs` — `ModelElement` and element type definitions.
- `uml-core/src/repository.rs` — `UmlModel` storage implementation.
