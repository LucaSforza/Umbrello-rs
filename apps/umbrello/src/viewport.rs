//! Pure viewport coordinate conversion and sizing helpers.

use egui::{Pos2, Rect, Vec2};
use uml_core::{Point, Rect as ModelRect};

/// Affine transform used by the diagram canvas.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ViewportTransform {
    pub(crate) origin: Pos2,
    pub(crate) pan: Vec2,
    pub(crate) scale: f64,
}

impl crate::app::UmbrelloApp {
    pub(crate) fn adjust_zoom(&mut self, delta: f64) {
        let Some(index) = self.active_diagram else {
            return;
        };
        if let Some(diagram) = self.model.diagrams().get(index) {
            let id = diagram.id;
            let zoom = diagram.zoom_percent();
            if let Some(diagram) = self.model.get_diagram_mut(id) {
                diagram.set_zoom_percent(zoom + delta);
            }
        }
    }

    pub(crate) fn reset_viewport(&mut self) {
        let Some(index) = self.active_diagram else {
            return;
        };
        if let Some(diagram) = self.model.diagrams().get(index) {
            let id = diagram.id;
            if let Some(diagram) = self.model.get_diagram_mut(id) {
                diagram.set_zoom_percent(100.0);
            }
            self.viewport_pans.insert(id, Vec2::ZERO);
        }
    }

    pub(crate) fn fit_active_diagram(&mut self, canvas: Rect) {
        let Some(index) = self.active_diagram else {
            return;
        };
        let Some(diagram) = self.model.diagrams().get(index) else {
            return;
        };
        let id = diagram.id;
        let mut bounds: Option<ModelRect> = None;
        for node in diagram.nodes.values().filter(|node| node.visible) {
            let r = node.bounds;
            bounds = Some(match bounds {
                None => r,
                Some(b) => ModelRect::new(
                    b.x().min(r.x()),
                    b.y().min(r.y()),
                    (b.x() + b.width()).max(r.x() + r.width()) - b.x().min(r.x()),
                    (b.y() + b.height()).max(r.y() + r.height()) - b.y().min(r.y()),
                ),
            });
        }
        let Some(bounds) = bounds else {
            self.reset_viewport();
            return;
        };
        let margin = 32.0_f32;
        let width = (canvas.width() - 2.0 * margin).max(1.0);
        let height = (canvas.height() - 2.0 * margin).max(1.0);
        let scale = (f64::from(width) / bounds.width()).min(f64::from(height) / bounds.height());
        let zoom = (scale * 100.0).clamp(10.0, 500.0);
        if let Some(diagram) = self.model.get_diagram_mut(id) {
            diagram.set_zoom_percent(zoom);
        }
        let transform = crate::app::viewport::ViewportTransform::new(canvas.min, Vec2::ZERO, zoom);
        let center = transform.model_to_screen(Point::new(
            bounds.x() + bounds.width() / 2.0,
            bounds.y() + bounds.height() / 2.0,
        ));
        self.viewport_pans.insert(id, canvas.center() - center);
    }

    pub(crate) fn zoom_at(&mut self, canvas: Rect, cursor: Pos2, factor: f64) {
        let Some(index) = self.active_diagram else {
            return;
        };
        let Some(diagram) = self.model.diagrams().get(index) else {
            return;
        };
        let id = diagram.id;
        let old = diagram.zoom_percent();
        let transform = crate::app::viewport::ViewportTransform::new(
            canvas.min,
            self.viewport_pans.get(&id).copied().unwrap_or_default(),
            old,
        );
        let model_point = transform.screen_to_model(cursor);
        let new_zoom = (old * factor).clamp(10.0, 500.0);
        if let Some(diagram) = self.model.get_diagram_mut(id) {
            diagram.set_zoom_percent(new_zoom);
        }
        let new_scale = new_zoom / 100.0;
        self.viewport_pans.insert(
            id,
            cursor
                - canvas.min
                - egui::vec2(
                    (model_point.x * new_scale) as f32,
                    (model_point.y * new_scale) as f32,
                ),
        );
    }
}

impl ViewportTransform {
    pub(crate) fn new(origin: Pos2, pan: Vec2, zoom_percent: f64) -> Self {
        Self {
            origin,
            pan,
            scale: zoom_percent / 100.0,
        }
    }

    pub(crate) fn model_to_screen(self, point: Point) -> Pos2 {
        self.origin
            + self.pan
            + egui::vec2((point.x * self.scale) as f32, (point.y * self.scale) as f32)
    }

    pub(crate) fn screen_to_model(self, point: Pos2) -> Point {
        let delta = point - self.origin - self.pan;
        Point::new(f64::from(delta.x) / self.scale, f64::from(delta.y) / self.scale)
    }

    pub(crate) fn model_rect_to_screen(self, rect: ModelRect) -> Rect {
        let min = self.model_to_screen(Point::new(rect.x(), rect.y()));
        Rect::from_min_size(
            min,
            Vec2::new((rect.width() * self.scale) as f32, (rect.height() * self.scale) as f32),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_round_trips() {
        let transform = ViewportTransform::new(Pos2::new(20.0, 30.0), Vec2::new(7.0, -4.0), 200.0);
        let point = Point::new(-12.5, 8.0);
        let screen = transform.model_to_screen(point);
        let restored = transform.screen_to_model(screen);
        assert!((restored.x - point.x).abs() < 1e-5);
        assert!((restored.y - point.y).abs() < 1e-5);
    }
}
