//! Arena-based storage for UML model elements.
//!
//! The ModelRepository provides indexed storage using generational arenas,
//! replacing the C++ pattern of QObject parent-child ownership with safe,
//! ID-based access.

/// The model repository — central storage for all UML model elements.
///
/// Uses `slotmap` for generational-index-based storage, providing O(1) access
/// and automatic detection of stale (use-after-free) access attempts.
#[derive(Debug, Default)]
pub struct ModelRepository {
    // Storage will be added in subsequent phases.
    _placeholder: (),
}

impl ModelRepository {
    /// Create a new, empty model repository.
    #[must_use]
    pub fn new() -> Self {
        Self { _placeholder: () }
    }
}
