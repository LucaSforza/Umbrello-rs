//! Application state for Umbrello-RS.
//!
//! Defines the `UmbrelloApp` struct — the top-level application state that
//! owns the UML model, undo history, diagram selection, drag state, tool
//! palette mode, and file I/O tracking. The `eframe::App` implementation
//! orchestrates rendering via sub-modules (tool_palette, canvas, menu, tree,
//! file_io, property_editor).

use std::collections::HashMap;
use std::path::PathBuf;
use uml_core::{Command, UmlId, UmlModel};

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
        }
    }

    /// Set the current file path (used after CLI loading).
    pub fn set_current_file_path(&mut self, path: Option<PathBuf>) {
        self.current_file_path = path;
    }

    /// Execute a command and mark the model as dirty on success.
    pub(crate) fn execute_command(&mut self, cmd: Box<dyn Command>) {
        if self.history.execute(cmd, &mut self.model).is_ok() {
            self.is_dirty = true;
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
                self.current_tool = crate::tool_palette::ToolMode::Select;
                self.preview_position = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::C)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateClass;
                self.preview_position = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::I)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateInterface;
                self.preview_position = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::E)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateEnum;
                self.preview_position = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::D)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateDatatype;
                self.preview_position = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::P)) {
                self.current_tool = crate::tool_palette::ToolMode::CreatePackage;
                self.preview_position = None;
            }
            // ── Edge tool keyboard shortcuts (M19) ──
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::G)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateGeneralization;
                self.preview_position = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::R)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateRealization;
                self.preview_position = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::A)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateAssociation;
                self.preview_position = None;
            }
            // 'N' (without Ctrl) is for Dependency; Ctrl+N is New File, handled above.
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::N)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateDependency;
                self.preview_position = None;
                self.drag_source_node_id = None;
            }
            // ── Actor (T) & UseCase (U) keyboard shortcuts (M20) ──
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::T)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateActor;
                self.preview_position = None;
                self.drag_source_node_id = None;
            }
            if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::U)) {
                self.current_tool = crate::tool_palette::ToolMode::CreateUseCase;
                self.preview_position = None;
                self.drag_source_node_id = None;
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
    }
}
