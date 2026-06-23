//! Persistence layer for Umbrello-RS.
//!
//! Handles file I/O operations including format detection, compressed archive
//! support (`.xmi.tgz`, `.xmi.tar.bz2`), autosave, and the `StorageBackend`
//! abstraction for supporting multiple file formats.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::doc_markdown)]

pub mod storage;

/// Persistence-specific error type.
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Unsupported file format.
    #[error("Unsupported file format: {0}")]
    UnsupportedFormat(String),
}

/// Detected file format for a model file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Plain XMI file (`.xmi`).
    XmiPlain,
    /// Gzip-compressed tar XMI file (`.xmi.tgz`).
    XmiGzip,
    /// Bzip2-compressed tar XMI file (`.xmi.tar.bz2`).
    XmiBzip2,
    /// ArgoUML ZIP archive (`.zargo`).
    Zargo,
}
