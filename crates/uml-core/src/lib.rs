//! Pure UML domain model for Umbrello-RS.
//!
//! This crate contains all UML model types, type enums, identity management,
//! and the arena-based model repository. It has no I/O, no GUI, and no persistence
//! knowledge — it is pure data and domain logic.
//!
//! # Crate structure
//!
//! - `id`     — Unique identifier types (`UmlId`)
//! - `types`  — Enumerations: `ObjectType`, `AssociationType`, `DiagramType`, `Visibility`, etc.
//! - `elements` — UML model element types: `Package`, `Class`, `Interface`, `Enum`
//! - `model`  — UML model structs (future)
//! - `repository` — Central model storage using `IndexMap<UmlId, ModelElement>` with parent index
//! - `event`   — Model change events for undo/observer systems (future)

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::doc_markdown)]

pub mod elements;
pub mod event;
pub mod id;
pub mod model;
pub mod repository;
pub mod types;

// Re-exports for convenient access
pub use elements::{
    Attribute, Class, ClassifierData, ElementBase, Enum, EnumLiteral, Interface, ModelElement,
    NamedElement, Operation, Package, Parameter, Relationship, TemplateParameter, TypeReference,
};
pub use id::UmlId;
pub use repository::{ModelError, ReferenceError, ReferenceField, UmlModel};
pub use types::{AssociationType, DiagramType, ObjectType, ParameterDirection, Visibility};
