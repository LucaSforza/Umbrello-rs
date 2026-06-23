//! Pure UML domain model for Umbrello-RS.
//!
//! This crate contains all UML model types, type enums, identity management,
//! and the arena-based model repository. It has no I/O, no GUI, and no persistence
//! knowledge — it is pure data and domain logic.
//!
//! # Crate structure
//!
//! - `types`  — Enumerations: ObjectType, AssociationType, DiagramType, Visibility, etc.
//! - `model`  — UML model structs (UmlClass, UmlAttribute, UmlPackage, etc.)
//! - `id`     — Unique identifier types (UmlId, ObjectKey)
//! - `repository` — Arena-based storage for UML model elements
//! - `event`   — Model change events for undo/observer systems

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]
#![allow(clippy::doc_markdown)]

pub mod event;
pub mod id;
pub mod model;
pub mod repository;
pub mod types;

// No re-exports yet — types will be added when implemented in Phase 1
