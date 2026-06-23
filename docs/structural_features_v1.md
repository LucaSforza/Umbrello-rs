# Structural Features — TypeReference Extraction

**Status:** Design v1  
**Audit Ref:** D1 (HIGH) — Duplicated type reference pattern  
**Target crate:** `uml-core`  
**Date:** 2026-06-23  
**Related docs:** `phase1_architecture_audit.md`, `domain_model_v1.md`

---

## 1. Problem Statement

The `phase1_architecture_audit.md` identifies D1 as the single most impactful
refactoring target in the codebase. Three distinct structs carry the same pair
of fields for referencing a type:

| Struct | Fields | Purpose |
|--------|--------|---------|
| `Attribute` | `type_id: Option<UmlId>`, `type_name: Option<String>` | Attribute type |
| `Operation` | `return_type_id: Option<UmlId>`, `return_type_name: Option<String>` | Return type |
| `Parameter` | `type_id: Option<UmlId>`, `type_name: Option<String>` | Parameter type |

That is **6 fields across 3 structs** with identical semantics, identical serde
attributes, and identical validation logic — but no shared abstraction.

### 1.1 Consequences of Duplication

**Maintenance hazard.** Any change to the type-reference representation
(e.g., adding a `type_alias` field, changing `type_name` to use a `Box<str>`,
or adding multiplicity) must be applied in three independent locations.

**No invariant enforcement.** The two fields form a logical XOR — at most one
should be `Some`. Nothing prevents constructing an `Attribute` where both
`type_id` and `type_name` are `Some`, which creates ambiguity: should the
type be resolved via the model or the string? Currently, callers must
manually ensure consistency.

**Undocumented dual-reference protocol.** API consumers receive no guidance
about which field to set. In the C++ codebase, this was accepted practice;
in Rust, we can encode the constraint in the type system.

**Repeated serde boilerplate.** Each field pair needs identical
`#[serde(default, skip_serializing_if = "Option::is_none")]` attributes.

**Duplicated doc comments.** The same "reference to a UML type by ID" / "type
name (fallback)" doc pattern is written three times.

### 1.2 Scope

Only `Attribute`, `Operation`, and `Parameter` have this pattern.
`TemplateParameter` does not reference a type (it *is* a type variable).
`EnumLiteral` does not have a type reference. `ElementBase::stereotype_id`
is a single `Option<UmlId>` without a fallback name — it is always a model
reference, so it does not fit the pattern.

---

## 2. Solution: `TypeReference` Struct

Extract the dual-reference pattern into a dedicated struct with named
constructors, query methods, and validation.

### 2.1 Definition

```rust
/// A reference to a type in the UML model.
///
/// Types can be either:
/// - A UML classifier (class, interface, enumeration, datatype) referenced by
///   `model_id` — resolved against the model repository at runtime.
/// - A primitive or external type referenced by name (e.g., `"int"`,
///   `"String"`, `"float"`) — self-describing, no model lookup needed.
///
/// At most one of `model_id` or `type_name` should be `Some`. When both are
/// `None`, the type is unspecified (e.g., a void return type on an operation
/// that has no return value declared).
///
/// # Validity
///
/// Use [`TypeReference::is_valid()`] to check that the reference is internally
/// consistent. The three named constructors (`unspecified`, `model`, `primitive`)
/// always produce valid references. Invalid state can arise only via
/// deserialization or direct field mutation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeReference {
    /// Reference to a UML model element (classifier).
    ///
    /// When `Some`, the type is resolved by looking up this `UmlId` in the
    /// model repository. The owning element is said to have a "resolved" type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<UmlId>,

    /// Type name for primitives or external types.
    ///
    /// When `Some` (and `model_id` is `None`), the type is a primitive or
    /// external type identified by its string name. Examples: `"int"`,
    /// `"String"`, `"boolean"`, `"void"`.
    ///
    /// When both `model_id` and `type_name` are `None`, the type is unspecified.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
}

impl TypeReference {
    /// Create an unspecified type reference (both fields `None`).
    ///
    /// Used for void return types, untyped parameters, or attributes whose
    /// type has not yet been assigned.
    #[must_use]
    pub fn unspecified() -> Self {
        Self { model_id: None, type_name: None }
    }

    /// Create a type reference to a UML model element.
    ///
    /// The referenced element must exist in the model for the reference to
    /// be resolvable at runtime. See [`UmlModel::validate_references`].
    #[must_use]
    pub fn model(id: UmlId) -> Self {
        Self { model_id: Some(id), type_name: None }
    }

    /// Create a type reference to a primitive or external type by name.
    ///
    /// Examples: `TypeReference::primitive("int")`,
    /// `TypeReference::primitive("String")`.
    #[must_use]
    pub fn primitive(name: impl Into<String>) -> Self {
        Self { model_id: None, type_name: Some(name.into()) }
    }

    // ── Query methods ────────────────────────────────────────────────

    /// Returns `true` if the type is resolved (has either a `model_id` or a
    /// `type_name`).
    ///
    /// The complement, `!is_resolved()`, indicates an unspecified type (both
    /// fields `None`). This is distinct from "void" — in UML, `void` is a
    /// named type, while unspecified means the type has not been assigned.
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.model_id.is_some() || self.type_name.is_some()
    }

    /// Returns `true` if this references a UML model element.
    #[must_use]
    pub fn is_model_type(&self) -> bool {
        self.model_id.is_some()
    }

    /// Returns `true` if this is a primitive or external type name.
    #[must_use]
    pub fn is_primitive(&self) -> bool {
        self.type_name.is_some()
    }

    /// Validate that this reference is internally consistent.
    ///
    /// Returns `true` if at most one of `model_id` or `type_name` is set.
    /// Both being `None` is valid (unspecified type).
    /// Both being `Some` is invalid (ambiguous — which representation wins?).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        // Both None is valid (unspecified). Only one of Some is valid.
        // Both Some is invalid (ambiguous).
        !(self.model_id.is_some() && self.type_name.is_some())
    }
}
```

