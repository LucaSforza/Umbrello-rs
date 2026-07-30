//! Application state for Umbrello-RS.
//!
//! Defines the `UmbrelloApp` struct — the top-level application state that
//! owns the UML model, undo history, diagram selection, drag state, tool
//! palette mode, and file I/O tracking. The `eframe::App` implementation
//! orchestrates rendering via sub-modules (tool_palette, canvas, menu, tree,
//! file_io, property_editor).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc::SyncSender,
    Arc,
};
use std::time::{Duration, Instant};

#[path = "qa/mod.rs"]
pub(crate) mod qa;
#[path = "viewport.rs"]
pub(crate) mod viewport;
use uml_core::{Command, ParameterDirection, TypeReference, UmlId, UmlModel, Visibility};

/// Draft of an attribute being edited in the classifier feature editor.
#[derive(Debug, Clone)]
pub(crate) struct DraftAttribute {
    pub(crate) name: String,
    pub(crate) type_text: String,
    pub(crate) original_type: TypeReference,
    pub(crate) visibility: Visibility,
    pub(crate) initial_value: String,
    pub(crate) is_static: bool,
}

/// Draft of an operation parameter being edited.
#[derive(Debug, Clone)]
pub(crate) struct DraftParameter {
    pub(crate) name: String,
    pub(crate) type_text: String,
    pub(crate) original_type: TypeReference,
    pub(crate) direction: ParameterDirection,
    pub(crate) default_value: String,
}

/// Draft of an operation being edited in the classifier feature editor.
#[derive(Debug, Clone)]
pub(crate) struct DraftOperation {
    pub(crate) name: String,
    pub(crate) return_type_text: String,
    pub(crate) original_return_type: TypeReference,
    pub(crate) parameters: Vec<DraftParameter>,
    pub(crate) visibility: Visibility,
    pub(crate) is_static: bool,
    pub(crate) is_abstract: bool,
    pub(crate) is_virtual: bool,
}

/// Persistent classifier feature values edited before Apply.
#[derive(Debug, Clone)]
pub(crate) struct ClassifierDraft {
    pub(crate) attributes: Vec<DraftAttribute>,
    pub(crate) operations: Vec<DraftOperation>,
}

/// Persistent relationship values edited by the inspector before Apply.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RelationshipDraft {
    pub(crate) kind: uml_core::AssociationType,
    pub(crate) name: String,
    pub(crate) documentation: String,
    pub(crate) source_role: String,
    pub(crate) source_multiplicity: String,
    pub(crate) target_role: String,
    pub(crate) target_multiplicity: String,
    pub(crate) source_navigable: bool,
    pub(crate) target_navigable: bool,
}

type QaReply = SyncSender<Result<self::qa::protocol::QaResponse, self::qa::protocol::QaError>>;
struct PendingQaReply {
    reply: QaReply,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
}

struct PendingScreenshot {
    reply: QaReply,
    deadline: Instant,
    cancelled: Arc<AtomicBool>,
    requested_revision: u64,
    issued_revision: Option<u64>,
}

/// The Umbrello application state.
pub(crate) struct UmbrelloApp {
    pub(crate) model: UmlModel,
    pub(crate) history: uml_core::History,
    pub(crate) active_diagram: Option<usize>,
    pub(crate) drag_node_id: Option<uml_core::UmlId>,
    pub(crate) drag_start_pos: Option<egui::Pos2>,
    pub(crate) drag_preview_pos: Option<uml_core::Point>,
    /// Accumulated screen-space displacement since drag begin, used to
    /// compute cumulative model-position updates across multiple movement
    /// frames (each ctx.run() resets pointer.delta but not this field).
    pub(crate) drag_accum_screen_delta: egui::Vec2,
    pub(crate) status_message: String,
    /// REVIEW CONDITION C1: Track whether model was loaded from XMI.
    #[allow(dead_code)]
    pub(crate) loaded_from_xmi: bool,
    /// Path to the currently open file, if any. `None` for new/untitled models.
    pub(crate) current_file_path: Option<PathBuf>,
    /// Whether the model has unsaved changes since the last save/load.
    pub(crate) is_dirty: bool,
    /// The currently active tool in the tool palette.
    pub(crate) current_tool: crate::tool_palette::ToolMode,
    /// Counter for auto-generated element names, keyed by element type name.
    /// Tracks the next suffix number for each type (e.g., "Class" → 3 means next is "Class_3").
    #[allow(dead_code)]
    pub(crate) name_counters: HashMap<String, u64>,
    /// Ghost-rectangle position for creation preview (in canvas coordinates).
    pub(crate) preview_position: Option<uml_core::Point>,
    pub(crate) viewport_pans: HashMap<uml_core::DiagramId, egui::Vec2>,
    pub(crate) last_canvas_rect: Option<egui::Rect>,

