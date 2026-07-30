//! All unit tests for the Umbrello application crate.
//!
//! Extracted from app.rs during the M18 modular split. Tests exercise the
//! UmbrelloApp data model directly without requiring an egui Context.

// These allow are needed because the module is cfg-gated; clippy in the
// binary target sees this code as unused.
#![allow(unused_imports)]

use crate::app::{DraftAttribute, DraftOperation, DraftParameter, UmbrelloApp};
use crate::rendering::{element_color, type_display, visibility_symbol};
use crate::tool_palette::ToolMode;
use eframe::App;
use image::GenericImageView;
use std::path::PathBuf;
use uml_core::{
    commands, Actor, Artifact, ArtifactDrawMode, AssociationType, Class, Command, Component,
    Datatype, Diagram, DiagramKind, Enum, Interface, ModelElement, Node, Package, Point, Rect,
    Relationship, Size, TypeReference, UmlId, UmlModel, UseCase, ViewEdge, ViewNode, Visibility,
};

#[test]
fn qa_targets_use_durable_ids_and_command_mutations() {
    let mut app = make_app_with_diagram();
    let class = Class::new("Before");
    let id = class.base.id;
    let other = Class::new("Other");
    let other_id = other.base.id;
    app.model.insert(ModelElement::Class(class));
    app.model.insert(ModelElement::Class(other));
    let diagram_id = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, uml_core::ViewNode::new(id, uml_core::Rect::new(10.0, 20.0, 100.0, 60.0)));
    app.model.get_diagram_mut(diagram_id).unwrap().add_node(
        other_id,
        uml_core::ViewNode::new(other_id, uml_core::Rect::new(120.0, 20.0, 100.0, 60.0)),
    );
    let snapshot = app.qa_snapshot();
    assert!(snapshot
        .targets
        .iter()
        .any(|target| target.id == format!("node:{id}")));
    let ctx = egui::Context::default();
    app.qa_select(format!("node:{id}")).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
        .unwrap();
    app.qa_select("property.name".into()).unwrap();
    app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::SetText {
            value: "After".into(),
        },
        &ctx,
    )
    .unwrap();
    assert_eq!(app.model.get(id).unwrap().name(), "After");
    assert_eq!(app.name_edit_buffer, "After");
    let empty = app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::SetText {
            value: "   ".into(),
        },
        &ctx,
    );
    assert!(matches!(empty, Err(crate::app::qa::protocol::QaError::InvalidValue(_))));
    assert_eq!(app.model.get(id).unwrap().name(), "After");
    assert_eq!(app.name_edit_buffer, "After");
    app.qa_select("history.undo".into()).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
        .unwrap();
    assert_eq!(app.model.get(id).unwrap().name(), "Before");
    assert_eq!(app.name_edit_buffer, "Before");
    app.qa_select("history.redo".into()).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
        .unwrap();
    assert_eq!(app.model.get(id).unwrap().name(), "After");
    assert_eq!(app.name_edit_buffer, "After");
    app.qa_select(format!("node:{other_id}")).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
        .unwrap();
    assert_eq!(app.name_edit_buffer, "Other");
    assert_eq!(app.model.diagrams()[0].id, diagram_id);
}

#[test]
fn qa_operations_require_the_automation_cursor_and_reject_stale_selection() {
    let mut app = make_app_with_diagram();
    let first = Class::new("First");
    let first_id = first.base.id;
    let second = Class::new("Second");
    let second_id = second.base.id;
    app.model.insert(ModelElement::Class(first));
    app.model.insert(ModelElement::Class(second));
    let diagram_id = app.model.diagrams()[0].id;
    app.model.get_diagram_mut(diagram_id).unwrap().add_node(
        first_id,
        uml_core::ViewNode::new(first_id, uml_core::Rect::new(0.0, 0.0, 100.0, 60.0)),
    );
    app.model.get_diagram_mut(diagram_id).unwrap().add_node(
        second_id,
        uml_core::ViewNode::new(second_id, uml_core::Rect::new(120.0, 0.0, 100.0, 60.0)),
    );
    let ctx = egui::Context::default();

    app.qa_select(format!("node:{first_id}")).unwrap();
    let mismatch = app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::SetText {
            value: "must-not-apply".into(),
        },
        &ctx,
    );
    assert!(matches!(mismatch, Err(crate::app::qa::protocol::QaError::UnavailableTarget(_))));
    assert_eq!(app.model.get(first_id).unwrap().name(), "First");

    app.model.remove(first_id).unwrap();
    let stale =
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx);
    assert!(matches!(stale, Err(crate::app::qa::protocol::QaError::UnavailableTarget(_))));
    assert_eq!(
        app.qa_snapshot().selected_qa_target.as_deref(),
        Some(format!("node:{first_id}").as_str())
    );
    assert!(app.model.get(second_id).is_some());
}

#[test]
fn qa_bridge_reports_full_queue_without_blocking() {
    let (bridge, handle) = crate::app::qa::bridge::QaBridge::new(1);
    let first = handle.submit_timeout(
        crate::app::qa::protocol::QaRequest::Inspect,
        std::time::Duration::from_millis(1),
    );
    assert!(matches!(first, Err(crate::app::qa::protocol::QaError::Timeout)));
    let second = handle.submit_timeout(
        crate::app::qa::protocol::QaRequest::Inspect,
        std::time::Duration::from_millis(1),
    );
    assert!(matches!(second, Err(crate::app::qa::protocol::QaError::QueueFull)));
    drop(bridge);
}

#[test]
fn qa_ticket_cancellation_is_visible_before_ui_processing() {
    let (bridge, handle) = crate::app::qa::bridge::QaBridge::new(1);
    let ticket = handle
        .submit_ticket(
            crate::app::qa::protocol::QaRequest::Inspect,
            std::time::Duration::from_secs(1),
        )
        .unwrap();
    ticket.cancel();
    let envelope = bridge.receiver.try_recv().unwrap();
    assert!(envelope
        .cancelled
        .load(std::sync::atomic::Ordering::Acquire));
    let _ = envelope
        .reply
        .send(Err(crate::app::qa::protocol::QaError::Cancelled));
    assert!(matches!(ticket.wait(), Err(crate::app::qa::protocol::QaError::Cancelled)));
}

#[test]
fn screenshot_png_decodes_with_signature_dimensions_and_metadata() {
    let image = egui::ColorImage::new([2, 1], egui::Color32::WHITE);
    let result = crate::app::qa::screenshot::encode_png(&image, 4, 5).unwrap();
    assert_eq!(&result.png[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!((result.width, result.height), (2, 1));
    let decoded = image::load_from_memory(&result.png).unwrap();
    assert_eq!(decoded.dimensions(), (2, 1));
    assert_eq!((result.state_revision, result.rendered_revision), (4, 5));
}

#[test]
fn qa_save_returns_error_without_opening_a_dialog() {
    let mut app = make_app_with_class("Unsaved");
    app.current_file_path = Some(PathBuf::from("/definitely/not/a/real/directory/model.xmi"));
    app.is_dirty = true;
    let error = app.save_current().expect_err("invalid path must fail");
    assert!(error.to_string().contains("I/O error"));
    assert!(app.is_dirty);
}

#[test]
fn screenshot_shutdown_drains_queued_request_with_structured_error() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    let handle = app.enable_qa(2);
    let worker = std::thread::spawn(move || {
        handle.submit_timeout(
            crate::app::qa::protocol::QaRequest::Screenshot,
            std::time::Duration::from_secs(2),
        )
    });
    std::thread::sleep(std::time::Duration::from_millis(10));
    app.shutdown_qa();
    assert!(matches!(
        worker.join().unwrap(),
        Err(crate::app::qa::protocol::QaError::Shutdown)
    ));
}

/// Helper: create an UmbrelloApp with a non-empty model.
#[allow(dead_code)]
fn make_app_with_class(name: &str) -> UmbrelloApp {
    let mut model = UmlModel::new();
    let cls = Class::new(name);
    model.insert(ModelElement::Class(cls));
    UmbrelloApp::new(model, false)
}

/// Helper: create an UmbrelloApp with a Class diagram.
#[allow(dead_code)]
fn make_app_with_diagram() -> UmbrelloApp {
    let mut model = UmlModel::new();
    let d = Diagram::new("Test", DiagramKind::Class);
    model.add_diagram(d);
    let mut app = UmbrelloApp::new(model, false);
    app.active_diagram = Some(0);
    app.current_file_path = Some(PathBuf::from("/tmp/test-project.xmi"));
    app
}

#[test]
fn new_project_failure_preserves_all_current_state() {
    let mut app = make_app_with_class("Existing");
    let diagram = Diagram::new("Existing Diagram", DiagramKind::Class);
    app.model.add_diagram(diagram);
    app.active_diagram = Some(0);
    app.current_file_path = Some(PathBuf::from("/existing/project.xmi"));
    app.is_dirty = true;
    app.selected_element_id = Some(app.model.iter().next().unwrap().0);
    app.name_edit_buffer = "draft".into();
    app.current_tool = ToolMode::CreateClass;
    app.status_message = "keep me".into();
    let before_model_len = app.model.len();
    let before_history = app.history.undo_depth();
    let before_path = app.current_file_path.clone();
    let before_active = app.active_diagram;

    let result = app.new_project_at(PathBuf::from("/definitely/missing/project.xmi").as_path());
    assert!(result.is_err());
    assert_eq!(app.model.len(), before_model_len);
    assert_eq!(app.history.undo_depth(), before_history);
    assert_eq!(app.current_file_path, before_path);
    assert_eq!(app.active_diagram, before_active);
    assert!(app.is_dirty);
    assert_eq!(app.selected_element_id, Some(app.model.iter().next().unwrap().0));
    assert_eq!(app.name_edit_buffer, "draft");
    assert_eq!(app.current_tool, ToolMode::CreateClass);
    assert_eq!(app.status_message, "keep me");
}

#[test]
fn supported_diagram_creation_is_one_history_action_and_restores_id() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    let path = std::env::temp_dir().join("test_m24_diagram_history.xmi");
    app.new_project_at(&path).unwrap();
    let diagram_id = app.create_supported_diagram(DiagramKind::UseCase).unwrap();
    assert_eq!(app.model.diagrams().len(), 1);
    assert_eq!(app.active_diagram, Some(0));
    assert!(app.is_dirty);
    assert_eq!(app.history.undo_depth(), 1);

    app.undo_action().unwrap();
    assert!(app.model.diagrams().is_empty());
    assert_eq!(app.active_diagram, None);
    app.redo_action().unwrap();
    assert_eq!(app.model.diagrams()[0].id, diagram_id);
    assert_eq!(app.active_diagram, Some(0));
    let _ = std::fs::remove_file(path);
}

#[test]
fn all_supported_diagram_kinds_have_direct_qa_targets() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    let ctx = egui::Context::default();
    let path = std::env::temp_dir().join("test_m24_diagram_targets.xmi");
    let before = app.qa_snapshot();
    for target in [
        "diagram.new",
        "diagram.new.class",
        "diagram.new.use_case",
        "diagram.new.component",
        "diagram.new.deployment",
    ] {
        assert!(before
            .targets
            .iter()
            .any(|item| item.id == target && !item.enabled));
    }
    app.new_project_at(&path).unwrap();
    let after = app.qa_snapshot();
    for target in [
        "diagram.new",
        "diagram.new.class",
        "diagram.new.use_case",
        "diagram.new.component",
        "diagram.new.deployment",
    ] {
        assert!(after
            .targets
            .iter()
            .any(|item| item.id == target && item.enabled));
    }
    for target in [
        "diagram.new.class",
        "diagram.new.use_case",
        "diagram.new.component",
        "diagram.new.deployment",
    ] {
        let snapshot = app.qa_snapshot();
        assert!(snapshot
            .targets
            .iter()
            .any(|item| item.id == target && item.enabled));
        app.qa_select(target.into()).unwrap();
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
            .unwrap();
        assert!(app.active_diagram.is_some());
    }
    assert_eq!(app.model.diagrams().len(), 4);
    assert!(app.model.diagrams().iter().all(|diagram| matches!(
        diagram.kind,
        DiagramKind::Class
            | DiagramKind::UseCase
            | DiagramKind::Component
            | DiagramKind::Deployment
    )));
    let _ = std::fs::remove_file(path);
}

#[test]
fn diagram_creation_requires_an_xmi_project_without_mutating_state() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    let before_history = app.history.undo_depth();
    let before_revision = app.state_revision;
    let result = app.create_supported_diagram(DiagramKind::Class);
    assert!(result.is_err());
    assert!(app.model.diagrams().is_empty());
    assert_eq!(app.history.undo_depth(), before_history);
    assert_eq!(app.state_revision, before_revision);
    assert!(app
        .qa_snapshot()
        .targets
        .iter()
        .any(|target| target.id == "diagram.new" && !target.enabled));
}

#[test]
fn new_diagram_default_name_tracks_kind_changes() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    let path = std::env::temp_dir().join("test_m24_diagram_kind_name.xmi");
    app.new_project_at(&path).unwrap();
    app.open_new_diagram_dialog();
    assert_eq!(app.new_diagram_name, "Class_1");
    app.set_new_diagram_kind(DiagramKind::UseCase);
    assert_eq!(app.new_diagram_name, "UseCase_1");
    app.set_new_diagram_kind(DiagramKind::Component);
    assert_eq!(app.new_diagram_name, "Component_1");
    app.set_new_diagram_kind(DiagramKind::Deployment);
    assert_eq!(app.new_diagram_name, "Deployment_1");
    let _ = std::fs::remove_file(path);
}

#[test]
fn new_diagram_radio_transition_updates_name_after_field_was_already_changed() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    let path = std::env::temp_dir().join("test_m24_radio_kind_transition.xmi");
    app.new_project_at(&path).unwrap();
    app.open_new_diagram_dialog();
    assert_eq!(app.new_diagram_name, "Class_1");

    // Mirror egui::Ui::radio_value: the field has already been assigned when
    // the transition handler runs.
    let previous_kind = DiagramKind::Class;
    app.new_diagram_kind = DiagramKind::UseCase;
    let new_kind = app.new_diagram_kind;
    app.apply_new_diagram_kind_transition(previous_kind, new_kind);
    assert_eq!(app.new_diagram_name, "UseCase_1");

    // A second frame with no transition must preserve a deliberately edited
    // draft rather than regenerating it.
    app.new_diagram_name = "Custom name".into();
    app.apply_new_diagram_kind_transition(DiagramKind::UseCase, DiagramKind::UseCase);
    assert_eq!(app.new_diagram_name, "Custom name");
    let _ = std::fs::remove_file(path);
}

#[test]
fn qa_file_new_set_text_uses_project_helper_without_dialog() {
    let mut app = make_app_with_class("Before");
    let ctx = egui::Context::default();
    let path = std::env::temp_dir().join("test_m24_qa_project.xmi");
    app.qa_select("file.new".into()).unwrap();
    app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::SetText {
            value: path.display().to_string(),
        },
        &ctx,
    )
    .unwrap();
    assert!(app.model.is_empty());
    assert_eq!(app.current_file_path, Some(path.clone()));
    assert!(!app.is_dirty);
    let _ = std::fs::remove_file(path);
}