### 2.2 Design Rationale

**Why not an enum?** An enum like `enum TypeRef { Model(UmlId), Primitive(String), Unspecified }`
would enforce validity at the type level. However, it would:
- Break serde compatibility with a two-field JSON object (enums serialize
  differently from structs, requiring `#[serde(tag = "...")]` or
  `#[serde(untagged)]` which are less ergonomic).
- Prevent partial deserialization — an XMI import might encounter a
  `type_id` reference but no `type_name`, or vice versa, and an enum cannot
  represent "only model_id is known" without a wrapper.
- Make pattern matching more verbose — callers matching on three variants
  instead of two `Option` fields.

A struct with two `Option` fields is the pragmatic choice. The three
constructors serve as the primary API, and `is_valid()` catches the invalid
state (both `Some`).

**Why model_id instead of type_id?** The audit (and the existing code) uses
`type_id`, but that collides with the noun "type" — which is both a UML
concept and a Rust concept. Renaming to `model_id` disambiguates: it is the
ID of a model element, not a Rust type ID. This also mirrors the naming in
`Relationship` (`source_id`, `target_id`).

**Why `is_valid()` returns `bool` instead of `Result`?** The `bool` return
matches the validation-as-query pattern (check without side effects). Callers
that need errors can add their own context. The method is designed for
assertions and defensive checks, not error propagation.

### 2.3 Serde Representation

```json
// Unspecified
{ "model_id": null, "type_name": null }

// Model type
{ "model_id": "550e8400-e29b-41d4-a716-446655440000", "type_name": null }

// Primitive type
{ "model_id": null, "type_name": "int" }
```

With `#[serde(default, skip_serializing_if = "Option::is_none")]`:

```json
// Unspecified — both fields skipped (empty object)
{}

// Model type
{ "model_id": "550e8400-e29b-41d4-a716-446655440000" }

// Primitive type
{ "type_name": "int" }
```

The `skip_serializing_if` attribute means unspecified types serialize as
empty objects `{}`, which is the most compact representation. Deserialization
accepts all forms (with `null` fields, missing fields, or empty objects).

### 2.4 `display_name()` (Optional, for Code Generation)

For code generation, a `display_name()` helper resolves the type to a string:

