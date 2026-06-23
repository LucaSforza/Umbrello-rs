//! Common error types for Umbrello-RS.

/// Primary error type for Umbrello-RS.
///
/// All workspace crates should produce errors that either are or convert into
/// `UmbrelloError`. Individual crates may define more specific error enums that
/// implement `From<>` into this type.
#[derive(Debug, thiserror::Error)]
pub enum UmbrelloError {
    /// An I/O error occurred.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A parse error occurred.
    #[error("Parse error: {0}")]
    Parse(String),

    /// A validation error occurred.
    #[error("Validation error: {0}")]
    Validation(String),

    /// A resource was not found.
    #[error("Not found: {0}")]
    NotFound(String),

    /// An unsupported version was encountered.
    #[error("Unsupported version: {0}")]
    UnsupportedVersion(String),

    /// A duplicate identifier was detected.
    #[error("Duplicate ID: {0}")]
    DuplicateId(String),

    /// A generic/unexpected error.
    #[error("Internal error: {0}")]
    Internal(String),
}
