# Persistence / Serialization / XMI Analysis — Umbrello to Rust rewrite

Date: 2026-06-23  
Author: Rust rewrite team  
Status: Draft

---

## 1. Save/Load Pipeline Analysis

### 1.1 Save Pipeline

The save pipeline is triggered through the UI (`File → Save`) and proceeds as follows:

```
User clicks Save
    → UMLApp::slotFileSave()           [umbrello/umlapp.cpp]
        → UMLDoc::saveDocument(QUrl)   [umbrello/umldoc.cpp:808]
            → if .xmi.tgz / .xmi.tar.bz2:
                → KTar archive creation
                → saveToXMI(tmp_xmi_file)  [write XMI to temp file]
                → archive->addLocalFile()  [insert XMI into archive]
                → if remote: KIO::copy()
            → if .xmi (plain):
                → QTemporaryFile
                → saveToXMI(tmpfile)        [umldoc.cpp:2064]
                → if remote: KIO::copy()
                → if local: QFile::rename()
```

The core serialization method `UMLDoc::saveToXMI()` (umldoc.cpp:2064) uses `QXmlStreamWriter` for streaming XML output:

```
saveToXMI(QIODevice &file)
  → QXmlStreamWriter writer(&file)
  → writer.setAutoFormatting(true)
  → if uml2: write xmi:XMI root (xmi:version="2.1")
      else:  write XMI root (xmi.version="1.2")
  → write XMI header/documentation/metamodel (UML1.2 only)
  → write UML:Model / uml:Model element
  → write Stereotypes (saved first so they are known on load)
  → write Root Folders via UMLFolder::saveToXMI() (recursive)
      → each folder saves its child UMLObjects
      → each folder saves its diagrams (XMI.extension → <diagrams>)
  → write docsettings (viewid, documentation, uniqueid)
  → write ListView state
  → write CodeGenerator state
  → close all elements
```

**Key observations:**
- Uses streaming writer (`QXmlStreamWriter`) — good for large files
- XMI is written to a **temporary file first** then atomically renamed to the target path (prevents corruption)
- Stereotypes are saved first so they're available on load
- Remote files use KIO::copy() for upload

### 1.2 Load Pipeline

```
User clicks Open
    → UMLApp::slotFileOpen()           [umbrello/umlapp.cpp]
        → UMLDoc::openDocument(QUrl)   [umbrello/umldoc.cpp:602]
            → closeDocument()  (clear previous state)
            → m_bLoading = true (disables undo recording)
            → KIO::copy(url → tempfile)
            → Determine file type:
                .xmi          → loadFromXMI(file, ENC_UNKNOWN)
                .xmi.tgz      → KTar → extract .xmi → loadFromXMI()
                .xmi.tar.bz2  → KTar → extract .xmi → loadFromXMI()
                .mdl          → Import_Rose::loadFromMDL()
                .zargo        → Import_Argo::loadFromZArgoFile()
            → m_bLoading = false
            → post-load: setModified(false), initSaveTimer()
            → post-load: checkAndFixFileAfterLoad()
                → checkAssociationWidgetsAfterLoad() — creates missing widgets
```

Core load method `UMLDoc::loadFromXMI()` (umldoc.cpp:2257) uses **DOM** (not streaming):

```
loadFromXMI(QIODevice &file, short encode)
  → Detect encoding (encoding()) by checking XML processing instruction
  → QTextStream → readAll() → QDomDocument::setContent()
  → Determine XMI version from root element
  → PASS 1 (loadUMLObjectsFromXMI):
      → Walk DOM tree, identify <UML:Model> / <packagedElement>
      → Recurse into <Namespace.ownedElement>
      → For each element with xmi.id/xmi:id:
          → Object_Factory::makeObjectFromXMI(tag, stID) — determine type
          → pObject->loadFromXMI(element) — load attributes
          → Add to parent package / datatype folder / stereotype list
  → resolveTypes() — resolve forward references
      → Recursively calls resolveRef() on all objects
  → loadDiagrams1() — deferred diagram loading (PASS 3)
  → activateAllViews() — adjust widgets after loading
  → Restore last-viewed diagram
```

**Critical architectural decision:** The entire XMI file is read into memory as a DOM tree (`QDomDocument`). For large files this is a significant memory concern (the Rust rewrite should use streaming/SAX-style parsing).

### 1.3 Loading Phases in Detail

| Phase | Method | Purpose |
|-------|--------|---------|
| PASS 1 | `loadUMLObjectsFromXMI()` | Load all `UMLObject` subclasses (classifiers, packages, attributes, operations, stereotypes, etc.) into the object tree |
| PASS 2 | `resolveTypes()` → `resolveRef()` | Resolve forward references (deferred because objects may reference IDs defined later in the file) |
| PASS 3 | `loadDiagrams1()` | Load UML views/diagrams (deferred because widgets reference objects loaded in PASS 1) |
| PASS 4 | `loadExtensionsFromXMI1()` | Load listview state and codegenerator state |
| Post | `activateAllViews()` | Activate after-load adjustments on widgets |
| Post | `checkAndFixFileAfterLoad()` | Create missing association widgets for orphaned UMLAssociation objects |