```rust
impl TypeReference {
    /// Display the type as a string for code generation or diagnostics.
    ///
    /// Returns the `type_name` if present. Otherwise looks up the model
    /// element by `model_id` and returns its name. If neither field is set,
    /// returns `"void"` as the default unspecified type name.
    ///
    /// If the model is provided but the ID is not found, returns
    /// `"<unknown:{id}>"` to signal the dangling reference.
    #[must_use]
    pub fn display_name(&self, model: Option<&UmlModel>) -> String {
        if let Some(ref name) = self.type_name {
            name.clone()
        } else if let Some(id) = self.model_id {
            model
                .and_then(|m| m.get(id))
                .map(|e| e.name().to_string())
                .unwrap_or_else(|| format!("<unknown:{id}>"))
        } else {
            "void".to_string()
        }
    }
}
```

`display_name()` is *not* part of the core TypeReference abstraction — it
is a convenience for code generation and diagnostics. It depends on
`UmlModel`, so it creates a dependency edge from `elements.rs` to
`repository.rs`. Two options:

1. **Define `display_name()` on `TypeReference` in `elements.rs`** — requires
   importing `UmlModel`, which currently lives in `repository.rs`. This
   creates a circular-ish dependency (elements → repository is fine;
   repository → elements already exists). **Decision: acceptable.**
2. **Define `display_name()` as a free function in `repository.rs` or in a
   utility module** — avoids the import but separates the method from the
   type. Less discoverable.

**Recommendation:** Include `display_name()` in `TypeReference` for
discoverability. The dependency from `elements.rs` to `repository.rs` is
not circular (repository already imports elements).

---

## 3. Updated Structs

### 3.1 Attribute

Replace the two-field pattern with a single `type_ref: TypeReference` field:

