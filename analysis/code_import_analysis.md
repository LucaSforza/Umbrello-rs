# Code Import Subsystem Analysis

## 1. Import Architecture Overview

The code import subsystem is responsible for parsing source files in various
programming languages and creating corresponding UML model objects. It is a
pluggable architecture centered on a factory method and an abstract base class.

### Key components

| Component | File | Role |
|---|---|---|
| `ClassImport` (abstract base) | `codeimport/classimport.h` | Defines the import lifecycle: `initialize()` → `initPerFile()` → `parseFile()`. Holds `m_thread`, `m_enabled`, `m_rootPath`. |
| `ClassImport::createImporterByFileExt()` | `codeimport/classimport.cpp:40-67` | **Factory method** — selects importer by file extension. C++ is the default fallback. |
| `CodeImpThread` | `codeimpwizard/codeimpthread.h` | **Not a real QThread** despite its name. It is a `QObject` whose `run()` slot is invoked directly (not via `QThread::start()`). Wraps per-file import. |
| `CodeImportingWizard` | `codeimpwizard/codeimportingwizard.h` | 3-page wizard: select files → options → status. |

### Factory dispatch

The factory `createImporterByFileExt()` maps file extensions to importers:

```
.idl     → IDLImport
.py, .pyw → PythonImport
.java    → JavaImport
.ads, .adb, .ada → AdaImport
.pas     → PascalImport
.cs      → CSharpImport
.vala, .vapi, .vpa → ValaImport
.sql     → SQLImport
.php     → PHPImport (conditionally compiled, #ifdef ENABLE_PHP_IMPORT)
*        → CppImport (default fallback)
```

### Import lifecycle

```
importFiles(fileNames)
  │
  ├─ initialize()           ← one-time setup
  │
  ├─ for each fileName:
  │     ├─ initPerFile()    ← per-file reset
  │     └─ parseFile(file)  ← actual parsing + UML model creation
  │
  └─ finalize (done in UMLDoc state flags)
```

---

## 2. Parser Infrastructure Analysis

The subsystem uses **two fundamentally different parser architectures**:

### Architecture A: External parser — C++ (and PHP)

These languages delegate parsing to a full external parser library that produces
an AST, which is then traversed by a visitor that maps AST nodes to UML objects.

**C++ pipeline:**
```
Source file
  └─ CppImport::parseFile()
       └─ Driver::parseFile(fileName)     ← lib/cppparser/ parser
            ├─ Lexer (full preprocessor: #define, #ifdef, #include)
            └─ Parser (recursive descent)
                 └─ TranslationUnitAST     ← AST root (~50 node types)
       └─ CppImport::feedTheModel()
            └─ CppTree2Uml (TreeParser visitor)
                 └─ Import_Utils calls     ← creates UML objects
```

**PHP pipeline:**
```
Source file
  └─ PHPImport::parseFile()
       └─ PHPImportPrivate::parseFile()
            └─ Php::ParseSession          ← KDevelop PHP parser
                 ├─ Php::Lexer
                 └─ Php::Parser
                      └─ StartAst
       └─ PHPImport::feedTheModel()
            └─ PHPImportVisitor (DefaultVisitor subclass)
                 └─ Import_Utils calls
```

### Architecture B: Native line-by-line — all other languages

These languages use `NativeImportBase`, a hand-written lexer/parser framework
that processes files line-by-line.

**Pipeline:**
```
Source file
  └─ NativeImportBase::parseFile()
       ├─ initVars()                     ← reset state
       ├─ scan(line) for each line:
       │    ├─ preprocess(line)          ← strip comments, handle multi-line
       │    ├─ split(line)               ← tokenize into words
       │    └─ fillSource(word)          ← language-specific token handling
       │         └─ m_source.append()    ← build token list
       ├─ pushScope(globalScope)
       └─ for each token in m_source:
            ├─ parseStmt()               ← language-specific parsing
            └─ skipStmt()                ← fallback: skip to ";"
```

**State maintained by NativeImportBase:**
- `m_source[QStringList]` — scanned tokens
- `m_srcIndex` — cursor into token list
- `m_scope[QList<UMLPackage*>]` — scope stack (pushScope/popScope)
- `m_klass` — current classifier being processed
- `m_currentAccess` — current visibility (public/protected/private)
- `m_comment` — accumulated comment text
- `m_inComment` — multi-line comment state
- `m_isAbstract` — abstractness accumulator

