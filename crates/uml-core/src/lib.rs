//! Pure UML domain model for Umbrello-RS.
//!
//! This crate contains all UML model types, type enums, identity management,
//! the arena-based model repository, and sub-modules merged from sibling
//! crates (XMI serialisation, undo/redo, diagram model, layout, rendering).
//!
//! # Crate structure
//!
//! - `id`     — Unique identifier types (`UmlId`)
//! - `types`  — Enumerations: `ObjectType`, `AssociationType`, `DiagramType`, `Visibility`, etc.
//! - `elements` — UML model element types: `Package`, `Class`, `Interface`, `Enum`
//! - `model`  — UML model structs (future)
//! - `repository` — Central model storage using `IndexMap<UmlId, ModelElement>` with parent index
//! - `event`   — Model change events for undo/observer systems (future)
//! - `common`  — Shared utilities (merged from `uml-common`)
//! - `xmi`     — XMI serialisation (merged from `uml-xmi`)
//! - `undo`    — Undo/redo command system (merged from `uml-undo`)
//! - `diagram` — Diagram scene model (merged from `uml-diagram`)
//! - `layout`  — Layout algorithms (merged from `uml-layout`)
//! - `render`  — Rendering abstraction (merged from `uml-render`)

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::doc_markdown)]

pub mod common;
pub mod diagram;
pub mod elements;
pub mod event;
pub mod id;
pub mod layout;
pub mod model;
pub mod render;
pub mod repository;
pub mod types;
pub mod undo;
pub mod xmi;

// Re-exports for convenient access
pub use common::UmbrelloError;
pub use elements::{
    Attribute, Class, ClassifierData, Datatype, ElementBase, Enum, EnumLiteral, Interface,
    ModelElement, NamedElement, Operation, Package, Parameter, Relationship, TemplateParameter,
    TypeReference,
};
pub use id::UmlId;
pub use repository::{ModelError, ReferenceError, ReferenceField, UmlModel};
pub use types::{AssociationType, DiagramType, ObjectType, ParameterDirection, Visibility};
pub use undo::commands;
pub use undo::{Command, CommandError, History};
