//! Unique identifier types for UML model objects.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

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

impl FromStr for UmlId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(uuid::Uuid::parse_str(s)?))
    }
}

/// Arena key for objects stored in the ModelRepository.
///
/// This is a generational index — it encodes both the slot index and a generation
/// counter to detect use-after-free (stale access). It is cheap to copy and compare.
pub type ObjectKey = slotmap::DefaultKey;

// ─── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_new_produces_unique_ids() {
        let count = 100;
        let ids: HashSet<_> = (0..count).map(|_| UmlId::new()).collect();
        assert_eq!(ids.len(), count, "UmlId::new() produced duplicates");
    }

    #[test]
    fn test_default_works() {
        let id = UmlId::default();
        let s = id.to_string();
        assert!(!s.is_empty(), "default UmlId should produce non-empty Display");
    }

    #[test]
    fn test_display_valid_uuid() {
        let id = UmlId::new();
        let s = id.to_string();
        // Parse the UUID string back — should succeed
        let parsed = uuid::Uuid::parse_str(&s);
        assert!(parsed.is_ok(), "Display output should be a valid UUID string");
    }

    #[test]
    fn test_serde_roundtrip() {
        let id = UmlId::new();
        let json = serde_json::to_string(&id).expect("serialize");
        let back: UmlId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(id, back, "serde round-trip failed");
    }

    #[test]
    fn test_clone_preserves_value() {
        let id = UmlId::new();
        let cloned = id;
        assert_eq!(id, cloned);
        // Verify they have the same string representation
        assert_eq!(id.to_string(), cloned.to_string());
    }

    #[test]
    fn test_from_str_valid() {
        let id = UmlId::new();
        let s = id.to_string();
        let parsed: UmlId = s.parse().expect("should parse valid UUID string");
        assert_eq!(id, parsed);
    }

    #[test]
    fn test_from_str_invalid() {
        let result = UmlId::from_str("not-a-uuid");
        assert!(result.is_err(), "should reject invalid UUID string");
    }

    #[test]
    fn test_partial_eq_and_hash() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let a = UmlId::new();
        let b = a; // same value

        // Same IDs must have same hash
        let mut ha1 = DefaultHasher::new();
        let mut ha2 = DefaultHasher::new();
        a.hash(&mut ha1);
        b.hash(&mut ha2);
        assert_eq!(ha1.finish(), ha2.finish(), "same IDs must have same hash");

        // Different IDs (with high probability) have different hashes
        let c = UmlId::new();
        let mut hc = DefaultHasher::new();
        c.hash(&mut hc);
        // Two consecutive new_v4() calls are statistically guaranteed to differ
        assert_ne!(a, c, "newly generated IDs should differ");
    }
}