---

## 3. Language-Specific Importers Analysis

### 3.1 C++ Import (`CppImport`)

**File:** `codeimport/cppimport.cpp`, `lib/cppparser/` (23 files)

- Depends on the self-contained `lib/cppparser/` library (KDevelop-derived).
- The parser has a **full C preprocessor**: `#define`, `#ifdef`, `#include`, macros.
- `ms_driver` (static `CppDriver`) manages all parsed translation units, include
  paths, and macro state across multiple files.
- `CppTree2Uml` is a `TreeParser` visitor that recursively walks the AST and
  calls `Import_Utils` functions.
- **Dependency resolution**: `feedTheModel()` recursively processes `#include`
  dependencies before the including file, so inner includes are fed to the model
  first.
- Static state (`ms_driver`, `ms_seenFiles`) means state leaks between import
  sessions — `initialize()` calls `ms_driver->reset()` to clear it.

**C++ AST node types (~50):** `TranslationUnitAST`, `DeclarationAST`,
`ClassSpecifierAST`, `EnumSpecifierAST`, `FunctionDefinitionAST`,
`SimpleDeclarationAST`, `TemplateDeclarationAST`, `NamespaceAST`,
`UsingAST`, `TypedefAST`, `BaseSpecifierAST`, `ParameterDeclarationAST`,
`DeclaratorAST`, etc.

### 3.2 PHP Import (`PHPImport`)

**File:** `codeimport/phpimport.cpp`

- **Conditionally compiled** (`#ifdef ENABLE_PHP_IMPORT`), depends on
  `lib/kdev5-php/` (KDevelop PHP parser) and `lib/kdevplatform/`.
- Uses the KDevelop-PG-Qt parser framework (`Php::ParseSession`, `Php::Lexer`,
  `Php::Parser`).
