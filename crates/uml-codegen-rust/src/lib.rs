//! Rust code generator for Umbrello-RS.
//!
//! Generates `.rs` files from UML classifiers. Handles structs, enums, traits,
//! impl blocks, generics, and module structure.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// Rust code generator.
#[derive(Debug, Default)]
pub struct RustGenerator;

impl RustGenerator {
    /// Create a new Rust generator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
