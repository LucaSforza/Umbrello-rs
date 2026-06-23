//! Diagram rendering engine for Umbrello-RS.
//!
//! Handles GPU-accelerated rendering of diagram widgets and association lines.
//! Abstracted behind a `Renderer` trait to allow backend swapping (CPU/GPU).

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms, clippy::all, clippy::pedantic)]

/// Rendering canvas abstraction.
#[derive(Debug)]
pub struct Canvas;

/// Trait for widget renderers.
pub trait WidgetRenderer {
    /// Draw the widget.
    fn draw(&self, _canvas: &mut Canvas) {}
}
