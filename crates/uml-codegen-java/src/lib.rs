//! Java code generator for Umbrello-RS.
//!
//! Generates `.java` files from UML classifiers. Handles packages, interfaces,
//! generics, annotations, and associations.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// Java code generator.
#[derive(Debug, Default)]
pub struct JavaGenerator;

impl JavaGenerator {
    /// Create a new Java generator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
