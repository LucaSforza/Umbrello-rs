//! Canvas rendering — partitioned UML node boxes, edge drawing with arrowheads,
//! node drag-and-drop, and ghost preview for creation tools.

use crate::app::UmbrelloApp;
use crate::rendering::{
    draw_dashed_line, draw_filled_diamond, draw_hollow_diamond, draw_hollow_triangle,
    draw_open_arrow, element_color, type_display, visibility_symbol,
};
use crate::tool_palette::ToolMode;
use uml_core::{commands, AssociationType, Diagram, ModelElement, Point, ViewNode};

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
                        ui.label("No UML model loaded. Try:");
                        ui.label("  cargo run -- tests/data/xmi/test-COG.xmi");
                    } else {
                        ui.label(format!(
                            "Model has {} elements but no diagrams.",
                            self.model.len()
                        ));
                        ui.add_space(8.0);
                        ui.label("→ Click 'New Class Diagram' in the left panel");
                        ui.label("  to create a visual layout.");
                    }
                } else {
                    ui.label("→ Select a diagram from the left panel to view it.");
                }
            });
            return;
        };

        let diagram = self.model.diagrams()[diag_idx].clone();
        let diagram_id = diagram.id;

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
                        let src_center = egui::pos2(
                            (source_node.bounds.x() + source_node.bounds.width() / 2.0) as f32,
                            (source_node.bounds.y() + source_node.bounds.height() / 2.0) as f32,
                        );
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

            let rect = egui::Rect::from_min_size(
                egui::pos2(node.bounds.x() as f32, node.bounds.y() as f32),
                egui::Vec2::new(node.bounds.width() as f32, node.bounds.height() as f32),
            );

            // Draw the partitioned node
            self.draw_partitioned_node(ui, node, rect);

            node_rects.push((node.model_element_id, rect, node.bounds.x(), node.bounds.y()));
        }

        // ── Handle interactions ──
        if self.current_tool == ToolMode::Select {
            // ── Select mode: click-to-select + drag-to-move ──
            for &(model_element_id, rect, orig_x, orig_y) in &node_rects {
                let response = ui.allocate_rect(rect, egui::Sense::click_and_drag());

                if response.dragged() {
                    if self.drag_node_id != Some(model_element_id) {
                        self.drag_node_id = Some(model_element_id);
                        self.drag_start_pos = Some(egui::pos2(orig_x as f32, orig_y as f32));
                    }
                    let delta = response.drag_delta();
                    let new_pos =
                        Point::new(orig_x + f64::from(delta.x), orig_y + f64::from(delta.y));
                    if let Ok(cmd) =
                        commands::MoveNode::new(&self.model, diagram_id, model_element_id, new_pos)
                    {
                        self.execute_command(Box::new(cmd));
                    }
                }
                if response.clicked() {
                    let name = self
                        .model
                        .get(model_element_id)
                        .map(|e| e.name().to_string())
                        .unwrap_or_default();
                    self.selected_element_id = Some(model_element_id);
                    if let Some(elem) = self.model.get(model_element_id) {
                        self.name_edit_buffer = elem.name().to_string();
                    }
                    self.status_message = format!("Selected: {}", name);
                }
                if response.drag_stopped() {
                    self.drag_node_id = None;
                    self.drag_start_pos = None;
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
                        self.preview_position =
                            Some(Point::new(f64::from(pointer_pos.x), f64::from(pointer_pos.y)));
                    }
                } else {
                    self.preview_position = None;
                }

                // Click to create
                if bg_response.clicked() {
                    if let Some(click_pos) = bg_response.interact_pointer_pos() {
                        let pos = Point::new(f64::from(click_pos.x), f64::from(click_pos.y));
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
        if self.current_tool == ToolMode::Select && self.selected_element_id.is_some() {
            let bg_interact = ui.interact(ui.max_rect(), ui.next_auto_id(), egui::Sense::click());
            if bg_interact.clicked() {
                self.selected_element_id = None;
                self.name_edit_buffer.clear();
                self.status_message = "Selection cleared".into();
            }
        }

        // ── Ghost preview rectangle ─────────────────────────────────
        if let Some(preview_pos) = self.preview_position {
            let preview_rect = egui::Rect::from_min_size(
                egui::pos2(preview_pos.x as f32 - 80.0, preview_pos.y as f32 - 30.0),
                egui::Vec2::new(160.0, 60.0),
            );
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

    fn draw_partitioned_node(&self, ui: &egui::Ui, node: &ViewNode, full_rect: egui::Rect) {
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

    fn draw_edges(&self, diagram: &Diagram, ui: &egui::Ui) {
        let painter = ui.painter();

        for (_, edge) in &diagram.edges {
            let src_node = diagram.get_node(edge.source_node_id);
            let tgt_node = diagram.get_node(edge.target_node_id);
            let (Some(src), Some(tgt)) = (src_node, tgt_node) else {
                continue;
            };

            let src_center = egui::pos2(
                (src.bounds.x() + src.bounds.width() / 2.0) as f32,
                (src.bounds.y() + src.bounds.height() / 2.0) as f32,
            );
            let tgt_center = egui::pos2(
                (tgt.bounds.x() + tgt.bounds.width() / 2.0) as f32,
                (tgt.bounds.y() + tgt.bounds.height() / 2.0) as f32,
            );

            // Determine relationship type
            let rel_kind = self
                .model
                .get(edge.relationship_id)
                .and_then(|e| match e {
                    ModelElement::Relationship(r) => Some(r.kind),
                    _ => None,
                })
                .unwrap_or(AssociationType::Association);

            let dir = tgt_center - src_center;
            let len = dir.length();
            if len < 1.0 {
                continue;
            }
            let unit = dir / len;
            let perp = egui::vec2(-unit.y, unit.x);
            let black = egui::Color32::BLACK;
            let gray = egui::Color32::from_gray(100);

            // Arrow tip: step back from target center to touch the rectangle edge
            let tip = tgt_center - unit * 20.0;

            match rel_kind {
                AssociationType::Generalization => {
                    painter.line_segment([src_center, tip], egui::Stroke::new(1.5, black));
                    draw_hollow_triangle(painter, tip, unit, perp, black);
                },
                AssociationType::Realization => {
                    draw_dashed_line(painter, src_center, tip, egui::Stroke::new(1.5, black));
                    draw_hollow_triangle(painter, tip, unit, perp, black);
                },
                AssociationType::Aggregation => {
                    let diamond_center = src_center;
                    painter
                        .line_segment([diamond_center, tgt_center], egui::Stroke::new(1.5, black));
                    draw_hollow_diamond(painter, diamond_center, unit, perp, black);
                },
                AssociationType::Composition => {
                    let diamond_center = src_center;
                    painter
                        .line_segment([diamond_center, tgt_center], egui::Stroke::new(1.5, black));
                    draw_filled_diamond(painter, diamond_center, unit, perp, black);
                },
                AssociationType::Dependency => {
                    draw_dashed_line(painter, src_center, tip, egui::Stroke::new(1.0, gray));
                    draw_open_arrow(painter, tip, unit, perp, gray);
                },
                _ => {
                    // Plain association
                    painter.line_segment([src_center, tgt_center], egui::Stroke::new(1.0, gray));
                },
            }
        }
    }
}