```rust
/// A classifier attribute (field / member variable).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Attribute {
    /// Attribute name.
    pub name: String,
    /// The type of this attribute.
    ///
    /// References either a UML classifier (via `model_id`) or a primitive
    /// type (via `type_name`). See [`TypeReference`] for details.
    pub type_ref: TypeReference,
    /// Visibility.
    pub visibility: Visibility,
    /// Initial value expression (e.g., `"0"`, `"null"`, `"Some(42)"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial_value: Option<String>,
    /// Whether the attribute is static (class-level).
    #[serde(default)]
    pub is_static: bool,
}
```

**Diff from current:**
- Removed `type_id: Option<UmlId>` (line 109)
- Removed `type_name: Option<String>` (line 111)
- Added `type_ref: TypeReference`

### 3.2 Parameter

```rust
/// An operation parameter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Parameter {
    /// Parameter name.
    pub name: String,
    /// The type of this parameter.
    ///
    /// References either a UML classifier (via `model_id`) or a primitive
    /// type (via `type_name`). See [`TypeReference`] for details.
    pub type_ref: TypeReference,
    /// Parameter direction (in, out, inout, return).
    pub direction: ParameterDirection,
    /// Default value expression (e.g., `"0"`, `"None"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
}
```

**Diff from current:**
- Removed `type_id: Option<UmlId>` (line 130)
- Removed `type_name: Option<String>` (line 132)
- Added `type_ref: TypeReference`

### 3.3 Operation

```rust
/// A classifier operation (method).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Operation {
    /// Operation name.
    pub name: String,
    /// The return type of this operation.
    ///
    /// References either a UML classifier (via `model_id`) or a primitive
    /// type (via `type_name`). An unspecified return type (both `None`)
    /// represents a void return. See [`TypeReference`] for details.
    pub return_type: TypeReference,
    /// Formal parameters.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    /// Visibility.
    pub visibility: Visibility,
    /// Whether the operation is static (class-level).
    #[serde(default)]
    pub is_static: bool,
    /// Whether the operation has no implementation.
    #[serde(default)]
    pub is_abstract: bool,
    /// Whether the operation is virtual / overridable.
    #[serde(default)]
    pub is_virtual: bool,
}
```

**Diff from current:**
- Removed `return_type_id: Option<UmlId>` (line 148)
- Removed `return_type_name: Option<String>` (line 150)
- Added `return_type: TypeReference`

**Field naming note:** The field is named `return_type` (not `type_ref`) to
distinguish it from parameter/attribute `type_ref`. This makes the field's
purpose self-documenting: "the return type of this operation." In contrast,
`Attribute.type_ref` and `Parameter.type_ref` are unambiguous because these
types only have one type reference each.

### 3.4 ClassifierData — No Structural Change

`ClassifierData` stores `Vec<Attribute>` and `Vec<Operation>`. The struct
itself does not change. However, the inner field types of its elements change:

```rust
pub struct ClassifierData {
    pub attributes: Vec<Attribute>,   // Attribute now uses TypeReference
    pub operations: Vec<Operation>,   // Operation now uses TypeReference
    pub templates: Vec<TemplateParameter>,
}
```

The `add_attribute()` and `add_operation()` methods accept the updated types
— callers must now construct `Attribute` with `type_ref: TypeReference`
instead of `type_id` + `type_name`.

---

## 4. Impact on Reference Validation

### 4.1 `validate_classifier_references()` Changes

The private helper `UmlModel::validate_classifier_references()` currently
accesses `attr.type_id`, `op.return_type_id`, and `param.type_id` directly.
With `TypeReference`, these become:

```rust
fn validate_classifier_references(
    &self,
    source_id: UmlId,
    classifier: &ClassifierData,
    errors: &mut Vec<ReferenceError>,
) {
    for attr in &classifier.attributes {
        // Before: if let Some(type_id) = attr.type_id { ... }
        // After:
        if let Some(type_id) = attr.type_ref.model_id {
            if !self.contains(type_id) {
                errors.push(ReferenceError {
                    source_id,
                    field: ReferenceField::AttributeType,
                    target_id: type_id,
                });
            }
        }
    }
    for op in &classifier.operations {
        // Before: if let Some(ret_id) = op.return_type_id { ... }
        // After:
        if let Some(ret_id) = op.return_type.model_id {
            if !self.contains(ret_id) {
                errors.push(ReferenceError {
                    source_id,
                    field: ReferenceField::OperationReturnType,
                    target_id: ret_id,
                });
            }
        }
        for param in &op.parameters {
            // Before: if let Some(param_type_id) = param.type_id { ... }
            // After:
            if let Some(param_type_id) = param.type_ref.model_id {
                if !self.contains(param_type_id) {
                    errors.push(ReferenceError {
                        source_id,
                        field: ReferenceField::ParameterType,
                        target_id: param_type_id,
                    });
                }
            }
        }
    }
}
```

### 4.2 `ReferenceField` Enum — No Change

The `ReferenceField` enum keeps its current variants:

```rust
pub enum ReferenceField {
    PackageChild,
    AttributeType,         // Still accurate — references the type field
    OperationReturnType,   // Still accurate
    ParameterType,         // Still accurate
    Stereotype,
    RelationshipSource,
    RelationshipTarget,
}
```

The variant names describe *what kind of reference* is dangling, not the
internal representation. They remain correct: `AttributeType` means "an
attribute's type reference is dangling."

### 4.3 `validate_references()` — No Structural Change

The main `validate_references()` method stays the same — it delegates to
`validate_classifier_references()` for all classifier elements. The only
difference is that the borrowed fields are now `TypeReference` structs
instead of bare `Option<UmlId>` fields.

---

## 5. Full File Changes Summary

### `crates/uml-core/src/elements.rs`

| Change | Lines affected |
|--------|---------------|
| Add `TypeReference` struct + impl | ~60 new lines |
| Remove `type_id` from `Attribute` | 1 line removed |
| Remove `type_name` from `Attribute` | 1 line removed |
| Add `type_ref` to `Attribute` | 1 line added |
| Remove `type_id` from `Parameter` | 1 line removed |
| Remove `type_name` from `Parameter` | 1 line removed |
| Add `type_ref` to `Parameter` | 1 line added |
| Remove `return_type_id` from `Operation` | 1 line removed |
| Remove `return_type_name` from `Operation` | 1 line removed |
| Add `return_type` to `Operation` | 1 line added |
| Update all test code (7 test locations) | ~14 lines changed |
| **Net** | **+~50 lines** |

### `crates/uml-core/src/repository.rs`

| Change | Lines affected |
|--------|---------------|
| `validate_classifier_references`: `attr.type_id` → `attr.type_ref.model_id` | 1 line |
| `validate_classifier_references`: `op.return_type_id` → `op.return_type.model_id` | 1 line |
| `validate_classifier_references`: `param.type_id` → `param.type_ref.model_id` | 1 line |
| Update `validate_references_dangling_type` test | ~5 lines |
| Update any other tests that construct `Attribute`/`Operation`/`Parameter` | varies |
| **Net** | **+~10 lines** |

### `crates/uml-core/src/lib.rs`

No changes — `TypeReference` is re-exported implicitly via `pub mod elements`
and the existing `pub use` patterns. If TypeReference is used in the public
API (which it is — it's a field of public structs), it is automatically
visible.

### Test files

| File | Changes |
|------|---------|
| `elements.rs` test block | Update 7 struct literals to use `type_ref` |
| `repository.rs` test block | Update 1 struct literal + any affected assertions |
| `tests/serde_roundtrip.rs` | No changes (no direct type_id/type_name usage) |

---

## 6. Serde Compatibility

### 6.1 Breaking Change

This is a **breaking serialization change**. The JSON representation of
`Attribute`, `Operation`, and `Parameter` changes:

```diff
// Before
- { "name": "age", "type_id": "uuid...", "type_name": null, "visibility": "private" }

