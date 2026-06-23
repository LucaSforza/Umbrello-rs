//! Import registry.

#![allow(clippy::zero_sized_map_values)]

use std::collections::HashMap;

/// Registry mapping file extensions to language importers.
///
/// Importers are registered at startup (via `#[cfg(feature)]` blocks) and
/// looked up by file extension when import is requested.
#[derive(Debug, Default)]
pub struct ImportRegistry {
    /// Placeholder — concrete importers will be added when Phase 8 is implemented.
    _importers: HashMap<String, ()>,
}

impl ImportRegistry {
    /// Create a new, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _importers: HashMap::new(),
        }
    }
}
