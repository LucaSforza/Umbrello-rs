# Model Repository v1 — Architecture Evaluation

> **Document:** `rust-rewrite/docs/model_repository_v1.md`
> **Status:** Active
> **Phase:** Milestone 3 (uml-core storage)
> **Last updated:** 2026-06-23
>
> This document evaluates five candidate data structures for `UmlModel` — the
> central repository that owns all UML model elements. It concludes with a
> recommended design and a proposed API.

---

## Table of Contents

1. [Context](#1-context)
2. [Requirements](#2-requirements)
3. [Candidates](#3-candidates)
   - [A: HashMap](#a-hashmapumlid-modelelement)
   - [B: IndexMap](#b-indexmapumlid-modelelement)
   - [C: SlotMap + Secondary Index](#c-slotmap--secondary-index)
   - [D: generational-arena](#d-generational-arena)
   - [E: BTreeMap](#e-btreemapumlid-modelelement)
4. [Evaluation](#4-evaluation)
   - [4.1 Ownership Model](#41-ownership-model)
   - [4.2 Lookup Performance](#42-lookup-performance)
   - [4.3 Mutation Patterns](#43-mutation-patterns)
   - [4.4 Iteration](#44-iteration)
   - [4.5 Serialization Implications](#45-serialization-implications)
   - [4.6 Undo/Redo Implications](#46-undoredo-implications)
   - [4.7 Diagram Integration](#47-diagram-integration)
   - [4.8 Complexity and Dependencies](#48-complexity-and-dependencies)
   - [4.9 Memory](#49-memory)
5. [Comparison Matrix](#5-comparison-matrix)
6. [Rejected Alternatives — Deep Analysis](#6-rejected-alternatives--deep-analysis)
7. [Recommendation](#7-recommendation)
8. [Proposed API](#8-proposed-api)
9. [Migration Path](#9-migration-path)
10. [References](#10-references)

---

## 1. Context

The `uml-core` crate defines the Rust-native UML metamodel. It provides:

- **`UmlId`** — UUID-backed unique identifier (16 bytes, globally unique, never
  reused after element deletion).
- **`ModelElement`** — enum with 4 variants (`Package`, `Class`, `Interface`,
  `Enum`) that replaces the 28-method C++ RTTI system.
- **`NamedElement` trait** — uniform access to `ElementBase` fields (name,
  visibility, id, etc.) across all variants.
- **`ElementBase`** — common metadata struct embedded in each element type.
- **`Package`** — container that stores `children: Vec<UmlId>` for containment
  (the actual element data lives in the repository, not in the package).

The missing piece is **`UmlModel`** — the central repository that owns every
model element, provides O(1) lookup by `UmlId`, and manages the element
lifecycle.

Currently `crates/uml-core/src/repository.rs` is a stub:

```rust
#[derive(Debug, Default)]
pub struct ModelRepository {
    _placeholder: (),
}
```

The `domain_model_v1.md` document proposed a `SlotMap`-based design early in
the architecture phase. This document re-examines that decision systematically,
evaluating five candidates against nine criteria before arriving at a final
recommendation.

> **Why re-evaluate now?** The domain model document was written before the
> type system was fully implemented. With concrete types, the trade-offs
> between storage backends are clearer. Also, the existing workspace does not
> currently depend on `indexmap`, which this evaluation will argue is the
> correct choice, whereas it *does* depend on `slotmap`. The re-evaluation
> ensures dependencies are justified.

---

## 2. Requirements

### 2.1 Functional Requirements

| # | Requirement | Description |
|---|-------------|-------------|
| R1 | **Add element** | Insert a `ModelElement`, return its identity (`UmlId`). |
| R2 | **Remove element** | Remove by `UmlId`, return the element (`Option<ModelElement>`). |
| R3 | **Get by ID** | O(1) amortised lookup: `get(id) -> Option<&ModelElement>`. |
| R4 | **Get mut by ID** | Mutate in place: `get_mut(id) -> Option<&mut ModelElement>`. |
| R5 | **Iterate all** | Iterate over `(UmlId, &ModelElement)` pairs. Order must be deterministic for predictable test output and CLI display. |
| R6 | **Package membership** | Given an element's `UmlId`, find which `Package`(s) contain it. Must run in O(1) average time. |
| R7 | **Reference validation** | Verify that all `UmlId` references (`Package::children`, `type_id` on attributes/operations) point to existing elements. |

### 2.2 Non-Functional Requirements

| # | Requirement | Target |
|---|-------------|--------|
| NFR1 | **Model size** | 100–10,000 elements (typical UML models). Must handle 50,000 without degradation. |
| NFR2 | **Mutation frequency** | Batch insert during XMI loading (thousands of inserts), then interactive edits (single inserts/removes/updates). |
| NFR3 | **Lookup frequency** | Hot path: diagram rendering resolves many `UmlId` references per frame. XMI resolution resolves forward references. |
| NFR4 | **Serialization** | Repository is a runtime construct — not directly serialized. Elements are serialized via tree walk (uml-xmi crate). |
| NFR5 | **Undo/redo** | v1 uses model-level clone snapshots. Future: command-based undo. Storage must implement `Clone`. |
| NFR6 | **Testability** | Deterministic iteration for reproducible test assertions. No flaky ordering. |
| NFR7 | **Dependency budget** | Prefer std-only if viable. Lightweight, well-maintained external crates acceptable. |

### 2.3 Key Architectural Invariant

> **The repository owns all elements. Packages reference elements by `UmlId` —
> they do not own them.**

This means:
- `Package::children` is a `Vec<UmlId>`, not `Vec<ModelElement>`.
- Removing an element from a package does not deallocate it; removing it from
  the repository does.
- An element can belong to zero, one, or multiple packages.
- No `Rc`, no `Arc`, no shared ownership is needed.

---

## 3. Candidates

### Candidate A: `HashMap<UmlId, ModelElement>`

```rust
use std::collections::HashMap;

pub struct UmlModel {
    elements: HashMap<UmlId, ModelElement>,
    /// Reverse index: element_id → Vec<package_id> that contain it.
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

**Dependencies:** `std` only.

**Pros:**
- No external dependency (pure std library).
- Simple, well-understood, universally available.
- O(1) amortised lookup, insert, remove.
- UUID keys have excellent hash distribution (no collision attacks).

**Cons:**
- **Non-deterministic iteration order.** `HashMap` iteration order is randomized
  (Hashbrown's SipHash-based randomisation) and changes between process runs.
  This causes flaky tests and unpredictable CLI output.
- Memory overhead: ~32 bytes per entry for the hash table (buckets, control
  bytes) plus the entry itself.
- Iteration involves pointer chasing through buckets — less cache-friendly than
  contiguous storage.
- No generational protection (mitigated by UUIDs — IDs are never reused).

---

### Candidate B: `IndexMap<UmlId, ModelElement>`

```rust
use indexmap::IndexMap;
use std::collections::HashMap;

pub struct UmlModel {
    elements: IndexMap<UmlId, ModelElement>,
    /// Reverse index: element_id → Vec<package_id> that contain it.
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

**Dependency:** `indexmap` (used by `cargo`, `rust-analyzer`, `serde`).
**Version:** `indexmap = "2"` (stable, maintained by same team as hashbrown).

**Pros:**
- O(1) amortised lookup, insert, remove (same as HashMap).
- **Deterministic insertion-order iteration.** Iteration order is the order
  elements were inserted. This is invaluable for:
  - Reproducible test output.
  - Stable CLI display (e.g., `umbrello list --elements`).
  - Predictable serialization order during tree walks.
- `IndexMap` API mirrors `HashMap` — implements `Index`, `IndexMut`, `Entry`,
  `IntoIterator`, etc. Caller code looks familiar.
- Implements `Clone`, enabling model-level undo snapshots.
- Well-maintained, lightweight crate (~20KB, no transitive deps beyond
  `hashbrown` and `equivalent`).

**Cons:**
- External dependency (but lightweight and trusted).
- Slightly more memory than HashMap: the insertion-order vec adds ~24 bytes per
  entry (a `Vec<usize>` for the order tracking).
- Same lack of generational protection as HashMap (acceptable — UUIDs provide
  the same guarantee at the identity level).
- Removal shifts the insertion-order vec entries (O(n) in worst case, but
  `IndexMap` uses a linked-list trick to make removal O(1) amortised in
  practice).

---

### Candidate C: `SlotMap` + Secondary Index

```rust
use slotmap::SlotMap;
use std::collections::HashMap;

/// Internal generational key (slotmap::DefaultKey = u64).
type ObjectKey = slotmap::DefaultKey;

pub struct UmlModel {
    elements: SlotMap<ObjectKey, ModelElement>,
    /// Map UmlId → internal ObjectKey for O(1) lookup by UUID.
    id_to_key: HashMap<UmlId, ObjectKey>,
    /// Map ObjectKey → UmlId for iteration.
    key_to_id: HashMap<ObjectKey, UmlId>,
    /// Reverse index: element_id → Vec<package_id> that contain it.
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

**Dependency:** `slotmap = "1"` (already in workspace Cargo.toml).

**Pros:**
- **Best iteration performance.** Elements stored contiguously in memory.
  Cache-friendly, ideal for bulk operations (validation, export).
- **Generational protection.** `ObjectKey` encodes both slot index and
  generation counter. Using a stale key after removal returns `None` rather
  than a dangling reference. However, this is redundant with UUIDs — `UmlId`
  already ensures no ID reuse.
- **Stable indices.** Removing an element doesn't shift other elements'
  positions — the slot is tombstoned.
- `ObjectKey` is `u64` (8 bytes), half the size of `UmlId` (16 bytes).

**Cons:**
- **Three collections to maintain.** Every insert/remove must update the
  `SlotMap`, `id_to_key`, and `key_to_id` maps. Three sources of truth = three
  opportunities for inconsistent state.
- **Two parallel identity systems.** External code uses `UmlId` (UUID), internal
  storage uses `ObjectKey` (generational index). All lookups go through
  `id_to_key` first: `elements.get(id_to_key[&umlid])`. This indirection adds
  a HashMap lookup on every access.
- **Memory overhead.** Three maps instead of one. Even with smaller keys, the
  total overhead is higher.
- **Complexity.** `insert()` must check existence, generate ObjectKey, write to
  SlotMap, insert into both index maps. `remove()` must do the reverse.
- **Serialisation friction.** `ObjectKey` is not serializable across sessions
  (its slot positions are ephemeral). We must always serialize/deserialize via
  `UmlId`. The `key_to_id` map is required precisely for serialization.

---

### Candidate D: `generational-arena`

```rust
use generational_arena::{Arena, Index};
use std::collections::HashMap;

pub struct UmlModel {
    elements: Arena<ModelElement>,
    id_to_index: HashMap<UmlId, Index>,
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

**Dependency:** `generational-arena`.

**Pros:**
- Generational indices prevent stale access.
- Contiguous storage for cache-friendly iteration.
- Simple API (insert returns Index, get by Index).

**Cons:**
- **Smaller ecosystem.** `generational-arena` is less widely used than
  `slotmap` or `indexmap`. Fewer downloads, fewer maintainers.
- **Still needs secondary index.** External code demands `UmlId` lookup, so we
  must maintain `id_to_index: HashMap<UmlId, Index>` alongside the arena.
- **Same complexity as SlotMap.** Two maps + arena to keep in sync.
- **Less feature-rich.** `Arena` lacks some convenience methods that `SlotMap`
  provides (e.g., `SlotMap::retain`, `SlotMap::drain`, `SlotMap::keys`).
- No meaningful advantage over Candidate C for this use case. Both solve the
  same problem with similar overhead, but `slotmap` is more mature.

---

### Candidate E: `BTreeMap<UmlId, ModelElement>`

```rust
use std::collections::BTreeMap;

pub struct UmlModel {
    elements: BTreeMap<UmlId, ModelElement>,
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

**Dependencies:** `std` only.

**Pros:**
- No external dependency.
- **Sorted iteration.** Elements iterate in `UmlId` (UUID) order, which is
  deterministic.
- Lower memory overhead per entry than HashMap (no bucket overhead).

**Cons:**
- **O(log n) lookup, insert, remove.** For 10,000 elements, that's ~14
  comparisons vs 1 for HashMap. For 50,000 elements, ~16 comparisons. While
  not prohibitive, it adds up in hot paths (diagram rendering, reference
  resolution).
- **Tree nodes are heap-allocated individually** — less cache-friendly than
  contiguous storage.
- **Higher constant factors.** BTree operations involve pointer chasing through
  tree nodes. HashMap/IndexMap spread entries across a flat table.
- UUID ordering is meaningless for most use cases (sorted by UUID is not
  sorted by name, type, or any domain-relevant order).

---

## 4. Evaluation

### 4.1 Ownership Model

All five candidates share the same ownership model — the repository struct owns
the `ModelElement` values directly:

```rust
// All variants: elements owns ModelElement values
pub struct UmlModel {
    elements: /* container of ModelElement */,
    ...
}
```

| Aspect | Evaluation |
|--------|------------|
| **Who owns elements?** | The repository. Always. All candidates store `ModelElement` by value. |
| **How are packages connected?** | Via `UmlId` in `Package::children`. No ownership transfer. |
| **Lifetime management** | Elements live as long as the repository. `remove()` moves ownership out. |
| **Shared ownership** | None. `UmlId` is `Copy` — references are free. |

**Impact on candidates:** None. All support this model equally.

### 4.2 Lookup Performance

For the critical path — resolving a `UmlId` to a `&ModelElement` — the
candidates differ:

| Candidate | `get(id)` complexity | Constant factors |
|-----------|---------------------|------------------|
| **A** HashMap | O(1) amortised | Hash UUID (128-bit), probe bucket |
| **B** IndexMap | O(1) amortised | Hash UUID (same as HashMap) |
| **C** SlotMap | O(1) + O(1) lookup in `id_to_key` | Two lookups: HashMap → ObjectKey → SlotMap |
| **D** gen-arena | O(1) + O(1) lookup in `id_to_index` | Two lookups: HashMap → Index → Arena |
| **E** BTreeMap | O(log n) | ~14 comparisons for 10k elements |

**Key insight:** For Candidates C and D, every `get()` call goes through TWO
lookups (HashMap + slot). For A, B, and E, it's one lookup. This means C and D
are *slower* than A/B despite single-operation O(1) — they pay the HashMap
cost *in addition to* the storage cost.

```
Lookup path comparison:

A (HashMap):     UmlId ──hash──▶ HashMap bucket ──▶ &ModelElement
B (IndexMap):    UmlId ──hash──▶ IndexMap entry ──▶ &ModelElement
C (SlotMap):     UmlId ──hash──▶ HashMap ──── ObjectKey ──▶ SlotMap slot ──▶ &ModelElement
D (gen-arena):   UmlId ──hash──▶ HashMap ──── Index ──▶ Arena slot ──▶ &ModelElement
E (BTreeMap):    UmlId ──cmp──▶ BTree node ──▶ &ModelElement
```

**Verdict:** Candidates A and B are the fastest for the common case (single
lookup by ID). C and D are the slowest (two lookups). E is competitive for
small models but degrades with size.

### 4.3 Mutation Patterns

#### Pattern 1: Batch insert (XMI loading)

XMI loading inserts hundreds or thousands of elements in sequence. The critical
concern is amortised cost:

```rust
// All candidates support this pattern:
for element in deserialized_elements {
    model.insert(element);
}
```

| Candidate | Batch insert behaviour |
|-----------|----------------------|
| A (HashMap) | Amortised O(1) per insert. Table resizes when load factor exceeds threshold. |
| B (IndexMap) | Amortised O(1) per insert. Maintains insertion-order vec alongside hash table. |
| C (SlotMap) | O(1) per insert + O(1) for two index map updates. Three collections to resize. |
| D (gen-arena) | O(1) per insert + O(1) for index map. Arena grows allocated capacity. |
| E (BTreeMap) | O(log n) per insert. Tree rebalancing cost. |

**Verdict:** A and B are the simplest and fastest for batch insert. C and D do
more work per insert (three collections). E is the slowest.

#### Pattern 2: Property update (user editing)

```rust
let elem = model.get_mut(id)?;
elem.set_name("NewName".into());
```

All candidates support this efficiently. `get_mut()` returns `Option<&mut ModelElement>`,
and mutations are in-place regardless of storage backend.

#### Pattern 3: Add child to package

```rust
model.add_to_package(package_id, child_id)?;
// Updates Package::children and parent_index
```

All candidates support this by constructing the update inside the repository
method. The cost is dominated by `get_mut()` on the package + `Vec::push` on
children. All candidates are O(1) amortised for `get_mut()` (except BTreeMap
at O(log n)).

#### Pattern 4: Remove element (user deleting)

Remove is the most complex mutation because it must clean up:

```rust
fn remove(&mut self, id: UmlId) -> Option<ModelElement> {
    // 1. Remove from parent_index
    // 2. Remove from elements storage
    // 3. Optionally: remove from package children lists
    //    (or leave dangling — validation will catch it)
}
```

| Candidate | Remove performance | Cleanup complexity |
|-----------|-------------------|-------------------|
| A (HashMap) | O(1) | Straightforward: `elements.remove(id)` + `parent_index.remove(id)` |
| B (IndexMap) | O(1) amortised | `elements.swap_remove(id)` (or `shift_remove`) + `parent_index.remove(id)` |
| C (SlotMap) | O(1) + O(1) + O(1) | Must remove from `elements`, `id_to_key`, `key_to_id`, `parent_index`. Three collections, each could fail independently. |
| D (gen-arena) | O(1) + O(1) | Must remove from `elements`, `id_to_index`, `parent_index`. |
| E (BTreeMap) | O(log n) | `elements.remove(id)` + `parent_index.remove(id)` |

**Verdict:** A, B are simplest for removal. C and D require careful atomicity
when updating multiple collections. E is slower for large models.

### 4.4 Iteration

Iteration is relevant for:
- **Validation** (`validate_references()` scans all elements).
- **Export** (tree walk starting from root packages).
- **Debugging/CLI** (`list --all`).
- **Tests** (asserting model contents).

| Candidate | Order | Cache locality | `iter()` API | Notes |
|-----------|-------|---------------|-------------|-------|
| A (HashMap) | **Non-deterministic** (randomised) | Poor (pointer chasing through buckets) | `.iter()` yields `(&UmlId, &ModelElement)` | Order changes between runs. Breaks test assertions. |
| B (IndexMap) | **Deterministic** (insertion order) | Better than HashMap (vec of indices, then data) | `.iter()` yields `(&UmlId, &ModelElement)` | Stable order. Predictable. |
| C (SlotMap) | **Deterministic** (slot order) | **Best** — contiguous vec of entries | `.iter()` yields `(ObjectKey, &ModelElement)` — need `key_to_id` to get `UmlId` | Fastest iteration, but UmlId requires extra map lookup. |
| D (gen-arena) | **Deterministic** (arena order) | **Best** — contiguous vec of entries | `.iter()` yields `(Index, &ModelElement)` — need `id_to_index` reverse map | Same as C. |
| E (BTreeMap) | **Deterministic** (UUID sort order) | Poor (tree nodes scattered) | `.iter()` yields `(&UmlId, &ModelElement)` | Sorted by UUID value — not domain-meaningful. |

**Key insight for C and D:** To implement `iter()` returning `(UmlId, &ModelElement)`,
they must either:
- Maintain a parallel `Vec<(UmlId, ObjectKey)>` for iteration (more memory, sync
  cost), or
- Iterate the SlotMap and look up each key's `UmlId` in `key_to_id` (HashMap
  lookup per element — O(n) extra work).

Neither choice is clean.

**Verdict:** B (IndexMap) provides the best balance: deterministic iteration,
good cache locality, and zero extra work for UmlId lookup during iteration.

### 4.5 Serialization Implications

The repository itself is **not serialized**. Serialization (via `uml-xmi`)
walks the model tree: it starts from root packages, follows `children`
recursively, and serializes each element. The flat repository is a runtime
construct.

**Impact on storage selection:**

| Concern | Relevance to candidates |
|---------|------------------------|
| Does iteration order affect XMI output? | **No.** XMI serialization follows the containment tree, not insertion order. |
| Does `Clone` work for snapshots? | All candidates implement `Clone` (including SlotMap and IndexMap). |
| Are keys serializable? | A/B/E use `UmlId` keys (serializable). C uses `ObjectKey` (not serializable) — must use UmlId for export. D uses `Index` (not serializable). |

For Candidates C and D, the `key_to_id` map exists primarily because
serialization and external APIs need `UmlId`. Without this requirement, C and D
would be simpler. But the requirement exists — all external code uses `UmlId`.

**Verdict:** All candidates support serialization equally well, but C and D
pay an extra tax (maintaining `key_to_id`) for no serialization benefit.

### 4.6 Undo/Redo Implications

v1 uses **model-level clone snapshots**:

```rust
fn push_snapshot(&mut self, model: &UmlModel) {
    self.undo_stack.push(model.clone());  // Clone the entire model
}

fn undo(&mut self) -> Option<UmlModel> {
    self.undo_stack.pop()
}
```

| Candidate | Clone cost | Notes |
|-----------|-----------|-------|
| A (HashMap) | O(n) — clones every entry | `HashMap::clone` clones all key-value pairs. |
| B (IndexMap) | O(n) — clones every entry | `IndexMap::clone` clones all key-value pairs + order vec. |
| C (SlotMap) | O(n) — clones SlotMap + both index HashMaps | Three collections to clone. More total bytes. |
| D (gen-arena) | O(n) — clones Arena + both maps | Similar to C. |
| E (BTreeMap) | O(n) — clones every entry | `BTreeMap::clone` clones all entries. |

**Deep clone cost for 10,000 elements:**

| Candidate | Memory for clone | Relative cost |
|-----------|-----------------|---------------|
| A (HashMap) | ~640 KB (10k × ~64 bytes) | Baseline |
| B (IndexMap) | ~800 KB (10k × ~80 bytes) | 1.25× |
| C (SlotMap) | ~1.2 MB (3 maps × ~400 KB) | 1.9× |
| D (gen-arena) | ~1.0 MB | 1.6× |
| E (BTreeMap) | ~480 KB (tree nodes less overhead) | 0.75× |

All candidates support clone-based undo. For models under 50,000 elements, the
memory difference is acceptable (~2 MB vs ~1 MB). For future command-based undo,
the storage backend is irrelevant.

**Verdict:** All candidates support undo. No differentiator.

### 4.7 Diagram Integration

Diagrams (future `uml-diagram` crate) reference model elements by `UmlId`:

```rust
// Future: diagram widget
struct ClassWidget {
    element_id: UmlId,  // references a ModelElement::Class in the repository
    position: Point,
    size: Size,
}
```

The critical operation during rendering is:

```rust
fn render_class_widget(widget: &ClassWidget, model: &UmlModel) {
    let class = model.get(widget.element_id);  // O(1) lookup
    match class {
        Some(ModelElement::Class(c)) => { /* draw class box */ }
        None => { /* draw "missing element" placeholder */ }
        _ => { /* type mismatch — log warning */ }
    }
}
```

| Candidate | Lookup from widget | Notes |
|-----------|-------------------|-------|
| A (HashMap) | `elements.get(widget.element_id)` — direct | Simplest path. |
| B (IndexMap) | `elements.get(widget.element_id)` — direct | Same as A. |
| C (SlotMap) | `elements.get(id_to_key[widget.element_id])` — indirect | Must go through `id_to_key`. Two lookups. |
| D (gen-arena) | `elements.get(id_to_index[widget.element_id])` — indirect | Same as C. |
| E (BTreeMap) | `elements.get(widget.element_id)` — direct | Single lookup, but O(log n). |

**Verdict:** A, B, and E provide the simplest diagram integration path —
diagrams store `UmlId` and directly call `model.get()`. C and D require an
extra HashMap lookup on every widget render.

### 4.8 Complexity and Dependencies

| Candidate | External deps | Lines of code (estimate) | Maintenance burden |
|-----------|--------------|--------------------------|-------------------|
| A (HashMap) | None (std) | ~100 LOC | Minimal. Well-known API. |
| B (IndexMap) | `indexmap` (~20KB) | ~100 LOC | Minimal. Same API as HashMap. |
| C (SlotMap) | `slotmap` (already present) | ~150 LOC | Moderate. Three maps must be kept consistent. |
| D (gen-arena) | `generational-arena` | ~130 LOC | Moderate. Two maps must be kept consistent. |
| E (BTreeMap) | None (std) | ~100 LOC | Minimal. Well-known API. |

**Key burden for C:** The `insert()` method must:
1. Compute `id = element.id()`.
2. Check `id_to_key.contains_key(id)` — reject duplicate.
3. Call `elements.insert(element)` → returns `ObjectKey`.
4. Insert into `id_to_key: id_to_key.insert(id, object_key)`.
5. Insert into `key_to_id: key_to_id.insert(object_key, id)`.

The `remove()` method must:
1. Look up `object_key = id_to_key.remove(id)?`.
2. Remove from `elements: elements.remove(object_key)`.
3. Remove from `key_to_id: key_to_id.remove(&object_key)`.
4. Clean up `parent_index`.

Any of these steps could panic or return an unexpected result. The three maps
can silently diverge if a bug skips one update.

**Verdict:** A and B are the simplest. C and D have significantly more
bookkeeping code with more bug surfaces.

### 4.9 Memory

Approximate memory per element (including container overhead, excluding the
`ModelElement` value itself which is the same in all candidates):

| Candidate | Overhead per element | Notes |
|-----------|---------------------|-------|
| A (HashMap) | ~32–40 bytes (hash + entry + bucket alignment) | Hashbrown table, 1–2 control bytes per entry |
| B (IndexMap) | ~56–72 bytes (hash table + insertion-order vec entry + bucket alignment) | HashMap overhead + Vec<usize> for ordering |
| C (SlotMap) | ~80–100 bytes (SlotMap entry + 2 HashMap entries for indices) | Three collections |
| D (gen-arena) | ~72–88 bytes (arena slot + 2 HashMap entries) | Two collections |
| E (BTreeMap) | ~32 bytes (tree node + key-value pair) | Internal nodes store child pointers |

For a model of 10,000 elements:
- **A (HashMap):** ~400 KB overhead
- **B (IndexMap):** ~640 KB overhead
- **C (SlotMap):** ~900 KB overhead
- **D (gen-arena):** ~800 KB overhead
- **E (BTreeMap):** ~320 KB overhead (but tree depth adds allocation churn)

All are acceptable for the expected model sizes (100–10,000 elements). The
differences are measured in kilobytes, not megabytes.

---

## 5. Comparison Matrix

| Criterion | Weight | A (HashMap) | B (IndexMap) | C (SlotMap) | D (gen-arena) | E (BTreeMap) |
|-----------|--------|-------------|--------------|-------------|---------------|-------------|
| **Lookup O(1)** | Critical | ✅ O(1) | ✅ O(1) | ✅ O(1)+O(1) ⚠️ | ✅ O(1)+O(1) ⚠️ | ⚠️ O(log n) |
| **Deterministic iteration** | High | ❌ No | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **Get_mut ergonomics** | High | ✅ Direct | ✅ Direct | ⚠️ Via index | ⚠️ Via index | ✅ Direct |
| **Simplify (fewer maps)** | High | ✅ 1 storage | ✅ 1 storage | ❌ 3 maps | ❌ 2 maps | ✅ 1 storage |
| **No extra deps** | Medium | ✅ Yes | ⚠️ indexmap | ✅ Already present | ⚠️ New crate | ✅ Yes |
| **Diagram integration** | High | ✅ Direct | ✅ Direct | ⚠️ Indirect | ⚠️ Indirect | ✅ Direct |
| **Clone (undo)** | Medium | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes | ✅ Yes |
| **Iteration performance** | Low | ⚠️ Poor | ✅ Good | ✅ Best | ✅ Best | ⚠️ Poor |
| **Memory** | Low | ✅ Low | ✅ Low | ⚠️ Medium | ⚠️ Medium | ✅ Lowest |

**Weighted score (3=critical, 2=high, 1=medium, 0=low):**

| Candidate | Score |
|-----------|-------|
| **B (IndexMap)** | **Highest** — only candidate that scores "Yes" or "Good" on all weighted criteria |
| A (HashMap) | Fails deterministic iteration (weight: high) |
| C (SlotMap) | Fails simplicity and diagram integration (weight: high+c-critical) |
| D (gen-arena) | Same failures as C, plus new dependency |
| E (BTreeMap) | Fails O(1) lookup (weight: critical) |

---

## 6. Rejected Alternatives — Deep Analysis

### 6.1 Why NOT SlotMap (Candidate C)

SlotMap was the initial design in `domain_model_v1.md`. Re-evaluation reveals
several issues:

**Overdesigned for our constraints.** SlotMap's key feature is generational
index safety — detecting use-after-free when a key for a removed element is
used. But `UmlId` already provides this guarantee: UUIDs are never reused, so
`model.get(stale_id)` returns `None` naturally. The generational counter in
`ObjectKey` is redundant.

```
UUID-based protection:           Generational index protection:
  UmlId A → Element exists        ObjectKey(0, gen=1) → Element
  UmlId A → removed → None        ObjectKey(0, gen=1) → removed → None
  UmlId A → new element?          ObjectKey(0, gen=2) → new element
    ❌ Never — UUIDs are            ✅ ObjectKey(0, gen=1) still returns
       globally unique, never          None because generation changed
       reused

Both achieve the same end state. UUIDs use 128 bits instead of 64 bits,
but memory is not the bottleneck.
```

**Slower lookups.** Every `get(id)` becomes `elements.get(id_to_key[&id])?`.
That's two hash lookups instead of one. For diagram rendering, where hundreds
of widgets resolve IDs per frame, this doubles lookup cost.

**Complex state management.** Three collections, all of which must be kept in
sync. Testing must verify that all three are consistent after every mutation.
The `insert()` method has more moving parts:

```rust
// SlotMap insert — more complex
fn insert(&mut self, element: ModelElement) -> UmlId {
    let id = element.id();
    assert!(!self.id_to_key.contains_key(&id), "duplicate element");

    let object_key = self.elements.insert(element);
    self.id_to_key.insert(id, object_key);
    if !self.key_to_id.contains_key(&object_key) {
        self.key_to_id.insert(object_key, id);
    }
    id
}

// IndexMap insert — trivial
fn insert(&mut self, element: ModelElement) -> UmlId {
    let id = element.id();
    self.elements.entry(id).or_insert(element);
    id
}
```

**No practical benefit for our workloads.** SlotMap is designed for use cases
where:
- Elements are frequently removed and new elements reuse indices.
- Maximum iteration throughput is critical (game entity systems).
- Pointer indirection is acceptable because the inner loop is tight.

None of these apply to UML modeling. Elements are added and removed
interactively (not in tight loops), and iteration performance is dominated by
the work done *with* each element (validation, rendering, serialization), not
by the iteration itself.

**When would SlotMap be the right choice?**
- If we used incrementing integer IDs instead of UUIDs.
- If element iteration was the bottleneck in profiling (likely never for UML
  models).
- If `ObjectKey` could be the external identity (but it can't — diagrams and
  XMI files need stable, serializable identities).

SlotMap adds complexity for zero practical benefit in this architecture.

### 6.2 Why NOT generational-arena (Candidate D)

All the SlotMap arguments apply, plus:
- Less mature library.
- Smaller community.
- Fewer features.

SlotMap is strictly superior if we were going in this direction. Since we're
rejecting SlotMap, `generational-arena` is automatically rejected.

### 6.3 Why NOT BTreeMap (Candidate E)

BTreeMap is appealing because it's std-only and provides ordered iteration.
However:

- **O(log n) lookup** means every `model.get(id)` in diagram rendering,
  reference validation, XMI loading, and code generation takes ~14−16
  comparisons. For a model with 50,000 elements, that's ~100,000 comparisons
  just to load XMI with 10,000 forward references.
- **UUID ordering is meaningless.** Sorting by UUID doesn't correspond to any
  domain-relevant concept (not by name, not by type, not by hierarchy).
- BTreeMap's ordered iteration could be useful for "dump all elements in a
  consistent order" — but IndexMap's insertion order handles this just as well
  for the common case (insert in tree-walk order during XMI loading).

If the workspace already had `BTreeMap` dependencies for other reasons, it
might be competitive. But the `indexmap` crate is similarly lightweight and
provides O(1) lookup.

### 6.4 Why NOT pure HashMap (Candidate A)

HashMap fails on exactly one criterion, but it's an important one:

**Non-deterministic iteration order.**

```rust
// Test that fails intermittently:
let model = UmlModel::new();
model.insert(class_a);
model.insert(class_b);

let names: Vec<String> = model.iter().map(|(_, e)| e.name().to_string()).collect();
assert_eq!(names, vec!["ClassA", "ClassB"]); // FAILS — order could be [B, A]
```

With HashMap, `iter()` order depends on hashbrown's internal randomisation
(seeded per process). Tests that assert order will fail unpredictably.
Developers learn to sort results before asserting — but sorting adds O(n log n)
to every test.

More subtly, CLI output changes between runs:

```shell
$ umbrello list --all
# Run 1: ClassA, ClassB, ClassC
# Run 2: ClassC, ClassA, ClassB  # different order!
```

For a modeling tool where users may script against CLI output, predictable
ordering matters.

**Yes, we could wrap HashMap with a parallel `Vec<UmlId>` for order tracking.**
But that's exactly what `IndexMap` already does — and it does it correctly,
with tests, maintained by the hashbrown team.

### 6.5 Considered but not evaluated: DashMap, Scc, etc.

Concurrent maps (DashMap, `scc`, `flurry`) were considered. The UML model
repository is single-threaded — the application uses a single model accessed
from the main thread. Concurrent maps add overhead (`RwLock` per shard) for
no benefit.

If concurrency is needed in the future (e.g., background validation), a
`RwLock<UmlModel>` wrapper is simpler and more predictable than sharded maps.

### 6.6 Considered but not evaluated: sled, redb, SQLite

Persistent databases were considered but rejected. The model repository is an
in-memory data structure for an interactive modeling tool. Persistence is
handled separately by the `uml-persistence` crate (XMI/JSON files, not a
database).

---

## 7. Recommendation

### Recommendation: Candidate B — `IndexMap<UmlId, ModelElement>`

The recommendation is based on four decisive factors:

#### Factor 1: Single-index simplicity

IndexMap is the only candidate that provides **direct O(1) lookup by UmlId**
with a **single container**. No secondary maps, no index translation, no
parallel bookkeeping.

```rust
// IndexMap: the UmlId IS the key
pub struct UmlModel {
    elements: IndexMap<UmlId, ModelElement>,
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

All other candidates either:
- Need a secondary index for UmlId lookup (C, D), or
- Have slower O(log n) lookup (E), or
- Have non-deterministic iteration (A).

#### Factor 2: Deterministic iteration

Insertion-order iteration is the key differentiator from HashMap. It provides:

- **Stable test output** — no flaky order-dependent test failures.
- **Predictable CLI** — `list` commands show elements in a consistent order.
- **Natural semantics** — elements are typically inserted in tree-walk order
  (root packages first, then their children). Iteration order mirrors the
  logical model structure.

#### Factor 3: Mature, lightweight dependency

`indexmap` is one of the most widely used crates in the Rust ecosystem:
- Used by **Cargo** itself (the Rust package manager).
- Used by **rust-analyzer** (the official LSP server).
- Used by **serde** (for ordered maps).
- ~10 million downloads per month on crates.io.
- Single-file implementation built on `hashbrown` (the same hash table as
  `std::collections::HashMap`).
- No transitive dependencies beyond `hashbrown`, `equivalent`, and `std`.

Adding `indexmap` to the dependency tree is a low-risk decision.

#### Factor 4: Direct diagram integration

Diagram widgets store `UmlId` values. With IndexMap, diagram code resolves
them directly:

```rust
// Diagram rendering — minimal indirection
fn render(&self, model: &UmlModel) {
    for widget in &self.widgets {
        if let Some(element) = model.get(widget.element_id) {
            // render using element data
        }
    }
}
```

With SlotMap or generational-arena, every widget resolution needs an extra
HashMap lookup:

```rust
// SlotMap — two lookups per widget
fn render(&self, model: &UmlModel) {
    for widget in &self.widgets {
        if let Some(key) = model.id_to_key.get(&widget.element_id) {
            if let Some(element) = model.elements.get(*key) {
                // render
            }
        }
    }
}
```

For diagrams with hundreds of widgets (each rendered at ~60fps), this
indirection adds measurable overhead.

### What about slotmap? It's already in the dep tree.

The workspace Cargo.toml lists `slotmap` as a dependency, and `id.rs` defines
`pub type ObjectKey = slotmap::DefaultKey`. This was an early architectural
decision captured in `domain_model_v1.md`.

**The recommendation is to remove `slotmap` from `uml-core`'s dependencies**
and replace it with `indexmap`. The `ObjectKey` type can be deprecated or
removed. If slotmap is needed by other crates (e.g., `uml-diagram` for widget
storage internally), it can remain as a workspace dependency but not a direct
dependency of `uml-core`.

### Summary

| Factor | Why IndexMap wins |
|--------|------------------|
| **Lookup** | O(1) direct, single container. No double-lookup indirection. |
| **Iteration** | Deterministic insertion order. Testable, predictable output. |
| **Simplicity** | One storage map + one parent index. Trivial insert/remove. |
| **Diagram-friendly** | Widgets store UmlId, call `model.get(id)` directly. |
| **Undo/redo** | Clone works. Future command-based undo is storage-agnostic. |
| **Dependencies** | Lightweight, trusted, well-maintained. |
| **Migration** | Same API as HashMap. Minimal call-site changes. |

---

## 8. Proposed API

### 8.1 The `UmlModel` Struct

```rust
/// Central storage for all UML model elements.
///
/// Owns all elements by value. Packages reference elements via `UmlId` —
/// they do not own them. Uses `IndexMap` for deterministic insertion-order
/// iteration and O(1) lookup by ID.
///
/// # Ownership Model
///
/// The repository is the single source of truth for element ownership:
/// - Elements are stored by value in `elements: IndexMap<UmlId, ModelElement>`.
/// - Packages store only `Vec<UmlId>` references to their children.
/// - The `parent_index` maintains a reverse mapping for O(1) membership queries.
///
/// # Deterministic Iteration
///
/// Elements iterate in insertion order. This means:
/// - During XMI loading, elements are typically inserted in tree-walk order
///   (root packages first, then their children). Iteration follows this order.
/// - Tests can assert on iteration order without sorting.
/// - CLI output is reproducible between runs.
#[derive(Debug, Clone)]
pub struct UmlModel {
    /// All elements, keyed by UmlId. Insertion order is preserved.
    elements: IndexMap<UmlId, ModelElement>,
    /// Reverse index: element_id → set of package_ids that contain it.
    /// Maintained automatically by add_to_package / remove_from_package.
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

> **NOTE:** `Package::children` is `pub(crate)` — external code must use
> `UmlModel::add_to_package` / `remove_from_package` to modify containment.
> The `Package::add_child()` / `remove_child()` methods on Package itself
> will be removed or made `pub(crate)`.

### 8.2 Core Methods

```rust
impl UmlModel {
    /// Create a new, empty model repository.
    #[must_use]
    pub fn new() -> Self;

    /// Insert an element. The element's embedded `UmlId` is used as the key.
    ///
    /// Returns the element's `UmlId`. If an element with the same ID already
    /// exists, the old element is replaced and returned as `Some(old_element)`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut model = UmlModel::new();
    /// let elem = ModelElement::Package(Package::new("Root"));
    /// let id = elem.id();
    /// assert!(model.insert(elem).is_none());
    /// assert!(model.contains(id));
    /// ```
    pub fn insert(&mut self, element: ModelElement) -> Option<ModelElement>;

    /// Remove an element by ID.
    ///
    /// This performs cascading cleanup:
    /// 1. Removes the element from `parent_index`.
    /// 2. Removes the element's ID from every package's `children` list
    ///    (using `parent_index` to find all parent packages).
    /// 3. Removes the element from the elements map.
    ///
    /// Returns the element if it existed.
    pub fn remove(&mut self, id: UmlId) -> Option<ModelElement>;

    /// Get a reference to an element by ID.
    #[must_use]
    pub fn get(&self, id: UmlId) -> Option<&ModelElement>;

    /// Get a mutable reference to an element by ID.
    pub fn get_mut(&mut self, id: UmlId) -> Option<&mut ModelElement>;

    /// Iterate over all `(UmlId, &ModelElement)` pairs in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (UmlId, &ModelElement)>;

    /// Number of elements in the repository.
    #[must_use]
    pub fn len(&self) -> usize;

    /// Returns `true` if the repository contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool;

    /// Returns `true` if an element with the given ID exists.
    #[must_use]
    pub fn contains(&self, id: UmlId) -> bool;

    /// Remove all elements that do NOT match the predicate.
    ///
    /// Elements matching the predicate are kept. All others are removed,
    /// with their parent_index entries and package memberships cleaned up.
    pub fn retain(&mut self, predicate: impl FnMut(UmlId, &ModelElement) -> bool);

    /// Remove all elements and return the underlying storage iterator.
    ///
    /// Clears parent_index as well. All elements are moved out.
    pub fn drain(&mut self) -> impl Iterator<Item = (UmlId, ModelElement)>;
}
```

### 8.3 Package Membership Methods

```rust
impl UmlModel {
    /// Add a child element to a package.
    ///
    /// Updates both `Package::children` and the `parent_index`.
    ///
    /// # Errors
    ///
    /// Returns `ModelError::ElementNotFound` if either `package_id` or
    /// `child_id` does not exist in the model.
    /// Returns `ModelError::WouldCreateCycle` if adding `child_id` to
    /// `package_id` would create a cycle in the containment hierarchy
    /// (i.e., if `package_id` is already contained by `child_id`).
    pub fn add_to_package(&mut self, package_id: UmlId, child_id: UmlId)
        -> Result<(), ModelError>;

    /// Remove a child element from a package.
    ///
    /// Updates both `Package::children` and the `parent_index`.
    ///
    /// # Errors
    ///
    /// Returns `ModelError::ElementNotFound` if either `package_id` or
    /// `child_id` does not exist.
    /// Returns `ModelError::NotAChild` if `child_id` is not a child of
    /// the specified package.
    pub fn remove_from_package(&mut self, package_id: UmlId, child_id: UmlId)
        -> Result<(), ModelError>;

    /// Get the package IDs that contain the given element.
    ///
    /// Returns `None` if the element does not exist in the model.
    /// Returns `Some(&[])` if the element exists but has no parents.
    #[must_use]
    pub fn parents_of(&self, element_id: UmlId) -> Option<&[UmlId]>;
}
```

### 8.4 Reference Validation

```rust
impl UmlModel {
    /// Validate all inter-element references in the model.
    ///
    /// Checks that every `UmlId` reference in `Package::children`,
    /// `Attribute::type_id`, `Operation::return_type_id`, and
    /// `Parameter::type_id` points to an existing element in this model.
    ///
    /// Returns a list of dangling references. An empty list means the model
    /// is fully consistent.
    ///
    /// This method is O(n × m) where n = elements and m = references per
    /// element. Acceptable for model validation (not in hot path).
    #[must_use]
    pub fn validate_references(&self) -> Vec<ReferenceError>;
}
```

### 8.5 Error Types

```rust
/// Errors that can occur during model operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ModelError {
    /// Element with the given ID was not found.
    #[error("element not found: {0}")]
    ElementNotFound(UmlId),

    /// The target element is not a child of the specified package.
    #[error("element {child} is not a child of package {parent}")]
    NotAChild {
        parent: UmlId,
        child: UmlId,
    },

    /// Operation is not supported for this element type.
    #[error("operation not supported for element type")]
    UnsupportedOperation,

    /// Adding `child` to `parent` would create a containment cycle.
    #[error("adding {child} to {parent} would create a containment cycle")]
    WouldCreateCycle {
        parent: UmlId,
        child: UmlId,
    },
}

/// A dangling reference found during validation.
#[derive(Debug, Clone, PartialEq)]
pub struct ReferenceError {
    /// The ID of the element that contains the dangling reference.
    pub source_id: UmlId,
    /// The field or context where the dangling reference was found.
    pub field: ReferenceField,
    /// The dangling ID that does not resolve to any element.
    pub target_id: UmlId,
}

/// The specific field where a dangling reference was found.
#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceField {
    /// In a Package's children list.
    PackageChild,
    /// In an Attribute's type_id.
    AttributeType,
    /// In an Operation's return_type_id.
    OperationReturnType,
    /// In a Parameter's type_id.
    ParameterType,
    /// In an ElementBase's stereotype_id.
    Stereotype,
}
```

### 8.6 Complete Example

```rust
use uml_core::{UmlModel, ModelElement, Package, Class, UmlId};

// Create a model and populate it.
let mut model = UmlModel::new();

// Create elements.
let pkg = ModelElement::Package(Package::new("com.example"));
let pkg_id = pkg.id();
let cls = ModelElement::Class(Class::new("Person"));
let cls_id = cls.id();

// Insert elements. Returns None for new IDs, or Some(old_element) on
// duplicate.
assert!(model.insert(pkg).is_none());
assert!(model.insert(cls).is_none());

// Establish containment.
model.add_to_package(pkg_id, cls_id).unwrap();

// Query membership.
assert_eq!(model.parents_of(cls_id), Some(&[pkg_id][..]));

// Look up and mutate.
let cls = model.get_mut(cls_id).unwrap();
cls.set_name("Employee".into());

// Iterate (insertion order: Package first, then Class).
let names: Vec<String> = model.iter()
    .map(|(_, e)| e.name().to_string())
    .collect();
assert_eq!(names, vec!["com.example", "Employee"]);

// Validate.
assert!(model.validate_references().is_empty());

// Remove.
let removed = model.remove(cls_id).unwrap();
assert_eq!(removed.object_type(), ObjectType::Class);
assert!(!model.contains(cls_id));
```

---

## 9. Migration Path

### Phase 1: Implement `UmlModel` with IndexMap

1. Add `indexmap = "2"` to `uml-core/Cargo.toml`.
2. Replace the `ModelRepository` stub with the `UmlModel` implementation.
3. Remove `slotmap` from `uml-core/Cargo.toml`.
4. Remove the `ObjectKey` type alias from `id.rs` (or deprecate it).
5. Write exhaustive tests for all methods.

### Phase 2: Update domain_model_v1.md

The domain model document references `SlotMap<UmlId, ModelElement>` in the
`ModelRepository` section. Update this to reflect the IndexMap-based design.

### Phase 3: Future performance optimisation

If profiling reveals that iteration performance is a bottleneck (e.g., for
models with >50,000 elements during export), the internal storage can be
replaced with a SlotMap cache layer **without changing the public API**:

```rust
// Future optimisation — internal detail only
struct UmlModel {
    // Primary storage for direct UmlId lookup
    elements: IndexMap<UmlId, ModelElement>,
    // Optional: SlotMap cache for fast iter (transparent to callers)
    // slot_cache: Option<SlotMap<ObjectKey, UmlId>>,
    parent_index: HashMap<UmlId, Vec<UmlId>>,
}
```

Because the public API uses `UmlId` exclusively, the storage backend is an
implementation detail that can be swapped without affecting callers.

### Phase 4: Command-based undo

When command-based undo replaces clone-based snapshots, implement `UndoCommand`
traits that operate on `&mut UmlModel`. The IndexMap-based storage is
storage-agnostic — commands call the same `insert()`, `remove()`,
`add_to_package()` methods regardless of backend.

---

## 10. References

- [Domain Model v1](./domain_model_v1.md) — The Rust-native UML metamodel.
- [indexmap crate](https://docs.rs/indexmap/latest/indexmap/) — `IndexMap` documentation.
- [Crate boundary review](./crate_boundary_review.md) — Umbrello-RS workspace organisation.
- [slotmap crate](https://docs.rs/slotmap/latest/slotmap/) — documentation for the rejected alternative.
- `crates/uml-core/src/repository.rs` — Current stub implementation.
- `crates/uml-core/src/id.rs` — `UmlId` and `ObjectKey` definition.
- `crates/uml-core/src/elements.rs` — `ModelElement`, `Package`, `Class`, etc.