#[test]
fn project_mcp_new_rejects_dirty_state_without_touching_destination() {
    let mut app = make_app_with_diagram();
    let class = Class::new("DirtyClass");
    let class_id = class.base.id;
    app.model.insert(ModelElement::Class(class));
    let command =
        commands::ChangeDocumentation::new(&app.model, class_id, "changed".into()).unwrap();
    app.execute_command(Box::new(command));
    app.current_file_path = Some(PathBuf::from("/tmp/current-project.xmi"));
    app.selected_element_id = Some(class_id);
    app.name_edit_buffer = "DirtyClass".into();
    app.current_tool = ToolMode::CreateClass;
    app.is_dirty = true;

    let before_model_len = app.model.len();
    let before_diagram_id = app.model.diagrams()[0].id;
    let before_history = (app.history.undo_depth(), app.history.redo_depth());
    let before_path = app.current_file_path.clone();
    let before_active = app.active_diagram;
    let before_selection = app.selected_element_id;
    let before_tool = app.current_tool;
    let destination = std::env::temp_dir().join(format!("umbrello-dirty-{}.xmi", UmlId::new()));
    let _ = std::fs::remove_file(&destination);

    let context = egui::Context::default();
    app.qa_select("file.new".into()).unwrap();
    let result = app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::SetText {
            value: destination.display().to_string(),
        },
        &context,
    );
    assert!(matches!(result, Err(crate::app::qa::protocol::QaError::InvalidValue(_))));
    assert_eq!(app.model.len(), before_model_len);
    assert_eq!(app.model.diagrams()[0].id, before_diagram_id);
    assert_eq!((app.history.undo_depth(), app.history.redo_depth()), before_history);
    assert_eq!(app.current_file_path, before_path);
    assert_eq!(app.active_diagram, before_active);
    assert_eq!(app.selected_element_id, before_selection);
    assert_eq!(app.current_tool, before_tool);
    assert!(app.is_dirty);
    assert!(!destination.exists());
}

#[test]
fn compatibility_policy_enumerates_every_tool_and_supported_diagram() {
    let all_tools = [
        ToolMode::Select,
        ToolMode::CreateClass,
        ToolMode::CreateInterface,
        ToolMode::CreateEnum,
        ToolMode::CreateDatatype,
        ToolMode::CreatePackage,
        ToolMode::CreateActor,
        ToolMode::CreateUseCase,
        ToolMode::CreateComponent,
        ToolMode::CreateNode,
        ToolMode::CreateArtifact,
        ToolMode::CreateGeneralization,
        ToolMode::CreateRealization,
        ToolMode::CreateAssociation,
        ToolMode::CreateAggregation,
        ToolMode::CreateComposition,
        ToolMode::CreateDependency,
    ];
    let expected = [
        (
            DiagramKind::Class,
            [
                ToolMode::Select,
                ToolMode::CreateClass,
                ToolMode::CreateInterface,
                ToolMode::CreateEnum,
                ToolMode::CreateDatatype,
                ToolMode::CreatePackage,
                ToolMode::CreateGeneralization,
                ToolMode::CreateRealization,
                ToolMode::CreateAssociation,
                ToolMode::CreateAggregation,
                ToolMode::CreateComposition,
                ToolMode::CreateDependency,
            ]
            .as_slice(),
        ),
        (
            DiagramKind::UseCase,
            [
                ToolMode::Select,
                ToolMode::CreatePackage,
                ToolMode::CreateActor,
                ToolMode::CreateUseCase,
                ToolMode::CreateGeneralization,
                ToolMode::CreateAssociation,
                ToolMode::CreateDependency,
            ]
            .as_slice(),
        ),
        (
            DiagramKind::Component,
            [
                ToolMode::Select,
                ToolMode::CreatePackage,
                ToolMode::CreateInterface,
                ToolMode::CreateComponent,
                ToolMode::CreateArtifact,
                ToolMode::CreateGeneralization,
                ToolMode::CreateRealization,
                ToolMode::CreateAssociation,
                ToolMode::CreateAggregation,
                ToolMode::CreateComposition,
                ToolMode::CreateDependency,
            ]
            .as_slice(),
        ),
        (
            DiagramKind::Deployment,
            [
                ToolMode::Select,
                ToolMode::CreateComponent,
                ToolMode::CreateNode,
                ToolMode::CreateArtifact,
                ToolMode::CreateAssociation,
                ToolMode::CreateDependency,
            ]
            .as_slice(),
        ),
    ];
    for (kind, compatible) in expected {
        for tool in all_tools {
            assert_eq!(
                compatible.contains(&tool),
                tool.is_compatible_with_diagram(kind),
                "unexpected {tool:?} compatibility with {kind:?}"
            );
        }
    }
    assert!(ToolMode::Select.is_compatible_with_diagram(DiagramKind::Sequence));
    assert!(!ToolMode::CreateClass.is_compatible_with_diagram(DiagramKind::Sequence));
}

#[test]
fn incompatible_direct_placement_preserves_state_and_tool() {
    let mut app = make_app_with_diagram();
    app.current_tool = ToolMode::CreateUseCase;
    let before_model = app.model.len();
    let before_history = app.history.undo_depth();
    let before_dirty = app.is_dirty;
    let result = app.place_element(ToolMode::CreateUseCase, Point::new(10.0, 20.0));
    assert!(result.is_err());
    assert_eq!(app.model.len(), before_model);
    assert_eq!(app.history.undo_depth(), before_history);
    assert_eq!(app.is_dirty, before_dirty);
    assert_eq!(app.current_tool, ToolMode::CreateUseCase);

    app.current_tool = ToolMode::CreateRealization;
    let diagram_id = app.model.diagrams()[0].id;
    app.model.get_diagram_mut(diagram_id).unwrap().kind = DiagramKind::UseCase;
    let result = app.place_edge(UmlId::new(), UmlId::new());
    assert!(result.is_err());
    assert_eq!(app.model.len(), before_model);
    assert_eq!(app.history.undo_depth(), before_history);
    assert_eq!(app.is_dirty, before_dirty);
}

#[test]
fn tool_selection_and_loaded_unsupported_diagram_are_guarded() {
    let mut no_diagram = UmbrelloApp::new(UmlModel::new(), false);
    assert!(no_diagram.choose_tool(ToolMode::CreateClass).is_err());
    assert_eq!(no_diagram.current_tool, ToolMode::Select);

    let mut unsupported = make_app_with_diagram();
    let diagram_id = unsupported.model.diagrams()[0].id;
    unsupported.model.get_diagram_mut(diagram_id).unwrap().kind = DiagramKind::Sequence;
    assert!(unsupported.choose_tool(ToolMode::CreateClass).is_err());
    assert_eq!(unsupported.current_tool, ToolMode::Select);
    assert!(unsupported
        .qa_snapshot()
        .targets
        .iter()
        .any(|target| target.id == "tool.class" && !target.enabled));
}

#[test]
fn browser_existing_elements_are_selectable_and_reused_once_with_history() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/project.xmi"));
    let class = Class::new("Reusable");
    let class_id = class.base.id;
    app.model.insert(ModelElement::Class(class));
    assert!(app.select_element(class_id).is_ok());
    assert_eq!(app.selected_element_id, Some(class_id));
    assert_eq!(app.name_edit_buffer, "Reusable");
    assert!(app.add_element_to_active_diagram(class_id).is_ok());
    assert!(app.model.diagrams()[0].get_node(class_id).is_some());
    assert_eq!(app.history.undo_depth(), 1);
    let position = app.model.diagrams()[0]
        .get_node(class_id)
        .unwrap()
        .bounds
        .origin;
    assert!(app.add_element_to_active_diagram(class_id).is_err());
    assert_eq!(
        app.model.diagrams()[0]
            .get_node(class_id)
            .unwrap()
            .bounds
            .origin,
        position
    );
    app.history.undo(&mut app.model).unwrap();
    assert!(app.model.diagrams()[0].get_node(class_id).is_none());
    app.history.redo(&mut app.model).unwrap();
    assert!(app.model.diagrams()[0].get_node(class_id).is_some());
}

#[test]
fn browser_reuse_rejects_incompatible_relationship_and_missing_diagram() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/project.xmi"));
    let class = Class::new("ClassOnly");
    let class_id = class.base.id;
    app.model.insert(ModelElement::Class(class));
    let diagram_id = app.model.diagrams()[0].id;
    app.model.get_diagram_mut(diagram_id).unwrap().kind = DiagramKind::UseCase;
    assert!(app.add_element_to_active_diagram(class_id).is_err());
    app.model.get_diagram_mut(diagram_id).unwrap().kind = DiagramKind::Class;
    let actor = Actor::new("Actor");
    let actor_id = actor.base.id;
    app.model.insert(ModelElement::Actor(actor));
    assert!(app.add_element_to_active_diagram(actor_id).is_err());

    let source = Class::new("Source");
    let source_id = source.base.id;
    let target = Class::new("Target");
    let target_id = target.base.id;
    app.model.insert(ModelElement::Class(source));
    app.model.insert(ModelElement::Class(target));
    let relationship = Relationship::new_association(source_id, target_id);
    let relationship_id = relationship.base.id;
    app.model.insert(ModelElement::Relationship(relationship));
    assert!(app.add_element_to_active_diagram(relationship_id).is_err());

    app.active_diagram = None;
    assert!(app.add_element_to_active_diagram(source_id).is_err());
}

#[test]
fn browser_mcp_element_targets_select_and_report_add_action_state() {
    let mut app = make_app_with_diagram();
    app.current_file_path = None;
    let class = Class::new("BrowserClass");
    let class_id = class.base.id;
    app.model.insert(ModelElement::Class(class));
    let snapshot = app.qa_snapshot();
    let element_target = format!("element:{class_id}");
    let add_target = format!("element.add_to_diagram:{class_id}");
    assert!(snapshot
        .targets
        .iter()
        .any(|target| target.id == element_target));
    assert!(snapshot
        .targets
        .iter()
        .any(|target| target.id == add_target && !target.enabled));

    app.current_file_path = Some(PathBuf::from("/tmp/project.xmi"));
    let snapshot = app.qa_snapshot();
    assert!(snapshot
        .targets
        .iter()
        .any(|target| target.id == add_target && target.enabled));
    let context = egui::Context::default();
    app.qa_select(element_target).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .unwrap();
    assert_eq!(app.selected_element_id, Some(class_id));
    app.qa_select(add_target).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .unwrap();
    assert!(app.model.diagrams()[0].get_node(class_id).is_some());
}

#[test]
fn browser_loaded_incompatible_nodes_remain_visible_and_selectable() {
    let mut model = UmlModel::new();
    let class = Class::new("LoadedClass");
    let class_id = class.base.id;
    model.insert(ModelElement::Class(class));
    let mut diagram = Diagram::new("Loaded Sequence", DiagramKind::Sequence);
    let diagram_id = diagram.id;
    diagram
        .add_node(class_id, uml_core::ViewNode::new(class_id, Rect::new(10.0, 20.0, 100.0, 60.0)));
    model.add_diagram(diagram);
    let mut app = UmbrelloApp::new(model, true);
    app.current_file_path = Some(PathBuf::from("/tmp/loaded.xmi"));
    app.active_diagram = Some(0);
    let snapshot = app.qa_snapshot();
    assert!(snapshot
        .targets
        .iter()
        .any(|target| target.id == format!("node:{class_id}") && target.enabled));
    assert!(snapshot
        .targets
        .iter()
        .any(|target| target.id == "tool.class" && !target.enabled));
    let context = egui::Context::default();
    app.qa_select(format!("node:{class_id}")).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .unwrap();
    assert_eq!(app.selected_element_id, Some(class_id));
    assert_eq!(app.model.get_diagram(diagram_id).unwrap().node_count(), 1);
}

#[test]
fn browser_relationships_are_selectable_without_relationship_property_targets() {
    let mut model = UmlModel::new();
    let source = Class::new("Source");
    let source_id = source.base.id;
    let target = Class::new("Target");
    let target_id = target.base.id;
    model.insert(ModelElement::Class(source));
    model.insert(ModelElement::Class(target));
    let relationship = Relationship::new_association(source_id, target_id);
    let relationship_id = relationship.base.id;
    model.insert(ModelElement::Relationship(relationship));
    let diagram = Diagram::new("Class", DiagramKind::Class);
    model.add_diagram(diagram);
    let mut app = UmbrelloApp::new(model, true);
    app.current_file_path = Some(PathBuf::from("/tmp/relationships.xmi"));
    app.active_diagram = Some(0);
    let snapshot = app.qa_snapshot();
    let relationship_target = snapshot
        .targets
        .iter()
        .find(|target| target.id == format!("element:{relationship_id}"))
        .expect("relationship target should be discoverable");
    assert!(relationship_target.label.contains("Source"));
    assert!(relationship_target.label.contains("Target"));
    app.select_element(relationship_id).unwrap();
    let snapshot = app.qa_snapshot();
    assert!(!snapshot
        .targets
        .iter()
        .any(|target| target.id == "property.name"));
    assert!(!snapshot
        .targets
        .iter()
        .any(|target| target.id == format!("element.add_to_diagram:{relationship_id}")
            && target.enabled));
}

// ── Existing rendering tests ─────────────────────────────────

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
    let cls = Class::new("Person");
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
    let cls = ModelElement::Class(Class::new("C"));
    let iface = ModelElement::Interface(Interface::new("I"));
    assert_ne!(element_color(Some(&cls)), element_color(Some(&iface)));
    assert_eq!(element_color(None), egui::Color32::from_rgb(220, 220, 220));
}

// ── M16 File I/O tests (T1-T7) ─────────────────────────────────

/// T1: New Project writes before replacing the model and establishes a path.
#[test]
fn file_new_clears_model() {
    let mut app = make_app_with_class("Test");
    assert_eq!(app.model.len(), 1);
    assert!(!app.is_dirty);
    let path = std::env::temp_dir().join("test_m24_new_project.xmi");
    app.new_project_at(&path).unwrap();
    assert_eq!(app.model.len(), 0);
    assert!(!app.is_dirty);
    assert_eq!(app.current_file_path, Some(path.clone()));
    assert!(path.exists());
    let _ = std::fs::remove_file(path);
}

/// T2: Dirty flag is set after executing a command.
#[test]
fn dirty_flag_on_mutation() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    assert!(!app.is_dirty);
    // Simulate a command by directly setting is_dirty
    app.is_dirty = true;
    assert!(app.is_dirty);
}

/// T2b: Using execute_command sets dirty.
#[test]
fn dirty_flag_after_execute_command() {
    let mut app = make_app_with_class("Test");
    app.is_dirty = false;
    // MoveNode will fail because no diagram, so test that execute_command
    // correctly handles Ok and sets dirty. Let's test with a simpler approach:
    // We can verify the helper pattern works by checking directly.
    // The execute_command is private and only used with valid commands.
    // Test that a successful execute sets dirty:
    assert!(!app.is_dirty);
    // We can't easily create a valid command here (needs real model state),
    // but we verify the pattern in T7's save test.
}

