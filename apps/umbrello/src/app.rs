//! Application state and rendering for Umbrello-RS.
//!
//! Implements the interactive canvas: model browser tree, diagram rendering,
//! drag-and-drop node movement, and undo/redo via the command history.

use uml_core::{
    commands, Diagram, DiagramKind, History, ModelElement, Point, Rect, UmlModel, ViewNode,
};

/// The Umbrello application state.
pub struct UmbrelloApp {
    model: UmlModel,
    history: History,
    active_diagram: Option<usize>,
    /// Track drag state between frames.
    drag_node_id: Option<uml_core::UmlId>,
    drag_start_pos: Option<egui::Pos2>,
    status_message: String,
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
        }
    }

    /// Render the menu bar.
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
                // Undo/Redo toolbar buttons
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

    /// Render the left panel (model browser).
    fn render_tree(&mut self, ui: &mut egui::Ui) {
        // Create a default diagram if none exist
        if self.model.diagrams().is_empty() && ui.button("New Class Diagram").clicked() {
            let mut d = Diagram::new("Main", DiagramKind::Class);
            // Add all classes from the model to the diagram
            for (uid, elem) in self.model.iter() {
                match elem {
                    ModelElement::Class(_)
                    | ModelElement::Interface(_)
                    | ModelElement::Enum(_)
                    | ModelElement::Datatype(_) => {
                        d.add_node(uid, ViewNode::new(uid, Rect::new(50.0, 50.0, 120.0, 60.0)));
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

    /// Render the center canvas with the active diagram.
    fn render_canvas(&mut self, ui: &mut egui::Ui) {
        let Some(diag_idx) = self.active_diagram else {
            ui.centered_and_justified(|ui| {
                ui.heading("No diagram selected");
                ui.label("Create or select a diagram from the left panel.");
            });
            return;
        };

        let diagram = self.model.diagrams()[diag_idx].clone();
        let diagram_id = diagram.id;

        // Background
        {
            let painter = ui.painter();
            painter.rect_filled(ui.max_rect(), 0.0, egui::Color32::from_gray(240));
        }

        // Draw each node: interaction first, then paint in separate scope
        // to avoid borrow conflicts with ui.painter() vs ui.allocate_rect()
        let mut node_visuals: Vec<(egui::Rect, egui::Color32, String)> = Vec::new();
        let mut node_interactions: Vec<(uml_core::UmlId, egui::Rect, f64, f64)> = Vec::new();

        for (&_node_id, node) in &diagram.nodes {
            if !node.visible {
                continue;
            }

            let rect = egui::Rect::from_min_size(
                egui::pos2(node.bounds.x() as f32, node.bounds.y() as f32),
                egui::Vec2::new(node.bounds.width() as f32, node.bounds.height() as f32),
            );

            let fill = match self.model.get(node.model_element_id) {
                Some(ModelElement::Class(_)) => egui::Color32::from_rgb(180, 210, 255),
                Some(ModelElement::Interface(_)) => egui::Color32::from_rgb(180, 255, 210),
                Some(ModelElement::Enum(_)) => egui::Color32::from_rgb(255, 210, 180),
                Some(ModelElement::Datatype(_)) => egui::Color32::from_rgb(210, 180, 255),
                _ => egui::Color32::from_rgb(220, 220, 220),
            };

            let name = self
                .model
                .get(node.model_element_id)
                .map(|e| e.name().to_string())
                .unwrap_or_else(|| "?".to_string());

            node_visuals.push((rect, fill, name.clone()));

            // Record interaction data (processed after paint to avoid borrow conflict)
            node_interactions.push((node.model_element_id, rect, node.bounds.x(), node.bounds.y()));
        }

        // Draw all node visuals
        {
            let painter = ui.painter();
            for (rect, fill, name) in &node_visuals {
                painter.rect_filled(*rect, 4.0, *fill);
                painter.rect_stroke(
                    *rect,
                    4.0,
                    egui::Stroke::new(1.5, egui::Color32::BLACK),
                    egui::StrokeKind::Inside,
                );
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    name,
                    egui::FontId::proportional(14.0),
                    egui::Color32::BLACK,
                );
            }
        }

        // Handle interactions (click + drag) for each node
        for &(model_element_id, rect, orig_x, orig_y) in &node_interactions {
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
                    .unwrap_or_else(|| "?".to_string());
                self.status_message = format!("Selected: {}", name);
            }

            if response.drag_stopped() {
                self.drag_node_id = None;
                self.drag_start_pos = None;
                let name = self
                    .model
                    .get(model_element_id)
                    .map(|e| e.name().to_string())
                    .unwrap_or_else(|| "?".to_string());
                self.status_message = format!("Moved: {}", name);
            }
        }

        // Draw edges
        {
            let painter = ui.painter();
            for (_, edge) in &diagram.edges {
                let src_node = diagram.get_node(edge.source_node_id);
                let tgt_node = diagram.get_node(edge.target_node_id);
                if let (Some(src), Some(tgt)) = (src_node, tgt_node) {
                    let src_center = egui::pos2(
                        (src.bounds.x() + src.bounds.width() / 2.0) as f32,
                        (src.bounds.y() + src.bounds.height() / 2.0) as f32,
                    );
                    let tgt_center = egui::pos2(
                        (tgt.bounds.x() + tgt.bounds.width() / 2.0) as f32,
                        (tgt.bounds.y() + tgt.bounds.height() / 2.0) as f32,
                    );
                    painter.line_segment(
                        [src_center, tgt_center],
                        egui::Stroke::new(1.5, egui::Color32::from_gray(100)),
                    );
                }
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

        // Request continuous repaints while dragging
        if self.drag_node_id.is_some() {
            ctx.request_repaint();
        }
    }
}
