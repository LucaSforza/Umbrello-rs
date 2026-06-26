//! Property editor panel — right-side inspector for selected model elements.
//!
//! When an element is selected on the canvas, this panel shows:
//! - Read-only type and ID
//! - Editable name field (commits `RenameElement` on Enter or focus loss)
//! - Visibility dropdown (commits `ChangeVisibility`)
//! - Abstract / Static checkboxes (commits `ChangeElementFlags`)
//! - Documentation text area (commits `ChangeDocumentation` on focus loss)
//! - Read-only classifier details (attribute and operation listing)

use crate::app::UmbrelloApp;
use crate::rendering::{type_display, visibility_name, visibility_symbol};
use uml_core::commands;

impl UmbrelloApp {
    /// Render the right-side property editor panel.
    pub(crate) fn render_property_editor(&mut self, ui: &mut egui::Ui) {
        ui.heading("Properties");

        // ── Nothing selected placeholder ────────────────────────────
        let selected_id = match self.selected_element_id {
            Some(id) => id,
            None => {
                ui.add_space(20.0);
                ui.centered_and_justified(|ui| {
                    ui.label(egui::RichText::new("Nothing selected").size(14.0).weak());
                });
                ui.add_space(8.0);
                ui.label("Click a node on the canvas to inspect its properties.");
                return;
            },
        };

        // Verify the element still exists.
        if self.model.get(selected_id).is_none() {
            self.selected_element_id = None;
            self.name_edit_buffer.clear();
            return;
        }

        // ── Read-Only Fields ────────────────────────────────────────
        // Extract snapshot data to avoid holding borrows across closures.
        let (type_str, id_str_snapshot, current_name, current_vis, current_doc, is_abs, is_sta) = {
            let elem = self.model.get(selected_id).unwrap();
            let id_full = elem.id().to_string();
            let id_trunc = if id_full.len() > 20 {
                format!("{}...", &id_full[..20])
            } else {
                id_full
            };
            (
                elem.object_type().as_str().to_string(),
                id_trunc,
                elem.name().to_string(),
                elem.base().visibility,
                elem.base().documentation.clone(),
                elem.base().is_abstract,
                elem.base().is_static,
            )
        };

        ui.label(format!("Type: {}", type_str));
        ui.label(format!("ID: {}", id_str_snapshot));
        ui.add_space(6.0);

        // ── Editable Name ───────────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Name:");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.name_edit_buffer)
                    .desired_width(ui.available_width()),
            );
            // Commit rename on Enter or focus loss
            if (response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                || response.lost_focus()
            {
                let new_name = self.name_edit_buffer.trim().to_string();
                if !new_name.is_empty() && new_name != current_name {
                    if let Ok(cmd) =
                        commands::RenameElement::new(&self.model, selected_id, new_name.clone())
                    {
                        self.execute_command(Box::new(cmd));
                        self.name_edit_buffer = new_name;
                    }
                }
            }
        });
        ui.add_space(4.0);

        // ── Visibility Dropdown ─────────────────────────────────────
        ui.horizontal(|ui| {
            ui.label("Visibility:");
            let vis_label =
                format!("{} {}", visibility_symbol(current_vis), visibility_name(current_vis));
            egui::ComboBox::from_id_salt("visibility_combo")
                .selected_text(vis_label)
                .show_ui(ui, |ui| {
                    let vis_options = [
                        uml_core::Visibility::Public,
                        uml_core::Visibility::Protected,
                        uml_core::Visibility::Private,
                        uml_core::Visibility::Implementation,
                    ];
                    for &vis in &vis_options {
                        let label = format!("{} {}", visibility_symbol(vis), visibility_name(vis));
                        if ui.selectable_label(current_vis == vis, label).clicked()
                            && vis != current_vis
                        {
                            if let Ok(cmd) =
                                commands::ChangeVisibility::new(&self.model, selected_id, vis)
                            {
                                self.execute_command(Box::new(cmd));
                            }
                        }
                    }
                });
        });
        ui.add_space(4.0);

        // ── Abstract / Static Checkboxes ────────────────────────────
        ui.horizontal(|ui| {
            let mut new_abstract = is_abs;
            let mut new_static = is_sta;
            let changed_abs = ui.checkbox(&mut new_abstract, "Abstract").changed();
            let changed_sta = ui.checkbox(&mut new_static, "Static").changed();

            if changed_abs || changed_sta {
                if let Ok(cmd) = commands::ChangeElementFlags::new(
                    &self.model,
                    selected_id,
                    new_abstract,
                    new_static,
                ) {
                    self.execute_command(Box::new(cmd));
                }
            }
        });
        ui.add_space(6.0);

        // ── Documentation TextEdit ──────────────────────────────────
        ui.label("Documentation:");
        let doc_orig = current_doc.clone();
        let mut doc = current_doc;
        let doc_edit = ui.add(
            egui::TextEdit::multiline(&mut doc)
                .desired_rows(3)
                .desired_width(ui.available_width()),
        );
        if doc_edit.lost_focus()
            && doc != doc_orig
            && (!doc.trim().is_empty() || !doc_orig.is_empty())
        {
            if let Ok(cmd) = commands::ChangeDocumentation::new(&self.model, selected_id, doc) {
                self.execute_command(Box::new(cmd));
            }
        }

        // ── Classifier Details (Read-Only) ─────────────────────────
        // Extract classifier snapshot data.
        let classifier_info: Option<ClassifierSnapshot> =
            self.model.get(selected_id).and_then(|elem| {
                elem.classifier_data().map(|cd| ClassifierSnapshot {
                    attrs: cd
                        .attributes
                        .iter()
                        .map(|a| {
                            (
                                visibility_symbol(a.visibility),
                                a.name.clone(),
                                type_display(&a.type_ref, Some(&self.model)),
                            )
                        })
                        .collect(),
                    ops: cd
                        .operations
                        .iter()
                        .map(|op| {
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
                            (visibility_symbol(op.visibility), op.name.clone(), params, ret)
                        })
                        .collect(),
                })
            });

        if let Some(info) = classifier_info {
            ui.separator();
            ui.heading("Classifier Details");
            ui.add_space(4.0);

            ui.label(format!("Attributes ({}):", info.attrs.len()));
            for (vis, name, type_name) in &info.attrs {
                ui.label(format!("  {} {}: {}", vis, name, type_name));
            }

            ui.add_space(4.0);
            ui.label(format!("Operations ({}):", info.ops.len()));
            for (vis, name, params, ret) in &info.ops {
                ui.label(format!("  {} {}({}): {}", vis, name, params.join(", "), ret));
            }
        }
    }
}

/// Snapshot of classifier data for rendering without holding model borrows.
struct ClassifierSnapshot {
    attrs: Vec<(&'static str, String, String)>,
    ops: Vec<(&'static str, String, Vec<String>, String)>,
}
