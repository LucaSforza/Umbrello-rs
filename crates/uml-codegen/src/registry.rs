//! Generator registry.

#![allow(clippy::zero_sized_map_values)]

use crate::ProgrammingLanguage;
use std::collections::HashMap;

/// Registry mapping programming languages to their code generators.
///
/// Generators are registered at startup (via `#[cfg(feature)]` blocks) and
/// looked up by language when code generation is requested.
#[derive(Debug, Default)]
pub struct GeneratorRegistry {
    /// Placeholder — concrete generators will be added when Phase 12 is implemented.
    _generators: HashMap<ProgrammingLanguage, ()>,
}

impl GeneratorRegistry {
    /// Create a new, empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            _generators: HashMap::new(),
        }
    }
}