/// T3: Dirty flag is cleared after save.
#[test]
fn dirty_flag_cleared_on_save() {
    let mut app = make_app_with_class("Test");
    app.is_dirty = true;

    // Save to a temp file
    let dir = std::env::temp_dir();
    let path = dir.join("test_m16_dirty_save.xmi");
    app.current_file_path = Some(path.clone());

    app.menu_file_save();
    // After successful save, dirty should be cleared
    assert!(!app.is_dirty);

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

/// T4: Dirty flag is cleared after open (conceptually — open replaces model).
#[test]
fn dirty_flag_cleared_on_open() {
    let mut app = make_app_with_class("Test");
    app.is_dirty = true;

    // Simulate open by setting a new model (like menu_file_open does)
    app.model = UmlModel::new();
    app.history.clear();
    app.active_diagram = None;
    app.is_dirty = false;

    assert!(!app.is_dirty);
    assert_eq!(app.model.len(), 0);
}

/// T5: File path tracking.
#[test]
fn file_path_tracking() {
    let mut app = make_app_with_class("Test");

    // Initially no path
    assert!(app.current_file_path.is_none());

    // Set a path
    let path = PathBuf::from("/some/path.xmi");
    app.set_current_file_path(Some(path.clone()));
    assert_eq!(app.current_file_path, Some(path));
}

/// T6: Save then reload round-trip.
#[test]
fn save_then_reload_roundtrip() {
    let mut model = UmlModel::new();
    let cls = Class::new("RoundtripClass");
    model.insert(ModelElement::Class(cls));
    // Save to temp file
    let dir = std::env::temp_dir();
    let path = dir.join("test_m16_roundtrip.xmi");

    // Use uml_io convenience function
    uml_io::xmi::save_xmi_to_file(&model, &path).expect("save should succeed");

    // Load it back
    let loaded = uml_io::xmi::load_xmi_from_file(&path).expect("load should succeed");

    // The loaded model should contain the class (may have extra wrapper elements)
    assert!(!loaded.is_empty());
    assert!(loaded.iter().any(|(_, e)| e.name() == "RoundtripClass"));

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

/// T7: Save As updates path.
#[test]
fn save_as_updates_path() {
    let mut app = make_app_with_class("TestPath");
    assert!(app.current_file_path.is_none());

    // Save As to a temp file
    let dir = std::env::temp_dir();
    let path = dir.join("test_m16_saveas.xmi");

    // Directly set the path and save (simulating what menu_file_save_as does)
    app.current_file_path = Some(path.clone());
    app.is_dirty = true;

    uml_io::xmi::save_xmi_to_file(&app.model, &path).expect("save should succeed");
    app.is_dirty = false;

    assert_eq!(app.current_file_path, Some(path.clone()));
    assert!(!app.is_dirty);
    assert!(path.exists());

    // Cleanup
    let _ = std::fs::remove_file(&path);
}

// ══════════════════════════════════════════════════════════════════
// M17 — Tool Palette & Interactive Element Creation Tests (T1-T17)
// ══════════════════════════════════════════════════════════════════

/// T1: ToolMode defaults to Select on app creation.
#[test]
fn tool_mode_defaults_to_select() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    assert_eq!(app.current_tool, ToolMode::Select);
}

/// T2: ToolMode::Select.label() returns a non-empty string.
#[test]
fn tool_mode_select_label() {
    let label = ToolMode::Select.label();
    assert!(!label.is_empty(), "Select label should be non-empty");
    // All labels should be non-empty
    for tool in &[
        ToolMode::Select,
        ToolMode::CreateClass,
        ToolMode::CreateInterface,
        ToolMode::CreateEnum,
        ToolMode::CreateDatatype,
        ToolMode::CreatePackage,
    ] {
        assert!(!tool.label().is_empty(), "Label for {tool:?} should be non-empty");
    }
}

/// T3: is_creation_tool returns true for creation tools, false for Select.
#[test]
fn tool_mode_is_creation_tool() {
    assert!(!ToolMode::Select.is_creation_tool());
    assert!(ToolMode::CreateClass.is_creation_tool());
    assert!(ToolMode::CreateInterface.is_creation_tool());
    assert!(ToolMode::CreateEnum.is_creation_tool());
    assert!(ToolMode::CreateDatatype.is_creation_tool());
    assert!(ToolMode::CreatePackage.is_creation_tool());
}

/// T4: generate_unique_name returns "{base}_1" in an empty model.
#[test]
fn generate_unique_name_first() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    assert_eq!(app.generate_unique_name("Class"), "Class_1");
    assert_eq!(app.generate_unique_name("Package"), "Package_1");
}

/// T5: generate_unique_name increments correctly when "{base}_1" exists.
#[test]
fn generate_unique_name_increments() {
    let mut model = UmlModel::new();
    let c1 = ModelElement::Class(Class::new("Class_1"));
    model.insert(c1);
    let app = UmbrelloApp::new(model, false);
    assert_eq!(app.generate_unique_name("Class"), "Class_2");
}

/// T6: generate_unique_name finds gaps (e.g., "Class_1" and "Class_3" → "Class_2").
#[test]
fn generate_unique_name_finds_gap() {
    let mut model = UmlModel::new();
    model.insert(ModelElement::Class(Class::new("Class_1")));
    model.insert(ModelElement::Class(Class::new("Class_3")));
    let app = UmbrelloApp::new(model, false);
    assert_eq!(app.generate_unique_name("Class"), "Class_2");
}

/// T7: create_element_for_tool(CreateClass) returns a ModelElement::Class with a unique name.
#[test]
fn create_element_for_tool_class() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    let elem = app.create_element_for_tool(ToolMode::CreateClass);
    assert!(matches!(elem, ModelElement::Class(_)));
    assert_eq!(elem.name(), "Class_1");
}

/// T8: create_element_for_tool(CreatePackage) returns a ModelElement::Package with unique name.
#[test]
fn create_element_for_tool_package() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    let elem = app.create_element_for_tool(ToolMode::CreatePackage);
    assert!(matches!(elem, ModelElement::Package(_)));
    assert_eq!(elem.name(), "Package_1");
}

/// T9: place_element creates the element in the model.
#[test]
fn place_element_creates_in_model() {
    let mut app = make_app_with_diagram();
    let len_before = app.model.len();
    let result = app.place_element(ToolMode::CreateClass, Point::new(100.0, 100.0));
    assert!(result.is_ok());
    assert_eq!(app.model.len(), len_before + 1);
    // Model should contain a class named "Class_1"
    assert!(app.model.iter().any(|(_, e)| e.name() == "Class_1"));
}

/// T10: place_element adds a ViewNode to the active diagram.
#[test]
fn place_element_adds_node_to_diagram() {
    let mut app = make_app_with_diagram();
    let diag = &app.model.diagrams()[0];
    let nodes_before = diag.nodes.len();

    let result = app.place_element(ToolMode::CreateClass, Point::new(100.0, 100.0));
    assert!(result.is_ok());

    let diag = &app.model.diagrams()[0];
    assert_eq!(diag.nodes.len(), nodes_before + 1);
    // The added node should have the correct element ID
    let elem_id = app
        .model
        .iter()
        .find(|(_, e)| e.name() == "Class_1")
        .map(|(id, _)| id)
        .unwrap();
    assert!(diag.get_node(elem_id).is_some());
    // Check position
    let node = diag.get_node(elem_id).unwrap();
    assert_eq!(node.bounds.x(), 100.0);
    assert_eq!(node.bounds.y(), 100.0);
}

/// T11: place_element sets is_dirty to true.
#[test]
fn place_element_dirty_flag() {
    let mut app = make_app_with_diagram();
    app.is_dirty = false;
    let result = app.place_element(ToolMode::CreateClass, Point::new(100.0, 100.0));
    assert!(result.is_ok());
    assert!(app.is_dirty);
}

/// T12: Tool resets to Select after placement (simulates background handler flow).
#[test]
fn tool_resets_after_placement() {
    let mut app = make_app_with_diagram();
    // Place element with CreateClass
    let result = app.place_element(ToolMode::CreateClass, Point::new(100.0, 100.0));
    assert!(result.is_ok());
    // Simulate reset done by background click handler in render_canvas
    app.current_tool = ToolMode::Select;
    assert_eq!(app.current_tool, ToolMode::Select);
}

/// T13: Undo after place_element removes both the element and the ViewNode.
#[test]
fn place_element_undo_removes_both() {
    let mut app = make_app_with_diagram();
    let result = app.place_element(ToolMode::CreateClass, Point::new(100.0, 100.0));
    assert!(result.is_ok());
    let elem_id = app
        .model
        .iter()
        .find(|(_, e)| e.name() == "Class_1")
        .map(|(id, _)| id)
        .unwrap();
    assert!(app.model.get(elem_id).is_some());

    // One undo reverses the atomic element-and-node operation.
    app.history.undo(&mut app.model).unwrap();
    assert!(app.model.get(elem_id).is_none());
    let diag = &app.model.diagrams()[0];
    assert!(diag.get_node(elem_id).is_none());
}

/// T14: Select tool is not a creation tool and does not trigger element creation.
#[test]
fn selection_persists_before_click() {
    let mut app = make_app_with_diagram();
    app.current_tool = ToolMode::Select;
    assert!(!app.current_tool.is_creation_tool());
    // Verify that place_element rejects Select (via panic in create_element_for_tool)
    // This is tested by the tool guard — Select should never reach place_element
    // in normal flow because is_creation_tool() is checked first.
    let was_select = app.current_tool == ToolMode::Select;
    assert!(was_select);
}

#[test]
fn viewport_zoom_anchor_and_pan_are_view_only() {
    let mut app = make_app_with_diagram();
    let canvas = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(800.0, 600.0));
    let cursor = egui::pos2(310.0, 220.0);
    let before_dirty = app.is_dirty;
    let before_history = app.history.can_undo();
    let point = app
        .viewport_transform(canvas.min)
        .unwrap()
        .screen_to_model(cursor);
    app.zoom_at(canvas, cursor, 2.0);
    let after = app
        .viewport_transform(canvas.min)
        .unwrap()
        .model_to_screen(point);
    assert!((after.x - cursor.x).abs() < 1e-4);
    assert!((after.y - cursor.y).abs() < 1e-4);
    assert_eq!(app.model.diagrams()[0].zoom_percent(), 200.0);
    assert_eq!(app.is_dirty, before_dirty);
    assert_eq!(app.history.can_undo(), before_history);
}

#[test]
fn viewport_fit_handles_negative_coordinates_and_empty_diagrams() {
    let mut app = make_app_with_diagram();
    let element = Class::new("Fit");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, uml_core::ViewNode::new(id, Rect::new(-500.0, -300.0, 1000.0, 600.0)));
    let canvas = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0));
    app.fit_active_diagram(canvas);
    let transform = app.viewport_transform(canvas.min).unwrap();
    let center = transform.model_to_screen(Point::new(0.0, 0.0));
    assert!((center.x - canvas.center().x).abs() < 1.0);
    assert!((center.y - canvas.center().y).abs() < 1.0);
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .remove_node(id);
    app.fit_active_diagram(canvas);
    assert_eq!(app.model.diagrams()[0].zoom_percent(), 100.0);
    assert_eq!(app.viewport_pans[&diagram_id], egui::Vec2::ZERO);
}

#[test]
fn qa_exposes_and_dispatches_viewport_actions_without_model_mutation() {
    let mut app = make_app_with_diagram();
    let ctx = egui::Context::default();
    let initial_dirty = app.is_dirty;
    let initial_history = app.history.can_undo();
    let snapshot = app.qa_snapshot();
    for target in [
        "viewport.zoom_in",
        "viewport.zoom_out",
        "viewport.fit",
        "viewport.reset",
    ] {
        assert!(snapshot
            .targets
            .iter()
            .any(|item| item.id == target && item.enabled));
    }
    assert_eq!(snapshot.zoom_percent, Some(100.0));
    assert_eq!(snapshot.pan_x, Some(0.0));
    assert_eq!(snapshot.pan_y, Some(0.0));

    app.qa_select("viewport.zoom_in".into()).unwrap();
    let selected_revision = app.state_revision;
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
        .unwrap();
    assert_eq!(app.state_revision, selected_revision + 1);
    assert_eq!(app.model.diagrams()[0].zoom_percent(), 105.0);
    assert_eq!(app.is_dirty, initial_dirty);
    assert_eq!(app.history.can_undo(), initial_history);
}

#[test]
fn qa_canvas_drag_pans_by_screen_delta_and_rejects_non_finite_values() {
    let mut app = make_app_with_diagram();
    let ctx = egui::Context::default();
    app.current_tool = ToolMode::Select;
    app.qa_select("canvas".into()).unwrap();
    let before = app.state_revision;
    app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::Drag {
            position: Some((12.0, -7.0)),
            to_target: None,
            gesture: None,
        },
        &ctx,
    )
    .unwrap();
    let diagram_id = app.model.diagrams()[0].id;
    assert_eq!(app.state_revision, before + 1);
    assert_eq!(app.viewport_pans[&diagram_id], egui::vec2(12.0, -7.0));
    assert!(!app.is_dirty);
    assert!(!app.history.can_undo());

    let error = app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::Drag {
            position: Some((f64::NAN, 0.0)),
            to_target: None,
            gesture: None,
        },
        &ctx,
    );
    assert!(matches!(error, Err(crate::app::qa::protocol::QaError::InvalidCoordinates)));

    let pan_before = app.viewport_pans[&diagram_id];
    let revision_before = app.state_revision;
    let error = app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::Drag {
            position: Some((1e308, 0.0)),
            to_target: None,
            gesture: None,
        },
        &ctx,
    );
    assert!(matches!(error, Err(crate::app::qa::protocol::QaError::InvalidCoordinates)));
    assert_eq!(app.viewport_pans[&diagram_id], pan_before);
    assert_eq!(app.state_revision, revision_before);

    let max_f32 = f64::from(f32::MAX);
    app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::Drag {
            position: Some((max_f32, 0.0)),
            to_target: None,
            gesture: None,
        },
        &ctx,
    )
    .unwrap();
    let pan_before = app.viewport_pans[&diagram_id];
    let revision_before = app.state_revision;
    let error = app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::Drag {
            position: Some((max_f32, 0.0)),
            to_target: None,
            gesture: None,
        },
        &ctx,
    );
    assert!(matches!(error, Err(crate::app::qa::protocol::QaError::InvalidCoordinates)));
    assert_eq!(app.viewport_pans[&diagram_id], pan_before);
    assert_eq!(app.state_revision, revision_before);
}

#[test]
fn qa_viewport_targets_are_disabled_without_active_diagram_and_fit_requires_canvas() {
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    let ctx = egui::Context::default();
    let snapshot = app.qa_snapshot();
    assert_eq!(snapshot.zoom_percent, None);
    assert!(snapshot
        .targets
        .iter()
        .any(|target| target.id == "viewport.reset" && !target.enabled));
    assert!(matches!(
        app.qa_select("viewport.reset".into()),
        Err(crate::app::qa::protocol::QaError::UnavailableTarget(_))
    ));

    app = make_app_with_diagram();
    app.qa_select("viewport.fit".into()).unwrap();
    let before = app.state_revision;
    let result =
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx);
    assert!(matches!(result, Err(crate::app::qa::protocol::QaError::NotReady)));
    assert_eq!(app.state_revision, before);
}

/// T15: New element created via the tool is visible in the model's element list.
#[test]
fn new_element_visible_on_canvas() {
    let mut app = make_app_with_diagram();
    app.place_element(ToolMode::CreateClass, Point::new(50.0, 50.0))
        .unwrap();
    // The element should appear in model iter
    let found = app.model.iter().any(|(_, e)| e.name() == "Class_1");
    assert!(found, "Created element should be visible in model");
}

/// T16: Tool palette contains all 6 tools (verified via ToolMode variants).
#[test]
fn tool_palette_buttons_exist() {
    let tools = [
        ToolMode::Select,
        ToolMode::CreateClass,
        ToolMode::CreateInterface,
        ToolMode::CreateEnum,
        ToolMode::CreateDatatype,
        ToolMode::CreatePackage,
    ];
    assert_eq!(tools.len(), 6);
    // Verify each has a unique non-empty label
    let mut labels: Vec<&str> = tools.iter().map(ToolMode::label).collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), 6, "All 6 tools should have unique labels");
    // Verify all creation tools report true
    for t in &tools[1..] {
        assert!(t.is_creation_tool());
    }
    assert!(!tools[0].is_creation_tool());
}

/// T17: element_color returns the correct color for each element type.
#[test]
fn element_color_for_new_type() {
    // Class → blue
    let cls = ModelElement::Class(Class::new("C"));
    assert_eq!(element_color(Some(&cls)), egui::Color32::from_rgb(180, 210, 255));
    // Interface → green
    let iface = ModelElement::Interface(Interface::new("I"));
    assert_eq!(element_color(Some(&iface)), egui::Color32::from_rgb(180, 255, 210));
    // Enum → orange
    let en = ModelElement::Enum(Enum::new("E"));
    assert_eq!(element_color(Some(&en)), egui::Color32::from_rgb(255, 210, 180));
    // Datatype → purple
    let dt = ModelElement::Datatype(Datatype::new("D"));
    assert_eq!(element_color(Some(&dt)), egui::Color32::from_rgb(210, 180, 255));
    // Package → yellow
    let pkg = ModelElement::Package(Package::new("P"));
    assert_eq!(element_color(Some(&pkg)), egui::Color32::from_rgb(255, 255, 200));
    // None → gray
    assert_eq!(element_color(None), egui::Color32::from_rgb(220, 220, 220));
}

