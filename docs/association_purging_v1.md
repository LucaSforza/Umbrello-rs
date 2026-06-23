# AssociationType Variant Purging — v1 Design

**Status:** Design proposal  
**Applies to:** `uml-core` domain model (`types.rs`, `elements.rs`, `repository.rs`)  
**Goal:** Remove 6 diagram-specific variants from `AssociationType`, leaving only 100% semantic UML relationship kinds.

---

## 1. Motivation

The current `AssociationType` enum mixes two concerns:

1. **Semantic UML relationships** — relationships that have meaning in the UML metamodel and survive XMI export.
2. **Visual/diagram-only edge kinds** — concepts that only matter for rendering lines on a diagram canvas and have no place in a pure domain model.

This violates the principle that the core domain model should be independent of presentation. It also forces downstream code (serialization, validation, query) to handle variants that cannot meaningfully appear in any model export.

---

## 2. Current State: 12 Variants

```rust
pub enum AssociationType {
    Association,          // SEMANTIC — keep
    DirectedAssociation,  // DIAGRAM   — navigability is a property flag, not a type
    Generalization,       // SEMANTIC — keep
    Realization,          // SEMANTIC — keep
    Aggregation,          // SEMANTIC — keep
    Composition,          // SEMANTIC — keep
    Dependency,           // SEMANTIC — keep
    Anchor,               // DIAGRAM   — visual note-anchoring line
    Containment,          // DIAGRAM   — visual nesting in component diagrams
    Exception,            // DIAGRAM   — sequence-diagram throw annotation
    Category2Parent,      // DIAGRAM   — EER specialization visual arrow
    Child2Category,       // DIAGRAM   — EER specialization visual arrow
}
```

Each variant is classified as either **SEMANTIC** (part of the UML metamodel) or **DIAGRAM** (a visual rendering concept).

---

## 3. Target State: 6 Variants

```rust
pub enum AssociationType {
    /// A plain structural relationship between classifiers.
    Association,
    /// Inheritance relationship (subclass → superclass).
    Generalization,
    /// Interface implementation relationship.
    Realization,
    /// Whole-part relationship with shared lifecycle (whole → part).
    Aggregation,
    /// Whole-part relationship with exclusive lifecycle (whole → part).
    Composition,
    /// A usage dependency relationship.
    Dependency,
}
```

### 3.1 Kept variants — rationale

| Variant | UML Metaclass | XMI Element | Why semantic |
|---------|---------------|-------------|--------------|
| `Association` | `uml:Association` | `<UML:Association>` | Core structural relationship |
| `Generalization` | `uml:Generalization` | `<UML:Generalization>` | Subclass/superclass relationship |
| `Realization` | `uml:Realization` | `<UML:Realization>` | Interface implementation |
| `Aggregation` | `uml:Association` (aggregation=shared) | `<UML:Association>` with `aggregation="shared"` | Shared whole-part semantics |
| `Composition` | `uml:Association` (aggregation=composite) | `<UML:Association>` with `aggregation="composite"` | Exclusive whole-part semantics |
| `Dependency` | `uml:Dependency` | `<UML:Dependency>` | Usage/client relationship |

Every kept variant maps 1:1 to a UML metamodel construct and has a well-defined XMI serialization.

---

## 4. Analysis of Removed Variants

### 4.1 `DirectedAssociation` — REMOVE

**Why it is diagram-specific:**

"Directed" refers to which end(s) of the line have an arrowhead. This is purely a visual property. In UML, a directed association is simply an `Association` where one end has `isNavigable="true"` and the other has `isNavigable="false"`.

**Where the semantics already live:**

The `Relationship` struct already carries navigability flags:

```rust
pub struct Relationship {
    pub id: UmlId,
    pub source_to_target_navigable: bool,
    pub target_to_source_navigable: bool,
    // ...
}
```

An `Association` with `source_to_target_navigable: true` IS a directed association. A separate enum variant adds nothing.

