# Milestone 16 — File I/O: Open, Save, Save As, and New

**Status:** Design v1  
**Target Milestone:** M16  
**Dependencies:** M15 (Rich UML Canvas), M10 (XMI Writer), M8 (XMI Reader)

---

## 1. Objective

M15 delivered a rich UML canvas that correctly renders partitioned class boxes and semantic edges. However, the application is functionally unusable for real work because:

1. **File > Open** is a stub that prints `"not yet implemented"`.
2. **File > Save** does not exist at all.
3. The only way to load a model is a hardcoded path in `main.rs`.
4. There is no dirty-flag tracking, so changes are silently lost on exit.

M16 closes this gap by implementing the complete File menu workflow: **New**, **Open**, **Save**, **Save As**, and **Quit** with unsaved-changes prompting. It also fixes the CLI to accept a file argument instead of hardcoding a path.

**Out of scope:** File > Open Recent (stubbed), Export as Image, Close Diagram, file format selection (only XMI 1.2), compression support, any domain model or rendering changes.

---

## 2. Crates to Modify

| Crate | Changes | Rationale |
|-------|---------|-----------|
| `apps/umbrello` | Primary — all menu actions, dirty tracking, rfd dialogs, CLI args | The GUI and CLI live here |
| `uml-io` | Light touch — possibly a convenience `save_xmi()` free function in `xmi/mod.rs` | Avoids duplicating the XmiWriter setup pattern across call sites |
| `uml-core` | **Zero changes** | The domain model stays pure |
| `xtask` | None | — |

**New dependency for `apps/umbrello`:**
- `rfd = "0.15"` — Native Rust file dialogs (Linux: GTK3/XDG, Windows: IFileDialog, macOS: NSSavePanel)
- Note: `rfd` 0.15 is compatible with the egui 0.31 ecosystem and supports `FileDialog::new().add_filter(...).pick_file()` and `.save_file()` synchronously. It may require the GTK3 development libraries on Linux for the native backend; if unavailable, it falls back to a non-native dialog.

---

## 3. New Types, Fields, and Functions

### 3.1 `UmbrelloApp` — New Fields

```rust
pub struct UmbrelloApp {
    // ... existing fields ...
    /// Path to the currently open file, if any. `None` for new/untitled models.
    current_file_path: Option<std::path::PathBuf>,
    /// Whether the model has unsaved changes since the last save/load.
    is_dirty: bool,
}
```