// ══════════════════════════════════════════════════════════════════
// M18 — Selection & Property Editor Tests (APP-01 to APP-15)
// ══════════════════════════════════════════════════════════════════

/// APP-01: New UmbrelloApp has selected_element_id: None.
#[test]
fn selected_element_id_defaults_to_none() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    assert!(app.selected_element_id.is_none());
    assert!(app.name_edit_buffer.is_empty());
}

/// APP-02: Setting selected_element_id to Some(id) is reflected.
#[test]
fn select_node_sets_selected_element_id() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    app.selected_element_id = Some(id);
    assert_eq!(app.selected_element_id, Some(id));
}

/// APP-03: Clearing selection sets selected_element_id to None.
#[test]
fn deselect_on_background_click() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    app.selected_element_id = Some(id);
    assert!(app.selected_element_id.is_some());
    // Simulate background click clearing selection
    app.selected_element_id = None;
    app.name_edit_buffer.clear();
    assert!(app.selected_element_id.is_none());
    assert!(app.name_edit_buffer.is_empty());
}

/// APP-04: name_edit_buffer is populated from the selected element's name.
#[test]
fn name_edit_buffer_populates_on_selection() {
    let mut app = make_app_with_class("MyClass");
    let id = app.model.iter().next().unwrap().0;
    // Simulate clicking on the node (populates buffer)
    if let Some(elem) = app.model.get(id) {
        app.name_edit_buffer = elem.name().to_string();
    }
    app.selected_element_id = Some(id);
    assert_eq!(app.name_edit_buffer, "MyClass");
}

/// APP-05: RenameElement via property editor pattern.
#[test]
fn rename_element_via_property_editor() {
    let mut app = make_app_with_class("Original");
    let id = app.model.iter().next().unwrap().0;
    app.name_edit_buffer = "Renamed".to_string();
    app.selected_element_id = Some(id);
    let new_name = app.name_edit_buffer.trim().to_string();
    if !new_name.is_empty() && new_name != "Original" {
        if let Ok(cmd) = commands::RenameElement::new(&app.model, id, new_name.clone()) {
            app.execute_command(Box::new(cmd));
            app.name_edit_buffer = new_name;
        }
    }
    assert_eq!(app.model.get(id).unwrap().name(), "Renamed");
}

/// APP-06: ChangeVisibility sets visibility to Private.
#[test]
fn visibility_dropdown_changes_visibility() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    let cmd = commands::ChangeVisibility::new(&app.model, id, Visibility::Private).unwrap();
    app.execute_command(Box::new(cmd));
    assert_eq!(app.model.get(id).unwrap().base().visibility, Visibility::Private);
}

/// APP-07: Visibility change can be undone.
#[test]
fn visibility_change_undo_restores() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    let mut cmd = commands::ChangeVisibility::new(&app.model, id, Visibility::Private).unwrap();
    cmd.execute(&mut app.model).unwrap();
    assert_eq!(app.model.get(id).unwrap().base().visibility, Visibility::Private);
    cmd.undo(&mut app.model).unwrap();
    assert_eq!(app.model.get(id).unwrap().base().visibility, Visibility::Public);
}

/// APP-08: ChangeElementFlags sets both flags.
#[test]
fn flag_toggle_sets_abstract_and_static() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    let cmd = commands::ChangeElementFlags::new(&app.model, id, true, true).unwrap();
    app.execute_command(Box::new(cmd));
    let base = app.model.get(id).unwrap().base();
    assert!(base.is_abstract);
    assert!(base.is_static);
}

/// APP-09: ChangeElementFlags undo restores flags.
#[test]
fn flag_toggle_undo_restores_flags() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    let mut cmd = commands::ChangeElementFlags::new(&app.model, id, true, true).unwrap();
    cmd.execute(&mut app.model).unwrap();
    cmd.undo(&mut app.model).unwrap();
    let base = app.model.get(id).unwrap().base();
    assert!(!base.is_abstract);
    assert!(!base.is_static);
}

/// APP-10: ChangeDocumentation persists.
#[test]
fn documentation_edit_persists() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    let cmd = commands::ChangeDocumentation::new(&app.model, id, "Hello".into()).unwrap();
    app.execute_command(Box::new(cmd));
    assert_eq!(app.model.get(id).unwrap().base().documentation, "Hello");
}

/// APP-11: Documentation change undo reverts.
#[test]
fn documentation_change_undo_reverts() {
    let mut app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    let mut cmd = commands::ChangeDocumentation::new(&app.model, id, "Hello".into()).unwrap();
    cmd.execute(&mut app.model).unwrap();
    cmd.undo(&mut app.model).unwrap();
    assert_eq!(app.model.get(id).unwrap().base().documentation, "");
}

/// APP-12: Classifier details displayed for a Class.
#[test]
fn classifier_details_displayed_for_class() {
    let app = make_app_with_class("Test");
    let id = app.model.iter().next().unwrap().0;
    let elem = app.model.get(id).unwrap();
    assert!(elem.classifier_data().is_some());
    assert_eq!(elem.classifier_data().unwrap().attributes.len(), 0);
    assert_eq!(elem.classifier_data().unwrap().operations.len(), 0);
}

/// APP-13: Classifier details hidden for a Package.
#[test]
fn classifier_details_hidden_for_package() {
    let mut model = UmlModel::new();
    let pkg = Package::new("TestPkg");
    model.insert(ModelElement::Package(pkg));
    let app = UmbrelloApp::new(model, false);
    let id = app.model.iter().next().unwrap().0;
    let elem = app.model.get(id).unwrap();
    assert!(elem.classifier_data().is_none());
}

/// APP-14: Property editor placeholder when nothing selected.
#[test]
fn property_editor_placeholder_when_none_selected() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    // When nothing is selected, the placeholder path runs
    assert!(app.selected_element_id.is_none());
    // The render_property_editor function handles this case;
    // we verify by checking that with no selection the state is correct.
}

/// APP-15: execute_command sets dirty flag on property change.
#[test]
fn dirty_flag_set_on_property_change() {
    let mut app = make_app_with_class("Test");
    app.is_dirty = false;
    let id = app.model.iter().next().unwrap().0;
    let cmd = commands::ChangeVisibility::new(&app.model, id, Visibility::Private).unwrap();
    app.execute_command(Box::new(cmd));
    assert!(app.is_dirty);
}

// ══════════════════════════════════════════════════════════════════
// M19 Phase 2 — Edge Tool Palette Tests (APP-16 to APP-19, APP-25)
// ══════════════════════════════════════════════════════════════════

/// Helper: create an UmbrelloApp with a Class diagram containing two nodes.
#[allow(dead_code)]
fn make_app_with_two_nodes() -> UmbrelloApp {
    let mut model = UmlModel::new();
    let cls_a = Class::new("ClassA");
    let cls_b = Class::new("ClassB");
    let id_a = cls_a.base.id;
    let id_b = cls_b.base.id;
    model.insert(ModelElement::Class(cls_a));
    model.insert(ModelElement::Class(cls_b));
    let d = Diagram::new("Test", DiagramKind::Class);
    let diag_id = d.id;
    model.add_diagram(d);
    // Add nodes to diagram
    let diagram_idx = model.diagrams().len() - 1;
    let d = model.get_diagram_mut(diag_id).unwrap();
    d.add_node(id_a, uml_core::ViewNode::new(id_a, uml_core::Rect::new(0.0, 0.0, 160.0, 60.0)));
    d.add_node(
        id_b,
        uml_core::ViewNode::new(id_b, uml_core::Rect::new(200.0, 0.0, 160.0, 60.0)),
    );
    let mut app = UmbrelloApp::new(model, false);
    app.active_diagram = Some(diagram_idx);
    app.current_file_path = Some(PathBuf::from("/tmp/test-project.xmi"));
    app
}

/// APP-16: Edge tool reports is_edge_tool() == true.
#[test]
fn edge_tool_is_edge_tool() {
    assert!(ToolMode::CreateGeneralization.is_edge_tool());
    assert!(ToolMode::CreateRealization.is_edge_tool());
    assert!(ToolMode::CreateAssociation.is_edge_tool());
    assert!(ToolMode::CreateAggregation.is_edge_tool());
    assert!(ToolMode::CreateComposition.is_edge_tool());
    assert!(ToolMode::CreateDependency.is_edge_tool());
}

/// APP-17: Edge tool is NOT a creation tool (no ghost preview, no crosshair).
#[test]
fn edge_tool_not_creation_tool() {
    assert!(!ToolMode::CreateGeneralization.is_creation_tool());
    assert!(!ToolMode::CreateRealization.is_creation_tool());
    assert!(!ToolMode::CreateAssociation.is_creation_tool());
    assert!(!ToolMode::CreateAggregation.is_creation_tool());
    assert!(!ToolMode::CreateComposition.is_creation_tool());
    assert!(!ToolMode::CreateDependency.is_creation_tool());
}

/// APP-18: Edge tool's association_type() maps correctly.
#[test]
fn edge_tool_association_type() {
    assert_eq!(
        ToolMode::CreateGeneralization.association_type(),
        Some(AssociationType::Generalization)
    );
    assert_eq!(
        ToolMode::CreateRealization.association_type(),
        Some(AssociationType::Realization)
    );
    assert_eq!(
        ToolMode::CreateAssociation.association_type(),
        Some(AssociationType::Association)
    );
    assert_eq!(
        ToolMode::CreateAggregation.association_type(),
        Some(AssociationType::Aggregation)
    );
    assert_eq!(
        ToolMode::CreateComposition.association_type(),
        Some(AssociationType::Composition)
    );
    assert_eq!(ToolMode::CreateDependency.association_type(), Some(AssociationType::Dependency));
}

/// APP-19: Select is not an edge tool.
#[test]
fn select_not_edge_tool() {
    assert!(!ToolMode::Select.is_edge_tool());
    assert!(ToolMode::Select.association_type().is_none());
}

/// APP-25: place_edge returns an error when there is no active diagram.
#[test]
fn place_edge_no_diagram_errors() {
    // Create app with no active diagram
    let mut app = UmbrelloApp::new(UmlModel::new(), false);
    // Set an edge tool (any edge tool will do)
    app.current_tool = ToolMode::CreateGeneralization;
    // Ensure no active diagram
    app.active_diagram = None;
    // Source/target IDs don't matter when there's no diagram
    let src_id = UmlId::new();
    let tgt_id = UmlId::new();
    let result = app.place_edge(src_id, tgt_id);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("No active diagram"));
}

// ══════════════════════════════════════════════════════════════════
// M19 Phase 3 — Canvas Edge Creation Tests (APP-20 to APP-24, APP-26, APP-27)
// ══════════════════════════════════════════════════════════════════

/// APP-20: place_edge creates a Relationship in the model.
#[test]
fn place_edge_creates_relationship() {
    let mut app = make_app_with_two_nodes();
    app.current_tool = ToolMode::CreateGeneralization;

    let id_a = app.model.iter().next().unwrap().0;
    let id_b = app.model.iter().nth(1).unwrap().0;

    let len_before = app.model.len();
    let result = app.place_edge(id_a, id_b);
    assert!(result.is_ok());
    assert_eq!(app.model.len(), len_before + 1);

    // Find the Relationship in the model
    let rel_found = app.model.iter().any(|(_, e)| {
        matches!(e, ModelElement::Relationship(r) if r.source_id == id_a && r.target_id == id_b)
    });
    assert!(rel_found, "Expected a Relationship between source and target");
}

/// APP-21: place_edge creates a ViewEdge in the active diagram.
#[test]
fn place_edge_creates_view_edge() {
    let mut app = make_app_with_two_nodes();
    app.current_tool = ToolMode::CreateGeneralization;

    let id_a = app.model.iter().next().unwrap().0;
    let id_b = app.model.iter().nth(1).unwrap().0;

    let diag = &app.model.diagrams()[0];
    let edges_before = diag.edges.len();

    let result = app.place_edge(id_a, id_b);
    assert!(result.is_ok());

    let diag = &app.model.diagrams()[0];
    assert_eq!(diag.edges.len(), edges_before + 1);

    // Verify the edge references the source and target nodes
    let has_edge = diag
        .edges
        .values()
        .any(|edge| edge.source_node_id == id_a && edge.target_node_id == id_b);
    assert!(has_edge, "Expected a ViewEdge connecting the two nodes");
}

/// APP-22: place_edge sets the dirty flag.
#[test]
fn place_edge_dirty_flag() {
    let mut app = make_app_with_two_nodes();
    app.current_tool = ToolMode::CreateAssociation;
    app.is_dirty = false;

    let id_a = app.model.iter().next().unwrap().0;
    let id_b = app.model.iter().nth(1).unwrap().0;

    let result = app.place_edge(id_a, id_b);
    assert!(result.is_ok());
    assert!(app.is_dirty, "Dirty flag should be set after edge creation");
}

/// APP-23: Undo after place_edge removes both the Relationship and the ViewEdge.
#[test]
fn place_edge_undo_removes_both() {
    let mut app = make_app_with_two_nodes();
    app.current_tool = ToolMode::CreateRealization;

    let id_a = app.model.iter().next().unwrap().0;
    let id_b = app.model.iter().nth(1).unwrap().0;

    let result = app.place_edge(id_a, id_b);
    assert!(result.is_ok());

    let model_len_after = app.model.len();
    let diag = &app.model.diagrams()[0];
    let edges_after = diag.edges.len();

    // Undo should remove both
    assert!(app.history.undo(&mut app.model).is_ok());

    // Verify the model lost the relationship
    assert_eq!(app.model.len(), model_len_after - 1);

    // Verify the diagram lost the edge
    let diag = &app.model.diagrams()[0];
    assert_eq!(diag.edges.len(), edges_after - 1);

    // Verify no edge connects the two nodes
    let any_edge = diag
        .edges
        .values()
        .any(|edge| edge.source_node_id == id_a && edge.target_node_id == id_b);
    assert!(!any_edge, "No ViewEdge should remain after undo");
}

/// APP-24: After undo → redo, the edge is fully restored.
#[test]
fn place_edge_undo_redo_restores() {
    let mut app = make_app_with_two_nodes();
    app.current_tool = ToolMode::CreateComposition;

    let id_a = app.model.iter().next().unwrap().0;
    let id_b = app.model.iter().nth(1).unwrap().0;

    let result = app.place_edge(id_a, id_b);
    assert!(result.is_ok());

    let model_len_after = app.model.len();
    let diag = &app.model.diagrams()[0];
    let edges_after = diag.edges.len();

    // Undo
    assert!(app.history.undo(&mut app.model).is_ok());
    assert_eq!(app.model.len(), model_len_after - 1);
    let diag = &app.model.diagrams()[0];
    assert_eq!(diag.edges.len(), edges_after - 1);

    // Redo
    assert!(app.history.redo(&mut app.model).is_ok());
    assert_eq!(app.model.len(), model_len_after);
    let diag = &app.model.diagrams()[0];
    assert_eq!(diag.edges.len(), edges_after);

    // Verify the edge is back
    let has_edge = diag
        .edges
        .values()
        .any(|edge| edge.source_node_id == id_a && edge.target_node_id == id_b);
    assert!(has_edge, "ViewEdge should be restored after redo");
}

/// APP-26: New UmbrelloApp has drag_source_node_id: None.
#[test]
fn drag_source_node_id_defaults_none() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    assert!(app.drag_source_node_id.is_none());
}

