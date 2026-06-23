# Milestone 8 Architectural Review — XMI Persistence

> **Document:** `rust-rewrite/docs/milestone8_review.md`  
> **Review date:** 2026-06-23  
> **Reviewer:** Umbrello-RS Reviewer  
> **Proposal under review:** XMI persistence architecture for Milestone 8  
> **Proposal document:** `xmi_persistence_architecture_v1.md` (not yet written; proposal described in review brief)  
> **Codebase verified against:** `rust-rewrite/` as of 2026-06-23; XMI reader/writer are stubs (59 lines total); `UmlModel` repository is fully implemented (1176 lines, 42 tests)

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Parser Choice: quick-xml vs Alternatives](#2-parser-choice-quick-xml-vs-alternatives)
3. [Two-Pass Strategy](#3-two-pass-strategy)
4. [ID Mapping](#4-id-mapping)
5. [Scope Boundaries: M8 vs M9](#5-scope-boundaries-m8-vs-m9)
6. [Error Handling](#6-error-handling)
7. [Integration with UmlModel](#7-integration-with-umlmodel)
8. [Test Coverage](#8-test-coverage)
9. [Round-Trip Compatibility](#9-round-trip-compatibility)
10. [Architectural Gaps Identified](#10-architectural-gaps-identified)
11. [Summary of Required Conditions](#11-summary-of-required-conditions)
12. [Final Recommendation](#12-final-recommendation)

---

## 1. Executive Summary

This review evaluates the proposed two-pass XMI parser design against:

- The **current Rust codebase** (fully-implemented `UmlModel` repository, `IndexMap<UmlId, ModelElement>` storage, `ElementBase` metadata struct, five `ModelElement` variants)
- The **XMI format** as actually produced by C++ Umbrello (verified against 10 test `.xmi` files in `test/` and the `models/` directory)
- The **testing strategy** defined in `docs/testing_strategy.md` (XMI round-trip is the critical invariant for the project)
- The **workspace dependencies** that already include `quick-xml 0.37`

**Overall finding:** The proposed architecture is sound in its core design but has one critical flaw — it does not address round-trip XMI compatibility. The proposal uses `UmlId::new()` (UUID v4) for all XMI elements, which destroys the original `xmi.id` strings and makes round-trip testing impossible. This must be resolved before implementation begins.

Additionally, the proposed M8/M9 scope split is suboptimal: `DataType` is trivial (13 elements in every XMI file) and should be included in M8; `Stereotype` resolution must be at least partially handled in M8 because stereotype references appear on structural elements and are semantically meaningful for distinguishing classifiers.

**Verdict:** APPROVE WITH CONDITIONS — 8 mandatory conditions, 4 recommendations.

---

## 2. Parser Choice: quick-xml vs Alternatives

### 2.1 Analysis

The proposal selects `quick-xml` (event-based SAX-style parser). This is already a workspace dependency (`quick-xml = "0.37"` in workspace `Cargo.toml`).

**Alternatives considered (implicitly):**

| Parser | Style | Pros | Cons |
|--------|-------|------|------|
| quick-xml | SAX / event-based | Zero-alloc streaming, already in deps, fast | Manual state machine, verbose dispatch |
| serde-xml-rs / serde + quick-xml | DOM / serde derive | Type-safe, minimal code | Requires full document in memory, brittle to XMI schema variations |
| roxmltree | DOM tree | Small API, well-tested | Tree allocation for entire document, no streaming |
| xml-rs | SAX | Older, stable | Slower, minimal community |

**Verdict on quick-xml: CORRECT choice**, for the following reasons:

1. **Streaming is essential for large models.** Umbrello models can contain hundreds of diagram widgets with geometry data. A DOM-based approach would require allocating the entire tree in memory. The proposal does not mention this, but it is the correct justification.

2. **Event-based API matches the XMI structure.** XMI 1.2 uses a nested containment pattern (`XMI.content → UML:Model → UML:Namespace.ownedElement → UML:Package/Class/etc`). An event-based parser can track depth via a stack and dispatch on `StartElement` events without ever materializing the intermediate DOM nodes.

3. **Already in workspace dependencies.** No new dependency overhead.

4. **No schema coupling.** The XMI files have irregular attribute sets (some elements have `comment`, `stereotype`, `discriminator`, etc.; others don't). A serde-derive approach would require `#[serde(flatten)]` hacks or manual `Deserialize` implementations — both fragile. Raw event dispatch gives full control.

### 2.2 Edge Cases

**Edge cases quick-xml handles well:**
- Malformed XML (reports errors before any processing)
- Large files (streaming, no memory proportional to file size)
- Unicode names and documentation text (UTF-8 by default)
- Mixed content in documentation/comments

**Edge case: XMI 2.1 namespace format.** XMI 2.1 uses `xmi:id` (colon instead of dot), `uml:` prefix (lowercase), and `<packagedElement>` instead of `<UML:Namespace.ownedElement>`. The proposal mentions both versions in `XmiVersion` enum (`mod.rs` line 12-16). The parser must handle both. `quick-xml`'s namespace-aware API (`Reader::with_ns`) can handle the `UML:` prefix via a namespace buffer, or the parser can match on the local name alone. **Recommendation:** match on local name only (strip prefixes) for maximum compatibility — XMI files may use different prefix conventions.

**Edge case: `<XMI.extension>` blocks.** These contain diagram widgets, listview settings, and codegeneration data. They are large but not part of the structural model. The proposal's two-pass strategy must skip/ignore these blocks during Pass 1. The parser must correctly track nesting depth to avoid misinterpreting `<XMI.extension>` children as structural elements.

### 2.3 Event-Based API and Two-Pass Strategy

**Question:** Does the event-based API make the two-pass strategy harder or easier?

**Answer: Easier.** The event-based API naturally supports multi-pass parsing because:

1. **Idempotent replay.** The parser can be re-instantiated for Pass 2 with zero state from Pass 1 except the ID map.

2. **Selective dispatching.** Pass 1 only needs to handle `StartElement` events for structural type tags. Pass 2 can handle `StartElement` for the same tags but with full resolution context. The event-based API avoids materializing intermediate XML data that would need to be discarded between passes.

3. **Skip-optimized.** `quick-xml` `Reader` has `read_to_end(...)` for skipping entire subtrees, allowing the parser to jump over `<XMI.extension>` blocks and diagram sections without processing their content. This is more efficient than parsing into intermediate DOM nodes and then discarding them.

**One caution:** The two-pass strategy reads the file twice. For rare very large files (the test files are < 1 MB; the UmbrelloArchitecture model is ~100 KB), this is negligible. If future models exceed 10+ MB and performance becomes a concern, consider a single-pass approach with deferred resolution (collect references in a `Vec<DeferredRef>` and resolve after all elements are parsed). But for M8, two-pass is simpler and correct.

---

## 3. Two-Pass Strategy

### 3.1 Pass Separation Analysis

The proposal specifies:

- **Pass 1:** Extract structural elements (Package, Class, Interface, Enum, Datatype), build ID map
- **Pass 2:** Resolve cross-references (stereotype references, type references, association endpoints, generalization child/parent)

**Is the separation correct? YES, with one reservation.**

The separation is correct for the XMI 1.2 format because:

1. **Elements are definition-before-use in XMI files.** The C++ Umbrello serializer writes stereotypes first, then structural elements, then relationships. Pass 1 can collect all element definitions before Pass 2 resolves references.

2. **No circular dependency between element types and their references.** A Class does not need its stereotype resolved to be structurally defined (stereotype is metadata). A Package does not need its namespace resolved to be structurally defined (namespace is a reference to a parent that was already defined earlier in the file).

**One reservation: Inline stereotype resolution.** Stereotypes are defined as independent `<UML:Stereotype>` elements (lines 14-17 of `test-COG.xmi`), but they are referenced by name on other elements (`stereotype="folder"`, `stereotype="datatype"`). Pass 1 can extract stereotype definitions and map name→xmi.id. Pass 2 can resolve `stereotype="datatype"` → stereotype_id by looking up "datatype" in the stereotype name map. This works as long as stereotypes are defined before they are referenced — which holds for all Umbrello XMI files examined.

### 3.2 Containment Tracking

**Question:** Is the `parent_stack` approach for containment tracking correct?

**Answer: YES, with one caveat.**

The XMI format uses explicit nesting:

```xml
<UML:Model xmi.id="Logical View" name="Logical View">
  <UML:Namespace.ownedElement>
    <UML:Class xmi.id="sGBeu79qqOiF" name="ClassA">
      ...
    </UML:Class>
  </UML:Namespace.ownedElement>
</UML:Model>
```

A stack-based approach: push on `UML:Model`/`UML:Package` (containers), pop on end-element, and when encountering a non-container element, record the top of stack as the parent.

**Caveat: `namespace` attribute vs. nesting.** Some XMI elements have both a `namespace` attribute (textual reference) and are physically nested. The C++ parser uses the `namespace` attribute for lookups. The proposal's stack-based approach must handle these consistently:

- If an element has `namespace="Logical View"` as an attribute, AND it is nested inside `<UML:Model xmi.id="Logical View">`, the stack correctly identifies the parent.
- If an element's `namespace` attribute references a different parent than the nesting stack top, this is a malformed file and should produce an error.

**Recommendation:** After parsing, validate that each element's `namespace` attribute (if present) matches the stack-derived parent. If it doesn't, emit a warning but use the stack (the physical nesting is more reliable than a textual attribute that may be stale).

### 3.3 Deferred Stereotype Resolution

**Question:** Does the `pending_stereotypes` approach work for all XMI patterns?

**Answer: YES, for the XMI 1.2 format used by C++ Umbrello.**

The pattern in all 10 test files is:
```xml
<UML:Stereotype xmi.id="folder" name="folder"/>
<UML:Stereotype xmi.id="datatype" name="datatype"/>
<UML:Stereotype xmi.id="enum" name="enum"/>
<UML:Stereotype xmi.id="interface" name="interface"/>
<UML:Class stereotype="folder" ...>
```

The `stereotype` attribute on elements is the *name* of a stereotype defined elsewhere. Pass 1 should collect `(name → xmi.id)` mappings for all `<UML:Stereotype>` elements. Pass 2 can then resolve `stereotype="folder"` by looking up "folder" in the map → `xmi.id="folder"`, then looking up `"folder"` in the ID map → `UmlId`.

**One caution:** Not all elements have stereotype attributes. The absence of a stereotype attribute means no stereotype — not an error.

---

## 4. ID Mapping

### 4.1 String → UmlId Mapping Correctness

The proposal: `HashMap<String, UmlId>` to map XMI `xmi.id` strings to generated `UmlId` values.

**Technically correct for single-pass model construction.** When a new XMI element is encountered, the parser:
1. Reads the `xmi.id` attribute string (e.g., `"sGBeu79qqOiF"`)
2. Generates a new `UmlId` via `UmlId::new()` — producing a UUID v4
3. Inserts `("sGBeu79qqOiF" → generated_uuid)` into the map

This works for loading an XMI file once and building an in-memory model. All cross-references (stereotype `type` attributes, association endpoints' `type`, generalization `child`/`parent`) are resolved by looking up the XMI string in the map and using the corresponding `UmlId`.

### 4.2 Duplicate xmi.id Handling (Malformed Input)

**Question:** What happens when two XMI elements have the same `xmi.id`?

This is invalid XMI but must be handled defensively. The proposal should specify:

```rust
// Pseudocode for defensive insert
fn insert_id_mapping(&mut self, xmi_id: String, uml_id: UmlId) -> Result<(), XmiParseError> {
    if self.id_map.contains_key(&xmi_id) {
        return Err(XmiParseError::DuplicateId {
            id: xmi_id,
            context: self.current_context(),
        });
    }
    self.id_map.insert(xmi_id, uml_id);
    Ok(())
}
```

**Recommendation:** Include `XmiParseError::DuplicateId` in the error enum. Do not silently overwrite — this would cause silent data corruption if cross-references resolve to the wrong element.

### 4.3 Cross-Reference Resolution During Parsing

A subtle issue: when an element references another element by XMI ID (e.g., `type="jfBF3PIuZOVp"` in an attribute), the referenced element might not yet have been parsed (it may appear later in the file). The two-pass strategy handles this:

- Pass 1 collects all element IDs first (no references processed)
- Pass 2 resolves references against the complete ID map

**This is correct** because all elements in an XMI file are defined before any element's detail content that references them. In the 10 test files, all `<UML:Class>` and `<UML:DataType>` elements appear before their `type` references in `<UML:Attribute>`.

**One caution:** `<UML:Association>` elements contain `<UML:AssociationEnd>` elements with `type` attributes that reference UML elements. These references are resolved in Pass 2. The associations are deferred to M9, so this is not an immediate concern, but the architecture should note that the two-pass approach works for association endpoints as well.

---

## 5. Scope Boundaries: M8 vs M9

### 5.1 Proposed Split

| M8 (structural elements) | M9 (detailed elements) |
|--------------------------|------------------------|
| Package, Class, Interface, Enum | Stereotype, DataType, Relationships, Attributes/Operations |

### 5.2 Analysis

**The split is flawed in two ways:**

#### 5.2.1 DataType should be in M8 (CRITICAL)

**Reason:** Every XMI test file contains a `Datatypes` package with 11-19 `<UML:DataType>` elements (e.g., `int`, `char`, `bool`, `float`, `double`, `short`, `long`, `string`, `unsigned int`, `unsigned short`, `unsigned long`, `byte`, `decimal`, `fixed`, `object`, `sbyte`, `uint`, `ulong`, `ushort`). These are structurally trivial:

```xml
<UML:DataType stereotype="datatype" visibility="public" xmi.id="kyEtsyFPGmQH" name="int"/>
```

A `DataType` element has only:
- `xmi.id` — unique identifier
- `name` — the type name
- `visibility` — always "public"
- `stereotype` — always "datatype" (reference to a stereotype)
- Standard attributes: `isSpecification`, `namespace`, `isAbstract`, `isLeaf`, `isRoot`

**There is no `ModelElement::Datatype` variant in the current codebase.** This must be added. The `ObjectType::Datatype` variant already exists in `types.rs` (line 26), confirming this is a recognized UML concept.

**Recommendation:** Add `ModelElement::Datatype(Datatype)` where `Datatype` is a struct with only `base: ElementBase`. It is the simplest classifier type — no attributes, no operations, no literals. Including it in M8 allows loading complete XMI files without datatype-related errors, making the ID map complete for M9 relationship resolution.

#### 5.2.2 Stereotype resolution must be partially in M8 (HIGH)

**Reason:** The `stereotype` attribute on structural elements is critical for semantic interpretation:

- `stereotype="folder"` on a `<UML:Model>` — indicates this is a logical container folder, not a UML Model in the strict sense
- `stereotype="datatype"` on a `<UML:DataType>` — distinguishes it from a class
- `stereotype="interface"` on a `<UML:Class>` — historical XMI format where interfaces were serialized as classes with an `interface` stereotype (the C++ codebase has logic to promote such elements to `UMLInterface` at load time)
- `stereotype="enum"` — similar promotion logic for enumerations

**Without stereotype resolution, the parser cannot correctly categorize elements.** A `<UML:Class stereotype="interface">` should become a `ModelElement::Interface`, not `ModelElement::Class`.

**Recommendation for scope split:**

| M8 (this milestone) | M9 (next milestone) |
|----------------------|----------------------|
| Package | Relationships (Generalization, Association, Dependency, etc.) |
| Class | Attributes (UML:Classifier.feature → UML:Attribute) |
| Interface | Operations (UML:Classifier.feature → UML:Operation) |
| Enum | Enumeration literals |
| **Datatype** (add) | Template parameters |
| **Stereotype** (parse and resolve, partial) | Full stereotype attribute resolution |

In M8, stereotypes should be:
1. Parsed: extract `<UML:Stereotype>` elements, store name → xmi.id mapping
2. Partially applied: resolve `stereotype="datatype"` → record as `stereotype_id` on the element
3. Promotion: resolve `stereotype="interface"` on a Class → create Interface instead

In M9, stereotypes can be fully integrated (custom stereotype attributes, tagged values).

#### 5.2.3 Current ModelElement variants need a Datatype addition

The current `ModelElement` enum has 5 variants:

```rust
pub enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
    Relationship(Relationship),
}
```

A `Datatype` variant must be added for M8:

```rust
/// A UML datatype (primitive or structured).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Datatype {
    pub base: ElementBase,
}

impl Datatype {
    pub fn new(name: impl Into<String>) -> Self {
        Self { base: ElementBase::new(name) }
    }
}

// In ModelElement:
pub enum ModelElement {
    Package(Package),
    Class(Class),
    Interface(Interface),
    Enum(Enum),
    Datatype(Datatype),  // NEW
    Relationship(Relationship),
}
```

---

## 6. Error Handling

### 6.1 Proposed Error Types

The proposal suggests error types but does not enumerate them. Based on the architecture and XMI format analysis, the following error types are required:

```rust
#[derive(Debug, thiserror::Error)]
pub enum XmiParseError {
    /// XML syntax error from quick-xml.
    #[error("XML parse error at position {pos}: {msg}")]
    XmlError { pos: u64, msg: String },

    /// Expected XMI content structure not found.
    #[error("missing required XMI element: {element}")]
    MissingElement { element: String },

    /// Two elements have the same xmi.id.
    #[error("duplicate xmi.id '{id}' at {context}")]
    DuplicateId { id: String, context: String },

    /// A cross-reference target was not found.
    #[error("unresolved xmi.id reference '{target}' from {context}")]
    UnresolvedReference { target: String, context: String },

    /// Unknown or unsupported element tag.
    #[error("unknown XMI element tag '{tag}' at {context}")]
    UnknownElement { tag: String, context: String },

    /// Required attribute missing on element.
    #[error("missing required attribute '{attr}' on element '{element}'")]
    MissingAttribute { element: String, attr: String },

    /// Name/id mismatch or invalid value.
    #[error("invalid value for '{field}': {value}")]
    InvalidValue { field: String, value: String },

    /// I/O error reading the file.
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}
```

**Verdict: These types are sufficient** for M8. The `from` impl for `std::io::Error` is essential for `?` propagation. The `from` impl for `quick_xml::Error` should also be added.

### 6.2 Malformed XML Handling

Quick-xml returns errors for:
- Unclosed tags
- Mismatched tags
- Invalid UTF-8
- Unexpected EOF

These are propagated as `XmiParseError::XmlError`. No special handling is needed.

### 6.3 Lenient vs. Strict Parsing

**Recommendation: Strict by default, lenient behind a flag.**

- **Strict mode (default):** Unknown elements produce errors. This catches incorrectly formatted XMI or XMI from non-Umbrello tools that the parser doesn't understand. Fail-fast is better than silent data loss.
- **Lenient mode (`--lenient` or similar):** Unknown elements are skipped with a `warn!()` log. This enables loading XMI from tools that add custom extensions.

The architecture should support:

```rust
pub struct XmiReaderConfig {
    pub strict: bool,          // default true
    pub xmi_version: Option<XmiVersion>,  // None = auto-detect
}
```

**For M8, strict mode only is acceptable.** Add lenient mode in a follow-up milestone.

### 6.4 Missing Optional Attributes

Many XMI attributes are optional or have sensible defaults:
- `visibility` — default to `Visibility::Public`
- `isAbstract` — default to `false`
- `isSpecification` — default to `false`
- `comment` — default to empty string

The parser should use `Option::unwrap_or_default()` or similar patterns for these. Missing required attributes (`xmi.id`, `name`) should produce `XmiParseError::MissingAttribute`.

---

## 7. Integration with UmlModel

### 7.1 add_to_package Integration

**Question:** Does `add_to_package` work correctly with the `parent_stack`?

**Answer: YES, with one sequencing constraint.**

The `UmlModel::add_to_package(package_id, child_id)` method:
1. Validates both elements exist (returns `ElementNotFound` if not)
2. Checks for containment cycles (returns `WouldCreateCycle` if detected)
3. Adds `child_id` to `package.children`
4. Updates `parent_index`

During XMI parsing, elements must be inserted into `UmlModel` in **top-down order**: the parent package must be inserted before its children. The `parent_stack` approach naturally produces this order — when a container element is opened, it's pushed onto the stack. When a child element is encountered, the parent (top of stack) already exists in the model.

**One constraint:** Elements must be inserted into `UmlModel` when their start-tag is parsed, not when their end-tag is parsed. If elements were inserted at end-tag, children would be inserted before parents (children end before parents). The event-based parser must call `model.insert(...)` on `StartElement` for structural elements.

### 7.2 Cycle Detection During XMI Parsing

The `UmlModel::would_create_cycle` check uses the `parent_index` to walk up the ancestor chain. During XMI parsing, the parent chain is built incrementally:

1. Root `UML:Model` is inserted → no parents
2. `Logical View` (a `UML:Model`) is inserted → `add_to_package(root_id, logical_view_id)` → no cycle (logical_view has no ancestors)
3. `Datatypes` Package is inserted → `add_to_package(logical_view_id, datatypes_id)` → no cycle

This is correct because XMI files define a tree, not a graph, for containment. Cycles in the XMI containment structure would be malformed. The `would_create_cycle` check would catch any attempt to create one.

**Verdict: The existing cycle detection works correctly during XMI loading.** The parser does not need additional cycle-detection logic.

### 7.3 validate_references After XMI Parsing

**Question:** Does the existing `validate_references()` work after XMI parsing?

**Answer: YES.**

After all elements are inserted and all cross-references are resolved (Pass 2 completes), `validate_references()` checks:
- Package children — all children must exist
- Attribute/Operation/Parameter type references
- Stereotype references
- Relationship source/target

A clean load should produce zero reference errors. The XMI parser should call `model.validate_references()` after Pass 2 completes and return any errors found.

**Recommendation:** Add a `validate_after_load()` step that:
1. Calls `model.validate_references()`
2. Logs each error at `warn!` level
3. Returns all errors to the caller
4. The caller decides whether to reject the load or proceed with warnings

---

## 8. Test Coverage

### 8.1 Proposed 8 Tests

The proposal mentions "8 proposed tests" without enumerating them. The testing strategy document specifies the following for XMI compatibility:

From `testing_strategy.md` Section 5:

| Test Type | Description | Files |
|-----------|-------------|-------|
| Round-trip test | Load XMI → save XMI → compare byte-level | 10 C++ test `.xmi` files |
| Cross-tool verification | Load XMI in Rust, save, verify C++ Umbrello can load result | Requires C++ binary |
| Malformed XMI | Load invalid/malformed files, verify error handling | Synthetic test files |
| Empty model | Load XMI with no elements, verify empty model | Synthetic |
| Duplicate IDs | XMI with duplicate xmi.id values | Synthetic |
| Missing attributes | XMI with missing required attributes | Synthetic |
| Deep nesting | XMI with deeply nested packages | Synthetic |
| Large model stress | XMI with 10,000+ elements | Synthetic or generated |

**Verdict: 8 tests are the minimum viable set. The following specific tests are required for M8:**

1. **`test_parse_empty_xmi`** — Load an XMI file with XMI.header/XMI.content but no owned elements → verify empty model
2. **`test_parse_single_class`** — Load XMI with one class → verify element count, name, ID present
3. **`test_parse_package_with_classes`** — Load XMI with package containing classes → verify containment
4. **`test_parse_stereotypes`** — Load XMI with stereotypes and stereotyped elements → verify stereotype resolution
5. **`test_parse_datatypes`** — Load XMI with Datatypes package → verify all datatypes parsed
6. **`test_parse_model_views`** — Load XMI with Logical View, Use Case View, etc. → verify multiple top-level packages
7. **`test_parse_missing_xmi_id`** — Load XMI with an element missing xmi.id → verify error returned
8. **`test_parse_duplicate_xmi_id`** — Load XMI with two elements sharing xmi.id → verify `DuplicateId` error

### 8.2 Should We Test with All 10 C++ XMI Test Files?

**YES, as integration tests.** M8 should include a test that iterates over the 10 XMI files in `test/` and verifies:

1. Each file parses without errors (strict mode)
2. Each file produces a non-empty model
3. `validate_references()` returns empty after load
4. Element count is non-zero

These 10 files are:
```
test-BVW.xmi  test-CDL.xmi  test-COG.xmi  test-DCL.xmi  test-DCL2.xmi
test-DSM.xmi  test-DST.xmi  test-DUC.xmi  test-DUC2.xmi  test-RFA.xmi
```

The testing strategy document says these should be at `crates/uml-core/tests/fixtures/` and loaded via `include_str!()`.

**One reservation:** Some test files contain element types deferred to M9 (associations, use cases, actors). The parser must handle unknown elements gracefully — either skip them (lenient mode) or produce errors for them (strict mode). For M8, the test should use strict mode and verify that:
- All known element types (Package, Class, Interface, Enum, Datatype) are parsed correctly
- Unknown elements produce parse errors (verifying strict mode works)
- The model is non-empty and `validate_references()` passes for the known elements

### 8.3 XMI 2.1 Format Testing

**Should we test XMI 2.1?** The testing strategy document mentions `test-2.1.xmi` as a fixture, but no such file exists in the repo. The C++ Umbrello codebase uses XMI 1.2 exclusively. XMI 2.1 testing should be deferred to M9+ when the parser is known to handle the structural format correctly. The `XmiVersion` enum is already defined and provides the framework.

**Recommendation:** Add XMI 2.1 support in M9. M8 should have one smoke test for XMI 2.1 detection (auto-detect `xmi.version="2.1"` attribute on root element, verify it doesn't crash), but not comprehensive 2.1 parsing.

---

## 9. Round-Trip Compatibility

### 9.1 The Critical Flaw

**The proposal says "UmlId::new() for XMI elements — this loses the original xmi.id."**

**This is a CRITICAL defect.** The primary validation mechanism for XMI persistence is the round-trip test:

```
Load XMI → Save XMI → Compare byte-for-byte
```

If `UmlId::new()` generates UUID v4 for every element, the saved XMI will have different IDs than the original XMI. The round-trip comparison will fail even though the model is semantically identical.

**This also breaks the C++ Umbrello compatibility guarantee.** If a user opens an XMI file in Rust Umbrello, makes a change, and saves, the C++ Umbrello will not recognize any of the cross-references because the IDs have changed.

### 9.2 Required Solution: Preserve Original XMI ID

The `ElementBase` struct must be extended to preserve the original XMI ID:

**Option A: Add `original_xmi_id` field to `ElementBase` (RECOMMENDED)**

```rust
pub struct ElementBase {
    pub id: UmlId,
    pub name: String,
    pub visibility: Visibility,
    pub stereotype_id: Option<UmlId>,
    pub documentation: String,
    pub is_abstract: bool,
    pub is_static: bool,
    /// Original XMI `xmi.id` string, if this element was loaded from XMI.
    /// Preserved for round-trip XMI serialization — the XMI writer emits
    /// this value as the `xmi.id` attribute instead of `id.to_string()`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_xmi_id: Option<String>,
}
```

**Implications:**
- The `id` field still uses `UmlId` (UUID v4) for all internal referencing. No changes needed to `UmlModel`, cross-references, or `validate_references()`.
- The XMI writer checks `element.base().original_xmi_id` — if `Some`, writes that string as `xmi.id`; if `None`, falls back to `element.base().id.to_string()`.
- The XMI reader sets `original_xmi_id = Some(xmi_id_string)` when loading.
- Elements created programmatically (not from XMI) have `original_xmi_id = None` and get UUID-based IDs in XMI output.
- JSON serde round-trip: `original_xmi_id` is `Option<String>`, skips serialization when `None`. This has no impact on existing serde tests.

**Option B: Maintain a separate reverse map in the XMI writer**

A `HashMap<UmlId, String>` in the XMI writer that maps internal IDs to original XMI strings. This is conceptually simpler (no change to `ElementBase`) but is fragile:
- The map must be kept in sync with model mutations. If elements are added or removed, the map becomes stale.
- The map is external to the model — it must be passed around alongside `UmlModel`.
- Two sources of truth for element identity.

**Recommendation: Option A.** Embedding `original_xmi_id` in `ElementBase` is the correct pattern — it follows Principle 5 (ID-based references with metadata) and keeps the identity information co-located with the element. The field is `Option` so it has zero cost for programmatically-created elements.

### 9.3 Round-Trip Test Implementation

With `original_xmi_id` preserved, the round-trip test works:

1. Load XMI → model has `original_xmi_id` set on every element
2. Save model as XMI → writer emits `original_xmi_id` values
3. Compare bytes → should be near-identical (may differ in non-semantic whitespace and attribute ordering)

For byte-level comparison, the XMI writer must:
- Preserve attribute ordering (C++ Umbrello uses a specific attribute order: `stereotype`, `visibility`, `isSpecification`, `namespace`, `isAbstract`, `isLeaf`, `isRoot`, `xmi.id`, `name`, `comment`)
- Preserve self-closing tags vs. open/close tags where the original used them
- Not reorder elements within `UML:Namespace.ownedElement`

**This is the hardest part of XMI writer implementation** and should be addressed in a separate writer design document for M8.5 or M9. For M8, it is sufficient to verify that `original_xmi_id` is preserved correctly — exact byte-level round-trip can be a stretch goal.

### 9.4 Alternative for M8: Canonical Comparison

Rather than byte-level comparison, M8 can implement a *canonical comparison*:

```
Load XMI → Save XMI to temp file → Load the saved XMI → Compare models structurally
```

Two models are structurally equal if:
- Same number of elements
- Same element names, object types, visibilities, stereotypes
- Same containment structure (parent_index)
- Same `original_xmi_id` values
- Same attributes/operations/literals

This bypasses the byte-level ordering problem while still validating correctness. Byte-level comparison can be added in a later milestone when the writer's attribute ordering is aligned.

---

## 10. Architectural Gaps Identified

### 10.1 Missing Element Type: Datatype

The `ModelElement` enum has no `Datatype` variant. This must be added as a precondition for M8.

### 10.2 Missing Element Type: Stereotype

The `ModelElement` enum has no `Stereotype` variant, yet stereotypes appear in every XMI file. For M8, stereotypes can be partially handled (resolution only, no full model element representation). For M9, a proper `ModelElement::Stereotype(Stereotype)` variant should be added.

### 10.3 Subordinate Element Parsing (Attributes/Operations)

The proposal defers attributes/operations to M9. These are children of `<UML:Classifier.feature>` in the XMI:

```xml
<UML:Class xmi.id="sGBeu79qqOiF" name="ClassA">
  <UML:Classifier.feature>
    <UML:Attribute xmi.id="Pm8KwN2qa8F1" type="jfBF3PIuZOVp" name="m_attrA1"/>
    <UML:Operation xmi.id="BBBuUhBJCCSr" name="start"/>
  </UML:Classifier.feature>
</UML:Class>
```

The event-based parser must handle this nesting in M8 even if it only skips the content. The `<UML:Classifier.feature>` element is a child of `<UML:Class>` and must be recognized to avoid misinterpreting its children as package-level elements.

**Recommendation:** In M8, recognize `<UML:Classifier.feature>` and skip its children (using `read_to_end`). In M9, parse them into `ClassifierData`.

### 10.4 Namespace Resolution

The `namespace` attribute on XMI elements (e.g., `namespace="Logical View"`) references the parent by name, not by ID. The C++ codebase resolves this by looking up the name in a name-to-id map. The proposal's `parent_stack` approach bypasses this need, but the parser should also collect `(name → xmi.id)` mappings for elements that have names. This enables:

1. Resolving `namespace` attributes that disagree with the stack
2. Name-based lookups for elements that are referenced by name in XMI extensions

**Recommendation:** Build a `HashMap<String, Vec<UmlId>>` mapping names to IDs during Pass 1. Use it for validation and for name-based reference resolution.

### 10.5 Diagram and Widget Data

The `<XMI.extension xmi.extender="umbrello">` block contains `<diagrams>`, `<listview>`, and `<codegeneration>` sections. These are not structural UML elements and should be skipped entirely in M8.

**Recommendation:** The parser should recognize `<XMI.extension>` and skip to the corresponding closing tag using `read_to_end`. This is O(element count within the extension block) but avoids processing non-structural data.

---

## 11. Summary of Required Conditions

### Mandatory (must be resolved before M8 implementation begins)

| # | Condition | Rationale |
|---|-----------|-----------|
| **C1** | Add `original_xmi_id: Option<String>` to `ElementBase` | Critical for round-trip XMI testing and C++ compatibility |
| **C2** | Add `ModelElement::Datatype(Datatype)` variant to elements | Every XMI file contains datatypes; needed for complete ID map |
| **C3** | Parse and partially resolve stereotypes in M8 (name→xmi.id mapping, `stereotype_id` assignment) | Stereotype resolution is needed for correct element categorization (Class vs Interface) |
| **C4** | Add `XmiParseError` enum with `DuplicateId`, `UnresolvedReference`, `UnknownElement`, `MissingAttribute`, `InvalidValue` variants | Error handling must be comprehensive from the start |
| **C5** | Handle `<UML:Classifier.feature>` as recognized-but-skipped element in M8 | Prevents misinterpreting attributes/operations as package-level elements |
| **C6** | Build both ID map (`xmi.id → UmlId`) and name map (`name → Vec<UmlId>`) during Pass 1 | Enables namespace validation and future name-based resolution |
| **C7** | Implement all 8 specified tests, plus parse-all-10-xmi-files integration test | Minimum viable validation |
| **C8** | The XMI writer must emit `original_xmi_id` when present, falling back to `id.to_string()` otherwise | Enables round-trip comparison in M9 |

### Recommendations (should be addressed, but can follow after initial M8 implementation)

| # | Recommendation | Rationale |
|---|----------------|-----------|
| **R1** | Match XMI element tags by local name (strip `UML:` prefix) | Enables parsing XMI with different namespace prefixes |
| **R2** | Implement canonical comparison for round-trip testing (load → save → load → compare structurally) | Bypasses byte-level ordering issues; simpler than exact byte comparison |
| **R3** | Include one smoke test for XMI 2.1 format detection (don't crash on `xmi.version="2.1"`) | Framework exists in `XmiVersion` enum; test validates it doesn't break |
| **R4** | Add `XmiReaderConfig` struct with `strict: bool` (default true) for lenient mode in future | Prepares for loading XMI from non-Umbrello tools |

---

## 12. Final Recommendation

### APPROVE WITH CONDITIONS

The proposed architecture — quick-xml event-based parser, two-pass strategy, `parent_stack` containment tracking — is sound and well-suited to the XMI format. The parser choice is correct, the two-pass separation is clean, and the integration with the existing `UmlModel` repository is well-aligned.

However, the proposal cannot be implemented as-is due to **one critical defect**: the loss of original XMI IDs via `UmlId::new()`. This would make round-trip XMI testing impossible, which is identified in `testing_strategy.md` as the single most important test category. The `original_xmi_id` field on `ElementBase` (Condition C1) is a minimal, backward-compatible fix that preserves all existing code while enabling the critical validation path.

Additionally, the M8/M9 scope split must be adjusted to include `DataType` (trivial, needed for complete XMI loading) and partial stereotype resolution (needed for correct element categorization). These are low-effort additions that dramatically increase the value of the M8 deliverable.

The parser team should:

1. **Before writing any code:** Update `ElementBase` with `original_xmi_id: Option<String>` (Condition C1) and add `ModelElement::Datatype` (Condition C2).
2. **During implementation:** Build the `XmiParseError` enum first (Condition C4) to establish the error contract.
3. **After implementation:** Run the 8 specified tests and the 10-file integration test (Condition C7). Verify that `validate_references()` returns empty after loading.
4. **Before declaring M8 complete:** Demonstrate that `original_xmi_id` is preserved (Condition C8) by writing a test that loads an XMI file, checks the `original_xmi_id` values, and verifies the XMI writer emits them.

The round-trip byte-level comparison can be deferred to M8.5 or M9, but the foundation (ID preservation) must be in place from M8.

---

> **Signed:** Umbrello-RS Reviewer  
> **Date:** 2026-06-23  
> **Disposition:** APPROVE WITH CONDITIONS (8 mandatory, 4 recommendations)  
> **Required changes before implementation:** Add `original_xmi_id` to `ElementBase`, add `Datatype` to `ModelElement`  
> **Scope adjustment:** Include Datatype and partial stereotype resolution in M8
