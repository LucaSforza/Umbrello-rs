//! Left panel tree view — diagram list and element flat list.
//!
//! Provides the "New Diagram…" button, diagram selection list, and a flat
//! element browser showing all model elements by type and name.

use crate::app::UmbrelloApp;
use uml_core::ModelElement;

impl UmbrelloApp {
    /// Render the fixed New Diagram control above the scrolling left-panel content.
    pub(crate) fn render_new_diagram_control(&mut self, ui: &mut egui::Ui) {
        let can_create_diagram = self.current_file_path.is_some();
        let new_diagram = ui.add_enabled(can_create_diagram, egui::Button::new("New Diagram…"));
        if !can_create_diagram {
            new_diagram.on_hover_text("Create or open an XMI project before adding diagrams");
        } else if new_diagram.clicked() {
            self.open_new_diagram_dialog();
        }
    }

    /// Render the scrolling left-panel model browser tree.
    pub(crate) fn render_tree(&mut self, ui: &mut egui::Ui) {
        ui.heading("Diagrams");
        let mut selected_index = None;
        for (i, diag) in self.model.diagrams().iter().enumerate() {
            let selected = self.active_diagram == Some(i);
            if ui
                .selectable_label(selected, format!("{} ({})", diag.name, diag.kind.as_str()))
                .clicked()
            {
                selected_index = Some(i);
            }
        }
        if let Some(index) = selected_index {
            self.activate_diagram_index(index);
        }
        ui.separator();
        ui.heading("Elements");
        let elements: Vec<_> = self
            .model
            .iter()
            .map(|(id, element)| {
                let label = if let ModelElement::Relationship(relationship) = element {
                    let source = self.model.get(relationship.source_id).map_or_else(
                        || relationship.source_id.to_string(),
                        |item| item.name().to_string(),
                    );
                    let target = self.model.get(relationship.target_id).map_or_else(
                        || relationship.target_id.to_string(),
                        |item| item.name().to_string(),
                    );
                    format!("{}: {source} → {target}", relationship.kind.as_str())
                } else {
                    format!("{}: {}", element.object_type().as_str(), element.name())
                };
                (id, label)
            })
            .collect();
        for (id, label) in elements {
            if ui
                .selectable_label(self.selected_element_id == Some(id), label)
                .clicked()
            {
                let _ = self.select_element(id);
                self.status_message = "Element selected".into();
            }
        }
    }
}
