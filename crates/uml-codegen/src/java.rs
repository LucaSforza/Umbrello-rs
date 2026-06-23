//! Java code generator (was `uml-codegen-java`).

/// Java code generator.
pub struct JavaGenerator;

impl JavaGenerator {
    /// Create a new Java code generator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for JavaGenerator {
    fn default() -> Self {
        Self::new()
    }
}
