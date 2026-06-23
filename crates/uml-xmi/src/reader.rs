//! Streaming XMI reader.
//!
//! Stubbed for Milestone 1.

/// Reads UML model data from an XMI file.
#[derive(Debug)]
pub struct XmiReader;

impl XmiReader {
    /// Create a new XMI reader.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for XmiReader {
    fn default() -> Self {
        Self::new()
    }
}
