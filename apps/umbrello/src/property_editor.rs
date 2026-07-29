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
use crate::tool_palette::element_is_compatible_with_diagram;
use uml_core::{AssociationType, ModelElement};

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
                ui.label("Select a node, edge, or browser element to inspect its properties.");
                return;
            },
        };

        // Verify the element still exists.
        if self.model.get(selected_id).is_none() {
            self.clear_selection();
            return;
        }

        if matches!(self.model.get(selected_id), Some(ModelElement::Relationship(_))) {
            self.render_relationship_editor(ui, selected_id);
            return;
        }

        // ── Read-Only Fields ────────────────────────────────────────
        // Extract snapshot data to avoid holding borrows across closures.
        let (
            type_str,
            id_str_snapshot,
            current_name,
            current_vis,
            current_doc,
            is_abs,
            is_sta,
            is_classifier,
        ) = {
            let Some(elem) = self.model.get(selected_id) else {
                return;
            };
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
                elem.classifier_data().is_some(),
            )
        };

        ui.label(format!("Type: {}", type_str));
        ui.label(format!("ID: {}", id_str_snapshot));
        ui.add_space(6.0);

        let add_state = self.add_to_diagram_state(selected_id);
        let add_button =
            ui.add_enabled(add_state.is_ok(), egui::Button::new("Add to active diagram"));
        let add_clicked = add_button.clicked();
        if let Err(reason) = &add_state {
            add_button.on_hover_text(*reason);
        }
        if add_clicked {
            match self.add_element_to_active_diagram(selected_id) {
                Ok(()) => self.status_message = "Element added to active diagram".into(),
                Err(error) => self.status_message = format!("Error: {error}"),
            }
        }
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
                if !new_name.is_empty()
                    && new_name != current_name
                    && self.rename_element(selected_id, new_name.clone()).is_ok()
                {
                    self.name_edit_buffer = new_name;
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
                            let _ = self.set_visibility(selected_id, vis);
                        }
                    }
                });
        });
        ui.add_space(4.0);

        // ── Abstract / Static Checkboxes (classifiers only) ──────────
        if is_classifier {
            ui.horizontal(|ui| {
                let mut new_abstract = is_abs;
                let mut new_static = is_sta;
                let changed_abs = ui.checkbox(&mut new_abstract, "Abstract").changed();
                let changed_sta = ui.checkbox(&mut new_static, "Static").changed();

                if changed_abs || changed_sta {
                    let _ = self.set_flags(selected_id, new_abstract, new_static);
                }
            });
            ui.add_space(6.0);
        }

        // ── Documentation TextEdit ──────────────────────────────────
        ui.label("Documentation:");
        let doc_edit = ui.add(
            egui::TextEdit::multiline(&mut self.documentation_edit_buffer)
                .desired_rows(3)
                .desired_width(ui.available_width()),
        );
        if doc_edit.lost_focus() {
            let doc = self.documentation_edit_buffer.clone();
            if doc != current_doc && self.set_documentation(selected_id, doc).is_ok() {
                self.documentation_edit_buffer = self
                    .model
                    .get(selected_id)
                    .map_or_else(String::new, |element| element.base().documentation.clone());
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

impl UmbrelloApp {
    fn render_relationship_editor(&mut self, ui: &mut egui::Ui, id: uml_core::UmlId) {
        let needs_refresh = self
            .relationship_draft
            .as_ref()
            .is_none_or(|(draft_id, _)| *draft_id != id);
        if needs_refresh {
            self.refresh_property_buffers();
        }
        let Some(ModelElement::Relationship(relationship)) = self.model.get(id) else {
            return;
        };
        let source_name = self.model.get(relationship.source_id).map_or_else(
            || relationship.source_id.to_string(),
            |element| element.name().to_string(),
        );
        let target_name = self.model.get(relationship.target_id).map_or_else(
            || relationship.target_id.to_string(),
            |element| element.name().to_string(),
        );
        let id_text = relationship.base.id.to_string();
        let id_text = if id_text.len() > 20 {
            format!("{}...", &id_text[..20])
        } else {
            id_text
        };
        ui.label("Type: Relationship");
        ui.label(format!("ID: {id_text}"));
        ui.label(format!("Source: {source_name}"));
        ui.label(format!("Target: {target_name}"));
        ui.add_space(6.0);

        let kinds = self.allowed_relationship_kinds();
        let kind_availability: Vec<_> = kinds
            .iter()
            .copied()
            .map(|kind| (kind, self.relationship_kind_allowed(kind)))
            .collect();
        let Some((_, draft)) = self.relationship_draft.as_mut() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label("Kind:");
            egui::ComboBox::from_id_salt("relationship_kind_combo")
                .selected_text(draft.kind.as_str())
                .show_ui(ui, |ui| {
                    for (kind, allowed) in &kind_availability {
                        let enabled = *kind == draft.kind || *allowed;
                        ui.add_enabled_ui(enabled, |ui| {
                            if ui
                                .selectable_label(draft.kind == *kind, kind.as_str())
                                .clicked()
                            {
                                draft.kind = *kind;
                            }
                        });
                    }
                });
        });
        relationship_text_edit(ui, "Name:", &mut draft.name);
        relationship_text_edit(ui, "Documentation:", &mut draft.documentation);
        relationship_text_edit(ui, "Source role:", &mut draft.source_role);
        relationship_text_edit(ui, "Source multiplicity:", &mut draft.source_multiplicity);
        ui.checkbox(&mut draft.source_navigable, "Source → target navigable");
        relationship_text_edit(ui, "Target role:", &mut draft.target_role);
        relationship_text_edit(ui, "Target multiplicity:", &mut draft.target_multiplicity);
        ui.checkbox(&mut draft.target_navigable, "Target → source navigable");
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                match self.apply_relationship_draft(id) {
                    Ok(true) => self.status_message = "Relationship updated".into(),
                    Ok(false) => self.status_message = "Relationship unchanged (no changes)".into(),
                    Err(error) => {
                        self.status_message = format!("Relationship apply failed: {error}")
                    },
                }
            }
            if ui.button("Revert").clicked() {
                self.refresh_property_buffers();
                self.status_message = "Relationship draft reverted".into();
            }
        });
    }

    fn allowed_relationship_kinds(&self) -> Vec<AssociationType> {
        [
            AssociationType::Association,
            AssociationType::Generalization,
            AssociationType::Realization,
            AssociationType::Aggregation,
            AssociationType::Composition,
            AssociationType::Dependency,
        ]
        .into_iter()
        .collect()
    }

    pub(crate) fn relationship_kind_allowed(&self, kind: AssociationType) -> bool {
        let Some(diagram_kind) = self.active_diagram_kind() else {
            return true;
        };
        let tool = match kind {
            AssociationType::Association => crate::tool_palette::ToolMode::CreateAssociation,
            AssociationType::Generalization => crate::tool_palette::ToolMode::CreateGeneralization,
            AssociationType::Realization => crate::tool_palette::ToolMode::CreateRealization,
            AssociationType::Aggregation => crate::tool_palette::ToolMode::CreateAggregation,
            AssociationType::Composition => crate::tool_palette::ToolMode::CreateComposition,
            AssociationType::Dependency => crate::tool_palette::ToolMode::CreateDependency,
        };
        tool.is_compatible_with_diagram(diagram_kind)
    }

    pub(crate) fn apply_relationship_draft(&mut self, id: uml_core::UmlId) -> Result<bool, String> {
        let Some(ModelElement::Relationship(current)) = self.model.get(id) else {
            return Err("selected element is not a relationship".into());
        };
        let Some((draft_id, draft)) = self.relationship_draft.as_ref() else {
            return Err("relationship draft is unavailable".into());
        };
        if *draft_id != id
            || (!self.relationship_kind_allowed(draft.kind) && draft.kind != current.kind)
        {
            return Err("relationship draft is stale or incompatible".into());
        }
        let mut replacement = current.clone();
        replacement.kind = draft.kind;
        replacement.base.name = draft.name.clone();
        replacement.base.documentation = draft.documentation.clone();
        replacement.source_role_name = optional_text(&draft.source_role);
        replacement.source_multiplicity = optional_text(&draft.source_multiplicity);
        replacement.target_role_name = optional_text(&draft.target_role);
        replacement.target_multiplicity = optional_text(&draft.target_multiplicity);
        replacement.source_to_target_navigable = draft.source_navigable;
        replacement.target_to_source_navigable = draft.target_navigable;
        if replacement == *current {
            return Ok(false);
        }
        let command = uml_core::commands::UpdateRelationship::new(&self.model, id, replacement)
            .map_err(|error| error.to_string())?;
        self.execute_command_result(Box::new(command))
            .map_err(|error| error.to_string())?;
        self.refresh_property_buffers();
        Ok(true)
    }

    pub(crate) fn add_to_diagram_state(&self, id: uml_core::UmlId) -> Result<(), &'static str> {
        if self.current_file_path.is_none() {
            return Err("Create or open an XMI project first");
        }
        let Some(index) = self.active_diagram else {
            return Err("Select an active diagram first");
        };
        let Some(diagram) = self.model.diagrams().get(index) else {
            return Err("Active diagram is unavailable");
        };
        let Some(element) = self.model.get(id) else {
            return Err("Element is unavailable");
        };
        if matches!(element, ModelElement::Relationship(_)) {
            return Err("Relationships cannot be added as nodes");
        }
        if !element_is_compatible_with_diagram(element, diagram.kind) {
            return Err("Element is incompatible with the active diagram");
        }
        if diagram.get_node(id).is_some() {
            return Err("Element is already on the active diagram");
        }
        Ok(())
    }
}

fn optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

fn relationship_text_edit(ui: &mut egui::Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.text_edit_singleline(value);
}

/// Snapshot of classifier data for rendering without holding model borrows.
struct ClassifierSnapshot {
    attrs: Vec<(&'static str, String, String)>,
    ops: Vec<(&'static str, String, Vec<String>, String)>,
}
