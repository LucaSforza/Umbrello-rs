# Build System Analysis — Umbrello to Rust rewrite

Date: 2026-06-23  
Author: Rust rewrite team  
Status: Draft

---

## 1. Current CMake Structure Analysis

### 1.1 Top-level layout

The project root `CMakeLists.txt` (321 lines) orchestrates the entire build:

```
umbrello/
├── CMakeLists.txt              (root, 321 lines)
├── cmake/modules/
│   ├── Macros.cmake            (icon generation from SVG)
│   ├── QtVersionOption.cmake   (Qt5/Qt6 selection)
│   └── ECMAddTests.cmake       (local ECM test helpers)
├── umbrello/
│   ├── CMakeLists.txt          (585 lines — main library + executable)
│   ├── codeimport/
│   │   └── CMakeLists.txt      (79 lines — static lib)
│   ├── codegenerators/         (15+ language writers, ~150 files)
│   ├── dialogs/                (~80 source files)
│   ├── umlmodel/               (~40 source files)
│   ├── umlwidgets/             (~40 source files)
│   └── ...
├── lib/
│   ├── cppparser/              (C++ parser for code import)
│   ├── kdevplatform/           (KDevelop platform, PHP import)
│   └── interfaces/             (shared interfaces)
├── unittests/                  (13 test executables)
├── tools/                      (CI scripts, Docker)
├── po/                         (62 languages)
└── doc/                        (documentation)
```

### 1.2 Targets

