//! Menu bar rendering and file operations.
//!
//! Implements the File and Edit menus, with New/Open/Save/Save As/Quit actions
//! and Undo/Redo support.

use crate::app::UmbrelloApp;

impl UmbrelloApp {
    // ═══════════════════════════════════════════════════════════════════
    // Menu bar
    // ═══════════════════════════════════════════════════════════════════

    /// Render the main menu bar.
    pub(crate) fn render_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project…\tCtrl+N").clicked() {
                        self.menu_file_new();
                        ui.close_menu();
                    }
                    if ui.button("Open XMI...\tCtrl+O").clicked() {
                        self.menu_file_open();
                        ui.close_menu();
                    }
                    if ui.button("Save\tCtrl+S").clicked() {
                        self.menu_file_save();
                        ui.close_menu();
                    }
                    if ui.button("Save As...\tCtrl+Shift+S").clicked() {
                        self.menu_file_save_as();
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add_enabled(false, egui::Button::new("Open Recent"))
                        .clicked()
                    {
                        // Stubbed
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit\tCtrl+Q").clicked() {
                        if self.prompt_save_if_dirty() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        if self.history.can_undo() {
                            let _ = self.undo_action();
                            self.is_dirty = true;
                            self.status_message = "Undo".into();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked() {
                        if self.history.can_redo() {
                            let _ = self.redo_action();
                            self.is_dirty = true;
                            self.status_message = "Redo".into();
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("View", |ui| {
                    let enabled = self.active_diagram.is_some();
                    if ui
                        .add_enabled(enabled, egui::Button::new("Zoom In (+5%)"))
                        .clicked()
                    {
                        self.adjust_zoom(5.0);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Zoom Out (-5%)"))
                        .clicked()
                    {
                        self.adjust_zoom(-5.0);
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Fit Diagram"))
                        .clicked()
                    {
                        if let Some(rect) = self.last_canvas_rect {
                            self.fit_active_diagram(rect);
                        }
                        ui.close_menu();
                    }
                    if ui.add_enabled(enabled, egui::Button::new("100%")).clicked() {
                        self.reset_viewport();
                        ui.close_menu();
                    }
                    let label = self
                        .active_diagram
                        .and_then(|i| self.model.diagrams().get(i))
                        .map_or_else(
                            || "Zoom: —".to_string(),
                            |d| format!("Zoom: {:.0}%", d.zoom_percent()),
                        );
                    ui.label(label);
                });
                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("↩ Undo"))
                    .clicked()
                {
                    let _ = self.undo_action();
                    self.is_dirty = true;
                    self.status_message = "Undo".into();
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("↪ Redo"))
                    .clicked()
                {
                    let _ = self.redo_action();
                    self.is_dirty = true;
                    self.status_message = "Redo".into();
                }
                ui.separator();
                ui.label(&self.status_message);
            });
        });
    }

    /// File > New: create a new empty model.
    pub(crate) fn menu_file_new(&mut self) -> bool {
        if !self.prompt_save_if_dirty() {
            return false;
        }
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("XMI files", &["xmi"])
            .save_file()
        else {
            return false;
        };
        if path.extension().is_none_or(|ext| ext != "xmi") {
            path.set_extension("xmi");
        }
        if let Err(error) = self.new_project_at(&path) {
            self.show_save_error(&path, error.to_string());
            false
        } else {
            true
        }
    }

    /// File > Open: load an XMI file via native dialog.
    pub(crate) fn menu_file_open(&mut self) {
        if !self.prompt_save_if_dirty() {
            return;
        }
        let file = rfd::FileDialog::new()
            .add_filter("XMI files", &["xmi", "xml"])
            .pick_file();
        let Some(path) = file else {
            return;
        };
        match uml_io::xmi::load_xmi_from_file(&path) {
            Ok(model) => {
                let count = model.len();
                let diag_count = model.diagrams().len();
                self.model = model;
                self.history.clear();
                self.active_diagram = None;
                self.clear_viewport_pans();
                self.current_file_path = Some(path.clone());
                self.is_dirty = false;
                self.loaded_from_xmi = true;
                self.new_diagram_open = false;
                self.new_diagram_name.clear();
                self.selected_qa_target = None;
                self.selected_element_id = None;
                self.name_edit_buffer.clear();
                self.normalize_transient_state();
                self.status_message = format!(
                    "Loaded: {} ({} elements, {} diagrams)",
                    path.display(),
                    count,
                    diag_count
                );
            },
            Err(e) => {
                let msg = format!("Could not open '{}':\n{}", path.display(), e);
                rfd::MessageDialog::new()
                    .set_title("Error Opening File")
                    .set_description(&msg)
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
                self.status_message = format!("Error opening {}: {e}", path.display());
            },
        }
    }

    /// File > Save: save to current file path, or delegate to Save As if none.
    pub(crate) fn menu_file_save(&mut self) -> bool {
        if self.current_file_path.is_none() {
            return self.menu_file_save_as();
        }
        if let Err(error) = self.save_current() {
            let path = self.current_file_path.clone().unwrap_or_default();
            self.show_save_error(&path, error.to_string());
            false
        } else {
            true
        }
    }

    /// Save to the current path without opening a dialog.
    pub(crate) fn save_current(&mut self) -> Result<(), uml_io::xmi::XmiWriteError> {
        let path = self.current_file_path.as_ref().ok_or_else(|| {
            uml_io::xmi::XmiWriteError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "no current file path",
            ))
        })?;
        uml_io::xmi::save_xmi_to_file(&self.model, path)?;
        self.is_dirty = false;
        self.status_message = format!("Saved: {}", path.display());
        Ok(())
    }

    /// File > Save As: prompt for a path and save.
    pub(crate) fn menu_file_save_as(&mut self) -> bool {
        let file = rfd::FileDialog::new()
            .add_filter("XMI files", &["xmi"])
            .save_file();
        let Some(mut path) = file else {
            return false;
        };
        // Ensure .xmi extension
        if path.extension().is_none_or(|ext| ext != "xmi") {
            path.set_extension("xmi");
        }
        match uml_io::xmi::save_xmi_to_file(&self.model, &path) {
            Ok(_) => {
                self.current_file_path = Some(path.clone());
                self.is_dirty = false;
                self.status_message = format!("Saved: {}", path.display());
                true
            },
            Err(e) => {
                self.show_save_error(&path, e.to_string());
                false
            },
        }
    }

    fn show_save_error(&mut self, path: &std::path::Path, error: String) {
        let msg = format!("Could not save '{}':\n{error}", path.display());
        rfd::MessageDialog::new()
            .set_title("Error Saving File")
            .set_description(&msg)
            .set_buttons(rfd::MessageButtons::Ok)
            .show();
        self.status_message = format!("Error saving {}: {error}", path.display());
    }
}