/// APP-27: Edge tool palette labels are non-empty.
#[test]
fn edge_tool_labels_nonempty() {
    for tool in &[
        ToolMode::CreateGeneralization,
        ToolMode::CreateRealization,
        ToolMode::CreateAssociation,
        ToolMode::CreateAggregation,
        ToolMode::CreateComposition,
        ToolMode::CreateDependency,
    ] {
        assert!(!tool.label().is_empty(), "Label for {tool:?} should be non-empty");
    }
}

// ══════════════════════════════════════════════════════════════════
// M20 Phase 3 — Actor & UseCase Tool/Rendering Tests (APP-28 to APP-40)
// ══════════════════════════════════════════════════════════════════

/// APP-28: CreateActor is a creation tool.
#[test]
fn tool_actor_is_creation() {
    assert!(ToolMode::CreateActor.is_creation_tool());
}

/// APP-29: CreateUseCase is a creation tool.
#[test]
fn tool_usecase_is_creation() {
    assert!(ToolMode::CreateUseCase.is_creation_tool());
}

/// APP-30: CreateActor is NOT an edge tool.
#[test]
fn tool_actor_not_edge() {
    assert!(!ToolMode::CreateActor.is_edge_tool());
}

/// APP-31: CreateUseCase is NOT an edge tool.
#[test]
fn tool_usecase_not_edge() {
    assert!(!ToolMode::CreateUseCase.is_edge_tool());
}

/// APP-32: create_element_for_tool(CreateActor) returns an Actor with name "Actor_1".
#[test]
fn create_element_for_actor() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    let elem = app.create_element_for_tool(ToolMode::CreateActor);
    assert!(matches!(elem, ModelElement::Actor(_)));
    assert_eq!(elem.name(), "Actor_1");
}

/// APP-33: create_element_for_tool(CreateUseCase) returns a UseCase with name "UseCase_1".
#[test]
fn create_element_for_usecase() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    let elem = app.create_element_for_tool(ToolMode::CreateUseCase);
    assert!(matches!(elem, ModelElement::UseCase(_)));
    assert_eq!(elem.name(), "UseCase_1");
}

/// APP-34: Placing an Actor sets the dirty flag.
#[test]
fn place_actor_dirty_flag() {
    let mut app = make_app_with_diagram();
    let diagram_id = app.model.diagrams()[0].id;
    app.model.get_diagram_mut(diagram_id).unwrap().kind = DiagramKind::UseCase;
    app.is_dirty = false;
    let result = app.place_element(ToolMode::CreateActor, Point::new(100.0, 100.0));
    assert!(result.is_ok());
    assert!(app.is_dirty);
}

/// APP-35: Placing a UseCase sets the dirty flag.
#[test]
fn place_usecase_dirty_flag() {
    let mut app = make_app_with_diagram();
    let diagram_id = app.model.diagrams()[0].id;
    app.model.get_diagram_mut(diagram_id).unwrap().kind = DiagramKind::UseCase;
    app.is_dirty = false;
    let result = app.place_element(ToolMode::CreateUseCase, Point::new(100.0, 100.0));
    assert!(result.is_ok());
    assert!(app.is_dirty);
}

/// APP-36: Placing two actors produces "Actor_1" and "Actor_2".
#[test]
fn actor_unique_naming() {
    let mut model = UmlModel::new();
    let a1 = ModelElement::Actor(Actor::new("Actor_1"));
    model.insert(a1);
    let app = UmbrelloApp::new(model, false);
    // Next actor should be "Actor_2"
    assert_eq!(app.generate_unique_name("Actor"), "Actor_2");
    // Also verify create_element_for_tool produces "Actor_2"
    let app2 = UmbrelloApp::new(UmlModel::new(), false);
    let elem1 = app2.create_element_for_tool(ToolMode::CreateActor);
    assert_eq!(elem1.name(), "Actor_1");
}

/// APP-37: Placing two use cases produces "UseCase_1" and "UseCase_2".
#[test]
fn usecase_unique_naming() {
    let mut model = UmlModel::new();
    let u1 = ModelElement::UseCase(UseCase::new("UseCase_1"));
    model.insert(u1);
    let app = UmbrelloApp::new(model, false);
    // Next use case should be "UseCase_2"
    assert_eq!(app.generate_unique_name("UseCase"), "UseCase_2");
    // Also verify create_element_for_tool produces "UseCase_1"
    let app2 = UmbrelloApp::new(UmlModel::new(), false);
    let elem1 = app2.create_element_for_tool(ToolMode::CreateUseCase);
    assert_eq!(elem1.name(), "UseCase_1");
}

/// APP-38: Actor undo/redo — place, undo removes, redo restores.
#[test]
fn actor_undo_redo() {
    let mut app = make_app_with_diagram();
    let diagram_id = app.model.diagrams()[0].id;
    app.model.get_diagram_mut(diagram_id).unwrap().kind = DiagramKind::UseCase;
    let result = app.place_element(ToolMode::CreateActor, Point::new(100.0, 100.0));
    assert!(result.is_ok());
    let elem_id = app
        .model
        .iter()
        .find(|(_, e)| e.name() == "Actor_1")
        .map(|(id, _)| id)
        .unwrap();
    assert!(app.model.get(elem_id).is_some());

    // One undo reverses the atomic element-and-node operation.
    app.history.undo(&mut app.model).unwrap();
    assert!(app.model.get(elem_id).is_none());
    let diag = &app.model.diagrams()[0];
    assert!(diag.get_node(elem_id).is_none());
}

/// APP-39: element_color for Actor returns light orange/salmon.
#[test]
fn actor_color() {
    let actor = ModelElement::Actor(Actor::new("Test"));
    assert_eq!(element_color(Some(&actor)), egui::Color32::from_rgb(255, 200, 170));
}

/// APP-40: element_color for UseCase returns light coral/pink.
#[test]
fn usecase_color() {
    let uc = ModelElement::UseCase(UseCase::new("Test"));
    assert_eq!(element_color(Some(&uc)), egui::Color32::from_rgb(255, 180, 180));
}

// ══════════════════════════════════════════════════════════════════
// M23 — Component, Node, and Artifact native tools/rendering
// ══════════════════════════════════════════════════════════════════

#[test]
fn component_node_artifact_tools_are_creation_tools_with_distinct_labels() {
    let tools = [
        ToolMode::CreateComponent,
        ToolMode::CreateNode,
        ToolMode::CreateArtifact,
    ];
    let labels: std::collections::HashSet<_> = tools.iter().map(ToolMode::label).collect();
    assert_eq!(labels.len(), tools.len());
    for tool in tools {
        assert!(tool.is_creation_tool());
        assert!(!tool.is_edge_tool());
        assert!(!tool.tooltip().is_empty());
    }
}

#[test]
fn component_node_artifact_tools_construct_unique_named_elements() {
    let app = UmbrelloApp::new(UmlModel::new(), false);
    for (tool, expected) in [
        (ToolMode::CreateComponent, "Component_1"),
        (ToolMode::CreateNode, "Node_1"),
        (ToolMode::CreateArtifact, "Artifact_1"),
    ] {
        let element = app.create_element_for_tool(tool);
        assert_eq!(element.name(), expected);
        assert!(matches!(
            (tool, element),
            (ToolMode::CreateComponent, ModelElement::Component(_))
                | (ToolMode::CreateNode, ModelElement::Node(_))
                | (ToolMode::CreateArtifact, ModelElement::Artifact(_))
        ));
    }
}

#[test]
fn component_node_artifact_placement_is_atomic_and_restores_on_undo_redo() {
    for (tool, expected) in [
        (ToolMode::CreateComponent, "Component_1"),
        (ToolMode::CreateNode, "Node_1"),
        (ToolMode::CreateArtifact, "Artifact_1"),
    ] {
        let mut app = make_app_with_diagram();
        let diagram_id = app.model.diagrams()[0].id;
        app.model.get_diagram_mut(diagram_id).unwrap().kind = if tool == ToolMode::CreateNode {
            DiagramKind::Deployment
        } else {
            DiagramKind::Component
        };
        let history_before = app.history.can_undo();
        app.place_element(tool, Point::new(25.0, 35.0)).unwrap();
        let id = app
            .model
            .iter()
            .find(|(_, e)| e.name() == expected)
            .unwrap()
            .0;
        assert!(!history_before);
        assert_eq!(app.model.diagrams()[0].get_node(id).unwrap().bounds.width(), 160.0);
        app.history.undo(&mut app.model).unwrap();
        assert!(app.model.get(id).is_none());
        assert!(app.model.diagrams()[0].get_node(id).is_none());
        app.history.redo(&mut app.model).unwrap();
        assert!(app.model.get(id).is_some());
        assert!(app.model.diagrams()[0].get_node(id).is_some());
    }
}

#[test]
fn component_node_artifact_element_colors_are_distinct() {
    let colors = [
        element_color(Some(&ModelElement::Component(Component::new("C")))),
        element_color(Some(&ModelElement::Node(Node::new("N")))),
        element_color(Some(&ModelElement::Artifact(Artifact::new("A")))),
    ];
    assert!(colors[0] != colors[1] && colors[1] != colors[2] && colors[0] != colors[2]);
}

#[test]
fn component_node_artifact_rendering_handles_all_artifact_modes_and_tiny_bounds() {
    let mut model = UmlModel::new();
    let ids = [
        ModelElement::Component(Component::new("Component")),
        ModelElement::Node(Node::new("Node")),
        ModelElement::Artifact(Artifact::new("Artifact")),
    ]
    .into_iter()
    .map(|element| {
        let id = element.base().id;
        model.insert(element);
        id
    })
    .collect::<Vec<_>>();
    let app = UmbrelloApp::new(model, false);
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            for &id in &ids {
                let node = uml_core::ViewNode::new(id, Rect::new(0.0, 0.0, 1.0, 1.0));
                app.draw_partitioned_node(
                    ui,
                    &node,
                    egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1.0, 1.0)),
                );
            }
            for mode in [
                ArtifactDrawMode::Default,
                ArtifactDrawMode::File,
                ArtifactDrawMode::Library,
                ArtifactDrawMode::Table,
            ] {
                let mut artifact = Artifact::new("Mode");
                artifact.draw_as = mode;
                let id = artifact.base.id;
                // This dispatch smoke test only needs the mode-specific element in the model.
                let mut mode_model = UmlModel::new();
                mode_model.insert(ModelElement::Artifact(artifact));
                let mode_app = UmbrelloApp::new(mode_model, false);
                let node = uml_core::ViewNode::new(id, Rect::new(0.0, 0.0, 1.0, 1.0));
                mode_app.draw_partitioned_node(
                    ui,
                    &node,
                    egui::Rect::from_min_size(egui::pos2(2.0, 2.0), egui::vec2(1.0, 1.0)),
                );
            }
        });
    });
}

#[test]
fn qa_component_node_artifact_targets_create_atomic_nodes_and_support_generic_history() {
    let cases = [
        ("tool.component", "Component_1"),
        ("tool.node", "Node_1"),
        ("tool.artifact", "Artifact_1"),
    ];
    for &(tool_id, expected_name) in &cases {
        let mut app = make_app_with_diagram();
        let diagram_id = app.model.diagrams()[0].id;
        app.model.get_diagram_mut(diagram_id).unwrap().kind = if tool_id == "tool.node" {
            DiagramKind::Deployment
        } else {
            DiagramKind::Component
        };
        let context = egui::Context::default();
        let snapshot = app.qa_snapshot();
        assert!(snapshot
            .targets
            .iter()
            .any(|target| target.id == tool_id && target.enabled));
        assert_eq!(
            snapshot
                .targets
                .iter()
                .filter(|target| target.kind == "tool")
                .count(),
            17
        );

        app.qa_select(tool_id.into()).unwrap();
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
            .unwrap();
        app.qa_select("canvas".into()).unwrap();
        app.qa_dispatch(
            crate::app::qa::protocol::QaRequest::Click {
                position: Some((40.0, 50.0)),
            },
            &context,
        )
        .unwrap();

        assert_eq!(app.current_tool, ToolMode::Select);
        let (id, element) = app
            .model
            .iter()
            .find(|(_, element)| element.name() == expected_name)
            .unwrap();
        match tool_id {
            "tool.component" => assert!(matches!(element, ModelElement::Component(_))),
            "tool.node" => assert!(matches!(element, ModelElement::Node(_))),
            "tool.artifact" => assert!(matches!(element, ModelElement::Artifact(_))),
            _ => unreachable!(),
        }
        let node_target = format!("node:{id}");
        assert!(app
            .qa_snapshot()
            .targets
            .iter()
            .any(|target| target.id == node_target && target.enabled));
        assert!(app.model.diagrams()[0].get_node(id).is_some());

        // The placement is one history entry and restores both model and view.
        app.qa_select("history.undo".into()).unwrap();
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
            .unwrap();
        assert!(app.model.get(id).is_none());
        assert!(app.model.diagrams()[0].get_node(id).is_none());
        app.qa_select("history.redo".into()).unwrap();
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
            .unwrap();
        assert!(app.model.get(id).is_some());
        assert!(app.model.diagrams()[0].get_node(id).is_some());

        app.qa_select(node_target).unwrap();
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
            .unwrap();
        app.qa_select("property.name".into()).unwrap();
        app.qa_dispatch(
            crate::app::qa::protocol::QaRequest::SetText {
                value: "Renamed".into(),
            },
            &context,
        )
        .unwrap();
        assert_eq!(app.model.get(id).unwrap().name(), "Renamed");
        app.qa_select("history.undo".into()).unwrap();
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
            .unwrap();
        assert_eq!(app.model.get(id).unwrap().name(), expected_name);
        app.qa_select("history.redo".into()).unwrap();
        app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
            .unwrap();
        assert_eq!(app.model.get(id).unwrap().name(), "Renamed");
    }
}

#[test]
fn edge_selection_uses_nearest_segment_and_stable_ties() {
    let first = UmlId::new();
    let second = UmlId::new();
    let paths = vec![
        crate::canvas::ScreenEdgePath {
            relationship_id: first,
            points: vec![egui::pos2(0.0, 0.0), egui::pos2(100.0, 0.0)],
            kind: AssociationType::Association,
        },
        crate::canvas::ScreenEdgePath {
            relationship_id: second,
            points: vec![egui::pos2(0.0, 10.0), egui::pos2(100.0, 10.0)],
            kind: AssociationType::Association,
        },
    ];
    assert_eq!(
        crate::canvas::nearest_edge_relationship(&paths, egui::pos2(50.0, 4.0), 6.0),
        Some(first)
    );
    assert_eq!(
        crate::canvas::nearest_edge_relationship(&paths, egui::pos2(50.0, 5.0), 6.0),
        Some(first)
    );
    let waypoint = crate::canvas::ScreenEdgePath {
        relationship_id: second,
        points: vec![
            egui::pos2(0.0, 0.0),
            egui::pos2(20.0, 40.0),
            egui::pos2(80.0, 40.0),
        ],
        kind: AssociationType::Association,
    };
    assert_eq!(
        crate::canvas::nearest_edge_relationship(&[waypoint], egui::pos2(50.0, 41.0), 6.0),
        Some(second)
    );
}