    /// The currently selected element on the canvas, if any.
    /// Set by clicking a node; cleared by clicking background or pressing Escape.
    pub(crate) selected_element_id: Option<UmlId>,

    /// Cached property-panel edit buffer for the name field.
    /// Populated when a new element is selected; flushed to RenameElement on commit.
    pub(crate) name_edit_buffer: String,

    /// Persistent documentation editor buffer for ordinary elements.
    pub(crate) documentation_edit_buffer: String,

    /// Relationship draft and the semantic element it belongs to.
    pub(crate) relationship_draft: Option<(UmlId, RelationshipDraft)>,

    /// Classifier feature draft and the semantic element it belongs to.
    pub(crate) classifier_draft: Option<(UmlId, ClassifierDraft)>,

    /// When an edge tool is active, this tracks the source node of a click-drag.
    /// Set to `Some(id)` on mousedown over a node; cleared on mouseup or Escape.
    pub(crate) drag_source_node_id: Option<UmlId>,

    /// Tracks whether the primary mouse button was down in the previous frame,
    /// used to detect edge-drag start transitions.
    #[allow(dead_code)]
    pub(crate) pointer_was_down: bool,
    pub(crate) selected_qa_target: Option<String>,
    pub(crate) ui_frame: u64,
    pub(crate) state_revision: u64,
    pub(crate) rendered_revision: u64,
    pub(crate) qa_bridge: Option<self::qa::bridge::QaBridge>,
    pending_screenshots: HashMap<u64, PendingScreenshot>,
    pending_syncs: Vec<(u64, PendingQaReply)>,
    pub(crate) next_screenshot_id: u64,
    pub(crate) new_diagram_open: bool,
    pub(crate) new_diagram_name: String,
    pub(crate) new_diagram_kind: uml_core::DiagramKind,
}

impl UmbrelloApp {
    /// Create a new application state wrapping the given model.
    pub fn new(model: UmlModel, loaded: bool) -> Self {
        let msg = if loaded {
            format!("Loaded model with {} elements", model.len())
        } else {
            "Empty model — no XMI file loaded".to_string()
        };
        Self {
            model,
            history: uml_core::History::new(100),
            active_diagram: None,
            drag_node_id: None,
            drag_start_pos: None,
            drag_preview_pos: None,
            drag_accum_screen_delta: egui::Vec2::ZERO,
            status_message: msg,
            loaded_from_xmi: loaded,
            current_file_path: None,
            is_dirty: false,
            current_tool: crate::tool_palette::ToolMode::Select,
            name_counters: HashMap::new(),
            preview_position: None,
            viewport_pans: HashMap::new(),
            last_canvas_rect: None,
            selected_element_id: None,
            name_edit_buffer: String::new(),
            documentation_edit_buffer: String::new(),
            relationship_draft: None,
            classifier_draft: None,
            drag_source_node_id: None,
            pointer_was_down: false,
            selected_qa_target: None,
            ui_frame: 0,
            state_revision: 0,
            rendered_revision: 0,
            qa_bridge: None,
            pending_screenshots: HashMap::new(),
            pending_syncs: Vec::new(),
            next_screenshot_id: 1,
            new_diagram_open: false,
            new_diagram_name: String::new(),
            new_diagram_kind: uml_core::DiagramKind::Class,
        }
    }

    pub(crate) fn viewport_transform(
        &self,
        origin: egui::Pos2,
    ) -> Option<crate::app::viewport::ViewportTransform> {
        let index = self.active_diagram?;
        let diagram = self.model.diagrams().get(index)?;
        Some(crate::app::viewport::ViewportTransform::new(
            origin,
            self.viewport_pans
                .get(&diagram.id)
                .copied()
                .unwrap_or_default(),
            diagram.zoom_percent(),
        ))
    }

