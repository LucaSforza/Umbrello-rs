//! UI-thread target discovery and action dispatch.

use super::protocol::{QaError, QaRequest, QaResponse, UiSnapshot, UiTarget};
use crate::app::UmbrelloApp;
use crate::tool_palette::ToolMode;
use uml_core::{commands, ModelElement, UmlId, Visibility};

const TOOLS: &[(&str, ToolMode)] = &[
    ("select", ToolMode::Select),
    ("class", ToolMode::CreateClass),
    ("interface", ToolMode::CreateInterface),
    ("enum", ToolMode::CreateEnum),
    ("datatype", ToolMode::CreateDatatype),
    ("package", ToolMode::CreatePackage),
    ("actor", ToolMode::CreateActor),
    ("use_case", ToolMode::CreateUseCase),
    ("component", ToolMode::CreateComponent),
    ("node", ToolMode::CreateNode),
    ("artifact", ToolMode::CreateArtifact),
    ("generalization", ToolMode::CreateGeneralization),
    ("realization", ToolMode::CreateRealization),
    ("association", ToolMode::CreateAssociation),
    ("aggregation", ToolMode::CreateAggregation),
    ("composition", ToolMode::CreateComposition),
    ("dependency", ToolMode::CreateDependency),
];

impl UmbrelloApp {
    pub(crate) fn qa_snapshot(&self) -> UiSnapshot {
        let active_diagram = self
            .active_diagram
            .and_then(|i| self.model.diagrams().get(i))
            .map(|d| d.id.to_string());
        let active_data = self
            .active_diagram
            .and_then(|i| self.model.diagrams().get(i));
        let (zoom_percent, pan_x, pan_y) = active_data.map_or((None, None, None), |diagram| {
            let pan = self
                .viewport_pans
                .get(&diagram.id)
                .copied()
                .unwrap_or_default();
            (Some(diagram.zoom_percent()), Some(f64::from(pan.x)), Some(f64::from(pan.y)))
        });
        let mut targets = Vec::new();
        let mut add = |id: String,
                       kind: &str,
                       label: String,
                       enabled: bool,
                       _selected: bool,
                       element_id: Option<String>,
                       diagram_id: Option<String>| {
            let cursor_selected = self.selected_qa_target.as_deref() == Some(id.as_str());
            targets.push(UiTarget {
                id,
                kind: kind.into(),
                label,
                enabled,
                selected: cursor_selected,
                element_id,
                diagram_id,
            });
        };
        add(
            "history.undo".into(),
            "action",
            "Undo".into(),
            self.history.can_undo(),
            false,
            None,
            None,
        );
        add(
            "history.redo".into(),
            "action",
            "Redo".into(),
            self.history.can_redo(),
            false,
            None,
            None,
        );
        add("file.new".into(), "action", "New Project…".into(), true, false, None, None);
        add(
            "file.save".into(),
            "action",
            "Save".into(),
            self.current_file_path.is_some(),
            false,
            None,
            None,
        );
        add("app.quit".into(), "action", "Quit".into(), true, false, None, None);
        for &(name, tool) in TOOLS {
            add(
                format!("tool.{name}"),
                "tool",
                tool.label().to_string(),
                self.is_tool_available(tool),
                self.current_tool == tool,
                None,
                None,
            );
        }
        let can_create_diagram = self.current_file_path.is_some();
        add(
            "diagram.new".into(),
            "action",
            "New Diagram…".into(),
            can_create_diagram,
            false,
            None,
            None,
        );
        for (id, kind) in [
            ("diagram.new.class", uml_core::DiagramKind::Class),
            ("diagram.new.use_case", uml_core::DiagramKind::UseCase),
            ("diagram.new.component", uml_core::DiagramKind::Component),
            ("diagram.new.deployment", uml_core::DiagramKind::Deployment),
        ] {
            add(
                id.into(),
                "diagram",
                format!("New {} Diagram", kind.as_str()),
                can_create_diagram,
                false,
                None,
                None,
            );
        }
        add(
            "diagram.new_class".into(),
            "diagram",
            "New Class Diagram".into(),
            can_create_diagram,
            false,
            None,
            None,
        );
        for diagram in self.model.diagrams() {
            let id = diagram.id.to_string();
            add(
                format!("diagram:{id}"),
                "diagram",
                diagram.name.clone(),
                true,
                false,
                None,
                Some(id),
            );
        }
        add(
            "canvas".into(),
            "canvas",
            "Canvas".into(),
            active_diagram.is_some(),
            false,
            None,
            active_diagram.clone(),
        );
        for (id, label) in [
            ("viewport.zoom_in", "Zoom In"),
            ("viewport.zoom_out", "Zoom Out"),
            ("viewport.fit", "Fit Diagram"),
            ("viewport.reset", "Reset View"),
        ] {
            add(
                id.into(),
                "action",
                label.into(),
                active_diagram.is_some(),
                false,
                None,
                active_diagram.clone(),
            );
        }
        if let Some(diagram) = self
            .active_diagram
            .and_then(|i| self.model.diagrams().get(i))
        {
            for node in diagram.nodes.values().filter(|node| node.visible) {
                let id = node.model_element_id.to_string();
                add(
                    format!("node:{id}"),
                    "node",
                    self.model
                        .get(node.model_element_id)
                        .map_or_else(|| id.clone(), |e| e.name().to_string()),
                    true,
                    false,
                    Some(id),
                    Some(diagram.id.to_string()),
                );
            }
            for edge in diagram.edges.values() {
                let Some(ModelElement::Relationship(relationship)) =
                    self.model.get(edge.relationship_id)
                else {
                    continue;
                };
                let source = self.model.get(relationship.source_id).map_or_else(
                    || relationship.source_id.to_string(),
                    |element| element.name().to_string(),
                );
                let target = self.model.get(relationship.target_id).map_or_else(
                    || relationship.target_id.to_string(),
                    |element| element.name().to_string(),
                );
                let relationship_id = relationship.base.id.to_string();
                add(
                    format!("edge:{relationship_id}"),
                    "edge",
                    format!("{}: {source} → {target}", relationship.kind.as_str()),
                    true,
                    false,
                    Some(relationship_id),
                    Some(diagram.id.to_string()),
                );
            }
        }
        let semantic_elements: Vec<_> = self
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
        for (id, label) in semantic_elements {
            let id_string = id.to_string();
            add(
                format!("element:{id_string}"),
                "element",
                label,
                true,
                false,
                Some(id_string.clone()),
                None,
            );
            add(
                format!("element.add_to_diagram:{id_string}"),
                "action",
                "Add to active diagram".into(),
                self.add_to_diagram_state(id).is_ok(),
                false,
                Some(id_string),
                active_diagram.clone(),
            );
        }
        if let Some(selected) = self.selected_element_id {
            if self
                .model
                .get(selected)
                .is_some_and(|element| !matches!(element, ModelElement::Relationship(_)))
            {
                add(
                    "property.name".into(),
                    "property",
                    "Name".into(),
                    true,
                    false,
                    Some(selected.to_string()),
                    None,
                );
                add(
                    "property.documentation".into(),
                    "property",
                    "Documentation".into(),
                    true,
                    false,
                    Some(selected.to_string()),
                    None,
                );
                for value in ["public", "protected", "private", "implementation"] {
                    add(
                        format!("property.visibility.{value}"),
                        "property",
                        value.into(),
                        true,
                        false,
                        Some(selected.to_string()),
                        None,
                    );
                }
                if self
                    .model
                    .get(selected)
                    .is_some_and(|element| element.classifier_data().is_some())
                {
                    add(
                        "property.abstract".into(),
                        "property",
                        "Abstract".into(),
                        true,
                        self.model
                            .get(selected)
                            .is_some_and(|e| e.base().is_abstract),
                        Some(selected.to_string()),
                        None,
                    );
                    add(
                        "property.static".into(),
                        "property",
                        "Static".into(),
                        true,
                        self.model.get(selected).is_some_and(|e| e.base().is_static),
                        Some(selected.to_string()),
                        None,
                    );
                }
            }
            if self
                .model
                .get(selected)
                .is_some_and(|element| matches!(element, ModelElement::Relationship(_)))
            {
                let draft = self.relationship_draft.as_ref().map(|(_, draft)| draft);
                let fields = [
                    ("name", "Name"),
                    ("documentation", "Documentation"),
                    ("source_role", "Source role"),
                    ("source_multiplicity", "Source multiplicity"),
                    ("target_role", "Target role"),
                    ("target_multiplicity", "Target multiplicity"),
                ];
                for (field, label) in fields {
                    add(
                        format!("property.relationship.{field}"),
                        "property",
                        label.into(),
                        true,
                        false,
                        Some(selected.to_string()),
                        None,
                    );
                }
                for (kind, label) in [
                    ("association", "Association"),
                    ("generalization", "Generalization"),
                    ("realization", "Realization"),
                    ("aggregation", "Aggregation"),
                    ("composition", "Composition"),
                    ("dependency", "Dependency"),
                ] {
                    let association = match kind {
                        "association" => uml_core::AssociationType::Association,
                        "generalization" => uml_core::AssociationType::Generalization,
                        "realization" => uml_core::AssociationType::Realization,
                        "aggregation" => uml_core::AssociationType::Aggregation,
                        "composition" => uml_core::AssociationType::Composition,
                        _ => uml_core::AssociationType::Dependency,
                    };
                    add(
                        format!("property.relationship.kind.{kind}"),
                        "property",
                        label.into(),
                        self.relationship_kind_allowed(association)
                            || draft.is_some_and(|draft| draft.kind == association),
                        draft.is_some_and(|draft| draft.kind == association),
                        Some(selected.to_string()),
                        None,
                    );
                }
                for (field, label, selected_value) in [
                    (
                        "source_navigable",
                        "Source navigable",
                        draft.is_some_and(|draft| draft.source_navigable),
                    ),
                    (
                        "target_navigable",
                        "Target navigable",
                        draft.is_some_and(|draft| draft.target_navigable),
                    ),
                ] {
                    add(
                        format!("property.relationship.{field}"),
                        "property",
                        label.into(),
                        true,
                        selected_value,
                        Some(selected.to_string()),
                        None,
                    );
                }
                for (id, label) in [
                    ("property.relationship.apply", "Apply"),
                    ("property.relationship.revert", "Revert"),
                ] {
                    add(
                        id.into(),
                        "property",
                        label.into(),
                        true,
                        false,
                        Some(selected.to_string()),
                        None,
                    );
                }
            }
        }
        UiSnapshot {
            ready: true,
            ui_frame: self.ui_frame,
            state_revision: self.state_revision,
            rendered_revision: self.rendered_revision,
            active_tool: self.current_tool.label().into(),
            active_diagram,
            selected_element: self.selected_element_id.map(|id| id.to_string()),
            selected_qa_target: self.selected_qa_target.clone(),
            zoom_percent,
            pan_x,
            pan_y,
            status: self.status_message.clone(),
            targets,
        }
    }

