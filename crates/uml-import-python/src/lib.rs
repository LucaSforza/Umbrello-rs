//! Python code import for Umbrello-RS.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// Python code importer.
#[derive(Debug, Default)]
pub struct PythonImporter;

impl PythonImporter {
    /// Create a new Python importer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