    pub(crate) fn clear_viewport_pans(&mut self) {
        self.viewport_pans.clear();
    }

    pub(crate) fn active_diagram_kind(&self) -> Option<uml_core::DiagramKind> {
        self.active_diagram
            .and_then(|index| self.model.diagrams().get(index).map(|diagram| diagram.kind))
    }

    pub(crate) fn is_tool_available(&self, tool: crate::tool_palette::ToolMode) -> bool {
        if tool != crate::tool_palette::ToolMode::Select && self.current_file_path.is_none() {
            return false;
        }
        self.active_diagram_kind()
            .is_some_and(|kind| tool.is_compatible_with_diagram(kind))
    }

    pub(crate) fn tool_unavailable_reason(
        &self,
        tool: crate::tool_palette::ToolMode,
    ) -> &'static str {
        if tool != crate::tool_palette::ToolMode::Select && self.current_file_path.is_none() {
            return "Create or open an XMI project before authoring elements";
        }
        if let Some(kind) = self.active_diagram_kind() {
            if !tool.is_compatible_with_diagram(kind) {
                return "This tool is not supported by the active diagram kind";
            }
        } else {
            return "Select or create a supported diagram first";
        }
        "Tool unavailable"
    }

    pub(crate) fn normalize_transient_state(&mut self) {
        if self
            .active_diagram
            .is_some_and(|index| index >= self.model.diagrams().len())
        {
            self.active_diagram = None;
        }
        if self
            .selected_element_id
            .is_some_and(|id| self.model.get(id).is_none())
        {
            self.selected_element_id = None;
            self.name_edit_buffer.clear();
            self.documentation_edit_buffer.clear();
            self.relationship_draft = None;
            self.classifier_draft = None;
        }
        if self.selected_element_id.is_none() {
            self.documentation_edit_buffer.clear();
            self.relationship_draft = None;
            self.classifier_draft = None;
        }
        self.current_tool = crate::tool_palette::ToolMode::Select;
        self.preview_position = None;
        self.drag_source_node_id = None;
        self.drag_node_id = None;
        self.drag_start_pos = None;
        self.drag_preview_pos = None;
        self.drag_accum_screen_delta = egui::Vec2::ZERO;
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selected_element_id = None;
        self.name_edit_buffer.clear();
        self.documentation_edit_buffer.clear();
        self.relationship_draft = None;
        self.classifier_draft = None;
    }

    /// Shared drag state machine — begin phase.
    ///
    /// Sets drag_node_id and drag_start_pos to the original model position.
    /// Caller must supply the original (pre-drag) model position so that
    /// cumulative screen displacement can be correctly converted back to
    /// model coordinates.
    pub(crate) fn begin_node_drag(&mut self, node_id: UmlId, original_pos: uml_core::Point) {
        self.drag_node_id = Some(node_id);
        self.drag_start_pos = Some(egui::pos2(original_pos.x as f32, original_pos.y as f32));
        self.drag_preview_pos = None;
        self.drag_accum_screen_delta = egui::Vec2::ZERO;
    }

    /// Shared drag state machine — update phase.
    ///
    /// Sets the transient preview position in model coordinates.
    /// Safe to call every frame during a drag; the last value before
    /// commit_node_drag determines the final position.
    pub(crate) fn update_node_drag(&mut self, model_position: uml_core::Point) {
        self.drag_preview_pos = Some(model_position);
    }

    /// Shared drag state machine — commit phase.
    ///
    /// Clears all drag state first, then conditionally executes a MoveNode.
    /// If no movement occurred (no preview was set, or preview equals the
    /// original position), this is a successful no-op — no history entry
    /// is created and the model stays clean.
    pub(crate) fn commit_node_drag(
        &mut self,
        diagram_id: uml_core::DiagramId,
    ) -> Result<(), self::qa::protocol::QaError> {
        let node_id = self
            .drag_node_id
            .take()
            .ok_or(self::qa::protocol::QaError::UnavailableTarget("no active drag".into()))?;
        // Original model position stored by begin_node_drag.
        let original = self
            .drag_start_pos
            .map(|p| uml_core::Point::new(p.x as f64, p.y as f64));
        let position = self.drag_preview_pos.take();
        // Clear all drag state.
        self.drag_start_pos = None;
        self.drag_accum_screen_delta = egui::Vec2::ZERO;

        // No preview → no movement → no-op.
        let Some(position) = position else {
            return Ok(());
        };

        // Preview matches original → no meaningful movement → no-op.
        if let Some(orig) = original {
            if (position.x - orig.x).abs() < 0.001 && (position.y - orig.y).abs() < 0.001 {
                return Ok(());
            }
        }

        let cmd = uml_core::commands::MoveNode::new(&self.model, diagram_id, node_id, position)
            .map_err(|e| self::qa::protocol::QaError::Command(e.to_string()))?;
        self.execute_command_result(Box::new(cmd))
    }

    /// Execute a native-equivalent node gesture: begin, update, commit
    /// and clear using the shared helpers.
    ///
    /// Used by the MCP gesture mode so that semantic `ui_drag` exercises
    /// the same control flow as native pointer interaction.
    pub(crate) fn execute_gesture_move(
        &mut self,
        diagram_id: uml_core::DiagramId,
        node_id: UmlId,
        destination: uml_core::Point,
    ) -> Result<(), self::qa::protocol::QaError> {
        // Look up the original node position.
        let original = self
            .model
            .get_diagram(diagram_id)
            .and_then(|d| d.get_node(node_id))
            .map(|n| uml_core::Point::new(n.bounds.x(), n.bounds.y()))
            .ok_or(self::qa::protocol::QaError::UnavailableTarget(format!("node:{node_id}")))?;
        self.begin_node_drag(node_id, original);
        self.update_node_drag(destination);
        self.commit_node_drag(diagram_id)
    }

    pub(crate) fn refresh_property_buffers(&mut self) {
        let Some(id) = self.selected_element_id else {
            self.name_edit_buffer.clear();
            self.documentation_edit_buffer.clear();
            self.relationship_draft = None;
            self.classifier_draft = None;
            return;
        };
        let Some(element) = self.model.get(id) else {
            self.clear_selection();
            return;
        };
        self.name_edit_buffer = element.name().to_string();
        self.documentation_edit_buffer = element.base().documentation.clone();

        // Populate classifier draft for classifiers, clear for others.
        self.classifier_draft = element.classifier_data().map(|cd| {
            let draft = ClassifierDraft {
                attributes: cd
                    .attributes
                    .iter()
                    .map(|a| {
                        let type_text = a.type_ref.display_name(Some(&self.model));
                        DraftAttribute {
                            name: a.name.clone(),
                            type_text: type_text.clone(),
                            original_type: a.type_ref.clone(),
                            visibility: a.visibility,
                            initial_value: a.initial_value.clone().unwrap_or_default(),
                            is_static: a.is_static,
                        }
                    })
                    .collect(),
                operations: cd
                    .operations
                    .iter()
                    .map(|op| {
                        let return_type_text = op.return_type.display_name(Some(&self.model));
                        DraftOperation {
                            name: op.name.clone(),
                            return_type_text: return_type_text.clone(),
                            original_return_type: op.return_type.clone(),
                            parameters: op
                                .parameters
                                .iter()
                                .map(|p| {
                                    let type_text = p.type_ref.display_name(Some(&self.model));
                                    DraftParameter {
                                        name: p.name.clone(),
                                        type_text: type_text.clone(),
                                        original_type: p.type_ref.clone(),
                                        direction: p.direction,
                                        default_value: p.default_value.clone().unwrap_or_default(),
                                    }
                                })
                                .collect(),
                            visibility: op.visibility,
                            is_static: op.is_static,
                            is_abstract: op.is_abstract,
                            is_virtual: op.is_virtual,
                        }
                    })
                    .collect(),
            };
            (id, draft)
        });

        self.relationship_draft = match element {
            uml_core::ModelElement::Relationship(relationship) => Some((
                id,
                RelationshipDraft {
                    kind: relationship.kind,
                    name: relationship.base.name.clone(),
                    documentation: relationship.base.documentation.clone(),
                    source_role: relationship.source_role_name.clone().unwrap_or_default(),
                    source_multiplicity: relationship
                        .source_multiplicity
                        .clone()
                        .unwrap_or_default(),
                    target_role: relationship.target_role_name.clone().unwrap_or_default(),
                    target_multiplicity: relationship
                        .target_multiplicity
                        .clone()
                        .unwrap_or_default(),
                    source_navigable: relationship.source_to_target_navigable,
                    target_navigable: relationship.target_to_source_navigable,
                },
            )),
            _ => None,
        };
    }

    /// Set the current file path (used after CLI loading).
    pub fn set_current_file_path(&mut self, path: Option<PathBuf>) {
        self.current_file_path = path;
    }

    /// Execute a command and mark the model as dirty on success.
    #[allow(dead_code)] // Compatibility helper retained for existing UI tests.
    pub(crate) fn execute_command(&mut self, cmd: Box<dyn Command>) {
        let _ = self.execute_command_result(cmd);
    }

    pub(crate) fn execute_command_result(
        &mut self,
        cmd: Box<dyn Command>,
    ) -> Result<(), self::qa::protocol::QaError> {
        self.history
            .execute(cmd, &mut self.model)
            .map_err(|error| self::qa::protocol::QaError::Command(error.to_string()))?;
        self.is_dirty = true;
        self.bump_state();
        Ok(())
    }

    pub(crate) fn bump_state(&mut self) {
        self.state_revision = self.state_revision.saturating_add(1);
    }

    pub(crate) fn open_new_diagram_dialog(&mut self) {
        if self.current_file_path.is_none() {
            self.status_message = "Create or open an XMI project before adding diagrams".into();
            return;
        }
        self.new_diagram_kind = uml_core::DiagramKind::Class;
        self.new_diagram_name = self.unique_diagram_name(self.new_diagram_kind);
        self.new_diagram_open = true;
    }

    #[allow(dead_code)] // Direct state helper used by tests and future non-egui callers.
    pub(crate) fn set_new_diagram_kind(&mut self, kind: uml_core::DiagramKind) {
        let previous_kind = self.new_diagram_kind;
        self.apply_new_diagram_kind_transition(previous_kind, kind);
    }

    pub(crate) fn apply_new_diagram_kind_transition(
        &mut self,
        previous_kind: uml_core::DiagramKind,
        new_kind: uml_core::DiagramKind,
    ) {
        if previous_kind != new_kind {
            self.new_diagram_kind = new_kind;
            self.new_diagram_name = self.unique_diagram_name(new_kind);
        }
    }

    pub(crate) fn unique_diagram_name(&self, kind: uml_core::DiagramKind) -> String {
        self.generate_unique_diagram_name(kind.as_str())
    }

    pub(crate) fn generate_unique_diagram_name(&self, base: &str) -> String {
        let existing: std::collections::HashSet<&str> = self
            .model
            .diagrams()
            .iter()
            .map(|diagram| diagram.name.as_str())
            .collect();
        let prefix = format!("{base}_");
        let mut suffixes: Vec<u64> = existing
            .iter()
            .filter_map(|name| name.strip_prefix(&prefix)?.parse().ok())
            .collect();
        suffixes.sort_unstable();
        let next = (1_u64..)
            .find(|candidate| suffixes.binary_search(candidate).is_err())
            .unwrap_or(1);
        format!("{base}_{next}")
    }

    pub(crate) fn create_diagram(
        &mut self,
        kind: uml_core::DiagramKind,
        name: String,
    ) -> Result<uml_core::DiagramId, String> {
        if self.current_file_path.is_none() {
            return Err("create or open an XMI project before adding diagrams".into());
        }
        if !matches!(
            kind,
            uml_core::DiagramKind::Class
                | uml_core::DiagramKind::UseCase
                | uml_core::DiagramKind::Component
                | uml_core::DiagramKind::Deployment
        ) {
            return Err(format!("unsupported diagram kind: {}", kind.as_str()));
        }
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err("diagram name cannot be empty".into());
        }
        let diagram = uml_core::Diagram::new(name, kind);
        let diagram_id = diagram.id;
        let command = uml_core::commands::CreateDiagram::new(&self.model, diagram)
            .map_err(|error| error.to_string())?;
        self.execute_command_result(Box::new(command))
            .map_err(|error| error.to_string())?;
        self.active_diagram = self
            .model
            .diagrams()
            .iter()
            .position(|candidate| candidate.id == diagram_id);
        self.normalize_transient_state();
        self.active_diagram = self
            .model
            .diagrams()
            .iter()
            .position(|candidate| candidate.id == diagram_id);
        self.bump_state();
        Ok(diagram_id)
    }

    pub(crate) fn render_new_diagram_dialog(&mut self, ctx: &egui::Context) {
        if !self.new_diagram_open {
            return;
        }
        let mut open = self.new_diagram_open;
        egui::Window::new("New Diagram")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Diagram type");
                let previous_kind = self.new_diagram_kind;
                for kind in [
                    uml_core::DiagramKind::Class,
                    uml_core::DiagramKind::UseCase,
                    uml_core::DiagramKind::Component,
                    uml_core::DiagramKind::Deployment,
                ] {
                    ui.radio_value(&mut self.new_diagram_kind, kind, kind.as_str());
                }
                if self.new_diagram_kind != previous_kind {
                    self.apply_new_diagram_kind_transition(previous_kind, self.new_diagram_kind);
                }
                ui.separator();
                ui.label("Name");
                ui.text_edit_singleline(&mut self.new_diagram_name);
                ui.horizontal(|ui| {
                    if ui.button("Create").clicked() {
                        if self
                            .create_diagram(self.new_diagram_kind, self.new_diagram_name.clone())
                            .is_ok()
                        {
                            self.new_diagram_open = false;
                            self.new_diagram_name.clear();
                        } else {
                            self.status_message = "Diagram name cannot be empty".into();
                        }
                    }
                    if ui.button("Cancel").clicked() {
                        self.new_diagram_open = false;
                    }
                });
            });
        self.new_diagram_open = open && self.new_diagram_open;
    }

    #[allow(dead_code)]
    pub(crate) fn enable_qa(&mut self, capacity: usize) -> self::qa::QaHandle {
        let (bridge, handle) = self::qa::QaBridge::new(capacity);
        self.qa_bridge = Some(bridge);
        handle
    }

    fn process_qa(&mut self, ctx: &egui::Context) {
        let Some(bridge) = self.qa_bridge.take() else {
            return;
        };
        let now = Instant::now();
        self.pending_screenshots.retain(|_, pending| {
            if pending.cancelled.load(Ordering::Acquire) || pending.deadline <= now {
                let error = if pending.cancelled.load(Ordering::Acquire) {
                    self::qa::protocol::QaError::Cancelled
                } else {
                    self::qa::protocol::QaError::Timeout
                };
                let _ = pending.reply.send(Err(error));
                false
            } else {
                true
            }
        });
        self.pending_syncs.retain(|(_, pending)| {
            if pending.cancelled.load(Ordering::Acquire) || pending.deadline <= now {
                let error = if pending.cancelled.load(Ordering::Acquire) {
                    self::qa::protocol::QaError::Cancelled
                } else {
                    self::qa::protocol::QaError::Timeout
                };
                let _ = pending.reply.send(Err(error));
                false
            } else {
                true
            }
        });
        for _ in 0..32 {
            let Ok(envelope) = bridge.receiver.try_recv() else {
                break;
            };
            if envelope.cancelled.load(Ordering::Acquire) || envelope.deadline <= Instant::now() {
                let _ = envelope
                    .reply
                    .send(Err(if envelope.cancelled.load(Ordering::Acquire) {
                        self::qa::protocol::QaError::Cancelled
                    } else {
                        self::qa::protocol::QaError::Timeout
                    }));
                continue;
            }
            if matches!(envelope.request, self::qa::protocol::QaRequest::Screenshot) {
                let id = self.next_screenshot_id;
                self.next_screenshot_id = self.next_screenshot_id.saturating_add(1);
                self.pending_screenshots.insert(
                    id,
                    PendingScreenshot {
                        reply: envelope.reply,
                        deadline: envelope.deadline,
                        cancelled: envelope.cancelled,
                        requested_revision: self.state_revision,
                        issued_revision: None,
                    },
                );
            } else if let self::qa::protocol::QaRequest::Sync { after_revision } = envelope.request
            {
                if self.rendered_revision >= after_revision {
                    let _ = envelope
                        .reply
                        .send(Ok(self::qa::protocol::QaResponse::Snapshot(self.qa_snapshot())));
                } else {
                    self.pending_syncs.push((
                        after_revision,
                        PendingQaReply {
                            reply: envelope.reply,
                            deadline: envelope.deadline,
                            cancelled: envelope.cancelled,
                        },
                    ));
                    ctx.request_repaint();
                }
            } else {
                let result = self.qa_dispatch(envelope.request, ctx);
                let _ = envelope.reply.send(result);
            }
        }
        self.qa_bridge = Some(bridge);
        let mut to_issue = Vec::new();
        for (&id, pending) in &mut self.pending_screenshots {
            if pending.issued_revision.is_none()
                && self.rendered_revision >= pending.requested_revision
                && !pending.cancelled.load(Ordering::Acquire)
            {
                pending.issued_revision = Some(self.rendered_revision);
                to_issue.push(id);
            }
        }
        for id in to_issue {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(id)));
        }
        let waiting = std::mem::take(&mut self.pending_syncs);
        let mut retained = Vec::new();
        for (revision, pending) in waiting {
            if self.rendered_revision >= revision {
                let _ = pending
                    .reply
                    .send(Ok(self::qa::protocol::QaResponse::Snapshot(self.qa_snapshot())));
            } else {
                retained.push((revision, pending));
            }
        }
        self.pending_syncs = retained;
        if !self.pending_screenshots.is_empty() || !self.pending_syncs.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(10));
        }
    }

    fn process_screenshot_events(&mut self, ctx: &egui::Context) {
        let events = ctx.input(|input| input.raw.events.clone());
        for event in events {
            let egui::Event::Screenshot {
                user_data, image, ..
            } = event
            else {
                continue;
            };
            let Some(data) = user_data.data else {
                continue;
            };
            let Ok(id) = data.downcast::<u64>() else {
                continue;
            };
            let id = *id;
            let Some(pending) = self.pending_screenshots.remove(&id) else {
                continue;
            };
            let captured_revision = pending.issued_revision.unwrap_or(self.rendered_revision);
            let result =
                self::qa::screenshot::encode_png(&image, captured_revision, captured_revision)
                    .map(self::qa::protocol::QaResponse::Screenshot)
                    .map_err(self::qa::protocol::QaError::Screenshot);
            let result = if pending.cancelled.load(Ordering::Acquire) {
                Err(self::qa::protocol::QaError::Cancelled)
            } else if pending.deadline <= Instant::now() {
                Err(self::qa::protocol::QaError::Timeout)
            } else {
                result
            };
            let _ = pending.reply.send(result);
        }
    }

    pub(crate) fn shutdown_qa(&mut self) {
        if let Some(bridge) = self.qa_bridge.take() {
            while let Ok(envelope) = bridge.receiver.try_recv() {
                let _ = envelope
                    .reply
                    .send(Err(self::qa::protocol::QaError::Shutdown));
            }
        }
        for pending in self.pending_screenshots.drain().map(|(_, p)| p) {
            let _ = pending
                .reply
                .send(Err(self::qa::protocol::QaError::Shutdown));
        }
        for (_, pending) in self.pending_syncs.drain(..) {
            let _ = pending
                .reply
                .send(Err(self::qa::protocol::QaError::Shutdown));
        }
    }

    /// Generate a unique default name for a new element of the given type.
    /// Scans existing elements to find the next available suffix.
    /// E.g., if "Class_1" and "Class_2" exist, returns "Class_3".
    pub(crate) fn generate_unique_name(&self, base: &str) -> String {
        // Collect all existing element names from the model.
        let existing: std::collections::HashSet<&str> =
            self.model.iter().map(|(_, e)| e.name()).collect();

        // Find all names matching "{base}_{N}" and collect the suffix numbers.
        let prefix = format!("{base}_");
        let mut suffixes: Vec<u64> = existing
            .iter()
            .filter_map(|name| {
                if let Some(rest) = name.strip_prefix(&prefix) {
                    rest.parse::<u64>().ok()
                } else {
                    None
                }
            })
            .collect();

        suffixes.sort_unstable();

        // Find the first gap starting from 1.
        let next = (1u64..)
            .find(|n| suffixes.binary_search(n).is_err())
            .unwrap_or(1);

        format!("{base}_{next}")
    }

    /// Update the window title to reflect current file path and dirty state.
    fn update_title(&self, ctx: &egui::Context) {
        let base = match &self.current_file_path {
            Some(path) => format!("Umbrello-RS — {}", path.display()),
            None => "Umbrello-RS — Untitled".into(),
        };
        let title = if self.is_dirty {
            format!("{base} *")
        } else {
            base
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
    }
}

