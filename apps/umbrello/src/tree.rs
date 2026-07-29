//! Left panel tree view — diagram list and element flat list.
//!
//! Provides the "New Class Diagram" button, diagram selection list, and a flat
//! element browser showing all model elements by type and name.

use crate::app::UmbrelloApp;

impl UmbrelloApp {
    /// Render the left panel (model browser tree).
    pub(crate) fn render_tree(&mut self, ui: &mut egui::Ui) {
        if self.model.diagrams().is_empty() && ui.button("New Class Diagram").clicked() {
            self.new_class_diagram();
        }

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
        for (_, elem) in self.model.iter() {
            ui.label(format!("{}: {}", elem.object_type().as_str(), elem.name()));
        }
    }
}
