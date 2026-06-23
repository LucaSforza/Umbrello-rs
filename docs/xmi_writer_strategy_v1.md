# XMI Writer Strategy v1 — XMI 1.2 Serialization for Umbrello-RS

> **Document:** `rust-rewrite/docs/xmi_writer_strategy_v1.md`
> **Status:** Draft
> **Phase:** Milestone 10 (XMI Persistence — Writer Implementation)
> **Last updated:** 2026-06-23
>
> This document defines the strategy for implementing `XmiWriter` — the XMI 1.2
> serializer that produces output byte-for-byte compatible with legacy Umbrello
> C++ `.xmi` files, enabling round-trip persistence.
>
> **Scope:** XMI **writer** only. The reader (M8–M9) is already implemented.
> This document covers structural serialization of all model elements, the
> ID mapping strategy for round-trip compatibility, the `quick-xml::Writer`-
> based API, and the round-trip test plan.

---

## Table of Contents

1. [Design Goals & Constraints](#1-design-goals--constraints)
2. [Architecture Overview](#2-architecture-overview)
3. [Writer Location & Module Structure](#3-writer-location--module-structure)
4. [ID Mapping Strategy (Reverse Map)](#4-id-mapping-strategy-reverse-map)
5. [XmiWriter API](#5-xmiwriter-api)
6. [Writing Algorithm](#6-writing-algorithm)
7. [Element Serialization Reference](#7-element-serialization-reference)
8. [XMI.extensions Section](#8-xmi-extensions-section)
9. [Round-Trip Compatibility](#9-round-trip-compatibility)
10. [Error Handling](#10-error-handling)
11. [Round-Trip Test Plan](#11-round-trip-test-plan)
12. [Implementation Plan](#12-implementation-plan)
13. [References](#13-references)

---

## 1. Design Goals & Constraints

### Goals

| # | Goal | Motivation |
|---|------|------------|
| G1 | **Semantic round-trip:** `read → write → read` preserves all model structure | Core persistence requirement; users must not lose data when saving and re-opening |
| G2 | **Backward-compatible output:** written XMI should be loadable by C++ Umbrello | Interoperability during the migration period |
| G3 | **Deterministic output:** writing the same model twice produces identical XML (modulo timestamps) | Reproducible builds, diff-friendly CI |
| G4 | **Preserve original XMI IDs** when available from `ElementBase::original_xmi_id` | Round-trip compatibility and human traceability |
| G5 | **Generate short stable IDs** for new elements | Keep file size small and IDs readable |

### Non-Goals

- Not required to preserve byte-identical output (IDs, timestamps, and formatting may differ for new elements).
- Not required to write diagram data or widget positions (deferred to later milestone).
- Not required to support XMI 2.1 output (only XMI 1.2).

### Key Constraints

1. **Use `quick-xml::Writer`** — same XML library as the reader, already a workspace dependency.
2. **Write to any `std::io::Write`** — file, buffer, network, etc.
3. **Model is read-only during writing** — the writer borrows `&UmlModel`.
4. **No Arc/Mutex** — single-threaded synchronous writing.
5. **No unwrap/expect** — all errors through `Result<(), XmiWriteError>`.

---

## 2. Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        XmiWriter                                │
│                                                                 │
│  ┌─────────────────────┐     ┌──────────────────────────────┐   │
│  │    IdMap             │     │    quick_xml::Writer<W>       │   │
│  │    (UmlId → String)  │────▶│    (low-level XML emitter)    │   │
│  └─────────────────────┘     └──────────────────────────────┘   │
│         ↕                           ↕                           │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │              Element Visitors / Writers                   │    │
│  │  write_class()  write_interface()  write_enum()          │    │
│  │  write_package() write_datatype() write_relationship()    │    │
│  │  write_attribute() write_operation() write_parameter()    │    │
│  └─────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────┘
```

The writer processes the `UmlModel` in a well-defined order:

1. **Build ID map** — walk all elements to pre-compute UmlId → XMI string ID mappings.
2. **Write XML declaration** — `<?xml version="1.0" encoding="UTF-8"?>`.
3. **Write XMI root** — `<XMI xmi.version="1.2" ...>`.
4. **Write XMI.header** — exporter identity, metamodel reference.
5. **Write XMI.content** — the actual `<UML:Model>` tree:
   - Stereotypes
   - Top-level packages (recursively)
   - Top-level classifiers
   - Relationships (as siblings under `<UML:Namespace.ownedElement>`)
6. **Write XMI.extensions** — docsettings, uniqueid, documentation.
7. **Close root** — `</XMI>`.

---

## 3. Writer Location & Module Structure

### Decision: Write in `crates/uml-io/src/xmi/writer.rs`

The reader lives in `uml-io/src/xmi/`. The writer logically co-locates with
the reader in the same module.

**Rationale:**
- The `uml-core` crate already has a stub at `crates/uml-core/src/xmi/writer.rs`.
  This stub is a no-op placeholder from the initial workspace consolidation.
- The actual writer depends on `quick-xml` (which `uml-core` already has as a
  dependency), but the reader is already in `uml-io`.
- Since both reader and writer share the `IdMap` concept and XMI constants,
  keeping them together in `uml-io` avoids duplication.

### Proposed layout

```
crates/uml-io/src/
├── lib.rs
├── storage.rs
└── xmi/
    ├── mod.rs
    ├── reader.rs
    ├── error.rs          ← re-exported as XmiParseError
    ├── writer.rs         ← NEW: XmiWriter + XmiWriteError
    └── id_map.rs         ← FUTURE: shared IdMap if refactored
```

### Fate of `crates/uml-core/src/xmi/writer.rs`

The existing stub in `uml-core/src/xmi/writer.rs` should be **removed** after
the real writer is implemented in `uml-io`. The empty `pub struct XmiWriter;`
placeholder was useful during early milestones but has no callers.

Alternatively, replace the stub with:
```rust
// uml-core/src/xmi/writer.rs — re-export only
pub use uml_io::xmi::writer::XmiWriter;
```
to avoid breaking any code that may reference `uml_core::xmi::writer::XmiWriter`.
However, since no production code depends on it, outright removal is cleaner.

### uml-io dependencies (already satisfied)

```toml
[dependencies]
uml-core = { path = "../uml-core" }
quick-xml.workspace = true   # needed for Writer<W>
thiserror.workspace = true   # for XmiWriteError
tracing.workspace = true     # for debug logging
```

No new dependencies required.

---

## 4. ID Mapping Strategy (Reverse Map)

### 4.1 Problem

XMI uses short string IDs (`"sGBeu79qqOiF"`, `"m1"`, `"Pm8KwN2qa8F1"`).
Our domain model uses UUID-backed `UmlId`. When writing, we need to convert
`UmlId` → XMI string ID for every element reference:

- `xmi.id="..."` on the element itself
- `type="..."` on attributes and parameters (referencing a classifier or datatype)
- `supplier="..."` and `client="..."` on dependencies
- `child="..."` and `parent="..."` on generalizations
- `xmi.idref="..."` on generalizations inside `GeneralizableElement.generalization`
- `type="..."` on AssociationEnd elements

### 4.2 Strategy: Pre-built HashMap

```rust
use std::collections::HashMap;
use uml_core::id::UmlId;

/// Maps UmlId → XMI string ID.
///
/// Built before writing begins by walking the entire model.
#[derive(Debug, Clone)]
struct IdMap {
    uml_to_xmi: HashMap<UmlId, String>,
    /// Counter for generating new IDs for elements without original_xmi_id.
    next_id: u64,
}
```

#### ID resolution priority:

1. **Use `original_xmi_id`** when available (`element.base().original_xmi_id` is `Some`).
2. **Generate new ID** for elements created natively: `format!("rs{:08x}", self.next_id.fetch_add(1, Ordering::Relaxed))`.

```rust
impl IdMap {
    fn new() -> Self {
        Self {
            uml_to_xmi: HashMap::new(),
            next_id: 1,
        }
    }

    /// Build the complete ID mapping by walking the model.
    fn build(&mut self, model: &UmlModel) {
        for (id, elem) in model.iter() {
            let xmi_id = self.xmi_id_for_element(id, elem);
            self.uml_to_xmi.insert(id, xmi_id);
        }
    }

    /// Determine the XMI string ID for a UmlId.
    fn xmi_id_for_element(&mut self, id: UmlId, elem: &ModelElement) -> String {
        if let Some(ref orig) = elem.base().original_xmi_id {
            return orig.clone();
        }
        let new_id = format!("rs{:08x}", self.next_id);
        self.next_id += 1;
        new_id
    }

    /// Resolve a UmlId to its XMI string ID.
    /// Panics only in case of programmer error (ID not in map).
    fn resolve(&self, id: UmlId) -> &str {
        self.uml_to_xmi.get(&id).expect("UmlId not in IdMap — was build() called?")
    }

    /// Resolve a TypeReference to its XMI string for the `type` attribute.
    fn resolve_type(&self, type_ref: &TypeReference) -> Option<String> {
        if let Some(model_id) = type_ref.model_id {
            Some(self.resolve(model_id).to_string())
        } else if let Some(ref name) = type_ref.type_name {
            // Primitives and external types are written as literal names
            Some(name.clone())
        } else {
            None // unspecified type — omit attribute
        }
    }
}
```

### 4.3 ID Prefix Strategy

For newly generated IDs, use the prefix `"rs"` to distinguish Rust-generated
IDs from C++-generated ones. This aids debugging and makes it visually clear
which tool created the element.

```
rs00000001
rs0000002a
rs0000abcd
```

The format is `rs` + 8 hex digits = 10 characters, matching the typical length
of a C++ `createChildId()` output (e.g., `"sGBeu79qqOiF"` = 12 chars).

### 4.4 Building the Map (Two Phases)

It is critical that the ID map is built **before** any element is written,
because a relationship may reference a classifier that is serialized later.

```rust
/// Phase 1: Build the complete mapping.
let mut id_map = IdMap::new();
for (_id, elem) in model.iter() {
    let xmi_id = if let Some(ref orig) = elem.base().original_xmi_id {
        orig.clone()
    } else {
        let new_id = format!("rs{:08x}", id_map.next_id);
        id_map.next_id += 1;
        new_id
    };
    id_map.uml_to_xmi.insert(elem.id(), xmi_id);
}

/// Phase 2: Write using the map.
let mut writer = quick_xml::Writer::new_with_indent(output, b' ', 2);
// ... serialization ...
```

This two-phase approach guarantees that when we write `<UML:Dependency
supplier="..." client="...">`, both `supplier` and `client` IDs are already
in the map.

---

## 5. XmiWriter API

### 5.1 Error Type

```rust
use std::io;
use thiserror::Error;

/// Errors that can occur during XMI writing.
#[derive(Debug, Error)]
pub enum XmiWriteError {
    /// I/O error from the underlying writer.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// XML serialization error from quick-xml.
    #[error("XML serialization error: {0}")]
    Xml(#[from] quick_xml::Error),
}
```

### 5.2 XmiWriter Struct

```rust
use std::io::Write;
use uml_core::repository::UmlModel;

/// Writes a `UmlModel` to XMI 1.2 format.
///
/// Produces output compatible with legacy Umbrello C++ XMI files.
/// Preserves original XMI IDs when available for round-trip compatibility.
///
/// # Example
///
/// ```rust,ignore
/// use uml_io::xmi::writer::XmiWriter;
/// use uml_core::repository::UmlModel;
///
/// let model = UmlModel::new();
/// let mut output = Vec::new();
/// let mut writer = XmiWriter::new(&mut output);
/// writer.write_document(&model).unwrap();
/// let xmi_string = String::from_utf8(output).unwrap();
/// ```
pub struct XmiWriter<'w, W: Write> {
    /// The underlying quick-xml writer.
    inner: quick_xml::Writer<W>,
    /// The ID mapping (UmlId → XMI string).
    id_map: IdMap,
    /// Phantom lifetime for the borrow of the writer output.
    _lifetime: std::marker::PhantomData<&'w ()>,
}

impl<'w, W: Write> XmiWriter<'w, W> {
    /// Create a new XMI writer that writes to the given output.
    ///
    /// Uses 2-space indentation for readable output.
    pub fn new(inner: W) -> Self {
        Self {
            inner: quick_xml::Writer::new_with_indent(inner, b' ', 2),
            id_map: IdMap::new(),
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Write the complete XMI document for the given model.
    ///
    /// This is the main entry point. It builds the ID map, writes the
    /// XML declaration, header, content, extensions, and closing tags.
    ///
    /// # Errors
    ///
    /// Returns `XmiWriteError::Io` if the underlying writer fails.
    /// Returns `XmiWriteError::Xml` if the XML serialization fails
    /// (e.g., invalid characters in element names).
    pub fn write_document(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
        // Phase 1: build ID map
        self.id_map.build(model);

        // Phase 2: write document
        self.write_xml_declaration()?;
        self.write_xmi_root_start()?;
        self.write_header()?;
        self.write_content(model)?;
        self.write_extensions(model)?;
        self.write_xmi_root_end()?;

        Ok(())
    }
}
```

### 5.3 Helper Methods

The `XmiWriter` has a set of `write_*` helper methods, all `pub(crate)` or
private, that handle individual element types:

```rust
impl<'w, W: Write> XmiWriter<'w, W> {
    // ── Document structure ──────────────────────────────────────────
    fn write_xml_declaration(&mut self) -> Result<(), XmiWriteError>;
    fn write_xmi_root_start(&mut self) -> Result<(), XmiWriteError>;
    fn write_xmi_root_end(&mut self) -> Result<(), XmiWriteError>;
    fn write_header(&mut self) -> Result<(), XmiWriteError>;

    // ── Content section ─────────────────────────────────────────────
    fn write_content(&mut self, model: &UmlModel) -> Result<(), XmiWriteError>;
    fn write_uml_model_root(&mut self, model: &UmlModel) -> Result<(), XmiWriteError>;
    fn write_stereotypes(&mut self, model: &UmlModel) -> Result<(), XmiWriteError>;
    fn write_package(&mut self, model: &UmlModel, pkg_id: UmlId) -> Result<(), XmiWriteError>;
    fn write_classifier(&mut self, model: &UmlModel, elem_id: UmlId) -> Result<(), XmiWriteError>;
    fn write_relationships(&mut self, model: &UmlModel) -> Result<(), XmiWriteError>;

    // ── Feature serialization ───────────────────────────────────────
    fn write_classifier_features(&mut self, cd: &ClassifierData) -> Result<(), XmiWriteError>;
    fn write_attribute(&mut self, attr: &Attribute) -> Result<(), XmiWriteError>;
    fn write_operation(&mut self, op: &Operation) -> Result<(), XmiWriteError>;
    fn write_parameters(&mut self, op: &Operation) -> Result<(), XmiWriteError>;

    // ── Relationship serialization ──────────────────────────────────
    fn write_generalization(&mut self, rel: &Relationship) -> Result<(), XmiWriteError>;
    fn write_association(&mut self, rel: &Relationship) -> Result<(), XmiWriteError>;
    fn write_dependency(&mut self, rel: &Relationship) -> Result<(), XmiWriteError>;

    // ── Extensions section ──────────────────────────────────────────
    fn write_extensions(&mut self, model: &UmlModel) -> Result<(), XmiWriteError>;
    fn write_docsettings(&mut self, model: &UmlModel) -> Result<(), XmiWriteError>;

    // ── Attribute helpers ───────────────────────────────────────────
    fn write_base_attrs(&mut self, elem: &ModelElement) -> Result<(), XmiWriteError>;
}
```

---

## 6. Writing Algorithm

### 6.1 Top-Level Flow

```text
write_document(model):
  1. id_map.build(model)
  2. write_xml_declaration()
  3. write_xmi_root_start()
  4. write_header()
  5. write_content(model)
  6. write_extensions(model)
  7. write_xmi_root_end()
```

### 6.2 XML Declaration & XMI Root

```rust
fn write_xml_declaration(&mut self) -> Result<(), XmiWriteError> {
    self.inner.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;
    Ok(())
}

fn write_xmi_root_start(&mut self) -> Result<(), XmiWriteError> {
    let now = chrono_now_rfc3339(); // or just use current UTC timestamp
    self.inner
        .write_event(Event::Start(BytesStart::new("XMI")
            .with_attributes(vec![
                ("verified", "false"),
                ("xmi.version", "1.2"),
                ("timestamp", &now),
                ("xmlns:UML", "http://schema.omg.org/spec/UML/1.3"),
            ])
        ))?;
    Ok(())
}

fn write_xmi_root_end(&mut self) -> Result<(), XmiWriteError> {
    self.inner.write_event(Event::End(BytesEnd::new("XMI")))?;
    Ok(())
}
```

### 6.3 Header

```rust
fn write_header(&mut self) -> Result<(), XmiWriteError> {
    // <XMI.header>
    self.inner.write_event(Event::Start(BytesStart::new("XMI.header")))?;

    //   <XMI.documentation>
    self.inner.write_event(Event::Start(BytesStart::new("XMI.documentation")))?;

    //     <XMI.exporter>...</XMI.exporter>
    self.inner.write_event(Event::Start(BytesStart::new("XMI.exporter")))?;
    self.inner.write_event(Event::Text(BytesText::new(
        crate::xmi::reader::XMI_EXPORTER,  // "umbrello uml modeller http://umbrello.kde.org"
    )))?;
    self.inner.write_event(Event::End(BytesEnd::new("XMI.exporter")))?;

    //     <XMI.exporterVersion>...</XMI.exporterVersion>
    self.inner.write_event(Event::Start(BytesStart::new("XMI.exporterVersion")))?;
    self.inner.write_event(Event::Text(BytesText::new("1.5.8")))?;
    self.inner.write_event(Event::End(BytesEnd::new("XMI.exporterVersion")))?;

    //     <XMI.exporterEncoding>...</XMI.exporterEncoding>
    self.inner.write_event(Event::Start(BytesStart::new("XMI.exporterEncoding")))?;
    self.inner.write_event(Event::Text(BytesText::new("UnicodeUTF8")))?;
    self.inner.write_event(Event::End(BytesEnd::new("XMI.exporterEncoding")))?;

    //   </XMI.documentation>
    self.inner.write_event(Event::End(BytesEnd::new("XMI.documentation")))?;

    //   <XMI.metamodel xmi.version="1.3" href="UML.xml" xmi.name="UML"/>
    self.inner.write_event(Event::Empty(BytesStart::new("XMI.metamodel")
        .with_attributes(vec![
            ("xmi.version", "1.3"),
            ("href", "UML.xml"),
            ("xmi.name", "UML"),
        ])
    ))?;

    // </XMI.header>
    self.inner.write_event(Event::End(BytesEnd::new("XMI.header")))?;
    Ok(())
}
```

### 6.4 Content Section

```rust
fn write_content(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
    // <XMI.content>
    self.inner.write_event(Event::Start(BytesStart::new("XMI.content")))?;

    // Write the root <UML:Model>
    self.write_uml_model_root(model)?;

    // </XMI.content>
    self.inner.write_event(Event::End(BytesEnd::new("XMI.content")))?;
    Ok(())
}

fn write_uml_model_root(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
    // <UML:Model isSpecification="false" isAbstract="false" isLeaf="false"
    //            xmi.id="m1" isRoot="false" name="UML Model">
    let mut start = BytesStart::new("UML:Model");
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("isAbstract", "false"));
    start.push_attribute(("isLeaf", "false"));
    start.push_attribute(("xmi.id", "m1"));
    start.push_attribute(("isRoot", "false"));
    start.push_attribute(("name", "UML Model"));
    self.inner.write_event(Event::Start(start))?;

    // <UML:Namespace.ownedElement>
    self.inner.write_event(Event::Start(BytesStart::new("UML:Namespace.ownedElement")))?;

    // 1. Write stereotypes
    self.write_stereotypes(model)?;

    // 2. Write top-level packages (those without parents in model)
    self.write_top_level_packages(model)?;

    // 3. Write top-level classifiers (classes, interfaces, enums, datatypes)
    //    that are NOT inside any package
    self.write_top_level_classifiers(model)?;

    // 4. Write relationships
    self.write_relationships(model)?;

    // </UML:Namespace.ownedElement>
    self.inner.write_event(Event::End(BytesEnd::new("UML:Namespace.ownedElement")))?;

    // </UML:Model>
    self.inner.write_event(Event::End(BytesEnd::new("UML:Model")))?;
    Ok(())
}
```

#### Determining "top-level" elements

An element is considered **top-level** if it has no parents in the model's
`parent_index`:

```rust
fn is_top_level(model: &UmlModel, element_id: UmlId) -> bool {
    model.parents_of(element_id).map_or(true, |parents| parents.is_empty())
}
```

Packages written inside another package are handled recursively by
`write_package()`. The top-level loop only processes elements not contained
by any package.

### 6.5 Stereotype Writing

Stereotypes are not yet stored as separate domain objects in M9 (the reader
skips `<UML:Stereotype>` elements). However, the model has `stereotype_id`
references on elements. For the initial writer, stereotypes are **skipped**
(deferred to M11 when the stereotype registry is implemented).

For the initial implementation:
- Write an empty `<UML:Namespace.ownedElement>` if there are no stereotypes.
- When stereotype data is available, iterate `model` for elements with
  `object_type == Stereotype` and write them.

```rust
fn write_stereotypes(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
    // Deferred until stereotypes are stored as first-class elements.
    // In the reader, <UML:Stereotype> is skipped (M9).
    // Writer currently writes nothing for stereotypes.
    Ok(())
}
```

### 6.6 Package Writing (Recursive)

```rust
fn write_package(&mut self, model: &UmlModel, pkg_id: UmlId) -> Result<(), XmiWriteError> {
    let elem = model.get(pkg_id).expect("package must exist");
    let pkg = match elem {
        ModelElement::Package(p) => p,
        _ => return Ok(()), // skip non-packages
    };

    let xmi_id = self.id_map.resolve(pkg_id);

    // <UML:Package visibility="..." xmi.id="..." name="...">
    let mut start = BytesStart::new("UML:Package");
    start.push_attribute(("visibility", pkg.base.visibility.as_str()));
    start.push_attribute(("xmi.id", xmi_id));
    start.push_attribute(("name", &pkg.base.name));
    // Optional attributes: isSpecification, isAbstract, isLeaf, isRoot, stereotype
    self.write_optional_base_attrs(&pkg.base, &mut start);
    self.inner.write_event(Event::Start(start))?;

    // <UML:Namespace.ownedElement>
    self.inner.write_event(Event::Start(BytesStart::new("UML:Namespace.ownedElement")))?;

    // Write child elements recursively
    for &child_id in &pkg.children {
        let child = model.get(child_id).expect("child must exist");
        match child {
            ModelElement::Package(_) => {
                self.write_package(model, child_id)?;
            }
            ModelElement::Class(_)
            | ModelElement::Interface(_)
            | ModelElement::Enum(_)
            | ModelElement::Datatype(_) => {
                self.write_classifier(model, child_id)?;
            }
            ModelElement::Relationship(_) => {
                // Relationships are written as siblings of the top-level model,
                // not inside packages. Skip if encountered here.
            }
        }
    }

    // </UML:Namespace.ownedElement>
    self.inner.write_event(Event::End(BytesEnd::new("UML:Namespace.ownedElement")))?;

    // </UML:Package>
    self.inner.write_event(Event::End(BytesEnd::new("UML:Package")))?;
    Ok(())
}
```

### 6.7 Classifier Writing (Class, Interface, Enum, Datatype)

```rust
fn write_classifier(&mut self, model: &UmlModel, elem_id: UmlId) -> Result<(), XmiWriteError> {
    let elem = model.get(elem_id).expect("classifier must exist");
    let xmi_id = self.id_map.resolve(elem_id);
    let (tag_name, cd) = match elem {
        ModelElement::Class(c) => ("UML:Class", &c.classifier),
        ModelElement::Interface(i) => ("UML:Interface", &i.classifier),
        ModelElement::Enum(e) => ("UML:Enumeration", &e.classifier),
        ModelElement::Datatype(d) => ("UML:DataType", &d.classifier),
        _ => return Ok(()),
    };

    let mut start = BytesStart::new(tag_name);
    start.push_attribute(("visibility", elem.base().visibility.as_str()));
    start.push_attribute(("xmi.id", xmi_id));
    start.push_attribute(("name", elem.base().name.as_str()));
    self.write_optional_base_attrs(&elem.base(), &mut start);

    // Check if this classifier has generalizations (as child, inside element)
    let generalizations: Vec<_> = model
        .generalizations_of(elem_id)
        .into_iter()
        .filter(|r| r.source_id == elem_id) // we are the subclass
        .collect();

    // Check if this classifier has features (attributes, operations)
    let has_features = !cd.attributes.is_empty() || !cd.operations.is_empty();
    let has_generalizations = !generalizations.is_empty();
    let is_self_closing = !has_generalizations && !has_features;

    if is_self_closing {
        // Self-closing tag
        self.inner.write_event(Event::Empty(start))?;
    } else {
        self.inner.write_event(Event::Start(start))?;

        // Write generalizations (child-ref style)
        if has_generalizations {
            self.inner.write_event(Event::Start(
                BytesStart::new("UML:GeneralizableElement.generalization"),
            ))?;
            for gen in &generalizations {
                let super_xmi_id = self.id_map.resolve(gen.target_id);
                self.inner.write_event(Event::Empty(
                    BytesStart::new("UML:Generalization")
                        .with_attributes(vec![("xmi.idref", super_xmi_id)]),
                ))?;
            }
            self.inner.write_event(Event::End(
                BytesEnd::new("UML:GeneralizableElement.generalization"),
            ))?;
        }

        // Write features
        if has_features {
            self.write_classifier_features(cd)?;
        }

        self.inner.write_event(Event::End(BytesEnd::new(tag_name)))?;
    }

    Ok(())
}
```

### 6.8 Writing Generalizations Inside a Classifier

The C++ Umbrello writes generalizations in two formats:

1. **Child-ref style** (preferred): inside the subclass element
   ```xml
   <UML:Class xmi.id="sub1" name="SubClass">
     <UML:GeneralizableElement.generalization>
       <UML:Generalization xmi.idref="super1"/>
     </UML:GeneralizableElement.generalization>
   </UML:Class>
   ```

2. **Standalone style** (also used): as a sibling element
   ```xml
   <UML:Generalization child="sub1" parent="super1" xmi.id="g1" name=""/>
   ```

Our writer uses **child-ref style** (format 1) when writing generalizations
inside a classifier that has them. This matches the pattern seen in
test-COG.xmi for `ClassA → BaseClassA`.

Standalone generalizations (format 2) are used when writing generalizations
as peer elements in the relationships section, but this is **not** the
primary path — see section 6.9.

### 6.9 Writing Relationships (Peer Elements)

Relationships that are **not** written inside a classifier (non-generalization
types: Association, Dependency, plus standalone Generalizations) are written
as sibling elements under `<UML:Namespace.ownedElement>`.

```rust
fn write_relationships(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
    for (id, elem) in model.iter() {
        let rel = match elem {
            ModelElement::Relationship(r) => r,
            _ => continue,
        };

        // Skip generalizations that were already written inside the
        // subclass classifier (child-ref style). Standalone generalizations
        // are still written here.
        // We write standalone generalizations, associations, and dependencies.
        match rel.kind {
            AssociationType::Generalization => {
                // Write standalone generalization. Skip if the superclass
                // already has the generalization written inside it (child-ref).
                // For simplicity, always write as standalone for now.
                self.write_standalone_generalization(rel)?;
            }
            AssociationType::Association
            | AssociationType::Aggregation
            | AssociationType::Composition => {
                self.write_association(rel)?;
            }
            AssociationType::Dependency => {
                self.write_dependency(rel)?;
            }
            AssociationType::Realization => {
                // Write as abstraction
                self.write_abstraction(rel)?;
            }
        }
    }
    Ok(())
}
```

#### Standalone Generalization

```rust
fn write_standalone_generalization(&mut self, rel: &Relationship) -> Result<(), XmiWriteError> {
    let source_xmi = self.id_map.resolve(rel.source_id);
    let target_xmi = self.id_map.resolve(rel.target_id);
    let rel_xmi = self.id_map.resolve(rel.base.id);

    // <UML:Generalization discriminator="" visibility="public"
    //                     isSpecification="false"
    //                     child="sub_xmi" xmi.id="g1" parent="super_xmi" name=""/>
    let mut start = BytesStart::new("UML:Generalization");
    start.push_attribute(("discriminator", ""));
    start.push_attribute(("visibility", "public"));
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("child", source_xmi));
    start.push_attribute(("xmi.id", rel_xmi));
    start.push_attribute(("parent", target_xmi));
    start.push_attribute(("name", ""));
    self.inner.write_event(Event::Empty(start))?;
    Ok(())
}
```

#### Association

```rust
fn write_association(&mut self, rel: &Relationship) -> Result<(), XmiWriteError> {
    let rel_xmi = self.id_map.resolve(rel.base.id);
    let source_xmi = self.id_map.resolve(rel.source_id);
    let target_xmi = self.id_map.resolve(rel.target_id);

    let aggregation_attr = match rel.kind {
        AssociationType::Aggregation => "shared",
        AssociationType::Composition => "composite",
        _ => "none",
    };

    // Generate unique IDs for each AssociationEnd
    let end1_xmi = format!("{}_end1", rel_xmi);
    let end2_xmi = format!("{}_end2", rel_xmi);

    // <UML:Association visibility="public" isSpecification="false"
    //                  xmi.id="..." name="">
    let mut start = BytesStart::new("UML:Association");
    start.push_attribute(("visibility", "public"));
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("xmi.id", rel_xmi));
    start.push_attribute(("name", ""));
    self.inner.write_event(Event::Start(start))?;

    //   <UML:Association.connection>
    self.inner.write_event(Event::Start(BytesStart::new("UML:Association.connection")))?;

    //     <!-- End 1 (source) -->
    //     <UML:AssociationEnd changeability="changeable" visibility="public"
    //                         isNavigable="true|false" isSpecification="false"
    //                         xmi.id="..." type="..." name="" aggregation="none"/>
    {
        let mut end = BytesStart::new("UML:AssociationEnd");
        end.push_attribute(("changeability", "changeable"));
        end.push_attribute(("visibility", "public"));
        end.push_attribute(("isNavigable", if rel.source_to_target_navigable { "true" } else { "false" }));
        end.push_attribute(("isSpecification", "false"));
        end.push_attribute(("xmi.id", &end1_xmi));
        end.push_attribute(("type", source_xmi));
        end.push_attribute(("name", ""));
        end.push_attribute(("aggregation", aggregation_attr));
        self.inner.write_event(Event::Empty(end))?;
    }

    //     <!-- End 2 (target) -->
    {
        let mut end = BytesStart::new("UML:AssociationEnd");
        end.push_attribute(("changeability", "changeable"));
        end.push_attribute(("visibility", "public"));
        end.push_attribute(("isNavigable", if rel.target_to_source_navigable { "true" } else { "false" }));
        end.push_attribute(("isSpecification", "false"));
        end.push_attribute(("xmi.id", &end2_xmi));
        end.push_attribute(("type", target_xmi));
        end.push_attribute(("name", ""));
        end.push_attribute(("aggregation", "none"));
        self.inner.write_event(Event::Empty(end))?;
    }

    //   </UML:Association.connection>
    self.inner.write_event(Event::End(BytesEnd::new("UML:Association.connection")))?;

    // </UML:Association>
    self.inner.write_event(Event::End(BytesEnd::new("UML:Association")))?;
    Ok(())
}
```

#### Dependency

```rust
fn write_dependency(&mut self, rel: &Relationship) -> Result<(), XmiWriteError> {
    let source_xmi = self.id_map.resolve(rel.source_id);
    let target_xmi = self.id_map.resolve(rel.target_id);
    let rel_xmi = self.id_map.resolve(rel.base.id);

    // <UML:Dependency visibility="public" isSpecification="false"
    //                  supplier="target_xmi" xmi.id="rel_xmi"
    //                  client="source_xmi" name=""/>
    let mut start = BytesStart::new("UML:Dependency");
    start.push_attribute(("visibility", "public"));
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("supplier", target_xmi));
    start.push_attribute(("xmi.id", rel_xmi));
    start.push_attribute(("client", source_xmi));
    start.push_attribute(("name", ""));
    self.inner.write_event(Event::Empty(start))?;
    Ok(())
}
```

#### Realization (as Abstraction)

```rust
fn write_abstraction(&mut self, rel: &Relationship) -> Result<(), XmiWriteError> {
    let source_xmi = self.id_map.resolve(rel.source_id);
    let target_xmi = self.id_map.resolve(rel.target_id);
    let rel_xmi = self.id_map.resolve(rel.base.id);

    // <UML:Abstraction visibility="public" isSpecification="false"
    //                    supplier="interface_xmi" xmi.id="rel_xmi"
    //                    client="class_xmi" name=""/>
    let mut start = BytesStart::new("UML:Abstraction");
    start.push_attribute(("visibility", "public"));
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("supplier", target_xmi));
    start.push_attribute(("xmi.id", rel_xmi));
    start.push_attribute(("client", source_xmi));
    start.push_attribute(("name", ""));
    self.inner.write_event(Event::Empty(start))?;
    Ok(())
}
```

### 6.10 Classifier Feature Writing

```rust
fn write_classifier_features(&mut self, cd: &ClassifierData) -> Result<(), XmiWriteError> {
    // <UML:Classifier.feature>
    self.inner
        .write_event(Event::Start(BytesStart::new("UML:Classifier.feature")))?;

    // Write attributes
    for attr in &cd.attributes {
        self.write_attribute(attr)?;
    }

    // Write operations
    for op in &cd.operations {
        self.write_operation(op)?;
    }

    // </UML:Classifier.feature>
    self.inner
        .write_event(Event::End(BytesEnd::new("UML:Classifier.feature")))?;
    Ok(())
}
```

#### Attribute

```rust
fn write_attribute(&mut self, attr: &Attribute) -> Result<(), XmiWriteError> {
    // Generate a fresh XMI ID for this attribute (not stored in model's IdMap)
    // We use a deterministic hash of name + index to keep output stable
    let attr_xmi = self.generate_feature_id("attr", &attr.name);

    // <UML:Attribute visibility="private" isSpecification="false"
    //                 xmi.id="..." type="..." name="m_attr"/>
    let mut start = BytesStart::new("UML:Attribute");
    start.push_attribute(("visibility", attr.visibility.as_str()));
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("xmi.id", &attr_xmi));

    // type attribute: resolved via IdMap or primitive name
    if let Some(type_str) = self.id_map.resolve_type(&attr.type_ref) {
        start.push_attribute(("type", &type_str));
    }

    start.push_attribute(("name", &attr.name));

    self.inner.write_event(Event::Empty(start))?;
    Ok(())
}
```

#### Operation

```rust
fn write_operation(&mut self, op: &Operation) -> Result<(), XmiWriteError> {
    let op_xmi = self.generate_feature_id("op", &op.name);

    // <UML:Operation visibility="public" isSpecification="false"
    //                 isQuery="false" isAbstract="false" isLeaf="false"
    //                 isRoot="false" xmi.id="..." name="...">
    let mut start = BytesStart::new("UML:Operation");
    start.push_attribute(("visibility", op.visibility.as_str()));
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("isQuery", "false"));
    start.push_attribute(("isAbstract", if op.is_abstract { "true" } else { "false" }));
    start.push_attribute(("isLeaf", "false"));
    start.push_attribute(("isRoot", "false"));
    start.push_attribute(("xmi.id", &op_xmi));
    start.push_attribute(("name", &op.name));

    // Check if operation has parameters or a return type
    let has_return = op.return_type.is_resolved();
    let has_params = !op.parameters.is_empty();

    if !has_return && !has_params {
        // Self-closing if no parameters
        self.inner.write_event(Event::Empty(start))?;
    } else {
        self.inner.write_event(Event::Start(start))?;

        // <UML:BehavioralFeature.parameter>
        self.inner
            .write_event(Event::Start(BytesStart::new("UML:BehavioralFeature.parameter")))?;

        // Write return parameter first (if present)
        if op.return_type.is_resolved() {
            self.write_parameter(&op.return_type, "return", None)?;
        }

        // Write in/out/inout parameters
        for param in &op.parameters {
            self.write_parameter(&param.type_ref, param.direction.as_str(), Some(&param.name))?;
        }

        // </UML:BehavioralFeature.parameter>
        self.inner
            .write_event(Event::End(BytesEnd::new("UML:BehavioralFeature.parameter")))?;

        // </UML:Operation>
        self.inner.write_event(Event::End(BytesEnd::new("UML:Operation")))?;
    }

    Ok(())
}
```

#### Parameter

```rust
fn write_parameter(
    &mut self,
    type_ref: &TypeReference,
    kind: &str,
    name: Option<&str>,
) -> Result<(), XmiWriteError> {
    // Generate a deterministic parameter ID
    let param_id = self.generate_feature_id("param", name.unwrap_or("return"));

    // <UML:Parameter kind="return|in|out|inout" xmi.id="..." type="..." name="..."/>
    let mut start = BytesStart::new("UML:Parameter");

    if kind == "return" {
        // Return parameters typically don't have a name attribute
        start.push_attribute(("kind", "return"));
        start.push_attribute(("xmi.id", &param_id));
        if let Some(type_str) = self.id_map.resolve_type(type_ref) {
            start.push_attribute(("type", &type_str));
        }
    } else {
        start.push_attribute(("kind", kind));
        start.push_attribute(("xmi.id", &param_id));
        if let Some(type_str) = self.id_map.resolve_type(type_ref) {
            start.push_attribute(("type", &type_str));
        }
        if let Some(n) = name {
            start.push_attribute(("name", n));
        }
    }

    self.inner.write_event(Event::Empty(start))?;
    Ok(())
}
```

### 6.11 Feature ID Generation

Attributes and operations do not have their own `UmlId` in our domain model
(they are embedded in `ClassifierData`). However, XMI requires each feature
to have a unique `xmi.id`. We generate deterministic IDs based on the parent
classifier's XMI ID and the feature name.

```rust
/// Counter for generating feature-level XMI IDs.
/// Features (attributes, operations, parameters) don't have UmlIds,
/// so we assign them deterministic IDs during writing.
feature_counter: std::cell::Cell<u64>,

fn generate_feature_id(&self, prefix: &str, name: &str) -> String {
    let count = self.feature_counter.get();
    self.feature_counter.set(count + 1);
    format!("{}_{:x}", prefix, count)
}
```

For better determinism, we could use a hash of the parent ID + feature name,
but a simple incrementing counter produces stable output as long as the
model iteration order is deterministic (which it is, via `IndexMap`).

### 6.12 Optional Base Attributes

The C++ Umbrello writes several optional boolean attributes on every element:

- `isSpecification="false"`
- `isAbstract="false"` (on classifiers)
- `isLeaf="false"`
- `isRoot="false"`
- `stereotype="..."` (when a stereotype is assigned)
- `namespace="..."` (reference to parent namespace — optional in Umbrello output)

```rust
fn write_optional_base_attrs(
    &mut self,
    base: &ElementBase,
    start: &mut BytesStart,
) {
    start.push_attribute(("isSpecification", "false"));
    start.push_attribute(("isLeaf", "false"));
    start.push_attribute(("isRoot", "false"));

    if base.is_abstract {
        start.push_attribute(("isAbstract", "true"));
    }

    // Stereotype reference (deferred — M11)
    // if let Some(stereotype_id) = base.stereotype_id {
    //     let stereo_xmi = self.id_map.resolve(stereotype_id);
    //     start.push_attribute(("stereotype", stereo_xmi));
    // }
}
```

---

## 7. Element Serialization Reference

### 7.1 Complete Element Table

| Rust Type | XMI Tag | Self-Closing? | Children |
|-----------|---------|---------------|----------|
| `Package` | `<UML:Package>` | No | `<UML:Namespace.ownedElement>` with nested elements |
| `Class` | `<UML:Class>` | If no features/gen | `<UML:GeneralizableElement.generalization>`, `<UML:Classifier.feature>` |
| `Interface` | `<UML:Interface>` | If no features | `<UML:Classifier.feature>` |
| `Enum` | `<UML:Enumeration>` | If no features/literals | `<UML:Classifier.feature>`, literals (deferred) |
| `Datatype` | `<UML:DataType>` | If no features | `<UML:Classifier.feature>` |
| `Relationship(Generalization)` | `<UML:Generalization>` | Yes | None (standalone) |
| `Relationship(Association)` | `<UML:Association>` | No | `<UML:Association.connection>` with 2 `<UML:AssociationEnd>` |
| `Relationship(Aggregation)` | `<UML:Association>` | No | Same as Association (aggregation="shared") |
| `Relationship(Composition)` | `<UML:Association>` | No | Same as Association (aggregation="composite") |
| `Relationship(Dependency)` | `<UML:Dependency>` | Yes | None |
| `Relationship(Realization)` | `<UML:Abstraction>` | Yes | None |
| `Attribute` | `<UML:Attribute>` | Yes | None |
| `Operation` | `<UML:Operation>` | If no params | `<UML:BehavioralFeature.parameter>` with `<UML:Parameter>` |
| `Parameter` | `<UML:Parameter>` | Yes | None |

### 7.2 Attribute Reference Table

This table shows how each attribute of each Rust type maps to an XMI attribute.

| Rust Field | XMI Attribute | Example | Required? |
|------------|---------------|---------|-----------|
| **All elements** | | | |
| `ElementBase::id` (via IdMap) | `xmi.id` | `xmi.id="sGBeu79qqOiF"` | **Yes** |
| `ElementBase::name` | `name` | `name="ClassA"` | **Yes** |
| `ElementBase::visibility` | `visibility` | `visibility="public"` | **Yes** |
| `ElementBase::is_abstract` | `isAbstract` | `isAbstract="false"` | If true |
| `ElementBase::stereotype_id` | `stereotype` | `stereotype="folder"` | If Some |
| — | `isSpecification` | `isSpecification="false"` | Always "false" |
| — | `isLeaf` | `isLeaf="false"` | Always "false" |
| — | `isRoot` | `isRoot="false"` | Always "false" |
| **Attribute** | | | |
| `Attribute::name` | `name` | `name="m_attrA1"` | **Yes** |
| `Attribute::type_ref` | `type` | `type="jfBF3PIuZOVp"` | If resolved |
| `Attribute::visibility` | `visibility` | `visibility="private"` | **Yes** |
| `Attribute::initial_value` | `initialValue` | `initialValue="0"` | If Some |
| **Operation** | | | |
| `Operation::name` | `name` | `name="start"` | **Yes** |
| `Operation::visibility` | `visibility` | `visibility="public"` | **Yes** |
| `Operation::is_abstract` | `isAbstract` | `isAbstract="false"` | Always "false" |
| — | `isQuery` | `isQuery="false"` | Always "false" |
| **Parameter** | | | |
| — | `kind` | `kind="return"` | "return" or "in" |
| `Parameter::type_ref` | `type` | `type="ksJknVIVTUOM"` | If resolved |
| `Parameter::name` | `name` | `name="startValue"` | If not return |
| **AssociationEnd** | | | |
| — | `changeability` | `changeability="changeable"` | Always "changeable" |
| — | `isNavigable` | `isNavigable="true"` | From Relationship |
| End participant ID | `type` | `type="sGBeu79qqOiF"` | **Yes** |
| — | `aggregation` | `aggregation="none"` | none/shared/composite |
| **Generalization** | | | |
| (child-ref) | `xmi.idref` | `xmi.idref="tvReBia5Mo10"` | Inside child element |
| (standalone) child | `child` | `child="sGBeu79qqOiF"` | **Yes** |
| (standalone) parent | `parent` | `parent="XznwGhvpCws8"` | **Yes** |
| **Dependency** | | | |
| `Relationship::source_id` | `client` | `client="..."` | **Yes** |
| `Relationship::target_id` | `supplier` | `supplier="..."` | **Yes** |

---

## 8. XMI.extensions Section

The `<XMI.extensions>` section contains Umbrello-specific data. For the
initial writer, we write only `<docsettings>` — the minimum required for
a valid file. Diagram and widget data is deferred.

### 8.1 docsettings

```rust
fn write_extensions(&mut self, model: &UmlModel) -> Result<(), XmiWriteError> {
    // <XMI.extensions xmi.extender="umbrello">
    let mut start = BytesStart::new("XMI.extensions");
    start.push_attribute(("xmi.extender", "umbrello"));
    self.inner.write_event(Event::Start(start))?;

    // <docsettings viewid="..." uniqueid="..." documentation=""/>
    //   viewid: the active diagram's XMI ID (empty string if none)
    //   uniqueid: some unique ID for the document (we generate one)
    //   documentation: model-level documentation string
    //
    // The C++ Umbrello stores:
    //   viewid = the last active diagram's XMI ID
    //   uniqueid = a unique ID for the document (used to detect copes)
    // Since we don't track diagram IDs yet, use placeholder values.
    {
        let mut ds = BytesStart::new("docsettings");
        ds.push_attribute(("viewid", "")); // placeholder
        ds.push_attribute(("uniqueid", &self.generate_unique_id()));
        ds.push_attribute(("documentation", ""));
        self.inner.write_event(Event::Empty(ds))?;
    }

    // </XMI.extensions>
    self.inner.write_event(Event::End(BytesEnd::new("XMI.extensions")))?;
    Ok(())
}

fn generate_unique_id(&self) -> String {
    // Generate a stable unique ID for the docsettings
    // Similar to the C++ behavior of generating a random 12-char string
    let id = UmlId::new();
    id.to_string()[..12].to_string()
}
```

### 8.2 Deferred Extension Sections

The following extension sections are **not** written in Milestone 10:

| Section | Purpose | Status |
|---------|---------|--------|
| `<docsettings>` | Active diagram view ID, unique document ID | ✅ Written |
| `<diagrams>` | Diagram widget data, positions, sizes | ❌ Deferred (M12+) |
| `<listview>` | Tree view state in the UI | ❌ Deferred |
| `<codegeneration>` | Source code snippets embedded in model | ❌ Deferred |

---

## 9. Round-Trip Compatibility

### 9.1 What WILL Be Preserved

| Aspect | Guarantee |
|--------|-----------|
| Elements | All classifiers, packages, relationships, features |
| Names | Element names, attribute names, operation names |
| Type references | Attribute/parameter types (resolved to model ID or primitive name) |
| Visibility | public/protected/private/implementation |
| Abstract flag | `isAbstract` on classes and operations |
| Original XMI IDs | Preserved when `original_xmi_id` is `Some` |
| Containment hierarchy | Package structure with nesting |
| Generalizations | Both child-ref and standalone formats |
| Associations | With aggregation/composition types |

### 9.2 What WILL Differ

| Aspect | Reason | Impact |
|--------|--------|--------|
| Timestamp | Generated fresh on each write | Cosmetic |
| Generated IDs (new elements) | `rs00000001` vs C++ random string | Cosmetic |
| Feature IDs (attr/op/param) | Generated fresh, not from original XMI | May differ per write |
| XML formatting | Indentation, line breaks | Cosmetic (quick-xml 2-space vs C++ no indent) |
| Attribute ordering | May differ from original file | Semantic — same elements, different order |
| Missing diagram data | Not written | C++ Umbrello can re-generate from model |
| Missing stereotypes | Not written (deferred) | Elements without stereotype assignment |

### 9.3 Mitigation: Semantic Equivalence

Since byte-identical output is not guaranteed, we test **semantic equivalence**:
the round-trip test verifies that the model structure is preserved, not byte
identity.

---

## 10. Error Handling

### 10.1 Error Enum

```rust
#[derive(Debug, Error)]
pub enum XmiWriteError {
    /// I/O error from the underlying writer.
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    /// XML serialization error from quick-xml.
    #[error("XML serialization error: {0}")]
    Xml(#[from] quick_xml::Error),
}
```

### 10.2 Error Recovery

The writer is **fail-fast**: any I/O or XML error immediately propagates up.
There is no partial-write recovery because:
- Writing is typically to a file or in-memory buffer.
- For file writes, the caller should write to a temp file and rename.
- The writer does not own the output — it borrows it.

### 10.3 Panic Prevention

The writer must **never panic**. All potential panic sources are guarded:

```rust
// ❌ Never do this:
// let xmi_id = self.id_map.resolve(id); // could panic if ID missing

// ✅ Instead, provide a safe fallback:
fn resolve_safe(&self, id: UmlId) -> String {
    self.id_map
        .get(&id)
        .cloned()
        .unwrap_or_else(|| format!("rs{:08x}", id.to_string()[..8]))
}
```

---

## 11. Round-Trip Test Plan

### 11.1 Test Strategy

Create `crates/uml-io/tests/test_round_trip.rs`:

1. **Load** `test/test-COG.xmi` into `UmlModel` via `XmiReader`.
2. **Write** the model to a `Vec<u8>` buffer via `XmiWriter`.
3. **Parse** the buffer back into a second `UmlModel` via `XmiReader`.
4. **Compare** the two models for semantic equivalence.

### 11.2 Semantic Equivalence Checks

```rust
/// Check that two models have the same structure.
fn assert_semantic_equivalence(original: &UmlModel, roundtripped: &UmlModel) {
    // 1. Same number of elements
    assert_eq!(
        original.len(),
        roundtripped.len(),
        "Element count mismatch"
    );

    // 2. Same number of classifiers by type
    for object_type in &[
        ObjectType::Class,
        ObjectType::Interface,
        ObjectType::Enumeration,
        ObjectType::Datatype,
        ObjectType::Package,
    ] {
        let orig_count = count_by_type(original, *object_type);
        let rt_count = count_by_type(roundtripped, *object_type);
        assert_eq!(
            orig_count, rt_count,
            "Mismatch count for {object_type}: original={orig_count}, rt={rt_count}"
        );
    }

    // 3. Same number of relationships by type
    for assoc_type in &[
        AssociationType::Generalization,
        AssociationType::Association,
        AssociationType::Aggregation,
        AssociationType::Composition,
        AssociationType::Dependency,
    ] {
        let orig_count = count_relationships_by_type(original, *assoc_type);
        let rt_count = count_relationships_by_type(roundtripped, *assoc_type);
        assert_eq!(
            orig_count, rt_count,
            "Mismatch count for relationship {assoc_type}: orig={orig_count}, rt={rt_count}"
        );
    }

    // 4. For each named classifier, check features
    //    (gather by name since IDs will differ for generated elements)
    for (orig_id, orig_elem) in original.iter() {
        if !orig_elem.is_classifier() {
            continue;
        }
        let name = orig_elem.name();

        // Find the same-named element in the roundtripped model
        let rt_elem = find_by_name(roundtripped, name)
            .unwrap_or_else(|| panic!("Element '{name}' not found in roundtripped model"));

        // Compare features
        let orig_cd = orig_elem.classifier_data().unwrap();
        let rt_cd = rt_elem.classifier_data().unwrap();

        assert_eq!(
            orig_cd.attributes.len(),
            rt_cd.attributes.len(),
            "Attribute count mismatch for '{name}': orig={}, rt={}",
            orig_cd.attributes.len(),
            rt_cd.attributes.len(),
        );

        assert_eq!(
            orig_cd.operations.len(),
            rt_cd.operations.len(),
            "Operation count mismatch for '{name}'",
        );

        // Check attribute names match (order should be preserved)
        for (i, orig_attr) in orig_cd.attributes.iter().enumerate() {
            assert_eq!(
                orig_attr.name,
                rt_cd.attributes[i].name,
                "Attribute name mismatch for '{name}' at index {i}"
            );
        }

        // Check operation names match
        for (i, orig_op) in orig_cd.operations.iter().enumerate() {
            assert_eq!(
                orig_op.name,
                rt_cd.operations[i].name,
                "Operation name mismatch for '{name}' at index {i}"
            );
            assert_eq!(
                orig_op.parameters.len(),
                rt_cd.operations[i].parameters.len(),
                "Parameter count mismatch for '{name}::{}'",
                orig_op.name,
            );
        }
    }

    // 5. Check that relationship endpoints resolve to same-named elements
    //    (since IDs differ, we compare by element name)
    for (_, rel) in original.iter().filter_map(|(_, e)| {
        if let ModelElement::Relationship(r) = e { Some(r) } else { None }
    }) {
        // Find the source and target names in original model
        let source_name = original.get(rel.source_id).map(|e| e.name().to_string());
        let target_name = original.get(rel.target_id).map(|e| e.name().to_string());

        // Find the same relationship type in roundtripped model between
        // same-named elements
        let found = roundtripped.iter().any(|(_, rt_elem)| {
            if let ModelElement::Relationship(rt_rel) = rt_elem {
                if rt_rel.kind != rel.kind {
                    return false;
                }
                let rt_source = roundtripped.get(rt_rel.source_id);
                let rt_target = roundtripped.get(rt_rel.target_id);
                rt_source.map(|s| s.name()) == source_name.as_deref()
                    && rt_target.map(|t| t.name()) == target_name.as_deref()
            } else {
                false
            }
        });
        assert!(
            found,
            "Relationship {}:({})→({}) not found in roundtripped model",
            rel.kind,
            source_name.unwrap_or_default(),
            target_name.unwrap_or_default(),
        );
    }
}
```

### 11.3 Test File

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use uml_core::UmlModel;
    use uml_io::xmi::reader::XmiReader;
    use uml_io::xmi::writer::XmiWriter;
    use uml_core::types::{AssociationType, ObjectType};

    const TEST_DIR: &str = "test";
    const TEST_FILES: &[&str] = &[
        "test-COG.xmi",
        // Add more as they become parseable:
        // "test-BVW.xmi",
        // "test-CDL.xmi",
        // "test-DCL.xmi",
    ];

    fn find_test_file(name: &str) -> String {
        let path = Path::new(TEST_DIR).join(name);
        assert!(path.exists(), "Test file not found: {}", path.display());
        path.to_string_lossy().to_string()
    }

    fn load_model(path: &str) -> UmlModel {
        let file = std::fs::File::open(path)
            .unwrap_or_else(|e| panic!("Failed to open {path}: {e}"));
        let mut model = UmlModel::new();
        let mut reader = XmiReader::new();
        reader
            .read_from(std::io::BufReader::new(file), &mut model)
            .unwrap_or_else(|e| panic!("Failed to parse {path}: {e}"));
        reader
            .resolve(&mut model)
            .unwrap_or_else(|e| panic!("Failed to resolve {path}: {e}"));
        model
    }

    fn write_model(model: &UmlModel) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer
            .write_document(model)
            .expect("Failed to write model to buffer");
        buffer
    }

    #[test]
    fn test_round_trip_cog() {
        let path = find_test_file("test-COG.xmi");
        let original = load_model(&path);
        let buffer = write_model(&original);
        let roundtripped = {
            let mut model = UmlModel::new();
            let mut reader = XmiReader::new();
            reader
                .read_from(std::io::BufReader::new(buffer.as_slice()), &mut model)
                .expect("Failed to parse round-tripped XMI");
            reader
                .resolve(&mut model)
                .expect("Failed to resolve round-tripped XMI");
            model
        };

        assert_semantic_equivalence(&original, &roundtripped);
    }

    #[test]
    fn test_output_is_valid_xml() {
        let model = UmlModel::new();
        let buffer = write_model(&model);
        // quick-xml can re-parse it
        let mut reader = quick_xml::Reader::from_reader(buffer.as_slice());
        let mut count = 0;
        let mut buf = Vec::new();
        loop {
            match reader.read_event_into(&mut buf) {
                Ok(quick_xml::events::Event::Eof) => break,
                Ok(_) => count += 1,
                Err(e) => panic!("Invalid XML output: {e}"),
            }
            buf.clear();
        }
        assert!(count > 0, "Output should have at least one XML event");
    }

    #[test]
    fn test_written_xmi_has_correct_structure() {
        let path = find_test_file("test-COG.xmi");
        let model = load_model(&path);
        let buffer = write_model(&model);
        let output = String::from_utf8(buffer).expect("Output should be valid UTF-8");

        // Check structural elements
        assert!(output.contains(r#"<?xml version="1.0" encoding="UTF-8"?>"#));
        assert!(output.contains(r#"xmi.version="1.2""#));
        assert!(output.contains(r#"<XMI.header>"#));
        assert!(output.contains(r#"<XMI.content>"#));
        assert!(output.contains(r#"<UML:Model"#));
        assert!(output.contains(r#"name="UML Model""#));
        assert!(output.contains(r#"<XMI.extensions"#));
        assert!(output.contains(r#"<docsettings"#));
        assert!(output.contains(r#"</XMI>"#));
    }

    // ── Helper functions ─────────────────────────────────────────────

    fn count_by_type(model: &UmlModel, object_type: ObjectType) -> usize {
        model
            .iter()
            .filter(|(_, e)| e.object_type() == object_type)
            .count()
    }

    fn count_relationships_by_type(model: &UmlModel, assoc_type: AssociationType) -> usize {
        model
            .iter()
            .filter(|(_, e)| {
                if let uml_core::ModelElement::Relationship(r) = e {
                    r.kind == assoc_type
                } else {
                    false
                }
            })
            .count()
    }

    fn find_by_name<'a>(model: &'a UmlModel, name: &str) -> Option<&'a uml_core::ModelElement> {
        model.iter().find(|(_, e)| e.name() == name).map(|(_, e)| e)
    }
}
```

### 11.4 Expected Counts for test-COG.xmi

| Metric | Expected |
|--------|----------|
| Packages | 5 (Logical View, Datatypes, Use Case View, Component View, Deployment View) |
| Classes | 4 (ClassA, ClassB, BaseClassA, ClassC) |
| Datatypes | 18 (int, char, bool, float, double, short, long, ...) |
| Generalizations | 1 (ClassA → BaseClassA) |
| Associations | 2 (ClassA↔ClassB, ClassC↔BaseClassA) |
| Total attributes | 9 (5 in ClassA, 2 in ClassB, 1 in BaseClassA, 2 in ClassC) |
| Total operations | 6 (3 in ClassA, 2 in ClassB, 1 in BaseClassA, 1 in ClassC) |

### 11.5 Running the Tests

```bash
# All round-trip tests
cargo test --package uml-io --test test_round_trip -- --nocapture

# With logging
RUST_LOG=debug cargo test --package uml-io --test test_round_trip -- --nocapture
```

---

## 12. Implementation Plan

### 12.1 Staged Implementation

| Stage | Description | Files | Acceptance Criteria |
|-------|-------------|-------|-------------------|
| **S1** | Skeleton + IdMap | `writer.rs`, `error.rs` | `XmiWriter::new()` compiles, `IdMap` build passes |
| **S2** | Document scaffold | XML declaration, XMI root, header, extensions | Output has correct structure, tests pass |
| **S3** | Classifier writing | `write_classifier`, features (attr/op/param) | test-COG.xmi round-trips with correct feature counts |
| **S4** | Package hierarchy | `write_package`, recursion | Nested packages written correctly |
| **S5** | Relationship writing | Generalization, Association, Dependency, Abstraction | All relationship types round-trip |
| **S6** | Round-trip tests | `test_round_trip.rs` | All assertions pass on test-COG.xmi |
| **S7** | Real corpus tests | Extend to all parseable test XMI files | All files round-trip without data loss |

### 12.2 Merge Strategy

| Merge Point | Contents | Dependencies |
|-------------|----------|--------------|
| Merge 1 | S1 + S2 (skeleton produces valid XMI for empty model) | M9 reader |
| Merge 2 | S3 + S4 (full model content for classifiers and packages) | Merge 1 |
| Merge 3 | S5 + S6 (relationships + round-trip tests) | Merge 2 |
| Merge 4 | S7 (corpus round-trip for all files) | Merge 3 |

### 12.3 Estimating Effort

| Stage | Estimated Lines | Complexity | Hours |
|-------|----------------|------------|-------|
| S1 | 100 | Low | 1 |
| S2 | 150 | Low | 2 |
| S3 | 350 | Medium | 4 |
| S4 | 100 | Low | 1 |
| S5 | 300 | Medium | 3 |
| S6 | 250 | Medium | 3 |
| S7 | 100 | Low | 1 |
| **Total** | **~1350** | | **~15h** |

---

## 13. References

### XMI Format

- [OMG XMI 1.2 Specification](https://www.omg.org/spec/XMI/1.2/) — formal OMG document
- [UML 1.3 Specification](https://www.omg.org/spec/UML/1.3/) — UML 1.3, which XMI 1.2 supports
- Legacy C++ output: `test/test-COG.xmi` — our reference output format

### Related Documents

| Document | Relationship |
|----------|-------------|
| `docs/xmi_persistence_architecture_v1.md` | Reader architecture, IdMap design (reading direction), format analysis |
| `docs/xmi_milestone9_strategy.md` | Reader feature/relationship parsing, corpus tests |
| `docs/domain_model_v1.md` | Domain model design decisions |
| `docs/workspace_consolidation_v2.md` | Crate layout, module placement rationale |

### C++ Reference

The C++ serialization code is in the legacy codebase:

| File | Responsibility |
|------|---------------|
| `umbrello/umlmodel.cpp` | `saveToXMI()` — top-level XMI writing orchestration |
| `umbrello/umlobject.cpp` | `saveToXMI()` per-element base attributes |
| `umbrello/umlclassifier.cpp` | `saveToXMI()` — features, generalizations |
| `umbrello/umlpackage.cpp` | `saveToXMI()` — owned elements |
| `umbrello/association.cpp` | `saveToXMI()` — relationship serialization |
| `umbrello/attribute.cpp` | `saveToXMI()` — attribute serialization |
| `umbrello/operation.cpp` | `saveToXMI()` — operation + parameter serialization |
| `umbrello/umldoc.cpp` | `saveToXMI()` — docsettings, extensions |

---

## Appendix A: Full Output Example

For a model loaded from `test-COG.xmi` and written back, the output should
follow this structure (abbreviated):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<XMI verified="false" xmi.version="1.2" timestamp="2026-06-23T12:00:00Z" xmlns:UML="http://schema.omg.org/spec/UML/1.3">
 <XMI.header>
  <XMI.documentation>
   <XMI.exporter>umbrello uml modeller http://umbrello.kde.org</XMI.exporter>
   <XMI.exporterVersion>1.5.8</XMI.exporterVersion>
   <XMI.exporterEncoding>UnicodeUTF8</XMI.exporterEncoding>
  </XMI.documentation>
  <XMI.metamodel xmi.version="1.3" href="UML.xml" xmi.name="UML"/>
 </XMI.header>
 <XMI.content>
  <UML:Model isSpecification="false" isAbstract="false" isLeaf="false" xmi.id="m1" isRoot="false" name="UML Model">
   <UML:Namespace.ownedElement>
    <!-- Stereotypes (deferred) -->
    <!-- Packages -->
    <UML:Package visibility="public" xmi.id="Logical View" isSpecification="false" isLeaf="false" isRoot="false" name="Logical View">
     <UML:Namespace.ownedElement>
      <UML:Package visibility="public" xmi.id="Datatypes" isSpecification="false" isLeaf="false" isRoot="false" name="Datatypes">
       <UML:Namespace.ownedElement>
        <UML:DataType visibility="public" xmi.id="kyEtsyFPGmQH" isSpecification="false" isLeaf="false" isRoot="false" name="int"/>
        <UML:DataType visibility="public" xmi.id="8mnPF0tnVVYf" isSpecification="false" isLeaf="false" isRoot="false" name="char"/>
        <!-- ... more datatypes ... -->
       </UML:Namespace.ownedElement>
      </UML:Package>

      <!-- Classes -->
      <UML:Class visibility="public" xmi.id="sGBeu79qqOiF" isSpecification="false" isLeaf="false" isRoot="false" name="ClassA">
       <UML:GeneralizableElement.generalization>
        <UML:Generalization xmi.idref="XznwGhvpCws8"/>
       </UML:GeneralizableElement.generalization>
       <UML:Classifier.feature>
        <UML:Attribute visibility="protected" isSpecification="false" xmi.id="attr_0" type="jfBF3PIuZOVp" name="m_attrA1"/>
        <UML:Attribute visibility="private" isSpecification="false" xmi.id="attr_1" type="ksJknVIVTUOM" name="m_attrA2"/>
        <!-- ... more attributes ... -->
        <UML:Operation visibility="public" isSpecification="false" isQuery="false" isAbstract="false" isLeaf="false" isRoot="false" xmi.id="op_0" name="start"/>
        <UML:Operation visibility="public" isSpecification="false" isQuery="false" isAbstract="false" isLeaf="false" isRoot="false" xmi.id="op_1" name="status">
         <UML:BehavioralFeature.parameter>
          <UML:Parameter kind="return" xmi.id="param_0" type="ksJknVIVTUOM"/>
         </UML:BehavioralFeature.parameter>
        </UML:Operation>
        <!-- ... more operations ... -->
       </UML:Classifier.feature>
      </UML:Class>

      <!-- ... more classes ... -->

      <!-- Associations -->
      <UML:Association visibility="public" isSpecification="false" xmi.id="wsbAbEUp0bG7" name="">
       <UML:Association.connection>
        <UML:AssociationEnd changeability="changeable" visibility="private" isNavigable="false" isSpecification="false" xmi.id="wsbAbEUp0bG7_end1" type="sGBeu79qqOiF" name="" aggregation="none"/>
        <UML:AssociationEnd changeability="changeable" visibility="private" isNavigable="true" isSpecification="false" xmi.id="wsbAbEUp0bG7_end2" type="jfBF3PIuZOVp" name="" aggregation="none"/>
       </UML:Association.connection>
      </UML:Association>

      <!-- Generalizations (standalone) -->
      <UML:Generalization discriminator="" visibility="public" isSpecification="false" child="sGBeu79qqOiF" xmi.id="tvReBia5Mo10" parent="XznwGhvpCws8" name=""/>

      <!-- ... more associations ... -->
     </UML:Namespace.ownedElement>
    </UML:Package>

    <!-- Other views (Use Case, Component, Deployment, Entity Relationship) -->
    <UML:Model stereotype="folder" visibility="public" xmi.id="Use Case View" name="Use Case View">
     <UML:Namespace.ownedElement/>
    </UML:Model>
    <!-- ... -->
   </UML:Namespace.ownedElement>
  </UML:Model>
 </XMI.content>
 <XMI.extensions xmi.extender="umbrello">
  <docsettings viewid="" uniqueid="aB3dE5fG7hI9" documentation=""/>
 </XMI.extensions>
</XMI>
```

**Key differences from the original file (expected and acceptable):**

1. **Stereotype elements** are missing (deferred to M11).
2. **Diagram data** is missing (deferred to M12+).
3. **Feature XMI IDs** are regenerated (`attr_0`, `op_1`, `param_0` instead of
   original `Pm8KwN2qa8F1`).
4. **AssociationEnd IDs** are generated from the association ID + suffix.
5. **Attributes `xmi.idref`** appear as `xmi.idref="XznwGhvpCws8"` (the
   original XMI ID of BaseClassA, preserved because `original_xmi_id` is set).

---

## Appendix B: Implementation Notes

### B.1 quick-xml Writer API

`quick_xml::Writer<W>` provides:

```rust
impl<W: Write> Writer<W> {
    pub fn new(inner: W) -> Self;
    pub fn new_with_indent(inner: W, indent_char: u8, indent_size: usize) -> Self;

    pub fn write_event<'a>(&mut self, event: Event<'a>) -> Result<(), Error>;

    pub fn into_inner(self) -> W;
    pub fn get_ref(&self) -> &W;
    pub fn get_mut(&mut self) -> &mut W;
}
```

Events to use:
- `Event::Decl(BytesDecl)` — for `<?xml ... ?>`
- `Event::Start(BytesStart)` — for `<tag ...>`
- `Event::End(BytesEnd)` — for `</tag>`
- `Event::Empty(BytesStart)` — for `<tag ... />`
- `Event::Text(BytesText)` — for text content inside elements

Building start/empty tags with attributes:
```rust
let mut start = BytesStart::new("UML:Class");
start.push_attribute(("xmi.id", "abc123"));
start.push_attribute(("name", "MyClass"));
```

### B.2 Timestamp Format

The C++ Umbrello uses `QDateTime::toString(Qt::ISODate)` which produces
`"2010-04-08T22:51:39"`. Our writer should produce the same ISO 8601 format
without timezone suffix (local time) or with `Z` for UTC:

```rust
fn current_timestamp() -> String {
    // Simple implementation using chrono if available, or fallback
    // "2026-06-23T12:00:00Z" format
    format!("2026-06-23T12:00:00Z") // placeholder — use actual time
}
```

The timestamp appears on the `<XMI>` root element as `timestamp="..."`.

### B.3 Handling Elements Not Yet in the Model

Some XMI elements visible in the reference files are not yet stored as first-
class domain objects, and therefore are not written:

| Missing element | Impact | Resolution |
|----------------|--------|------------|
| `UML:Stereotype` | Not written; elements with `stereotype_id` are written without the `stereotype` attribute | M11: stereotype registry |
| Diagram data | Not written; extensions section minimal | M12+: diagram persistence |
| Enum literals | Enum `literals` field exists but no writing code yet | Add during M10 or M11 |
| `UML:Model` views | Written as packages (correct) | Already handled |

### B.4 Writer Tests (Unit)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use uml_core::elements::*;
    use uml_core::repository::UmlModel;
    use uml_core::types::*;

    #[test]
    fn write_empty_model() {
        let model = UmlModel::new();
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("<XMI"));
        assert!(output.contains("</XMI>"));
    }

    #[test]
    fn write_single_class() {
        let mut model = UmlModel::new();
        let cls = ModelElement::Class(Class::new("MyClass"));
        model.insert(cls);
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains(r#"name="MyClass""#));
        assert!(output.contains("UML:Class"));
    }

    #[test]
    fn write_class_with_attributes() {
        let mut model = UmlModel::new();
        let mut cls = Class::new("MyClass");
        cls.classifier.add_attribute(Attribute {
            name: "myField".into(),
            type_ref: TypeReference::primitive("int"),
            visibility: Visibility::Private,
            initial_value: Some("42".into()),
            is_static: false,
        });
        let id = cls.base.id;
        model.insert(ModelElement::Class(cls));
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains(r#"name="myField""#));
        assert!(output.contains(r#"type="int""#));
    }

    #[test]
    fn write_class_with_operation() {
        let mut model = UmlModel::new();
        let mut cls = Class::new("Calculator");
        cls.classifier.add_operation(Operation {
            name: "add".into(),
            return_type: TypeReference::primitive("int"),
            parameters: vec![
                Parameter {
                    name: "a".into(),
                    type_ref: TypeReference::primitive("int"),
                    direction: ParameterDirection::In,
                    default_value: None,
                },
                Parameter {
                    name: "b".into(),
                    type_ref: TypeReference::primitive("int"),
                    direction: ParameterDirection::In,
                    default_value: None,
                },
            ],
            visibility: Visibility::Public,
            is_static: false,
            is_abstract: false,
            is_virtual: false,
        });
        model.insert(ModelElement::Class(cls));
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains(r#"name="add""#));
        assert!(output.contains(r#"kind="return""#));
        assert!(output.contains(r#"name="a""#));
        assert!(output.contains(r#"name="b""#));
    }

    #[test]
    fn write_package_with_class() {
        let mut model = UmlModel::new();
        let pkg = ModelElement::Package(Package::new("MyPackage"));
        let pkg_id = pkg.id();
        model.insert(pkg);
        let cls = ModelElement::Class(Class::new("MyClass"));
        let cls_id = cls.id();
        model.insert(cls);
        model.add_to_package(pkg_id, cls_id).unwrap();
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains(r#"name="MyPackage""#));
        assert!(output.contains(r#"name="MyClass""#));
        // MyClass should be inside MyPackage's ownedElement
        let pkg_start = output.find(r#"name="MyPackage""#).unwrap();
        let cls_start = output.find(r#"name="MyClass""#).unwrap();
        assert!(cls_start > pkg_start, "Class should appear after Package");
    }

    #[test]
    fn write_generalization() {
        let mut model = UmlModel::new();
        let sub = ModelElement::Class(Class::new("SubClass"));
        let sub_id = sub.id();
        model.insert(sub);
        let sup = ModelElement::Class(Class::new("SuperClass"));
        let sup_id = sup.id();
        model.insert(sup);
        let rel = ModelElement::Relationship(
            Relationship::new_generalization(sub_id, sup_id),
        );
        model.insert(rel);
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("UML:Generalization"));
        assert!(output.contains("child="));
        assert!(output.contains("parent="));
    }

    #[test]
    fn write_association() {
        let mut model = UmlModel::new();
        let a = ModelElement::Class(Class::new("ClassA"));
        let a_id = a.id();
        model.insert(a);
        let b = ModelElement::Class(Class::new("ClassB"));
        let b_id = b.id();
        model.insert(b);
        let mut rel = Relationship::new_association(a_id, b_id);
        rel.source_to_target_navigable = true;
        let rel = ModelElement::Relationship(rel);
        model.insert(rel);
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("UML:Association"));
        assert!(output.contains("UML:AssociationEnd"));
        assert!(output.contains("UML:Association.connection"));
    }

    #[test]
    fn write_dependency() {
        let mut model = UmlModel::new();
        let client = ModelElement::Class(Class::new("ClientClass"));
        let client_id = client.id();
        model.insert(client);
        let supplier = ModelElement::Class(Class::new("SupplierClass"));
        let supplier_id = supplier.id();
        model.insert(supplier);
        let rel = ModelElement::Relationship(
            Relationship::new_dependency(client_id, supplier_id),
        );
        model.insert(rel);
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("UML:Dependency"));
        assert!(output.contains("supplier="));
        assert!(output.contains("client="));
    }

    #[test]
    fn preserve_original_xmi_id() {
        let mut model = UmlModel::new();
        let mut cls = Class::new("LegacyClass");
        cls.base.original_xmi_id = Some("O0JJV24XoKdQ".into());
        model.insert(ModelElement::Class(cls));
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains(r#"xmi.id="O0JJV24XoKdQ""#));
    }

    #[test]
    fn generate_id_for_new_elements() {
        let mut model = UmlModel::new();
        model.insert(ModelElement::Class(Class::new("NewClass")));
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        // New elements get rs-prefixed IDs
        assert!(output.contains(r#"xmi.id="rs"#));
    }

    #[test]
    fn write_datatype() {
        let mut model = UmlModel::new();
        model.insert(ModelElement::Datatype(Datatype::new("int")));
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("UML:DataType"));
        assert!(output.contains(r#"name="int""#));
    }

    #[test]
    fn write_enum() {
        let mut model = UmlModel::new();
        let mut enm = Enum::new("Color");
        enm.add_literal("Red", Some("0".into()));
        enm.add_literal("Green", Some("1".into()));
        enm.add_literal("Blue", None);
        model.insert(ModelElement::Enum(enm));
        let mut buffer = Vec::new();
        let mut writer = XmiWriter::new(&mut buffer);
        writer.write_document(&model).unwrap();
        let output = String::from_utf8(buffer).unwrap();
        assert!(output.contains("UML:Enumeration"));
        assert!(output.contains(r#"name="Color""#));
        // Literals are written inside UML:Enumeration (deferred)
    }
}
```
