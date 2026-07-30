//! Canvas rendering — partitioned UML node boxes, edge drawing with arrowheads,
//! node drag-and-drop, and ghost preview for creation tools.

use crate::app::UmbrelloApp;
use crate::rendering::{
    draw_dashed_line, draw_filled_diamond, draw_hollow_diamond, draw_hollow_triangle,
    draw_open_arrow, element_color, type_display, visibility_symbol,
};
use crate::tool_palette::ToolMode;
use uml_core::{ArtifactDrawMode, AssociationType, Diagram, ModelElement, Point, ViewNode};

#[derive(Debug, Clone)]
pub(crate) struct ScreenEdgePath {
    pub(crate) relationship_id: uml_core::UmlId,
    pub(crate) points: Vec<egui::Pos2>,
    pub(crate) kind: AssociationType,
}

pub(crate) fn point_segment_distance_sq(
    point: egui::Pos2,
    start: egui::Pos2,
    end: egui::Pos2,
) -> f32 {
    let segment = end - start;
    let length_sq = segment.length_sq();
    if length_sq <= f32::EPSILON {
        return point.distance_sq(start);
    }
    let t = ((point - start).dot(segment) / length_sq).clamp(0.0, 1.0);
    point.distance_sq(start + segment * t)
}

pub(crate) fn preview_node_position(
    original: Point,
    screen_delta: egui::Vec2,
    scale: f64,
) -> Point {
    Point::new(
        original.x + f64::from(screen_delta.x) / scale,
        original.y + f64::from(screen_delta.y) / scale,
    )
}

pub(crate) fn no_diagram_guidance(has_project: bool) -> (&'static str, &'static str) {
    if !has_project {
        ("No XMI project is open.", "Use File > New Project or File > Open to begin.")
    } else {
        (
            "No diagram exists in this project.",
            "Use 'New Diagram…' to create a Class, Use Case, Component, or Deployment diagram.",
        )
    }
}

pub(crate) fn nearest_edge_relationship(
    paths: &[ScreenEdgePath],
    point: egui::Pos2,
    tolerance: f32,
) -> Option<uml_core::UmlId> {
    let limit = tolerance * tolerance;
    let mut nearest = None;
    for path in paths {
        let distance = path
            .points
            .windows(2)
            .map(|segment| point_segment_distance_sq(point, segment[0], segment[1]))
            .fold(f32::INFINITY, f32::min);
        if distance <= limit && nearest.is_none_or(|(best, _)| distance < best) {
            nearest = Some((distance, path.relationship_id));
        }
    }
    nearest.map(|(_, relationship_id)| relationship_id)
}

impl UmbrelloApp {
    /// Render the main diagram canvas with all nodes and edges.
    pub(crate) fn render_canvas(&mut self, ui: &mut egui::Ui) {
        // ── Crosshair cursor for creation tools ──────────────────────
        if self.current_tool.is_creation_tool() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }

        let Some(diag_idx) = self.active_diagram else {
            ui.centered_and_justified(|ui| {
                ui.heading("No diagram selected");
                ui.add_space(12.0);
                if self.model.diagrams().is_empty() {
                    if self.model.is_empty() {
                        let (heading, detail) =
                            no_diagram_guidance(self.current_file_path.is_some());
                        ui.label(heading);
                        ui.label(detail);
                    } else {
                        let (heading, detail) =
                            no_diagram_guidance(self.current_file_path.is_some());
                        ui.label(format!(
                            "{} Model has {} elements but no diagrams.",
                            heading,
                            self.model.len()
                        ));
                        ui.add_space(8.0);
                        ui.label(format!("→ {detail}"));
                    }
                } else {
                    ui.label("→ Select a diagram from the left panel to view it.");
                }
            });
            return;
        };

