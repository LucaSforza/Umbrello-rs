//! Unique identifier types for UML model objects.

use serde::{Deserialize, Serialize};

/// Universally unique identifier for UML model objects.
///
/// Backed by a UUID v4 for global uniqueness. Supports conversion
/// to/from XMI string format for compatibility with C++ Umbrello files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UmlId(uuid::Uuid);

impl UmlId {
    /// Generate a new unique identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

impl Default for UmlId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UmlId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Arena key for objects stored in the ModelRepository.
///
/// This is a generational index — it encodes both the slot index and a generation
/// counter to detect use-after-free (stale access). It is cheap to copy and compare.
pub type ObjectKey = slotmap::DefaultKey;