- `PHPImportVisitor` extends `DefaultVisitor` (KDevelop's visitor base).
- Imports: `#include <parser/parsesession.h>`, `<parser/phplexer.h>`,
  `<parser/phpparser.h>`, `<parser/phpast.h>`, `<parser/tokenstream.h>`.
- Also depends on KDevelop platform libraries: `DUChain`, `TestCore`,
  `AutoTestShell`.
- **Heavy dependency chain**: requires KDevelop PHP support library +
  KDevelop platform infrastructure to be compiled and available.

### 3.3 Java, C#, Vala Import

**Hierarchy:**
```
NativeImportBase
  └─ JavaCsValaImportBase (codeimport/javacsvalaimportbase.h)
       ├─ JavaImport
       └─ CsValaImportBase (codeimport/csvalaimportbase.h)
            ├─ CSharpImport
            └─ ValaImport
```

**`JavaCsValaImportBase`** adds:
- `fillSource()` — tokenizer for Java/C#/Vala syntax
- `parseClassDeclaration()`, `parseEnumDeclaration()` — type declaration parsers
- `resolveClass()` — resolve class names against imports
- `joinTypename()` — reconstruct qualified type names
- `spawnImport()` — virtual for language-specific import of referenced files
- Static `s_filesAlreadyParsed` / `s_parseDepth` — cross-file dedup
- `m_currentPackage`, `m_imports` — current file context

**`CsValaImportBase`** adds:
- `parseStmt()` — C#/Vala statement parser
- `preprocess()` — C# preprocessor directives
- `parseUsingDirectives()`, `parseNamespaceDeclaration()`, `parseAnnotation()`
- `parseStructDeclaration()`, `parseDelegateDeclaration()`
- Modifier check methods: `isTypeDeclaration()`, `isClassModifier()`,
  `isCommonModifier()`

### 3.4 Python Import (`PythonImport`)

- **Indentation-driven**: transforms Python indentation into brace-like block
  markers (`m_braceWasOpened`).
- `m_srcIndent[100]` / `m_srcIndentIndex` track indentation levels.
- `preprocess()` override handles indentation-to-brace conversion.
- `fillSource()` override — Python-specific tokenizer that inserts `{`/`}` and
  `;` tokens based on indentation changes.
- `parseInitializer()`, `parseAssignmentStmt()`, `parseMethodParameters()`

### 3.5 Ada Import (`AdaImport`)

- `split()` override — Ada-specific tokenization (operators like `=>`, `..`,
  `<>` are single tokens).
- `fillSource()` builds token list.
- `expand()` resolves Ada package renames.
- `parseStems()` handles Ada's `with` clauses (package dependencies).
- `m_renaming` (QMap<String, String>) maps package renames to expanded names.
- `m_classesDefinedInThisScope` helps distinguish primitive vs. non-primitive
  methods.

### 3.6 Pascal Import (`PascalImport`)

- `split()` override for Pascal syntax.
- `checkModifiers()` extracts `virtual`/`abstract` from method declarations.
- Tracks `m_inInterface` (interface vs. implementation section).
- `Section_Type` enum tracks current section (label/const/type/var/threadvar).

### 3.7 IDL Import (`IDLImport`)

- **External C preprocessor**: uses `QProcess` to run the system C preprocessor
  (`cpp`/`gcc -E`), stored in `m_preProcessor` (discovered once, static).
- `parseFile()` override: spawns cpp, reads preprocessed output.
- `preprocess()` override handles CORBA IDL-specific preprocessing.
- `skipStructure()`, `isValidScopedName()`, `joinTypename()`
- Flags: `m_isOneway`, `m_isReadonly`, `m_isAttribute`, `m_isUnionDefault`
- `m_unionCases` tracks CORBA union case labels.

### 3.8 SQL Import (`SQLImport`)

- Creates `UMLEntity` objects (database tables) rather than `UMLClassifier`.
- Overrides `advance()` (SQL-specific token advancement).
- Rich parsing methods:
  - `parseCreateTable()`, `parseAlterTable()` — DDL statement parsing
  - `parseFieldType()`, `parseColumnConstraints()`, `parseTableConstraints()`
  - `parseDefaultExpression()`, `parseIdentifier()`
  - `addDatatype()`, `addPrimaryKey()`, `addUniqueConstraint()`,
    `addForeignConstraint()`
- `ColumnConstraints` / `TableConstraints` inner structs capture DDL detail.

---

## 4. AST → UML Mapping Analysis

The bridge between parsers and UML model objects is the **`Import_Utils`**
namespace (`codeimport/import_utils.h`).

### Key mapping functions

| Function | Purpose |
|---|---|
| `createUMLObject(type, name, parentPkg)` | Find-or-create any UML object type. Handles C++ adorned types (const, volatile, *, &) by creating `UMLDatatype` with `originType`. Handles scoped names (e.g. `std::string`). |
| `createUMLObjectHierarchy(type, name, topLevelParent)` | Create scoped names as nested packages/classes. |
| `insertAttribute(klass, scope, name, type)` | Find-or-create a `UMLAttribute` on a classifier. |
| `makeOperation(parent, name)` | Create a `UMLOperation` without emitting signals. |
| `insertMethod(klass, op, scope, type, ...)` | Create-or-merge a method with signature matching. |
| `addMethodParameter(method, type, name)` | Add a parameter to an operation. |
| `addEnumLiteral(enumType, literal, comment, value)` | Add an enum constant. |
| `createGeneralization(child, parent)` | Create generalization/realization associations. |
| `formatComment(comment)` | Strip `/* ... */` markers and leading stars. |
| `checkStdString(typeName)` | Map `std::string` → `string`. |

### C++ type adornment handling

When `createUMLObject()` encounters adorned types like `const QString&`,
`volatile int*`, it:

1. Strips adornments to find the base type.
2. Creates the base type as a UML object.
3. Creates a `UMLDatatype` with the full adorned name.
4. Sets `originType` on the adorned datatype pointing to the base type.
5. Sets `isReference` flag for `&` and `*` types.

### Scope management

- `NativeImportBase::pushScope()` / `popScope()` maintain a `QList<UMLPackage*>`
  stack (index 0 = global scope).
- C++ `CppTree2Uml` uses stack arrays `m_currentNamespace[STACKSIZE+1]` and
  `m_currentClass[STACKSIZE+1]` with `m_nsCnt`/`m_clsCnt` as stack pointers.
- PHP uses `QVector<QPointer<UMLPackage>> m_currentNamespace[NamespaceSize]`.

### Global mutable state

`Import_Utils` uses several file-scope global variables that are manipulated
before calling creation functions:
- `bNewUMLObjectWasCreated` — whether `createUMLObject()` created a new object.
- `gRelatedClassifier` — for creating template parameter dependencies.
- `bPutAtGlobalScope` — force creation at global scope.
- `incPathList` — include path list for file resolution.

This global mutable state is an area of concern for the Rust rewrite.

---

## 5. Import Workflow / Pipeline

### Full workflow from user action

```
User action: File → Import → "Import Source Files..."
  │
  ├─ CodeImportingWizard opens
  │    ├─ Page 1: CodeImpSelectPage  — file selection, language filter
  │    ├─ Page 2: CodeImpOptionsPage — options (create artifacts, resolve deps)
  │    └─ Page 3: CodeImpStatusPage  — progress display
  │
  ├─ For each selected file:
  │    └─ CodeImpThread::run()      ← invoked as slot, not real thread
  │         └─ ClassImport::createImporterByFileExt(fileName, thread)
  │              └─ classImporter->importFile(fileName)
  │                   ├─ initPerFile()
  │                   └─ parseFile(fileName)    ← language-specific
  │
  └─ Done. UML model now contains classes, attributes, operations, etc.
```

### Thread model

`CodeImpThread` extends `QObject`, not `QThread`. The TODO comment in
`codeimpthread.h` states: *"For a start it is only a QObject and is used to
signals messages."*

The import runs in the main thread, with signals used to:
- Log messages to the status page.
- Ask yes/no questions via `UmlMessageBox`.
- Report per-file progress.

### File deduplication

- `NativeImportBase::m_parsedFiles` (static `QStringList`) — prevents parsing
  the same file twice across the session.
- `JavaCsValaImportBase::s_filesAlreadyParsed` (static `QStringList`) — same
  purpose for the Java/C#/Vala family.
- `CppImport::ms_seenFiles` (static `QStringList`) — prevents re-processing
  files in C++ include dependency resolution.

### Include path resolution

- `Import_Utils::addIncludePath()` / `Import_Utils::includePathList()` manage
  a global include path list.
- `UMBRELLO_INCPATH` environment variable contributes to include paths.
- `NativeImportBase::parseFile()` searches include paths for relative filenames.
- C++ `Driver` has its own include path management.

---

## 6. Dependency Analysis on External Parsing Libraries

### 6.1 lib/cppparser/ (C++ parser)

- **23 files** in `lib/cppparser/`:
  `ast.h/.cpp`, `ast_utils.h/.cpp`, `cachemanager.h/.cpp`, `driver.h/.cpp`,
  `errors.h/.cpp`, `keywords.h`, `lexer.h/.cpp`, `lexercache.h/.cpp`,
  `lookup.h/.cpp`, `macro.h`, `parser.h/.cpp`, `tree_parser.h/.cpp`, `README`.
- **Self-contained**: no external dependencies beyond Qt and STL.
- **Mature**: originally from KDevelop (Roberto Raggi, 2002-2003), forked and
  maintained within Umbrello.
- **Full C++ parser**: handles templates, preprocessor, macros, C++11/14 features.
- **Bespoke code**: hand-written lexer and recursive-descent parser.

### 6.2 lib/kdev5-php/ (PHP parser)

- **External dependency** on KDevelop PHP infrastructure:
  - `lib/kdev5-php/` — PHP language support plugin (parser, lexer, AST).
  - `lib/kdevplatform/` — KDevelop platform (DUChain, TestCore, etc.).
- **Conditional compilation**: `#ifdef ENABLE_PHP_IMPORT` — disabled if
  dependencies are not met.
- **Heavy weight**: initializes `KDevelop::AutoTestShell`, `KDevelop::TestCore`,
  and `KDevelop::DUChain` — all KDevelop platform services.
- **KDevelop-PG-Qt**: uses the KDevelop parser generator framework.

### 6.3 Native Import Base languages

All other languages (Java, Python, Ada, Pascal, C#, Vala, IDL, SQL) use
**zero external parsing libraries**. Parsing is done entirely with hand-written
code in `NativeImportBase` derivatives. The only external dependency is:
- **IDL**: uses `QProcess` to shell out to the system C preprocessor.

---

## 7. Rust Recommendations

### 7.1 Parser technology selection

#### C++: use tree-sitter with `tree-sitter-cpp`

**Rationale:**
- `tree-sitter-cpp` grammar is mature, well-maintained, and supports C++11/14/17/20.
- tree-sitter provides **incremental parsing** — re-parse only changed regions.
- **Error recovery**: tree-sitter can produce a parse tree even from invalid
  code, which is important for partial/incomplete source files.
- Tree-sitter has first-class Rust support via the `tree-sitter` crate.
- Replaces 23 bespoke C++ parser files with a single grammar dependency.

**Crates:**
```toml
[dependencies]
tree-sitter = "0.24"
tree-sitter-cpp = "0.23"  # language grammar
```

#### PHP: use tree-sitter with `tree-sitter-php`

- Eliminates the heavy KDevelop dependency chain.
- No more conditional compilation / `#ifdef ENABLE_PHP_IMPORT`.
- Single consistent API across all languages.

#### Java, C#, Python: use tree-sitter grammars

- `tree-sitter-java`, `tree-sitter-c-sharp`, `tree-sitter-python` all exist
  and are well-maintained.
- Would replace the hand-written line-by-line parsers with proper ASTs.

#### Ada, Pascal, IDL, SQL: tree-sitter or hand-written

- `tree-sitter-ada` exists but is less mature.
- Pascal, IDL, SQL grammars exist but may need evaluation.
- **Alternative**: keep the line-by-line approach for these less common
  languages if tree-sitter grammars are not suitable, but refactor using
  Rust idioms (e.g., `logos` crate for lexing).

### 7.2 Plugin architecture via traits

```rust
/// Result of analyzing a single file: a batch of model mutations.
pub struct UmlChange {
    pub change_type: ChangeType,       // Create, Update, Delete
    pub object_type: UmlObjectType,     // Class, Interface, Enum, Attribute, etc.
    pub qualified_name: Vec<String>,    // e.g. ["com", "example", "MyClass"]
    pub properties: HashMap<String, Value>,
    pub children: Vec<UmlChange>,
}

/// A single language importer.
pub trait LanguageImporter: Send + Sync {
    /// Return file extensions this importer handles.
    fn file_extensions(&self) -> &[&str];

    /// Parse a single file and return a batch of model changes.
    fn parse_file(&self, file_path: &Path, source: &str) -> Result<Vec<UmlChange>, ImportError>;

    /// Optional: parse a file referenced by an import/include directive.
    fn parse_dependency(&self, file_path: &Path, source: &str)
        -> Result<Vec<UmlChange>, ImportError>
    {
        self.parse_file(file_path, source)
    }
}

/// Registry of language importers.
pub struct ImportRegistry {
    importers: Vec<Box<dyn LanguageImporter>>,
}

impl ImportRegistry {
    pub fn importer_for(&self, ext: &str) -> Option<&dyn LanguageImporter> {
        self.importers.iter().find(|i| i.file_extensions().contains(&ext))
    }
}
```

### 7.3 Tokenizer / Lexer

For languages that warrant a custom lexer rather than tree-sitter:

```rust
/// Use the `logos` crate for fast, zero-allocation lexing.
use logos::Logos;

#[derive(Logos, Debug, PartialEq)]
pub enum Token {
    #[token("class")]
    Class,
    #[token("interface")]
    Interface,
    #[token("enum")]
    Enum,
    #[token("extends")]
    Extends,
    #[token("implements")]
    Implements,
    #[token("{")]
    OpenBrace,
    #[token("}")]
    CloseBrace,
    #[token(";")]
    Semicolon,
    #[regex(r"[a-zA-Z_]\w*", |lex| lex.slice().to_string())]
    Identifier(String),
    #[regex(r"[ \t\n\r]+", logos::skip)]
    Whitespace,
    // ...
}
```

### 7.4 Language-agnostic Intermediate AST

Define a common intermediate AST that all language parsers produce, then map
this IAST to UML changes:

```rust
/// Language-agnostic intermediate AST for code import.
pub enum Declaration {
    Package {
        name: Vec<String>,
        children: Vec<Declaration>,
    },
    Class {
        name: String,
        access_modifier: AccessModifier,
        is_abstract: bool,
        is_static: bool,
        extends: Vec<TypeRef>,
        implements: Vec<TypeRef>,
        type_parameters: Vec<TypeParameter>,
        members: Vec<Member>,
    },
    Interface {
        name: String,
        access_modifier: AccessModifier,
        extends: Vec<TypeRef>,
        type_parameters: Vec<TypeParameter>,
        members: Vec<Member>,
    },
    Enum {
        name: String,
        access_modifier: AccessModifier,
        literals: Vec<EnumLiteral>,
    },
    // ...
}

pub enum Member {
    Field {
        name: String,
        type_ref: TypeRef,
        access_modifier: AccessModifier,
        is_static: bool,
    },
    Method {
        name: String,
        return_type: Option<TypeRef>,
        parameters: Vec<Parameter>,
        access_modifier: AccessModifier,
        is_static: bool,
        is_abstract: bool,
    },
}

pub struct TypeRef {
    pub name: String,
    pub type_arguments: Vec<TypeRef>,
    pub is_const: bool,
    pub is_pointer: bool,
    pub is_reference: bool,
}
```

### 7.5 Import as batch of model mutations

Instead of directly manipulating the UML model during parsing (as the current
C++ code does with `Import_Utils`), the Rust rewrite should:

1. **Parse files to intermediate AST** — no model side effects.
2. **Convert AST to a batch of `UmlChange` values** — pure transformation.
3. **Apply the batch atomically** to the UML model — single commit.

```rust
/// Parse a batch of files and produce a set of model changes.
pub fn analyze_files(
    files: &[FileEntry],
    registry: &ImportRegistry,
) -> Result<Vec<UmlChange>, ImportError> {
    let mut changes = Vec::new();
    for file in files {
        let importer = registry.importer_for(&file.extension)
            .ok_or(ImportError::NoImporter(file.path.clone()))?;
        let file_changes = importer.parse_file(&file.path, &file.source)?;
        changes.extend(file_changes);
    }
    // Resolve cross-references between changes.
    resolve_cross_references(&mut changes);
    Ok(changes)
}
```

**Benefits:**
- Parsing is pure and testable without needing a UML model.
- Cross-file references can be resolved in a second pass.
- Batch application allows undo/redo (single command).
- Streaming possible for large codebases: parse one file at a time, produce
  changes incrementally.

### 7.6 Streaming for large codebases

```rust
/// Process files incrementally, yielding batches of changes.
pub fn process_streaming(
    files: impl Iterator<Item = FileEntry>,
    registry: &ImportRegistry,
) -> impl Iterator<Item = Result<Vec<UmlChange>, ImportError>> {
    files.filter_map(move |file| {
        let importer = registry.importer_for(&file.extension)?;
        Some(importer.parse_file(&file.path, &file.source))
    })
}
```

### 7.7 Tree-sitter advantages summary

| Feature | Benefit |
|---|---|
| Incremental parsing | Re-parse only changed files on re-import |
| Error recovery | Parse partial/invalid code gracefully |
| 100+ language grammars | One technology for all supported languages |
| First-class Rust support | `tree-sitter` crate, no FFI complexity |
| Active ecosystem | Grammars maintained by language communities |
| CST (Concrete Syntax Tree) | Preserve full source structure for round-trip |

### 7.8 Crate selection summary

| Purpose | Crate | Notes |
|---|---|---|
| C++ parsing | `tree-sitter` + `tree-sitter-cpp` | Replace 23-file bespoke parser |
| Java parsing | `tree-sitter` + `tree-sitter-java` | Replace hand-written line-by-line |
| Python parsing | `tree-sitter` + `tree-sitter-python` | Replace indentation hack |
| C# parsing | `tree-sitter` + `tree-sitter-c-sharp` | Replace line-by-line parser |
| PHP parsing | `tree-sitter` + `tree-sitter-php` | Remove KDevelop dependency |
| Ada/Pascal/IDL/SQL | `tree-sitter` or hand-written | Evaluate grammar quality |
| Lexing (for custom parsers) | `logos` | Zero-allocation, derive macro |
| Error handling | `thiserror` | Idiomatic error types |
| Serialization | `serde` + `serde_json` | For UmlChange persistence |

---

## 8. Proposed Trait Design for Rust Importers

```rust
// === Core traits ===

/// Error types for code import.
#[derive(thiserror::Error, Debug)]
pub enum ImportError {
    #[error("no importer available for file: {0}")]
    NoImporter(PathBuf),

    #[error("parse error in {path}:{line}:{column}: {message}")]
    ParseError {
        path: PathBuf,
        line: usize,
        column: usize,
        message: String,
    },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("tree-sitter error: {0}")]
    TreeSitter(#[from] tree_sitter::LanguageError),
}

/// A single model mutation produced by parsing a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmlChange {
    pub change_type: ChangeType,
    pub object_type: UmlObjectType,
    pub qualified_name: Vec<String>,
    pub properties: BTreeMap<String, serde_json::Value>,
    pub children: Vec<UmlChange>,
    pub associations: Vec<UmlAssociation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChangeType {
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UmlObjectType {
    Package,
    Class,
    Interface,
    Enum,
    Attribute,
    Operation,
    Parameter,
    Association,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UmlAssociation {
    pub assoc_type: AssociationType,
    pub source: Vec<String>,
    pub target: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AssociationType {
    Generalization,
    Realization,
    Dependency,
    Association,
    Aggregation,
    Composition,
}

/// Language-specific importer.
#[async_trait]
pub trait LanguageImporter: Send + Sync {
    /// File extensions this importer handles (e.g. [".cpp", ".cxx", ".h"]).
    fn file_extensions(&self) -> &[&str];

    /// Parse a single file into model changes.
    fn parse_file(
        &self,
        path: &Path,
        source: &str,
    ) -> Result<Vec<UmlChange>, ImportError>;

    /// Optional: resolve include/import directives to produce additional changes.
    fn resolve_dependencies(
        &self,
        path: &Path,
        source: &str,
        _changes: &mut Vec<UmlChange>,
    ) -> Result<(), ImportError> {
        // Default: no-op
        Ok(())
    }

    /// Human-readable language name.
    fn display_name(&self) -> &str {
        "Unnamed Importer"
    }
}

/// Common tree-sitter based importer.
pub struct TreeSitterImporter {
    language: tree_sitter::Language,
    extensions: Vec<String>,
    name: String,
    // Optional: query patterns for extracting UML-relevant nodes
    queries: Option<tree_sitter::Query>,
}

impl TreeSitterImporter {
    pub fn new(
        language: tree_sitter::Language,
        extensions: &[&str],
        name: &str,
    ) -> Self {
        Self {
            language,
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
            name: name.to_string(),
            queries: None,
        }
    }
}

impl LanguageImporter for TreeSitterImporter {
    fn file_extensions(&self) -> &[&str] {
        &self.extensions.iter().map(|s| s.as_str()).collect::<Vec<_>>()
    }

    fn parse_file(
        &self,
        path: &Path,
        source: &str,
    ) -> Result<Vec<UmlChange>, ImportError> {
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&self.language)?;

        let tree = parser.parse(source, None)
            .ok_or_else(|| ImportError::ParseError {
                path: path.to_path_buf(),
                line: 0, column: 0,
                message: "failed to parse source".to_string(),
            })?;

        let changes = self.walk_cst(tree.root_node(), source)?;
        Ok(changes)
    }
}

/// Registry that holds all importers.
pub struct ImportRegistry {
    importers: Vec<Box<dyn LanguageImporter>>,
}

impl ImportRegistry {
    pub fn new() -> Self {
        Self { importers: Vec::new() }
    }

    pub fn register(&mut self, importer: Box<dyn LanguageImporter>) {
        self.importers.push(importer);
    }

    pub fn importer_for(&self, extension: &str) -> Option<&dyn LanguageImporter> {
        let ext = extension.trim_start_matches('.');
        self.importers.iter()
            .find(|i| i.file_extensions().contains(&ext))
            .map(|b| b.as_ref())
    }

    pub fn all_importers(&self) -> Vec<&dyn LanguageImporter> {
        self.importers.iter().map(|b| b.as_ref()).collect()
    }
}
```

---

## 9. Migration Strategy

### Phase 1: Foundation (completed in isolation)

1. Define the `LanguageImporter` trait, `UmlChange`, `ImportError` types.
2. Implement `ImportRegistry`.
3. Create the `TreeSitterImporter` base using `tree-sitter` crate.
4. Set up the `logos`-based lexer for custom parsers.

### Phase 2: Rewrite core importers (one at a time)

Each language importer is rewritten independently, allowing parallel development.

1. **C++** — `TreeSitterImporter` with `tree-sitter-cpp` grammar. This is the
   highest value replacement (eliminates 23 files of bespoke parser code).

2. **Java** — `TreeSitterImporter` with `tree-sitter-java` grammar. Replaces
   `JavaImport` + `JavaCsValaImportBase` shared logic.

3. **C#** — `TreeSitterImporter` with `tree-sitter-c-sharp` grammar. Replaces
   `CSharpImport` + `CsValaImportBase` + `JavaCsValaImportBase`.

4. **Python** — `TreeSitterImporter` with `tree-sitter-python` grammar.
   Eliminates the indentation-to-brace hack.

5. **PHP** — `TreeSitterImporter` with `tree-sitter-php` grammar. Eliminates
   the KDevelop dependency chain entirely.

### Phase 3: Rewrite less common importers

1. **Ada** — Evaluate `tree-sitter-ada` grammar. Fallback: keep hand-written
   but port to Rust using `logos` for lexing.

2. **Pascal** — Evaluate `tree-sitter-pascal`. Fallback: Rust rewrite using
   the `NativeImportBase` pattern in Rust idiom.

3. **IDL** — Evaluate `tree-sitter-idl` (CORBA IDL). Fallback: Rust rewrite
   using `logos` + manual parser.

4. **SQL** — Evaluate `tree-sitter-sql` (PostgreSQL/MySQL dialect). Fallback:
   Rust rewrite using `logos` + manual parser.

### Phase 4: Integration

1. Wire `ImportRegistry` into the import wizard.
2. Replace `ClassImport::createImporterByFileExt()` with `ImportRegistry`.
3. `UmlChange` batch application to the UML model.
4. Add undo/redo support via the batch mutation model.

### Risk assessment

| Risk | Likelihood | Mitigation |
|---|---|---|
| `tree-sitter-cpp` grammar misses C++ features | Low | The grammar is actively maintained; can fall back to the existing parser for edge cases during transition |
| `tree-sitter` error recovery produces incomplete AST | Medium | Validate parsed AST structure; fall back to partial import |
| Missing tree-sitter grammar for Ada/Pascal/IDL/SQL | Medium | Keep hand-written Rust parsers using `logos` |
| Performance regression vs. line-by-line parsers | Low | tree-sitter is highly optimized; microbenchmarks during development |
| Loss of C++ type adornment logic (const/volatile/*/&) | Medium | Port `Import_Utils::createUMLObject()` adornment logic into a dedicated `TypeMapper` component |
| Cross-file reference resolution complexity | Medium | Two-pass design: parse all files, then resolve references |

### Testing strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpp_class_import() {
        let source = r#"
            class Foo {
            public:
                int bar();
            private:
                int x_;
            };
        "#;

        let importer = TreeSitterImporter::new(
            tree_sitter_cpp::language(),
            &["cpp", "h"],
            "C++",
        );
        let changes = importer.parse_file(
            Path::new("test.cpp"),
            source,
        ).unwrap();

        assert_eq!(changes.len(), 1);
        if let UmlChange { change_type: ChangeType::Create,
                           object_type: UmlObjectType::Class,
                           qualified_name, .. } = &changes[0] {
            assert_eq!(qualified_name, &["Foo"]);
        } else {
            panic!("expected class creation");
        }
    }

    #[test]
    fn test_java_import_with_package() {
        let source = r#"
            package com.example;
            public class MyClass {
                private String name;
            }
        "#;

        let importer = TreeSitterImporter::new(
            tree_sitter_java::language(),
            &["java"],
            "Java",
        );
        let changes = importer.parse_file(
            Path::new("MyClass.java"),
            source,
        ).unwrap();

        // Should produce: package change + class change + attribute change
        assert!(changes.len() >= 3);
    }

    #[test]
    fn test_importer_registry() {
        let mut registry = ImportRegistry::new();
        registry.register(Box::new(
            TreeSitterImporter::new(tree_sitter_cpp::language(), &["cpp", "h"], "C++")
        ));
        registry.register(Box::new(
            TreeSitterImporter::new(tree_sitter_java::language(), &["java"], "Java")
        ));

        assert!(registry.importer_for("cpp").is_some());
        assert!(registry.importer_for("java").is_some());
        assert!(registry.importer_for("py").is_none());
    }

    #[test]
    fn test_cross_file_reference_resolution() {
        // Two files: Foo.h declares class Foo, Bar.h extends Foo.
        // The resolved changes should produce a generalization between Bar and Foo.
        // This test verifies the two-pass resolution logic.
    }
}
```
