//! Generator registry.
use crate::ProgrammingLanguage;
use std::collections::HashMap;

/// Registry of available code generators.
#[derive(Debug, Default)]
pub struct GeneratorRegistry {
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
