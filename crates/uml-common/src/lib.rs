//! Common utilities and types shared across all Umbrello-RS crates.
//!
//! This crate provides foundational infrastructure types (errors, logging, version
//! constants) with zero domain knowledge. It must never depend on any other workspace
//! crate.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

pub mod error;
pub mod version;

// Re-export commonly used items
pub use error::UmbrelloError;
pub use tracing;
