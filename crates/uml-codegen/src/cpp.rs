//! C++ code generator (was `uml-codegen-cpp`).

/// C++ code generator.
pub struct CppGenerator;

impl CppGenerator {
    /// Create a new C++ code generator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for CppGenerator {
    fn default() -> Self {
        Self::new()
    }
}