**Migration:**

- Replace `AssociationType::DirectedAssociation` with `AssociationType::Association`
- Set `source_to_target_navigable` / `target_to_source_navigable` as appropriate

**Impact on domain model:** None.

---

### 4.2 `Anchor` — REMOVE

**Why it is diagram-specific:**

Anchor edges connect notes (`NoteWidget`) to diagram elements. A note has no semantic relationship to the element it annotates — it is purely a visual annotation. The UML metamodel has no `Anchor` metaclass; notes are a diagram presentation concern.

**Future home:**

A `uml-diagram` crate (or diagram module) will define:

```rust
pub enum EdgeKind {
    Anchor,
    // ...
}
```

Diagram data will map `(DiagramId, EdgeId) -> EdgeKind`, keeping annotations out of the domain model.

**Migration:**

- Notes reference elements via diagram metadata, not `Relationship`
- Remove all `Anchor` construction/serialization paths

**Impact on domain model:** None. Notes already exist as diagram widgets, not model elements.

---

### 4.3 `Containment` — REMOVE

**Why it is diagram-specific:**

Containment edges show visual nesting — for example, a class widget drawn inside a component widget. In UML, containment is a model ownership concept (`Package::packagedElement`, `Class::nestedClassifier`), not an association type. The domain model already handles this via `Package::children` and `UmlModel::add_to_package()`.

**Future home:**

The diagram crate will define `EdgeKind::Containment` or handle visual nesting via widget parent-child relationships.

**Migration:**

- Replace containment references with model-level ownership (`add_to_package`)
- Remove serialization/deserialization paths for containment

**Impact on domain model:** None. Package ownership is the correct semantic mechanism.

---

### 4.4 `Exception` — REMOVE

**Why it is diagram-specific:**

Exception lines appear on sequence diagrams to show which exceptions an operation can throw. They are a visual annotation on a message, not a first-class UML relationship. UML models exceptions via `Operation::raisedException` or tagged values.

**Future home:**

The sequence-diagram crate/module will store exception metadata on message widgets.

**Migration:**

- Convert exception relationships to tagged values or `Operation::raised_exception` references
- Remove from `AssociationType`

**Impact on domain model:** None if exception data was unused in domain logic.

---

### 4.5 `Category2Parent` / `Child2Category` — REMOVE

**Why they are diagram-specific:**

These two variants represent EER (Extended Entity-Relationship) specialization arrows. They are visual connectors in ER diagrams. UML has no equivalent; they are not part of the UML specification.

**Future home:**

A future `uml-eer` crate or an ER diagram module can define:

```rust
pub enum ErEdgeKind {
    CategoryToSupertype,
    SubtypeToCategory,
}
```

**Migration:**

- Remove both variants from `AssociationType`
- ER diagrams that need these edges will use a dedicated ER module outside `uml-core`

**Impact on domain model:** None. EER specialization is not part of UML.

---

## 5. Design Principle

> **The core domain model must be 100% semantic UML.** Visual rendering concepts (arrowheads, nesting, anchors) belong in diagram crates. If a concept is about *how* something looks on a diagram rather than *what* it means semantically, it does not belong in `uml-core`.

### 5.1 Litmus test

When evaluating whether a variant belongs in the domain model, apply this test:

> **"If I export this model as pure XMI with no diagram information, would this variant still be meaningful?"**