    fn qa_target(&self, id: &str) -> Result<(), QaError> {
        let target = self
            .qa_snapshot()
            .targets
            .into_iter()
            .find(|target| target.id == id)
            .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
        if !target.enabled {
            return Err(QaError::UnavailableTarget(id.into()));
        }
        Ok(())
    }

    pub(crate) fn qa_select(&mut self, id: String) -> Result<UiSnapshot, QaError> {
        self.qa_target(&id)?;
        self.selected_qa_target = Some(id);
        self.bump_state();
        Ok(self.qa_snapshot())
    }

    pub(crate) fn qa_dispatch(
        &mut self,
        request: QaRequest,
        ctx: &egui::Context,
    ) -> Result<QaResponse, QaError> {
        match request {
            QaRequest::Inspect => Ok(QaResponse::Snapshot(self.qa_snapshot())),
            QaRequest::Select { target_id } => {
                let snapshot = self.qa_select(target_id)?;
                ctx.request_repaint();
                Ok(QaResponse::Accepted(snapshot))
            },
            QaRequest::Click { position } => {
                let target_id = self
                    .selected_qa_target
                    .clone()
                    .ok_or(QaError::UnavailableTarget("no selected QA target".into()))?;
                self.require_selected(&target_id)?;
                self.qa_click(&target_id, position, ctx)?;
                ctx.request_repaint();
                Ok(QaResponse::Accepted(self.qa_snapshot()))
            },
            QaRequest::SetText { value } => {
                let target_id = self
                    .selected_qa_target
                    .clone()
                    .ok_or(QaError::UnavailableTarget("no selected QA target".into()))?;
                self.require_selected(&target_id)?;
                self.qa_set_text(&target_id, value)?;
                ctx.request_repaint();
                Ok(QaResponse::Accepted(self.qa_snapshot()))
            },
            QaRequest::Drag {
                position,
                to_target,
                gesture,
            } => {
                let target_id = self
                    .selected_qa_target
                    .clone()
                    .ok_or(QaError::UnavailableTarget("no selected QA target".into()))?;
                self.require_selected(&target_id)?;
                self.qa_drag(&target_id, position, to_target, gesture)?;
                ctx.request_repaint();
                Ok(QaResponse::Accepted(self.qa_snapshot()))
            },
            QaRequest::Sync { after_revision } => {
                if self.rendered_revision < after_revision {
                    return Err(QaError::Timeout);
                }
                Ok(QaResponse::Snapshot(self.qa_snapshot()))
            },
            QaRequest::Screenshot => {
                Err(QaError::Screenshot("screenshot requests are handled by the frame pump".into()))
            },
        }
    }

