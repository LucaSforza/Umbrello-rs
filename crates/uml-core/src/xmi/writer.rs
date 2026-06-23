//! Streaming XMI writer.
//!
//! Stubbed for Milestone 1.

/// Writes UML model data to an XMI file.
#[derive(Debug)]
pub struct XmiWriter;

impl XmiWriter {
    /// Create a new XMI writer.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for XmiWriter {
    fn default() -> Self {
        Self::new()
    }
}