// After
+ { "name": "age", "type_ref": { "model_id": "uuid..." }, "visibility": "private" }
```

### 6.2 Why This Is Acceptable

1. **No consumers of the serialized format exist.** XMI import/export is not
   yet implemented. The serialized JSON is only used for in-memory round-trip
   testing (`serde_json::to_string` / `from_str`).

2. **No persistent storage.** There is no `UmlModel::save()` or
   `UmlModel::load()` — the model lives only in memory during a session.

3. **The change is mechanical.** It is a one-to-one field replacement
   (`type_id` → `type_ref.model_id`). Downstream code (none yet) would
   need to update field access paths, not logic.

4. **Any future XMI import would translate the XMI type reference into
   `TypeReference` anyway.** The internal representation is opaque to XMI
   consumers.

### 6.3 Serde Round-Trip Test

```rust
#[test]
fn type_reference_serde_roundtrip() {
    let unspecified = TypeReference::unspecified();
    let model_ref = TypeReference::model(UmlId::new());
    let prim_ref = TypeReference::primitive("int");

    for tref in &[unspecified, model_ref, prim_ref] {
        let json = serde_json::to_string(tref).unwrap();
        let back: TypeReference = serde_json::from_str(&json).unwrap();
        assert_eq!(*tref, back);
    }
}
```

---

## 7. Test Plan

### 7.1 TypeReference Unit Tests (10 new tests)

```rust
#[cfg(test)]
mod type_reference_tests {
    use super::*;

    // ── Construction ──────────────────────────────────────────────────

    #[test]
    fn unspecified() {
        let t = TypeReference::unspecified();
        assert!(t.model_id.is_none());
        assert!(t.type_name.is_none());
    }

    #[test]
    fn model() {
        let id = UmlId::new();
        let t = TypeReference::model(id);
        assert_eq!(t.model_id, Some(id));
        assert!(t.type_name.is_none());
    }

    #[test]
    fn primitive() {
        let t = TypeReference::primitive("int");
        assert!(t.model_id.is_none());
        assert_eq!(t.type_name, Some("int".to_string()));
    }

    #[test]
    fn primitive_from_string() {
        let t = TypeReference::primitive("String".to_string());
        assert_eq!(t.type_name, Some("String".to_string()));
    }

    // ── Query methods ────────────────────────────────────────────────

    #[test]
    fn is_resolved_unspecified_is_false() {
        assert!(!TypeReference::unspecified().is_resolved());
    }

    #[test]
    fn is_resolved_model_type_is_true() {
        assert!(TypeReference::model(UmlId::new()).is_resolved());
    }

    #[test]
    fn is_resolved_primitive_is_true() {
        assert!(TypeReference::primitive("int").is_resolved());
    }

    #[test]
    fn is_model_type_true_for_model_ref() {
        assert!(TypeReference::model(UmlId::new()).is_model_type());
    }

    #[test]
    fn is_model_type_false_for_primitive() {
        assert!(!TypeReference::primitive("int").is_model_type());
    }

    #[test]
    fn is_primitive_true_for_primitive() {
        assert!(TypeReference::primitive("int").is_primitive());
    }

    #[test]
    fn is_primitive_false_for_model_ref() {
        assert!(!TypeReference::model(UmlId::new()).is_primitive());
    }

    // ── Validation ───────────────────────────────────────────────────

    #[test]
    fn is_valid_when_both_none() {
        assert!(TypeReference::unspecified().is_valid());
    }

    #[test]
    fn is_valid_when_only_model_id() {
        assert!(TypeReference::model(UmlId::new()).is_valid());
    }

