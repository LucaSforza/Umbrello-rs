# Code Generation Subsystem Analysis

> **Scope**: `umbrello/codegenerators/` — ~203 source files across 22 programming languages
> **Status**: Dual strategy with massive duplication; no plugin architecture; no template engine
> **Last updated**: 2026-06-23

---

## 1. Architecture Overview

The code generation subsystem translates UML model elements (classes, interfaces, attributes,
operations, relationships) into compilable source code across 22 programming languages.
It provides two fundamentally different implementation strategies, a factory-based dispatching
mechanism, a policy framework, and a wizard-driven user interface.

### 1.1 Two Generation Paths

```
UML Model (UMLClassifier, UMLOperation, UMLAttribute, ...)
    │
    ├── Simple Path (18 languages)
    │   └── SimpleCodeGenerator::writeClass()
    │       └── QTextStream → QString → file I/O
    │
    └── Advanced Path (4 languages: C++, D, Java, Ruby)
        └── AdvancedCodeGenerator::newClassifierCodeDocument()
            └── CodeDocument tree (TextBlock hierarchy)
                ├── syncCodeToDocument() (model → document)
                ├── syncToParent() (child → parent block)
                └── toString() → QString → file I/O
```

**Simple Path** — Used by 18 of 22 languages:
- Direct stream-based output via `QTextStream`.
- Each language subclass implements a single `writeClass()` method.
- No in-memory representation; generation is immediate and one-shot.
- No editing support; code is written to file and not reloadable for editing.
- Disconnected from model — if the model changes, generated code is stale.

**Advanced Path** — Used by 4 languages (C++, D, Java, Ruby):
- Builds an in-memory `CodeDocument` tree composed of `TextBlock` nodes.
- Supports `syncCodeToDocument()` for bidirectional model/document synchronization.
- Enables in-application code editing (via KTextEditor integration).
- Maintains a persistent connection to the model: changes propagate to the document tree.
- Significantly more complex: ~37 files for C++, ~27 for Java, ~25 each for D and Ruby.

### 1.2 Class Hierarchy

```
CodeGenerator (abstract, QObject)
├── SimpleCodeGenerator
│     ├── AdaWriter
│     ├── ASWriter (ActionScript)
│     ├── CppWriter
│     ├── CSharpWriter
│     ├── DWriter
│     ├── IDLWriter
│     ├── JavaWriter
│     ├── JSWriter
│     ├── PascalWriter
│     ├── PerlWriter
│     ├── PhpWriter (PHP4)
│     ├── Php5Writer (PHP5)
│     ├── PythonWriter
│     ├── RubyWriter
│     ├── SQLWriter
│     │    ├── MySQLWriter
│     │    └── PostgreSQLWriter
│     ├── TclWriter
│     ├── ValaWriter
│     └── XMLSchemaWriter
└── AdvancedCodeGenerator
      ├── CPPCodeGenerator
      ├── DCodeGenerator
      ├── JavaCodeGenerator
      └── RubyCodeGenerator
```

### 1.3 CodeDocument Model (Advanced Only)

The `CodeDocument` model is a hierarchical tree of text-producing blocks:

```
TextBlock (abstract, QObject)
└── CodeBlock
    └── CodeBlockWithComments
        └── HierarchicalCodeBlock
            └── OwnedCodeBlock
                └── CodeMethodBlock
                    ├── CodeOperation (maps to UMLOperation)
                    └── CodeAccessorMethod (getter/setter)

CodeParameter (hold parameter metadata)
CodeClassField (maps to UMLAttribute or UMLAssociation end)
CodeDocument (abstract, container)
└── ClassifierCodeDocument (per-classifier)
    ├── CPPHeaderCodeDocument
    ├── CPPSourceCodeDocument
    ├── DClassifierCodeDocument
    ├── JavaClassifierCodeDocument
    └── RubyClassifierCodeDocument
```

Key relationships:
- `ClassifierCodeDocument` owns a list of `CodeOperation` and `CodeClassField` objects.
- `CodeMethodBlock` generates method signatures, bodies, and comments.
- `CodeClassField` represents both attributes (from `UMLAttribute`) and association ends.
- `CodeParameter` holds name, type, initial value, and comments for method parameters.
- Sync direction: `syncCodeToDocument()` reads the model and updates the document tree;
  `syncToParent()` propagates content changes upward through the block hierarchy.

### 1.4 Writing to File

All generators ultimately produce a `QString` and write it to disk:

```cpp
// Simple path — immediate write
QString code = writer->writeClass(classifier);
QFile file(path);
file.open(QIODevice::WriteOnly);
file.write(code.toUtf8());

// Advanced path — document-tree accumulation then write
CodeDocument* doc = generator->newClassifierCodeDocument(classifier);
doc->syncCodeToDocument();
QString code = doc->toString();
QFile file(path);
file.open(QIODevice::WriteOnly);
file.write(code.toUtf8());
```

---

## 2. All Supported Languages with Generator Classes

### 2.1 Language Enumeration

Languages are identified by the `ProgrammingLanguage` enum (defined in
`umbrello/codegenerators/codegenfactory.h`). Each value corresponds to exactly one
writer or code-generator class.

### 2.2 Language Table

| Language | Enum Value | Generator Class | Strategy | Files | Policy Extension |
|---|---|---|---|---|---|
| Ada | `Ada` | `AdaWriter` | Simple | 2 | — |
| ActionScript | `ActionScript` | `ASWriter` | Simple | 2 | — |
| C++ | `Cpp` | `CppWriter` / `CPPCodeGenerator` | **Both** | 2+37 | Extended (inline accessors, virtual destructors, namespace-as-package) |
| C# | `CSharp` | `CSharpWriter` | Simple | 2 | — |
| D | `D` | `DWriter` / `DCodeGenerator` | **Both** | 2+25 | Extended (auto-generate accessors) |
| IDL | `IDL` | `IDLWriter` | Simple | 2 | — |
| Java | `Java` | `JavaWriter` / `JavaCodeGenerator` | **Both** | 2+27 | Extended (auto-generate accessors) |
| JavaScript | `JavaScript` | `JSWriter` | Simple | 2 | — |
| MySQL | `MySQL` | `MySQLWriter` | Simple | 2 | — |
| Pascal | `Pascal` | `PascalWriter` | Simple | 2 | — |
| Perl | `Perl` | `PerlWriter` | Simple | 2 | — |
| PHP4 | `PHP` | `PhpWriter` | Simple | 2 | — |
| PHP5 | `PHP5` | `Php5Writer` | Simple | 2 | — |
| PostgreSQL | `PostgreSQL` | `PostgreSQLWriter` | Simple | 2 | — |
| Python | `Python` | `PythonWriter` | Simple | 2 | — |
| Ruby | `Ruby` | `RubyWriter` / `RubyCodeGenerator` | **Both** | 2+25 | Extended (auto-generate accessors) |
| SQL | `SQL` | `SQLWriter` | Simple | 2 | — |
| Tcl | `Tcl` | `TclWriter` | Simple | 2 | — |
| Vala | `Vala` | `ValaWriter` | Simple | 2 | — |
| XML Schema | `XMLSchema` | `XMLSchemaWriter` | Simple | 2 | — |

