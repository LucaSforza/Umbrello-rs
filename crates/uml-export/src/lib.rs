//! Diagram export for Umbrello-RS.
//!
//! Converts rendered diagrams to image files: SVG, PNG, and optionally PDF.
//! This is a leaf crate — it depends on `uml-render` for the actual drawing.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// Supported export formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Scalable Vector Graphics.
    Svg,
    /// Portable Network Graphics (with DPI).
    Png(u32),
    /// Portable Document Format.
    Pdf,
}

/// Error during export.
#[derive(Debug, thiserror::Error)]
pub enum ExportError {
    /// I/O error writing the output file.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Unsupported format requested.
    #[error("Unsupported format")]
    UnsupportedFormat,
}