**Disabling Undo During Load:** `m_bLoading` flag prevents any `setModified()` calls from pushing commands onto the undo stack during file loading. After load, `clearUndoStack()` is called.

---

## 2. XMI Format Analysis

### 2.1 XMI 1.2 (Default)

```
<?xml version="1.0" encoding="UTF-8"?>
<XMI xmi.version="1.2" timestamp="..." verified="false"
     xmlns:UML="http://schema.omg.org/spec/UML/1.4">
  <XMI.header>
    <XMI.documentation>
      <XMI.exporter>umbrello uml modeller ...</XMI.exporter>
      <XMI.exporterVersion>1.7.0</XMI.exporterVersion>
      <XMI.exporterEncoding>UnicodeUTF8</XMI.exporterEncoding>
    </XMI.documentation>
    <XMI.metamodel xmi.name="UML" xmi.version="1.4" href="UML.xml"/>
  </XMI.header>
  <XMI.content>
    <UML:Model xmi.id="m1" name="..." isSpecification="false"
               isAbstract="false" isRoot="false" isLeaf="false">
      <UML:Namespace.ownedElement>
        <!-- Models, Packages, Classes, Stereotypes, etc. -->
      </UML:Namespace.ownedElement>
    </UML:Model>
  </XMI.content>
  <XMI.extensions xmi.extender="umbrello">
    <docsettings viewid="..." documentation="..." uniqueid="..."/>
    <diagrams>
      <!-- UML views → UMLScene → widgets -->
    </diagrams>
    <listview>
      <!-- ListView tree state -->
    </listview>
    <codegeneration>
      <!-- Code generator state -->
    </codegeneration>
  </XMI.extensions>
</XMI>
```

**ID attribute:** `xmi.id`  
**Namespace prefix:** `UML:` (e.g., `UML:Class`, `UML:Interface`, `UML:Operation`)

### 2.2 XMI 2.1 (Optional)

```
<?xml version="1.0" encoding="UTF-8"?>
<xmi:XMI xmi:version="2.1"
     xmlns:xmi="http://schema.omg.org/spec/XMI/2.1"
     xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
     xmlns:uml="http://schema.omg.org/spec/UML/2.1">
  <xmi:Documentation exporter="..." exporterVersion="2.0.0"/>
  <uml:Model xmi:id="m1" name="...">
    <!-- packagedElement instead of Namespace.ownedElement -->
    <packagedElement xmi:type="uml:Class" xmi:id="..." name="..."/>
    <packagedElement xmi:type="uml:Package" xmi:id="..." name="..."/>
  </uml:Model>
  <xmi:Extension extender="umbrello">
    <docsettings .../>
    <listview .../>
    <codegeneration .../>
  </xmi:Extension>
</xmi:XMI>
```

**ID attribute:** `xmi:id`  
**Namespace prefix:** `uml:`  
**Key structural difference:** Uses `<packagedElement xmi:type="uml:*">` instead of `<UML:*>`  
**No `<XMI.header>` / `<XMI.content>` envelope**

### 2.3 Version Selection Logic

Toggled by `Settings::optionState().generalState.uml2`:

```cpp
if (Settings::optionState().generalState.uml2) {
    // write XMI 2.1
} else {
    // write XMI 1.2
}
```

On load, version is auto-detected from the root element attribute `xmi.version` / `xmi:version`:

```cpp
QString versionString = root.attribute("xmi.version");
if (versionString.isEmpty())
    versionString = root.attribute("xmi:version");
if (version >= 2.0)
    Settings::optionState().generalState.uml2 = true;
```

---

## 3. Serialization Pattern Analysis

### 3.1 The save1/load1 Pattern

Every `UMLObject` subclass implements serialization via two helper methods on `UMLObject`:

**Save path:**
```
UMLObject::saveToXMI(QXmlStreamWriter&)       // subclass override (often empty)
  → UMLObject::save1(writer, type, tag)       // writes opening element + common attrs
  → [subclass writes specific child elements]
  → UMLObject::save1end(writer)               // writes tagged values + closing element
```

**`save1()`** (umlobject.cpp:861):
- Opens XML element with appropriate namespace prefix
- Writes common attributes: id, name, visibility, stereotype, isAbstract, ownerScope
- Adapts to UML1 vs UML2 mode:
  - UML1: `<UML:Class xmi.id="...">`
  - UML2: `<packagedElement xmi:type="uml:Class" xmi:id="...">`
- Uses `<use_type_as_tag>` special value for tag parameter — uses type as element name directly (for root folders, etc.)

**`save1end()`** (umlobject.cpp:937):
- Writes `<UML:ModelElement.taggedValues>` if stereotype attributes exist
- Writes closing element

**Load path:**
```
UMLObject::loadFromXMI(QDomElement&)          // base class — reads common attrs
  → load1(QDomElement&)                        // subclass override — reads specific data
```