### 2.3 Dual-Implementation Languages

Four languages have **both** a simple writer and an advanced code generator:

| Language | Simple Writer | Advanced Generator | Files |
|---|---|---|---|
| C++ | `CppWriter` | `CPPCodeGenerator` | 2 + 37 = 39 |
| D | `DWriter` | `DCodeGenerator` | 2 + 25 = 27 |
| Java | `JavaWriter` | `JavaCodeGenerator` | 2 + 27 = 29 |
| Ruby | `RubyWriter` | `RubyCodeGenerator` | 2 + 25 = 27 |

This dual implementation is a major source of technical debt — each language has *two*
complete, independent implementations of the same generation logic with no shared code
between them. The simple writers are typically older/deprecated while the advanced
generators are newer and support editing.

### 2.4 SQL Variants

The SQL family demonstrates inheritance within simple writers:

```
SQLWriter (base for SQL)
├── MySQLWriter
└── PostgreSQLWriter
```

`SQLWriter` provides common SQL generation logic (CREATE TABLE, column types, constraints).
MySQL and PostgreSQL override dialect-specific details (auto-increment syntax, type mappings,
engine options).

### 2.5 Heading Templates

17 template files provide standardized file-level comment blocks:
- Language-specific comment syntax (C-style `/* */`, shell `#`, SQL `--`, etc.)
- Auto-generated warning disclaimers
- License header templates
- Customizable via `CodeGenerationPolicy` settings

---

## 3. Code Generation Workflow

### 3.1 User-Initiated Flow

```
User triggers CodeGenerationWizard
    │
    ▼
┌─────────────────────────────────────────────────────┐
│ Page 1: SelectClassPage                              │
│  - Tree view of all classifiers in the model         │
│  - Multi-select with checkboxes                      │
│  - "Select All" / "Deselect All" buttons             │
│  - Confirm → proceeds to options                     │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────┐
│ Page 2: CodeGenerationOptionsPage                    │
│  - CodeGenerationPolicy settings:                    │
│    ├── Output directory (file chooser)               │
│    ├── Overwrite existing (yes/no/ask)               │
│    ├── Indentation (tabs/spaces + width)             │
│    ├── Newline style (LF/CRLF)                       │
│    └── Comment style (JavaDoc / Qt / KDE)           │
│  - Language-specific policy (CodeGenPolicyExt):      │
│    ├── C++: inline accessors, virtual destructors    │
│    ├── Java: auto-generate accessors                 │
│    ├── D: auto-generate accessors                    │
│    └── Ruby: auto-generate accessors                 │
│  - Confirm → proceeds to generation                  │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌─────────────────────────────────────────────────────┐
│ Page 3: CodeGenerationProgressPage                   │
│  - For each selected classifier:                     │
│    1. Check overwrite policy                         │
│    2. Determine output file path                     │
│    3. Generate code via CodeGenerator::writeClass()  │
│       (simple) or newClassifierCodeDocument() (adv.) │
│    4. Write file to disk                             │
│    5. Report success/failure via signal              │
│  - Progress bar updates per-classifier               │
│  - Log of generated files / errors                   │
│  - "Finish" to close wizard                          │
└──────────────────────────────────────────────────────┘
```

### 3.2 Programmatic API Entry Points

```cpp
// CodeGenerator abstract base — key methods
class CodeGenerator : public QObject {
public:
    virtual void writeClass(UMLClassifier *c, QIODevice &dev);  // Simple path only
    virtual ClassifierCodeDocument* newClassifierCodeDocument(UMLClassifier *c);  // Both
    virtual QStringList defaultDatatypes();                     // Language type mappings
    virtual QStringList reservedKeywords();                     // Reserved word list
    CodeGenerationPolicy *policy() const;                       // Policy access
};

// SimpleCodeGenerator — template method pattern
class SimpleCodeGenerator : public CodeGenerator {
    void writeClass(UMLClassifier *c, QIODevice &dev) final;    // Calls virtual methods
    virtual void makeFile(QString &code, ...);                  // File preamble/postamble
};

// AdvancedCodeGenerator — document-tree pattern
class AdvancedCodeGenerator : public CodeGenerator {
    ClassifierCodeDocument* newClassifierCodeDocument(UMLClassifier *c) override;
    virtual void syncCodeToDocument(UMLClassifier *c);          // Model → document sync
};
```

### 3.3 File Path Determination

Output paths follow language-specific conventions derived from classifier name and
package/namespace hierarchy:

```cpp
QString CodeGenerator::getFileName(UMLClassifier *c) {
    QString name = c->name();
    QString package = c->umlPackage()->name();  // e.g., "com.example.model"
    // Language-specific conversion:
    //   Java: replace '.' → '/', append ".java"
    //   C++: replace '.' → '/', append ".h" or ".cpp"
    //   Python: replace '.' → '/', append ".py"
    // etc.
    QString path = outputDirectory + "/" + packagePath + "/" + name + extension;
    return path;
}
```

### 3.4 Advanced Sync Pipeline

For the 4 advanced languages, code generation is a multi-phase process:

```
1. newClassifierCodeDocument(classifier)
   │  Creates empty document structure (header, body, includes, etc.)
   ▼
2. syncCodeToDocument()
   │  Reads UML model elements:
   │  ├── Attributes → CodeClassField objects
   │  ├── Operations → CodeOperation objects
   │  ├── Associations → CodeClassField objects (role names)
   │  ├── Templates → Generic parameter declarations
   │  ├── Parent class → base class reference
   │  └── Interfaces → implemented interface references
   │  Populates the document tree with content blocks.
   ▼
3. ClassifierCodeDocument::toString()
   │  Recursively serializes the TextBlock tree to a QString.
   │  Each block handles its own indentation, formatting, and comments.
   ▼
4. File write
   │  QFile::write(toString().toUtf8())
   ▼
5. (Optional) syncToParent() / syncFromParent()
   │  Bidirectional sync between code document and model:
   │  - Model changes → syncCodeToDocument() updates blocks
   │  - Code edits → syncFromParent() updates model
```

---

## 4. Factory Pattern Analysis

### 4.1 CodeGenFactory Namespace

`CodeGenFactory` is a namespace containing 7 static factory methods, each dispatching
on the `ProgrammingLanguage` enum via a `switch` statement.

```cpp
namespace CodeGenFactory {
    CodeGenerator* createObject(const ProgrammingLanguage &lang);
    CodeGenerator* createObject(const QString &lang);     // String overload
    CodeGenPolicyExt* createPolicyExt(CodeGenerationPolicy *parent);
    CodeGenPolicyExt* createPolicyExt(const QString &lang);
    // … 3 more factory methods for related objects
}
```

The main factory method is 190+ lines with this structure:

```cpp
CodeGenerator* CodeGenFactory::createObject(const ProgrammingLanguage &lang) {
    switch (lang) {
    case ProgrammingLanguage::Ada:
        return new AdaWriter();
    case ProgrammingLanguage::ActionScript:
        return new ASWriter();
    case ProgrammingLanguage::Cpp:
        // Special-cased: dynamic_cast to determine header vs source
        // Returns CPPCodeGenerator (advanced), not CppWriter
        return new CPPCodeGenerator();
    case ProgrammingLanguage::CSharp:
        return new CSharpWriter();
    // … 19 more cases …
    }
}
```

### 4.2 Factory Methods

| Factory Method | Return Type | Purpose |
|---|---|---|
| `createObject(ProgrammingLanguage)` | `CodeGenerator*` | Main generator creation |
| `createObject(QString)` | `CodeGenerator*` | String-based dispatch (used by UI) |
| `createPolicyExt(CodeGenerationPolicy*)` | `CodeGenPolicyExt*` | Language policy extension |
| `createPolicyExt(QString)` | `CodeGenPolicyExt*` | String-based policy creation |
| `createDefaultPolicy(…)` | `CodeGenerationPolicy*` | Default policy with language defaults |
| `createCodeDoc(…)` | `CodeDocument*` | Document model creation |
| `createCodeDocForLanguage(…)` | `CodeDocument*` | Another document factory variant |

### 4.3 C++ Special-Casing

C++ is treated differently throughout the factory and the broader codebase:

1. **Advanced generator by default**: Unlike other languages where the factory returns
   the simple writer, for C++ it returns the advanced `CPPCodeGenerator`.
2. **Header/source split**: `CPPCodeGenerator` creates two `ClassifierCodeDocument`
   objects per classifier — a header document and a source document.
3. **`dynamic_cast` checks**: Code throughout the codebase uses `dynamic_cast` to
   detect `CPPCodeGenerator` specifically, e.g.:
   ```cpp
   if (auto *cppGen = dynamic_cast<CPPCodeGenerator*>(generator)) {
       // Handle C++-specific header/source logic
   }
   ```
4. **Default language**: C++ is the default fallback in the language selection UI.
5. **Most complex generator**: 37 files vs. 2 for most other languages.

### 4.4 Problems with the Current Factory

| Problem | Impact |
|---|---|
| **Massive switch statement** (190+ lines) | Violates Open/Closed Principle; every new language requires modifying the factory |
| **90+ headers included** in the factory | All generator classes are statically linked; long compile times |
| **String-based overload** duplicates dispatch logic | Two switch statements to maintain in sync |
| **`dynamic_cast` on C++** | Type-unsafe; fragile across refactoring; breaks polymorphism |
| **No lazy instantiation** | All generator types are available immediately; no plugin-based loading |
| **No configuration-driven creation** | Language → generator mapping is hardcoded; cannot be overridden or extended |
| **Return type is raw pointer** | Manual memory management; ownership semantics are unclear |

---

## 5. Code Document Model Analysis

### 5.1 TextBlock Hierarchy

The `TextBlock` hierarchy forms a composite pattern for representing source code as
an editable tree of blocks. This is used **only** by the 4 advanced languages.

```
TextBlock (QObject) — abstract base
│  Properties: text, indentLevel, blockNumber
│  Methods: toString(), setText(), getText()
│
└── CodeBlock — basic text content
    │  Properties: contentType (AutoGenerated, UserGenerated, etc.)
    │  Method: setContentType()
    │
    └── CodeBlockWithComments — adds comment management
        │  Properties: comment (CodeComment), writeOutText
        │
        └── HierarchicalCodeBlock — can contain child blocks
            │  Properties: childBlocks list
            │  Methods: addChildBlock(), insertChildBlock()
            │
            └── OwnedCodeBlock — knows its owning document
                │
                └── CodeMethodBlock — method/function body
                    │  Properties: startMethod, endMethod
                    │
                    ├── CodeOperation — bridges UMLOperation → method
                    │    Owns: list of CodeParameter
                    │
                    └── CodeAccessorMethod — getter/setter
                         Types: Getter, Setter, GetterSetter
```

### 5.2 Supporting Classes

```text
CodeParameter — method parameter
  Properties: name, type, initialValue, comment
  Comment management: each parameter can have its own CodeComment

CodeClassField — class member variable (attribute or association role)
  Properties: name, type, visibility, initialValue, comment
  Types: Attribute, Association (role), AssociationMultiRole (list-valued)

CodeDocument (abstract) — top-level document
  Properties: classifier, language, fileName
  Methods: addOperation(), addClassField(), toString()
  Status: status() → New, Modified, Saved

ClassifierCodeDocument — document for one UML classifier
  Specialized for: C++ header, C++ source, D, Java, Ruby
  Divides into: header section, body section, includes, etc.
```

### 5.3 Block Content Types

Each `CodeBlock` has a `ContentType` enum that controls behavior:

| Content Type | Meaning | Behavior |
|---|---|---|
| `AutoGenerated` | Generated from model | Regenerated on `syncCodeToDocument()` |
| `UserGenerated` | Added by user in editor | Preserved across re-syncs |
| `UserEditable` | Auto-generated but editable | Regenerated unless user modified |
| `CodeSnippet` | Custom code snippet | Always preserved |

This classification enables the bidirectional sync engine to distinguish between
generated content (which can be overwritten) and user modifications (which must be
preserved).

### 5.4 Sync Algorithm

```
syncCodeToDocument() algorithm (pseudocode):

1. Clear all AutoGenerated blocks from the document tree
2. For each UMLAttribute in classifier:
   a. Create or update CodeClassField with name, type, default value
   b. Generate comment list from attribute documentation
   c. Add accessor operations if policy says so (Java, D, Ruby)
3. For each UMLOperation in classifier:
   a. Create or update CodeOperation with signature, params, return type
   b. Generate body from documentation or default implementation
   c. Add comment from operation documentation
4. For each UMLAssociation where classifier is an end:
   a. Create CodeClassField with role name, type, multiplicity
   b. Mark as Association type
5. For each UMLTemplateParameter:
   a. Add generic type parameter to document header
6. Update file name, package/namespace declarations
7. Propagate changes: syncToParent() → update parent blocks
```

### 5.5 Problems with the CodeDocument Model

| Problem | Impact |
|---|---|
| **Deep inheritance hierarchy** (7 levels) | Rigid; hard to extend; diamond inheritance risks (TextBlock is QObject and also inherits via CodeBlock) |
| **Multiple inheritance of QObject** | The diamond pattern (`CodeMethodBlock` inherits from both `HierarchicalCodeBlock` and `OwnedCodeBlock`, both of which derive from `QObject`) is fragile and can cause memory issues |
| **Bidirectional sync is complex** | `syncCodeToDocument()` + `syncToParent()` + content type tracking is hard to get right; bugs are common |
| **Only 4 languages use it** | 18 languages don't benefit from the document model; massive code duplication |
| **Memory-heavy** | Every keyword, brace, and comment is a `QObject` on the heap |
| **No serialization** | CodeDocument trees are not persisted; they are rebuilt from the model each time |
| **Editor coupling** | `CodeDocument` is designed for KTextEditor integration; not usable in a headless/CLI context |
| **Content type tracking** | Distinguishing AutoGenerated vs UserGenerated is fragile; content loss bugs are easy to trigger |

