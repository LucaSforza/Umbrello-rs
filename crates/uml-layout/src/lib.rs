//! Layout algorithms for Umbrello-RS diagrams.
//!
//! Provides geometric operations: grid snapping, widget alignment guides,
//! and graph-based auto-layout algorithms. Optional Graphviz integration
//! for advanced layout.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// Grid snapping helper.
#[derive(Debug)]
pub struct GridSnapper;

/// Alignment guide helper.
#[derive(Debug)]
pub struct AlignmentGuides;
