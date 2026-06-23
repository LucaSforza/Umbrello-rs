//! C++ code generator for Umbrello-RS.
//!
//! Generates `.h` and `.cpp` files from UML classifiers. Handles classes,
//! structs, enums, namespaces, inheritance, attributes, operations, and
//! associations as member variables.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// C++ code generator.
#[derive(Debug, Default)]
pub struct CppGenerator;

impl CppGenerator {
    /// Create a new C++ generator.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}