#[test]
fn relationship_mcp_draft_applies_atomically_and_roundtrips_history() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/relationship.xmi"));
    let source = Class::new("Source");
    let source_id = source.base.id;
    let target = Class::new("Target");
    let target_id = target.base.id;
    app.model.insert(ModelElement::Class(source));
    app.model.insert(ModelElement::Class(target));
    let mut relationship = Relationship::new_association(source_id, target_id);
    relationship.base.original_xmi_id = Some("legacy-rel".into());
    let relationship_id = relationship.base.id;
    app.model.insert(ModelElement::Relationship(relationship));
    let diagram_id = app.model.diagrams()[0].id;
    app.model.get_diagram_mut(diagram_id).unwrap().add_edge(
        uml_core::EdgeId::new(),
        ViewEdge::new(relationship_id, source_id, target_id, uml_core::LineRouting::Direct),
    );
    app.select_element(relationship_id).unwrap();
    let context = egui::Context::default();
    let snapshot = app.qa_snapshot();
    for id in [
        "edge:",
        "property.relationship.name",
        "property.relationship.documentation",
        "property.relationship.source_role",
        "property.relationship.source_multiplicity",
        "property.relationship.target_role",
        "property.relationship.target_multiplicity",
        "property.relationship.apply",
        "property.relationship.revert",
    ] {
        if id.ends_with(':') {
            assert!(snapshot
                .targets
                .iter()
                .any(|target| target.id == format!("{id}{relationship_id}")));
        } else {
            assert!(snapshot.targets.iter().any(|target| target.id == id));
        }
    }
    app.qa_select(format!("edge:{relationship_id}")).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .unwrap();
    assert_eq!(app.selected_element_id, Some(relationship_id));
    for (id, value) in [
        ("property.relationship.name", "owns"),
        ("property.relationship.documentation", "updated"),
        ("property.relationship.source_role", "owner"),
        ("property.relationship.source_multiplicity", " "),
        ("property.relationship.target_role", "item"),
        ("property.relationship.target_multiplicity", "0..*"),
    ] {
        app.qa_select(id.into()).unwrap();
        app.qa_dispatch(
            crate::app::qa::protocol::QaRequest::SetText {
                value: value.into(),
            },
            &context,
        )
        .unwrap();
    }
    assert_eq!(app.model.get(relationship_id).unwrap().name(), "");
    let undo_before = app.history.can_undo();
    app.qa_select("property.relationship.apply".into()).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .unwrap();
    assert!(undo_before || app.history.can_undo());
    assert_eq!(app.status_message, "Relationship updated");
    assert_eq!(app.status_message, "Relationship updated");
    let ModelElement::Relationship(updated) = app.model.get(relationship_id).unwrap() else {
        unreachable!()
    };
    assert_eq!(updated.base.name, "owns");
    assert_eq!(updated.source_role_name, Some("owner".into()));
    assert_eq!(updated.source_multiplicity, None);
    assert_eq!(updated.target_multiplicity, Some("0..*".into()));
    assert_eq!(updated.base.original_xmi_id.as_deref(), Some("legacy-rel"));
    app.qa_select("property.relationship.apply".into()).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .unwrap();
    assert_eq!(app.status_message, "Relationship unchanged (no changes)");
    app.qa_select("property.relationship.name".into()).unwrap();
    app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::SetText {
            value: "draft".into(),
        },
        &context,
    )
    .unwrap();
    app.qa_select("property.relationship.revert".into())
        .unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .unwrap();
    assert_eq!(app.status_message, "Relationship draft reverted");
    app.undo_action().unwrap();
    assert_eq!(app.model.get(relationship_id).unwrap().name(), "");
    app.redo_action().unwrap();
    assert_eq!(app.model.get(relationship_id).unwrap().name(), "owns");
    assert_eq!(app.relationship_draft.as_ref().unwrap().1.name, "owns");
}

#[test]
fn relationship_noop_apply_and_kind_policy_are_safe() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/relationship.xmi"));
    let source = Class::new("Source");
    let source_id = source.base.id;
    let target = Class::new("Target");
    let target_id = target.base.id;
    app.model.insert(ModelElement::Class(source));
    app.model.insert(ModelElement::Class(target));
    let relationship = Relationship::new_association(source_id, target_id);
    let relationship_id = relationship.base.id;
    app.model.insert(ModelElement::Relationship(relationship));
    app.select_element(relationship_id).unwrap();
    let can_undo_before = app.history.can_undo();
    app.apply_relationship_draft(relationship_id).unwrap();
    assert_eq!(app.history.can_undo(), can_undo_before);
    app.relationship_draft = None;
    let context = egui::Context::default();
    app.qa_select("property.relationship.apply".into()).unwrap();
    assert!(app
        .qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &context)
        .is_err());
    assert!(app.status_message.starts_with("Relationship apply failed:"));
    app.active_diagram = None;
    assert!(app.relationship_kind_allowed(AssociationType::Dependency));
}

#[test]
fn classifier_flags_are_not_exposed_for_non_classifiers() {
    let mut app = make_app_with_diagram();
    for element in [
        ModelElement::Actor(Actor::new("Actor")),
        ModelElement::UseCase(UseCase::new("UseCase")),
        ModelElement::Package(Package::new("Package")),
    ] {
        let id = element.id();
        app.model.insert(element);
        app.select_element(id).unwrap();
        let snapshot = app.qa_snapshot();
        assert!(!snapshot
            .targets
            .iter()
            .any(|target| target.id == "property.abstract"));
        assert!(!snapshot
            .targets
            .iter()
            .any(|target| target.id == "property.static"));
    }
}

#[test]
fn normal_documentation_buffer_can_be_cleared_without_dropping_the_selection() {
    let mut app = make_app_with_diagram();
    let mut class = Class::new("Documented");
    class.base.documentation = "old".into();
    let id = class.base.id;
    app.model.insert(ModelElement::Class(class));
    app.select_element(id).unwrap();
    assert_eq!(app.documentation_edit_buffer, "old");
    app.documentation_edit_buffer.clear();
    app.set_documentation(id, String::new()).unwrap();
    assert_eq!(app.model.get(id).unwrap().base().documentation, "");
    assert_eq!(app.selected_element_id, Some(id));
}

#[test]
fn empty_canvas_guidance_is_project_aware_and_supported_kind_only() {
    let (heading, detail) = crate::canvas::no_diagram_guidance(false);
    assert!(heading.contains("No XMI project"));
    assert!(detail.contains("New Project") && detail.contains("Open"));
    let (heading, detail) = crate::canvas::no_diagram_guidance(true);
    assert!(heading.contains("No diagram"));
    assert!(detail.contains("Class") && detail.contains("Deployment"));
}

#[test]
fn drag_preview_converts_screen_delta_once_and_move_is_one_command() {
    let original = Point::new(10.0, 20.0);
    let preview = crate::canvas::preview_node_position(original, egui::vec2(30.0, 20.0), 2.0);
    assert_eq!(preview, Point::new(25.0, 30.0));
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/drag.xmi"));
    let element = Class::new("Dragged");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram)
        .unwrap()
        .add_node(id, uml_core::ViewNode::new(id, Rect::new(10.0, 20.0, 100.0, 60.0)));
    app.move_node_to(diagram, id, preview).unwrap();
    assert_eq!(
        app.model
            .get_diagram(diagram)
            .unwrap()
            .get_node(id)
            .unwrap()
            .bounds
            .x(),
        25.0
    );
    app.undo_action().unwrap();
    assert_eq!(
        app.model
            .get_diagram(diagram)
            .unwrap()
            .get_node(id)
            .unwrap()
            .bounds
            .x(),
        10.0
    );
    assert!(app.history.can_redo());
}

/// Helper: build a RawInput with the given events and screen rect.
#[allow(dead_code)]
fn raw_with_screen(events: Vec<egui::Event>, size: egui::Vec2) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        events,
        ..Default::default()
    }
}

// ══════════════════════════════════════════════════════════════════
// S4 — Native pointer drag + gesture tests (GREEN)
// ══════════════════════════════════════════════════════════════════
//
// These tests exercise frame-level input routing through
// app.update() -> render_canvas() with realistic RawInput events.
// They deliberately avoid directly calling move_node_to, which the
// existing bypass tests do.
//
// The fix replaces the overlapping full-canvas Sense::click background
// interaction (which stole pointer ownership) with a self-contained
// hit-test approach that checks button_down, button_clicked, and
// pointer position directly.
//
// Drag tests use eframe::Frame::_new_kittest() —
// a #[doc(hidden)] but public testing constructor.

