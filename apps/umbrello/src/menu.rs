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
                    if ui.button("New\tCtrl+N").clicked() {
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
                    if ui.button("Undo").clicked()
                        || (ui
                            .ctx()
                            .input(|i| i.key_pressed(egui::Key::Z) && i.modifiers.ctrl))
                    {
                        if self.history.can_undo() {
                            self.history.undo(&mut self.model).unwrap();
                            self.is_dirty = true;
                            self.status_message = "Undo".into();
                        }
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked()
                        || (ui
                            .ctx()
                            .input(|i| i.key_pressed(egui::Key::Y) && i.modifiers.ctrl))
                    {
                        if self.history.can_redo() {
                            self.history.redo(&mut self.model).unwrap();
                            self.is_dirty = true;
                            self.status_message = "Redo".into();
                        }
                        ui.close_menu();
                    }
                });
                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("↩ Undo"))
                    .clicked()
                {
                    self.history.undo(&mut self.model).unwrap();
                    self.is_dirty = true;
                    self.status_message = "Undo".into();
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("↪ Redo"))
                    .clicked()
                {
                    self.history.redo(&mut self.model).unwrap();
                    self.is_dirty = true;
                    self.status_message = "Redo".into();
                }
                ui.separator();
                ui.label(&self.status_message);
            });
        });
    }

    /// File > New: create a new empty model.
    pub(crate) fn menu_file_new(&mut self) {
        if !self.prompt_save_if_dirty() {
            return;
        }
        self.model = UmlModel::new();
        self.history.clear();
        self.active_diagram = None;
        self.current_file_path = None;
        self.is_dirty = false;
        self.status_message = "New model created".into();
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
                self.current_file_path = Some(path.clone());
                self.is_dirty = false;
                self.loaded_from_xmi = true;
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
    pub(crate) fn menu_file_save(&mut self) {
        match &self.current_file_path {
            Some(path) => match uml_io::xmi::save_xmi_to_file(&self.model, path) {
                Ok(_) => {
                    self.is_dirty = false;
                    self.status_message = format!("Saved: {}", path.display());
                },
                Err(e) => {
                    let msg = format!("Could not save '{}':\n{}", path.display(), e);
                    rfd::MessageDialog::new()
                        .set_title("Error Saving File")
                        .set_description(&msg)
                        .set_buttons(rfd::MessageButtons::Ok)
                        .show();
                    self.status_message = format!("Error saving {}: {e}", path.display());
                },
            },
            None => self.menu_file_save_as(),
        }
    }

    /// File > Save As: prompt for a path and save.
    pub(crate) fn menu_file_save_as(&mut self) {
        let file = rfd::FileDialog::new()
            .add_filter("XMI files", &["xmi"])
            .save_file();
        let Some(mut path) = file else {
            return;
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
            },
            Err(e) => {
                let msg = format!("Could not save '{}':\n{}", path.display(), e);
                rfd::MessageDialog::new()
                    .set_title("Error Saving File")
                    .set_description(&msg)
                    .set_buttons(rfd::MessageButtons::Ok)
                    .show();
                self.status_message = format!("Error saving {}: {e}", path.display());
            },
        }
    }
}

use uml_core::UmlModel;
