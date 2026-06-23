//! I/O operations for Umbrello-RS.
//!
//! Handles persistence (file load/save), code import, and diagram export.
//! Merged from `uml-persistence`, `uml-import*`, and `uml-export` crates.

pub mod storage;
pub mod xmi;

/// Supported file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// Plain XMI file (`.xmi`).
    XmiPlain,
    /// Gzip-compressed tar (`.xmi.tgz`).
    XmiGzip,
    /// Bzip2-compressed tar (`.xmi.tar.bz2`).
    XmiBzip2,
}