        let diagram = self.model.diagrams()[diag_idx].clone();
        let diagram_id = diagram.id;
        let canvas_rect = ui.max_rect();
        self.last_canvas_rect = Some(canvas_rect);
        let Some(transform) = self.viewport_transform(canvas_rect.min) else {
            return;
        };
        if ui.rect_contains_pointer(canvas_rect) {
            let scroll = ui.input(|i| i.raw_scroll_delta.y);
            if scroll.abs() > f32::EPSILON {
                self.zoom_at(
                    canvas_rect,
                    ui.ctx()
                        .pointer_latest_pos()
                        .unwrap_or(canvas_rect.center()),
                    if scroll > 0.0 { 1.15 } else { 1.0 / 1.15 },
                );
            }
            if ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle)) {
                let delta = ui.input(|i| i.pointer.delta());
                if delta != egui::Vec2::ZERO {
                    let pan = self.viewport_pans.entry(diagram_id).or_default();
                    *pan += delta;
                }
            }
        }

        // Background
        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, egui::Color32::from_gray(245));

        // ── Draw edges first (behind nodes) ──────────────────────────
        self.draw_edges(&diagram, ui);

        // ── Rubber-band preview during edge drag ────────────────────
        if let Some(source_id) = self.drag_source_node_id {
            if self.current_tool.is_edge_tool() {
                if let Some(source_node) = diagram.get_node(source_id) {
                    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                        let src_center = transform.model_to_screen(Point::new(
                            source_node.bounds.x() + source_node.bounds.width() / 2.0,
                            source_node.bounds.y() + source_node.bounds.height() / 2.0,
                        ));
                        let cursor = pointer_pos;
                        let dir = cursor - src_center;
                        let len = dir.length();
                        if len > 1.0 {
                            let unit = dir / len;
                            let perp = egui::vec2(-unit.y, unit.x);
                            let preview_color =
                                egui::Color32::from_rgba_premultiplied(100, 100, 100, 120);
                            let painter = ui.painter();

                            match self.current_tool {
                                ToolMode::CreateGeneralization => {
                                    painter.line_segment(
                                        [src_center, cursor],
                                        egui::Stroke::new(1.5, preview_color),
                                    );
                                    draw_hollow_triangle(
                                        painter,
                                        cursor,
                                        unit,
                                        perp,
                                        preview_color,
                                    );
                                },
                                ToolMode::CreateRealization => {
                                    draw_dashed_line(
                                        painter,
                                        src_center,
                                        cursor,
                                        egui::Stroke::new(1.5, preview_color),
                                    );
                                    draw_hollow_triangle(
                                        painter,
                                        cursor,
                                        unit,
                                        perp,
                                        preview_color,
                                    );
                                },
                                ToolMode::CreateAssociation => {
                                    painter.line_segment(
                                        [src_center, cursor],
                                        egui::Stroke::new(1.0, preview_color),
                                    );
                                },
                                ToolMode::CreateAggregation => {
                                    painter.line_segment(
                                        [src_center, cursor],
                                        egui::Stroke::new(1.5, preview_color),
                                    );
                                    draw_hollow_diamond(
                                        painter,
                                        src_center,
                                        unit,
                                        perp,
                                        preview_color,
                                    );
                                },
                                ToolMode::CreateComposition => {
                                    painter.line_segment(
                                        [src_center, cursor],
                                        egui::Stroke::new(1.5, preview_color),
                                    );
                                    draw_filled_diamond(
                                        painter,
                                        src_center,
                                        unit,
                                        perp,
                                        preview_color,
                                    );
                                },
                                ToolMode::CreateDependency => {
                                    draw_dashed_line(
                                        painter,
                                        src_center,
                                        cursor,
                                        egui::Stroke::new(1.0, preview_color),
                                    );
                                    draw_open_arrow(painter, cursor, unit, perp, preview_color);
                                },
                                _ => {},
                            }
                        }
                    }
                }
            }
        }

        // ── Draw nodes ───────────────────────────────────────────────
        let mut node_rects: Vec<(uml_core::UmlId, egui::Rect, f64, f64)> = Vec::new();

        for (&_node_id, node) in &diagram.nodes {
            if !node.visible {
                continue;
            }

            let display_bounds = if self.drag_node_id == Some(node.model_element_id) {
                self.drag_preview_pos.map_or(node.bounds, |position| {
                    uml_core::Rect::new(
                        position.x,
                        position.y,
                        node.bounds.width(),
                        node.bounds.height(),
                    )
                })
            } else {
                node.bounds
            };
            let rect = transform.model_rect_to_screen(display_bounds);

            // Draw the partitioned node
            if display_bounds == node.bounds {
                self.draw_partitioned_node(ui, node, rect);
            } else {
                let mut preview_node = node.clone();
                preview_node.bounds = display_bounds;
                self.draw_partitioned_node(ui, &preview_node, rect);
            }

            node_rects.push((node.model_element_id, rect, node.bounds.x(), node.bounds.y()));
        }

        // ── Handle interactions ──
        let mut selection_handled = false;
        let ptr_down = ui.input(|i| i.pointer.button_down(egui::PointerButton::Primary));
        let ptr_clicked = ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Primary));
        let ptr_released = ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
        // Use press_origin for hit-testing during drag (latest_pos may be
        // outside the node rect at non-100% zoom after pointer movement).
        let drag_hit = self.drag_node_id.or_else(|| {
            ui.input(|i| i.pointer.press_origin()).and_then(|origin| {
                node_rects
                    .iter()
                    .find(|(_, rect, _, _)| rect.contains(origin))
                    .map(|(id, _, _, _)| *id)
            })
        });

        if self.current_tool == ToolMode::Select {
            // ── Select mode: click-to-select + drag-to-move ──
            //
            // Uses self-contained hit testing rather than relying solely on
            // egui's Response::dragged / drag_stopped flags, because those
            // flags depend on multi-frame interaction snapshots that are
            // unreliable in test contexts (isolated ctx.run() calls).
            for &(model_element_id, rect, orig_x, orig_y) in &node_rects {
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());
                let hit = drag_hit == Some(model_element_id);
                let is_being_dragged = self.drag_node_id == Some(model_element_id);

                // Click to select
                if response.clicked() {
                    let name = self
                        .model
                        .get(model_element_id)
                        .map(|e| e.name().to_string())
                        .unwrap_or_default();
                    let _ = self.select_element(model_element_id);
                    self.status_message = format!("Selected: {}", name);
                    selection_handled = true;
                }

                // Drag update (while button is down, pointer has moved)
                if hit && ptr_down {
                    selection_handled = true;
                    if self.drag_node_id != Some(model_element_id) {
                        self.drag_node_id = Some(model_element_id);
                        self.drag_start_pos = Some(egui::pos2(orig_x as f32, orig_y as f32));
                    }
                    let screen_delta = ui.input(|i| i.pointer.delta());
                    if screen_delta != egui::Vec2::ZERO {
                        let new_pos = preview_node_position(
                            Point::new(orig_x, orig_y),
                            screen_delta,
                            transform.scale,
                        );
                        self.drag_preview_pos = Some(new_pos);
                    }
                }

                // Drag commit (button was just released while this node was
                // the drag target, or has a pending preview).
                if ptr_released && is_being_dragged {
                    if self.drag_preview_pos.is_none() {
                        let screen_delta = ui.input(|i| i.pointer.delta());
                        self.drag_preview_pos = Some(preview_node_position(
                            Point::new(orig_x, orig_y),
                            screen_delta,
                            transform.scale,
                        ));
                    }
                    if let Some(position) = self.drag_preview_pos.take() {
                        let _ = self.move_node_to(diagram_id, model_element_id, position);
                    }
                    self.drag_node_id = None;
                    self.drag_start_pos = None;
                }
            }
            if ptr_clicked && !selection_handled {
                if let Some(pointer) = ui.ctx().pointer_latest_pos() {
                    let paths = self.screen_edge_paths(&diagram, canvas_rect.min);
                    let valid_paths: Vec<_> = paths
                        .into_iter()
                        .filter(|path| {
                            matches!(
                                self.model.get(path.relationship_id),
                                Some(ModelElement::Relationship(_))
                            )
                        })
                        .collect();
                    if let Some(relationship_id) =
                        nearest_edge_relationship(&valid_paths, pointer, 6.0)
                    {
                        if matches!(
                            self.model.get(relationship_id),
                            Some(ModelElement::Relationship(_))
                        ) {
                            let _ = self.select_element(relationship_id);
                            self.status_message = "Selected relationship".into();
                            selection_handled = true;
                        }
                    }
                }
            }
        } else if self.current_tool.is_edge_tool() {
            // ── Edge tool: drag from source node ──
            for &(model_element_id, rect, _, _) in &node_rects {
                let response = ui.allocate_rect(rect, egui::Sense::drag());
                if response.dragged() && self.drag_source_node_id.is_none() {
                    self.drag_source_node_id = Some(model_element_id);
                    ui.ctx().request_repaint();
                }
            }
        } else {
            // ── Creation tool: no node interaction ──
            // Node creation is handled by the background click below.
        }

        // ── Continuous repaint requests ──
        if self.drag_node_id.is_some() {
            ui.ctx().request_repaint();
        }
        if self.drag_source_node_id.is_some() && self.current_tool.is_edge_tool() {
            ui.ctx().request_repaint();
        }

        // ── Edge drag: detect release on target node ────────
        if self.drag_source_node_id.is_some() && self.current_tool.is_edge_tool() {
            let released = ui.input(|i| i.pointer.button_released(egui::PointerButton::Primary));
            if released {
                let source_id = self.drag_source_node_id.take().unwrap();
                if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                    let mut found_target = false;
                    for &(target_id, target_rect, _, _) in &node_rects {
                        if target_rect.contains(pointer_pos) && target_id != source_id {
                            if let Err(e) = self.place_edge(source_id, target_id) {
                                self.status_message = format!("Error: {e}");
                            } else {
                                self.status_message = "Edge created — tool reset to Select".into();
                            }
                            self.current_tool = ToolMode::Select;
                            found_target = true;
                            break;
                        }
                    }
                    if !found_target {
                        self.status_message = "Edge creation cancelled".into();
                    }
                }
                ui.ctx().request_repaint();
            }
        }

        // ── Background click for creation tools ─────────────────────
        if self.current_tool.is_creation_tool() {
            if self.active_diagram.is_some() {
                let bg_rect = ui.max_rect();
                let bg_response = ui.interact(bg_rect, ui.next_auto_id(), egui::Sense::click());

                // Hover preview
                if bg_response.hovered() {
                    if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                        self.preview_position = Some(transform.screen_to_model(pointer_pos));
                    }
                } else {
                    self.preview_position = None;
                }

                // Click to create
                if bg_response.clicked() {
                    if let Some(click_pos) = bg_response.interact_pointer_pos() {
                        let pos = transform.screen_to_model(click_pos);
                        if let Err(e) = self.place_element(self.current_tool, pos) {
                            self.status_message = format!("Error: {e}");
                        }
                        // Reset tool to Select after creation
                        self.current_tool = ToolMode::Select;
                        self.preview_position = None;
                    }
                }
            } else {
                // No active diagram — show message on click attempt
                let bg_response =
                    ui.interact(ui.max_rect(), ui.next_auto_id(), egui::Sense::click());
                if bg_response.clicked() {
                    self.status_message = "No active diagram. Create a diagram first.".into();
                }
            }
        }

        // ── Background click to deselect (only in Select mode) ──────
        // NOTE: This does NOT allocate an overlapping Sense::click interact because
        // doing so steals pointer ownership from the node click_and_drag widgets,
        // preventing native node drag from working (S3/S4 regression).
        // Instead, we inspect the global click state and verify the click was not
        // on any node rect.
        if self.current_tool == ToolMode::Select
            && self.selected_element_id.is_some()
            && !selection_handled
            && ui.input(|input| input.pointer.button_clicked(egui::PointerButton::Primary))
        {
            let on_a_node = ui
                .input(|input| input.pointer.press_origin())
                .or_else(|| ui.ctx().pointer_latest_pos())
                .is_some_and(|pointer_pos| {
                    node_rects
                        .iter()
                        .any(|(_, rect, _, _)| rect.contains(pointer_pos))
                });
            if !on_a_node {
                self.clear_selection();
                self.status_message = "Selection cleared".into();
            }
        }

        // ── Ghost preview rectangle ─────────────────────────────────
        if let Some(preview_pos) = self.preview_position {
            let preview_rect = transform.model_rect_to_screen(uml_core::Rect::new(
                preview_pos.x - 80.0,
                preview_pos.y - 30.0,
                160.0,
                60.0,
            ));
            ui.painter().rect_filled(
                preview_rect,
                4.0,
                egui::Color32::from_rgba_premultiplied(100, 100, 255, 40),
            );
            ui.painter().rect_stroke(
                preview_rect,
                4.0,
                egui::Stroke::new(1.0, egui::Color32::from_rgba_premultiplied(100, 100, 255, 120)),
                egui::StrokeKind::Inside,
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Partitioned node drawing
    // ═══════════════════════════════════════════════════════════════════

    pub(crate) fn draw_partitioned_node(
        &self,
        ui: &egui::Ui,
        node: &ViewNode,
        full_rect: egui::Rect,
    ) {
        let painter = ui.painter();
        let font_id = egui::FontId::proportional(12.0);
        let name_font = egui::FontId::proportional(13.0);
        let small_font = egui::FontId::proportional(10.0);
        let elem = self.model.get(node.model_element_id);

        let mut y = full_rect.top() + 4.0;
        let left = full_rect.left() + 6.0;
        let right = full_rect.right() - 6.0;

        // Background fill
        let fill = element_color(elem);
        painter.rect_filled(full_rect, 4.0, fill);
        painter.rect_stroke(
            full_rect,
            4.0,
            egui::Stroke::new(1.5, egui::Color32::BLACK),
            egui::StrokeKind::Inside,
        );

        match elem {
            Some(ModelElement::Class(cls)) => {
                // Zone 0: Stereotype
                if cls.base.stereotype_id.is_some() {
                    let stereo_text = "<<stereotype>>".to_string();
                    painter.text(
                        egui::pos2(full_rect.center().x, y),
                        egui::Align2::CENTER_TOP,
                        stereo_text,
                        small_font.clone(),
                        egui::Color32::GRAY,
                    );
                    y += 16.0;
                }
                // Zone 1: Name (bold, centered)
                painter.text(
                    egui::pos2(full_rect.center().x, y),
                    egui::Align2::CENTER_TOP,
                    &cls.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
                y += 18.0;
                // Divider
                y += 2.0;
                painter.line_segment(
                    [egui::pos2(left, y), egui::pos2(right, y)],
                    egui::Stroke::new(1.0, egui::Color32::from_gray(150)),
                );
                y += 4.0;
                // Zone 2: Attributes
                for attr in &cls.classifier.attributes {
                    let vis = visibility_symbol(attr.visibility);
                    let type_name = type_display(&attr.type_ref, Some(&self.model));
                    let line = format!("{} {}: {}", vis, attr.name, type_name);
                    painter.text(
                        egui::pos2(left, y),
                        egui::Align2::LEFT_TOP,
                        line,
                        font_id.clone(),
                        egui::Color32::BLACK,
                    );
                    y += 15.0;
                }
                // Divider (only if there are operations below)
                if !cls.classifier.operations.is_empty() {
                    y += 2.0;
                    painter.line_segment(
                        [egui::pos2(left, y), egui::pos2(right, y)],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(150)),
                    );
                    y += 4.0;
                }
                // Zone 3: Operations
                for op in &cls.classifier.operations {
                    let vis = visibility_symbol(op.visibility);
                    let params: Vec<String> = op
                        .parameters
                        .iter()
                        .map(|p| {
                            format!("{}: {}", p.name, type_display(&p.type_ref, Some(&self.model)))
                        })
                        .collect();
                    let ret = type_display(&op.return_type, Some(&self.model));
                    let line = format!("{} {}({}): {}", vis, op.name, params.join(", "), ret);
                    painter.text(
                        egui::pos2(left, y),
                        egui::Align2::LEFT_TOP,
                        line,
                        font_id.clone(),
                        egui::Color32::BLACK,
                    );
                    y += 15.0;
                }
            },
            Some(ModelElement::Interface(iface)) => {
                painter.text(
                    egui::pos2(full_rect.center().x, y),
                    egui::Align2::CENTER_TOP,
                    "<<interface>>",
                    small_font.clone(),
                    egui::Color32::GRAY,
                );
                y += 14.0;
                painter.text(
                    egui::pos2(full_rect.center().x, y),
                    egui::Align2::CENTER_TOP,
                    &iface.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
                y += 18.0;
                if !iface.classifier.operations.is_empty() {
                    y += 2.0;
                    painter.line_segment(
                        [egui::pos2(left, y), egui::pos2(right, y)],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(150)),
                    );
                    y += 4.0;
                    for op in &iface.classifier.operations {
                        let vis = visibility_symbol(op.visibility);
                        let params: Vec<String> = op
                            .parameters
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}: {}",
                                    p.name,
                                    type_display(&p.type_ref, Some(&self.model))
                                )
                            })
                            .collect();
                        let ret = type_display(&op.return_type, Some(&self.model));
                        let line = format!("{} {}({}): {}", vis, op.name, params.join(", "), ret);
                        painter.text(
                            egui::pos2(left, y),
                            egui::Align2::LEFT_TOP,
                            line,
                            font_id.clone(),
                            egui::Color32::BLACK,
                        );
                        y += 15.0;
                    }
                }
            },
            Some(ModelElement::Enum(e)) => {
                painter.text(
                    egui::pos2(full_rect.center().x, y),
                    egui::Align2::CENTER_TOP,
                    "<<enumeration>>",
                    small_font.clone(),
                    egui::Color32::GRAY,
                );
                y += 14.0;
                painter.text(
                    egui::pos2(full_rect.center().x, y),
                    egui::Align2::CENTER_TOP,
                    &e.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
                y += 18.0;
                if !e.literals.is_empty() {
                    y += 2.0;
                    painter.line_segment(
                        [egui::pos2(left, y), egui::pos2(right, y)],
                        egui::Stroke::new(1.0, egui::Color32::from_gray(150)),
                    );
                    y += 4.0;
                    for lit in &e.literals {
                        let line = match &lit.value {
                            Some(v) => format!("{} = {}", lit.name, v),
                            None => lit.name.clone(),
                        };
                        painter.text(
                            egui::pos2(left, y),
                            egui::Align2::LEFT_TOP,
                            line,
                            font_id.clone(),
                            egui::Color32::BLACK,
                        );
                        y += 15.0;
                    }
                }
            },
            Some(ModelElement::Datatype(dt)) => {
                painter.text(
                    egui::pos2(full_rect.center().x, y),
                    egui::Align2::CENTER_TOP,
                    "<<datatype>>",
                    small_font.clone(),
                    egui::Color32::GRAY,
                );
                y += 14.0;
                painter.text(
                    egui::pos2(full_rect.center().x, y),
                    egui::Align2::CENTER_TOP,
                    &dt.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
            },
            Some(ModelElement::Package(pkg)) => {
                // Tab-style package header
                let tab_rect =
                    egui::Rect::from_min_size(full_rect.left_top(), egui::vec2(100.0, 20.0));
                painter.rect_filled(tab_rect, 0.0, fill);
                painter.rect_stroke(
                    tab_rect,
                    0.0,
                    egui::Stroke::new(1.5, egui::Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    tab_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &pkg.base.name,
                    font_id.clone(),
                    egui::Color32::BLACK,
                );
                // Main body
                let body = egui::Rect::from_min_max(
                    egui::pos2(full_rect.left(), tab_rect.bottom()),
                    full_rect.right_bottom(),
                );
                painter.rect_stroke(
                    body,
                    0.0,
                    egui::Stroke::new(1.5, egui::Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
            },
            Some(ModelElement::Actor(actor)) => {
                // ── Stick-figure icon ──
                let cx = full_rect.center().x;
                let top = full_rect.top() + 4.0;
                let stick_color = egui::Color32::from_gray(60);

                // Head (circle)
                let head_center = egui::pos2(cx, top + 6.0);
                painter.circle_filled(head_center, 5.0, stick_color);

                // Body (vertical line from below head)
                let body_top = egui::pos2(cx, top + 12.0);
                let body_bottom = egui::pos2(cx, top + 28.0);
                painter.line_segment([body_top, body_bottom], egui::Stroke::new(1.5, stick_color));

                // Arms (horizontal line at shoulder level)
                let shoulder_y = top + 16.0;
                painter.line_segment(
                    [
                        egui::pos2(cx - 8.0, shoulder_y),
                        egui::pos2(cx + 8.0, shoulder_y),
                    ],
                    egui::Stroke::new(1.5, stick_color),
                );

                // Left leg
                painter.line_segment(
                    [body_bottom, egui::pos2(cx - 6.0, top + 36.0)],
                    egui::Stroke::new(1.5, stick_color),
                );
                // Right leg
                painter.line_segment(
                    [body_bottom, egui::pos2(cx + 6.0, top + 36.0)],
                    egui::Stroke::new(1.5, stick_color),
                );

                // Name below the stick figure
                painter.text(
                    egui::pos2(cx, top + 40.0),
                    egui::Align2::CENTER_TOP,
                    &actor.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
            },
            Some(ModelElement::UseCase(uc)) => {
                // ── Ellipse with centered name ──
                let ellipse_color = egui::Color32::from_gray(60);
                let stroke = egui::Stroke::new(1.5, ellipse_color);
                let inset = egui::vec2(6.0, 8.0);
                let ellipse_rect = full_rect.shrink2(inset);
                let corner_radius = (ellipse_rect.height() / 2.0).min(ellipse_rect.width() / 2.0);
                painter.rect_stroke(ellipse_rect, corner_radius, stroke, egui::StrokeKind::Inside);

                // Name centered inside the ellipse
                painter.text(
                    ellipse_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    &uc.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
            },
            Some(ModelElement::Component(component)) => {
                let stroke_width = if component.executable { 3.0 } else { 1.5 };
                let stroke = egui::Stroke::new(stroke_width, egui::Color32::BLACK);
                painter.rect_stroke(full_rect, 0.0, stroke, egui::StrokeKind::Inside);

                // UML 2 component glyph: two small tabs in the upper-right corner.
                let glyph_width = (full_rect.width() * 0.18).clamp(10.0, 24.0);
                let glyph_height = (full_rect.height() * 0.32).clamp(10.0, 18.0);
                let glyph = egui::Rect::from_min_size(
                    egui::pos2(
                        (full_rect.right() - glyph_width - 5.0).max(full_rect.left()),
                        (full_rect.top() + 4.0).min(full_rect.bottom()),
                    ),
                    egui::vec2(glyph_width, glyph_height),
                );
                painter.rect_stroke(glyph, 0.0, stroke, egui::StrokeKind::Inside);
                let tab_width = (glyph_width * 0.35).max(3.0);
                for offset in [0.25_f32, 0.55] {
                    let y = glyph.top() + glyph.height() * offset;
                    painter.line_segment(
                        [
                            egui::pos2(glyph.left() - tab_width, y),
                            egui::pos2(glyph.left(), y),
                        ],
                        stroke,
                    );
                }
                painter.text(
                    egui::pos2(full_rect.center().x, full_rect.center().y),
                    egui::Align2::CENTER_CENTER,
                    &component.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
            },
            Some(ModelElement::Node(node)) => {
                // Keep the pseudo-3D depth bounded for tiny or transformed nodes.
                let depth = (full_rect.width().min(full_rect.height()) / 3.0).clamp(0.0, 12.0);
                let front = full_rect.shrink2(egui::vec2(depth, depth));
                let top = egui::Rect::from_min_max(
                    egui::pos2(full_rect.left(), full_rect.top()),
                    egui::pos2(front.right(), front.top()),
                );
                let side = egui::Rect::from_min_max(
                    egui::pos2(front.right(), full_rect.top()),
                    egui::pos2(full_rect.right(), front.bottom()),
                );
                if depth > 0.0 {
                    painter.rect_filled(top, 0.0, fill.gamma_multiply(0.85));
                    painter.rect_filled(side, 0.0, fill.gamma_multiply(0.7));
                    painter.line_segment(
                        [full_rect.left_top(), front.left_top()],
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                    painter.line_segment(
                        [front.right_top(), full_rect.right_top()],
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                    painter.line_segment(
                        [front.right_bottom(), full_rect.right_bottom()],
                        egui::Stroke::new(1.0, egui::Color32::BLACK),
                    );
                }
                painter.rect_stroke(
                    front,
                    0.0,
                    egui::Stroke::new(1.5, egui::Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    front.center(),
                    egui::Align2::CENTER_CENTER,
                    &node.base.name,
                    name_font.clone(),
                    egui::Color32::BLACK,
                );
            },
            Some(ModelElement::Artifact(artifact)) => {
                let stroke = egui::Stroke::new(1.5, egui::Color32::BLACK);
                let inset_amount =
                    (full_rect.width().min(full_rect.height()) / 4.0).clamp(0.0, 3.0);
                let inset = full_rect.shrink(inset_amount);
                match artifact.draw_as {
                    ArtifactDrawMode::Default => {
                        painter.rect_stroke(inset, 0.0, stroke, egui::StrokeKind::Inside);
                    },
                    ArtifactDrawMode::File => {
                        painter.rect_stroke(inset, 0.0, stroke, egui::StrokeKind::Inside);
                        let fold = (inset.width().min(inset.height()) * 0.2).clamp(6.0, 16.0);
                        painter.line_segment(
                            [
                                egui::pos2(inset.right() - fold, inset.top()),
                                egui::pos2(inset.right() - fold, inset.top() + fold),
                            ],
                            stroke,
                        );
                        painter.line_segment(
                            [
                                egui::pos2(inset.right() - fold, inset.top() + fold),
                                egui::pos2(inset.right(), inset.top() + fold),
                            ],
                            stroke,
                        );
                    },
                    ArtifactDrawMode::Library => {
                        painter.rect_stroke(inset, 2.0, stroke, egui::StrokeKind::Inside);
                        let shelf_y = inset.center().y;
                        painter.line_segment(
                            [
                                egui::pos2(inset.left() + 5.0, shelf_y),
                                egui::pos2(inset.right() - 5.0, shelf_y),
                            ],
                            stroke,
                        );
                        painter.line_segment(
                            [
                                egui::pos2(inset.left() + 5.0, shelf_y - 8.0),
                                egui::pos2(inset.left() + 5.0, shelf_y + 8.0),
                            ],
                            stroke,
                        );
                        painter.line_segment(
                            [
                                egui::pos2(inset.right() - 5.0, shelf_y - 8.0),
                                egui::pos2(inset.right() - 5.0, shelf_y + 8.0),
                            ],
                            stroke,
                        );
                    },
                    ArtifactDrawMode::Table => {
                        painter.rect_stroke(inset, 0.0, stroke, egui::StrokeKind::Inside);
                        let x = inset.left() + inset.width() / 3.0;
                        let y = inset.top() + inset.height() / 2.0;
                        painter.line_segment(
                            [egui::pos2(x, inset.top()), egui::pos2(x, inset.bottom())],
                            stroke,
                        );
                        painter.line_segment(
                            [egui::pos2(inset.left(), y), egui::pos2(inset.right(), y)],
                            stroke,
                        );
                    },
                }
                painter.text(
                    inset.center(),
                    egui::Align2::CENTER_CENTER,
                    &artifact.base.name,
                    name_font,
                    egui::Color32::BLACK,
                );
            },
            _ => {
                let name = elem.map(|e| e.name().to_string()).unwrap_or_default();
                painter.text(
                    full_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    name,
                    name_font,
                    egui::Color32::BLACK,
                );
            },
        }

        // Selection highlight border (draw on top of normal border)
        if self.selected_element_id == Some(node.model_element_id) {
            painter.rect_stroke(
                full_rect,
                4.0,
                egui::Stroke::new(2.5, egui::Color32::from_rgb(0, 120, 215)),
                egui::StrokeKind::Inside,
            );
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Edge drawing with UML arrowheads
    // ═══════════════════════════════════════════════════════════════════

    pub(crate) fn screen_edge_paths(
        &self,
        diagram: &Diagram,
        origin: egui::Pos2,
    ) -> Vec<ScreenEdgePath> {
        let Some(transform) = self.viewport_transform(origin) else {
            return Vec::new();
        };
        diagram
            .edges
            .values()
            .filter_map(|edge| {
                let src = diagram.get_node(edge.source_node_id)?;
                let tgt = diagram.get_node(edge.target_node_id)?;
                let kind = match self.model.get(edge.relationship_id) {
                    Some(ModelElement::Relationship(relationship)) => relationship.kind,
                    _ => {
                        return Some(ScreenEdgePath {
                            relationship_id: edge.relationship_id,
                            points: edge_path_points(src, tgt, edge, transform),
                            kind: AssociationType::Association,
                        })
                    },
                };
                Some(ScreenEdgePath {
                    relationship_id: edge.relationship_id,
                    points: edge_path_points(src, tgt, edge, transform),
                    kind,
                })
            })
            .filter(|path| path.points.len() >= 2)
            .collect()
    }

    fn draw_edges(&self, diagram: &Diagram, ui: &egui::Ui) {
        let painter = ui.painter();
        for path in self.screen_edge_paths(diagram, ui.max_rect().min) {
            let [src_center, .., tgt_center] = path.points.as_slice() else {
                continue;
            };
            let rel_kind = path.kind;
            let final_dir = *tgt_center - path.points[path.points.len() - 2];
            let final_len = final_dir.length();
            if final_len < 1.0 {
                continue;
            }
            let final_unit = final_dir / final_len;
            let final_perp = egui::vec2(-final_unit.y, final_unit.x);
            if self.selected_element_id == Some(path.relationship_id)
                && matches!(
                    self.model.get(path.relationship_id),
                    Some(ModelElement::Relationship(_))
                )
            {
                for segment in path.points.windows(2) {
                    painter.line_segment(
                        [segment[0], segment[1]],
                        egui::Stroke::new(
                            5.0,
                            egui::Color32::from_rgba_premultiplied(30, 120, 255, 100),
                        ),
                    );
                }
            }
            let dir = *tgt_center - *src_center;
            let len = dir.length();
            if len < 1.0 {
                continue;
            }
            let unit = dir / len;
            let perp = egui::vec2(-unit.y, unit.x);
            let black = egui::Color32::BLACK;
            let gray = egui::Color32::from_gray(100);

            let tip = *tgt_center - final_unit * 20.0;
            let draw_path = |stroke: egui::Stroke| {
                for (index, segment) in path.points.windows(2).enumerate() {
                    let end = if index + 2 == path.points.len() {
                        tip
                    } else {
                        segment[1]
                    };
                    painter.line_segment([segment[0], end], stroke);
                }
            };
            let draw_dashed_path = |stroke: egui::Stroke| {
                for (index, segment) in path.points.windows(2).enumerate() {
                    let end = if index + 2 == path.points.len() {
                        tip
                    } else {
                        segment[1]
                    };
                    draw_dashed_line(painter, segment[0], end, stroke);
                }
            };

            match rel_kind {
                AssociationType::Generalization => {
                    draw_path(egui::Stroke::new(1.5, black));
                    draw_hollow_triangle(painter, tip, final_unit, final_perp, black);
                },
                AssociationType::Realization => {
                    draw_dashed_path(egui::Stroke::new(1.5, black));
                    draw_hollow_triangle(painter, tip, final_unit, final_perp, black);
                },
                AssociationType::Aggregation => {
                    let diamond_center = *src_center;
                    draw_path(egui::Stroke::new(1.5, black));
                    draw_hollow_diamond(painter, diamond_center, unit, perp, black);
                },
                AssociationType::Composition => {
                    let diamond_center = *src_center;
                    draw_path(egui::Stroke::new(1.5, black));
                    draw_filled_diamond(painter, diamond_center, unit, perp, black);
                },
                AssociationType::Dependency => {
                    draw_dashed_path(egui::Stroke::new(1.0, gray));
                    draw_open_arrow(painter, tip, final_unit, final_perp, gray);
                },
                _ => {
                    draw_path(egui::Stroke::new(1.0, gray));
                },
            }
        }
    }
}

fn edge_path_points(
    source: &ViewNode,
    target: &ViewNode,
    edge: &uml_core::ViewEdge,
    transform: crate::app::viewport::ViewportTransform,
) -> Vec<egui::Pos2> {
    let source = Point::new(
        source.bounds.x() + source.bounds.width() / 2.0,
        source.bounds.y() + source.bounds.height() / 2.0,
    );
    let target = Point::new(
        target.bounds.x() + target.bounds.width() / 2.0,
        target.bounds.y() + target.bounds.height() / 2.0,
    );
    let mut points = Vec::with_capacity(edge.waypoints.len() + 2);
    points.push(transform.model_to_screen(source));
    points.extend(
        edge.waypoints
            .iter()
            .copied()
            .map(|point| transform.model_to_screen(point)),
    );
    points.push(transform.model_to_screen(target));
    points
}