    #[test]
    fn is_valid_when_only_type_name() {
        assert!(TypeReference::primitive("int").is_valid());
    }

    #[test]
    fn is_invalid_when_both_set() {
        let t = TypeReference {
            model_id: Some(UmlId::new()),
            type_name: Some("int".to_string()),
        };
        assert!(!t.is_valid());
    }

    // ── Display name ─────────────────────────────────────────────────

    #[test]
    fn display_name_for_primitive() {
        let t = TypeReference::primitive("int");
        assert_eq!(t.display_name(None), "int");
    }

    #[test]
    fn display_name_for_unspecified() {
        let t = TypeReference::unspecified();
        assert_eq!(t.display_name(None), "void");
    }

    // ── Serde ─────────────────────────────────────────────────────────

    #[test]
    fn serde_roundtrip_unspecified() {
        let t = TypeReference::unspecified();
        let json = serde_json::to_string(&t).unwrap();
        let back: TypeReference = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn serde_roundtrip_model() {
        let t = TypeReference::model(UmlId::new());
        let json = serde_json::to_string(&t).unwrap();
        let back: TypeReference = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn serde_roundtrip_primitive() {
        let t = TypeReference::primitive("double");
        let json = serde_json::to_string(&t).unwrap();
        let back: TypeReference = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }

    #[test]
    fn serde_deserialize_empty_object() {
        let back: TypeReference = serde_json::from_str("{}").unwrap();
        assert_eq!(back, TypeReference::unspecified());
    }

    #[test]
    fn serde_deserialize_null_fields() {
        let back: TypeReference =
            serde_json::from_str(r#"{"model_id":null,"type_name":null}"#).unwrap();
        assert_eq!(back, TypeReference::unspecified());
    }
}
```

### 7.2 Updated Attribute/Operation/Parameter Tests

The existing tests in `elements.rs` that construct `Attribute`/`Operation`/
`Parameter` with the old `type_id`/`type_name` fields must be updated:

**Before** (line 701–708):
```rust
data.add_attribute(Attribute {
    name: "count".into(),
    type_id: None,
    type_name: Some("int".into()),
    visibility: Visibility::Private,
    initial_value: Some("0".into()),
    is_static: false,
});
```

**After**:
```rust
data.add_attribute(Attribute {
    name: "count".into(),
    type_ref: TypeReference::primitive("int"),
    visibility: Visibility::Private,
    initial_value: Some("0".into()),
    is_static: false,
});
```

The affected test locations are:

| Test | Location (line) |
|------|-----------------|
| `classifier_data_add_attribute` | 701–708 |
| `classifier_data_add_operation` | 714–731 |
| `model_element_classifier_data_access` | 765–772 |
| `serde_roundtrip_class` | 838–845 |

### 7.3 Updated Repository Tests

The test `validate_references_dangling_type` (line 841) constructs an
`Attribute` with a dangling `type_id`. After the refactor:

```rust
cls.classifier.add_attribute(Attribute {
    name: "address".into(),
    type_ref: TypeReference::model(dangling),  // dangling model_id
    visibility: Visibility::Private,
    initial_value: None,
    is_static: false,
});
```

The assertion checks remain the same — they assert on `ReferenceField::AttributeType`.

### 7.4 New Integration Tests

```rust
#[test]
fn attribute_with_model_type_roundtrip() {
    let attr = Attribute {
        name: "owner".into(),
        type_ref: TypeReference::model(UmlId::new()),
        visibility: Visibility::Public,
        initial_value: None,
        is_static: false,
    };
    let json = serde_json::to_string(&attr).unwrap();
    let back: Attribute = serde_json::from_str(&json).unwrap();
    assert_eq!(attr, back);
}

#[test]
fn operation_with_parameters_roundtrip() {
    let op = Operation {
        name: "find".into(),
        return_type: TypeReference::primitive("bool"),
        parameters: vec![
            Parameter {
                name: "query".into(),
                type_ref: TypeReference::primitive("String"),
                direction: ParameterDirection::In,
                default_value: None,
            },
        ],
        visibility: Visibility::Public,
        is_static: false,
        is_abstract: false,
        is_virtual: false,
    };
    let json = serde_json::to_string(&op).unwrap();
    let back: Operation = serde_json::from_str(&json).unwrap();
    assert_eq!(op, back);
}
```

---

## 8. Migration Plan

### Step 1: Define `TypeReference` in `elements.rs`

Insert the `TypeReference` struct and its `impl` block between `ElementBase`
and `Attribute` (around line 100). This placement is logical — `TypeReference`
is used by `Attribute`, `Parameter`, and `Operation`, all of which follow.

### Step 2: Update `Attribute`, `Parameter`, `Operation`

Replace the old fields with `TypeReference` fields. Update doc comments to
cross-reference `TypeReference`.

### Step 3: Update `validate_classifier_references()` in `repository.rs`

Change the three field access paths (`attr.type_id` → `attr.type_ref.model_id`,
etc.). The `ReferenceField` enum variants do not change.

### Step 4: Update test code

- `elements.rs` tests: 4 test locations (7 struct literals)
- `repository.rs` tests: 1 test (`validate_references_dangling_type`)

### Step 5: Run `cargo test`

All 110+ tests must pass. The serde round-trip tests in `tests/serde_roundtrip.rs`
do not touch `Attribute`/`Operation`/`Parameter` directly and require no changes.

### Step 6: Run `cargo clippy` and `cargo fmt`

Ensure no new warnings or formatting issues.

### Step 7: Verify validation behavior

Construct a manual test case with `TypeReference` where both `model_id` and
`type_name` are `Some`, and verify `is_valid()` returns `false`. This is a
defensive check — deserialization could produce this state, and the check
catches it early.

### Effort Estimate

| Step | Time |
|------|------|
| Step 1: TypeReference definition | 15 min |
| Step 2: Update Attribute/Parameter/Operation | 10 min |
| Step 3: Update validate_classifier_references | 5 min |
| Step 4: Update test code | 20 min |
| Step 5: Run tests & fix | 10 min |
| Step 6: Clippy + fmt | 5 min |
| **Total** | **~65 min** |

---

## 9. What Does NOT Change

| Area | Reason |
|------|--------|
| `TemplateParameter` | No type reference — it *is* a type variable |
| `EnumLiteral` | No type reference — it has a value, not a type |
| `Package` | Contains children, not types |
| `Relationship` | Uses `source_id`/`target_id` for element references, not types |
| `ElementBase::stereotype_id` | Always a model reference (no `type_name` fallback) — different pattern |
| `ClassifierData` struct | Still holds `Vec<Attribute>` and `Vec<Operation>` — only the inner field types change |
| `NamedElement` trait | No impact — trait does not deal with types |
| `ModelElement` enum | No new variants needed |
| `UmlId` | No changes |
| `Visibility`, `ParameterDirection` | No changes |
| `ObjectType`, `AssociationType`, `DiagramType` | No changes |
| `ReferenceField` enum | Variants stay the same |
| `UmlModel` public API | All existing methods (`insert`, `remove`, `get`, `contains`, etc.) unchanged |
| Workspace crate structure | No new crates needed |
| Existing module boundaries | `TypeReference` lives in `elements.rs` alongside the types it references |

---

## 10. Future Extensions

### 10.1 Multiplicity

UML types can carry multiplicity (e.g., `String[0..1]`, `int[1..*]`). When
needed, add fields to `TypeReference`:

```rust
pub struct TypeReference {
    pub model_id: Option<UmlId>,
    pub type_name: Option<String>,
    /// Lower bound (default 1).
    pub lower: u64,
    /// Upper bound (default 1; `u64::MAX` represents `*`).
    pub upper: u64,
}
```

This is a backward-compatible addition — existing serialized data without
`lower`/`upper` fields default to `(1, 1)` via `#[serde(default)]`.

### 10.2 Type Aliases

If type aliases (e.g., `using IntList = List<int>`) are needed, add:

```rust
pub struct TypeReference {
    pub model_id: Option<UmlId>,
    pub type_name: Option<String>,
    /// Optional type alias — when set, the type is an alias for another type.
    pub alias: Option<String>,
}
```

### 10.3 TypeReference as an Enum (Future Consideration)

If the dual-Option pattern proves error-prone in practice, the struct can be
converted to an enum via `#[serde(untagged)]`:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TypeReference {
    Unspecified,
    Model { model_id: UmlId },
    Primitive { type_name: String },
}
```

This would be a breaking change requiring migration of serialized data. The
current struct design is forward-compatible with this conversion.

---

## 11. Appendix: Complete TypeReference Source

For reference, the complete `TypeReference` implementation as it should
appear in `elements.rs`:

```rust
// ─── TypeReference ──────────────────────────────────────────────────

/// A reference to a type in the UML model.
///
/// Types can be either:
/// - A UML classifier (class, interface, enumeration, datatype) referenced by
///   `model_id` — resolved against the model repository at runtime.
/// - A primitive or external type referenced by name (e.g., `"int"`,
///   `"String"`, `"float"`) — self-describing, no model lookup needed.
///
/// At most one of `model_id` or `type_name` should be `Some`. When both are
/// `None`, the type is unspecified (e.g., a void return type).
///
/// # Validity
///
/// The three named constructors (`unspecified`, `model`, `primitive`) always
/// produce valid references. Invalid state (both `Some`) can arise only via
/// deserialization or direct field mutation. Use [`TypeReference::is_valid()`]
/// to check.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TypeReference {
    /// Reference to a UML model element (classifier).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<UmlId>,

    /// Type name for primitives or external types.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
}

impl TypeReference {
    /// Create an unspecified type reference (both fields `None`).
    #[must_use]
    pub fn unspecified() -> Self {
        Self { model_id: None, type_name: None }
    }

    /// Create a type reference to a UML model element.
    #[must_use]
    pub fn model(id: UmlId) -> Self {
        Self { model_id: Some(id), type_name: None }
    }

    /// Create a type reference to a primitive or external type by name.
    #[must_use]
    pub fn primitive(name: impl Into<String>) -> Self {
        Self { model_id: None, type_name: Some(name.into()) }
    }

    /// Returns `true` if the type is resolved (has either a `model_id` or
    /// a `type_name`).
    #[must_use]
    pub fn is_resolved(&self) -> bool {
        self.model_id.is_some() || self.type_name.is_some()
    }

    /// Returns `true` if this references a UML model element.
    #[must_use]
    pub fn is_model_type(&self) -> bool {
        self.model_id.is_some()
    }

    /// Returns `true` if this is a primitive or external type name.
    #[must_use]
    pub fn is_primitive(&self) -> bool {
        self.type_name.is_some()
    }

    /// Validate that this reference is internally consistent.
    ///
    /// Returns `true` if at most one of `model_id` or `type_name` is set.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !(self.model_id.is_some() && self.type_name.is_some())
    }

    /// Display the type as a string for code generation or diagnostics.
    ///
    /// Returns the `type_name` if present. Otherwise looks up the model
    /// element by `model_id` and returns its name. If neither field is set,
    /// returns `"void"` as the default unspecified type name.
    #[must_use]
    pub fn display_name(&self, model: Option<&crate::repository::UmlModel>) -> String {
        if let Some(ref name) = self.type_name {
            name.clone()
        } else if let Some(id) = self.model_id {
            model
                .and_then(|m| m.get(id))
                .map(|e| e.name().to_string())
                .unwrap_or_else(|| format!("<unknown:{id}>"))
        } else {
            "void".to_string()
        }
    }
}
```

The `display_name` method uses a qualified path `crate::repository::UmlModel`
to avoid a circular import issue within `elements.rs`. If `UmlModel` is
re-exported from `lib.rs`, the path can be `crate::UmlModel`.

---

## 12. Appendix: Full Test Inventory (Pre-Migration)

All 21 test locations that reference `type_id`, `type_name`, `return_type_id`,
or `return_type_name` (must be updated):

| # | File | Line | Context |
|---|------|------|---------|
| 1 | `elements.rs` | 703 | `classifier_data_add_attribute` — `Attribute { type_id: None, type_name: Some("int") }` |
| 2 | `elements.rs` | 718 | `classifier_data_add_operation` — `Operation { return_type_id: None, return_type_name: Some("void") }` |
| 3 | `elements.rs` | 722 | `classifier_data_add_operation` — `Parameter { type_id: None, type_name: Some("int") }` |
| 4 | `elements.rs` | 767 | `model_element_classifier_data_access` — `Attribute { type_id: None, type_name: Some("int") }` |
| 5 | `elements.rs` | 840 | `serde_roundtrip_class` — `Attribute { type_id: None, type_name: Some("String") }` |
| 6 | `repository.rs` | 847 | `validate_references_dangling_type` — `Attribute { type_id: Some(dangling), type_name: None }` |