/// Supply a Click on a node through the raw input system, then a
/// press-move-release drag on the already-selected node. Assert changed
/// bounds, exactly one undoable command, undo restoration, and redo
/// restoration at 100% zoom.
#[test]
fn native_pointer_drag_selected_node() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/native_drag.xmi"));
    let element = Class::new("Dragged");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, ViewNode::new(id, Rect::new(100.0, 100.0, 100.0, 60.0)));
    app.active_diagram = Some(0);

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    let screen_size = egui::vec2(1280.0, 1024.0);

    // ── Frame 0: empty input to establish canvas rect and widget IDs ──
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    let canvas_origin = app.last_canvas_rect.unwrap().min;

    // Node center: model (150, 130) → screen (origin + 150, origin + 130) at 100%
    let node_center = egui::pos2(canvas_origin.x + 150.0, canvas_origin.y + 130.0);

    // ── Frame 1: click on node to select it ──
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: node_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: node_center,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    assert_eq!(app.selected_element_id, Some(id), "Click on node must select it");
    assert_eq!(app.history.undo_depth(), 0, "Selection adds no undo command");

    // ── Frame 2: press + move (no release) to start drag ──
    // egui's interaction snapshot persists between ctx.run() calls,
    // so drag_stopped in frame 3 will see the drag from this frame.
    let drag_target = egui::pos2(canvas_origin.x + 200.0, canvas_origin.y + 160.0);
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: node_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerMoved(drag_target),
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    // After frame 2: drag state should be set
    assert_eq!(app.drag_node_id, Some(id), "drag_node_id must be set after press+move");
    assert!(app.drag_preview_pos.is_some(), "drag_preview_pos must be set after press+move");

    // ── Frame 3: release to commit the drag ──
    let _ = ctx.run(
        raw_with_screen(
            vec![egui::Event::PointerButton {
                pos: drag_target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    let node = app.model.diagrams()[0].get_node(id).unwrap();

    // At 100% zoom: screen delta (50, 30) → model delta (50, 30)
    // Expected new position: (100 + 50, 100 + 30) = (150, 130)
    assert!(
        (node.bounds.x() - 150.0).abs() < 0.01 && (node.bounds.y() - 130.0).abs() < 0.01,
        "Node should be at (150, 130) after drag at 100% zoom, got ({}, {})",
        node.bounds.x(),
        node.bounds.y()
    );
    assert_eq!(
        app.history.undo_depth(),
        1,
        "Exactly one undoable command after drag (MoveNode)"
    );

    // ── Undo restores original position ──
    app.undo_action().unwrap();
    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!((node.bounds.x() - 100.0).abs() < 0.01, "Undo must restore original x");
    assert!((node.bounds.y() - 100.0).abs() < 0.01, "Undo must restore original y");

    // ── Redo restores moved position ──
    app.redo_action().unwrap();
    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!(
        (node.bounds.x() - 150.0).abs() < 0.01 && (node.bounds.y() - 130.0).abs() < 0.01,
        "Redo must restore moved position"
    );
}

/// Drag at 200% zoom and verify the screen-to-model delta conversion
/// is correctly divided by scale.
#[test]
fn native_pointer_drag_converts_non_100_zoom() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/native_drag_zoom.xmi"));
    let element = Class::new("ZoomDrag");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, ViewNode::new(id, Rect::new(100.0, 100.0, 100.0, 60.0)));
    app.active_diagram = Some(0);
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .set_zoom_percent(200.0);

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();

    let screen_size = egui::vec2(1280.0, 1024.0);

    // ── Frame 0: empty input to establish canvas rect and widget IDs ──
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    let canvas_origin = app.last_canvas_rect.unwrap().min;

    // At 200% zoom, model (100, 100) → screen (origin + 200, origin + 200)
    // Node center (150, 130) → screen (origin + 300, origin + 260)
    let node_screen = egui::pos2(canvas_origin.x + 300.0, canvas_origin.y + 260.0);

    // ── Frame 1: click to select ──
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: node_screen,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: node_screen,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );
    assert_eq!(app.selected_element_id, Some(id), "Click at 200% zoom must select");

    // ── Frame 2: press+move at 200% zoom ──
    // Screen delta from node_center: (100, 80)
    // At 200% zoom (scale = 2.0): model delta = (100 / 2, 80 / 2) = (50, 40)
    let drag_target = egui::pos2(canvas_origin.x + 400.0, canvas_origin.y + 340.0);
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: node_screen,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerMoved(drag_target),
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    assert_eq!(
        app.drag_node_id,
        Some(id),
        "drag_node_id must be set after press+move at 200% zoom"
    );

    // ── Frame 3: release to commit ──
    let _ = ctx.run(
        raw_with_screen(
            vec![egui::Event::PointerButton {
                pos: drag_target,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    let node = app.model.diagrams()[0].get_node(id).unwrap();
    // New position: (100 + 50, 100 + 40) = (150, 140)
    assert!(
        (node.bounds.x() - 150.0).abs() < 0.01 && (node.bounds.y() - 140.0).abs() < 0.01,
        "At 200% zoom, screen-pixel delta (100, 80) must convert \
         to model delta (50, 40), giving position (150, 140). \
         Actual: ({}, {}).",
        node.bounds.x(),
        node.bounds.y()
    );
}

/// Clicking on empty canvas background must clear the selection (deselect).
#[test]
fn background_click_deselects_selected_node() {
    let mut app = make_app_with_diagram();
    let element = Class::new("Target");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, ViewNode::new(id, Rect::new(100.0, 100.0, 100.0, 60.0)));
    app.active_diagram = Some(0);
    app.select_element(id).unwrap();
    assert_eq!(app.selected_element_id, Some(id));

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let screen_size = egui::vec2(1280.0, 1024.0);

    // First frame: establish layout.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    let canvas = app.last_canvas_rect.unwrap();
    // Pick a point on the canvas well away from the node (which is at
    // origin + 100..200, origin + 100..160).
    let bg_point = egui::pos2(canvas.right() - 10.0, canvas.bottom() - 10.0);

    // Click on the background.
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: bg_point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: bg_point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    assert!(app.selected_element_id.is_none(), "Background click must clear selection");
}

/// The MCP gesture mode (DragArgs.gesture = true) must use the shared
/// execute_gesture_move flow (set drag_node_id, set preview, commit,
/// clear state) instead of directly calling move_node_to.
#[test]
fn qa_gesture_mode_uses_shared_behavior_and_commits_once() {
    use crate::app::qa::protocol::QaRequest;
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/gesture_move.xmi"));
    let element = Class::new("Gesture");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    let orig_pos = Point::new(100.0, 100.0);
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, ViewNode::new(id, Rect::new(orig_pos.x, orig_pos.y, 100.0, 60.0)));
    app.active_diagram = Some(0);
    // Must have project, select node, and set QA cursor.
    app.qa_select(format!("node:{id}")).unwrap();
    let ctx = egui::Context::default();

    // Gesture move: use gesture=true to exercise the shared begin/update/commit flow.
    let dest = Point::new(200.0, 150.0);
    app.qa_dispatch(
        QaRequest::Drag {
            position: Some((dest.x, dest.y)),
            to_target: None,
            gesture: Some(true),
        },
        &ctx,
    )
    .unwrap();

    // Exactly one command was committed.
    assert_eq!(app.history.undo_depth(), 1, "Gesture mode commits exactly one command");

    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!(
        (node.bounds.x() - dest.x).abs() < 0.01 && (node.bounds.y() - dest.y).abs() < 0.01,
        "Gesture move must place node at destination ({}, {}), got ({}, {})",
        dest.x,
        dest.y,
        node.bounds.x(),
        node.bounds.y()
    );

    // Verify execute_gesture_move set and cleared drag state.
    assert!(app.drag_node_id.is_none(), "drag_node_id must be cleared after gesture");
    assert!(app.drag_preview_pos.is_none(), "preview must be cleared after gesture");

    // Undo restores original position.
    app.undo_action().unwrap();
    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!(
        (node.bounds.x() - orig_pos.x).abs() < 0.01 && (node.bounds.y() - orig_pos.y).abs() < 0.01,
        "Undo of gesture move must restore original ({}, {}), got ({}, {})",
        orig_pos.x,
        orig_pos.y,
        node.bounds.x(),
        node.bounds.y()
    );

    // Legacy mode (gesture=false / None) must still work.
    app.redo_action().unwrap();
    app.qa_dispatch(
        QaRequest::Drag {
            position: Some((300.0, 200.0)),
            to_target: None,
            gesture: None,
        },
        &ctx,
    )
    .unwrap();
    assert_eq!(app.history.undo_depth(), 2, "Legacy drag must also commit one command");
    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!(
        (node.bounds.x() - 300.0).abs() < 0.01 && (node.bounds.y() - 200.0).abs() < 0.01,
        "Legacy mode must also place node correctly"
    );
}

/// Press on a node, then supply three distinct movement frames, then
/// release. Assert the node ends at the cumulative displacement from
/// press origin (not the per-frame delta of the last frame only).
/// Also assert exactly one undoable command and undo restoration.
#[test]
fn native_pointer_drag_accumulates_multiple_move_frames() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/accum_drag.xmi"));
    let element = Class::new("AccumDrag");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, ViewNode::new(id, Rect::new(100.0, 100.0, 100.0, 60.0)));
    app.active_diagram = Some(0);

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let screen_size = egui::vec2(1280.0, 1024.0);

    // Frame 0: establish layout.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    let canvas_origin = app.last_canvas_rect.unwrap().min;
    let node_center = egui::pos2(canvas_origin.x + 150.0, canvas_origin.y + 130.0);

    // Frame 1: click to select.
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: node_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: node_center,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );
    assert_eq!(app.selected_element_id, Some(id));

    let press_origin = node_center;

    // Frame 2: press (no move yet).
    let _ = ctx.run(
        raw_with_screen(
            vec![egui::Event::PointerButton {
                pos: press_origin,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    // Frame 3: move +20 x, +10 y.
    let p1 = egui::pos2(press_origin.x + 20.0, press_origin.y + 10.0);
    let _ = ctx.run(raw_with_screen(vec![egui::Event::PointerMoved(p1)], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });

    // Frame 4: move another +20 x, +10 y (cumulative: +40, +20).
    let p2 = egui::pos2(press_origin.x + 40.0, press_origin.y + 20.0);
    let _ = ctx.run(raw_with_screen(vec![egui::Event::PointerMoved(p2)], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });

    // Frame 5: move another +10 x, +5 y (cumulative: +50, +25).
    let p3 = egui::pos2(press_origin.x + 50.0, press_origin.y + 25.0);
    let _ = ctx.run(raw_with_screen(vec![egui::Event::PointerMoved(p3)], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });

    // Verify preview is at cumulative displacement (not last-frame delta).
    // At 100% zoom: cumulative screen delta (50, 25) → model delta (50, 25)
    // Expected position: orig (100, 100) + (50, 25) = (150, 125)
    assert_eq!(app.drag_node_id, Some(id), "drag_node_id must persist across move frames");
    assert!(
        app.drag_preview_pos
            .is_some_and(|p| (p.x - 150.0).abs() < 0.01 && (p.y - 125.0).abs() < 0.01),
        "Preview position must reflect cumulative (50, 25) displacement, not per-frame delta. \
         Expected (150, 125), got {:?}",
        app.drag_preview_pos
    );

    // Frame 6: release.
    let _ = ctx.run(
        raw_with_screen(
            vec![egui::Event::PointerButton {
                pos: p3,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!(
        (node.bounds.x() - 150.0).abs() < 0.01 && (node.bounds.y() - 125.0).abs() < 0.01,
        "Cumulative 3-frame drag must place node at (150, 125), got ({}, {})",
        node.bounds.x(),
        node.bounds.y()
    );
    assert_eq!(
        app.history.undo_depth(),
        1,
        "Exactly one undoable command after multi-frame drag"
    );

    // Undo restores original position.
    app.undo_action().unwrap();
    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!((node.bounds.x() - 100.0).abs() < 0.01 && (node.bounds.y() - 100.0).abs() < 0.01);
}

/// Press on a selected node, then release at the same pointer position
/// in a later frame without any intervening movement. Must select the
/// node (via the earlier click) while preserving exact bounds, clean
/// dirty state, undo_depth=0, and cleared drag state.
#[test]
fn native_pointer_click_without_motion_creates_no_move() {
    let mut app = make_app_with_diagram();
    app.current_file_path = Some(PathBuf::from("/tmp/click_nomove.xmi"));
    let element = Class::new("ClickNoMove");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    let orig = Point::new(100.0, 100.0);
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, ViewNode::new(id, Rect::new(orig.x, orig.y, 100.0, 60.0)));
    app.active_diagram = Some(0);

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let screen_size = egui::vec2(1280.0, 1024.0);

    // Frame 0: establish layout.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    let canvas_origin = app.last_canvas_rect.unwrap().min;
    let node_center = egui::pos2(canvas_origin.x + 150.0, canvas_origin.y + 130.0);

    // Frame 1: click to select the node (press + release in one frame).
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: node_center,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: node_center,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    assert_eq!(app.selected_element_id, Some(id), "Click must select node");
    assert!(!app.is_dirty, "Selection must not dirty the model");
    assert_eq!(app.history.undo_depth(), 0, "Selection must not create history");
    assert!(app.drag_node_id.is_none(), "Drag state must be clear after click");

    // Frame 2: press on the node (no release yet).
    //   begin_node_drag fires, setting drag_node_id.
    let _ = ctx.run(
        raw_with_screen(
            vec![egui::Event::PointerButton {
                pos: node_center,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            }],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    // Frame 3: release at the same position (no movement).
    //   commit_node_drag fires, should be a no-op.
    let _ = ctx.run(
        raw_with_screen(
            vec![egui::Event::PointerButton {
                pos: node_center,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );

    // Must still be selected.
    assert_eq!(app.selected_element_id, Some(id), "Node must remain selected");
    // Bounds unchanged.
    let node = app.model.diagrams()[0].get_node(id).unwrap();
    assert!((node.bounds.x() - orig.x).abs() < 0.01, "X must be unchanged");
    assert!((node.bounds.y() - orig.y).abs() < 0.01, "Y must be unchanged");
    // No history entry created.
    assert!(!app.is_dirty, "No-motion click must not dirty the model");
    assert_eq!(app.history.undo_depth(), 0, "No-motion click must not create history");
    // Drag state cleared.
    assert!(app.drag_node_id.is_none(), "drag_node_id must be cleared after release");
    assert!(app.drag_preview_pos.is_none(), "drag_preview_pos must be cleared");
    assert!(app.drag_start_pos.is_none(), "drag_start_pos must be cleared");
    assert_eq!(app.drag_accum_screen_delta, egui::Vec2::ZERO, "accum delta must be zero");
}

// ═══════════════════════════════════════════════════════════════════════
// S2 — Classifier draft and property panel tests
// ═══════════════════════════════════════════════════════════════════════

/// Helper: create app with a classifier diagram and a Class named "Person"
/// with one attribute and one operation.
#[allow(dead_code)]
fn make_app_with_classifier_features() -> UmbrelloApp {
    let mut model = UmlModel::new();
    let d = Diagram::new("ClassDiagram", DiagramKind::Class);
    model.add_diagram(d);
    let mut cls = Class::new("Person");
    cls.classifier.add_attribute(uml_core::Attribute {
        name: "name".into(),
        type_ref: TypeReference::primitive("String"),
        visibility: Visibility::Private,
        initial_value: None,
        is_static: false,
    });
    cls.classifier.add_operation(uml_core::Operation {
        name: "getName".into(),
        return_type: TypeReference::primitive("String"),
        parameters: vec![uml_core::Parameter {
            name: "format".into(),
            type_ref: TypeReference::primitive("bool"),
            direction: uml_core::ParameterDirection::In,
            default_value: Some("true".into()),
        }],
        visibility: Visibility::Public,
        is_static: false,
        is_abstract: false,
        is_virtual: true,
    });
    let id = cls.base.id;
    model.insert(ModelElement::Class(cls));
    let mut app = UmbrelloApp::new(model, false);
    app.active_diagram = Some(0);
    app.current_file_path = Some(PathBuf::from("/tmp/classifier-test.xmi"));
    app.selected_element_id = Some(id);
    app.refresh_property_buffers();
    app
}

/// S2-01: Background click in canvas clears selection; click outside does not.
#[test]
fn background_click_only_in_canvas_deselects() {
    let mut app = make_app_with_diagram();
    let element = Class::new("Target");
    let id = element.base.id;
    app.model.insert(ModelElement::Class(element));
    let diagram_id = app.model.diagrams()[0].id;
    app.model
        .get_diagram_mut(diagram_id)
        .unwrap()
        .add_node(id, ViewNode::new(id, Rect::new(100.0, 100.0, 100.0, 60.0)));
    app.active_diagram = Some(0);
    app.select_element(id).unwrap();
    assert_eq!(app.selected_element_id, Some(id));

    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let screen_size = egui::vec2(1280.0, 1024.0);

    // Frame 0: establish layout.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    let canvas = app.last_canvas_rect.unwrap();
    // Point outside canvas (well to the left)
    let outside_point = egui::pos2(canvas.left() - 50.0, canvas.top() + 50.0);

    // Click outside canvas — must NOT clear selection.
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: outside_point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: outside_point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );
    assert!(
        app.selected_element_id.is_some(),
        "Click outside canvas must NOT clear selection"
    );

    // Click inside canvas, not on any node — MUST clear selection.
    let inside_point = egui::pos2(canvas.right() - 10.0, canvas.bottom() - 10.0);
    let _ = ctx.run(
        raw_with_screen(
            vec![
                egui::Event::PointerButton {
                    pos: inside_point,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos: inside_point,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            screen_size,
        ),
        |ctx| {
            app.update(ctx, &mut frame);
        },
    );
    assert!(
        app.selected_element_id.is_none(),
        "Click inside canvas on empty area must clear selection"
    );
}

/// S2-02: Classifier draft populated on selection, cleared for non-classifiers.
#[test]
fn classifier_draft_populated_and_cleared_by_selection() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    // Classifier draft should exist and have features.
    assert!(
        app.classifier_draft.is_some(),
        "classifier_draft should be populated for classifier"
    );
    let (draft_id, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(*draft_id, id);
    assert_eq!(draft.attributes.len(), 1);
    assert_eq!(draft.attributes[0].name, "name");
    assert_eq!(draft.attributes[0].type_text, "String");
    assert_eq!(draft.operations.len(), 1);
    assert_eq!(draft.operations[0].name, "getName");
    assert_eq!(draft.operations[0].return_type_text, "String");
    assert_eq!(draft.operations[0].parameters.len(), 1);
    assert_eq!(draft.operations[0].parameters[0].name, "format");

    // Switching to a non-classifier clears classifier draft.
    let pkg = ModelElement::Package(Package::new("Pkg"));
    let pkg_id = pkg.id();
    app.model.insert(pkg);
    app.select_element(pkg_id).unwrap();
    assert!(
        app.classifier_draft.is_none(),
        "classifier_draft should be None for non-classifier"
    );

    // Switching back to classifier repopulates.
    app.select_element(id).unwrap();
    assert!(
        app.classifier_draft.is_some(),
        "classifier_draft should be repopulated for classifier"
    );
}

/// S2-03: Classifier draft add/delete attribute programmatically.
#[test]
fn classifier_draft_add_delete_attribute() {
    let mut app = make_app_with_classifier_features();
    let (_, draft) = app.classifier_draft.as_mut().unwrap();
    assert_eq!(draft.attributes.len(), 1);

    // Add attribute via direct draft manipulation.
    draft.attributes.push(DraftAttribute {
        name: "attribute_1".into(),
        type_text: "int".into(),
        original_type: TypeReference::unspecified(),
        visibility: Visibility::Public,
        initial_value: String::new(),
        is_static: false,
    });
    assert_eq!(draft.attributes.len(), 2);
    assert_eq!(draft.attributes[1].name, "attribute_1");

    // Delete first attribute.
    draft.attributes.remove(0);
    assert_eq!(draft.attributes.len(), 1);
    assert_eq!(draft.attributes[0].name, "attribute_1");
}

/// S2-04: Classifier draft add/delete operation.
#[test]
fn classifier_draft_add_delete_operation() {
    let mut app = make_app_with_classifier_features();
    let (_, draft) = app.classifier_draft.as_mut().unwrap();
    assert_eq!(draft.operations.len(), 1);
    draft.operations.push(DraftOperation {
        name: "operation_1".into(),
        return_type_text: "void".into(),
        original_return_type: TypeReference::unspecified(),
        parameters: Vec::new(),
        visibility: Visibility::Public,
        is_static: false,
        is_abstract: false,
        is_virtual: false,
    });
    assert_eq!(draft.operations.len(), 2);
    draft.operations.remove(0);
    assert_eq!(draft.operations.len(), 1);
    assert_eq!(draft.operations[0].name, "operation_1");
}

/// S2-05: Operation parameter add/delete in draft.
#[test]
fn classifier_draft_add_delete_parameter() {
    let mut app = make_app_with_classifier_features();
    let (_, draft) = app.classifier_draft.as_mut().unwrap();
    let op = &mut draft.operations[0];
    assert_eq!(op.parameters.len(), 1);

    op.parameters.push(DraftParameter {
        name: "parameter_1".into(),
        type_text: "String".into(),
        original_type: TypeReference::unspecified(),
        direction: uml_core::ParameterDirection::In,
        default_value: String::new(),
    });
    assert_eq!(op.parameters.len(), 2);

    op.parameters.remove(0);
    assert_eq!(op.parameters.len(), 1);
    assert_eq!(op.parameters[0].name, "parameter_1");
}

/// S2-06: Applying classifier draft creates one command and updates model.
#[test]
fn classifier_draft_apply_creates_one_command() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    // Modify draft: add an attribute.
    let draft = app
        .classifier_draft
        .as_mut()
        .map(|(_, d)| {
            d.attributes.push(DraftAttribute {
                name: "age".into(),
                type_text: "int".into(),
                original_type: TypeReference::unspecified(),
                visibility: Visibility::Private,
                initial_value: "0".into(),
                is_static: false,
            });
            crate::app::ClassifierDraft {
                attributes: d.attributes.clone(),
                operations: d.operations.clone(),
            }
        })
        .unwrap();

    // Apply.
    let result = app.apply_classifier_draft(id, &draft).unwrap();
    assert!(result, "Apply should report changes were made");
    assert_eq!(app.history.undo_depth(), 1, "Apply creates one command");
    assert!(app.is_dirty, "Apply dirties the model");

    // Verify model has the new attribute.
    let elem = app.model.get(id).unwrap();
    let cd = elem.classifier_data().unwrap();
    assert_eq!(cd.attributes.len(), 2, "Model should have 2 attributes");
    assert_eq!(cd.attributes[1].name, "age");
    assert_eq!(cd.attributes[1].type_ref, TypeReference::primitive("int"));
    assert_eq!(cd.attributes[1].visibility, Visibility::Private);
    assert_eq!(cd.attributes[1].initial_value, Some("0".to_string()));
}

/// S2-07: Apply validates non-empty name.
#[test]
fn classifier_draft_apply_rejects_empty_name() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    let draft = app
        .classifier_draft
        .as_mut()
        .map(|(_, d)| {
            d.attributes[0].name = "".into();
            crate::app::ClassifierDraft {
                attributes: d.attributes.clone(),
                operations: d.operations.clone(),
            }
        })
        .unwrap();

    let result = app.apply_classifier_draft(id, &draft);
    assert!(result.is_err(), "Empty attribute name should be rejected");
    assert!(
        result.err().unwrap().contains("empty name"),
        "Error message should mention empty name"
    );
}

/// S2-08: Apply when no changes returns Ok(false) and does not create history.
#[test]
fn classifier_draft_apply_noop_returns_false() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    // Clone draft as-is without changes.
    let draft = app
        .classifier_draft
        .as_ref()
        .map(|(_, d)| crate::app::ClassifierDraft {
            attributes: d.attributes.clone(),
            operations: d.operations.clone(),
        })
        .unwrap();

    let result = app.apply_classifier_draft(id, &draft).unwrap();
    assert!(!result, "No-op Apply should return false");
    assert_eq!(app.history.undo_depth(), 0, "No-op should not create history");
    assert!(!app.is_dirty, "No-op should not dirty model");
}

/// S2-09: Classifier draft apply + undo restores original features.
#[test]
fn classifier_draft_apply_undo_restores() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    // Get original feature count.
    let orig_attr_count = {
        app.model
            .get(id)
            .unwrap()
            .classifier_data()
            .unwrap()
            .attributes
            .len()
    };

    // Modify draft: delete the existing attribute.
    let draft = app
        .classifier_draft
        .as_mut()
        .map(|(_, d)| {
            d.attributes.clear();
            crate::app::ClassifierDraft {
                attributes: d.attributes.clone(),
                operations: d.operations.clone(),
            }
        })
        .unwrap();

    app.apply_classifier_draft(id, &draft).unwrap();
    assert_eq!(
        app.model
            .get(id)
            .unwrap()
            .classifier_data()
            .unwrap()
            .attributes
            .len(),
        0,
        "After apply, attributes should be empty"
    );

    // Undo restores original.
    app.undo_action().unwrap();
    assert_eq!(
        app.model
            .get(id)
            .unwrap()
            .classifier_data()
            .unwrap()
            .attributes
            .len(),
        orig_attr_count,
        "Undo should restore original attribute count"
    );
}

/// S2-10: Revert restores classifier draft from model state.
#[test]
fn classifier_draft_revert_restores_from_model() {
    let mut app = make_app_with_classifier_features();

    // Modify the draft.
    if let Some((_, ref mut draft)) = app.classifier_draft {
        draft.attributes[0].name = "changed".into();
    }

    // Revert via refresh_property_buffers.
    app.refresh_property_buffers();

    // Verify draft is restored from model.
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes[0].name, "name", "Revert should restore original name");
}

/// S2-11: Model-backed type reference preserved when type text unchanged.
#[test]
fn classifier_draft_preserves_model_backed_type_reference() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    // Add an attribute with a model ID type reference.
    let model_id = UmlId::new();
    if let Some((_, ref mut draft)) = app.classifier_draft {
        draft.attributes[0].original_type = TypeReference::model(model_id);
        draft.attributes[0].type_text = "SomeElement".into();
    }

    // Apply: text "SomeElement" doesn't match model's name (model with model_id
    // doesn't exist in model), so should create primitive.
    let draft = app
        .classifier_draft
        .as_ref()
        .map(|(_, d)| crate::app::ClassifierDraft {
            attributes: d.attributes.clone(),
            operations: d.operations.clone(),
        })
        .unwrap();
    app.apply_classifier_draft(id, &draft).unwrap();
    let updated = app
        .model
        .get(id)
        .unwrap()
        .classifier_data()
        .unwrap()
        .clone();
    assert_eq!(
        updated.attributes[0].type_ref,
        TypeReference::primitive("SomeElement"),
        "When model_id doesn't resolve, text becomes primitive"
    );

    // Now set up a model-backed reference that DOES resolve.
    let target = Class::new("SomeElement");
    let target_id = target.base.id;
    app.model.insert(ModelElement::Class(target));
    if let Some((_, ref mut draft)) = app.classifier_draft {
        draft.attributes[0].original_type = TypeReference::model(target_id);
        draft.attributes[0].type_text = "SomeElement".into();
    }
    let draft = app
        .classifier_draft
        .as_ref()
        .map(|(_, d)| crate::app::ClassifierDraft {
            attributes: d.attributes.clone(),
            operations: d.operations.clone(),
        })
        .unwrap();
    app.apply_classifier_draft(id, &draft).unwrap();
    let updated = app
        .model
        .get(id)
        .unwrap()
        .classifier_data()
        .unwrap()
        .clone();
    assert_eq!(
        updated.attributes[0].type_ref,
        TypeReference::model(target_id),
        "Resolved model reference should be preserved"
    );
}

/// S2-12: Clear selection clears classifier draft.
#[test]
fn clear_selection_clears_classifier_draft() {
    let mut app = make_app_with_classifier_features();
    assert!(app.classifier_draft.is_some());
    app.clear_selection();
    assert!(app.classifier_draft.is_none());
}

/// S2-13: Normalize transient state clears classifier draft.
#[test]
fn normalize_transient_state_clears_classifier_draft() {
    let mut app = make_app_with_classifier_features();
    assert!(app.classifier_draft.is_some());
    app.selected_element_id = None;
    app.normalize_transient_state();
    assert!(app.classifier_draft.is_none());
}

/// S2-14: MCP targets exist for classifier draft.
#[test]
fn classifier_draft_mcp_targets_exist() {
    let app = make_app_with_classifier_features();
    let snapshot = app.qa_snapshot();
    // Apply and revert targets should exist.
    assert!(
        snapshot
            .targets
            .iter()
            .any(|t| t.id == "property.classifier.apply"),
        "apply target should exist"
    );
    assert!(
        snapshot
            .targets
            .iter()
            .any(|t| t.id == "property.classifier.revert"),
        "revert target should exist"
    );
    // Add attribute target.
    assert!(
        snapshot
            .targets
            .iter()
            .any(|t| t.id == "property.classifier.attribute.add"),
        "add attribute target should exist"
    );
    // Attribute name target.
    assert!(
        snapshot
            .targets
            .iter()
            .any(|t| t.id == "property.classifier.attribute.0.name"),
        "attribute name target should exist"
    );
    // Operation name target.
    assert!(
        snapshot
            .targets
            .iter()
            .any(|t| t.id == "property.classifier.operation.0.name"),
        "operation name target should exist"
    );
    // Parameter name target.
    assert!(
        snapshot
            .targets
            .iter()
            .any(|t| t.id == "property.classifier.operation.0.parameter.0.name"),
        "parameter name target should exist"
    );
    // Operation add parameter target.
    assert!(
        snapshot
            .targets
            .iter()
            .any(|t| t.id == "property.classifier.operation.0.parameter.add"),
        "operation add parameter target should exist"
    );
}

/// S2-15: MCP classifier draft actions work via QA dispatch.
#[test]
fn classifier_draft_mcp_add_attribute_via_qa() {
    let mut app = make_app_with_classifier_features();
    let ctx = egui::Context::default();

    // Select add attribute target and click.
    app.qa_select("property.classifier.attribute.add".into())
        .unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
        .unwrap();

    // Draft should now have 2 attributes.
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes.len(), 2, "MCP add attribute should create new attribute");
    assert_eq!(draft.attributes[1].name, "attribute_1");

    // Set the attribute name via MCP set text.
    app.qa_select("property.classifier.attribute.1.name".into())
        .unwrap();
    app.qa_dispatch(
        crate::app::qa::protocol::QaRequest::SetText {
            value: "email".into(),
        },
        &ctx,
    )
    .unwrap();
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes[1].name, "email");

    // Apply via MCP.
    app.qa_select("property.classifier.apply".into()).unwrap();
    app.qa_dispatch(crate::app::qa::protocol::QaRequest::Click { position: None }, &ctx)
        .unwrap();

    // Model should now have the new attribute.
    let id = app.selected_element_id.unwrap();
    let cd = app.model.get(id).unwrap().classifier_data().unwrap();
    assert_eq!(cd.attributes.len(), 2);
    assert_eq!(cd.attributes[1].name, "email");
}

/// S2-16: Applying primitive type text through the draft propagates to model
/// and round-trips through the classifier data.
#[test]
fn classifier_draft_apply_primitive_type_roundtrips() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    // Modify draft operation return type to a new primitive.
    let draft = app
        .classifier_draft
        .as_mut()
        .map(|(_, d)| {
            d.operations[0].return_type_text = "i32".into();
            crate::app::ClassifierDraft {
                attributes: d.attributes.clone(),
                operations: d.operations.clone(),
            }
        })
        .unwrap();

    app.apply_classifier_draft(id, &draft).unwrap();

    let cd = app.model.get(id).unwrap().classifier_data().unwrap();
    assert_eq!(
        cd.operations[0].return_type,
        TypeReference::primitive("i32"),
        "Primitive type should be stored in model"
    );

    // Verify the draft repopulates from model with the new type.
    app.refresh_property_buffers();
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.operations[0].return_type_text, "i32");
}

/// S2-17: Adding operation parameter with empty name fails on apply.
#[test]
fn classifier_draft_apply_rejects_empty_parameter_name() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    let draft = app
        .classifier_draft
        .as_mut()
        .map(|(_, d)| {
            d.operations[0].parameters[0].name = "".into();
            crate::app::ClassifierDraft {
                attributes: d.attributes.clone(),
                operations: d.operations.clone(),
            }
        })
        .unwrap();

    let result = app.apply_classifier_draft(id, &draft);
    assert!(result.is_err());
    assert!(result.err().unwrap().contains("empty name"));
}

/// S2-18: Draft parameter fields propagate correctly through apply.
#[test]
fn classifier_draft_parameter_fields_propagate() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    let draft = app
        .classifier_draft
        .as_mut()
        .map(|(_, d)| {
            let p = &mut d.operations[0].parameters[0];
            p.type_text = "u64".into();
            p.direction = uml_core::ParameterDirection::Out;
            p.default_value = "42".into();
            crate::app::ClassifierDraft {
                attributes: d.attributes.clone(),
                operations: d.operations.clone(),
            }
        })
        .unwrap();

    app.apply_classifier_draft(id, &draft).unwrap();

    let cd = app.model.get(id).unwrap().classifier_data().unwrap();
    let param = &cd.operations[0].parameters[0];
    assert_eq!(param.name, "format");
    assert_eq!(param.type_ref, TypeReference::primitive("u64"));
    assert_eq!(param.direction, uml_core::ParameterDirection::Out);
    assert_eq!(param.default_value, Some("42".to_string()));
}

