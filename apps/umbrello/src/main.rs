//! Umbrello-RS — UML modeling tool (GUI mode).
//!
//! Uses egui for immediate-mode rendering. Reads UML models via uml-core
//! and dispatches Commands to the History manager for undo/redo.

mod app;

use app::UmbrelloApp;
use std::io::BufReader;
use uml_core::UmlModel;
use uml_io::xmi::XmiReader;

fn main() -> anyhow::Result<()> {
    let mut model = UmlModel::new();
    // Try multiple paths: from workspace root (cargo run), from binary dir
    let loaded = load_xmi("../test/test-COG.xmi", &mut model)
        || load_xmi("../../test/test-COG.xmi", &mut model)
        || load_xmi("test/test-COG.xmi", &mut model);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("Umbrello-RS"),
        ..Default::default()
    };

    eframe::run_native(
        "Umbrello-RS",
        options,
        Box::new(|_cc| Ok(Box::new(UmbrelloApp::new(model, loaded)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}

fn load_xmi(path: &str, model: &mut UmlModel) -> bool {
    if let Ok(file) = std::fs::File::open(path) {
        let mut reader = XmiReader::new();
        if reader.read_from(BufReader::new(file), model).is_ok() {
            let _ = reader.resolve(model);
            return true;
        }
    }
    false
}