---

## 6. Technical Debt Inventory

### 6.1 Critical Debt

| # | Debt Item | Files Affected | Severity |
|---|---|---|---|
| D1 | Massive switch in `CodeGenFactory::createObject()` — 190 lines, 22 cases | 1 | **High** |
| D2 | C++ special-casing via `dynamic_cast` throughout the codebase | ~15+ | **High** |
| D3 | Dual implementations for C++, D, Java, Ruby (simple + advanced) — complete duplication | ~120+ | **High** |
| D4 | No plugin architecture — all languages hard-coded, all headers included at compile time | 1 (factory) + 22 | **High** |
| D5 | `TextBlock` multiple inheritance diamond pattern (QObject multi-path) | ~15 | **High** |

### 6.2 Moderate Debt

| # | Debt Item | Files Affected | Severity |
|---|---|---|---|
| D6 | No template engine — all code generated via string concatenation / tree construction | 203 | **Medium** |
| D7 | `SimpleCodeGenerator` disconnected from model changes — no sync | 18 | **Medium** |
| D8 | `getFileName()` path logic duplicated across generators | ~22 | **Medium** |
| D9 | `ProgrammingLanguage` enum must be updated for every new language | 1 (+ many switch statements) | **Medium** |
| D10 | CodeGenFactory returns raw pointers — ownership is implicit | 7 factory methods | **Medium** |
| D11 | 18 languages use 2 files each but share almost no code | 36 | **Medium** |
| D12 | `CodeGenPolicyExt` only exists for 4 languages; others use default | 1 (policy) | **Medium** |
| D13 | No batch/headless generation mode — only available via wizard | 1 | **Medium** |

### 6.3 Minor Debt

| # | Debt Item | Files Affected | Severity |
|---|---|---|---|
| D14 | 17 heading templates are duplicated per-generator rather than shared | 17 | **Low** |
| D15 | `defaultDatatypes()` and `reservedKeywords()` are redefined per language | ~22 | **Low** |
| D16 | No unit test coverage for most generators (`testcppwriter` is the exception) | 202 of 203 | **Low** |
| D17 | `writeClass()` method signature takes `QIODevice&` — not thread-safe | 1 (base) | **Low** |
| D18 | CodeGenWizard uses 3 fixed pages — not extensible | 1 | **Low** |

### 6.4 Duplication Metric

```
Total code generator files:     ~203
Unique simple languages:           18 files each × 2 files = 36 files
Unique advanced languages:          4 files (37+27+29+25) = 118 files
Heading templates:                                           17 files
Shared infrastructure (base, factory, policy, wizard):      ~32 files

Redundancy estimate:
- C++ simple writer logic is entirely duplicated in CPPCodeGenerator
- D, Java, Ruby each have two complete implementations
- Conservative estimate: 50-60% of generator code is duplicated
```

---

## 7. Rust Recommendations

### 7.1 Plugin Architecture (Primary Recommendation)

Each language should be implemented as a **separate Rust crate** that registers a
`CodeGenerator` trait implementation. This follows Rust's module system and aligns
with the Cargo workspace pattern.

```toml
# Workspace Cargo.toml
[workspace]
members = [
    "umbrello-core",          # Model, base traits
    "umbrello-codegen",       # Code generation framework
    "generators/umbrello-gen-cpp",     # C++ generator crate
    "generators/umbrello-gen-java",    # Java generator crate
    "generators/umbrello-gen-python",  # Python generator crate
    # ... one crate per language
]
```

**Advantages:**
- Languages can be compiled, tested, and maintained independently
- Adding a new language = adding a new crate (no central file modification)
- Optional inclusion: users can compile only the generators they need
- Clear dependency graph: core → codegen framework → per-language generators
- Each crate can have its own version, configuration, and dependencies

**Disadvantages:**
- Crate proliferation: 22+ crates for languages
- Cross-crate trait cohesion requires careful API design
- Dynamic dispatch overhead (negligible for generation workloads)

### 7.2 Registry Pattern (Replace the Switch Statement)

```rust
/// Trait implemented by all code generators.
pub trait CodeGenerator: Send + Sync {
    fn language_name(&self) -> &'static str;
    fn language_id(&self) -> LanguageId;
    fn file_extension(&self) -> &'static str;
    fn generate_class(&self, classifier: &Classifier, config: &GenConfig) -> Result<GeneratedFile, GenError>;
    fn default_datatypes(&self) -> Vec<DataTypeMapping>;
    fn reserved_keywords(&self) -> Vec<&'static str>;
    fn policy_schema(&self) -> Option<PolicySchema>;  // Per-language config schema
}

/// Thread-safe registry for language generators.
pub struct LanguageRegistry {
    generators: HashMap<LanguageId, Box<dyn CodeGenerator>>,
}

impl LanguageRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, gen: Box<dyn CodeGenerator>) -> Result<(), RegistryError>;
    pub fn get(&self, id: LanguageId) -> Option<&dyn CodeGenerator>;
    pub fn get_by_name(&self, name: &str) -> Option<&dyn CodeGenerator>;
    pub fn all_languages(&self) -> Vec<&dyn CodeGenerator>;
}
```

**Benefits over the C++ switch:**
- No central switch statement
- Registration can happen at compile time (via `inventory` or `linkme` crates) or at runtime
- New languages register themselves; the factory doesn't need to know about them
- Easy to query, list, and filter languages
- Can support "virtual" languages (combinations, aliases)

### 7.3 Template Engine Recommendation

**Primary recommendation: Askama** — compile-time template rendering with language-specific syntax.

```rust
// Askama template — compiled at build time
// templates/cpp/class_header.tera (or .askama)
{%- for include in includes %}
#include {{ include }}
{%- endfor %}

{%- if namespace %}
namespace {{ namespace }} {
{%- endif %}

class {{ class_name }}
{%- if base_class %} : public {{ base_class }}{%- endif %}
{
public:
    {%- for method in public_methods %}
    {{ method.signature }};
    {%- endfor %}
private:
    {%- for field in private_fields %}
    {{ field.type }} {{ field.name }}{%- if field.init %} = {{ field.init }}{%- endif %};
    {%- endfor %}
};

{%- if namespace %}
} // namespace {{ namespace }}
{%- endif %}
```

```rust
// Generated Rust code
#[derive(Template)]
#[template(path = "cpp/class_header.tera")]
struct CppClassHeader<'a> {
    includes: &'a [String],
    namespace: Option<&'a str>,
    class_name: &'a str,
    base_class: Option<&'a str>,
    public_methods: &'a [MethodInfo],
    private_fields: &'a [FieldInfo],
}
```

**Alternatives:**

| Engine | Strategy | Pros | Cons |
|---|---|---|---|
| **Askama** | Compile-time | Type-safe, fast, no runtime parsing | Template changes require recompilation |
| **Tera** | Runtime | Flexible, hot-reloadable, familiar syntax | Runtime errors, slower |
| **Handlebars** | Runtime | Logicless templates, widespread | Verbose for code gen |
| **Raw strings** | Programmatic | Full control, no deps | Same as C++ approach |

