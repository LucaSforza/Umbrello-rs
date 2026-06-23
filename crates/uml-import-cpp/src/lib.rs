//! C++ code import for Umbrello-RS.
//!
//! Parses C++ source files using tree-sitter-cpp and maps the resulting
//! AST to UML model objects.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// C++ code importer.
#[derive(Debug, Default)]
pub struct CppImporter;

impl CppImporter {
    /// Create a new C++ importer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