**`loadFromXMI()`** (umlobject.cpp:1027):
- Reads `name`, `xmi.id`/`xmi:id`, `documentation`/`comment`
- Reads `visibility`, `stereotype`, `isAbstract`, `ownerScope`
- If children exist, iterates sub-elements for:
  - `ModelElement.taggedValues`
  - `name`, `visibility`, `isAbstract`, `ownerScope` (as child elements, alternative to attributes)
  - `ownedComment`
  - stereotype references
- Calls `load1(element)` at the end for subclass-specific handling

### 3.2 Strengths of Current Pattern

- **Clean separation of concerns:** Common attributes handled by base class, specific data by subclass
- **XMI version adaptation transparent:** `save1()` abstracts UML1/UML2 differences
- **Robust backward compatibility:** `loadFromXMI()` handles multiple attribute formats (direct attributes, child elements, old scope encoding)
- **Error resilience:** Missing IDs trigger ID generation rather than failure (best-effort loading)
- **Extensible:** New object types simply override `saveToXMI()` and `load1()`

### 3.3 Weaknesses of Current Pattern

- **DOM-based loading:** Entire file is parsed into memory (`QDomDocument`) — problematic for large models
- **No streaming load:** `QXmlStreamReader` could be more memory-efficient
- **XMI version logic is scattered:** Every `save1()` call checks `Settings::optionState().generalState.uml2` — should be centralized
- **No schema validation at load time:** DTD files exist but are not used for validation
- **Hard-coded namespace string construction:** `nmSpc + ":" + type` pattern is fragile
- **Forward reference resolution is ad-hoc:** Uses string IDs stored in `m_SecondaryId` — works but could be more type-safe
- **No serialization of transient state:** Some UI state is saved in XMI extensions (listview, docsettings) mixing model and view concerns

---

## 4. Forward Reference Resolution Mechanism

### 4.1 How Forward References Work

XMI files may reference model objects by ID before those objects are defined. Umbrello handles this via a two-pass approach:

**Pass 1 — Load:** Every `UMLObject` stores unresolved references as string IDs.
- Base class `UMLObject` has `m_SecondaryId` (QString) — stores type reference (e.g., attribute type)
- Subclasses may store additional reference IDs in their own members
- The `loadStereotype()` method (umlobject.cpp:986) stores unresolved stereotype refs in `m_SecondaryId`
- Other subclasses (like `UMLAttribute`) store their type reference similarly

**Pass 2 — Resolve:** `UMLDoc::resolveTypes()` → recursive `resolveRef()` on all objects
- Called after all objects are loaded but before diagrams are loaded
- Uses `UMLDoc::findObjectById()` to resolve string IDs to actual `UMLObject` pointers
- Has fallback chain:
  1. Lookup by ID via `findObjectById()`
  2. If found and it's a stereotype, set as stereotype
  3. If not found but `m_SecondaryFallback` exists, try by name lookup
  4. If name contains `::`, use `Import_Utils::createUMLObject()` (on-the-fly creation)
  5. As last resort, create a new type (C++-specific heuristics for pointer/reference types)
- Subclasses override `resolveRef()` for additional resolution (e.g., `UMLRole` resolves role type)

### 4.2 Resolution Order

1. `UMLDoc::resolveTypes()` (umldoc.cpp:2495)
2. Called from `loadFromXMI()` after `loadUMLObjectsFromXMI()` completes
3. Also called from `loadExtensionsFromXMI1()` before loading listview (listview needs resolved objects)
4. Diagrams are loaded *after* resolution (`loadDiagrams1()`)
5. Widgets reference resolved objects at diagram load time

### 4.3 Thread Safety

**Not thread-safe.** The resolution process modifies live objects and can trigger UI updates (`qApp->processEvents()` in `resolveRef()`).

---

## 5. File Format and Compression Support

### 5.1 Supported Formats

| Extension | Format | Read | Write |
|-----------|--------|------|-------|
| `.xmi` | Plain XML | Yes | Yes |
| `.xmi.tgz` | GZip-compressed tar | Yes | Yes |
| `.xmi.tar.gz` | GZip-compressed tar | Yes | No |
| `.xmi.tar.bz2` | BZip2-compressed tar | Yes | Yes |
| `.bak.xmi` | Backup — plain XML | Yes | Yes |
| `.bak.xmi.tgz` | Backup — gzip tar | Yes | Yes |
| `.bak.xmi.tar.bz2` | Backup — bzip2 tar | Yes | Yes |
| `.zargo` | ArgoUML ZIP | Yes (import only) | No |
| `.mdl` | Rational Rose | Yes (import only) | No |

### 5.2 Compression Handling (Save)

```cpp
// umldoc.cpp:808 — saveDocument()
if (fileExt == "xmi.tgz" || fileExt == "bak.xmi.tgz") {
    KTar archive(url, "application/x-gzip");
    archive->open(QIODevice::WriteOnly);
    QTemporaryFile tmp_xmi;
    saveToXMI(tmp_xmi);  // write XMI to temp file
    archive->addLocalFile(tmp_xmi, name_without_ext);
    archive->close();
} else if (fileExt == "xmi.tar.bz2" || ...) {
    KTar archive(url, "application/x-bzip");
    // same pattern
} else {
    // plain .xmi — save directly to temp, then rename
    QTemporaryFile tmpfile;
    saveToXMI(tmpfile);
    tmpfile.rename(target);  // atomic replace
}
```

