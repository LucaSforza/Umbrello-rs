//! Code generation framework for Umbrello-RS.
//!
//! Defines the `CodeGenerator` trait and `GeneratorRegistry` for language
//! code generation plugins. Each supported language is a separate crate that
//! implements `CodeGenerator`. This eliminates the C++ factory switch-statement
//! anti-pattern.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::doc_markdown)]

pub mod registry;
pub mod writer;

/// Programming languages supported for code generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ProgrammingLanguage {
    /// Ada.
    Ada,
    /// ActionScript.
    ActionScript,
    /// C++.
    Cpp,
    /// C#.
    CSharp,
    /// D.
    D,
    /// CORBA IDL.
    Idl,
    /// Java.
    Java,
    /// JavaScript.
    JavaScript,
    /// Pascal.
    Pascal,
    /// Perl.
    Perl,
    /// PHP 4.
    Php4,
    /// PHP 5.
    Php5,
    /// Python.
    Python,
    /// Ruby.
    Ruby,
    /// Rust.
    Rust,
    /// SQL (generic).
    Sql,
    /// MySQL-specific SQL.
    MySql,
    /// PostgreSQL-specific SQL.
    PostgreSql,
    /// Tcl.
    Tcl,
    /// Vala.
    Vala,
    /// XML Schema.
    XmlSchema,
}
