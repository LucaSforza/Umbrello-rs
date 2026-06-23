# Command (Undo/Redo) Architecture v1

> **Document:** `rust-rewrite/docs/command_architecture_v1.md`
> **Status:** Active
> **Phase:** Milestone 11 (Undo/Redo Engine)
> **Last updated:** 2026-06-23
>
> This document defines the architecture for the undo/redo command system in
> Umbrello-RS. It covers the design trade-off between dynamic dispatch and
> enum-based commands, the `Command` trait, the `History` manager, four initial
> command implementations with full code sketches, and an integration test plan.
>
> **Scope:** Model-only commands operating on `UmlModel`. GUI-level undo
> (selection changes, viewport scrolling, widget repositioning) is deferred
> to a later milestone. All code lives in `crates/uml-core/src/undo/`.

---

## Table of Contents

1. [Context](#1-context)
2. [Design Decision: `Box<dyn Command>` vs `enum Command`](#2-design-decision-boxdyn-command-vs-enum-command)
   - [2.1 Option A: Dynamic Dispatch (Box\<dyn Command\>)](#21-option-a-dynamic-dispatch-boxdyn-command)
   - [2.2 Option B: Enum Command](#22-option-b-enum-command)
   - [2.3 Recommendation: Option A](#23-recommendation-option-a)
3. [The Command Trait](#3-the-command-trait)
   - [3.1 Trait Definition](#31-trait-definition)
   - [3.2 CommandError](#32-commanderror)
4. [The History Manager](#4-the-history-manager)
   - [4.1 Struct Definition](#41-struct-definition)
   - [4.2 Public Methods](#42-public-methods)
   - [4.3 Internal Logic](#43-internal-logic)
5. [Initial Commands (Milestone 11)](#5-initial-commands-milestone-11)
   - [5.1 CreateElementCommand](#51-createelementcommand)
   - [5.2 DeleteElementCommand](#52-deleteelementcommand)
   - [5.3 RenameElementCommand](#53-renameelementcommand)
   - [5.4 MoveElementCommand (bonus)](#54-moveelementcommand-bonus)
6. [Integration Test Plan](#6-integration-test-plan)
   - [6.1 Test Scenarios](#61-test-scenarios)
   - [6.2 Test Fixtures](#62-test-fixtures)
7. [Location and Module Structure](#7-location-and-module-structure)
8. [Future Extensions](#8-future-extensions)

---

## 1. Context

The `UmlModel` in `uml-core` is mutated via methods like:

- `insert(element) → Option<ModelElement>`
- `remove(id) → Option<ModelElement>`
- `add_to_package(pkg, child) → Result`
- `remove_from_package(pkg, child) → Result`
- `ModelElement::set_name(name)`, `set_visibility(vis)`, `base_mut()` for field mutation

All mutations must be wrapped in reversible commands for undo/redo. The current
`crates/uml-core/src/undo/mod.rs` is a 35-line stub containing only an empty
`UndoStack` struct with `can_undo() → false` / `can_redo() → false` stubs.
This document describes the architecture that replaces that stub.

### Architectural Constraints

1. **Model-only:** Commands operate on `&mut UmlModel` with no GUI dependencies.
   GUI-level undo (widget positions, selection, scroll) is separate.
2. **Stack-based undo:** Bounded undo/redo stacks with configurable max depth.
3. **Deterministic redo:** New actions clear the redo stack (standard UX).
4. **Descriptions:** Every command provides a human-readable string for menu/tooltip
   display (e.g., "Create class Person", "Rename to Employee").

---

## 2. Design Decision: `Box<dyn Command>` vs `enum Command`

### 2.1 Option A: Dynamic Dispatch (`Box<dyn Command>`)

```rust
pub trait Command: std::fmt::Debug + Send {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn description(&self) -> &str;
}

pub struct History {
    undo_stack: Vec<Box<dyn Command>>,
    redo_stack: Vec<Box<dyn Command>>,
    max_depth: usize,
}
```

**Pros:**

- **Open-closed principle:** Adding a command = new struct + impl `Command`.
  No changes to `History` or any existing types.
- **Small stack entries:** One fat pointer per command (`Box<dyn Command>` = 16 bytes
  on 64-bit).
- **Easy to test independently:** Each command struct is a plain type with no
  match-arm entanglement.
- **Clear separation:** Each command owns its undo data (e.g., the `Option<ModelElement>`
  for create/delete). Variant-specific data lives only in that variant's struct.

**Cons:**

- Dynamic dispatch overhead (a vtable lookup per `execute`/`undo` call).
  Negligible at human interaction speed (~10 commands/second max).
- Cannot derive `Serialize` on trait objects directly. If serialization of undo
  history is required later, we add a `serialize_state() → serde_json::Value`
  method to the trait.
- Each command allocates on the heap (`Box`). Allocation cost is negligible
  at command frequency.

### 2.2 Option B: Enum Command

```rust
pub enum UmlCommand {
    CreateElement { element: Option<ModelElement>, element_id: UmlId, description: String },
    DeleteElement { element: Option<ModelElement>, element_id: UmlId, description: String },
    RenameElement { id: UmlId, old_name: String, new_name: String, description: String },
    // ... 20+ more variants
}

impl UmlCommand {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> { ... }
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> { ... }
}
```

**Pros:**

- No heap allocation per command (enum is stack-allocated).
- Can derive `Serialize` on the enum (potentially useful for saving undo history
  to disk — e.g., auto-save crash recovery).
- No dynamic dispatch; all dispatch is a single match in `execute`/`undo`.

**Cons:**

- **Closed set:** Adding a command requires modifying the enum definition + all
  match arms (execute, undo, description, merge). Violates open-closed principle.
- **Large enum:** With 20+ variants, every match has O(n) branch cost.
- **Wasted memory:** Every variant carries all possible undo data fields even
  when unused for that variant. The enum size is the max of all variant sizes.
- **Plugin-hostile:** Future codegen/import commands (potentially from external
  crates) cannot add variants to a closed enum.

### 2.3 Recommendation: Option A (`Box<dyn Command>`)

For a UML modeling tool, commands execute at human speed (max ~10/second).
Dynamic dispatch overhead is unmeasurable in practice. The flexibility of adding
commands without modifying `History` or the trait is worth the minor cost.

If serialization of undo history is needed later (e.g., crash recovery), we can
add an optional `serialize_state() → serde_json::Value` method to the `Command`
trait with a default no-op implementation.

---

## 3. The Command Trait

### 3.1 Trait Definition

```rust
/// A reversible operation on a `UmlModel`.
///
/// Every command must record whatever state is necessary to reverse its
/// effect in `undo()`. The `merge()` method allows consecutive compatible
/// commands (e.g., text edits to the same field) to be collapsed into one.
pub trait Command: std::fmt::Debug + Send {
    /// Execute the command. Called once when the user performs the action.
    ///
    /// # Errors
    ///
    /// Returns `CommandError` if the operation cannot be completed (e.g.,
    /// element already removed, invalid name). The model is left unchanged
    /// on error — callers should not push failed commands to the stack.
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;

    /// Undo the command. Reverse the effect of `execute()`.
    ///
    /// The command must restore the model to the state before `execute()`
    /// was called, with all IDs preserved.
    ///
    /// # Errors
    ///
    /// Returns `CommandError` if the undo cannot be completed (should only
    /// happen if external code directly mutates the model between calls).
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;

    /// Human-readable description for UI display.
    ///
    /// Examples:
    /// - `"Create class Person"`
    /// - `"Rename 'OldName' → 'NewName'"`
    /// - `"Delete attribute 'count'"`
    fn description(&self) -> &str;

    /// Optionally merge this command with a subsequent command of the same
    /// logical operation.
    ///
    /// Returns `Some(merged)` if the two commands can be combined into one;
    /// returns `None` (the default) to keep them separate.
    ///
    /// # Typical Use
    ///
    /// Consecutive renames of the same element can be merged into a single
    /// command with the final name, so that undo jumps directly to the
    /// original name rather than stepping through each intermediate value.
    fn merge(&self, _other: &dyn Command) -> Option<Box<dyn Command>> {
        None
    }
}
```

### 3.2 `CommandError`

```rust
use crate::repository::ModelError;
use crate::id::UmlId;

/// Errors that can occur during command execution or undo.
#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    /// The element with the given ID was not found in the model.
    #[error("element not found: {0}")]
    ElementNotFound(UmlId),

    /// The operation is invalid for the current state.
    ///
    /// Examples:
    /// - Attempting to execute a `CreateElementCommand` twice
    /// - Attempting to undo a `DeleteElementCommand` when the element already
    ///   exists again (conflict)
    #[error("invalid operation: {0}")]
    InvalidOperation(String),

    /// An error from the underlying model repository.
    #[error("model error: {0}")]
    Model(#[from] ModelError),
}
```

`CommandError` implements `From<ModelError>` so that command implementations
can use the `?` operator on model operations that return `Result<_, ModelError>`.

---

## 4. The History Manager

### 4.1 Struct Definition

```rust
/// Manages undo/redo history for `UmlModel` mutations.
///
/// All model mutations intended to be undoable should go through
/// `History::execute()` to ensure they are tracked. Direct mutation of
/// `UmlModel` (calling `insert()`/`remove()` directly without a Command)
/// bypasses the history and cannot be undone.
///
/// # Disabled Mode
///
/// During file loading, undo is meaningless — we do not want to build up
/// thousands of "create element" commands. Setting `disabled = true` causes
/// `execute()` to run the command without pushing it onto the stack.
///
/// # Stack Semantics
///
/// - `undo()` pops from `undo_stack`, reverses the command, and pushes it
///   onto `redo_stack`.
/// - `redo()` pops from `redo_stack`, re-executes the command, and pushes it
///   onto `undo_stack`.
/// - Any new `execute()` call clears the `redo_stack` (new action invalidates
///   redo history).
/// - When `undo_stack` exceeds `max_depth`, the oldest command is dropped.
#[derive(Debug)]
pub struct History {
    /// Stack of past commands (most recent at the end).
    undo_stack: Vec<Box<dyn Command>>,
    /// Stack of undone commands available for redo (most recent at the end).
    redo_stack: Vec<Box<dyn Command>>,
    /// Maximum number of commands on the undo stack.
    max_depth: usize,
    /// If true, commands are executed but not pushed onto the stack.
    /// Used during file loading where undo is meaningless.
    disabled: bool,
}
```

### 4.2 Public Methods

```rust
impl History {
    /// Create a new history with the given maximum undo depth.
    ///
    /// A `max_depth` of 0 disables undo entirely (commands still execute
    /// but are not recorded). Typical values are 100–500.
    pub fn new(max_depth: usize) -> Self;

    /// Execute a command and push it onto the undo stack.
    ///
    /// The command is executed immediately via `cmd.execute(model)`. If
    /// execution succeeds and history is not disabled, the command is pushed
    /// onto `undo_stack` and `redo_stack` is cleared.
    ///
    /// If execution fails, the model is left unchanged and the command is
    /// NOT pushed onto any stack.
    ///
    /// Before pushing, attempts to merge with the most recent command via
    /// `merge()`. If merging succeeds, the merged command replaces the tip.
    ///
    /// # Errors
    ///
    /// Propagates errors from `cmd.execute()`.
    pub fn execute(
        &mut self,
        cmd: Box<dyn Command>,
        model: &mut UmlModel,
    ) -> Result<(), CommandError>;

    /// Undo the most recent command.
    ///
    /// Pops the tip of `undo_stack`, calls `cmd.undo(model)`, and pushes it
    /// onto `redo_stack`.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::InvalidOperation("nothing to undo")` if the
    /// undo stack is empty. Propagates errors from `cmd.undo()` (the
    /// undone command is still moved to `redo_stack` so it can be re-attempted).
    pub fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;

    /// Redo the most recently undone command.
    ///
    /// Pops the tip of `redo_stack`, calls `cmd.execute(model)`, and pushes it
    /// onto `undo_stack`.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::InvalidOperation("nothing to redo")` if the
    /// redo stack is empty. Propagates errors from `cmd.execute()`.
    pub fn redo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;

    /// Returns `true` if there are commands to undo.
    pub fn can_undo(&self) -> bool;

    /// Returns `true` if there are commands to redo.
    pub fn can_redo(&self) -> bool;

    /// Returns the description of the command at the top of the undo stack,
    /// or `None` if the stack is empty.
    pub fn undo_description(&self) -> Option<&str>;

    /// Returns the description of the command at the top of the redo stack,
    /// or `None` if the stack is empty.
    pub fn redo_description(&self) -> Option<&str>;

    /// Enable or disable history recording.
    ///
    /// When disabled, `execute()` runs commands without pushing them onto
    /// any stack. `undo()` and `redo()` still operate on existing stack
    /// contents.
    pub fn set_disabled(&mut self, disabled: bool);

    /// Clear both undo and redo stacks.
    ///
    /// All commands are dropped and their undo data is freed.
    pub fn clear(&mut self);

    /// Current depth of the undo stack.
    pub fn undo_depth(&self) -> usize;

    /// Current depth of the redo stack.
    pub fn redo_depth(&self) -> usize;
}
```

### 4.3 Internal Logic

#### `execute()` algorithm

```
fn execute(cmd, model):
    cmd.execute(model)?                     // run the command
    if disabled: return Ok(())              // skip recording
    redo_stack.clear()                      // new action invalidates redo
    if let Some(tip) = undo_stack.last():
        if let Some(merged) = tip.merge(&*cmd):
            undo_stack.pop()                // replace with merged
            undo_stack.push(merged)
            return Ok(())
    undo_stack.push(cmd)                    // push as-is
    if undo_stack.len() > max_depth:
        undo_stack.remove(0)                // drop oldest
    Ok(())
```

#### `undo()` algorithm

```
fn undo(model):
    let mut cmd = undo_stack.pop()?         // take ownership
    cmd.undo(model)?                        // reverse
    redo_stack.push(cmd)                    // save for redo
    Ok(())
```

#### `redo()` algorithm

```
fn redo(model):
    let mut cmd = redo_stack.pop()?         // take ownership
    cmd.execute(model)?                     // re-apply
    undo_stack.push(cmd)                    // back to undo stack
    Ok(())
```

---

## 5. Initial Commands (Milestone 11)

### 5.1 `CreateElementCommand`

Creates an element in the model. On undo, removes it. Stores the `ModelElement`
in an `Option` that is `Some` before execution and `Some` after undo (the
command acts as a "slot" that moves the element in and out of the model).

```rust
use crate::elements::ModelElement;
use crate::id::UmlId;
use crate::repository::UmlModel;

/// Command that creates a new UML model element.
///
/// On `execute()`, the element is moved out of `self.element` and inserted
/// into the model. On `undo()`, the element is removed from the model and
/// placed back into `self.element`.
///
/// The element's `UmlId` is preserved across the full create/undo cycle.
#[derive(Debug)]
pub struct CreateElementCommand {
    /// Holds the element before execute; empty (None) after execute.
    /// Set back to Some after undo, ready for re-execute.
    element: Option<ModelElement>,
    /// The ID of the element (stable across execute/undo cycles).
    element_id: UmlId,
    /// Human-readable description (e.g., "Create class Person").
    description: String,
}

impl CreateElementCommand {
    /// Create a new command that will insert `element` into the model.
    ///
    /// The description is generated automatically from the element's type
    /// and name (e.g., "Create class Person").
    pub fn new(element: ModelElement) -> Self {
        let id = element.id();
        let desc = format!(
            "Create {} '{}'",
            element.object_type().as_str(),
            element.name()
        );
        Self {
            element: Some(element),
            element_id: id,
            description: desc,
        }
    }
}

impl Command for CreateElementCommand {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = self
            .element
            .take()
            .ok_or_else(|| CommandError::InvalidOperation(
                "CreateElementCommand already executed".into()
            ))?;
        // Verify the ID matches what we stored
        debug_assert_eq!(elem.id(), self.element_id);
        model.insert(elem);
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model.remove(self.element_id).ok_or(
            CommandError::ElementNotFound(self.element_id),
        )?;
        self.element = Some(elem);
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

**Key design points:**

- `element: Option<ModelElement>` acts as a transfer slot. The element is moved
  out on execute, moved back in on undo.
- `element_id` is stored separately (not extracted from `element`) so that
  `description()` can function even when `element` is `None` (after execute).
- The `debug_assert_eq!` catches programmer errors in debug builds.
- The command is re-usable: after undo, it can be executed again (for redo via
  the redo stack).

### 5.2 `DeleteElementCommand`

Removes an element from the model. On undo, re-inserts it with the **same**
`UmlId`. The deleted element is stored in `self.element`.

```rust
/// Command that deletes a UML model element.
///
/// On `execute()`, the element is removed from the model and stored in
/// `self.element`. On `undo()`, the element is re-inserted with its
/// original `UmlId`.
///
/// # Construction
///
/// `DeleteElementCommand::new()` takes a model reference and looks up the
/// element by ID to generate the description string. It does NOT capture the
/// element data at construction time — capture happens on execute.
#[derive(Debug)]
pub struct DeleteElementCommand {
    /// Holds the deleted element after execute; None before execute.
    /// Set back to None after undo, ready for re-execute.
    element: Option<ModelElement>,
    /// The ID of the element to delete.
    element_id: UmlId,
    /// Human-readable description (e.g., "Delete class Person").
    description: String,
}

impl DeleteElementCommand {
    /// Create a new command that will delete the element with the given ID.
    ///
    /// Looks up the element in the model to generate a description.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the ID does not exist
    /// in the model at construction time.
    pub fn new(model: &UmlModel, id: UmlId) -> Result<Self, CommandError> {
        let elem = model
            .get(id)
            .ok_or(CommandError::ElementNotFound(id))?;
        let desc = format!(
            "Delete {} '{}'",
            elem.object_type().as_str(),
            elem.name()
        );
        Ok(Self {
            element: None,
            element_id: id,
            description: desc,
        })
    }
}

impl Command for DeleteElementCommand {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .remove(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        self.element = Some(elem);
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = self
            .element
            .take()
            .ok_or_else(|| CommandError::InvalidOperation(
                "DeleteElementCommand already undone".into()
            ))?;
        debug_assert_eq!(elem.id(), self.element_id);
        model.insert(elem);
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

**Key design points:**

- Construction requires a `&UmlModel` borrow. This is acceptable because
  construction happens at the UI layer which always has access to the model.
- The description is computed at construction time (before the element is
  deleted), so it can use the element's name.
- After undo, `self.element` is `None` again, making the command ready for
  redo (via the redo stack).

### 5.3 `RenameElementCommand`

Changes an element's name. Stores both old and new names. On undo, restores
the old name.

```rust
/// Command that renames a UML model element.
///
/// Stores both old and new names so that undo can restore the original.
/// Consecutive rename commands on the same element can be merged via
/// `merge()`.
#[derive(Debug)]
pub struct RenameElementCommand {
    /// The ID of the element being renamed.
    element_id: UmlId,
    /// The name before the rename (for undo).
    old_name: String,
    /// The name after the rename.
    new_name: String,
    /// Human-readable description (e.g., "Rename 'Old' → 'New'").
    description: String,
}

impl RenameElementCommand {
    /// Create a new command that will rename the element with the given ID.
    ///
    /// Reads the current name from the model to populate `old_name`.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the ID does not exist
    /// in the model at construction time.
    pub fn new(
        model: &UmlModel,
        id: UmlId,
        new_name: String,
    ) -> Result<Self, CommandError> {
        let elem = model
            .get(id)
            .ok_or(CommandError::ElementNotFound(id))?;
        let old_name = elem.name().to_string();
        let desc = format!("Rename '{}' → '{}'", old_name, new_name);
        Ok(Self {
            element_id: id,
            old_name,
            new_name,
            description: desc,
        })
    }
}

impl Command for RenameElementCommand {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        // Update description in case of re-execute after undo
        self.old_name = elem.name().to_string();
        elem.set_name(self.new_name.clone());
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        let elem = model
            .get_mut(self.element_id)
            .ok_or(CommandError::ElementNotFound(self.element_id))?;
        std::mem::swap(&mut self.old_name, &mut self.new_name);
        elem.set_name(self.new_name.clone());
        // Now: old_name == the current name, new_name == the previous name
        // Swap back so state is consistent for redo
        std::mem::swap(&mut self.old_name, &mut self.new_name);
        // After swap: old_name restored, new_name restored
        // Actually, let's be explicit:
        //   old_name holds the name before this undo call
        //   new_name holds the name after this undo call
        // The swap pattern: we want to set current to old_name,
        // then swap old_name and new_name so redo sees the right values.
        //
        // Simpler approach:
        elem.set_name(self.old_name.clone());
        std::mem::swap(&mut self.old_name, &mut self.new_name);
        // Now self.old_name == the name we just set (for next undo),
        // self.new_name == the name before this call (for redo).
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }

    /// Merge two consecutive renames of the same element.
    ///
    /// If both commands rename the same element, produce a single command
    /// that goes from the original name to the final name.
    fn merge(&self, other: &dyn Command) -> Option<Box<dyn Command>> {
        if let Some(other_rename) = other.downcast_ref::<RenameElementCommand>() {
            if self.element_id == other_rename.element_id {
                // Produce a merged command: from self.old_name to other_rename.new_name
                let desc = format!("Rename '{}' → '{}'", self.old_name, other_rename.new_name);
                return Some(Box::new(RenameElementCommand {
                    element_id: self.element_id,
                    old_name: self.old_name.clone(),
                    new_name: other_rename.new_name.clone(),
                    description: desc,
                }));
            }
        }
        None
    }
}
```

**Merge semantics:**

```
User types:   "A" → "AB" → "ABC" → "ABCD"
Commands:     Rename("A"→"AB") → Rename("AB"→"ABC") → Rename("ABC"→"ABCD")
Merged:       Rename("A"→"ABCD")     [single undo restores "A"]
```

The `merge()` method uses `downcast_ref` which requires the `Command` trait to
have a `fn type_id(&self) -> std::any::TypeId` method. This is provided by
adding a supertrait bound `Any` or using the `downcast-rs` crate. See the
trait definition note in §3.1 — we add `Any` as a supertrait for this purpose.

**Alternative (simpler) merge approach:**

If `downcast_ref` is undesirable, we can skip merge entirely for v1 and add it
later. The `merge()` default returns `None`, so the system works without it.
For v1, we can leave `RenameElementCommand::merge()` unimplemented (returning
`None`) and add merge support in a follow-up.

### 5.4 `MoveElementCommand` (bonus)

Moves an element between packages. Stores source and destination package IDs
so the move can be reversed.

```rust
/// Command that moves an element from one package to another.
///
/// On `execute()`, the element is removed from `from_package` and added
/// to `to_package`. On `undo()`, the reverse happens.
///
/// If `from_package` is `None`, the element is not currently in any package
/// (it is a root-level element being assigned to a package for the first time).
/// If `to_package` is `None`, the element is removed from its current package
/// without reassignment (becomes root-level).
#[derive(Debug)]
pub struct MoveElementCommand {
    /// The ID of the element being moved.
    element_id: UmlId,
    /// The package the element currently belongs to (None = root-level).
    from_package: Option<UmlId>,
    /// The package the element will be moved to (None = remove from package).
    to_package: Option<UmlId>,
    /// Human-readable description.
    description: String,
}

impl MoveElementCommand {
    /// Create a new command that moves `element_id` to `to_package`.
    ///
    /// # Errors
    ///
    /// Returns `CommandError::ElementNotFound` if the element does not exist.
    pub fn new(
        model: &UmlModel,
        element_id: UmlId,
        to_package: Option<UmlId>,
    ) -> Result<Self, CommandError> {
        let elem = model
            .get(element_id)
            .ok_or(CommandError::ElementNotFound(element_id))?;
        let elem_name = elem.name().to_string();
        let from_package = model
            .parents_of(element_id)
            .and_then(|parents| parents.first().copied());

        let desc = match (from_package, to_package) {
            (Some(src), Some(dst)) => {
                let src_name = model
                    .get(src)
                    .map_or_else(|| "?".to_string(), |p| p.name().to_string());
                let dst_name = model
                    .get(dst)
                    .map_or_else(|| "?".to_string(), |p| p.name().to_string());
                format!("Move '{}' from {} to {}", elem_name, src_name, dst_name)
            }
            (None, Some(dst)) => {
                let dst_name = model
                    .get(dst)
                    .map_or_else(|| "?".to_string(), |p| p.name().to_string());
                format!("Move '{}' to {}", elem_name, dst_name)
            }
            (Some(src), None) => {
                let src_name = model
                    .get(src)
                    .map_or_else(|| "?".to_string(), |p| p.name().to_string());
                format!("Remove '{}' from {}", elem_name, src_name)
            }
            (None, None) => {
                return Err(CommandError::InvalidOperation(
                    "cannot move from nowhere to nowhere".into(),
                ));
            }
        };

        Ok(Self {
            element_id,
            from_package,
            to_package,
            description: desc,
        })
    }
}

impl Command for MoveElementCommand {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // Remove from current package (if any)
        if let Some(src) = self.from_package {
            // It's okay if the element isn't actually in src anymore
            // (user may have moved it manually between construction and execute).
            // We silently ignore NotAChild errors.
            let _ = model.remove_from_package(src, self.element_id);
        }
        // Add to target package (if any)
        if let Some(dst) = self.to_package {
            if model.contains(self.element_id) {
                model.add_to_package(dst, self.element_id)?;
            }
        }
        Ok(())
    }

    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // Reverse: remove from target, add back to source
        if let Some(dst) = self.to_package {
            let _ = model.remove_from_package(dst, self.element_id);
        }
        if let Some(src) = self.from_package {
            if model.contains(self.element_id) {
                model.add_to_package(src, self.element_id)?;
            }
        }
        Ok(())
    }

    fn description(&self) -> &str {
        &self.description
    }
}
```

---

## 6. Integration Test Plan

Create `crates/uml-core/tests/test_history.rs` for integration-level tests
that exercise the full stack: command creation → execute → undo → redo.

### 6.1 Test Scenarios

#### 1. Create then undo

```
1. Create a Class element via CreateElementCommand
2. Assert model contains the element (by ID)
3. Call history.undo()
4. Assert model does NOT contain the element
5. Assert element is back in the command (ready for redo)
```

#### 2. Create then undo then redo

```
1. Create Class via CreateElementCommand
2. Call history.undo()
3. Assert model is empty
4. Call history.redo()
5. Assert model contains the element with the SAME ID
```

#### 3. Rename then undo

```
1. Create Class with name "Original"
2. Execute RenameElementCommand to change to "Renamed"
3. Assert element name is "Renamed"
4. Call history.undo()
5. Assert element name is "Original"
```

#### 4. Delete then undo

```
1. Create Class
2. Execute DeleteElementCommand
3. Assert model is empty (element removed)
4. Call history.undo()
5. Assert model contains the element with original ID and data
```

#### 5. Full cycle: create → rename → undo × 3

```
1. Create Class "A"
2. Rename "A" → "B"
3. Delete Class
4. Call history.undo()  → restores "B"
5. Call history.undo()  → restores "A" (rename reversed)
6. Call history.undo()  → removes "A" (create reversed)
7. Assert model is empty
```

#### 6. Redo stack cleared on new action

```
1. Create Class (stack: [Create])
2. Call undo()  (stack: undo=[], redo=[Create])
3. Create another Class (stack: [Create], redo=[])
4. Assert can_redo() == false
```

#### 7. Command descriptions

```
1. CreateElementCommand::new(Class("Person"))
   → description() == "Create Class 'Person'"
2. DeleteElementCommand::new(model, id) where id refers to Class("Person")
   → description() == "Delete Class 'Person'"
3. RenameElementCommand::new(model, id, "Employee") where old name is "Person"
   → description() == "Rename 'Person' → 'Employee'"
```

#### 8. Disabled mode

```
1. history.set_disabled(true)
2. Execute CreateElementCommand
3. Assert can_undo() == false
4. Assert element exists in model (command still ran)
```

#### 9. Max depth enforcement

```
1. Create History with max_depth = 3
2. Execute 5 CreateElementCommand
3. Assert undo_depth() == 3
4. Undo all 3: assert each undo succeeds
```

#### 10. Double execute protection

```
1. Execute CreateElementCommand (succeeds)
2. Execute the SAME command again → returns Err(CommandError::InvalidOperation)
   (command is not pushed to stack on error)
```

#### 11. Move element between packages

```
1. Create Package "P1", Package "P2", and Class "C" (root-level)
2. Move "C" to "P1"
3. Assert parents_of(C) contains P1
4. Undo: assert parents_of(C) is empty (back to root)
5. Redo: assert parents_of(C) contains P1 again
```

### 6.2 Test Fixtures

A helper function for test setup:

```rust
/// Helper: create a simple model with a Class element, returning
/// the model, element ID, and a History.
fn setup_class_model() -> (UmlModel, UmlId, History) {
    let mut model = UmlModel::new();
    let cls = ModelElement::Class(Class::new("TestClass"));
    let id = cls.id();
    model.insert(cls);
    let history = History::new(100);
    (model, id, history)
}
```

For the integration test file, use `#[cfg(test)]` module within `tests/`
or a standalone `tests/test_history.rs` file. Standalone is preferred because
integration tests should be in the Cargo-integration-test directory.

---

## 7. Location and Module Structure

All code goes in `crates/uml-core/src/undo/`:

```
crates/uml-core/src/undo/
├── mod.rs           — Command trait, CommandError, History struct + impl
└── commands.rs      — CreateElementCommand, DeleteElementCommand,
                       RenameElementCommand, MoveElementCommand
```

### `mod.rs` structure

```rust
//! Undo/redo command system for Umbrello-RS.
//!
//! Implements the Command pattern using a bounded undo stack with
//! `Box<dyn Command>` dynamic dispatch. Commands are model-only —
//! they operate on `UmlModel` and have no GUI dependencies.
//!
//! See `docs/command_architecture_v1.md` for design rationale.

pub mod commands;

use std::any::Any;

use crate::id::UmlId;
use crate::repository::{ModelError, UmlModel};

// ─── Command trait ────────────────────────────────────────────────────

/// A reversible operation on a `UmlModel`.
pub trait Command: std::fmt::Debug + Send {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError>;
    fn description(&self) -> &str;
    fn merge(&self, _other: &dyn Command) -> Option<Box<dyn Command>> { None }
}

// ─── CommandError ─────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("element not found: {0}")]
    ElementNotFound(UmlId),
    #[error("invalid operation: {0}")]
    InvalidOperation(String),
    #[error("model error: {0}")]
    Model(#[from] ModelError),
}

// ─── History ──────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct History { ... }

impl History { ... }

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // Unit tests for History (stack management, descriptions, etc.)
}
```

### `commands.rs` structure

```rust
//! Concrete command implementations for common UML model mutations.

use crate::elements::ModelElement;
use crate::id::UmlId;
use crate::repository::UmlModel;
use crate::undo::{Command, CommandError};

// ─── CreateElementCommand ─────────────────────────────────────────────

pub struct CreateElementCommand { ... }

// ─── DeleteElementCommand ─────────────────────────────────────────────

pub struct DeleteElementCommand { ... }

// ─── RenameElementCommand ─────────────────────────────────────────────

pub struct RenameElementCommand { ... }

// ─── MoveElementCommand ───────────────────────────────────────────────

pub struct MoveElementCommand { ... }

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    // Unit tests for each command (execute, undo, description)
}
```

---

## 8. Future Extensions

### Merge support via `downcast-rs`

The `merge()` method on `Command` accepts `&dyn Command`. To determine whether
two commands are of the same type, the trait needs `Any`-like downcasting.
Options:

1. **`downcast-rs` crate:** Provides `Downcast` trait with `downcast_ref()`.
   Add `Downcast` as a supertrait of `Command`.
2. **Manual `Any` bound:** Add `fn type_id(&self) -> std::any::TypeId` to the
   trait and implement it via a macro.
3. **Skip merge for v1:** The default `merge()` returns `None`. Merge can be
   added in a later milestone without breaking changes.

For v1, option 3 is recommended. Merge is a UX refinement, not a correctness
requirement.

### Composite / macro commands

A `MacroCommand` (or `CompoundCommand`) that groups multiple sub-commands
into a single undo step:

```rust
pub struct MacroCommand {
    commands: Vec<Box<dyn Command>>,
    description: String,
}

impl Command for MacroCommand {
    fn execute(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        for cmd in &mut self.commands {
            cmd.execute(model)?;
        }
        Ok(())
    }
    fn undo(&mut self, model: &mut UmlModel) -> Result<(), CommandError> {
        // Undo in reverse order
        for cmd in self.commands.iter_mut().rev() {
            cmd.undo(model)?;
        }
        Ok(())
    }
    fn description(&self) -> &str {
        &self.description
    }
}
```

This is useful for:
- Creating an element and immediately adding it to a package (two commands
  grouped as one undo step).
- Creating a relationship (which involves adding two role elements and the
  association — a multi-step operation in the C++ Umbrello).

### Serialization of undo history

If crash-recovery auto-save is desired, add a method:

```rust
pub trait Command: std::fmt::Debug + Send {
    // ...
    fn serialize_state(&self) -> Option<serde_json::Value> { None }
}
```

Commands that support serialization return `Some(value)`. The `History` struct
can then dump the undo stack to a file during auto-save and restore it on
recovery.

### Command IDs for auditing

Add a monotonically increasing ID to each command for logging and audit trails:

```rust
pub trait Command: std::fmt::Debug + Send {
    fn command_id(&self) -> u64;
    // ...
}
```

This is low priority and can be added as a wrapper struct `IdentifiedCommand`
that wraps any `Box<dyn Command>` and assigns an ID.

### GUI integration

In a future milestone, the `History` will be wired to:
- Edit → Undo / Redo menu items (enable/disable based on `can_undo/redo`)
- Keyboard shortcuts (Ctrl+Z / Ctrl+Shift+Z)
- Undo history panel showing the command list

The GUI layer will hold `Rc<RefCell<History>>` or `Arc<Mutex<History>>`
(whichever the widget framework uses) and call `history.execute()` before
every model mutation.

---

## Appendix: Current State of `undo/mod.rs` (Before Replacement)

The existing stub at `crates/uml-core/src/undo/mod.rs`:

```rust
//! Undo/redo command system for Umbrello-RS.
//!
//! Implements the Command pattern using a bounded undo stack. Commands are
//! model-only — they operate on `ModelRepository` and have no GUI dependencies.
//! Supports macro grouping for multi-step operations.

/// The undo/redo stack.
#[derive(Debug)]
pub struct UndoStack;

impl UndoStack {
    /// Create a new undo stack with the given maximum depth.
    #[must_use]
    pub fn new(_max_depth: usize) -> Self { Self }

    /// Returns whether there are commands to undo.
    #[must_use]
    pub fn can_undo(&self) -> bool { false }

    /// Returns whether there are commands to redo.
    #[must_use]
    pub fn can_redo(&self) -> bool { false }
}

impl Default for UndoStack {
    fn default() -> Self { Self::new(100) }
}
```

This stub is replaced in its entirety by the architecture described above.
The `UndoStack` name is changed to `History` for clarity (a stack is a data
structure; a history manages undo/redo semantics).