### 5.3 Compression Handling (Load)

```cpp
// umldoc.cpp:602 — openDocument()
if (filetype.endsWith(".tgz") || filetype.endsWith(".tar.gz")) {
    KTar archive(file, "application/x-gzip");
} else if (filetype.endsWith(".tar.bz2")) {
    KTar archive(file, "application/x-bzip");
}
// Archive extraction: walk entries → find application/x-uml mime type
// → extract to temp dir → loadFromXMI(extracted_file)
```

### 5.4 Atomic Save Pattern

For plain `.xmi` files, the save always writes to a **temporary file** first:
1. Write to `QTemporaryFile` (auto-remove = false)
2. If remote: `KIO::copy(temp → url)`
3. If local: `QFile::rename(temp → url)` (overwrite original)

This prevents file corruption if the save crashes mid-write.

---

## 6. DTD/Schema Analysis

### 6.1 Available DTDs

Located in `doc/xml/`:

| DTD File | Purpose |
|----------|---------|
| `uml-1.4-umbrello.dtd` | Main UML 1.4 model DTD (customized for Umbrello) |
| `umbrello-diagrams.dtd` | Diagram/widget elements |
| `umbrello-misc.dtd` | Miscellaneous — listview, codegeneration |
| `uml241.dtd` | UML 2.4.1 DTD (for UML 2.1 XMI output) |
| `01-02-16.dtd` | OMG XMI 1.2 shared DTD |

### 6.2 DTD Coverage

The DTDs define the structure of:
- **UML model elements** (Class, Interface, Association, Generalization, etc.)
- **Diagram elements** (widget types, coordinates, colors, fonts)
- **Listview state** (tree structure for the model browser)
- **Code generation state** (language, active code generator)

### 6.3 DTD Usage in Current Code

**DTDs are NOT used for validation at load time.** They exist as documentation only. The load pipeline never invokes DTD validation. If validation were desired, `QDomDocument::setContent()` supports DTD validation via the `bool validate` parameter (currently passed as `false`).

### 6.4 Schema Validation Gap

- No XSD (XML Schema Definition) files exist
- No DTD validation during load
- Best-effort error recovery instead: missing/unknown tags are silently skipped
- Some structural validation exists ad-hoc (e.g., checking `xmi.version` value, checking for required attributes)

---

## 7. Undo/Redo Architecture Analysis

### 7.1 Stack Management

- `QUndoStack*` owned by `UMLApp` (`umlapp.cpp`)
- `setUndoEnabled(bool)` flag — disabled during file loading
- `clearUndoStack()` — called after new document / load document
- Maximum undo count configurable via KConfig

### 7.2 Command Hierarchy

```
QUndoCommand
├── CmdBaseObjectCommand            (2 commands)
│   ├── CmdCreateUMLObject          [generic/cmdcreateumlobject]
│   └── CmdRemoveUMLObject          [generic/cmdremoveumlobject]
├── CmdBaseWidgetCommand            (14+ commands)
│   ├── CmdCreateWidget             [widget/cmdcreatewidget]
│   ├── CmdRemoveWidget             [widget/cmdremovewidget]
│   ├── CmdMoveWidget               [widget/cmdmovewidget]
│   ├── CmdResizeWidget             [widget/cmdresizewidget]
│   ├── CmdChangeFillColor          [widget/cmdchangefillcolor]
│   ├── CmdChangeLineColor          [widget/cmdchangelinecolor]
│   ├── CmdChangeTextColor          [widget/cmdchangetextcolor]
│   ├── CmdChangeFont               [widget/cmdchangefont]
│   ├── CmdChangeLineWidth          [widget/cmdchangelinewidth]
│   ├── CmdChangeUseFillColor       [widget/cmdchangeusefillcolor]
│   ├── CmdChangeVisualProperty     [widget/cmdchangevisualproperty]
│   ├── CmdChangeMultiplicity       [widget/cmdchangemultiplicity]
│   ├── CmdSetName                  [widget/cmdsetname]
│   └── CmdSetTxt                   [widget/cmdsettxt]
├── CmdRenameUMLObject              [generic/cmdrenameumlobject]
├── CmdCreateDiagram                [cmdcreatediagram]
├── CmdRemoveDiagram                [cmdremovediagram]
├── CmdSetStereotype                [cmdsetstereotype]
└── CmdSetVisibility                [cmdsetvisibility]
```

Total: ~20 command types in `umbrello/cmds/`

### 7.3 Command Execution

```cpp
void UMLApp::executeCommand(QUndoCommand* cmd) {
    if (isUndoEnabled()) {
        m_pUndoStack->push(cmd);   // QUndoStack calls cmd->redo() internally
        enableUndoAction(true);
    } else {
        cmd->redo();               // execute without pushing to stack
        delete cmd;
    }
}
```

