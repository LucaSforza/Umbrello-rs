//! Property editor panel — right-side inspector for selected model elements.
//!
//! When an element is selected on the canvas, this panel shows:
//! - Read-only type and ID
//! - Editable name field (commits `RenameElement` on Enter or focus loss)
//! - Visibility dropdown (commits `ChangeVisibility`)
//! - Abstract / Static checkboxes (commits `ChangeElementFlags`)
//! - Documentation text area (commits `ChangeDocumentation` on focus loss)
//! - Read-only classifier details (attribute and operation listing)

use crate::app::{DraftAttribute, DraftOperation, DraftParameter, UmbrelloApp};
use crate::rendering::{visibility_name, visibility_symbol};
use crate::tool_palette::element_is_compatible_with_diagram;
use uml_core::{AssociationType, ModelElement, TypeReference, Visibility};

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

        // ── Classifier Feature Editor (Editable Draft) ──────────
        if let Some((draft_id, _)) = self.classifier_draft.clone() {
            if draft_id == selected_id {
                self.render_classifier_draft_editor(ui, selected_id);
            } else {
                self.refresh_property_buffers();
            }
        }
    }

    /// Render the editable classifier draft (attributes, operations, parameters).
    fn render_classifier_draft_editor(&mut self, ui: &mut egui::Ui, id: uml_core::UmlId) {
        ui.separator();
        ui.heading("Classifier Features");
        ui.add_space(4.0);

        // ── Status message area ─────────────────────────────────
        let mut status = String::new();

        // Take draft ownership to avoid borrow conflicts.
        let (draft_id, mut draft) = self
            .classifier_draft
            .take()
            .expect("classifier draft must be present");

        // ── Attributes section ───────────────────────────────────
        ui.label(format!("Attributes ({}):", draft.attributes.len()));
        let add_attr = ui.button("+ Add Attribute").clicked();
        if add_attr {
            let next = self.generate_unique_name_in_draft(
                "attribute",
                draft.attributes.iter().map(|a| a.name.as_str()),
            );
            draft.attributes.push(DraftAttribute {
                name: next,
                type_text: String::new(),
                original_type: TypeReference::unspecified(),
                visibility: Visibility::Public,
                initial_value: String::new(),
                is_static: false,
            });
        }

        let mut delete_attr: Option<usize> = None;
        for (i, attr) in draft.attributes.iter_mut().enumerate() {
            ui.push_id(format!("attr_{i}"), |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{i}"));
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut attr.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Type:");
                        ui.text_edit_singleline(&mut attr.type_text);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Vis:");
                        let vis_options = [
                            Visibility::Public,
                            Visibility::Protected,
                            Visibility::Private,
                            Visibility::Implementation,
                        ];
                        let current_label = format!(
                            "{} {}",
                            visibility_symbol(attr.visibility),
                            visibility_name(attr.visibility)
                        );
                        egui::ComboBox::from_id_salt(format!("attr_vis_{i}"))
                            .selected_text(current_label)
                            .show_ui(ui, |ui| {
                                for &vis in &vis_options {
                                    let label = format!(
                                        "{} {}",
                                        visibility_symbol(vis),
                                        visibility_name(vis)
                                    );
                                    if ui.selectable_label(attr.visibility == vis, label).clicked()
                                    {
                                        attr.visibility = vis;
                                    }
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Init:");
                        ui.text_edit_singleline(&mut attr.initial_value);
                        ui.checkbox(&mut attr.is_static, "Static");
                    });
                    if ui.button("× Delete").clicked() {
                        delete_attr = Some(i);
                    }
                });
            });
        }
        if let Some(idx) = delete_attr {
            draft.attributes.remove(idx);
        }

        ui.add_space(8.0);

        // ── Operations section ───────────────────────────────────
        ui.label(format!("Operations ({}):", draft.operations.len()));
        let add_op = ui.button("+ Add Operation").clicked();
        if add_op {
            let next = self.generate_unique_name_in_draft(
                "operation",
                draft.operations.iter().map(|op| op.name.as_str()),
            );
            draft.operations.push(DraftOperation {
                name: next,
                return_type_text: String::new(),
                original_return_type: TypeReference::unspecified(),
                parameters: Vec::new(),
                visibility: Visibility::Public,
                is_static: false,
                is_abstract: false,
                is_virtual: false,
            });
        }

        let mut delete_op: Option<usize> = None;
        for (i, op) in draft.operations.iter_mut().enumerate() {
            ui.push_id(format!("op_{i}"), |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(format!("#{i}"));
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut op.name);
                        if ui.button("× Delete").clicked() {
                            delete_op = Some(i);
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Return:");
                        ui.text_edit_singleline(&mut op.return_type_text);
                        ui.label("Vis:");
                        let vis_options = [
                            Visibility::Public,
                            Visibility::Protected,
                            Visibility::Private,
                            Visibility::Implementation,
                        ];
                        let current_label = format!(
                            "{} {}",
                            visibility_symbol(op.visibility),
                            visibility_name(op.visibility)
                        );
                        egui::ComboBox::from_id_salt(format!("op_vis_{i}"))
                            .selected_text(current_label)
                            .show_ui(ui, |ui| {
                                for &vis in &vis_options {
                                    let label = format!(
                                        "{} {}",
                                        visibility_symbol(vis),
                                        visibility_name(vis)
                                    );
                                    if ui.selectable_label(op.visibility == vis, label).clicked() {
                                        op.visibility = vis;
                                    }
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut op.is_static, "Static");
                        ui.checkbox(&mut op.is_abstract, "Abstract");
                        ui.checkbox(&mut op.is_virtual, "Virtual");
                    });

                    // ── Parameters ─────────────────────────────
                    ui.add_space(4.0);
                    ui.label(format!("Parameters ({}):", op.parameters.len()));
                    let add_param = ui.button("+ Add Parameter").clicked();
                    if add_param {
                        let next = self.generate_unique_name_in_draft(
                            "parameter",
                            op.parameters.iter().map(|p| p.name.as_str()),
                        );
                        op.parameters.push(DraftParameter {
                            name: next,
                            type_text: String::new(),
                            original_type: TypeReference::unspecified(),
                            direction: uml_core::ParameterDirection::In,
                            default_value: String::new(),
                        });
                    }

                    let mut delete_param: Option<usize> = None;
                    for (j, param) in op.parameters.iter_mut().enumerate() {
                        ui.push_id(format!("param_{i}_{j}"), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("#{j}"));
                                ui.label("N:");
                                ui.text_edit_singleline(&mut param.name);
                                ui.label("T:");
                                ui.text_edit_singleline(&mut param.type_text);
                                ui.label("Dir:");
                                let dir_options = [
                                    uml_core::ParameterDirection::In,
                                    uml_core::ParameterDirection::Out,
                                    uml_core::ParameterDirection::InOut,
                                    uml_core::ParameterDirection::Return,
                                ];
                                let dir_label = param.direction.as_str();
                                egui::ComboBox::from_id_salt(format!("param_dir_{i}_{j}"))
                                    .selected_text(dir_label)
                                    .show_ui(ui, |ui| {
                                        for &dir in &dir_options {
                                            if ui
                                                .selectable_label(
                                                    param.direction == dir,
                                                    dir.as_str(),
                                                )
                                                .clicked()
                                            {
                                                param.direction = dir;
                                            }
                                        }
                                    });
                                ui.label("Def:");
                                ui.text_edit_singleline(&mut param.default_value);
                                if ui.button("×").clicked() {
                                    delete_param = Some(j);
                                }
                            });
                        });
                    }
                    if let Some(idx) = delete_param {
                        op.parameters.remove(idx);
                    }
                });
            });
        }
        if let Some(idx) = delete_op {
            draft.operations.remove(idx);
        }

        ui.add_space(4.0);

        // ── Apply / Revert buttons ──────────────────────────────
        ui.horizontal(|ui| {
            if ui.button("Apply").clicked() {
                match self.apply_classifier_draft(id, &draft) {
                    Ok(true) => {
                        status = "Classifier features updated".into();
                        // Restore from model after apply
                        self.refresh_property_buffers();
                    },
                    Ok(false) => {
                        status = "Classifier unchanged (no changes)".into();
                    },
                    Err(error) => {
                        status = format!("Apply failed: {error}");
                        // Restore draft in case of error
                        self.classifier_draft = Some((draft_id, draft));
                    },
                }
                self.status_message = status.clone();
                if status.contains("failed") {
                    // draft was restored in Err arm; do not overwrite
                } else {
                    return; // UI will re-render on next frame
                }
            }
            if ui.button("Revert").clicked() {
                self.refresh_property_buffers();
                self.status_message = "Classifier draft reverted".into();
            }
        });
    }
}

