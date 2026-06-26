//! File I/O helpers — prompting for unsaved changes and save/load orchestration.

use crate::app::UmbrelloApp;

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
            rfd::MessageDialogResult::Yes => {
                self.menu_file_save();
                true
            },
            rfd::MessageDialogResult::No => true,
            rfd::MessageDialogResult::Cancel => false,
            // Unexpected for YesNoCancel — treat as proceed
            rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Custom(_) => true,
        }
    }
}
