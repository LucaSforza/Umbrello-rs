//! Java code import for Umbrello-RS.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// Java code importer.
#[derive(Debug, Default)]
pub struct JavaImporter;

impl JavaImporter {
    /// Create a new Java importer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