impl UmbrelloApp {
    /// Apply the classifier draft — validates names, converts type text,
    /// builds replacement ClassifierData, and executes UpdateClassifierFeatures.
    ///
    /// Returns `Ok(true)` if applied with changes, `Ok(false)` if no changes,
    /// `Err` with a description on validation failure.
    pub(crate) fn apply_classifier_draft(
        &mut self,
        id: uml_core::UmlId,
        draft: &crate::app::ClassifierDraft,
    ) -> Result<bool, String> {
        // Validate non-empty names
        for (i, attr) in draft.attributes.iter().enumerate() {
            if attr.name.trim().is_empty() {
                return Err(format!("Attribute {} has an empty name", i));
            }
        }
        for (i, op) in draft.operations.iter().enumerate() {
            if op.name.trim().is_empty() {
                return Err(format!("Operation {} has an empty name", i));
            }
            for (j, param) in op.parameters.iter().enumerate() {
                if param.name.trim().is_empty() {
                    return Err(format!("Operation {} parameter {} has an empty name", i, j));
                }
            }
        }

        // Build replacement ClassifierData
        let current_data = self
            .model
            .get(id)
            .and_then(|elem| elem.classifier_data().cloned())
            .ok_or_else(|| "selected element is not a classifier".to_string())?;

        let replacement = uml_core::ClassifierData {
            attributes: draft
                .attributes
                .iter()
                .map(|da| uml_core::Attribute {
                    name: da.name.clone(),
                    type_ref: self.resolve_draft_type(da.type_text.clone(), &da.original_type),
                    visibility: da.visibility,
                    initial_value: optional_text(&da.initial_value),
                    is_static: da.is_static,
                })
                .collect(),
            operations: draft
                .operations
                .iter()
                .map(|dop| {
                    let return_type = self.resolve_draft_type(
                        dop.return_type_text.clone(),
                        &dop.original_return_type,
                    );
                    uml_core::Operation {
                        name: dop.name.clone(),
                        return_type,
                        parameters: dop
                            .parameters
                            .iter()
                            .map(|dp| uml_core::Parameter {
                                name: dp.name.clone(),
                                type_ref: self
                                    .resolve_draft_type(dp.type_text.clone(), &dp.original_type),
                                direction: dp.direction,
                                default_value: optional_text(&dp.default_value),
                            })
                            .collect(),
                        visibility: dop.visibility,
                        is_static: dop.is_static,
                        is_abstract: dop.is_abstract,
                        is_virtual: dop.is_virtual,
                    }
                })
                .collect(),
            templates: current_data.templates.clone(),
        };

        if replacement == current_data {
            return Ok(false);
        }

        let command =
            uml_core::commands::UpdateClassifierFeatures::new(&self.model, id, replacement)
                .map_err(|error| error.to_string())?;
        self.execute_command_result(Box::new(command))
            .map_err(|error| error.to_string())?;
        Ok(true)
    }