**Initialization:**
- On `new(model, loaded)`: `current_file_path = None` (even when loaded — the CLI path isn't tracked as "the file"), `is_dirty = false` (freshly loaded = clean).
- After File > Open succeeds: `current_file_path = Some(path)`, `is_dirty = false`.
- After File > Save / Save As succeeds: `current_file_path = Some(path)`, `is_dirty = false`.
- On any model mutation via `History::execute`: `is_dirty = true`.
- After File > New: `current_file_path = None`, `is_dirty = false`.

### 3.2 `UmbrelloApp` — New Methods

```rust
impl UmbrelloApp {
    /// Prompt the user to save unsaved changes. Returns `true` if the user
    /// wants to proceed with the operation, `false` if they cancelled.
    /// If the user chooses "Save", the file is saved before returning.
    fn prompt_save_if_dirty(&mut self) -> bool;

    /// Open an XMI file via native file dialog and load it into the model.
    fn menu_file_open(&mut self);

    /// Save the model; delegates to Save As if no current file path.
    fn menu_file_save(&mut self);

    /// Save the model via native save dialog to a new file path.
    fn menu_file_save_as(&mut self);

    /// Create a new empty model (after prompting if dirty).
    fn menu_file_new(&mut self);
}
```

### 3.3 `uml-io` — Convenience Function

Add to `crates/uml-io/src/xmi/mod.rs`:

```rust
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

/// Save a `UmlModel` to an XMI file at the given path.
///
/// This is a convenience wrapper around `XmiWriter::write_document`.
pub fn save_xmi_to_file(model: &UmlModel, path: &Path) -> Result<(), XmiWriteError> {
    let file = File::create(path)?;
    let mut writer = XmiWriter::new(BufWriter::new(file));
    writer.write_document(model)?;
    Ok(())
}
```

Similarly, add `load_xmi_from_file`:

```rust
/// Load an XMI file from the given path into a fresh UmlModel.
///
/// Returns the populated model on success.
pub fn load_xmi_from_file(path: &Path) -> Result<UmlModel, XmiParseError> {
    let file = File::open(path)?;
    let mut model = UmlModel::new();
    let mut reader = XmiReader::new();
    reader.read_from(BufReader::new(file), &mut model)?;
    reader.resolve(&mut model)?;
    Ok(model)
}
```

---

## 4. XMI Changes

**None.** The XMI reader and writer are already fully functional for the types they support. No changes to the XMI format, the reader, or the writer logic are needed for M16.

However, the `XmiWriter` currently has no public `write_document_to_file` convenience. We will add the `save_xmi_to_file` and `load_xmi_from_file` free functions in `crates/uml-io/src/xmi/mod.rs` (see Section 3.3 above) to avoid duplicating the setup pattern across GUI and CLI call sites.

---

## 5. UI Changes

### 5.1 File Menu — Complete Repertoire

```
File
├── New                    Ctrl+N
├── Open XMI...            Ctrl+O
├── Save                   Ctrl+S
├── Save As...             Ctrl+Shift+S
├── ─────────────────
├── Open Recent            (disabled/stubbed)
├── ─────────────────
├── Quit                   Ctrl+Q
```

All items except Open Recent must be functional.

### 5.2 Unsaved Changes Prompt

When the user attempts New, Open, or Quit with `is_dirty == true`, show a modal-style prompt via `rfd::MessageDialog`:

```
╔══════════════════════════════════╗
║  Unsaved Changes                ║
║                                  ║
║  The model has unsaved changes.  ║
║  Do you want to save before      ║
║  continuing?                     ║
║                                  ║
║  [Save]   [Discard]   [Cancel]   ║
╚══════════════════════════════════╝
```

- **Save**: Perform Save (or Save As if no path), then continue.
- **Discard**: Continue without saving.
- **Cancel**: Abort the triggering operation.

Implementation note: `rfd`'s `MessageDialog` is synchronous and blocks. In the egui immediate-mode context, this is acceptable because the blocking call only happens in response to a button click and returns before the next frame.

### 5.3 Error Messages

For file I/O and XMI parse errors, show a native message dialog:

```
╔══════════════════════════════════╗
║  Error Opening File             ║
║                                  ║
║  Could not open 'bad/file.xmi':  ║
║  No such file or directory       ║
║                                  ║
║                         [OK]     ║
╚══════════════════════════════════╝
```

For less severe errors (e.g., partial load success), update `status_message` in the status bar instead.

### 5.4 Window Title

Update the window title to reflect the current file:

- No file: `"Umbrello-RS — Untitled"`
- With file: `"Umbrello-RS — /path/to/model.xmi"`
- Dirty indicator: append `*` if dirty: `"Umbrello-RS — /path/to/model.xmi *"`

The title is updated via `ctx.send_viewport_cmd(ViewportCommand::Title(...))`.

### 5.5 Status Bar Feedback

After each file operation, update `status_message`:
- Open success: `"Loaded: /path/to/model.xmi (42 elements, 3 diagrams)"`
- Save success: `"Saved: /path/to/model.xmi"`
- New: `"New model created"`
- Open error: `"Error opening /path: <details>"`

---

## 6. CLI Changes

### 6.1 `main.rs` — Current State

```rust
fn main() -> anyhow::Result<()> {
    let mut model = UmlModel::new();
    let loaded = load_xmi("tests/data/xmi/test-COG.xmi", &mut model)
        || load_xmi("../tests/data/xmi/test-COG.xmi", &mut model);
    // ... launch eframe with model ...
}
```

### 6.2 `main.rs` — New State

Accept a positional file argument via `clap` (already a workspace dependency):

```rust
use clap::Parser;

#[derive(Parser)]
#[command(name = "umbrello", about = "UML modeling tool")]
struct Cli {
    /// Path to an XMI file to open on startup.
    file: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut model = UmlModel::new();
    let mut loaded = false;

    if let Some(path) = &cli.file {
        if let Ok(file) = std::fs::File::open(path) {
            let mut reader = XmiReader::new();
            if reader.read_from(BufReader::new(file), &mut model).is_ok() {
                let _ = reader.resolve(&mut model);
                loaded = true;
            }
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title(if loaded {
                format!("Umbrello-RS — {}", cli.file.as_ref().unwrap())
            } else {
                "Umbrello-RS — Untitled".into()
            }),
        ..Default::default()
    };

    // Pass the file path to UmbrelloApp so it can track current_file_path
    let current_file_path = if loaded {
        cli.file.map(std::path::PathBuf::from)
    } else {
        None
    };

    eframe::run_native(
        &options.title.clone(),
        options,
        Box::new(move |_cc| {
            let mut app = UmbrelloApp::new(model, loaded);
            app.set_current_file_path(current_file_path);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
```

Key changes:
- Remove the hardcoded `tests/data/xmi/test-COG.xmi` paths.
- Accept optional positional `file` argument.
- Pass `current_file_path` through to `UmbrelloApp`.

### 6.3 `UmbrelloApp` — New Setter

```rust
pub fn set_current_file_path(&mut self, path: Option<std::path::PathBuf>) {
    self.current_file_path = path;
}
```

---

## 7. Dirty-Flag Integration

The `is_dirty` flag must be set to `true` on every user-initiated model mutation. There are two integration points:

### 7.1 After `History::execute()` calls

Every call to `self.history.execute(...)` in `app.rs` must be followed by `self.is_dirty = true;` when the command succeeds. Current locations:
- `render_canvas()` — `MoveNode` command on drag (line ~216)
- Any future mutation commands

Wrap in a helper:

```rust
fn execute_command(&mut self, cmd: Box<dyn Command>) {
    if self.history.execute(cmd, &mut self.model).is_ok() {
        self.is_dirty = true;
    }
}
```

### 7.2 After File > New

Set `is_dirty = false` after creating a fresh model.

### 7.3 After File > Open or Save

Set `is_dirty = false` after a successful load or save.

---

## 8. Test Plan

### 8.1 Unit Tests (in `app.rs` `#[cfg(test)]`)

| # | Test Name | What It Verifies |
|---|-----------|-----------------|
| T1 | `file_new_clears_model` | After File > New, model is empty, is_dirty = false, file_path = None |
| T2 | `dirty_flag_on_mutation` | After calling `execute_command`, is_dirty = true |
| T3 | `dirty_flag_cleared_on_save` | After save, is_dirty = false |
| T4 | `dirty_flag_cleared_on_open` | After open, is_dirty = false |
| T5 | `file_path_tracking` | current_file_path is set correctly after open/save |
| T6 | `save_then_reload_roundtrip` | Create model → save to temp file → load back → structural equality |
| T7 | `save_as_updates_path` | Save As to new path updates current_file_path |

### 8.2 Integration Tests (in `tests/` or `apps/umbrello/tests/`)

| # | Test Name | What It Verifies |
|---|-----------|-----------------|
| T8 | `xmi_save_load_roundtrip` | Full cycle: build model in memory → save via `save_xmi_to_file` → load via `load_xmi_from_file` → verify element count, names, relationships match |
| T9 | `xmi_save_load_roundtrip_with_diagrams` | Same as T8 but includes diagrams with nodes and edges |

### 8.3 Tests in `uml-io`

| # | Test Name | What It Verifies |
|---|-----------|-----------------|
| T10 | `save_xmi_to_file_roundtrip` | The new `save_xmi_to_file` / `load_xmi_from_file` convenience functions work correctly |
| T11 | `save_xmi_to_file_error_on_bad_path` | Error is returned for unwritable path |

### 8.4 Manual Verification

- Open the app without arguments → shows "Untitled", empty model
- File > Open → select a valid XMI file → loads and renders diagrams
- Drag a node → `*` appears in title bar
- File > Save → `*` disappears
- Modify model → File > New → prompted to save → Save → new empty model
- File > Open → select invalid file → error dialog appears
- CLI: `cargo run -p umbrello -- ../test/test-COG.xmi` → loads that file
- CLI: `cargo run -p umbrello` → starts empty

---

## 9. Implementation Sub-Task Order

| Order | Sub-task | File(s) | Description |
|-------|----------|---------|-------------|
| **A** | Add `rfd` dependency | `apps/umbrello/Cargo.toml` | Add `rfd = "0.15"` |
| **B** | Convenience functions in `uml-io` | `crates/uml-io/src/xmi/mod.rs` | Add `save_xmi_to_file()` and `load_xmi_from_file()` |
| **C** | New fields + setters on `UmbrelloApp` | `apps/umbrello/src/app.rs` | Add `current_file_path`, `is_dirty`; add `set_current_file_path` |
| **D** | File > New | `apps/umbrello/src/app.rs` | `menu_file_new()` with dirty prompt |
| **E** | File > Open | `apps/umbrello/src/app.rs` | `menu_file_open()` with rfd dialog, dirty prompt, XMI loading |
| **F** | File > Save / Save As | `apps/umbrello/src/app.rs` | `menu_file_save()`, `menu_file_save_as()` with rfd dialog, XMI writing |
| **G** | Dirty prompt + Quit integration | `apps/umbrello/src/app.rs` | `prompt_save_if_dirty()`; wire into New/Open/Quit |
| **H** | Window title updates | `apps/umbrello/src/app.rs` | `update_title()` helper; call after file operations |
| **I** | CLI changes | `apps/umbrello/src/main.rs` | Remove hardcoded path; accept `clap` positional arg |
| **J** | `execute_command` helper | `apps/umbrello/src/app.rs` | Wrap `history.execute` with `is_dirty = true` |
| **K** | Tests (uml-io) | `crates/uml-io/src/xmi/mod.rs` | T10, T11 |
| **L** | Tests (app) | `apps/umbrello/src/app.rs` | T1–T7 |

---

## 10. Architecture Compliance

- **`uml-core` is untouched** — no domain model or diagram changes.
- **All mutations via Commands** — the dirty flag is set after `History::execute()`, not during direct mutations.
- **XMI compatibility preserved** — the reader/writer API is used as-is; no XMI schema changes.
- **Traits over inheritance** — the dirty-flag helper `execute_command` is a simple method, not a trait; it wraps the existing `History` API.
- **`cargo test --workspace` must pass** — all 206 existing tests remain green; new tests add to the count.

---

## 11. Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| `rfd` fails to compile on CI (missing GTK dev headers) | Medium | Use `rfd`'s `xdg-portal` backend with `default-features = false, features = ["xdg-portal"]` on Linux, or fall back to a pure-Rust dialog via `rfd`'s `sync` feature |
| `rfd::MessageDialog` doesn't have native "Discard" button | Low | Use `YesNoCancel` buttons: Yes=Save, No=Discard, Cancel=Cancel |
| egui event loop conflict with blocking `rfd` calls | Low | `rfd` spawns its own nested event loop; in extensive testing by the egui community this pattern is well-established |
| File save fails silently (permissions, disk full) | Low | Show error dialog on write failure; do not clear dirty flag on failure |

---

*Design document v1 — ready for implementation.*
