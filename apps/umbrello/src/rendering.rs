//! Free rendering helper functions for Umbrello-RS.
//!
//! These functions handle element colors, visibility symbols and names,
//! type display, and arrowhead drawing. They are shared between canvas.rs,
//! tree.rs, and property_editor.rs.

use uml_core::{ModelElement, TypeReference, UmlModel, Visibility};

/// Returns the fill color for a given UML element type.
pub(crate) fn element_color(elem: Option<&ModelElement>) -> egui::Color32 {
    match elem {
        Some(ModelElement::Class(_)) => egui::Color32::from_rgb(180, 210, 255),
        Some(ModelElement::Interface(_)) => egui::Color32::from_rgb(180, 255, 210),
        Some(ModelElement::Enum(_)) => egui::Color32::from_rgb(255, 210, 180),
        Some(ModelElement::Datatype(_)) => egui::Color32::from_rgb(210, 180, 255),
        Some(ModelElement::Package(_)) => egui::Color32::from_rgb(255, 255, 200),
        // ── M20: Actor & UseCase ──
        Some(ModelElement::Actor(_)) => egui::Color32::from_rgb(255, 200, 170), // light orange/salmon
        Some(ModelElement::UseCase(_)) => egui::Color32::from_rgb(255, 180, 180), // light coral/pink
        _ => egui::Color32::from_rgb(220, 220, 220),
    }
}

/// Returns the UML visibility symbol for a given visibility level.
pub(crate) fn visibility_symbol(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "+",
        Visibility::Protected => "#",
        Visibility::Private => "-",
        Visibility::Implementation => "~",
    }
}

/// Returns the human-readable name for a visibility level.
pub(crate) fn visibility_name(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "Public",
        Visibility::Protected => "Protected",
        Visibility::Private => "Private",
        Visibility::Implementation => "Impl",
    }
}

/// Formats a `TypeReference` for display, resolving model references if possible.
///
/// REVIEW CONDITION C5: Takes model reference for display_name resolution.
pub(crate) fn type_display(type_ref: &TypeReference, model: Option<&UmlModel>) -> String {
    type_ref.display_name(model)
}

/// Draw a hollow triangular arrowhead at `tip` pointing in direction `dir`.
pub(crate) fn draw_hollow_triangle(
    painter: &egui::Painter,
    tip: egui::Pos2,
    dir: egui::Vec2,
    perp: egui::Vec2,
    color: egui::Color32,
) {
    let arrow_len = 14.0;
    let half_width = 7.0;
    let base_center = tip - dir * arrow_len;
    let base_left = base_center + perp * half_width;
    let base_right = base_center - perp * half_width;
    let stroke = egui::Stroke::new(1.5, color);
    painter.line_segment([tip, base_left], stroke);
    painter.line_segment([tip, base_right], stroke);
    painter.line_segment([base_left, base_right], stroke);
}

/// Draw a hollow diamond at `center`.
pub(crate) fn draw_hollow_diamond(
    painter: &egui::Painter,
    center: egui::Pos2,
    dir: egui::Vec2,
    perp: egui::Vec2,
    color: egui::Color32,
) {
    let half = 8.0;
    let front = center + dir * half;
    let back = center - dir * half;
    let left = center + perp * half;
    let right = center - perp * half;
    let stroke = egui::Stroke::new(1.5, color);
    painter.line_segment([front, left], stroke);
    painter.line_segment([left, back], stroke);
    painter.line_segment([back, right], stroke);
    painter.line_segment([right, front], stroke);
}

/// Draw a filled diamond at `center`.
pub(crate) fn draw_filled_diamond(
    painter: &egui::Painter,
    center: egui::Pos2,
    dir: egui::Vec2,
    perp: egui::Vec2,
    color: egui::Color32,
) {
    let half = 8.0;
    let front = center + dir * half;
    let back = center - dir * half;
    let left = center + perp * half;
    let right = center - perp * half;
    painter.add(egui::Shape::convex_polygon(
        vec![front, left, back, right],
        color,
        egui::Stroke::new(1.0, color),
    ));
}

/// Draw an open arrow (two lines forming a V at the tip, no base).
pub(crate) fn draw_open_arrow(
    painter: &egui::Painter,
    tip: egui::Pos2,
    dir: egui::Vec2,
    perp: egui::Vec2,
    color: egui::Color32,
) {
    let arrow_len = 10.0;
    let half_width = 5.0;
    let left = tip - dir * arrow_len + perp * half_width;
    let right = tip - dir * arrow_len - perp * half_width;
    let stroke = egui::Stroke::new(1.0, color);
    painter.line_segment([tip, left], stroke);
    painter.line_segment([tip, right], stroke);
}

/// Draw a dashed line from start to end.
pub(crate) fn draw_dashed_line(
    painter: &egui::Painter,
    start: egui::Pos2,
    end: egui::Pos2,
    stroke: egui::Stroke,
) {
    let dir = end - start;
    let len = dir.length();
    if len < 1.0 {
        return;
    }
    let unit = dir / len;
    let dash = 6.0;
    let gap = 3.0;
    let mut pos = 0.0;
    while pos < len {
        let seg_end = (pos + dash).min(len);
        painter.line_segment([start + unit * pos, start + unit * seg_end], stroke);
        pos += dash + gap;
    }
}
