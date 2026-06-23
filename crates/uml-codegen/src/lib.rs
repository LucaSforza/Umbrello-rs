//! Code generation framework for Umbrello-RS.
//!
//! Merged from `uml-codegen`, `uml-codegen-cpp`, `uml-codegen-java`,
//! `uml-codegen-python`, and `uml-codegen-rust` crates.

pub mod registry;
pub mod writer;

/// Programming languages supported for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProgrammingLanguage {
    /// Ada
    Ada,
    /// ActionScript
    ActionScript,
    /// C++
    Cpp,
    /// C#
    CSharp,
    /// D
    D,
    /// IDL
    Idl,
    /// Java
    Java,
    /// JavaScript
    JavaScript,
    /// Pascal
    Pascal,
    /// Perl
    Perl,
    /// PHP 4
    Php4,
    /// PHP 5
    Php5,
    /// Python
    Python,
    /// Ruby
    Ruby,
    /// Rust
    Rust,
    /// SQL
    Sql,
    /// MySQL
    MySql,
    /// PostgreSQL
    PostgreSql,
    /// Tcl
    Tcl,
    /// Vala
    Vala,
    /// XML Schema
    XmlSchema,
}

/// A code generator produces source files from UML model elements.
pub trait CodeGenerator: Send + Sync {
    /// The programming language this generator targets.
    fn language(&self) -> ProgrammingLanguage;
}

// Re-export language-specific generators
#[cfg(feature = "cpp")]
pub mod cpp;
#[cfg(feature = "java")]
pub mod java;
#[cfg(feature = "python")]
pub mod python;
#[cfg(feature = "rust")]
pub mod rust;