| Target | Type | Description |
|--------|------|-------------|
| `libumbrello` | Static library | All core model, widget, codegen, dialog, command, menu logic. ~37 KLOC. |
| `umbrello5` / `umbrello6` | Executable | Thin `main.cpp` wrapper linked against `libumbrello`. |
| `codeimport` | Static library | Code import functionality (Ada, C++, C#, IDL, Java, Pascal, Python, SQL, Vala). |
| `kdevphpparser` | Static library | PHP parser (from KDevelop, built from source). |
| `kdevphpduchain` | Static library | PHP DUChain (built from source). |
| `kdevphpcompletion` | Static library | PHP completion (built from source). |
| `svg2png` | Executable | Internal tool for SVG→PNG icon rasterization. |

### 1.3 Library dependencies (link-level)

```
umbrello (executable)
  └── libumbrello (static)
       ├── Qt::Widgets, Qt::Xml, Qt::PrintSupport, Qt::Svg
       ├── KF::Archive, KF::Completion, KF::CoreAddons
       ├── KF::I18n, KF::IconThemes, KF::KIOCore
       ├── KF::TextEditor, KF::WidgetsAddons, KF::XmlGui, KF::Crash
       ├── LibXml2, LibXslt
       └── codeimport (static)
            ├── Qt::Widgets, Qt::Xml
            ├── KF::CoreAddons, KF::I18n, KF::IconThemes
            ├── KF::KIOCore, KF::TextEditor, KF::WidgetsAddons, KF::XmlGui
            └── [optional] KDev::Interfaces, KDev::Language, kdevphpparser
```

### 1.4 Key conditionals

- `BUILD_WITH_QT6` → selects Qt/KF major version, controls `APP_SUFFIX` (5/6)
- `BUILD_PHP_IMPORT` → optionally includes PHP import via KDevelop libraries
- `BUILD_TESTING` → enables unit tests subdirectory
- `BUILD_DOC` → enables DocTools KF module + kdoctools
- `BUILD_APIDOC` → Doxygen + QCH generation
- `BUILD_ICONS` / `BUILD_CURSOR_ICONS` → SVG→PNG rasterization
- `UMBRELLO_VERSION_PATCH >= 70` → enables 6 unstable features
- `LIBXML2_FOUND && LIBXSLT_FOUND` → gate for building main source

---

## 2. External Dependencies — Categorized

### 2.1 Required — Qt (5.15+ or 6.7+)

| Module | Purpose |
|--------|---------|
| `Qt::Core` | Base (signals, QObject, containers, IO, threading) |
| `Qt::Gui` | Painting, fonts, cursors, QPixmap |
| `Qt::Widgets` | Full widget toolkit (dialogs, menus, toolbars, docking) |
| `Qt::Xml` | XMI file parsing/serialization (QDomDocument) |
| `Qt::PrintSupport` | Printing diagrams |
| `Qt::Svg` | SVG icon rendering |
| `Qt::Test` | Unit test framework |

### 2.2 Required — KDE Frameworks (5.2+ or 6.1+)

| Module | Purpose |
|--------|---------|
| `KF::Archive` | Reading/writing compressed `.xmi` files |
| `KF::Completion` | Auto-completion popups in dialogs |
| `KF::Config` (`KConfig`) | Persistent settings (`umbrellosettings.kcfgc`) |
| `KF::CoreAddons` | `KAboutData`, `KStandardDirs`, `KRandom` |
| `KF::Crash` | Crash handler / bug reporter |
| `KF::I18n` (`KI18n`) | Translation (`.po` → `.mo`, `i18n()` calls) |
| `KF::IconThemes` | Themed icon loading |
| `KF::KIO` | Network file access, file dialogs, `KFileWidget` |
| `KF::TextEditor` | Syntax-highlighted code editor in dialogs |
| `KF::WidgetsAddons` | Extra widgets (`KPageDialog`, `KMessageWidget`) |
| `KF::WindowSystem` | Window system integration (taskbar, D-Bus) |
| `KF::XmlGui` | XML-based menu/toolbar layout (`umbrelloui.rc`) |

### 2.3 Required — External C libraries

| Library | Version | Purpose |
|---------|---------|---------|
| `LibXml2` | ≥2.0 | XMI XML parsing for `.xmi` files |
| `LibXslt` | ≥1.0 | XSLT transforms (XMI→DocBook, DocBook→XHTML) |

### 2.4 Optional

| Dependency | Condition | Purpose |
|------------|-----------|---------|
| KDevPlatform + KDevelop-PG-Qt | `BUILD_PHP_IMPORT=ON` | PHP import via KDevelop PHP parser |
| LLVM + Clang | Enabled in `unittests/` | C++ source code parsing tests |
| Doxygen + qhelpgenerator | `BUILD_APIDOC=ON` | API documentation generation |
| Graphviz (dot) | Optional via Doxygen | Call graphs in API docs |

---

## 3. Feature Flags and Their Impact

Six unstable features gated by patch version ≥ 70 (`UMBRELLO_VERSION_PATCH > 69`):

| Flag | Description | Rust impact |
|------|-------------|-------------|
| `ENABLE_WIDGET_SHOW_DOC` | Show documentation in class widgets | Minor — conditional UI element |
| `ENABLE_NEW_CODE_GENERATORS` | New C++ code generator | Major — alternative codegen backend |
| `ENABLE_UML_OBJECTS_WINDOW` | Objects dock window | Moderate — dockable panel |
| `ENABLE_XMIRESOLUTION` | Bug 90103 — XMI resolution fix | Minor — model resolution logic |
| `ENABLE_COMBINED_STATE_DIRECT_EDIT` | Direct editing of combined states | Minor — widget editing behavior |
| `ENABLE_OBJECT_DIAGRAM` | Object diagram (UML 1.4 variant) | Moderate — extra diagram type + XMI tag |

**Recommendation:** In the Rust rewrite, these should become Cargo features (`#[cfg(feature = "widget_show_doc")]`, etc.) rather than compile definitions. The patch-version gating can be a Cargo workspace-level constant or build.rs logic.

---

## 4. Build Modes: Qt5 vs Qt6

### 4.1 Selection mechanism

`cmake/modules/QtVersionOption.cmake` implements:

1. If `QT_MAJOR_VERSION` already defined → use it
2. If `Qt5::Core` or `Qt6::Core` targets exist → detect automatically
3. Otherwise, `BUILD_WITH_QT6` option (default OFF) determines

### 4.2 Impact

| Aspect | Qt5 mode | Qt6 mode |
|--------|----------|----------|
| Executable name | `umbrello5` | `umbrello6` |
| KF namespace | `KF5::` | `KF6::` |
| Min Qt version | 5.1.2 | 6.7 |
| Min KF version | 5.2.0 | 6.1 |
| Library suffix | `KF5` | `KF6` |
| Additional links | — | `KF6::KIOWidgets` |
| KDevelop-PG name | `KDevelop-PG-Qt` | `KDevelopPGQt` |

### 4.3 Rust migration impact

The Qt5/Qt6 duality disappears in a Rust rewrite. The choice of Rust GUI framework (see Section 7) is a single decision with no version fork. If we use `qmetaobject-rs` to bind Qt, Qt6-only is recommended. If we use a native Rust GUI (egui/iced/slint), there is no Qt dependency at all.

---

## 5. Test Infrastructure Analysis

### 5.1 Test framework

- **Framework:** Qt Test (`QObject` + `QTest` macros), **not** Google Test
- **Test data:** `test/import/` directory with input files per language
- **Base class:** `TestBase` (provides `UMLApp` instance), `TestCodeGeneratorBase` (provides temp dir)
- **Templates:** `TestUML<T,N>` / `TestWidget<T,N>` for save/load round-trip tests
- **9 XMI files:** in `test/test-*.xmi` for model loading/saving tests

### 5.2 Test registration

```cmake
# From ECMAddTests.cmake (local copy)
ecm_add_test(
    testbasictypes.cpp
    LINK_LIBRARIES ${LIBS}
    TEST_NAME testbasictypes
    ENVIRONMENT LANG=C.UTF-8 QT_LOGGING_RULES=umbrello.debug=false
)
```

Each test is a standalone executable linked against `libumbrello` + Qt test libs.

### 5.3 Test list (13 standard + 2 LLVM optional)

| Test | Source | Category |
|------|--------|----------|
| `testbasictypes` | `testbasictypes.cpp` | Model basic types |
| `testumlobject` | `testumlobject.cpp` + `testbase.cpp` | UML object model |
| `testassociation` | `testassociation.cpp` + `testbase.cpp` | Associations |
| `testclassifier` | `testclassifier.cpp` + `testbase.cpp` | Classifiers |
| `testpackage` | `testpackage.cpp` + `testbase.cpp` | Packages |
| `testcppwriter` | `testcppwriter.cpp` + `testbase.cpp` | C++ codegen |
| `testpythonwriter` | `testpythonwriter.cpp` + `testbase.cpp` | Python codegen |
| `testoptionstate` | `testoptionstate.cpp` + `testbase.cpp` | Options serialization |
| `testumlcanvasobject` | `testumlcanvasobject.cpp` + `testbase.cpp` | Canvas objects |
| `testpreconditionwidget` | `testpreconditionwidget.cpp` + `testbase.cpp` | Precondition widget |
| `testwidgetbase` | `testwidgetbase.cpp` + `testbase.cpp` | Widget base |
| `testumlroledialog` | Manual `add_executable` | Role dialog |
| `testcrashhandler` | Manual `add_executable` | Crash handler |
| `testlistpopupmenu` | Manual `add_executable` | Context menu |
| `testllvm` | (optional, LLVM+Clang) | C++ AST parsing |
| `testllvmparser` | (optional, LLVM+Clang) | C++ parser integration |

### 5.4 Execution constraints

- Requires display: `QT_QPA_PLATFORM=offscreen` or Xvfb
- Environment: `LANG=C.UTF-8 QT_LOGGING_RULES=umbrello.debug=false`
- Run via: `ctest --test-dir build -VV` or `./build/unittests/<testname>`
- Custom `check` target: `cmake --build . --target test`

---

## 6. CI/CD Pipeline Analysis

### 6.1 GitLab CI (`.gitlab-ci.yml`)

Uses KDE's shared `sysadmin/ci-utilities` templates:

```yaml
include:
  - /gitlab-templates/craft-windows-x86-64-qt6.yml
  - /gitlab-templates/flatpak.yml
  - /gitlab-templates/freebsd-qt6.yml
  - /gitlab-templates/linux-qt6.yml
  - /gitlab-templates/windows-qt6.yml
```

**Platform matrix:**

| Job | Platform | Qt |
|-----|----------|-----|
| `distro_kf5_leap` | openSUSE Leap (Docker) | Qt5/KF5 |
| `distro_kf6_leap_16` | openSUSE Leap 16 (Docker, allow_failure) | Qt6/KF6 |
| Linux Qt6 | Ubuntu (craft template) | Qt6 |
| Windows Qt6 | Windows x86_64 (craft template) | Qt6 |
| FreeBSD Qt6 | FreeBSD (craft template) | Qt6 |
| Flatpak | Flatpak build | Qt6 |

### 6.2 KDE CI (`.kde-ci.yml`)

Declares dependencies per-platform group:

- **Qt5 platforms:** `frameworks/{extra-cmake-modules,karchive,kcompletion,kconfig,kcoreaddons,kcrash,kdoctools,ki18n,kiconthemes,kio,ktexteditor,kwidgetsaddons,kwindowsystem,kxmlgui}` at `@stable`
- **Qt6 platforms:** Same deps at `@latest-kf6`
- Required passing tests: `Linux`, `Windows`

### 6.3 Local Docker CI

Pre-packaged scripts in `tools/`:

- `ci-build.sh` — autoconfigure (cmake), build, test, install in Docker
- `ci-install.sh` — install dependencies in Docker image
- `ci-run-docker-image.sh` — run prebuilt Docker image

### 6.4 Rust rewrite implications

The CI will shift from KDE infrastructure to standard Rust CI:

- **GitHub Actions** or **GitLab CI** with `actions-rs` / `cargo` workflows
- Platform matrix: Linux (x86_64, aarch64), macOS (x86_64, arm64), Windows (x86_64)
- `rustup` toolchain selection (stable vs nightly)
- `cargo build`, `cargo test`, `cargo clippy`, `cargo fmt --check`
- Binary caching with `Swatinem/rust-cache` or `sccache`
- Cross-compilation where needed

---

## 7. Dependency Replacement Recommendations

### 7.1 GUI Framework — Qt → Rust native

| Qt Module | Rust Replacement | Feasibility | Notes |
|-----------|-----------------|-------------|-------|
| Qt::Core (base) | Eliminated | ✅ | Replaced by Rust std / tokio / crossbeam |
| Qt::Gui | egui / iced / slint | ⚠️ High risk | Core rendering, painting, fonts, 2D canvas |
| Qt::Widgets | egui / iced / slint | ⚠️ High risk | Full widget hierarchy, dialogs, menus |
| Qt::Xml | quick-xml / roxmltree | ✅ | XMI parsing — fast, streaming, well-maintained |
| Qt::PrintSupport | printpdf / genpdf | ✅ | Export diagrams to PDF; system print via `print` crate |
| Qt::Svg | resvg / usvg | ✅ | SVG rendering, icon loading; `resvg` is excellent |
| Qt::Test | cargo test + rstest | ✅ | Native Rust testing, rstest for parameterized |

**Recommendation:** `egui` (immediate mode) for rapid prototyping, or `slint` (declarative) if widget-like UI desired. `iced` (ELM architecture) is also viable but has a smaller widget library. All three support the required features: canvas drawing for UML diagrams, docks, dialogs, menus, and toolbars.

For a controlled migration, `qmetaobject-rs` can call existing Qt C++ code, but this defeats the rewrite's purpose.

### 7.2 KDE Frameworks → Rust

| KF Module | Rust Replacement | Feasibility | Crate(s) |
|-----------|-----------------|-------------|----------|
| KConfig | Config crate with serde | ✅ | `serde` + `toml`/`json`/`ron`, `figment`, `config` |
| KCoreAddons (KAboutData) | Built-in struct | ✅ | Manual `clap` or custom version/name struct |
| KCrash | panic handler | ✅ | `human-panic`, `sentry`, `backtrace` |
| KI18n | Fluent / gettext | ✅ | `fluent-rs`, `gettext-rs`, `tr::tr!` macro |
| KIconThemes | Icon bundling | ✅ | `include_dir!` + `resvg` for SVG icons |
| KArchive | std compression | ✅ | `flate2`, `tar`, `zip`, `bzip2` |
| KCompletion | Custom autocomplete | ✅ | Built from scratch or use `rustyline` |
| KIO (network) | HTTP + fs | ✅ | `reqwest` / `ureq` (HTTP), `std::fs` (local), `walkdir` |
| KIO (file dialog) | Native file dialog | ✅ | `rfd` (Rust File Dialogs), `native-dialog` |
| KTextEditor | Syntax highlighting | ✅ | `syntect` (Sublime Text syntax defs), `tree-sitter` |
| KWidgetsAddons | GUI framework's extras | ✅ | Provided by egui/iced/slint widget libraries |
| KWindowSystem | window handling | ✅ | `winit` (window creation), `dpi` crate |
| KXmlGui | Eliminated | ✅ | UI layout done in Rust code or declarative DSL |

### 7.3 External C Libraries → Rust

| Library | Rust Replacement | Feasibility | Crate(s) |
|---------|-----------------|-------------|----------|
| LibXml2 | quick-xml / roxmltree | ✅ | `quick-xml` (streaming), `roxmltree` (DOM), `xmltree` |
| LibXslt | Custom transformer | ⚠️ Partial | No mature pure-Rust XSLT engine exists. Options: (1) `miniscript`-based template expansion, (2) use `saxon-rs` via JNI, (3) call `xsltproc` as subprocess, (4) port the specific XSLT transforms we need. The rewrite should evaluate which transforms are actually used (XMI→DocBook, DocBook→XHTML) and implement them as native Rust template expansions. |
| Graphviz | graphviz-rust or process | ✅ | `graphviz-rust` crate, or call `dot` CLI via `std::process::Command` |

### 7.4 PHP Import → Rust

| Current | Rust Replacement | Feasibility | Notes |
|---------|-----------------|-------------|-------|
| KDevPlatform + KDevelop-PG-Qt | `tree-sitter-php` | ✅ | `tree-sitter` with PHP grammar for PHP parsing |
| kdevphpparser | tree-sitter PHP parser | ✅ | Well-maintained PHP grammar in tree-sitter |
| kdevphpduchanin | semantic model | ⚠️ Medium | Need to build PHP symbol table from AST |
| kdevphpcompletion | Eliminated | ✅ | Not applicable outside KDevelop context |

### 7.5 LLVM/Clang tests → Rust

| Current | Rust Replacement | Feasibility | Crate(s) |
|---------|-----------------|-------------|----------|
| Clang C++ parser tests | `tree-sitter-cpp` | ✅ | C++ grammar for tree-sitter, or `clang-sys` bindings |

---

## 8. Proposed Cargo Workspace Structure

### 8.1 Workspace layout

```
umbrello-rs/
├── Cargo.toml                  (workspace root)
├── crates/
│   ├── umbrello-core/          (model, basic types, XMI serde)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-gui/           (GUI application)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-codegen/       (code generation framework + all languages)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-codegen-cpp/
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-codegen-java/
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── ... (per language)
│   ├── umbrello-codeimport/    (code import framework)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-cppparser/     (C++ parser — port from lib/cppparser/)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-phpimport/     (PHP import using tree-sitter)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-docgen/        (DocBook generator + XHTML export)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-xmi/           (XMI serialization/deserialization)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-diagram/       (diagram layout engine)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-icons/         (icon resources, embedded)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── umbrello-i18n/          (translation infrastructure)
│   │   ├── Cargo.toml
│   │   └── src/
│   └── umbrello-cli/           (CLI tool: export, import, batch)
│       ├── Cargo.toml
│       └── src/
├── tests/                      (integration tests)
│   ├── data/                   (test XMI files, import data)
│   └── ...
├── build.rs                    (workspace-level build logic if needed)
├── rust-toolchain.toml         (toolchain pinning)
├── .github/
│   └── workflows/
│       └── ci.yml
└── README.md
```

### 8.2 Dependency graph (crate-level)

```
umbrello-cli ─────┐
umbrello-gui ─────┤
                  ├── umbrello-core
                  │    ├── umbrello-xmi (quick-xml, roxmltree)
                  │    ├── umbrello-diagram
                  │    ├── umbrello-icons (include_dir!, resvg)
                  │    └── umbrello-i18n (fluent-rs)
                  │
                  ├── umbrello-codegen
                  │    ├── umbrello-codegen-cpp
                  │    ├── umbrello-codegen-java
                  │    ├── ... (per language)
                  │    └── umbrello-core
                  │
                  ├── umbrello-codeimport
                  │    ├── umbrello-cppparser (tree-sitter-cpp or custom)
                  │    ├── umbrello-phpimport (tree-sitter-php)
                  │    ├── ... (per language)
                  │    └── umbrello-core
                  │
                  └── umbrello-docgen
                       ├── umbrello-xmi
                       └── umbrello-core
```

### 8.3 Cargo workspace `Cargo.toml`

```toml
[workspace]
members = [
    "crates/umbrello-core",
    "crates/umbrello-gui",
    "crates/umbrello-cli",
    "crates/umbrello-codegen",
    "crates/umbrello-codegen-cpp",
    "crates/umbrello-codegen-java",
    # ... one per language
    "crates/umbrello-codeimport",
    "crates/umbrello-cppparser",
    "crates/umbrello-phpimport",
    "crates/umbrello-docgen",
    "crates/umbrello-xmi",
    "crates/umbrello-diagram",
    "crates/umbrello-icons",
    "crates/umbrello-i18n",
]
resolver = "2"

[workspace.package]
edition = "2021"
version = "0.1.0"
authors = ["Umbrello Contributors"]
license = "GPL-2.0-only OR GPL-3.0-only OR LicenseRef-KDE-Accepted-GPL"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
thiserror = "2"
anyhow = "1"
quick-xml = "0.36"
roxmltree = "0.20"
fluent-rs = { version = "0.7", package = "fluent" }
fluent-templates = "0.1"
resvg = "0.42"
usvg = "0.42"
syntect = { version = "5", default-features = false }
tree-sitter = "0.24"
reqwest = { version = "0.12", features = ["blocking"] }
clap = { version = "4", features = ["derive"] }
rstest = "0.23"
```

### 8.4 Build pipeline (`build.rs`)

```rust
// crates/umbrello-core/build.rs
fn main() {
    // Embed version from git
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    println!("cargo:rustc-env=UMBRELLO_VERSION={}", version);

    // Generate XMI constants, etc.
    println!("cargo:rerun-if-changed=xmi-schema.md");
}
```

```rust
// crates/umbrello-icons/build.rs
fn main() {
    // Check for icon generation if BUILD_ICONS is set
    println!("cargo:rerun-if-env-changed=UMBRELLO_BUILD_ICONS");
}
```

### 8.5 Feature flags mapping

```toml
[features]
default = []

# Unstable features (mapped from CMake add_unstable_feature)
widget_show_doc = []
new_code_generators = []
uml_objects_window = []
xmiresolution = []
combined_state_direct_edit = []
object_diagram = []

# Build options
php_import = ["tree-sitter-php"]
python_import = ["tree-sitter-python"]

# CI variants (was ci_variant in ci-build.sh)
ci-linux = []
ci-macos = []
ci-windows = []
```

---

## 9. Translation Infrastructure Replacement

### 9.1 Current: KI18n (KDE i18n)

- 62 languages in `po/LANGCODE/umbrello.po`
- `ki18n_wrap_ui()` handles `.ui` file translations
- `i18n()` calls throughout C++ code
- `po2xmi` and `xmi2pot` tools for PO↔XMI conversion
- `kdoctools_install(po)` for documentation

### 9.2 Proposed: Fluent (Project Fluent)

**Crates:** `fluent-rs`, `fluent-templates`, `unic-langid`

**Workflow:**

```
translations/
├── en/
│   ├── umbrello.ftl
│   └── main.ftl
├── de/
│   ├── umbrello.ftl
│   └── main.ftl
├── fr/
│   ├── umbrello.ftl
│   └── main.ftl
└── ... (62 languages, migrated from .po)
```

**Example FTL file:**

```ftl
# translations/en/umbrello.ftl
app-name = Umbrello UML Modeller
new-class = New Class
export-dialog-title = Export All Views
confirm-delete = Are you sure you want to delete { $name }?

# translations/de/umbrello.ftl
app-name = Umbrello UML-Modellierer
new-class = Neue Klasse
export-dialog-title = Alle Ansichten exportieren
confirm-delete = Sind Sie sicher, dass Sie { $name } löschen möchten?
```

**Rust usage:**

```rust
use fluent_templates::Loader;
use unic_langid::langid;

static LOCALES: fluent_templates::static_loader! {
    static LOCALES = {
        locales: "./translations",
        fallback_language: "en-US",
    };
};

fn main() {
    // Load translation for current locale
    let lang = langid!("de");
    let msg = LOCALES.lookup(&lang, "app-name");
    assert_eq!(msg, "Umbrello UML-Modellierer");
}
```

**Migration path:**

1. Convert all 62 `.po` files to `.ftl` using a script (existing `po2ftl` tools exist)
2. Replace all `i18n("string")` calls with `tr!("string")` or fluent lookups
3. Remove `ki18n_wrap_ui()` — UI text is now Rust-side, not in `.ui` XML files

### 9.3 Alternative: gettext-rs

The `gettext-rs` crate provides traditional GNU gettext, which maps more directly from the `.po` format. However, Fluent is preferred for:

- Better plural rules ( `{ $count } item(s)` vs complex ngettext)
- No need for `.mo` compilation
- Built-in variable interpolation
- Better tooling ecosystem (Firefox uses it)

---

## 10. Resource Management in Rust

### 10.1 Current: Qt resource system

- `icons.qrc` — 170+ entries (pixmaps, cursors, application icons)
- `ui.qrc.cmake` — generated QRC embedding `umbrelloui.rc`
- `.desktop` files — 17 auto-layout configuration files
- Headings — 16 code generation header templates
- XSLT files — `xmi2docbook.xsl`, `docbook2xhtml.xsl`
- PNG icons generated from SVG via `svg2png` tool
- Resources installed to `${KDE_INSTALL_DATADIR}/umbrello${APP_SUFFIX}`

### 10.2 Proposed: Rust resource embedding

#### 10.2.1 Icons (SVG + PNG)

Option A: **Compile-time embedding with `include_bytes!` or `include_dir!`**

```rust
// crates/umbrello-icons/src/lib.rs
use include_dir::{Dir, include_dir};
use usvg::{Tree, TreeParsing};
use resvg::render;

static ICON_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets/icons");

pub struct IconCache {
    cache: HashMap<String, egui::ImageData>,
}

impl IconCache {
    pub fn load() -> Self {
        let mut cache = HashMap::new();
        for entry in ICON_DIR.files() {
            let path = entry.path().to_string_lossy();
            if path.ends_with(".svg") {
                // Parse SVG and render to texture
                let svg_data = entry.contents();
                let tree = Tree::from_data(svg_data, &Default::default()).unwrap();
                // render at required size, store in cache
            }
        }
        Self { cache }
    }
}
```

Option B: **Procedural macro embedding**

```rust
// With a custom macro or include_bytes!
const ACTOR_ICON: &[u8] = include_bytes!("../../umbrello/pics/actor.png");
const CLASS_ICON: &[u8] = include_bytes!("../../umbrello/pics/class.png");
// ... 170+ constants
```

Option C: **`rust-embed` crate**

```rust
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../umbrello/pics"]
#[include = "*.png"]
#[include = "*.svg"]
struct IconAssets;

fn load_icon(name: &str) -> Option<egui::ImageData<'static>> {
    let asset = IconAssets::get(name)?;
    let data = asset.data;
    // decode PNG or SVG
}
```

**Recommendation:** Use `rust-embed` or `include_dir!` for icons, `resvg` crate for SVG rendering. The `svg2png` internal tool is eliminated — SVGs render at native resolution at runtime.

#### 10.2.2 Configuration resources

**Layout files** (`.desktop` layout configs) → Embed as static `&str` arrays or `serde_yaml`/`toml` configs.

```rust
// crates/umbrello-diagram/src/layouts.rs
pub static LAYOUTS: &[(&str, &str)] = &[
    ("default", include_str!("../layouts/default.toml")),
    ("three-column", include_str!("../layouts/three-column.toml")),
    // ... 17 layouts
];
```

#### 10.2.3 Code generation headers

16 template files → Embed as `include_str!` and use `mustache` / `minijinja` / `tera` for template expansion:

```rust
// crates/umbrello-codegen/src/templates.rs
use minijinja::Environment;

fn load_templates() -> Environment<'static> {
    let mut env = Environment::new();
    env.add_template_owned("cpp_header", include_str!("templates/cpp_header.h.jinja"))
       .unwrap();
    env.add_template_owned("java_class", include_str!("templates/JavaClass.java.jinja"))
       .unwrap();
    // ... 16 templates
    env
}
```

#### 10.2.4 XSLT transforms

The two XSLT files (XMI→DocBook, DocBook→XHTML) can be:

1. **Embedded and applied** → If we use a Rust XML manipulation library to perform the same transforms
2. **Embedded and called via subprocess** → Bundle `xsltproc` (less ideal)
3. **Reimplemented** → Write native Rust `Transform` structs that walk the XML tree and produce output

**Recommendation:** Reimplement as native Rust transforms in `umbrello-docgen`. The DocBook format is stable and the transform logic is well-understood. This removes the LibXslt dependency entirely.

```rust
// crates/umbrello-docgen/src/xmi2docbook.rs
use roxmltree::Document;

pub fn xmi_to_docbook(xmi: &Document) -> String {
    let mut output = String::from(r#"<?xml version="1.0"?>
<book xmlns="http://docbook.org/ns/docbook">"#);
    // recursive transform logic
    output.push_str("</book>");
    output
}
```

#### 10.2.5 Application data directory

```rust
use directories::ProjectDirs;

fn data_dir() -> Option<std::path::PathBuf> {
    ProjectDirs::from("org", "kde", "umbrello")
        .map(|d| d.data_dir().to_path_buf())
}
```

---

## Summary of Key Decisions

| Domain | Current | Rust target | Key crate(s) |
|--------|---------|-------------|--------------|
| Build system | CMake 3.16+ | Cargo | `cargo`, `build.rs` |
| GUI | Qt Widgets (C++) | egui / slint / iced | `egui`, `winit` |
| XML/XMI | LibXml2 (C) | quick-xml + roxmltree | `quick-xml`, `roxmltree` |
| XSLT | LibXslt (C) | Custom Rust transform | `roxmltree` |
| Translation | KI18n (PO files) | Fluent | `fluent-rs` |
| Settings | KConfig (kcfg) | serde + TOML | `serde`, `toml`, `figment` |
| Icons | QRC + PNG | rust-embed + resvg | `rust-embed`, `resvg` |
| Testing | Qt Test | cargo test + rstest | `rstest` |
| CI | GitLab CI (KDE) | GitHub Actions | `actions-rs` |
| PHP import | KDevelop-PG-Qt (C++) | tree-sitter-php | `tree-sitter` |
| C++ parser | Custom lib/cppparser (C++) | tree-sitter-cpp | `tree-sitter` |
| Undo/redo | QUndoCommand (Qt) | Cmd crate pattern | Custom or `undo` crate |
| Crash handler | KCrash (KDE) | human-panic | `human-panic`, `sentry` |
| Editor | KTextEditor | syntect | `syntect`, `tree-sitter` |
| Archive | KArchive | flate2 + tar | `flate2`, `tar`, `zip` |
| File dialogs | KIO / KFileWidget | rfd (Rust File Dialogs) | `rfd` |
| HTTP | KIO | reqwest | `reqwest`, `ureq` |

---

## Migration Strategy

**Phase 1 — Core library** (`umbrello-core`, `umbrello-xmi`):
- Port model types (UMLObject, UMLClassifier, UMLAssociation, etc.)
- Port XMI serialization/deserialization (replace QDomDocument with quick-xml)
- Port basic types, OptionState, unique ID generation
- Test: verify XMI round-trip matches C++ behavior

**Phase 2 — Code generation** (`umbrello-codegen`, language-specific crates):
- Port code generation framework (CodeDocument, CodeBlock, CodeComment, etc.)
- Port each language writer (C++, Java, Python, etc.) as independent crates
- Test: compare generated output against C++ reference

**Phase 3 — Code import** (`umbrello-cppparser`, `umbrello-codeimport`):
- Port C++ parser using tree-sitter-cpp or custom
- Port Java/Python/importers using tree-sitter grammars
- Port PHP import using tree-sitter-php

**Phase 4 — GUI** (`umbrello-gui`):
- Choose Rust GUI framework
- Implement main window, diagram canvas, docking windows
- Port dialog widgets and property editors
- Integrate with core library

**Phase 5 — Polish**:
- i18n migration (PO → FTL)
- Icon embedding
- CI/CD pipeline
- Packaging (deb, rpm, flatpak, Windows installer)
