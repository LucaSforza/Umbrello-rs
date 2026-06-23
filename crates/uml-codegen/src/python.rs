//! Python code generator (was `uml-codegen-python`).

/// Python code generator.
pub struct PythonGenerator;

impl PythonGenerator {
    /// Create a new Python code generator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for PythonGenerator {
    fn default() -> Self {
        Self::new()
    }
}
