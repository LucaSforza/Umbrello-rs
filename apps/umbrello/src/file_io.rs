//! File I/O helpers — prompting for unsaved changes and save/load orchestration.

use crate::app::UmbrelloApp;
use std::path::Path;

impl UmbrelloApp {
    /// Prompt the user to save unsaved changes.
    /// Returns `true` if the operation should proceed, `false` if cancelled.
    pub(crate) fn prompt_save_if_dirty(&mut self) -> bool {
        if !self.is_dirty {
            return true;
        }
        let result = rfd::MessageDialog::new()
            .set_title("Unsaved Changes")
            .set_description(
                "The model has unsaved changes. Do you want to save before continuing?",
            )
            .set_buttons(rfd::MessageButtons::YesNoCancel)
            .show();
        match result {
            rfd::MessageDialogResult::Yes => self.menu_file_save(),
            rfd::MessageDialogResult::No => true,
            rfd::MessageDialogResult::Cancel => false,
            rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Custom(_) => false,
        }
    }

    /// Create a new XMI project without changing application state until the
    /// initial empty model has been written successfully.
    pub(crate) fn new_project_at(&mut self, path: &Path) -> Result<(), uml_io::xmi::XmiWriteError> {
        let new_model = uml_core::UmlModel::new();
        uml_io::xmi::save_xmi_to_file(&new_model, path)?;
        self.model = new_model;
        self.history.clear();
        self.current_file_path = Some(path.to_path_buf());
        self.is_dirty = false;
        self.loaded_from_xmi = false;
        self.active_diagram = None;
        self.clear_viewport_pans();
        self.new_diagram_open = false;
        self.new_diagram_name.clear();
        self.selected_qa_target = None;
        self.selected_element_id = None;
        self.name_edit_buffer.clear();
        self.normalize_transient_state();
        self.status_message = format!("New project: {}", path.display());
        self.bump_state();
        Ok(())
    }
}
