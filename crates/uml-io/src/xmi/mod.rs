//! XMI persistence for Umbrello-RS.
//!
//! Reads legacy Umbrello C++ XMI 1.2 files and populates `UmlModel`.
//! Uses a two-pass strategy: Pass 1 extracts structural elements,
//! Pass 2 resolves cross-references (stereotype IDs).

pub mod error;
pub mod reader;
pub mod writer;

pub use error::XmiParseError;
pub use reader::XmiReader;
pub use writer::XmiWriter;