**Recommendation**: Use **Askama** for the 18 simple languages (templates are stable per language) and **Tera** for the advanced/document-model languages (where more dynamic structure is needed). Or, for maximum type safety, use Askama exclusively — the recompile cost is negligible since language templates change infrequently.

### 7.4 Programmatic Alternative (No Template Engine)

If templates are deemed too restrictive, a builder-pattern API can replace the template
engine while still being cleaner than the C++ string-concatenation approach:

```rust
/// Builder for a single source file.
pub struct SourceFile {
    lines: Vec<Line>,
    indent_level: usize,
}

pub enum Line {
    Text(String),
    Block { open: String, close: String, body: Vec<Line> },
    Comment(String),
    Blank,
}

impl SourceFile {
    pub fn new() -> Self;
    pub fn text(&mut self, s: impl Into<String>);
    pub fn block(&mut self, open: impl Into<String>, close: impl Into<String>, f: impl FnOnce(&mut Self));
    pub fn indent(&mut self, f: impl FnOnce(&mut Self));
    pub fn comment(&mut self, text: impl Into<String>);
    pub fn blank(&mut self);
    pub fn render(&self) -> String;
}

// Usage:
let mut file = SourceFile::new();
file.text("#include <iostream>");
file.blank();
file.block("class Foo {", "};", |f| {
    f.text("public:");
    f.indent(|f| {
        for method in &methods {
            f.text(format!("{} {}();", method.return_type, method.name));
        }
    });
});
let code = file.render();
```

This approach is simpler than the C++ `TextBlock` hierarchy and avoids template
engine dependencies, but still requires programmatic construction for each language.

### 7.5 CodeDocument as an AST (Enum-Based)

Replace the C++ `TextBlock` class hierarchy with a Rust enum AST:

```rust
/// A node in the code document tree.
pub enum CodeNode {
    Text(String),
    Block {
        open: String,
        close: String,
        children: Vec<CodeNode>,
    },
    Comment {
        style: CommentStyle,
        text: String,
    },
    Method {
        signature: MethodSignature,
        body: Vec<CodeNode>,
        is_abstract: bool,
    },
    Field {
        name: String,
        type_name: String,
        visibility: Visibility,
        initializer: Option<String>,
    },
    Include {
        path: String,
        kind: IncludeKind,  // System, Local
    },
    Namespace {
        name: String,
        children: Vec<CodeNode>,
    },
}

pub struct ClassifierCodeDocument {
    pub classifier_id: UmlId,
    pub language: LanguageId,
    pub file_name: String,
    pub nodes: Vec<CodeNode>,
    pub auto_generated: bool,
}
```

**Advantages over the C++ hierarchy:**
- Sum types (enums) are natural for tree structures in Rust
- Pattern matching provides exhaustive handling of all node types
- No `QObject` memory overhead — `CodeNode` is lightweight
- Serialization is trivial (derive `Serialize`/`Deserialize`)
- No diamond inheritance or multiple-inheritance issues

### 7.6 CodeWriter Trait

For the simple generation path, define a `CodeWriter` trait that mirrors the operations
a generator performs, providing a structured API:

```rust
/// Structured writer for code generation output.
pub trait CodeWriter {
    fn write_file_header(&mut self, config: &GenConfig) -> Result<(), GenError>;
    fn write_package(&mut self, package: &str) -> Result<(), GenError>;
    fn write_imports(&mut self, imports: &[Import]) -> Result<(), GenError>;
    fn write_class_header(&mut self, class: &ClassInfo) -> Result<(), GenError>;
    fn write_field(&mut self, field: &FieldInfo) -> Result<(), GenError>;
    fn write_method(&mut self, method: &MethodInfo) -> Result<(), GenError>;
    fn write_class_footer(&mut self) -> Result<(), GenError>;
    fn finalize(&mut self) -> Result<String, GenError>;
}

/// Implementations for each language.
pub struct CppWriter {
    output: String,
    indent_level: usize,
    config: GenConfig,
}

impl CodeWriter for CppWriter {
    fn write_file_header(&mut self, config: &GenConfig) -> Result<(), GenError> {
        writeln!(self.output, "// Auto-generated by Umbrello");
        writeln!(self.output, "// Language: C++");
        // ...
    }
    // ... remaining methods
}
```

This pattern works well when combined with the builder pattern for `SourceFile`:

```rust
pub struct CppWriter {
    file: SourceFile,
    config: GenConfig,
}

impl CodeWriter for CppWriter {
    fn write_file_header(&mut self, _config: &GenConfig) -> Result<(), GenError> {
        self.file.comment("Auto-generated by Umbrello");
        self.file.comment(format!("Language: {}", self.config.language));
        self.file.blank();
        Ok(())
    }
    // ...
}
```

### 7.7 Language Policy Configuration

Each language plugin should define its own configuration schema, parseable from a
`.toml` file:

```rust
/// Language-independent policy (all generators).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeGenConfig {
    pub output_directory: PathBuf,
    pub overwrite_mode: OverwriteMode,
    pub indent_style: IndentStyle,
    pub indent_width: usize,
    pub newline_style: NewlineStyle,
    pub comment_style: CommentStyle,
}

/// Language-specific configuration (per-plugin).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CppConfig {
    pub inline_accessors: bool,
    pub virtual_destructors: bool,
    pub use_namespace_as_package: bool,
    pub header_extension: String,      // Default: "h"
    pub source_extension: String,      // Default: "cpp"
    pub use_pragma_once: bool,
}

/// Unified configuration with per-language overrides.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationConfig {
    #[serde(flatten)]
    pub global: CodeGenConfig,
    pub languages: HashMap<String, serde_json::Value>,  // Per-language overrides
}
```

```toml
# Example: .umbrello-gen.toml
[global]
output_directory = "./src"
overwrite_mode = "ask"
indent_style = "spaces"
indent_width = 4
newline_style = "lf"
comment_style = "doxygen"

[languages.cpp]
inline_accessors = true
virtual_destructors = false
use_pragma_once = true

[languages.java]
auto_generate_accessors = true
```

### 7.8 Incremental Generation

Only regenerate files that have changed since the last generation. Use content hashing
to detect changes:

