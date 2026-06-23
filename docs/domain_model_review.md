# Domain Model v1 — Formal Design Review

> **Document:** `rust-rewrite/docs/domain_model_review.md`  
> **Review date:** 2026-06-23  
> **Objects under review:**
> - Design document: `domain_model_v1.md` (1241 lines)
> - Implementation: `crates/uml-core/src/elements.rs` (758 lines, 66 tests)
> - Supporting implementation: `crates/uml-core/src/model.rs` (14 lines)
>
> **Trigger:** Architecture-first process mandate requiring documentation to be
> authoritative and approved before implementation. The design document and
> implementation were produced in parallel before this rule took effect. This
> review identifies all divergences and recommends resolution.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Review Methodology](#2-review-methodology)
3. [Divergence Analysis](#3-divergence-analysis)
   - [3.1 Subordinate Element Identity — CRITICAL](#31-subordinate-element-identity--critical)
   - [3.2 Operation Field Differences — HIGH](#32-operation-field-differences--high)
   - [3.3 TemplateParameter Design — HIGH](#33-templateparameter-design--high)
   - [3.4 EnumLiteral Design — HIGH](#34-enumliteral-design--high)
   - [3.5 Parameter `default_value` — MEDIUM](#35-parameter-default_value--medium)
   - [3.6 Field Name `classifier_data` vs. `classifier` — LOW](#36-field-name-classifier_data-vs-classifier--low)
   - [3.7 ModelRepository Not Yet Implemented — INFORMATIONAL](#37-modelrepository-not-yet-implemented--informational)
   - [3.8 `type_name` Dual-Reference Pattern — LOW](#38-type_name-dual-reference-pattern--low)
   - [3.9 Operation `type_name` Fallback — LOW](#39-operation-type_name-fallback--low)
4. [Architecture Principles Cross-Check](#4-architecture-principles-cross-check)
5. [C++ Codebase Validation](#5-c-codebase-validation)
6. [Summary of Required Remediations](#6-summary-of-required-remediations)
7. [What Must NOT Change](#7-what-must-not-change)
8. [Final Recommendation](#8-final-recommendation)

---

## 1. Executive Summary

This review examines the Rust domain model design document (`domain_model_v1.md`)
against the current implementation in `crates/uml-core/src/elements.rs`. The
architecture-first process requires that documentation be the authoritative
source of truth — implementation must reflect documentation, and the document
must be approved before coding proceeds.

**Overall finding:** The implementation is the better design in every divergence.
The design document over-engineers subordinate element types (Attribute,
Operation, Parameter, TemplateParameter, EnumLiteral) by giving them independent
identity (`ElementBase` with `UmlId`), while the implementation correctly treats
them as pure value types embedded in their owning classifier. The implementation
also provides more practical field sets (e.g., `type_name` fallback for primitive
types, correct operation flags).

**Nine divergences were found** (1 CRITICAL, 3 HIGH, 1 MEDIUM, 3 LOW, 1 INFORMATIONAL).
All should be resolved by **updating the design document to match the implementation**.
Zero code changes are recommended.

---

## 2. Review Methodology

The review was conducted by:

1. **Line-by-line comparison** of every type definition in the design document
   (section 3.5, lines 509–585) against the implementation (lines 101–405 of
   `elements.rs`).

2. **Semantic analysis** of each divergence against:
   - The C++ Umbrello codebase (the ground truth for UML semantics)
   - Rust design principles (composition, flat hierarchy, ID-based references)
   - UML 2.5 metamodel semantics

3. **C++ path verification**: Subordinate types confirmed in
   `umlcanvasobject.h` (`m_List` for subordinates, line 22: "subordinate objects")
   vs. `umlpackage.h` (`m_objects` for standalone objects, line 78).

4. **Architecture principles cross-check**: Each of the 15 documented principles
   was verified against the implementation.

---

## 3. Divergence Analysis

### 3.1 Subordinate Element Identity — CRITICAL

**Severity:** CRITICAL  
**Classification:** Architectural design error in document

#### Document says (lines 540–584)

Attribute, Operation, Parameter, TemplateParameter, and EnumLiteral each embed
`ElementBase` with their own `UmlId`. They are treated as independent,
first-class model elements stored in `ModelRepository`:

```rust
// Document design (line 541):
pub struct Attribute {
    pub base: ElementBase,       // has its own id, name, visibility
    pub type_id: Option<UmlId>,
    pub default_value: String,   // non-optional, always present
}

// Document design (line 551):
pub struct Operation {
    pub base: ElementBase,       // has its own id
    pub return_type_id: Option<UmlId>,
    pub parameters: Vec<Parameter>,
    pub is_query: bool,
}

// Document design (line 572):
pub struct TemplateParameter {
    pub base: ElementBase,       // has its own id
    pub type_id: Option<UmlId>,
    pub default_value: String,
}

// Document design (line 580):
pub struct EnumLiteral {
    pub base: ElementBase,       // has its own id
    pub value: String,           // non-optional
}
```

The document also shows `child_ids()` on `ModelElement` (lines 646–695) pushing
these subordinate elements' IDs into the same namespace as package children.

#### Implementation has (lines 101–189)

Subordinate types have NO `ElementBase`, NO `UmlId`. They are pure value types
with plain `name: String` fields, embedded in `ClassifierData` vectors:

```rust
// Implementation (line 105):
pub struct Attribute {
    pub name: String,            // plain field, not in ElementBase
    pub type_id: Option<UmlId>,
    pub type_name: Option<String>,
    pub visibility: Visibility,
    pub initial_value: Option<String>,
    pub is_static: bool,
}

// Implementation (line 143):
pub struct Operation {
    pub name: String,            // plain field, not in ElementBase
    pub return_type_id: Option<UmlId>,
    pub return_type_name: Option<String>,
    pub parameters: Vec<Parameter>,
    pub visibility: Visibility,
    pub is_static: bool,
    pub is_abstract: bool,
    pub is_virtual: bool,
}

// Implementation (line 170):
pub struct TemplateParameter {
    pub name: String,            // plain field, not in ElementBase
    pub constraint: Option<String>,
}

// Implementation (line 182):
pub struct EnumLiteral {
    pub name: String,            // plain field, not in ElementBase
    pub value: Option<String>,   // optional
}
```

The implementation's `ModelElement` has NO `child_ids()` method — subordinate
elements are not individually addressable from the repository.

#### Analysis

The implementation's approach is correct for five reasons:

1. **UML semantics.** In UML, attributes and operations are *features* of a
   classifier, strictly subordinate — they never exist independently. An
   attribute cannot be "free-floating" in a model without a classifier to
   contain it.

2. **C++ codebase alignment.** The C++ codebase confirms this design:
   - `UMLCanvasObject::m_List` (subordinates, see `umlcanvasobject.h` line 22:
     "subordinate objects") stores `UMLAttribute`, `UMLOperation`, etc.
   - `UMLPackage::m_objects` (standalone, see `umlpackage.h` line 78) stores
     independent elements like `UMLClass`, `UMLInterface`, `UMLEnum`.
   - Attributes/operations are **never** in `m_objects`.

3. **Repository overhead.** If subordinate types had `UmlId`s and arena slots,
   every attribute and operation would require an insert/get/remove in the
   `ModelRepository`. A model with 100 classes each having 10 attributes would
   require 1000 extra arena slots for data that is never independently
   referenced.

4. **Parent-child consistency.** Independent IDs create a consistency problem:
   an attribute exists in the repository with some ID, but it must also be
   present in its parent's `attributes` vector. Two sources of truth for the
   same data.

5. **Reference pattern mismatch.** The ID-based reference system (Principle 5
   in the document, lines 227–241) is designed for cross-references between
   independent elements (Package → Class, Association → Role). Subordinate
   elements are never cross-referenced by ID — they are accessed positionally
   within their parent's vector.

#### Recommendation

**UPDATE THE DOCUMENT** to match the implementation. Section 3.5 (Concrete
Element Types, lines 509–585) must be rewritten to show Attribute, Operation,
Parameter, TemplateParameter, and EnumLiteral as pure value types without
`ElementBase` or `UmlId`. The `child_ids()` example (lines 646–695) must be
removed or replaced — subordinate elements are not individually addressable
from the repository.

---

### 3.2 Operation Field Differences — HIGH

**Severity:** HIGH  
**Classification:** Design document uses wrong field set

#### Document says (lines 557–559)

```rust
pub struct Operation {
    pub base: ElementBase,
    pub return_type_id: Option<UmlId>,
    pub parameters: Vec<Parameter>,
    pub is_query: bool,          // ← sole behavioural flag
}
```

#### Implementation has (lines 155–164)

```rust
pub struct Operation {
    // ...
    pub is_static: bool,         // ← class-level, not instance-level
    pub is_abstract: bool,       // ← no implementation
    pub is_virtual: bool,        // ← overridable
}
// No is_query
```

#### Analysis

The C++ codebase provides the ground truth. `UMLOperation` (in `umloperation.h`)
uses `m_bConst` (which IS the "is query" concept — line 84: "Holds the isQuery
attribute of the <UML:Operation>"). However, it ALSO inherits `isAbstract()` and
`isStatic()` from `UMLObject` (lines 169, 181 of `umlobject.h`), and adds
`isVirtual()` (line 69) and `isInline()` (line 71) at the operation level.

The implementation's choice of `is_static`, `is_abstract`, `is_virtual` is more
immediately useful:
- `is_static` distinguishes class methods from instance methods (critical for
  code generation)
- `is_abstract` marks pure virtual methods (critical for interface generation)
- `is_virtual` marks overridable methods (critical for polymorphism)

`is_query` (the `m_bConst` equivalent) is less essential for v1 code generation
and can be added in a future milestone.

#### Recommendation

**UPDATE THE DOCUMENT** to specify: `is_static: bool`, `is_abstract: bool`,
`is_virtual: bool`. Note that `is_query` may be added in a future milestone.

---

### 3.3 TemplateParameter Design — HIGH

**Severity:** HIGH  
**Classification:** Design document overcomplicates template parameters

#### Document says (lines 571–576)

```rust
pub struct TemplateParameter {
    pub base: ElementBase,       // independent identity
    pub type_id: Option<UmlId>,  // UML type reference
    pub default_value: String,   // always present, unused in practice
}
```

#### Implementation has (lines 170–177)

```rust
pub struct TemplateParameter {
    pub name: String,            // "T", "K", "V"
    pub constraint: Option<String>,  // "class", "Comparable"
}
```

#### Analysis

The implementation's design is simpler and correct:

1. **Template parameters are not UML types.** A template parameter like `T` or
   `K` is a placeholder, not a model element. It does not need `ElementBase` or
   a `UmlId`.

2. **`type_id` is wrong for template parameters.** In UML, template parameters
   may have optional *constraints* (e.g., `T extends Comparable`), not type
   references. The implementation's `constraint: Option<String>` correctly
   captures this.

3. **`default_value` is irrelevant.** Template parameters in C++/Java/UML do
   not have default values in the same sense that operation parameters do. The
   document's `default_value: String` (non-optional, always present) makes no
   semantic sense for template parameters.

4. **C++ alignment.** The C++ `UMLTemplate` class (in the inheritance tree
   at `umltemplate.h`) is essentially a thin wrapper around a name string.
   It does not carry a type reference or default value.

#### Recommendation

**UPDATE THE DOCUMENT** to match the implementation. Template parameters are
`{ name: String, constraint: Option<String> }` — no ElementBase, no UmlId,
no type_id, no default_value.

---

### 3.4 EnumLiteral Design — HIGH

**Severity:** HIGH  
**Classification:** Document has incorrect value semantics

#### Document says (lines 579–584)

```rust
pub struct EnumLiteral {
    pub base: ElementBase,       // independent identity
    pub value: String,           // non-optional — always present
}
```

#### Implementation has (lines 182–189)

```rust
pub struct EnumLiteral {
    pub name: String,
    pub value: Option<String>,   // optional — implicit values are common
}
```

#### Analysis

The implementation is correct:

1. **Enum literals often have implicit values.** In C enum: `enum Color { Red,
   Green, Blue }` — values are `0, 1, 2` implicitly. Making `value` optional
   correctly represents this.

2. **Enum literals are strictly subordinate.** They have no existence separate
   from their owning `Enum`. No `ElementBase` or `UmlId` is needed.

3. **C++ alignment.** The C++ `UMLEnumLiteral` inherits from
   `UMLClassifierListItem`, which is stored in `UMLCanvasObject::m_List`
   (subordinates). It is never independently referenced.

#### Recommendation

**UPDATE THE DOCUMENT** to match the implementation: `value: Option<String>` and
no `ElementBase`.

---

### 3.5 Parameter `default_value` — MEDIUM

**Severity:** MEDIUM  
**Classification:** Incorrect optionality in document

#### Document says (line 568)

```rust
pub struct Parameter {
    pub name: String,
    pub type_id: Option<UmlId>,
    pub direction: ParameterDirection,
    pub default_value: String,   // ← always present
}
```

#### Implementation has (line 137)

```rust
pub struct Parameter {
    // ...
    pub default_value: Option<String>,  // ← optional
}
```

#### Analysis

Parameters do not always have default values. In C++, `void foo(int x, int y =
42)` — `x` has no default, `y` does. The implementation's `Option<String>` is
correct. The document's `String` would require an empty string sentinel for "no
default," which is ambiguous (is empty string a valid default?).

#### Recommendation

**UPDATE THE DOCUMENT** to use `Option<String>`. This is a mechanical fix.

---

### 3.6 Field Name `classifier_data` vs. `classifier` — LOW

**Severity:** LOW  
**Classification:** Cosmetic naming difference

#### Document says (lines 521, 528, 535)

```rust
pub struct Class {
    pub base: ElementBase,
    pub classifier_data: ClassifierData,   // ← document name
}
pub struct Interface {
    pub base: ElementBase,
    pub classifier_data: ClassifierData,   // ← document name
}
pub struct Enum {
    pub base: ElementBase,
    pub classifier_data: ClassifierData,   // ← document name
    pub literals: Vec<EnumLiteral>,
}
```

#### Implementation has (lines 299, 332, 356)

```rust
pub struct Class {
    pub base: ElementBase,
    pub classifier: ClassifierData,   // ← shorter implementation name
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
```

#### Analysis

Purely cosmetic. Both names are clear in context. The implementation's
`classifier` is shorter and reads naturally: `my_class.classifier.attributes`.
The document's `classifier_data` is more explicit about the structural role but
verbosely repetitive since the type is already `ClassifierData`.

The implementation uses `classifier_data()` / `classifier_data_mut()` as
method names on `ModelElement` (lines 471, 481 of `elements.rs`) — this is
fine since those are accessor methods that return `Option<&ClassifierData>`.

#### Recommendation

**UPDATE THE DOCUMENT** to use `classifier` for struct field names. The
accessor method names `classifier_data()` / `classifier_data_mut()` are fine
as-is (they convey that an Option is returned).

---

### 3.7 ModelRepository Not Yet Implemented — INFORMATIONAL

**Severity:** INFORMATIONAL  
**Classification:** Expected sequencing, not a divergence

#### Document describes (lines 795–849)

Full `ModelRepository` struct with `SlotMap<UmlId, ModelElement>` and methods
`insert`, `get`, `get_mut`, `remove`, `iter`, `len`, `is_empty`.

#### Implementation has (`model.rs`, lines 1–14)

```rust
/// Placeholder for the full model.
///
/// Will be replaced with `ModelRepository` when the arena-based storage
/// is implemented (Phase 2 / Milestone 4).
#[derive(Debug, Clone, Default)]
pub struct Model {
    _private: (),
}
```

#### Analysis

This is expected sequencing — `ModelRepository` is a Phase 2 / Milestone 4
deliverable. The document correctly describes the intended design. No
divergence to fix, just timing.

#### Recommendation

No change required. Add a note in the design document: "ModelRepository is
implemented in Phase 2 / Milestone 4."

---

### 3.8 `type_name` Dual-Reference Pattern — LOW

**Severity:** LOW  
**Classification:** Document omits essential field

#### Document says (lines 543–544)

```rust
pub struct Attribute {
    pub base: ElementBase,
    pub type_id: Option<UmlId>,      // ← only type reference
    pub default_value: String,
}
// No type_name field
```

#### Implementation has (lines 105–120)

```rust
pub struct Attribute {
    pub name: String,
    pub type_id: Option<UmlId>,      // ← UML type reference (class, interface, enum)
    pub type_name: Option<String>,   // ← fallback: "int", "String", "bool", "float"
    pub visibility: Visibility,
    pub initial_value: Option<String>,
    pub is_static: bool,
}
```

#### Analysis

The implementation's `type_name` is essential for real-world use. When importing
source code, types like `int`, `String`, `bool`, `float` do not correspond to
UML model elements. The `type_name` fallback avoids forcing every primitive
type to be modeled as a UML element.

The C++ codebase has this exact pattern: `UMLClassifierListItem` stores the type
as a string when no corresponding `UMLClassifier` exists for it.

#### Recommendation

**UPDATE THE DOCUMENT** to document the `type_id` / `type_name` dual-reference
pattern on `Attribute`. Add a note: "`type_name` is used when the attribute's
type is a primitive or language built-in that has no UML model element. When
`type_id` is `Some`, it takes precedence for UML-aware tooling."

---

### 3.9 Operation `type_name` Fallback — LOW

**Severity:** LOW  
**Classification:** Same pattern as Attribute, but for return type

#### Document says (line 554)

```rust
pub struct Operation {
    pub return_type_id: Option<UmlId>,   // ← only return type reference
}
```

#### Implementation has (lines 143–150)

```rust
pub struct Operation {
    pub return_type_id: Option<UmlId>,      // ← UML type reference
    pub return_type_name: Option<String>,   // ← fallback: "void", "int", "String"
}
```

#### Analysis

Same pattern as attribute (divergence 3.8). Return types like `void`, `int`,
`String` need a string fallback when no UML element exists for them. The
C++ codebase follows this pattern too.

#### Recommendation

**UPDATE THE DOCUMENT** to add `return_type_name: Option<String>` on `Operation`.

---

## 4. Architecture Principles Cross-Check

Each of the 15 architectural promises in the design document was verified
against the implementation:

| # | Principle | Doc (section) | Impl evidence | Verdict |
|---|-----------|---------------|---------------|---------|
| 1 | No inheritance emulation | §2 (line 170) | Composition throughout; no trait inheritance chains | ✅ PASS |
| 2 | Flat type hierarchy | §2 (line 184) | `ModelElement` is a single enum (4 variants, line 396) | ✅ PASS |
| 3 | Composition over inheritance | §2 (line 200); §3.4 | `ElementBase` embedded in all concrete types; `ClassifierData` embedded in classifiers | ✅ PASS |
| 4 | Type-safe dispatch | §2 (line 212) | `match` on `ModelElement` in `object_type()`, `base()`, `is_classifier()` | ✅ PASS |
| 5 | ID-based references | §2 (line 227) | `Package.children: Vec<UmlId>`; `ElementBase.stereotype_id: Option<UmlId>` | ✅ PASS |
| 6 | Crate boundaries | §3.6 | `uml-core` has no external deps beyond `serde`, `uuid` | ✅ PASS |
| 7 | Ownership model | §3.8 | Clear single-owner model: arena will own all elements; packages store IDs only | ✅ PASS |
| 8 | Serialization strategy | §3.6 (line 631) | `#[derive(Serialize, Deserialize)]` on all types; tests confirm round-trip (lines 703–757) | ✅ PASS |
| 9 | Extension strategy | §5 (line 1056) | Adding type = struct + enum variant + 2 match arms; compiler enforces exhaustiveness | ✅ PASS |
| 10 | NamedElement trait | §3.3 (line 347) | Trait defined (line 22); implemented on `ModelElement` (line 493) | ✅ PASS |
| 11 | Package specified | §3.7 (line 698) | `Package` with `add_child`, `remove_child`, `child_ids`, `child_count` (lines 255–290) | ✅ PASS |
| 12 | Classifier specified | §3.4 (line 455) | `ClassifierData` with `add_attribute`, `add_operation`, `add_template` (lines 197–235) | ✅ PASS |
| 13 | Class specified | §3.5 | `Class::new()` and `Class::new_abstract()` (lines 302–323) | ✅ PASS |
| 14 | Interface specified | §3.5 | `Interface::new()` with `is_abstract: true` (lines 335–347) | ✅ PASS |
| 15 | Enum specified | §3.5 | `Enum::new()` with `add_literal()` (lines 362–386) | ✅ PASS |

**Result:** All 15 architecture principles pass. The implementation faithfully
implements every documented principle. The divergences are in the concrete
type details, not in the architecture.

---

## 5. C++ Codebase Validation

The review verified key claims against the C++ Umbrello codebase:

### 5.1 Subordinate vs. Standalone Storage

| Storage location | C++ source | Contains | Rust equivalent |
|------------------|------------|----------|-----------------|
| `UMLCanvasObject::m_List` | `umlcanvasobject.h:97` | Subordinates: `UMLAttribute`, `UMLOperation`, `UMLEnumLiteral`, etc. | `ClassifierData.attributes`, `ClassifierData.operations`, `Enum.literals` (inline Vecs) |
| `UMLPackage::m_objects` | `umlpackage.h:78` | Standalone: `UMLClass`, `UMLInterface`, `UMLEnum`, `UMLPackage`, etc. | `ModelRepository` (future SlotMap) |

The C++ comment at `umlpackage.h:75-76` explicitly acknowledges the distinction:
> "This design may be revisited — m_objects could be merged into
> UMLCanvasObject::m_List."

### 5.2 Operation Flags

| Flag | C++ source | Implementation |
|------|------------|----------------|
| `isAbstract()` | `umlobject.h:169` | `Operation.is_abstract` ✅ |
| `isStatic()` | `umlobject.h:181` | `Operation.is_static` ✅ |
| `isVirtual()` | `umloperation.h:69` | `Operation.is_virtual` ✅ |
| `m_bConst` (isQuery) | `umloperation.h:84` | Not in implementation ❔ (future) |

The implementation covers the three most essential operation flags. `is_query`
(the `m_bConst` equivalent) can be added later.

### 5.3 Type Name Fallback

The C++ `UMLClassifierListItem` stores a plain `QString` type name alongside
the UML type ID. The `type_name` fields on `Attribute` and `Operation` in the
implementation faithfully reproduce this pattern.

---

## 6. Summary of Required Remediations

All remediations are documentation changes. **Zero code changes are requested.**

| # | Severity | Divergence | Remediation | Affected doc lines |
|---|----------|------------|-------------|---------------------|
| 1 | CRITICAL | Subordinate types given ElementBase/UmlId | Remove ElementBase/UmlId from Attribute, Operation, Parameter, TemplateParameter, EnumLiteral. Rewrite as pure value types. Remove `child_ids()` example. | 509–585, 646–695 |
| 2 | HIGH | Operation has `is_query` instead of is_static/is_abstract/is_virtual | Replace `is_query` with `is_static`, `is_abstract`, `is_virtual` | 557–559 |
| 3 | HIGH | TemplateParameter has base, type_id, default_value | Rewrite as `{ name: String, constraint: Option<String> }` | 571–576 |
| 4 | HIGH | EnumLiteral has non-optional `value` and ElementBase | Rewrite as `{ name: String, value: Option<String> }` | 579–584 |
| 5 | MEDIUM | Parameter `default_value: String` (always present) | Change to `default_value: Option<String>` | 568 |
| 6 | LOW | Field name `classifier_data` on Class/Interface/Enum | Rename to `classifier` | 521, 528, 535 |
| 7 | INFORMATIONAL | ModelRepository not yet implemented | No change. Add note: "Phase 2 / Milestone 4" | 795–849 |
| 8 | LOW | Attribute lacks `type_name` | Add `type_name: Option<String>` with dual-reference explanation | 540–548 |
| 9 | LOW | Operation lacks `return_type_name` | Add `return_type_name: Option<String>` | 551–559 |

---

## 7. What Must NOT Change

The following architectural decisions are correct in both document and
implementation and must not be altered:

| Element | Rationale |
|---------|-----------|
| `ModelElement` enum with variant-per-type | Correct — enables exhaustive match, flat storage, serde tagged union |
| `ElementBase` as embedded struct | Correct — composition over inheritance, no virtual dispatch |
| `ClassifierData` as embedded struct | Correct — shared classifier behaviour without base class |
| `NamedElement` trait | Correct — uniform accessor protocol implemented via single match |
| `UmlId` as UUID v4 wrapper | Correct — ID-based references, no raw pointers |
| `Package` with `Vec<UmlId>` children | Correct — ID-based containment, no ownership cycles |
| `#[derive(Serialize, Deserialize)]` on all types | Correct — serde for JSON round-trip, decoupled XMI crate |
| Flat hierarchy (no base classes) | Correct — avoids fragile base class problem |
| Composition pattern (embedded structs) | Correct — avoids diamond problem, no virtual methods |
| `ObjectType` enum for type discrimination | Correct — replaces 28 `isUML*()` methods |
| `Visibility` enum with Public/Protected/Private/Implementation | Correct — standard UML visibility |
| `ParameterDirection` enum | Correct — In/Out/InOut/Return directions |
| `Enum.literals: Vec<EnumLiteral>` | Correct — enum-specific field on enum type |
| `Interface` always `is_abstract: true` | Correct — UML semantics |
| `Class::new_abstract()` constructor | Correct — convenience constructor |

---

## 8. Final Recommendation

### APPROVED — implementation is the authoritative design

The implementation in `crates/uml-core/src/elements.rs` is **approved as the
authoritative design** for the uml-core domain model.

The implementation makes the right engineering choices in every divergence:

1. **Subordinate elements as pure value types.** Attribute, Operation,
   Parameter, TemplateParameter, and EnumLiteral are correctly modeled as
   inline data within their owning classifier. They do not need independent
   identity, arena slots, or `ElementBase`. This matches UML semantics, the
   C++ codebase, and Rust's ownership model.

2. **Practical field sets.** The implementation's `is_static`/`is_abstract`/
   `is_virtual` on Operation, `constraint` on TemplateParameter, optional
   `value` on EnumLiteral, and optional `default_value` on Parameter are all
   correct and useful.

3. **`type_name` fallback pattern.** The dual `type_id`/`type_name` approach
   for type references is essential for real-world code import and matches
   the C++ codebase pattern.

### Action Required

**Update `domain_model_v1.md`** to reflect the implementation. Specifically:

1. Replace §3.5 concrete type definitions (lines 509–585) with the
   implementation's struct definitions.
2. Remove or rewrite the `child_ids()` example (lines 644–695) — subordinate
   types are not independently addressable from the repository.
3. Rename `classifier_data` → `classifier` in struct definitions.
4. Add documentation of the `type_id`/`type_name` dual-reference pattern.
5. Add a note that `ModelRepository` is pending (Phase 2 / Milestone 4).

### Verification

After the document is updated, the following manual verification is recommended:

- [ ] Every struct field in `elements.rs` is documented in the design document
- [ ] Every `ModelElement` variant is documented
- [ ] Every trait and impl block is described
- [ ] The subordinate vs. standalone distinction is clearly explained
- [ ] Code examples in the document compile against the implementation

---

> **Signed:** Umbrello-RS Reviewer  
> **Date:** 2026-06-23  
> **Disposition:** APPROVED — documentation update required  
> **Implementation:** `crates/uml-core/src/elements.rs` — NO CHANGES  
> **Documentation:** `docs/domain_model_v1.md` — UPDATE to match implementation
