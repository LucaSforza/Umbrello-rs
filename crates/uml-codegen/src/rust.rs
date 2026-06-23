//! Rust code generator (was `uml-codegen-rust`).

/// Rust code generator.
pub struct RustGenerator;

impl RustGenerator {
    /// Create a new Rust code generator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for RustGenerator {
    fn default() -> Self {
        Self::new()
    }
}
