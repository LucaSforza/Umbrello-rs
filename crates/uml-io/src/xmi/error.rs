//! XMI parsing error types.

/// Errors that can occur during XMI parsing.
#[derive(Debug, thiserror::Error)]
pub enum XmiParseError {
    /// XML parsing error from quick-xml.
    #[error("XML error: {0}")]
    Xml(#[from] quick_xml::Error),

    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A required attribute is missing from an element.
    #[error("missing required attribute '{attr}' on element '{element}'")]
    MissingAttribute {
        /// The XML element tag name.
        element: String,
        /// The missing attribute name.
        attr: String,
    },

    /// An unknown or unsupported XMI element was encountered.
    #[error("unknown XMI element: {0}")]
    UnknownElement(String),

    /// Invalid value for an attribute (e.g., unknown visibility string).
    #[error("invalid value '{value}' for attribute '{attr}' on element '{element}'")]
    InvalidAttribute {
        /// The XML element tag name.
        element: String,
        /// The invalid attribute name.
        attr: String,
        /// The invalid value.
        value: String,
    },

    /// A duplicate XMI ID was encountered.
    #[error("duplicate XMI id: {0}")]
    DuplicateId(String),

    /// An XMI ID reference could not be resolved.
    #[error("unresolved reference: {0}")]
    UnresolvedReference(String),
}