    fn require_selected(&self, id: &str) -> Result<(), QaError> {
        if self.selected_qa_target.as_deref() != Some(id) {
            return Err(QaError::UnavailableTarget(id.into()));
        }
        self.qa_target(id)
    }

    fn qa_click(
        &mut self,
        id: &str,
        position: Option<(f64, f64)>,
        ctx: &egui::Context,
    ) -> Result<(), QaError> {
        self.require_selected(id)?;
        if id == "history.undo" {
            return self.qa_undo();
        }
        if id == "history.redo" {
            return self.qa_redo();
        }
        if id == "file.new" {
            return if self.menu_file_new() {
                Ok(())
            } else {
                Err(QaError::UnavailableTarget(id.into()))
            };
        }
        if id == "file.save" {
            if self.current_file_path.is_none() {
                return Err(QaError::UnavailableTarget(id.into()));
            }
            self.save_current()
                .map_err(|e| QaError::Command(e.to_string()))?;
            return Ok(());
        }
        if id == "app.quit" {
            if !self.prompt_save_if_dirty() {
                return Err(QaError::UnavailableTarget(id.into()));
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return Ok(());
        }
        if id == "viewport.zoom_in" {
            self.adjust_zoom(5.0);
            self.bump_state();
            return Ok(());
        }
        if id == "viewport.zoom_out" {
            self.adjust_zoom(-5.0);
            self.bump_state();
            return Ok(());
        }
        if id == "viewport.reset" {
            self.reset_viewport();
            self.bump_state();
            return Ok(());
        }
        if id == "viewport.fit" {
            let rect = self.last_canvas_rect.ok_or(QaError::NotReady)?;
            self.fit_active_diagram(rect);
            self.bump_state();
            return Ok(());
        }
        if let Some(name) = id.strip_prefix("tool.") {
            let (_, tool) = TOOLS
                .iter()
                .find(|(candidate, _)| *candidate == name)
                .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
            self.choose_tool(*tool).map_err(QaError::Command)?;
            return Ok(());
        }
        if let Some(raw) = id.strip_prefix("element.add_to_diagram:") {
            let element_id = raw
                .parse()
                .map_err(|_| QaError::UnavailableTarget(id.into()))?;
            self.add_element_to_active_diagram(element_id)
                .map_err(QaError::Command)?;
            return Ok(());
        }
        if let Some(raw) = id.strip_prefix("element:") {
            let element_id = raw
                .parse()
                .map_err(|_| QaError::UnavailableTarget(id.into()))?;
            self.select_element(element_id)?;
            return Ok(());
        }
        if id == "diagram.new" {
            self.open_new_diagram_dialog();
            return Ok(());
        }
        if id == "diagram.new_class" || id == "diagram.new.class" {
            self.create_supported_diagram(uml_core::DiagramKind::Class)?;
            return Ok(());
        }
        if id == "diagram.new.use_case" {
            self.create_supported_diagram(uml_core::DiagramKind::UseCase)?;
            return Ok(());
        }
        if id == "diagram.new.component" {
            self.create_supported_diagram(uml_core::DiagramKind::Component)?;
            return Ok(());
        }
        if id == "diagram.new.deployment" {
            self.create_supported_diagram(uml_core::DiagramKind::Deployment)?;
            return Ok(());
        }
        if let Some(raw) = id.strip_prefix("diagram:") {
            let index = self
                .model
                .diagrams()
                .iter()
                .position(|diagram| diagram.id.to_string() == raw)
                .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
            self.activate_diagram_index(index);
            return Ok(());
        }
        if id == "canvas" {
            let (x, y) = position.ok_or(QaError::InvalidCoordinates)?;
            if !x.is_finite() || !y.is_finite() {
                return Err(QaError::InvalidCoordinates);
            }
            if self.current_tool.is_creation_tool() {
                self.place_element(self.current_tool, uml_core::Point::new(x, y))
                    .map_err(QaError::Command)?;
                self.choose_tool(ToolMode::Select)
                    .map_err(QaError::Command)?;
                return Ok(());
            }
            if self.current_tool == ToolMode::Select {
                return Err(QaError::WrongTargetKind(id.into()));
            }
            return Err(QaError::WrongTargetKind(id.into()));
        }
        if let Some(raw) = id.strip_prefix("node:") {
            let element = raw
                .parse()
                .map_err(|_| QaError::UnavailableTarget(id.into()))?;
            self.select_element(element)?;
            return Ok(());
        }
        if let Some(raw) = id.strip_prefix("edge:") {
            let relationship_id = raw
                .parse()
                .map_err(|_| QaError::UnavailableTarget(id.into()))?;
            self.select_element(relationship_id)?;
            return Ok(());
        }
        if id == "property.name" || id == "property.documentation" {
            return Ok(());
        }
        if id == "property.relationship.apply" {
            let selected = self
                .selected_element_id
                .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
            match self.apply_relationship_draft(selected) {
                Ok(true) => self.status_message = "Relationship updated".into(),
                Ok(false) => self.status_message = "Relationship unchanged (no changes)".into(),
                Err(error) => {
                    self.status_message = format!("Relationship apply failed: {error}");
                    return Err(QaError::Command(error));
                },
            }
            return Ok(());
        }
        if id == "property.relationship.revert" {
            self.refresh_property_buffers();
            self.status_message = "Relationship draft reverted".into();
            return Ok(());
        }
        if let Some(kind) = id.strip_prefix("property.relationship.kind.") {
            let kind = match kind {
                "association" => uml_core::AssociationType::Association,
                "generalization" => uml_core::AssociationType::Generalization,
                "realization" => uml_core::AssociationType::Realization,
                "aggregation" => uml_core::AssociationType::Aggregation,
                "composition" => uml_core::AssociationType::Composition,
                "dependency" => uml_core::AssociationType::Dependency,
                _ => return Err(QaError::UnavailableTarget(id.into())),
            };
            if !self.relationship_kind_allowed(kind)
                && self
                    .relationship_draft
                    .as_ref()
                    .is_none_or(|(_, draft)| draft.kind != kind)
            {
                return Err(QaError::UnavailableTarget(id.into()));
            }
            if let Some((_, draft)) = self.relationship_draft.as_mut() {
                draft.kind = kind;
            }
            self.bump_state();
            return Ok(());
        }
        for (suffix, source) in [("source_navigable", true), ("target_navigable", false)] {
            if id == format!("property.relationship.{suffix}") {
                if let Some((_, draft)) = self.relationship_draft.as_mut() {
                    if source {
                        draft.source_navigable = !draft.source_navigable;
                    } else {
                        draft.target_navigable = !draft.target_navigable;
                    }
                    self.bump_state();
                    return Ok(());
                }
            }
        }
        if let Some(value) = id.strip_prefix("property.visibility.") {
            let visibility = match value {
                "public" => Visibility::Public,
                "protected" => Visibility::Protected,
                "private" => Visibility::Private,
                "implementation" => Visibility::Implementation,
                _ => return Err(QaError::UnavailableTarget(id.into())),
            };
            let selected = self
                .selected_element_id
                .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
            self.set_visibility(selected, visibility)?;
            return Ok(());
        }
        for (name, is_abstract, is_static) in [
            ("property.abstract", true, false),
            ("property.static", false, true),
        ] {
            if id == name {
                let selected = self
                    .selected_element_id
                    .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
                let elem = self
                    .model
                    .get(selected)
                    .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
                if elem.classifier_data().is_none() {
                    return Err(QaError::UnavailableTarget(id.into()));
                }
                self.set_flags(
                    selected,
                    if is_abstract {
                        !elem.base().is_abstract
                    } else {
                        elem.base().is_abstract
                    },
                    if is_static {
                        !elem.base().is_static
                    } else {
                        elem.base().is_static
                    },
                )?;
                return Ok(());
            }
        }
        Err(QaError::UnavailableTarget(id.into()))
    }

    fn qa_set_text(&mut self, id: &str, value: String) -> Result<(), QaError> {
        self.require_selected(id)?;
        if id == "file.new" {
            if self.is_dirty {
                return Err(QaError::InvalidValue(
                    "save the current project before creating a new project".into(),
                ));
            }
            let path = std::path::PathBuf::from(value.trim());
            if !path.is_absolute()
                || path.extension().and_then(|extension| extension.to_str()) != Some("xmi")
            {
                return Err(QaError::InvalidValue(
                    "file.new requires an absolute .xmi path".into(),
                ));
            }
            self.new_project_at(&path)
                .map_err(|error| QaError::Command(error.to_string()))?;
            return Ok(());
        }
        let selected = self
            .selected_element_id
            .ok_or_else(|| QaError::UnavailableTarget(id.into()))?;
        match id {
            "property.name" => self.rename_element(selected, value),
            "property.documentation" => self.set_documentation(selected, value),
            "property.relationship.name" => {
                self.set_relationship_draft_text(|draft| draft.name = value)
            },
            "property.relationship.documentation" => {
                self.set_relationship_draft_text(|draft| draft.documentation = value)
            },
            "property.relationship.source_role" => {
                self.set_relationship_draft_text(|draft| draft.source_role = value)
            },
            "property.relationship.source_multiplicity" => {
                self.set_relationship_draft_text(|draft| draft.source_multiplicity = value)
            },
            "property.relationship.target_role" => {
                self.set_relationship_draft_text(|draft| draft.target_role = value)
            },
            "property.relationship.target_multiplicity" => {
                self.set_relationship_draft_text(|draft| draft.target_multiplicity = value)
            },
            _ => Err(QaError::UnavailableTarget(id.into())),
        }
    }

    fn set_relationship_draft_text<F>(&mut self, update: F) -> Result<(), QaError>
    where
        F: FnOnce(&mut crate::app::RelationshipDraft),
    {
        let Some((_, draft)) = self.relationship_draft.as_mut() else {
            return Err(QaError::UnavailableTarget("relationship draft".into()));
        };
        update(draft);
        self.bump_state();
        Ok(())
    }

    fn qa_drag(
        &mut self,
        id: &str,
        position: Option<(f64, f64)>,
        to_target: Option<String>,
        gesture: Option<bool>,
    ) -> Result<(), QaError> {
        self.require_selected(id)?;
        if id == "canvas" {
            if self.current_tool != ToolMode::Select {
                return Err(QaError::WrongTargetKind(id.into()));
            }
            let (x, y) = position.ok_or(QaError::InvalidCoordinates)?;
            if !x.is_finite() || !y.is_finite() {
                return Err(QaError::InvalidCoordinates);
            }
            let (pan_x, pan_y) = (x as f32, y as f32);
            if !pan_x.is_finite() || !pan_y.is_finite() {
                return Err(QaError::InvalidCoordinates);
            }
            let diagram_id = self
                .active_diagram
                .and_then(|i| self.model.diagrams().get(i))
                .ok_or(QaError::NotReady)?
                .id;
            let current_pan = self
                .viewport_pans
                .get(&diagram_id)
                .copied()
                .unwrap_or_default();
            let next_x = current_pan.x + pan_x;
            let next_y = current_pan.y + pan_y;
            if !next_x.is_finite() || !next_y.is_finite() {
                return Err(QaError::InvalidCoordinates);
            }
            self.viewport_pans
                .insert(diagram_id, egui::vec2(next_x, next_y));
            self.bump_state();
            return Ok(());
        }
        let source = id
            .strip_prefix("node:")
            .ok_or_else(|| QaError::WrongTargetKind(id.into()))?
            .parse()
            .map_err(|_| QaError::UnavailableTarget(id.into()))?;
        if let Some(target_id) = to_target {
            let target = target_id
                .strip_prefix("node:")
                .ok_or_else(|| QaError::WrongTargetKind(target_id.clone()))?
                .parse()
                .map_err(|_| QaError::UnavailableTarget(target_id.clone()))?;
            self.place_edge(source, target).map_err(QaError::Command)?;
            self.choose_tool(ToolMode::Select)
                .map_err(QaError::Command)?;
            return Ok(());
        }
        let (x, y) = position.ok_or(QaError::InvalidCoordinates)?;
        if !x.is_finite() || !y.is_finite() {
            return Err(QaError::InvalidCoordinates);
        }
        let diagram = self
            .active_diagram
            .and_then(|i| self.model.diagrams().get(i))
            .ok_or(QaError::NotReady)?
            .id;
        // When gesture mode is enabled, use the shared gesture simulation that
        // exercises the begin → preview → commit control flow instead of
        // directly calling move_node_to.
        if gesture.unwrap_or(false) {
            self.execute_gesture_move(diagram, source, uml_core::Point::new(x, y))
        } else {
            self.move_node_to(diagram, source, uml_core::Point::new(x, y))
        }
    }

    pub(crate) fn move_node_to(
        &mut self,
        diagram: uml_core::DiagramId,
        node: UmlId,
        position: uml_core::Point,
    ) -> Result<(), QaError> {
        let cmd = commands::MoveNode::new(&self.model, diagram, node, position)
            .map_err(|e| QaError::Command(e.to_string()))?;
        self.execute_command_result(Box::new(cmd))
    }

    pub(crate) fn undo_action(&mut self) -> Result<(), QaError> {
        if !self.history.can_undo() {
            return Err(QaError::UnavailableTarget("history.undo".into()));
        }
        let prior_active = self
            .active_diagram
            .and_then(|index| self.model.diagrams().get(index).map(|diagram| diagram.id));
        let prior_index = self.active_diagram;
        self.history
            .undo(&mut self.model)
            .map_err(|e| QaError::Command(e.to_string()))?;
        self.active_diagram = prior_active.and_then(|id| {
            self.model
                .diagrams()
                .iter()
                .position(|diagram| diagram.id == id)
        });
        if self.active_diagram.is_none() {
            self.active_diagram = prior_index
                .map(|index| index.min(self.model.diagrams().len().saturating_sub(1)))
                .filter(|_| !self.model.diagrams().is_empty());
        }
        self.normalize_transient_state();
        self.refresh_name_edit_buffer();
        self.is_dirty = true;
        self.status_message = "Undo".into();
        self.bump_state();
        Ok(())
    }

    pub(crate) fn rename_element(&mut self, id: UmlId, value: String) -> Result<(), QaError> {
        let canonical = value.trim().to_string();
        if canonical.is_empty() {
            return Err(QaError::InvalidValue("name cannot be empty".into()));
        }
        let cmd = commands::RenameElement::new(&self.model, id, canonical.clone())
            .map_err(|e| QaError::Command(e.to_string()))?;
        self.execute_command_result(Box::new(cmd))?;
        if self.selected_element_id == Some(id) {
            self.name_edit_buffer = canonical;
        }
        Ok(())
    }
    pub(crate) fn set_documentation(&mut self, id: UmlId, value: String) -> Result<(), QaError> {
        let cmd = commands::ChangeDocumentation::new(&self.model, id, value)
            .map_err(|e| QaError::Command(e.to_string()))?;
        self.execute_command_result(Box::new(cmd))
    }
    pub(crate) fn set_visibility(&mut self, id: UmlId, value: Visibility) -> Result<(), QaError> {
        let cmd = commands::ChangeVisibility::new(&self.model, id, value)
            .map_err(|e| QaError::Command(e.to_string()))?;
        self.execute_command_result(Box::new(cmd))
    }
    pub(crate) fn set_flags(
        &mut self,
        id: UmlId,
        is_abstract: bool,
        is_static: bool,
    ) -> Result<(), QaError> {
        let cmd = commands::ChangeElementFlags::new(&self.model, id, is_abstract, is_static)
            .map_err(|e| QaError::Command(e.to_string()))?;
        self.execute_command_result(Box::new(cmd))
    }
    pub(crate) fn redo_action(&mut self) -> Result<(), QaError> {
        if !self.history.can_redo() {
            return Err(QaError::UnavailableTarget("history.redo".into()));
        }
        let prior_active = self
            .active_diagram
            .and_then(|index| self.model.diagrams().get(index).map(|diagram| diagram.id));
        let diagram_ids_before: std::collections::HashSet<_> = self
            .model
            .diagrams()
            .iter()
            .map(|diagram| diagram.id)
            .collect();
        self.history
            .redo(&mut self.model)
            .map_err(|e| QaError::Command(e.to_string()))?;
        self.active_diagram = prior_active.and_then(|id| {
            self.model
                .diagrams()
                .iter()
                .position(|diagram| diagram.id == id)
        });
        if self.active_diagram.is_none() {
            self.active_diagram = self
                .model
                .diagrams()
                .iter()
                .position(|diagram| !diagram_ids_before.contains(&diagram.id));
        }
        self.normalize_transient_state();
        self.refresh_name_edit_buffer();
        self.is_dirty = true;
        self.status_message = "Redo".into();
        self.bump_state();
        Ok(())
    }

    fn qa_undo(&mut self) -> Result<(), QaError> {
        self.undo_action()
    }
    fn qa_redo(&mut self) -> Result<(), QaError> {
        self.redo_action()
    }
    pub(crate) fn choose_tool(&mut self, tool: ToolMode) -> Result<(), String> {
        if !self.is_tool_available(tool) {
            self.status_message = self.tool_unavailable_reason(tool).into();
            return Err(self.status_message.clone());
        }
        self.current_tool = tool;
        self.preview_position = None;
        self.drag_source_node_id = None;
        self.bump_state();
        Ok(())
    }
    pub(crate) fn activate_diagram_index(&mut self, index: usize) {
        if self.model.diagrams().get(index).is_none() {
            self.status_message = "Diagram is unavailable".into();
            return;
        }
        self.active_diagram = Some(index);
        if !self.is_tool_available(self.current_tool) {
            self.current_tool = ToolMode::Select;
            self.preview_position = None;
            self.drag_source_node_id = None;
            self.drag_node_id = None;
            self.drag_start_pos = None;
        }
        self.bump_state();
    }
    pub(crate) fn select_element(&mut self, id: UmlId) -> Result<(), QaError> {
        if self.model.get(id).is_none() {
            return Err(QaError::UnavailableTarget(format!("node:{id}")));
        }
        self.selected_element_id = Some(id);
        self.refresh_property_buffers();
        self.bump_state();
        Ok(())
    }

    pub(crate) fn add_element_to_active_diagram(&mut self, id: UmlId) -> Result<(), String> {
        if self.current_file_path.is_none() {
            return Err("Create or open an XMI project first".into());
        }
        let diagram_index = self
            .active_diagram
            .ok_or_else(|| "No active diagram".to_string())?;
        let diagram = self
            .model
            .diagrams()
            .get(diagram_index)
            .ok_or_else(|| "Active diagram is unavailable".to_string())?;
        let element = self
            .model
            .get(id)
            .ok_or_else(|| format!("Element {id} is unavailable"))?;
        if matches!(element, ModelElement::Relationship(_)) {
            return Err("Relationships cannot be added as nodes".into());
        }
        if !crate::tool_palette::element_is_compatible_with_diagram(element, diagram.kind) {
            return Err(format!(
                "{} is not compatible with {} diagrams",
                element.object_type().as_str(),
                diagram.kind.as_str()
            ));
        }
        if diagram.get_node(id).is_some() {
            return Err("Element is already present on the active diagram".into());
        }
        let node_count = diagram.nodes.len();
        let position = uml_core::Point::new(
            50.0 + f64::from((node_count % 4) as u32) * 220.0,
            50.0 + f64::from((node_count / 4) as u32) * 120.0,
        );
        let command = commands::AddNodeToDiagram::new(
            diagram.id,
            id,
            position,
            uml_core::Size::new(160.0, 60.0),
        );
        self.execute_command_result(Box::new(command))
            .map_err(|error| error.to_string())
    }
    fn refresh_name_edit_buffer(&mut self) {
        self.refresh_property_buffers();
    }
    pub(crate) fn create_supported_diagram(
        &mut self,
        kind: uml_core::DiagramKind,
    ) -> Result<uml_core::DiagramId, QaError> {
        let name = self.unique_diagram_name(kind);
        self.create_diagram(kind, name).map_err(QaError::Command)
    }
}