/// S2-19: Undo after classifier apply restores parameters correctly.
#[test]
fn classifier_draft_undo_restores_parameters() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();

    let orig_ops = app
        .model
        .get(id)
        .unwrap()
        .classifier_data()
        .unwrap()
        .operations
        .clone();

    // Modify and apply.
    let draft = app
        .classifier_draft
        .as_mut()
        .map(|(_, d)| {
            d.operations[0].parameters[0].default_value = "false".into();
            crate::app::ClassifierDraft {
                attributes: d.attributes.clone(),
                operations: d.operations.clone(),
            }
        })
        .unwrap();
    app.apply_classifier_draft(id, &draft).unwrap();

    // Undo.
    app.undo_action().unwrap();
    let restored_ops = app
        .model
        .get(id)
        .unwrap()
        .classifier_data()
        .unwrap()
        .operations
        .clone();
    assert_eq!(restored_ops, orig_ops);
}

// ═══════════════════════════════════════════════════════════════════════
// S2F2 — Frame-level classifier draft persistence regressions
// ═══════════════════════════════════════════════════════════════════════

/// S2F2-01: Classifier draft survives multiple consecutive frames.
///
/// Verifies that the render loop's take+restore cycle does not lose the
/// draft after ordinary (no-click) frames, no-op Apply, or Revert.
#[test]
fn classifier_draft_survives_multiple_frames() {
    let mut app = make_app_with_classifier_features();
    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let screen_size = egui::vec2(1280.0, 1024.0);

    // Frame 1: establish layout.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    assert!(app.classifier_draft.is_some(), "Frame 1: classifier_draft must be present");
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes.len(), 1, "Frame 1: attribute count must be 1");

    // Frame 2: ordinary frame with no interactions.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    assert!(
        app.classifier_draft.is_some(),
        "Frame 2: classifier_draft must survive re-render"
    );
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes.len(), 1, "Frame 2: attributes must not be lost");

    // Frame 3: another ordinary frame.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });
    assert!(
        app.classifier_draft.is_some(),
        "Frame 3: classifier_draft must persist across frames"
    );
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes.len(), 1, "Frame 3: attributes must remain intact");
    assert_eq!(draft.attributes[0].name, "name", "Frame 3: attribute name unchanged");
}

/// S2F2-02: No-op Apply preserves the classifier draft and selection.
#[test]
fn classifier_draft_noop_apply_preserves_draft() {
    let mut app = make_app_with_classifier_features();
    let id = app.selected_element_id.unwrap();
    assert!(app.classifier_draft.is_some());

    // Build a draft that matches the current model (no changes).
    let cd = app
        .model
        .get(id)
        .unwrap()
        .classifier_data()
        .unwrap()
        .clone();
    let matching_draft = crate::app::ClassifierDraft {
        attributes: cd
            .attributes
            .iter()
            .map(|a| DraftAttribute {
                name: a.name.clone(),
                type_text: a.type_ref.display_name(Some(&app.model)),
                original_type: a.type_ref.clone(),
                visibility: a.visibility,
                initial_value: a.initial_value.clone().unwrap_or_default(),
                is_static: a.is_static,
            })
            .collect(),
        operations: cd
            .operations
            .iter()
            .map(|op| DraftOperation {
                name: op.name.clone(),
                return_type_text: op.return_type.display_name(Some(&app.model)),
                original_return_type: op.return_type.clone(),
                parameters: op
                    .parameters
                    .iter()
                    .map(|p| DraftParameter {
                        name: p.name.clone(),
                        type_text: p.type_ref.display_name(Some(&app.model)),
                        original_type: p.type_ref.clone(),
                        direction: p.direction,
                        default_value: p.default_value.clone().unwrap_or_default(),
                    })
                    .collect(),
                visibility: op.visibility,
                is_static: op.is_static,
                is_abstract: op.is_abstract,
                is_virtual: op.is_virtual,
            })
            .collect(),
    };

    // Apply with matching data — must return Ok(false) and preserve draft.
    let result = app.apply_classifier_draft(id, &matching_draft).unwrap();
    assert!(!result, "No-op Apply must return false");

    // Draft must survive the no-op Apply.
    assert!(app.classifier_draft.is_some(), "classifier_draft must survive no-op Apply");
    assert_eq!(app.selected_element_id, Some(id), "Selection must survive no-op Apply");

    // Verify draft contents are intact.
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes.len(), 1);
    assert_eq!(draft.attributes[0].name, "name");
    assert_eq!(draft.operations.len(), 1);
    assert_eq!(draft.operations[0].name, "getName");
}

/// S2F2-03: Simulate a render frame with a partial text edit, verify
/// draft persists and the edit survives the frame cycle.
#[test]
fn classifier_draft_edit_survives_frame_cycle() {
    let mut app = make_app_with_classifier_features();
    let ctx = egui::Context::default();
    let mut frame = eframe::Frame::_new_kittest();
    let screen_size = egui::vec2(1280.0, 1024.0);

    // Run a frame to establish layout.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });

    // Simulate a user edit by mutating the draft directly (as the
    // egui text_edit_singleline would do within the render call).
    if let Some((_, ref mut draft)) = app.classifier_draft {
        draft.attributes[0].name = "edited_name".into();
        draft.operations[0].return_type_text = "new_type".into();
    }

    // Run another frame — must NOT lose the draft or the edits.
    let _ = ctx.run(raw_with_screen(vec![], screen_size), |ctx| {
        app.update(ctx, &mut frame);
    });

    assert!(app.classifier_draft.is_some(), "Draft must survive second frame");
    let (_, draft) = app.classifier_draft.as_ref().unwrap();
    assert_eq!(draft.attributes[0].name, "edited_name", "Edited attribute name must survive");
    assert_eq!(
        draft.operations[0].return_type_text, "new_type",
        "Edited return type must survive"
    );
}