| Variant | XMI representation | Survives litmus test? |
|---------|-------------------|-----------------------|
| Generalization | `<UML:Generalization child="A" parent="B"/>` | ✅ Yes — pure semantics |
| Realization | `<UML:Realization client="A" supplier="B"/>` | ✅ Yes — pure semantics |
| Association | `<UML:Association>` | ✅ Yes — pure semantics |
| Aggregation | `<UML:Association aggregation="shared">` | ✅ Yes — semantic property |
| Composition | `<UML:Association aggregation="composite">` | ✅ Yes — semantic property |
| Dependency | `<UML:Dependency client="A" supplier="B"/>` | ✅ Yes — pure semantics |
| DirectedAssociation | `<UML:Association>` with navigability flags | 🔶 No — navigability is a flag, not a type |
| Anchor | No XMI element exists | ❌ No — purely visual |
| Containment | Modeled as `packagedElement` ownership | ❌ No — diagram concern |
| Exception | No XMI element exists | ❌ No — diagram annotation |
| Category2Parent | No XMI element exists | ❌ No — EER, not UML |
| Child2Category | No XMI element exists | ❌ No — EER, not UML |

---

## 6. Impact on Existing Code

### 6.1 `types.rs`

| Change | Details |
|--------|---------|
| Enum variant removal | Remove `DirectedAssociation`, `Anchor`, `Containment`, `Exception`, `Category2Parent`, `Child2Category` |
| `as_str()` match arms | Remove the 6 corresponding arms |
| `from_str()` match arms | Remove the 6 corresponding arms |
| `has_visual_representation()` | Remove entirely — no variant has a visual-representation-only classification when all variants are semantic |
| `variants()` / iteration helpers | Reduce from 12 to 6 |
| Tests | Remove `test_association_type_has_visual_representation`; update any iter-all-variants tests |

### 6.2 `elements.rs` (Relationship)

| Change | Details |
|--------|---------|
| `object_type()` match | Remove the 6 removed-variant arms (should be unreachable after migration) |
| Constructor functions | Verify none of the 6 removed variants are referenced — if any exist, remove them |
| Widget-type matching | Update any match that dispatches on `AssociationType` for widget rendering — this belongs in a widget/diagram crate |
| Tests | Remove tests that construct `Relationship` with removed variants |

### 6.3 `repository.rs`

No direct changes. Query methods that iterate or filter relationships will naturally include only the 6 kept variants. If any query explicitly listed all 12 variants, reduce it to the 6.

### 6.4 Serialization (serde)

| Change | Details |
|--------|---------|
| `serde_roundtrip.rs` | `association_type_all_variants_roundtrip`: reduce from 12 to 6 variants |
| Deserialization of old data | If old serialized data contains removed variants, either reject them or map them (e.g. `DirectedAssociation` → `Association`). Generally, accept only the 6 kept variants on input. |

### 6.5 XMI import/export

| Change | Details |
|--------|---------|
| XMI reader | Remove branches that create removed-variant relationships; map `DirectedAssociation` to `Association` with navigability flags if encountered |
| XMI writer | Remove branches that emit removed variants |

### 6.6 Other modules

| Module | Potential impact |
|--------|------------------|
| `uml-widgets` | Widgets that formerly checked `AssociationType` for rendering decisions must now use a diagram-level edge kind enum. No changes to core — widget logic moves to diagram crate. |
| `codegenerators` | None — code generators only care about `Generalization`, `Realization`, `Dependency`, etc. which are kept. |
| `codeimport` | None — importers produce semantic relationships only. |
| `menus` | Context menu entries that offered "Set as Directed Association" etc. should set navigability flags instead. |

---

## 7. Forward Compatibility: EdgeKind

When full diagram support is implemented, visual edge classification will live in a diagram crate:

```rust
// uml-diagram or uml-core::diagram module
/// Visual edge kinds for diagram rendering.
///
/// These are NOT domain relationships — they are rendering hints
/// that describe how an edge appears on a canvas.
pub enum EdgeKind {
    /// A note-attachment line (no semantic meaning).
    Anchor,
    /// Visual nesting indicator in component/package diagrams.
    Containment,
    /// Exception throw indicator on sequence diagrams.
    Exception,
    /// Directed arrow (rendering hint for navigability).
    Directed,
    /// EER specialization from category to supertype.
    CategoryToSupertype,
    /// EER specialization from subtype to category.
    SubtypeToCategory,
}
```

