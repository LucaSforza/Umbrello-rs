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
use uml_core::{Command, UmlId, UmlModel};

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

    /// The currently selected element on the canvas, if any.
    /// Set by clicking a node; cleared by clicking background or pressing Escape.
    pub(crate) selected_element_id: Option<UmlId>,

    /// Cached property-panel edit buffer for the name field.
    /// Populated when a new element is selected; flushed to RenameElement on commit.
    pub(crate) name_edit_buffer: String,

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
            status_message: msg,
            loaded_from_xmi: loaded,
            current_file_path: None,
            is_dirty: false,
            current_tool: crate::tool_palette::ToolMode::Select,
            name_counters: HashMap::new(),
            preview_position: None,
            selected_element_id: None,
            name_edit_buffer: String::new(),
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
        }
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
                self.render_tool_palette(ui);
                ui.add_space(8.0);
                self.render_tree(ui);
            });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.render_canvas(ui);
        });
        egui::SidePanel::right("property_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                self.render_property_editor(ui);
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

        // ── Tool keyboard shortcuts ──────────────────────────────────
        // Only activate when no text input has focus.
        if !ctx.wants_keyboard_input() {
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::S)) {
                self.choose_tool(crate::tool_palette::ToolMode::Select);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::C)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateClass);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::I)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateInterface);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::E)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateEnum);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::D)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateDatatype);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::P)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreatePackage);
            }
            // ── Edge tool keyboard shortcuts (M19) ──
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::G)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateGeneralization);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateRealization);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::A)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateAssociation);
            }
            // 'N' (without Ctrl) is for Dependency; Ctrl+N is New File, handled above.
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::N)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateDependency);
                self.drag_source_node_id = None;
            }
            // ── Actor (T) & UseCase (U) keyboard shortcuts (M20) ──
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::T)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateActor);
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::U)) {
                self.choose_tool(crate::tool_palette::ToolMode::CreateUseCase);
            }
            // Note: Aggregation and Composition have no single-key shortcut
            // because 'C' is already used for Class and 'G' is for Generalization.
            // Use the tool palette buttons for these.
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                if self.selected_element_id.is_some() {
                    self.selected_element_id = None;
                    self.name_edit_buffer.clear();
                    self.status_message = "Selection cleared".into();
                } else if self.drag_source_node_id.is_some() {
                    self.drag_source_node_id = None;
                    self.status_message = "Edge creation cancelled".into();
                } else {
                    self.current_tool = crate::tool_palette::ToolMode::Select;
                    self.preview_position = None;
                }
            }

            // Update status message if tool changed via keyboard shortcut
            self.status_message = format!("Tool: {}", self.current_tool.label());
        }

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