    /// Resolve draft type text to a TypeReference.
    ///
    /// - If the text is empty, returns unspecified.
    /// - If the text matches the displayed name of the original type and the
    ///   original has a model_id (i.e., was never user-edited), preserves the
    ///   original model-backed reference.
    /// - Otherwise creates a primitive type reference from the text.
    fn resolve_draft_type(&self, type_text: String, original: &TypeReference) -> TypeReference {
        let text = type_text.trim().to_string();
        if text.is_empty() {
            return TypeReference::unspecified();
        }
        // The draft stores type_text as the displayed name (from display_name).
        // If type_text matches the original type_name, it was not edited -> keep original.
        if let Some(ref orig_name) = original.type_name {
            if *orig_name == text {
                return original.clone();
            }
        }
        // If original has model_id, look up its display name. If it matches,
        // the user didn't edit the field -> keep original.
        if let Some(model_id) = original.model_id {
            let orig_display = self
                .model
                .get(model_id)
                .map(|e| e.name().to_string())
                .unwrap_or_default();
            if orig_display == text {
                return original.clone();
            }
        }
        TypeReference::primitive(text)
    }

    pub(crate) fn generate_unique_name_in_draft<'a>(
        &self,
        base: &str,
        existing_names: impl Iterator<Item = &'a str>,
    ) -> String {
        let existing: std::collections::HashSet<&str> = existing_names.collect();
        let prefix = format!("{base}_");
        let mut suffixes: Vec<u64> = existing
            .iter()
            .filter_map(|name| name.strip_prefix(&prefix)?.parse().ok())
            .collect();
        suffixes.sort_unstable();
        let next = (1u64..)
            .find(|candidate| suffixes.binary_search(candidate).is_err())
            .unwrap_or(1);
        format!("{base}_{next}")
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