```rust
#[derive(Serialize, Deserialize)]
struct GenerationCache {
    files: HashMap<PathBuf, FileCacheEntry>,
}

#[derive(Serialize, Deserialize)]
struct FileCacheEntry {
    /// SHA-256 of the model state that produced this file
    model_hash: String,
    /// SHA-256 of the generated content
    content_hash: String,
    /// When the file was last generated
    generated_at: SystemTime,
}

/// Determines which files need regeneration.
fn compute_actions(
    cache: &GenerationCache,
    current_model_hash: &str,
    targets: &[GenerationTarget],
    overwrite: OverwriteMode,
) -> Vec<GenAction> {
    let mut actions = Vec::new();
    for target in targets {
        let file_path = target.output_path();
        let exists = file_path.exists();
        let entry = cache.files.get(&file_path);

        match (exists, entry) {
            (true, Some(e)) if e.model_hash == current_model_hash => {
                // Model unchanged; content is up-to-date
                actions.push(GenAction::Skip { reason: "unchanged".into() });
            }
            (true, Some(e)) if e.content_hash == compute_hash(&target.content) => {
                // Model changed but content is identical; update cache only
                actions.push(GenAction::UpdateCache);
            }
            (true, None) => {
                // File exists but not in cache — user-created or imported
                match overwrite {
                    OverwriteMode::Always => actions.push(GenAction::Regenerate),
                    OverwriteMode::Ask => actions.push(GenAction::Prompt),
                    OverwriteMode::Never => actions.push(GenAction::Skip { reason: "existing".into() }),
                }
            }
            (true, Some(_)) => {
                // Model changed and content differs
                actions.push(GenAction::Regenerate);
            }
            (false, _) => {
                // New file
                actions.push(GenAction::Generate);
            }
        }
    }
    actions
}
```

### 7.9 Proposed Workspace Layout

```
rust-rewrite/
├── Cargo.toml                              # Workspace root
├── umbrello-core/                           # Core types: Classifier, UML model, LanguageId
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── classifier.rs
│       ├── model.rs
│       └── language.rs
├── umbrello-codegen/                        # Code generation framework
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── traits.rs                        # CodeGenerator, CodeWriter traits
│       ├── registry.rs                      # LanguageRegistry
│       ├── source_file.rs                   # SourceFile builder
│       ├── code_document.rs                 # CodeNode AST enum
│       ├── config.rs                        # GenerationConfig, PolicySchema
│       ├── cache.rs                         # Incremental generation cache
│       └── error.rs                         # GenError types
├── generators/
│   ├── umbrello-gen-cpp/                    # C++ generator
│   │   ├── Cargo.toml
│   │   ├── templates/                       # Askama templates
│   │   │   ├── class_header.tera
│   │   │   └── class_source.tera
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── writer.rs
│   │       ├── types.rs                     # C++ type mappings
│   │       └── config.rs                    # CppConfig
│   ├── umbrello-gen-java/
│   │   └── ...
│   ├── umbrello-gen-python/
│   │   └── ...
│   └── ...                                  # One crate per language
└── umbrello-cli/                            # CLI for headless generation
    ├── Cargo.toml
    └── src/main.rs
```

---

## 8. Proposed Trait Design for Rust Code Generators

### 8.1 Core CodeGenerator Trait

```rust
/// Identifier for a programming language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageId {
    Ada,
    ActionScript,
    Cpp,
    CSharp,
    D,
    Idl,
    Java,
    JavaScript,
    MySql,
    Pascal,
    Perl,
    Php,
    Php5,
    PostgreSQL,
    Python,
    Ruby,
    Sql,
    Tcl,
    Vala,
    XmlSchema,
}

/// Result of code generation for a single classifier.
pub struct GeneratedFile {
    pub file_name: String,
    pub relative_path: PathBuf,
    pub content: String,
    pub language: LanguageId,
    pub classifier_id: UmlId,
}

/// Error type for code generation.
#[derive(Debug, thiserror::Error)]
pub enum GenError {
    #[error("Unsupported classifier type: {0}")]
    UnsupportedClassifier(String),
    #[error("Invalid name: {0}")]
    InvalidName(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Template error: {0}")]
    Template(#[from] askama::Error),
    #[error("Configuration error: {0}")]
    Config(String),
}

/// The primary trait for all code generators.
pub trait CodeGenerator: Debug + Send + Sync {
    /// Human-readable language name (e.g., "C++", "Python 3").
    fn display_name(&self) -> &'static str;

    /// Unique language identifier.
    fn language_id(&self) -> LanguageId;

    /// Default file extension without dot (e.g., "cpp", "py", "java").
    fn file_extension(&self) -> &'static str;

    /// Generate code for a single classifier.
    /// May produce multiple files (e.g., C++ header + source).
    fn generate(
        &self,
        classifier: &Classifier,
        config: &GenConfig,
    ) -> Result<Vec<GeneratedFile>, GenError>;

    /// Generate code for multiple classifiers.
    /// Default implementation calls `generate()` for each classifier.
    fn generate_all(
        &self,
        classifiers: &[&Classifier],
        config: &GenConfig,
    ) -> Result<Vec<GeneratedFile>, GenError> {
        let mut results = Vec::new();
        for c in classifiers {
            results.extend(self.generate(c, config)?);
        }
        Ok(results)
    }

    /// Default datatype mappings for this language
    /// (UML primitive type → language-specific type).
    fn default_datatypes(&self) -> Vec<(&'static str, &'static str)>;

    /// Reserved keywords that cannot be used as identifiers.
    fn reserved_keywords(&self) -> Vec<&'static str>;

    /// Language-specific configuration schema (for deserialization).
    fn config_schema(&self) -> Option<PolicySchema> {
        None
    }

    /// Whether this language supports the advanced (editable) document model.
    fn supports_document_model(&self) -> bool {
        false
    }
}
```

### 8.2 CodeWriter Trait (Alternative for Simple Languages)

```rust
/// Structured writer for building a source file line by line.
/// Used by generators that don't use templates.
pub trait CodeWriter: Debug {
    /// Write the file-level comment / license header.
    fn write_header(&mut self, config: &GenConfig) -> Result<(), GenError>;

    /// Write package/module/namespace declaration.
    fn write_package(&mut self, package: &str) -> Result<(), GenError>;

    /// Write import/include statements.
    fn write_imports(&mut self, imports: &[Import]) -> Result<(), GenError>;

    /// Begin a class/interface/enum declaration.
    fn write_type_header(&mut self, info: &TypeInfo) -> Result<(), GenError>;

    /// Write a field/attribute declaration.
    fn write_field(&mut self, field: &FieldInfo) -> Result<(), GenError>;

    /// Write a method/operation declaration.
    fn write_method(&mut self, method: &MethodInfo) -> Result<(), GenError>;

    /// End a class/interface/enum declaration.
    fn write_type_footer(&mut self) -> Result<(), GenError>;

    /// Write an enum literal.
    fn write_enum_literal(&mut self, name: &str, value: Option<&str>) -> Result<(), GenError>;

    /// Finalize and return the generated source code.
    fn finalize(&mut self) -> Result<String, GenError>;
}
```

### 8.3 Template-Based Generator (Auto-Impl of CodeGenerator)

For languages using templates, a helper macro or trait can auto-implement `CodeGenerator`:

```rust
/// A code generator that uses Askama templates.
pub struct TemplateGenerator {
    language_id: LanguageId,
    display_name: &'static str,
    extension: &'static str,
    type_mappings: HashMap<String, String>,
    keywords: HashSet<String>,
}

impl CodeGenerator for TemplateGenerator {
    fn generate(&self, classifier: &Classifier, config: &GenConfig) -> Result<Vec<GeneratedFile>, GenError> {
        let template = self.get_template(classifier)?;
        let rendered = template.render()?;
        let path = self.build_path(classifier, config)?;
        Ok(vec![GeneratedFile {
            file_name: path.to_string_lossy().to_string(),
            relative_path: path,
            content: rendered,
            language: self.language_id,
            classifier_id: classifier.id(),
        }])
    }
}
```

