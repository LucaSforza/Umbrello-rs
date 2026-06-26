//! Left panel tree view — diagram list and element flat list.
//!
//! Provides the "New Class Diagram" button, diagram selection list, and a flat
//! element browser showing all model elements by type and name.

use crate::app::UmbrelloApp;
use uml_core::{Diagram, DiagramKind, ModelElement, Rect, ViewNode};

impl UmbrelloApp {
    /// Render the left panel (model browser tree).
    pub(crate) fn render_tree(&mut self, ui: &mut egui::Ui) {
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
}
