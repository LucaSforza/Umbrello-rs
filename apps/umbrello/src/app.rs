//! Application state and rich UML rendering for Umbrello-RS.
//!
//! Implements:
//! - Partitioned class boxes with name/attribute/operation compartments
//! - Semantic edge engine with UML arrowheads (hollow triangle, diamonds, open arrow)
//! - Drag-and-drop node movement with undo/redo via Command history.

use uml_core::{
    commands, AssociationType, Diagram, DiagramKind, History, ModelElement, Point, Rect, UmlModel,
    ViewNode, Visibility,
};

/// The Umbrello application state.
pub struct UmbrelloApp {
    model: UmlModel,
    history: History,
    active_diagram: Option<usize>,
    drag_node_id: Option<uml_core::UmlId>,
    drag_start_pos: Option<egui::Pos2>,
    status_message: String,
    /// REVIEW CONDITION C1: Track whether model was loaded from XMI.
    #[allow(dead_code)]
    loaded_from_xmi: bool,
}

impl UmbrelloApp {
    pub fn new(model: UmlModel, loaded: bool) -> Self {
        let msg = if loaded {
            format!("Loaded model with {} elements", model.len())
        } else {
            "Empty model — no XMI file loaded".to_string()
        };
        Self {
            model,
            history: History::new(100),
            active_diagram: None,
            drag_node_id: None,
            drag_start_pos: None,
            status_message: msg,
            loaded_from_xmi: loaded,
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Menu bar
    // ═══════════════════════════════════════════════════════════════════

    fn render_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Open XMI...").clicked() {
                        self.status_message = "Open not yet implemented".into();
                        ui.close_menu();
                    }
                    if ui.button("Quit").clicked() {
                        std::process::exit(0);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked()
                        || (ui
                            .ctx()
                            .input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.ctrl))
                    {
                        if self.history.can_undo() {
                            self.history.undo(&mut self.model).unwrap();
                            self.status_message = "Undo".into();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked()
                        || (ui
                            .ctx()
                            .input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.ctrl))
                    {
                        if self.history.can_redo() {
                            self.history.redo(&mut self.model).unwrap();
                            self.status_message = "Redo".into();
                        }
                        ui.close_menu();
                    }
                });
                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("↩ Undo"))
                    .clicked()
                {
                    self.history.undo(&mut self.model).unwrap();
                    self.status_message = "Undo".into();
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("↪ Redo"))
                    .clicked()
                {
                    self.history.redo(&mut self.model).unwrap();
                    self.status_message = "Redo".into();
                }
                ui.separator();
                ui.label(&self.status_message);
            });
        });
    }

    // ═══════════════════════════════════════════════════════════════════
    // Left panel (model browser)
    // ═══════════════════════════════════════════════════════════════════

    fn render_tree(&mut self, ui: &mut egui::Ui) {
        if self.model.diagrams().is_empty() && ui.button("New Class Diagram").clicked() {
            let mut d = Diagram::new("Main", DiagramKind::Class);
            for (uid, elem) in self.model.iter() {
                match elem {
                    ModelElement::Class(_)
                    | ModelElement::Interface(_)
                    | ModelElement::Enum(_)
                    | ModelElement::Datatype(_) => {
                        d.add_node(uid, ViewNode::new(uid, Rect::new(50.0, 50.0, 160.0, 60.0)));
                    },
                    _ => {},
                }
            }
            self.model.add_diagram(d);
        }

        ui.heading("Diagrams");
        for (i, diag) in self.model.diagrams().iter().enumerate() {
            let selected = self.active_diagram == Some(i);
            if ui
                .selectable_label(selected, format!("{} ({})", diag.name, diag.kind.as_str()))
                .clicked()
            {
                self.active_diagram = Some(i);
            }
        }
        ui.separator();
        ui.heading("Elements");
        for (_, elem) in self.model.iter() {
            ui.label(format!("{}: {}", elem.object_type().as_str(), elem.name()));
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // Canvas — rich UML rendering
    // ═══════════════════════════════════════════════════════════════════

    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let Some(diag_idx) = self.active_diagram else {
            ui.centered_and_justified(|ui| {
                ui.heading("No diagram selected");
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

        // ── Draw nodes ───────────────────────────────────────────────
        let mut node_rects: Vec<(uml_core::UmlId, egui::Rect, f64, f64)> = Vec::new();

        for (&_node_id, node) in &diagram.nodes {
            if !node.visible {
                continue;
            }

            // REVIEW CONDITION C2: Use stored bounds for XMI-loaded nodes
            let rect = egui::Rect::from_min_size(
                egui::pos2(node.bounds.x() as f32, node.bounds.y() as f32),
                egui::Vec2::new(node.bounds.width() as f32, node.bounds.height() as f32),
            );

            // Draw the partitioned node
            self.draw_partitioned_node(ui, node, rect);

            node_rects.push((node.model_element_id, rect, node.bounds.x(), node.bounds.y()));
        }

        // ── Handle interactions (after paint to avoid borrow conflicts) ──
        for &(model_element_id, rect, orig_x, orig_y) in &node_rects {
            let sense = egui::Sense::click_and_drag();
            let response = ui.allocate_rect(rect, sense);

            if response.dragged() {
                if self.drag_node_id != Some(model_element_id) {
                    self.drag_node_id = Some(model_element_id);
                    self.drag_start_pos = Some(egui::pos2(orig_x as f32, orig_y as f32));
                }
                let delta = response.drag_delta();
                let new_pos = Point::new(orig_x + f64::from(delta.x), orig_y + f64::from(delta.y));
                if let Ok(cmd) =
                    commands::MoveNode::new(&self.model, diagram_id, model_element_id, new_pos)
                {
                    let _ = self.history.execute(Box::new(cmd), &mut self.model);
                }
            }
            if response.clicked() {
                let name = self
                    .model
                    .get(model_element_id)
                    .map(|e| e.name().to_string())
                    .unwrap_or_default();
                self.status_message = format!("Selected: {}", name);
            }
            if response.drag_stopped() {
                self.drag_node_id = None;
                self.drag_start_pos = None;
            }
        }

        if self.drag_node_id.is_some() {
            ui.ctx().request_repaint();
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
                // REVIEW CONDITION C3: Use e.literals
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

impl eframe::App for UmbrelloApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_menu(ctx);
        egui::SidePanel::left("tree_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                self.render_tree(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
        if self.drag_node_id.is_some() {
            ctx.request_repaint();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Free functions — rendering helpers
// ═══════════════════════════════════════════════════════════════════════

fn element_color(elem: Option<&ModelElement>) -> egui::Color32 {
    match elem {
        Some(ModelElement::Class(_)) => egui::Color32::from_rgb(180, 210, 255),
        Some(ModelElement::Interface(_)) => egui::Color32::from_rgb(180, 255, 210),
        Some(ModelElement::Enum(_)) => egui::Color32::from_rgb(255, 210, 180),
        Some(ModelElement::Datatype(_)) => egui::Color32::from_rgb(210, 180, 255),
        Some(ModelElement::Package(_)) => egui::Color32::from_rgb(255, 255, 200),
        _ => egui::Color32::from_rgb(220, 220, 220),
    }
}

fn visibility_symbol(v: Visibility) -> &'static str {
    match v {
        Visibility::Public => "+",
        Visibility::Protected => "#",
        Visibility::Private => "-",
        Visibility::Implementation => "~",
    }
}

/// REVIEW CONDITION C5: Takes model reference for display_name resolution.
fn type_display(type_ref: &uml_core::TypeReference, model: Option<&UmlModel>) -> String {
    type_ref.display_name(model)
}

/// Draw a hollow triangular arrowhead at `tip` pointing in direction `dir`.
fn draw_hollow_triangle(
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
fn draw_hollow_diamond(
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
fn draw_filled_diamond(
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
fn draw_open_arrow(
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
fn draw_dashed_line(
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

// ═══════════════════════════════════════════════════════════════════════
// Unit tests for rendering helpers (REVIEW CONDITION C4)
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use uml_core::TypeReference;

    #[test]
    fn visibility_symbols() {
        assert_eq!(visibility_symbol(Visibility::Public), "+");
        assert_eq!(visibility_symbol(Visibility::Protected), "#");
        assert_eq!(visibility_symbol(Visibility::Private), "-");
        assert_eq!(visibility_symbol(Visibility::Implementation), "~");
    }

    #[test]
    fn type_display_primitive() {
        let tr = TypeReference::primitive("int");
        assert_eq!(type_display(&tr, None), "int");
    }

    #[test]
    fn type_display_unspecified() {
        let tr = TypeReference::unspecified();
        assert_eq!(type_display(&tr, None), "void");
    }

    #[test]
    fn type_display_model_resolved() {
        let mut model = UmlModel::new();
        let cls = uml_core::Class::new("Person");
        let id = cls.base.id;
        model.insert(ModelElement::Class(cls));
        let tr = TypeReference::model(id);
        assert_eq!(type_display(&tr, Some(&model)), "Person");
    }

    #[test]
    fn type_display_model_dangling() {
        let tr = TypeReference::model(uml_core::UmlId::new());
        let display = type_display(&tr, None);
        assert!(display.starts_with("<unknown:"));
    }

    #[test]
    fn element_colors() {
        let cls = ModelElement::Class(uml_core::Class::new("C"));
        let iface = ModelElement::Interface(uml_core::Interface::new("I"));
        assert_ne!(element_color(Some(&cls)), element_color(Some(&iface)));
        assert_eq!(element_color(None), egui::Color32::from_rgb(220, 220, 220));
    }
}
