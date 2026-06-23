# XMI Milestone 9 Strategy — Feature Parsing, Relationships, Cross-Reference Resolution, and Corpus Tests

> **Document:** `rust-rewrite/docs/xmi_milestone9_strategy.md`
> **Status:** Active
> **Phase:** Milestone 9 (XMI Persistence — Feature Parsing & Relationship Resolution)
> **Last updated:** 2026-06-23
>
> This document defines the implementation strategy for completing the XMI
> deserializer. Milestone 8 (M8) implemented structural element parsing
> (Package, Class, Interface, Enum, Datatype) and stereotype reference
> resolution. M9 extends the reader to handle classifier features
> (attributes, operations with parameters), all relationship types
> (generalization, association, dependency, abstraction), cross-reference
> resolution for type references, and real-corpus integration tests.
>
> **Target:** Make `read_from` + `resolve` work correctly on all 10 real
> `.xmi` files in `test/`.

---

## Table of Contents

1. [Current State (M8 baseline)](#1-current-state-m8-baseline)
2. [Scope: What M9 Adds](#2-scope-what-m9-adds)
3. [M9 Phase 1 — Classifier Feature Parsing](#3-m9-phase-1--classifier-feature-parsing)
4. [M9 Phase 2 — Relationship Parsing](#4-m9-phase-2--relationship-parsing)
5. [M9 Phase 3 — Expanded Pass 2 Cross-Reference Resolution](#5-m9-phase-3--expanded-pass-2-cross-reference-resolution)
6. [M9 Phase 4 — Real Corpus Integration Test](#6-m9-phase-4--real-corpus-integration-test)
7. [Key Design Decisions](#7-key-design-decisions)
8. [Implementation Order](#8-implementation-order)
9. [Testing Plan](#9-testing-plan)

---

## 1. Current State (M8 baseline)

The reader (`crates/uml-io/src/xmi/reader.rs`) currently:

- **Parses:** `<UML:Class>`, `<UML:Interface>`, `<UML:Enumeration>`, `<UML:DataType>`, `<UML:Model>`, `<UML:Package>`, `<UML:Stereotype>`
- **Skips via `skip_element_named`:** `<UML:Classifier.feature>`  (defers Feature parsing entirely)
- **Ignores silently:** `<UML:Association>`, `<UML:Dependency>`, `<UML:Generalization>`, `<UML:Abstraction>`, `<UML:GeneralizableElement.generalization>`
- **Pass 2 resolves:** stereotype references only (`pending_stereotypes`)

**Reader struct fields:**

```rust
pub struct XmiReader {
    id_map: HashMap<String, UmlId>,                 // XMI string ID → UmlId
    name_map: HashMap<String, Vec<UmlId>>,          // element name → UmlIds
    pending_stereotypes: Vec<(UmlId, String)>,      // (element_uml_id, stereotype_xmi_id)
    parent_stack: Vec<UmlId>,                       // containment tracking
}
```

---

## 2. Scope: What M9 Adds

| Feature | XMI element(s) | Status |
|---|---|---|
| Attributes | `<UML:Attribute>` inside `Classifier.feature` | ❌ Skipped |
| Operations | `<UML:Operation>` inside `Classifier.feature` | ❌ Skipped |
| Parameters | `<UML:Parameter>` inside `BehavioralFeature.parameter` | ❌ Skipped |
| Generalization (child ref) | `<UML:GeneralizableElement.generalization>` → `<UML:Generalization xmi.idref="...">` | ❌ Ignored |
| Generalization (standalone) | `<UML:Generalization child="..." parent="..." xmi.id="...">` | ❌ Ignored |
| Association | `<UML:Association>` → `Association.connection` → `AssociationEnd` | ❌ Ignored |
| Dependency | `<UML:Dependency supplier="..." client="...">` | ❌ Ignored |
| Abstraction | `<UML:Abstraction supplier="..." client="...">` | ❌ Ignored |
| Type reference resolution | `type="..."` on Attribute, Parameter, Operation return | ❌ Unhandled |
| Real corpus tests | All `test/*.xmi` files | ❌ Not implemented |

---

## 3. M9 Phase 1 — Classifier Feature Parsing

### 3.1 Problem: `Classifier.feature` is a wrapper

In XMI, classifier features appear inside a wrapper:

```xml
<UML:Class xmi.id="abc" name="Car">
  <UML:Classifier.feature>
    <UML:Attribute visibility="private" xmi.id="abc1" type="xyz" name="m_type"/>
    <UML:Operation visibility="public" xmi.id="abc2" name="startEngine">
      <UML:BehavioralFeature.parameter>
        <UML:Parameter kind="return" xmi.id="abc3" type="xyz"/>
      </UML:BehavioralFeature.parameter>
    </UML:Operation>
  </UML:Classifier.feature>
</UML:Class>
```

The `Classifier.feature` element is **not** a model element — it is a syntactic
wrapper whose children belong to the enclosing classifier. We need to track
which classifier (by `UmlId`) is currently open when we enter
`Classifier.feature`.

### 3.2 Solution

**Add to `XmiReader`:**

```rust
/// Track the current classifier for feature child elements.
/// Set when entering `<UML:Classifier.feature>`, cleared when exiting.
current_classifier: Option<UmlId>,

/// Pending type references for Pass 2 resolution.
pending_type_refs: Vec<PendingTypeRef>,

/// Pending relationships for Pass 2 resolution.
pending_relationships: Vec<PendingRelationship>,

/// Pending generalizations for Pass 2 resolution.
pending_generalizations: Vec<PendingGeneralization>,
```

**Data structures for pending references:**

```rust
/// A type reference that needs resolution in Pass 2.
struct PendingTypeRef {
    /// The UmlId of the element that contains the reference.
    element_id: UmlId,
    /// The XMI ID string of the target type.
    xmi_type_id: String,
    /// Which kind of reference this is.
    kind: TypeRefKind,
}

enum TypeRefKind {
    /// An attribute's type: attribute_index in classifier.attributes.
    AttributeType { classifier_id: UmlId, attr_index: usize },
    /// An operation's return type.
    OperationReturnType { classifier_id: UmlId, op_index: usize },
    /// A parameter's type within an operation.
    ParameterType { classifier_id: UmlId, op_index: usize, param_index: usize },
}

/// A pending relationship to resolve in Pass 2.
struct PendingRelationship {
    kind: AssociationType,
    source_xmi_id: String,
    target_xmi_id: String,
}

/// A pending generalization to resolve in Pass 2.
struct PendingGeneralization {
    subclass_id: UmlId,
    superclass_xmi_id: String,
}
```

### 3.3 Event handler changes

**In `Event::Start` (and `Event::Empty`), add cases for:**

```rust
// Track which classifier is current
"Classifier.feature" => {
    // The last pushed parent should be a classifier (Class, Interface, Enum, etc.)
    // If parent_stack is non-empty, set current_classifier.
    self.current_classifier = self.parent_stack.last().copied();
}

// Parse attributes
"Attribute" => {
    if let Some(classifier_id) = self.current_classifier {
        let xmi_id = Self::require_attr(e, "xmi.id", "Attribute")?;
        let name = Self::attr_value(e, "name").unwrap_or_default();
        let vis = Self::parse_visibility(
            &Self::attr_value(e, "visibility").unwrap_or_else(|| "public".to_string())
        );
        let type_xmi = Self::attr_value(e, "type");
        let init_val = Self::attr_value(e, "initialValue");

        let attr = Attribute {
            name,
            type_ref: TypeReference::unspecified(),
            visibility: vis,
            initial_value: init_val,
            is_static: false,
        };

        // Store in the model — need mutable access to the classifier
        let model_ptr: *mut UmlModel = &mut *model;
        // SAFETY: we are not borrowing model elsewhere in this branch
        // (the model reference is only used here)
        if let Some(elem) = unsafe { &mut *model_ptr }.get_mut(classifier_id) {
            if let Some(cd) = elem.classifier_data_mut() {
                let idx = cd.attributes.len();
                cd.attributes.push(attr);
                // Defer type reference resolution
                if let Some(tid) = type_xmi {
                    self.pending_type_refs.push(PendingTypeRef {
                        element_id: classifier_id,
                        xmi_type_id: tid,
                        kind: TypeRefKind::AttributeType { classifier_id, attr_index: idx },
                    });
                }
            }
        }
    }
}

// Parse operations (with nested parameters from Empty events)
"Operation" => {
    if let Some(classifier_id) = self.current_classifier {
        let xmi_id = Self::require_attr(e, "xmi.id", "Operation")?;
        let name = Self::attr_value(e, "name").unwrap_or_default();
        let vis = Self::parse_visibility(
            &Self::attr_value(e, "visibility").unwrap_or_else(|| "public".to_string())
        );
        let is_abstract = Self::attr_value(e, "isAbstract").is_some_and(|v| v == "true");
        let is_query = Self::attr_value(e, "isQuery").is_some_and(|v| v == "true");

        // The return type parameter and any in/out parameters will be parsed
        // as children. Create a placeholder operation first.
        let op = Operation {
            name,
            return_type: TypeReference::unspecified(),
            parameters: Vec::new(),
            visibility: vis,
            is_static: false,
            is_abstract,
            is_virtual: false,
        };

        // Store placeholder in model and set up tracking
        // We'll use the current_classifier + a counter to track which
        // operation we're populating via nested Parameter parsing.
    }
}
```

### 3.4 Parameter parsing

Parameters appear both as `Event::Empty` (self-closing) and potentially as
`Event::Start`+`Event::End` pairs. The most common form:

```xml
<UML:BehavioralFeature.parameter>
  <UML:Parameter kind="return" xmi.id="abc3" type="xyz"/>
</UML:BehavioralFeature.parameter>
```

We need to track when we're inside an operation to know where to push
parameters. Add:

```rust
/// Stack of operation IDs being populated.
pending_operation_stack: Vec<(UmlId, usize)>,  // (classifier_id, op_index)
```

```rust
"Operation" => {
    // ... parse as above, then push to stack
    self.pending_operation_stack.push((classifier_id, op_index));
}

"BehavioralFeature.parameter" => {
    // Wrapper — parameters inside are children of current operation
}

"Parameter" => {
    if let Some(&(classifier_id, op_index)) = self.pending_operation_stack.last() {
        let kind = Self::attr_value(e, "kind").unwrap_or_default();
        let type_xmi = Self::attr_value(e, "type");
        let param_name = Self::attr_value(e, "name").unwrap_or_default();

        let direction = match kind.as_str() {
            "return" => ParameterDirection::Return,
            "in" => ParameterDirection::In,
            "out" => ParameterDirection::Out,
            "inout" => ParameterDirection::InOut,
            _ => ParameterDirection::In,
        };

        let param = Parameter {
            name: param_name,
            type_ref: TypeReference::unspecified(),
            direction,
            default_value: None,
        };

        // Push to the model
        if let Some(elem) = model.get_mut(classifier_id) {
            if let Some(cd) = elem.classifier_data_mut() {
                if op_index < cd.operations.len() {
                    let param_idx = cd.operations[op_index].parameters.len();
                    cd.operations[op_index].parameters.push(param);

                    // Defer type reference for non-return params too
                    if let Some(tid) = type_xmi {
                        self.pending_type_refs.push(PendingTypeRef {
                            element_id: classifier_id,
                            xmi_type_id: tid,
                            kind: TypeRefKind::ParameterType {
                                classifier_id,
                                op_index,
                                param_index: param_idx,
                            },
                        });
                    }

                    // If this is a return parameter, set the return type
                    if direction == ParameterDirection::Return {
                        cd.operations[op_index].return_type = if let Some(tid) = &type_xmi {
                            // Will be resolved in Pass 2
                            TypeReference::unspecified()
                        } else {
                            TypeReference::unspecified()
                        };
                        // Defer return type reference
                        if let Some(tid) = type_xmi {
                            self.pending_type_refs.push(PendingTypeRef {
                                element_id: classifier_id,
                                xmi_type_id: tid,
                                kind: TypeRefKind::OperationReturnType {
                                    classifier_id,
                                    op_index,
                                },
                            });
                        }
                    }
                }
            }
        }
    }
}
```

### 3.5 End-event tracking

```rust
Event::End(ref e) => {
    let local_name = Self::local_name(tag);
    match local_name {
        "Classifier.feature" => {
            self.current_classifier = None;
        },
        "Operation" => {
            self.pending_operation_stack.pop();
        },
        // ... existing Model/Package/XMI.extensions handling
        _ => {},
    }
}
```

---

## 4. M9 Phase 2 — Relationship Parsing

### 4.1 XMI formats encountered in corpus files

From the 10 test `.xmi` files, three relationship patterns appear:

#### Pattern A: Generalization (child-ref style)
The subclass element contains a reference to the superclass:

```xml
<UML:Class xmi.id="child1" name="Sequence Diagram">
  <UML:GeneralizableElement.generalization>
    <UML:Generalization xmi.idref="parent1"/>
  </UML:GeneralizableElement.generalization>
</UML:Class>
```

#### Pattern B: Generalization (standalone element)

```xml
<UML:Generalization discriminator="" visibility="public"
    child="child1" xmi.id="g1" parent="parent1" name=""/>
```

Both patterns carry the same semantics: `child` inherits from `parent`.

#### Pattern C: Association

```xml
<UML:Association visibility="public" xmi.id="a1" name="">
  <UML:Association.connection>
    <UML:AssociationEnd type="class1" aggregation="aggregate" isNavigable="true" .../>
    <UML:AssociationEnd type="class2" aggregation="none" isNavigable="true" .../>
  </UML:Association.connection>
</UML:Association>
```

#### Pattern D: Dependency / Abstraction

```xml
<UML:Dependency supplier="supplier_id" xmi.id="d1" client="client_id" name=""/>
<UML:Abstraction supplier="supplier_id" xmi.id="d2" client="client_id" name=""/>
```

`Abstraction` is a subtype of `Dependency` in UML. Both have `client` (source)
and `supplier` (target). We parse both as `AssociationType::Dependency`.

### 4.2 Parsing standalone `<UML:Generalization>`

```rust
"Generalization" => {
    let child_xmi = Self::require_attr(e, "child", "Generalization")?;
    let parent_xmi = Self::require_attr(e, "parent", "Generalization")?;

    // Register its own ID so it can be referenced
    let _ = self.build_base(e, "Generalization")?;

    // Check if both sides are already known
    if let (Some(&child_id), Some(&parent_id)) =
        (self.id_map.get(&child_xmi), self.id_map.get(&parent_xmi))
    {
        let rel = Relationship::new_generalization(child_id, parent_id);
        model.insert(ModelElement::Relationship(rel));
    } else {
        // Defer to Pass 2
        self.pending_relationships.push(PendingRelationship {
            kind: AssociationType::Generalization,
            source_xmi_id: child_xmi,
            target_xmi_id: parent_xmi,
        });
    }
}
```

### 4.3 Parsing child-ref generalization (`GeneralizableElement.generalization`)

When inside a classifier (which we track via `parent_stack`), encountering:

```xml
<UML:GeneralizableElement.generalization>
  <UML:Generalization xmi.idref="parent_id"/>
</UML:GeneralizableElement.generalization>
```

We need to handle this in two steps:

```rust
// Track that parent_stack.last() is the subclass
// and xmi.idref is the superclass ID
"GeneralizableElement.generalization" => {
    // Set a flag to track we're inside this wrapper
    self.inside_generalization_wrapper = true;
}

// Inside Empty/Start events:
"Generalization" => {
    if self.inside_generalization_wrapper {
        if let Some(&subclass_id) = self.parent_stack.last() {
            let super_xmi = Self::require_attr(e, "xmi.idref", "Generalization")?;
            if let Some(&super_id) = self.id_map.get(&super_xmi) {
                let rel = Relationship::new_generalization(subclass_id, super_id);
                model.insert(ModelElement::Relationship(rel));
            } else {
                self.pending_generalizations.push(PendingGeneralization {
                    subclass_id,
                    superclass_xmi_id: super_xmi,
                });
            }
        }
    }
}
```

**Simpler alternative:** parse just the `xmi.idref` and defer everything to
Pass 2, which already does generalization resolution:

```rust
"Generalization" => {
    let xmi_idref = Self::attr_value(e, "xmi.idref");
    let child = Self::attr_value(e, "child");

    if let Some(ref_xmi) = xmi_idref {
        // Child-ref style: current classifier is subclass
        if let Some(&subclass_id) = self.parent_stack.last() {
            self.pending_generalizations.push(PendingGeneralization {
                subclass_id,
                superclass_xmi_id: ref_xmi,
            });
        }
    } else if let (Some(child_xmi), Some(parent_xmi)) = (child, Self::attr_value(e, "parent")) {
        // Standalone style
        self.pending_relationships.push(PendingRelationship {
            kind: AssociationType::Generalization,
            source_xmi_id: child_xmi,
            target_xmi_id: parent_xmi,
        });
    }
}
```

### 4.4 Parsing `<UML:Association>`

```rust
"Association" => {
    let _base = self.build_base(e, "Association")?;
    self.expecting_association_end = true;
    self.association_ends = Vec::new();
}

"AssociationEnd" => {
    if self.expecting_association_end {
        let type_xmi = Self::require_attr(e, "type", "AssociationEnd")?;
        let agg = Self::attr_value(e, "aggregation").unwrap_or_default();
        let is_nav = Self::attr_value(e, "isNavigable").is_some_and(|v| v == "true");

        self.association_ends.push(AssociationEndData {
            type_xmi_id: type_xmi,
            aggregation: agg,
            is_navigable: is_nav,
        });
    }
}
```

On exiting `Association.connection` (or `Association`), create the relationship:

```rust
"Association" => {
    if self.association_ends.len() == 2 {
        let end1 = &self.association_ends[0];
        let end2 = &self.association_ends[1];

        // Determine relationship kind from aggregation
        let kind = match end2.aggregation.as_str() {
            "shared" | "aggregate" => AssociationType::Aggregation,
            "composite" => AssociationType::Composition,
            _ => AssociationType::Association,
        };

        // Use end1 as source, end2 as target
        if let (Some(&source_id), Some(&target_id)) =
            (self.id_map.get(&end1.type_xmi_id), self.id_map.get(&end2.type_xmi_id))
        {
            let mut rel = Relationship::new(kind, source_id, target_id);
            rel.source_to_target_navigable = end1.is_navigable;
            rel.target_to_source_navigable = end2.is_navigable;
            model.insert(ModelElement::Relationship(rel));
        } else {
            self.pending_relationships.push(PendingRelationship {
                kind,
                source_xmi_id: end1.type_xmi_id.clone(),
                target_xmi_id: end2.type_xmi_id.clone(),
            });
        }
    }
    self.expecting_association_end = false;
    self.association_ends.clear();
}
```

**Helpers for association end tracking:**

```rust
#[derive(Default)]
struct AssociationEndData {
    type_xmi_id: String,
    aggregation: String,
    is_navigable: bool,
}

// Add to XmiReader:
expecting_association_end: bool,
association_ends: Vec<AssociationEndData>,
```

### 4.5 Parsing `<UML:Dependency>` and `<UML:Abstraction>`

```rust
"Dependency" | "Abstraction" => {
    let client = Self::require_attr(e, "client", "Dependency")?;
    let supplier = Self::require_attr(e, "supplier", "Dependency")?;

    if let (Some(&client_id), Some(&supplier_id)) =
        (self.id_map.get(&client), self.id_map.get(&supplier))
    {
        let rel = Relationship::new_dependency(client_id, supplier_id);
        model.insert(ModelElement::Relationship(rel));
    } else {
        self.pending_relationships.push(PendingRelationship {
            kind: AssociationType::Dependency,
            source_xmi_id: client,
            target_xmi_id: supplier,
        });
    }
}
```

---

## 5. M9 Phase 3 — Expanded Pass 2 Cross-Reference Resolution

The existing `resolve()` method only handles stereotype references. We expand
it to handle type references, relationships, and generalizations.

### 5.1 New resolve() implementation

```rust
pub fn resolve(&mut self, model: &mut UmlModel) -> Result<(), XmiParseError> {
    // Step 1: Resolve stereotypes (existing behavior)
    self.resolve_stereotypes(model);

    // Step 2: Resolve type references
    self.resolve_type_refs(model);

    // Step 3: Resolve pending relationships
    self.resolve_relationships(model);

    // Step 4: Resolve pending generalizations
    self.resolve_generalizations(model);

    Ok(())
}
```

### 5.2 Type reference resolution

```rust
fn resolve_type_refs(&mut self, model: &mut UmlModel) {
    let refs = std::mem::take(&mut self.pending_type_refs);
    for pending in refs {
        // Find the target UmlId
        let target_id = self.id_map.get(&pending.xmi_type_id).copied();

        // Build the resolved TypeReference
        let type_ref = if let Some(tid) = target_id {
            TypeReference::model(tid)
        } else {
            // Fallback: treat unresolved type IDs as primitive names
            // This ensures we never panic on missing types
            TypeReference::primitive(&pending.xmi_type_id)
        };

        // Apply to the appropriate field
        if let Some(elem) = model.get_mut(pending.element_id) {
            if let Some(cd) = elem.classifier_data_mut() {
                match pending.kind {
                    TypeRefKind::AttributeType { classifier_id: _, attr_index } => {
                        if attr_index < cd.attributes.len() {
                            cd.attributes[attr_index].type_ref = type_ref;
                        }
                    },
                    TypeRefKind::OperationReturnType { classifier_id: _, op_index } => {
                        if op_index < cd.operations.len() {
                            cd.operations[op_index].return_type = type_ref;
                        }
                    },
                    TypeRefKind::ParameterType { classifier_id: _, op_index, param_index } => {
                        if op_index < cd.operations.len()
                            && param_index < cd.operations[op_index].parameters.len()
                        {
                            cd.operations[op_index].parameters[param_index].type_ref = type_ref;
                        }
                    },
                }
            }
        }
    }
}
```

### 5.3 Relationship resolution

```rust
fn resolve_relationships(&mut self, model: &mut UmlModel) {
    let rels = std::mem::take(&mut self.pending_relationships);
    for pending in rels {
        let source_id = self.id_map.get(&pending.source_xmi_id).copied();
        let target_id = self.id_map.get(&pending.target_xmi_id).copied();

        match (source_id, target_id) {
            (Some(src), Some(tgt)) => {
                let rel = Relationship::new(pending.kind, src, tgt);
                model.insert(ModelElement::Relationship(rel));
            },
            _ => {
                // Log warning but don't fail — lenient parsing
                log::warn!(
                    "Skipping relationship: source={:?}, target={:?}",
                    pending.source_xmi_id,
                    pending.target_xmi_id,
                );
            },
        }
    }
}
```

### 5.4 Generalization resolution

```rust
fn resolve_generalizations(&mut self, model: &mut UmlModel) {
    let gens = std::mem::take(&mut self.pending_generalizations);
    for pending in gens {
        if let Some(&super_id) = self.id_map.get(&pending.superclass_xmi_id) {
            let rel = Relationship::new_generalization(pending.subclass_id, super_id);
            model.insert(ModelElement::Relationship(rel));
        } else {
            log::warn!(
                "Skipping generalization: superclass {} not found",
                pending.superclass_xmi_id,
            );
        }
    }
}
```

---

## 6. M9 Phase 4 — Real Corpus Integration Test

### 6.1 Test file

Create `crates/uml-io/tests/test_real_corpus.rs`:

```rust
//! Integration tests against real Umbrello C++ XMI files.
//!
//! These tests parse every `.xmi` file in the project's test directory
//! and verify that:
//! - All elements known to the reader are successfully parsed.
//! - Cross-references (stereotype, type, relationship) are resolved.
//! - Attributes and operations are attached to their classifiers.
//! - The model is structurally valid (no dangling references, etc.).
//!
//! NOTE: Path is relative to the workspace root. Adjust as needed.

use std::io::BufReader;
use std::path::Path;

use uml_core::UmlModel;
use uml_io::xmi::XmiReader;

/// Path to the C++ test XMI files, relative to workspace root.
const TEST_DIR: &str = "test";

/// Collect all `.xmi` files from the test directory.
fn find_xmi_files() -> Vec<std::path::PathBuf> {
    let dir = Path::new(TEST_DIR);
    if !dir.exists() {
        panic!("Test directory not found: {TEST_DIR}. Run from workspace root.");
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).expect("Failed to read test directory") {
        let path = entry.expect("Failed to read dir entry").path();
        if path.extension().map_or(false, |e| e == "xmi") {
            files.push(path);
        }
    }
    files.sort(); // deterministic order
    files
}

/// Parse a single XMI file and return the populated model.
fn parse_xmi(path: &Path) -> (UmlModel, XmiReader) {
    let file = std::fs::File::open(path)
        .unwrap_or_else(|e| panic!("Failed to open {}: {e}", path.display()));

    let mut model = UmlModel::new();
    let mut reader = XmiReader::new();

    let count = reader
        .read_from(BufReader::new(file), &mut model)
        .unwrap_or_else(|e| panic!("Failed to parse {}: {e}", path.display()));

    assert!(count > 0, "Should parse at least one element from {}", path.display());

    reader
        .resolve(&mut model)
        .unwrap_or_else(|e| panic!("Failed to resolve {}: {e}", path.display()));

    (model, reader)
}

// ─── Tests ────────────────────────────────────────────────────────────

#[test]
fn parse_all_cpp_test_files() {
    let files = find_xmi_files();
    eprintln!("Found {} XMI files to test", files.len());
    assert!(!files.is_empty(), "At least one .xmi file must exist in {TEST_DIR}");

    for path in &files {
        eprintln!("Parsing: {}", path.display());
        let (model, _reader) = parse_xmi(path);

        // Every file should have a root model and at least one element
        assert!(model.len() >= 1, "{}: expected >=1 elements", path.display());
    }
}

#[test]
fn parse_and_validate_each_file() {
    let files = find_xmi_files();

    for path in &files {
        let (model, _reader) = parse_xmi(path);

        // Validate the model structure
        let errors = model.validate();
        assert!(
            errors.is_empty(),
            "{}: validation errors: {:#?}",
            path.display(),
            errors,
        );
    }
}

#[test]
fn test_dcl_has_attributes_and_operations() {
    let path = Path::new(TEST_DIR).join("test-DCL.xmi");
    let (model, _reader) = parse_xmi(&path);

    // Find the Car class (xmi.id="izeLWQirW9jH")
    let car = model
        .iter()
        .find(|(_, e)| e.name() == "Car")
        .expect("Car class should exist in test-DCL.xmi");

    // Check Car's original_xmi_id
    assert_eq!(
        car.1.base().original_xmi_id.as_deref(),
        Some("izeLWQirW9jH"),
        "Car should have preserved original XMI ID"
    );

    // Car should have 3 attributes
    let attrs = &car.1.classifier_data().expect("Car should be a classifier").attributes;
    assert_eq!(attrs.len(), 3, "Car should have 3 attributes");

    // Check attribute details
    let attr_names: Vec<&str> = attrs.iter().map(|a| a.name.as_str()).collect();
    assert!(attr_names.contains(&"m_type"), "Car should have m_type attribute");
    assert!(attr_names.contains(&"m_owner"), "Car should have m_owner attribute");
    assert!(attr_names.contains(&"m_plate"), "Car should have m_plate attribute");

    // Car should have 3 operations
    let ops = &car.1.classifier_data().unwrap().operations;
    assert_eq!(ops.len(), 3, "Car should have 3 operations");

    let op_names: Vec<&str> = ops.iter().map(|o| o.name.as_str()).collect();
    assert!(op_names.contains(&"startEngine"), "Car should have startEngine");
    assert!(op_names.contains(&"stopEngine"), "Car should have stopEngine");
    assert!(op_names.contains(&"engineStatus"), "Car should have engineStatus");
}

#[test]
fn test_dcl_has_relationships() {
    let path = Path::new(TEST_DIR).join("test-DCL.xmi");
    let (model, _reader) = parse_xmi(&path);

    // Count relationships
    let rels: Vec<_> = model
        .iter()
        .filter(|(_, e)| matches!(e, uml_core::ModelElement::Relationship(_)))
        .collect();

    // test-DCL.xmi has:
    // - Many UML:Abstraction relationships (client-supplier)
    // - UML:Generalization elements (child-parent)
    // - UML:Association elements (with AssociationEnd)
    // At a rough count, over 20 relationships
    assert!(rels.len() >= 10, "Expected 10+ relationships in test-DCL.xmi, got {}", rels.len());
}
```

### 6.2 Running the corpus tests

```bash
# From workspace root
cargo test --package uml-io --test test_real_corpus -- --nocapture
```

The `--nocapture` flag allows the `eprintln!` diagnostic output to show which
file is being parsed.

---

## 7. Key Design Decisions

### 7.1 Unresolved types → primitive fallback

**Decision:** If a `type="..."` attribute references an XMI ID not in
`id_map`, create `TypeReference::primitive(xmi_type_id)` instead of returning
an error.

**Rationale:** XMI type references often point to elements in other files
(e.g., standard library types) or are forward references to elements parsed
later. Falling back to a primitive name ensures that parsing never fails on
missing type definitions — the type name is preserved as a string and can be
resolved later if needed.

### 7.2 AssociationEnd aggregation mapping

| XMI `aggregation` value | `AssociationType` |
|---|---|
| `"none"` or absent | `Association` |
| `"shared"` or `"aggregate"` | `Aggregation` |
| `"composite"` | `Composition` |

### 7.3 Parameter kind mapping

| XMI `kind` value | `ParameterDirection` |
|---|---|
| `"return"` | `Return` |
| `"in"` | `In` |
| `"out"` | `Out` |
| `"inout"` | `InOut` |
| absent | `In` |

### 7.4 Classifier.feature is a wrapper

`<UML:Classifier.feature>` is a syntactic wrapper — **not** a model element.
Its children (Attribute, Operation) belong to the outer classifier element.
Track `current_classifier` via `parent_stack.last()` when entering the
wrapper.

### 7.5 Generalization: two formats, same semantics

| Format | Where found | How to parse |
|---|---|---|
| Child-ref: `Generalization[xmi.idref]` inside `GeneralizableElement.generalization` | Inside classifier elements | Subclass = current `parent_stack.last()`, superclass = `xmi.idref` |
| Standalone: `Generalization[child, parent]` | As a peer element | Both IDs are explicit attributes |

### 7.6 Abstraction → Dependency

`<UML:Abstraction>` is a UML dependency subtype with the same
`client`/`supplier` attributes. We parse it identically to
`<UML:Dependency>`.

### 7.7 Lenient by default

All unrecognized elements are silently skipped. Missing type references become
primitive strings. Unresolvable relationships are logged as warnings but do
not cause parse failure. This matches the C++ Umbrello's own lenient XMI
loading behavior.

---

## 8. Implementation Order

| Step | Description | Dependencies |
|---|---|---|
| 1 | Add pending type ref / relationship / generalization data structures to `XmiReader` | None (M8 baseline) |
| 2 | Remove `skip_element` for `Classifier.feature`; add `current_classifier` tracking | Step 1 |
| 3 | Parse `<UML:Attribute>` inside Classifier.feature | Step 2 |
| 4 | Parse `<UML:Operation>` with nested `<UML:BehavioralFeature.parameter>` / `<UML:Parameter>` | Step 2 |
| 5 | Implement `resolve_type_refs()` in Pass 2 | Steps 3–4 |
| 6 | Parse standalone `<UML:Generalization>` (child/parent attrs) | Step 1 |
| 7 | Parse child-ref `<UML:Generalization>` (xmi.idref inside GeneralizableElement.generalization) | Step 1 |
| 8 | Parse `<UML:Association>` with AssociationEnd | Step 1 |
| 9 | Parse `<UML:Dependency>` and `<UML:Abstraction>` | Step 1 |
| 10 | Implement `resolve_relationships()` and `resolve_generalizations()` in Pass 2 | Steps 6–9 |
| 11 | Create unit tests for each new parser element | Steps 2–10 |
| 12 | Create `test_real_corpus.rs` integration test | Steps 5, 10 |
| 13 | Run all 10 test XMI files and fix edge cases | Step 12 |

### 8.1 Recommended merge strategy

Merge after **Step 5** (feature parsing + type ref resolution is a coherent
unit that unblocks attribute/operation support). Then merge again after
**Step 10** (full relationship support). The corpus test (Step 12) validates
both.

---

## 9. Testing Plan

### 9.1 Unit tests (in `reader.rs`)

| Test | What it covers | XMI snippet |
|---|---|---|
| `parse_attribute` | Single attribute on a class | `<UML:Attribute name="count" type="t1" visibility="private"/>` |
| `parse_operation` | Operation with return type parameter | `<UML:Operation name="getX"><UML:BehavioralFeature.parameter><UML:Parameter kind="return" type="t1"/></UML:BehavioralFeature.parameter></UML:Operation>` |
| `parse_operation_no_return` | Operation without return parameter | `<UML:Operation name="doSomething"/>` |
| `parse_operation_with_params` | Operation with in/out parameters | `<UML:Operation name="setXY"><UML:BehavioralFeature.parameter><UML:Parameter kind="in" name="x" type="t1"/><UML:Parameter kind="in" name="y" type="t2"/><UML:Parameter kind="return" type="void"/></UML:BehavioralFeature.parameter></UML:Operation>` |
| `parse_generalization_standalone` | Standalone Generalization element | `<UML:Generalization child="c1" parent="p1" xmi.id="g1"/>` |
| `parse_generalization_childref` | Child-ref Generalization | `<UML:GeneralizableElement.generalization><UML:Generalization xmi.idref="p1"/></UML:GeneralizableElement.generalization>` |
| `parse_association_aggregation` | Association with aggregation | Association with `aggregation="aggregate"` on one end |
| `parse_association_composition` | Association with composition | Association with `aggregation="composite"` on one end |
| `parse_dependency` | Dependency element | `<UML:Dependency supplier="sup" client="cli" xmi.id="d1"/>` |
| `parse_abstraction` | Abstraction element | `<UML:Abstraction supplier="sup" client="cli" xmi.id="a1"/>` |
| `resolve_type_ref_to_model` | Type reference resolved to model element | Attribute with `type` matching a known DataType |
| `resolve_type_ref_fallback_primitive` | Unresolved type becomes primitive | Attribute with `type="unknown_id"` not in id_map |
| `resolve_relationship_deferred` | Relationship resolved in Pass 2 | Dependency where supplier parsed after dependency |

### 9.2 Integration test (in `tests/test_real_corpus.rs`)

| Test | What it validates |
|---|---|
| `parse_all_cpp_test_files` | All 10 test XMI files parse without errors and produce ≥1 element each |
| `parse_and_validate_each_file` | Each parsed model passes `UmlModel::validate()` (no dangling references) |
| `test_dcl_has_attributes_and_operations` | Car class in test-DCL.xmi has 3 attributes + 3 operations |
| `test_dcl_has_relationships` | test-DCL.xmi contains 10+ relationships (generalizations, associations, abstractions) |

### 9.3 Running tests

```bash
# All tests (from workspace root)
cargo test --package uml-io

# Unit tests only
cargo test --package uml-io --lib

# Integration tests only
cargo test --package uml-io --test test_real_corpus

# With logging
RUST_LOG=warn cargo test --package uml-io --test test_real_corpus -- --nocapture
```