### 8.4 Registry Integration

```rust
/// Macro to register a generator at link time.
/// Uses the `inventory` crate for compile-time registration.
#[macro_export]
macro_rules! register_generator {
    ($gen:expr) => {
        inventory::submit! {
            GeneratorEntry::new(Box::new($gen))
        }
    };
}

/// Example usage in a generator crate:
/// generators/umbrello-gen-python/src/lib.rs
pub struct PythonGenerator { ... }

impl CodeGenerator for PythonGenerator { ... }

// Auto-register
register_generator!(PythonGenerator::new());
```

```rust
/// Linking-step registration collection.
pub struct GeneratorEntry {
    generator: Box<dyn CodeGenerator>,
}

inventory::collect!(GeneratorEntry);

impl LanguageRegistry {
    /// Create registry with all compile-time registered generators.
    pub fn with_builtin() -> Self {
        let mut reg = Self::new();
        for entry in inventory::iter::<GeneratorEntry> {
            let gen = entry.generator.as_ref();
            reg.register_dyn(gen.language_id(), ...);
        }
        reg
    }
}
```

### 8.5 File Path Resolution (Shared Utility)

```rust
/// Utility for resolving output file paths.
pub struct FilePathResolver {
    output_dir: PathBuf,
    use_folders: bool,   // Preserve package/namespace hierarchy
}

impl FilePathResolver {
    pub fn new(output_dir: PathBuf, use_folders: bool) -> Self;

    /// Resolve the output path for a classifier.
    /// E.g., "com.example.model" + "MyClass" + ".java"
    ///   → "./src/com/example/model/MyClass.java"
    pub fn resolve(
        &self,
        classifier: &Classifier,
        extension: &str,
    ) -> PathBuf {
        let package = classifier.package_name();
        let name = classifier.name();
        let mut path = self.output_dir.clone();
        if self.use_folders && !package.is_empty() {
            // Replace '.' or '::' with platform separator
            let sep = std::path::MAIN_SEPARATOR;
            let package_path = package.replace('.', &sep.to_string())
                                      .replace("::", &sep.to_string());
            path.push(package_path);
        }
        path.push(format!("{}.{}", name, extension));
        path
    }
}
```

---

## 9. Migration Strategy for Code Generators

### 9.1 Migration Principles

1. **Preserve existing behavior**: Generated output must be byte-identical (or acceptably
   similar) for the same input model.
2. **Incremental migration**: Not all 22 languages need to be ported at once.
3. **Parallel operation**: The Rust codegen can coexist with the C++ version during
   migration; the user chooses which to use.
4. **Test-driven**: Every generator must have a test suite that compares output against
   known-good C++ generator output.

### 9.2 Phase Plan

#### Phase 0: Foundation (Duration: ~4-6 weeks)

| Task | Deliverable | Dependencies |
|---|---|---|
| Define core types (`Classifier`, `LanguageId`, `UmlId`) | `umbrello-core` crate | None |
| Define `CodeGenerator` trait, `GenError`, `GeneratedFile` | `umbrello-codegen` traits | `umbrello-core` |
| Implement `LanguageRegistry` with `inventory` integration | `umbrello-codegen` registry | CodeGenerator trait |
| Implement `SourceFile` builder | `umbrello-codegen` source_file | None |
| Implement `CodeNode` AST enum | `umbrello-codegen` code_document | None |
| Implement `GenConfig` with Serde deserialization | `umbrello-codegen` config | None |
| Implement `GenerationCache` for incremental gen | `umbrello-codegen` cache | None |
| Write unit tests for all framework types | Tests | All of the above |

#### Phase 1: Template Engine Adoption (Duration: ~6-8 weeks)

