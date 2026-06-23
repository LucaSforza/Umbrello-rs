//! Code import framework for Umbrello-RS.
//!
//! Defines the `LanguageImporter` trait and `ImportRegistry` for language-specific
//! code importers. Each supported language is a separate crate implementing
//! `LanguageImporter`, eliminating the C++ factory switch-statement anti-pattern.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::doc_markdown)]

pub mod registry;

/// Error during code import.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Parse error in source file.
    #[error("Parse error: {0}")]
    Parse(String),
    /// Unsupported language.
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}