Diagram edge data will map entity IDs to `EdgeKind`:

```rust
pub struct DiagramEdge {
    pub id: EdgeId,
    pub diagram_id: DiagramId,
    pub relationship_id: UmlId,   // refers to a Relationship in the domain model
    pub edge_kind: EdgeKind,       // visual classification
    pub source_widget_id: WidgetId,
    pub target_widget_id: WidgetId,
    // rendering properties (line style, color, etc.)
}
```

This keeps the core domain pure while allowing the rendering layer full flexibility.

---

## 8. Migration Plan

### Phase 1: Remove from enum (this proposal)
1. Remove variants from `AssociationType`
2. Remove corresponding `as_str()` / `from_str()` arms
3. Remove `has_visual_representation()`
4. Update all match arms in `elements.rs`
5. Update tests
6. Verify compilation: `cargo build`

### Phase 2: Clean up dependent code
1. Search for any remaining references to removed variants across workspace
2. Replace `DirectedAssociation` constructions with `Association` + navigability flags
3. Map old serialized data on deserialization

### Phase 3: Establish `EdgeKind` (future)
1. Define `EdgeKind` in the diagram crate
2. Create `DiagramEdge` storage
3. Migrate widget rendering to use `EdgeKind` instead of inspecting `AssociationType`
4. Remove legacy `AssociationType`-as-visual-hint pattern

---

## 9. Migration Example: DirectedAssociation → Association + Navigability

**Before:**

```rust
// types.rs
AssociationType::DirectedAssociation => "directed_association",

// elements.rs
Relationship::new(AssociationType::DirectedAssociation, source, target)
```

**After:**

```rust
// types.rs (variant removed)

// elements.rs
Relationship {
    association_type: AssociationType::Association,
    source_to_target_navigable: true,
    target_to_source_navigable: false,
    // ...
}
```

This is a strict improvement: navigability is explicit, composable, and matches the UML metamodel.

---

## 10. Open Questions

1. **Backward compatibility of serialized data** — Should we silently map `DirectedAssociation` → `Association` on deserialization, or reject old data outright? **Recommendation:** Accept old data with a mapping layer for the 6 removed variants.

2. **When does `EdgeKind` land?** — It can be defined early (even before any diagram rendering code) as a pure-data enum, giving dependent code a place to migrate to. **Recommendation:** Define `EdgeKind` in Phase 1 alongside the removal, even if unused initially.

3. **What about foreign-key relationships in persistence?** — If a database stores `association_type` as a string/enum, existing rows with removed values must be migrated. **Recommendation:** Run a one-time migration script that converts old values to the mapped representations.

---

## Appendix A: Before/After Summary

| Aspect | Before (12 variants) | After (6 variants) |
|--------|---------------------|--------------------|
| Semantic variants | 6 | 6 (unchanged) |
| Diagram-only variants | 6 | 0 |
| `has_visual_representation()` | Present | Removed |
| Navigability encoding | Mixed (flags + DirectedAssociation variant) | Flags only |
| Note anchoring | `AssociationType::Anchor` | `EdgeKind::Anchor` (future) |
| Visual containment | `AssociationType::Containment` | `EdgeKind::Containment` (future) |
| Exception edges | `AssociationType::Exception` | `EdgeKind::Exception` (future) |
| EER arrows | `Category2Parent` / `Child2Category` | `ErEdgeKind` (future EER crate) |

## Appendix B: File Checklist

- [ ] `types.rs` — remove 6 variants, `has_visual_representation()`
- [ ] `elements.rs` — update `Relationship` match arms
- [ ] `serde_roundtrip.rs` — reduce roundtrip test
- [ ] `repository.rs` — verify no 12-variant iteration
- [ ] `xmi/reader.rs` — map removed variants on import
- [ ] `xmi/writer.rs` — skip removed variants on export
- [ ] All test files — remove or update tests referencing removed variants
