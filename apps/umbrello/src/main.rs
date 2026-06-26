//! Umbrello-RS — UML modeling tool (GUI mode).
//!
//! Uses egui for immediate-mode rendering. Reads UML models via uml-core
//! and dispatches Commands to the History manager for undo/redo.

mod app;

use app::UmbrelloApp;
use clap::Parser;
use std::io::BufReader;
use std::path::PathBuf;
use uml_core::UmlModel;
use uml_io::xmi::XmiReader;

#[derive(Parser)]
#[command(name = "umbrello", about = "UML modeling tool")]
struct Cli {
    /// Path to an XMI file to open on startup.
    file: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let mut model = UmlModel::new();
    let mut loaded = false;

    if let Some(path) = &cli.file {
        if let Ok(file) = std::fs::File::open(path) {
            let mut reader = XmiReader::new();
            if reader.read_from(BufReader::new(file), &mut model).is_ok() {
                let _ = reader.resolve(&mut model);
                loaded = true;
            }
        }
    }

    let title = if loaded {
        format!("Umbrello-RS — {}", cli.file.as_ref().unwrap())
    } else {
        "Umbrello-RS — Untitled".into()
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title(&title),
        ..Default::default()
    };

    let current_file_path: Option<PathBuf> = if loaded {
        cli.file.map(PathBuf::from)
    } else {
        None
    };

    eframe::run_native(
        &title,
        options,
        Box::new(move |_cc| {
            let mut app = UmbrelloApp::new(model, loaded);
            app.set_current_file_path(current_file_path);
            Ok(Box::new(app))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