### 7.4 Macro Grouping

Commands can be grouped for atomic undo/redo:

```cpp
UMLApp::app()->beginMacro("Create UML object : ClassName");
// ... push multiple commands ...
UMLApp::app()->endMacro();
```

Used extensively for composite operations (e.g., creating a UML object + creating its widget simultaneously).

### 7.5 Limitations of Current System

- **Object identity:** Commands store object IDs and use `QPointer<UMLObject>` — fragile if objects are deleted
- **Snapshot-based:** Some commands save/restore entire property sets rather than delta
- **No model/view separation:** Commands directly manipulate both model objects and widgets
- **No serialization:** Undo history is purely in-memory, not persisted

---

## 8. Foreign Format Import Analysis

### 8.1 ArgoUML Import (`Import_Argo`)

File: `umbrello/import_argo.cpp` (150 lines)

- Handles `.zargo` files (ArgoUML's ZIP-based format)
- Uses `KZip` to open the archive
- Extracts multiple XML files from the ZIP:
  - `xmi` — XMI model data (uses `QXmlStreamReader`)
  - `pgml` — PGML diagram data (parsed minimally)
  - `todo` — To-do items (parsed minimally)
- The XMI file is fed through the standard `loadFromXMI()` pipeline

### 8.2 Rational Rose Import (`Import_Rose`)

File: `umbrello/import_rose.cpp` (535 lines)

- Handles `.mdl` files (Rational Rose model files)
- Line-based parser (not XML — Rose uses a proprietary text format)
- Uses recursive descent parsing on tokenized lines
- Tokenization via `scan()` function splits on parentheses
- `PetalNode` / `PetalTree2UML` converts the parsed petal tree to Umbrello objects
- Files use a tree of parenthesized `(object_name attribute_list ...)` nodes
- No XMI involvement — direct object creation via `Object_Factory`

### 8.3 Foreign XMI Dialects

The `loadUMLObjectsFromXMI()` method handles multiple non-native XMI formats:

| Dialect | Recognition Pattern | Handling |
|---------|-------------------|----------|
| NSUML | Missing `<UML:Model>` wrapper — bare `<UML:Class>` etc. | Treated as individual objects, `guessContainer()` selects folder |
| Unisys JCR.1 | `<TaggedValue>` outside `<UML:Model>` | Extracts documentation tagged values |
| Unisys IntegratePlus | `<UISModelElement>` / `<uisOwnedDiagram>` | Handled in extension loading |
| Embarcadero Describe | `<Project>` tag, `<Element.ownedElement>` | Handled as alternative namespace |
| ArgoUML XMI | Via `.zargo` archive | Extracted and fed through standard loader |

### 8.4 Import Architecture Limitations

- **No plugin architecture**: Import formats hard-coded into UMLDoc
- **Inconsistent parsing**: Rose uses custom line parser, Argo uses `QXmlStreamReader`, native uses `QDomDocument`
- **No progress reporting**: Load is all-or-nothing in one blocking call
- **No partial load failure recovery**: If any object fails, the entire load may fail

---

## 9. CLI Export Analysis

### 9.1 Export Options

```
umbrello [file] --export <ext> [--directory <dir>] [--use-folders]
```

Implemented in `main.cpp` (lines 41-255).

### 9.2 Export Flow

```
main.cpp:exportAllViews(extension, directory, useFolders)
  → Load the .xmi file
  → For each view in the document:
      → UMLViewImageExporterModel::exportView(scene, imageType, url)
        → QImage / QSvgGenerator / QPrinter / DOT
        → Uses QPainter to render the scene
  → Post a CmdLineExportAllViewsEvent to quit after export
```

### 9.3 Supported Export Formats

From `UMLViewImageExporterModel::supportedImageTypes()`:

| Format | Method |
|--------|--------|
| SVG | `QSvgGenerator` |
| PNG | `QImage` + `QImageWriter` |
| JPEG | `QImage` + `QImageWriter` |
| BMP | `QImage` + `QImageWriter` |
| EPS | `QPrinter` (PostScript output) |
| DOT | Custom DOT generator (Graphviz format) |
| TIFF | `QImage` + `QImageWriter` |

Formats are determined by Qt's `QImageWriter::supportedImageFormats()` at runtime, plus EPS, SVG, and DOT are always added.

### 9.4 CLI Import Options

```
--import-files <files...>      Import source files (code → UML model)
--import-directory <dir>       Import all files from directory
--set-language <lang>          Set active programming language
```

---

## 10. Rust Rewrite Recommendations

### 10.1 XMI Serialization

**Current:** `QXmlStreamWriter` for save, `QDomDocument` for load  
**Recommended:** Use `serde` + `quick-xml` for both directions

- `quick-xml` provides streaming XML reader/writer
- `serde` provides derive macros for automatic serialization
- Decouple XMI format version from data model via visitor/traits

```rust
// Proposed approach:
#[derive(Serialize, Deserialize)]
#[serde(rename = "UML:Class", alias = "packagedElement")]
struct UmlClass {
    #[serde(rename = "@xmi.id", alias = "@xmi:id")]
    id: String,
    name: String,
    // ...
}
```

**Replace DOM loading** with streaming (SAX-style) for large files:
- Use `quick-xml::Reader` with `Events` for element-level parsing
- Build object tree incrementally without holding the entire XML in memory
- Only construct objects when their completion event fires

### 10.2 Serialize/Deserialize on All Model Types

```rust
trait UmlSerializable: Serialize + DeserializeOwned {
    fn xmi_type() -> &'static str;           // "uml:Class" or "UML:Class"
    fn xmi_tag() -> &'static str;            // "packagedElement" or "UML:Class"
    fn resolve_refs(&mut self, registry: &ObjectRegistry);
    fn collect_refs(&self) -> Vec<String>;   // IDs to resolve later
}
```

Every model type implements this trait. The `save1/load1` pattern collapses into a single derive.

### 10.3 Visitor Pattern for XMI Version Adaptation

```rust
trait XmiVersionAdapter {
    fn root_element(&self) -> &str;           // "XMI" vs "xmi:XMI"
    fn id_attribute(&self) -> &str;           // "xmi.id" vs "xmi:id"
    fn namespace(&self) -> &str;              // "UML" vs "uml"
    fn wrap_content(&self, writer: &mut Writer);
    fn extension_element(&self) -> &str;      // "XMI.extensions" vs "xmi:Extension"
}

struct Xmi12Adapter;
struct Xmi21Adapter;
```

### 10.4 Forward References — Two-Pass Loading

```rust
struct ObjectRegistry {
    objects: HashMap<String, Box<dyn UmlObject>>,
    pending_refs: Vec<PendingRef>,  // (object_id, field_name, target_id)
}

impl ObjectRegistry {
    fn register(&mut self, obj: Box<dyn UmlObject>);
    fn resolve(&mut self) -> Result<()> {
        // Iterate pending_refs, resolve each target_id from self.objects
    }
}
```

All objects loaded in PASS 1, registry holds unresolved refs as string IDs. PASS 2 resolves all at once via `HashMap<String, UmlObject>` lookup.

### 10.5 Compression

**Current:** KTar (KDE archive API)  
**Recommended:** `flate2` + `tar` crates

```rust
use flate2::{GzEncoder, GzDecoder};
use tar::{Builder, Archive};

// Save
let mut tar = Builder::new(GzEncoder::new(file, Compression::default()));
let mut xmi_data = Vec::new();
serialize_to_xmi(&mut xmi_data, &model)?;
tar.append_file("model.xmi", &mut Cursor::new(xmi_data))?;

// Load
let archive = Archive::new(GzDecoder::new(file));
for entry in archive.entries()? {
    // find application/x-uml entry
}
```

### 10.6 Undo/Redo Architecture

**Option A: Command trait with trait objects**

```rust
trait Command: Send {
    fn execute(&mut self) -> Result<()>;
    fn undo(&mut self) -> Result<()>;
    fn description(&self) -> String;
}

struct UndoStack {
    stack: Vec<Box<dyn Command>>,
    position: usize,
    enabled: bool,
    macro_group: Option<String>,
}
```

**Option B: Event sourcing pattern**

```rust
enum ModelEvent {
    ObjectCreated { id: String, object_type: ObjectType, name: String },
    ObjectRemoved { id: String, snapshot: Box<dyn UmlObject> },
    PropertyChanged { id: String, field: String, old_value: Value, new_value: Value },
    // ...
}

struct EventStore {
    events: Vec<ModelEvent>,
    current: usize,
}
```

**Recommendation:** Start with Command trait (simpler, more direct translation). Move to event sourcing if undo performance becomes an issue with large command sequences.

**Key considerations:**
- Disable recording during file load (like current `m_bLoading` flag)
- Commands should operate on model only, not widgets/views (separate concerns)
- Use snapshot for deletions (store the deleted object's state)
- Macro grouping for composite operations

### 10.7 File Format Support

```rust
trait StorageBackend {
    fn load(&self, path: &Path) -> Result<UmlModel>;
    fn save(&self, path: &Path, model: &UmlModel) -> Result<()>;
}

struct XmiFileStorage { version: XmiVersion }
struct XmiTgzStorage { version: XmiVersion }
struct XmiBz2Storage { version: XmiVersion }
```

Backend detection from file extension, automatic selection on load (probe file header).

### 10.8 Schema Validation

```rust
trait SchemaValidator: Send {
    fn validate(&self, doc: &[u8]) -> Result<()>;
}

struct DtdValidator { dtd: String }
struct XsdValidator { xsd: String }
```

Use `quick-xml`'s validation support or a dedicated XML schema crate. Provide both DTD and XSD validators. Make validation a configurable option (validate by default, skip for performance).

### 10.9 CLI Export

```rust
enum ExportFormat {
    Svg, Png, Jpeg, Bmp, Eps, Dot,
}

trait ImageExporter {
    fn export(&self, scene: &DiagramScene, path: &Path) -> Result<()>;
}

struct SvgExporter;   // via svg crate (resvg for rendering)
struct PngExporter;   // via image crate (image-rs)
struct DotRenderer;   // custom DOT output for Graphviz
```

Replace `QSvgGenerator`/`QPainter` with Rust-native rendering:
- `resvg` for SVG rendering
- `image` (image-rs) for raster formats
- Custom painter model for cross-format rendering

### 10.10 DocBook Export

```rust
#[derive(Serialize)]
struct DocBookDocument {
    #[serde(rename = "article")]
    article: DocBookArticle,
}

// Serialize directly to DocBook XML via quick-xml + serde
```

### 10.11 Storage Abstraction Trait

```rust
/// Storage abstraction — could add SQLite backend later
trait Storage {
    /// Load model from storage
    fn load(&self, source: &StorageSource) -> Result<UmlModel>;
    
    /// Save model to storage
    fn save(&self, model: &UmlModel, destination: &StorageSink) -> Result<()>;
    
    /// Import from foreign format
    fn import(&self, source: &StorageSource) -> Result<UmlModel>;
    
    /// Export diagram to image
    fn export_diagram(&self, scene: &DiagramScene, format: ExportFormat) -> Vec<u8>;
    
    /// List available diagrams (for CLI export)
    fn list_diagrams(&self, source: &StorageSource) -> Result<Vec<DiagramInfo>>;
}

enum StorageSource {
    Path(PathBuf),
    Stream(Box<dyn Read>),
    Bytes(Vec<u8>),
}

enum StorageSink {
    Path(PathBuf),
    Stream(Box<dyn Write>),
}

/// XMI implementation of Storage
struct XmiStorage {
    version: XmiVersion,
    validate: bool,
    compress: Option<Compression>,
}

enum XmiVersion { V1_2, V2_1 }

enum Compression {
    Gzip,      // .xmi.tgz
    Bzip2,     // .xmi.tar.bz2
    None,      // .xmi
}

/// Future: SQLite-backed storage
struct SqliteStorage {
    connection: sqlite::Connection,
    // ...
}
```

---

## 11. Proposed Trait Design for Persistence Layer

### 11.1 Core Serialization Traits

```rust
/// Every UML model element can be serialized to/from XMI
pub trait XmiSerializable: Serialize + DeserializeOwned {
    /// XMI type name used in xmi:type attribute (UML2 mode)
    fn xmi_type(&self) -> &'static str;
    
    /// XMI element tag (UML1 mode) or packagedElement tag (UML2 mode)
    fn xmi_tag(&self, version: XmiVersion) -> &'static str;
}

/// Objects that reference other objects by ID
pub trait HasReferences {
    /// Collect IDs of referenced objects (for forward ref resolution)
    fn referenced_ids(&self) -> Vec<String>;
    
    /// Resolve stored IDs to actual object references using the registry
    fn resolve_references(&mut self, registry: &ObjectRegistry) -> Result<()>;
}

/// A UML model element with identity
pub trait UmlObject: XmiSerializable + HasReferences + Debug {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn set_id(&mut self, id: String);
    fn object_type(&self) -> ObjectType;
}
```

### 11.2 Model and Registry

```rust
/// Central object registry — holds all loaded objects for ID resolution
#[derive(Default)]
pub struct ObjectRegistry {
    by_id: HashMap<String, Box<dyn UmlObject>>,
    pending: Vec<PendingReference>,    // forward references collected during load
}

pub struct PendingReference {
    pub owner_id: String,
    pub field_path: Vec<String>,       // e.g., ["type", "classifier_id"]
    pub target_id: String,
}

impl ObjectRegistry {
    pub fn insert(&mut self, obj: Box<dyn UmlObject>);
    pub fn get(&self, id: &str) -> Option<&dyn UmlObject>;
    pub fn get_mut(&mut self, id: &str) -> Option<&mut dyn UmlObject>;
    pub fn collect_references(&mut self);    // phase 1: walk all objects
    pub fn resolve_all(&mut self) -> Result<()>;  // phase 2: resolve
}

/// Top-level UML model container
#[derive(Serialize, Deserialize)]
pub struct UmlModel {
    pub model_id: String,
    pub name: String,
    pub stereotypes: Vec<Stereotype>,
    pub root_folders: Vec<Folder>,
    pub diagrams: Vec<Diagram>,
    pub doc_settings: DocSettings,
    pub listview_state: Option<ListViewState>,
    pub codegen_state: Option<CodeGenState>,
}
```

### 11.3 Storage and Versioning Traits

```rust
/// Version adapter — serialization strategy per XMI version
pub trait XmiVersionAdapter {
    fn version(&self) -> XmiVersion;
    
    // Element naming
    fn root_tag(&self) -> &'static str;       // "XMI" | "xmi:XMI"
    fn id_attr(&self) -> &'static str;         // "xmi.id" | "xmi:id"
    fn type_attr(&self) -> Option<&'static str>; // None | "xmi:type"
    fn namespace(&self) -> &'static str;       // "UML" | "uml"
    fn ns_uri(&self) -> &'static str;
    
    // Structural wrappers
    fn has_header(&self) -> bool;              // true for 1.2
    fn has_content_wrapper(&self) -> bool;     // true for 1.2
    fn content_element(&self) -> Option<&'static str>; // "XMI.content" | None
    fn extension_tag(&self) -> &'static str;   // "XMI.extensions" | "xmi:Extension"
    
    // Element wrapping
    fn owned_element_tag(&self) -> &'static str; // "Namespace.ownedElement" | ""
}

/// Storage backend with format detection
pub trait StorageBackend: Debug {
    fn extension_hints(&self) -> &[&str];       // ["xmi"], ["xmi.tgz", "xmi.tar.gz"]
    fn magic_bytes(&self) -> &[u8];              // optional header detection
    
    fn load_model(&self, reader: &mut dyn Read) -> Result<UmlModel>;
    fn save_model(&self, writer: &mut dyn Write, model: &UmlModel) -> Result<()>;
    fn import_model(&self, reader: &mut dyn Read, format: ImportFormat) -> Result<UmlModel>;
}

pub enum ImportFormat {
    ArgoUML,        // .zargo
    RationalRose,   // .mdl
    Nsuml,          // foreign XMI
    Unisys,         // Unisys JCR.1
    Embarcadero,    // Describe
}
```

### 11.4 Undo/Redo Traits

```rust
/// A single reversible operation on the model
pub trait Command: Send + Debug {
    fn execute(&mut self, model: &mut UmlModel) -> Result<()>;
    fn undo(&mut self, model: &mut UmlModel) -> Result<()>;
    fn description(&self) -> String;
}

/// Group of commands treated as a single undoable unit
pub struct CommandGroup {
    pub description: String,
    pub commands: Vec<Box<dyn Command>>,
}

/// Undo stack manager
pub struct UndoManager {
    stack: Vec<UndoEntry>,
    position: usize,           // current position in stack
    enabled: bool,             // false during loading
    in_macro: bool,            // true between begin/end macro
    active_group: Vec<Box<dyn Command>>,
}

impl UndoManager {
    pub fn push(&mut self, cmd: Box<dyn Command>, model: &mut UmlModel) -> Result<()>;
    pub fn undo(&mut self, model: &mut UmlModel) -> Result<()>;
    pub fn redo(&mut self, model: &mut UmlModel) -> Result<()>;
    pub fn clear(&mut self);
    pub fn begin_macro(&mut self, description: &str) -> Result<()>;
    pub fn end_macro(&mut self) -> Result<()>;
    
    pub fn disable(&mut self) { self.enabled = false; }
    pub fn enable(&mut self) { self.enabled = true; }
}
```

### 11.5 CLI Export Trait

```rust
pub trait DiagramExporter {
    fn export_diagram(&self, scene: &DiagramScene, format: ExportFormat) -> Result<Vec<u8>>;
    fn export_all(&self, model: &UmlModel, format: ExportFormat, dir: &Path) -> Result<Vec<PathBuf>>;
}

pub enum ExportFormat {
    Svg, Png, Jpeg, Bmp, Eps, Tiff, Dot,
}

pub struct ExportConfig {
    pub format: ExportFormat,
    pub directory: PathBuf,
    pub use_folders: bool,    // preserve tree structure
}
```

### 11.6 Error Handling

```rust
#[derive(thiserror::Error, Debug)]
pub enum PersistenceError {
    #[error("XMI parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Unresolved forward reference: {0} → {1}")]
    UnresolvedReference(String, String),   // (owner_id, target_id)
    
    #[error("Unsupported XMI version: {0}")]
    UnsupportedVersion(String),
    
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),
    
    #[error("Validation error: {0}")]
    ValidationError(String),
    
    #[error("Corrupted archive: {0}")]
    ArchiveError(String),
    
    #[error("Export failed: {0}")]
    ExportError(String),
}
```

---

## Migration Strategy Summary

| Component | C++ (Current) | Rust (Target) |
|-----------|--------------|---------------|
| XML save | `QXmlStreamWriter` | `quick-xml` + `serde` |
| XML load | `QDomDocument` (DOM) | `quick-xml` (streaming) |
| Serialization | Manual `save1/load1` | Derive `Serialize`/`Deserialize` |
| XMI versions | Inline `if(uml2)` checks | `XmiVersionAdapter` trait |
| Forward refs | `m_SecondaryId` + `resolveRef()` | `ObjectRegistry` + two-pass |
| Compression | `KTar` | `tar` + `flate2` crates |
| Undo/redo | `QUndoCommand` hierarchy | `Command` trait + `UndoManager` |
| Import formats | Hard-coded `if/else` | `StorageBackend` trait |
| Export images | `QPainter` + `QImage`/`QSvgGenerator` | `resvg` + `image` crates |
| Storage | Monolithic `UMLDoc` | `Storage` trait (XMI now, SQLite later) |
| Error handling | `bool` return + `KMessageBox` | `Result<T, PersistenceError>` |
| Validation | None | Optional DTD/XSD validator |