impl eframe::App for UmbrelloApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ui_frame = self.ui_frame.saturating_add(1);
        self.process_screenshot_events(ctx);
        self.render_menu(ctx);
        egui::SidePanel::left("tree_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                self.render_new_diagram_control(ui);
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_tool_palette(ui);
                        ui.add_space(8.0);
                        self.render_tree(ui);
                    });
            });
        egui::SidePanel::right("property_panel")
            .resizable(true)
            .default_width(280.0)
            .min_width(200.0)
            .max_width(400.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        self.render_property_editor(ui);
                    });
            });
        // CentralPanel must be last so its max_rect is the actual
        // canvas area (excluding both side panels).  If CentralPanel
        // renders before the right SidePanel, its max_rect includes
        // the future property panel space, causing canvas_rect to
        // span the full window width and breaking the canvas-only
        // background deselection guard.
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
        if self.drag_node_id.is_some() {
            ctx.request_repaint();
        }

        // ── Keyboard shortcuts (consume to avoid repeat triggers) ─────
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::N)) {
            self.menu_file_new();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::O)) {
            self.menu_file_open();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::S)) {
            if ctx.input(|i| i.modifiers.shift) {
                self.menu_file_save_as();
            } else {
                self.menu_file_save();
            }
        }
        if ctx.input_mut(|i| {
            i.consume_key(egui::Modifiers::CTRL | egui::Modifiers::SHIFT, egui::Key::S)
        }) {
            self.menu_file_save_as();
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Q))
            && self.prompt_save_if_dirty()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        if !ctx.wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Z))
            && self.undo_action().is_ok()
        {
            self.status_message = "Undo".into();
        }
        if !ctx.wants_keyboard_input()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Y))
            && self.redo_action().is_ok()
        {
            self.status_message = "Redo".into();
        }

        // ── Tool keyboard shortcuts ──────────────────────────────────
        // Only activate when no text input has focus.
        if !ctx.wants_keyboard_input() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::S)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::Select);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::C)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateClass);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::I)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateInterface);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::E)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateEnum);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::D)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateDatatype);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::P)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreatePackage);
            }
            // ── Edge tool keyboard shortcuts (M19) ──
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::G)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateGeneralization);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateRealization);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::A)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateAssociation);
            }
            // 'N' (without Ctrl) is for Dependency; Ctrl+N is New File, handled above.
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::N)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateDependency);
                self.drag_source_node_id = None;
            }
            // ── Actor (T) & UseCase (U) keyboard shortcuts (M20) ──
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::T)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateActor);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::U)) {
                let _ = self.choose_tool(crate::tool_palette::ToolMode::CreateUseCase);
            }
            // Note: Aggregation and Composition have no single-key shortcut
            // because 'C' is already used for Class and 'G' is for Generalization.
            // Use the tool palette buttons for these.
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                if self.drag_node_id.is_some() {
                    self.drag_node_id = None;
                    self.drag_start_pos = None;
                    self.drag_preview_pos = None;
                    self.drag_accum_screen_delta = egui::Vec2::ZERO;
                    self.status_message = "Node drag cancelled".into();
                } else if self.selected_element_id.is_some() {
                    self.clear_selection();
                    self.status_message = "Selection cleared".into();
                } else if self.drag_source_node_id.is_some() {
                    self.drag_source_node_id = None;
                    self.status_message = "Edge creation cancelled".into();
                } else {
                    self.current_tool = crate::tool_palette::ToolMode::Select;
                    self.preview_position = None;
                }
            }
        }

        self.render_new_diagram_dialog(ctx);

        // Update window title
        self.update_title(ctx);
        self.rendered_revision = self.state_revision;
        self.process_qa(ctx);
    }
}

impl Drop for UmbrelloApp {
    fn drop(&mut self) {
        self.shutdown_qa();
    }
}