| Task | Deliverable | Dependencies |
|---|---|---|
| Choose and integrate Askama | Build system setup | Phase 0 |
| Port SQL writer (simplest language) | `umbrello-gen-sql` crate | CodeGenerator trait, Askama |
| Port Ada writer | `umbrello-gen-ada` crate | SQL crate as reference |
| Port Python writer | `umbrello-gen-python` crate | Phase 0 |
| Port 4 more simple languages (Perl, Tcl, Pascal, IDL) | 4 generator crates | Template patterns |
| Port remaining 8 simple languages (AS, C#, JS, PHP4, PHP5, MySQL, PgSQL, Vala) | 8 generator crates | Template patterns |
| Port XML Schema writer | `umbrello-gen-xmlschema` crate | Template patterns |

**Verification for each language:**
```rust
#[test]
fn test_cpp_output_matches_legacy() {
    let model = load_test_model("simple_class.xmi");
    let rust_gen = CppGenerator::new();
    let legacy_output = load_legacy_output("simple_class.cpp");
    let rust_output = rust_gen.generate(&model.classifier("MyClass"), &default_config()).unwrap();
    assert_eq!(rust_output[0].content, legacy_output);
}
```

#### Phase 2: Advanced CodeGen Port (Duration: ~8-12 weeks)

This is the hardest phase — the 4 advanced languages (C++, D, Java, Ruby) with the
document model need special handling.

| Task | Deliverable | Dependencies |
|---|---|---|
| Port `CodeNode` AST to handle all document node types | Extended `CodeNode` enum | Phase 0 |
| Implement document-model traits (`CodeDocument`, `ClassCodeDocument`) | `umbrello-codegen` document module | `CodeNode` |
| Port C++ generator (header + source split) | `umbrello-gen-cpp` crate | Document model, Phase 1 templates |
| Port Java generator | `umbrello-gen-java` crate | Document model |
| Port D generator | `umbrello-gen-d` crate | Document model |
| Port Ruby generator | `umbrello-gen-ruby` crate | Document model |

**C++ port complexity:**
```
C++ generator scope (37 files):
├── CPPCodeGenerator       — main generator
├── CPPHeaderCodeDocument  — header document model
├── CPPSourceCodeDocument  — source document model
├── CPPHeaderCodeAccessorMethod
├── CPPSourceCodeAccessorMethod
├── CPPCodeClassFieldDeclarationBlock
├── CPPCodeOperation
├── cppheaderviewer.*      — KTextEditor integration
├── cppsourceviewer.*      — KTextEditor integration
├── cppcodeclassfield.*    — class field declarations
├── Type mapping files     — C++ type resolution
└── Tests                  — testcppwriter.cpp

Rust replacement: ~3-4 main source files + 2 template files
├── lib.rs                 — CodeGenerator impl
├── writer.rs              — writer logic (if not templates)
├── templates/
│   ├── header.tera        — header template
│   └── source.tera        — source template
├── types.rs               — type mappings
└── config.rs              — CppConfig
```

#### Phase 3: Configuration & Incremental Generation (Duration: ~2-3 weeks)

| Task | Deliverable | Dependencies |
|---|---|---|
| Implement TOML-based config loading | Config system | Phase 0 |
| Implement hash-based generation cache | `GenerationCache` | Phase 0 |
| Implement diff-based overwrite prompt | Interactive mode | Config system |
| Implement batch/headless CLI (`--generate`) | `umbrello-cli` crate | All generators |
| Implement project-level config (`.umbrello-gen.toml`) | Config autodiscovery | Config system |

#### Phase 4: Polish & Replace (Duration: ~4-6 weeks)

| Task | Deliverable | Dependencies |
|---|---|---|
| Validate all 22 generators against real-world models | Test suite | Phases 1-2 |
| Performance benchmark vs C++ generators | Benchmark suite | All generators |
| Add ROS / custom template support for user extensibility | User templates | Template engine |
| Remove C++ code generation subsystem (optional) | Cleanup | Full confidence in Rust replacement |
| Documentation for creating new language generators | Developer guide | Stable API |

### 9.3 Parallel Work Strategy

The phases can be parallelized:

```
Week:  0   2   4   6   8   10  12  14  16  18  20  22  24
      ┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐
Ph0   │░░░│░░░│░░░│   │   │   │   │   │   │   │   │   │   │
Ph1a  │   │   │░░░│░░░│░░░│░░░│   │   │   │   │   │   │   │  (SQL, Ada, Python, 4 more)
Ph1b  │   │   │   │   │░░░│░░░│░░░│░░░│   │   │   │   │   │  (remaining 8 simple)
Ph2a  │   │   │   │   │   │   │░░░│░░░│░░░│░░░│░░░│   │   │  (C++ + Java)
Ph2b  │   │   │   │   │   │   │   │   │░░░│░░░│░░░│░░░│   │  (D + Ruby)
Ph3   │   │   │   │   │   │   │   │   │   │░░░│░░░│░░░│   │
Ph4   │   │   │   │   │   │   │   │   │   │   │   │░░░│░░░│
      └───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┴───┘
```

Estimated total: **24-30 weeks** for complete migration.

### 9.4 Risk Mitigation

| Risk | Likelihood | Mitigation |
|---|---|---|
| Template output differs from legacy C++ output | Medium | Extensive test suite with golden files; byte-exact comparison |
| Document model (advanced gen) is hard to express in templates | Medium | Fall back to programmatic builder; `CodeNode` AST provides full flexibility |
| C++ header/source split logic is complex to restructure | High | Clone the C++ logic exactly first, then refactor; keep the two-file split |
| 22 crates create maintenance overhead | Low | Use workspace-level scripts (`cargo gen-all`); Bors/CI handles batch compilation |
| Language-specific edge cases missed | Medium | Test against real XMI files from the C++ test suite; community review per language |
| Performance regression compared to C++ | Low | Template engines are optimized; generation is I/O bound, not CPU bound |

---

## Appendix A: Key Files from the C++ Codebase

| File | Path | Lines | Role |
|---|---|---|---|
| CodeGenerator base | `umbrello/codegenerators/codegenerator.cpp` | ~200 | Abstract base class |
| SimpleCodeGenerator | `umbrello/codegenerators/simplecodegenerator.cpp` | ~150 | Stream-based generation |
| AdvancedCodeGenerator | `umbrello/codegenerators/advancedcodegenerator.cpp` | ~100 | Document model generation |
| CodeGenFactory | `umbrello/codegenerators/codegenfactory.cpp` | ~400 | Factory with 190-line switch |
| CodeGenerationPolicy | `umbrello/codegenerators/codegenpolicy.cpp` | ~300 | Language-independent policy |
| CodeGenPolicyExt | `umbrello/codegenerators/codegenpolicyext.cpp` | ~150 | Language-specific policy |
| CodeGenWizard | `umbrello/codegenwizard/` | ~500 | 3-page generation wizard |
| CodeDocument | `umbrello/codegenerators/codedocument.cpp` | ~300 | Document model base |
| ClassifierCodeDocument | `umbrello/codegenerators/classifiercodedocument.cpp` | ~200 | Per-classifier document |
| TextBlock | `umbrello/codegenerators/textblock.cpp` | ~100 | Base block node |
| CodeBlock | `umbrello/codegenerators/codeblock.cpp` | ~80 | Content block |
| HierarchicalCodeBlock | `umbrello/codegenerators/hierarchicalcodeblock.cpp` | ~100 | Container block |
| CodeMethodBlock | `umbrello/codegenerators/codemethodblock.cpp` | ~200 | Method body block |
| CodeClassField | `umbrello/codegenerators/codeclassfield.cpp` | ~200 | Class field representation |
| CodeOperation | `umbrello/codegenerators/codeoperation.cpp` | ~100 | Operation bridge |
| CodeParameter | `umbrello/codegenerators/codeparameter.cpp` | ~80 | Method parameter |

## Appendix B: Key Files for C++ Advanced Generator

| File | Path | Role |
|---|---|---|
| CPPCodeGenerator | `umbrello/codegenerators/cpp/cppcodegenerator.cpp` | Main C++ generator |
| CPPHeaderCodeDocument | `umbrello/codegenerators/cpp/cppheadercodedocument.cpp` | Header document model |
| CPPSourceCodeDocument | `umbrello/codegenerators/cpp/cppsourcecodedocument.cpp` | Source document model |
| CPPHeaderCodeAccessorMethod | `umbrello/codegenerators/cpp/cppheadercodeaccessormethod.cpp` | Header accessor methods |
| CPPSourceCodeAccessorMethod | `umbrello/codegenerators/cpp/cppsourcecodeaccessormethod.cpp` | Source accessor methods |
| CPPCodeClassFieldDeclBlock | `umbrello/codegenerators/cpp/cppcodeclassfielddeclarationblock.cpp` | Field declarations |
| CPPCodeOperation | `umbrello/codegenerators/cpp/cppcodeoperation.cpp` | C++ operation model |
| testcppwriter | `unittests/testcppwriter.cpp` | Test suite for C++ generation |

## Appendix C: Example Simple Generator Structure

Every simple generator follows this pattern (from `PascalWriter` as an example):

```cpp
// PascalWriter.h
class PascalWriter : public SimpleCodeGenerator {
    Q_OBJECT
public:
    ~PascalWriter() override;
protected:
    void writeClass(UMLClassifier *c, QIODevice &dev) override;
    Uml::ProgrammingLanguage language() const override;
    QStringList defaultDatatypes() override;
    QStringList reservedKeywords() override;
};

// PascalWriter.cpp
void PascalWriter::writeClass(UMLClassifier *c, QIODevice &dev) {
    QTextStream ts(&dev);
    ts << headerComment();
    // ... generate unit, interface, implementation sections
    for (UMLAttribute *attr : c->attributes()) {
        ts << "  " << attr->name() << " : " << typeName(attr) << ";\n";
    }
    for (UMLOperation *op : c->operations()) {
        ts << "  function " << op->name() << "(";
        // ... generate parameter list
        ts << "): " << typeName(op) << ";\n";
    }
    ts << footerComment();
}
```
