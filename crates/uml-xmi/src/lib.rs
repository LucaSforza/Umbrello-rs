//! XMI serialization for Umbrello-RS.
//!
//! Provides streaming XML reading and writing for UML model files in both
//! XMI 1.2 (legacy) and XMI 2.1 formats. Uses `quick-xml` for event-based
//! parsing — no DOM tree is constructed.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

pub mod reader;
pub mod writer;

/// XMI version selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XmiVersion {
    /// XMI 1.2 — uses `xmi.id`, `UML:` namespace prefix.
    V1_2,
    /// XMI 2.1 — uses `xmi:id`, `uml:` namespace prefix, `<packagedElement>`.
    V2_1,
}
