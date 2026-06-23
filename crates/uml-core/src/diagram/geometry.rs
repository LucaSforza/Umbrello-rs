//! Geometric primitives for diagram layout.
//!
//! Uses f64 for all coordinates. Origin is top-left.

use serde::{Deserialize, Serialize};

/// A 2D point in diagram coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point {
    /// The x-coordinate (horizontal, origin at left).
    pub x: f64,
    /// The y-coordinate (vertical, origin at top).
    pub y: f64,
}

impl Point {
    /// Create a new point.
    #[must_use]
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

/// A 2D size (width and height).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Size {
    /// The width.
    pub width: f64,
    /// The height.
    pub height: f64,
}

impl Size {
    /// Create a new size.
    #[must_use]
    pub fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }
}

/// A rectangle defined by origin and size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    /// Top-left corner of the rectangle.
    pub origin: Point,
    /// Width and height.
    pub size: Size,
}

impl Rect {
    /// Create a new rectangle.
    #[must_use]
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// The x-coordinate of the left edge.
    #[must_use]
    pub fn x(&self) -> f64 {
        self.origin.x
    }

    /// The y-coordinate of the top edge.
    #[must_use]
    pub fn y(&self) -> f64 {
        self.origin.y
    }

    /// The width.
    #[must_use]
    pub fn width(&self) -> f64 {
        self.size.width
    }

    /// The height.
    #[must_use]
    pub fn height(&self) -> f64 {
        self.size.height
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn point_creation() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(p.x, 10.0);
        assert_eq!(p.y, 20.0);
    }

    #[test]
    fn rect_creation() {
        let r = Rect::new(5.0, 10.0, 100.0, 50.0);
        assert_eq!(r.x(), 5.0);
        assert_eq!(r.y(), 10.0);
        assert_eq!(r.width(), 100.0);
        assert_eq!(r.height(), 50.0);
    }

    #[test]
    fn serde_roundtrip_point() {
        let p = Point::new(1.5, 2.5);
        let json = serde_json::to_string(&p).unwrap();
        let back: Point = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }

    #[test]
    fn serde_roundtrip_rect() {
        let r = Rect::new(10.0, 20.0, 100.0, 200.0);
        let json = serde_json::to_string(&r).unwrap();
        let back: Rect = serde_json::from_str(&json).unwrap();
        assert_eq!(r, back);
    }
}
