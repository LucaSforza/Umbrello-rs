//! Umbrello-RS — UML modeling tool (GUI mode).
//!
//! Uses egui for immediate-mode rendering. Reads UML models via uml-core
//! and dispatches Commands to the History manager for undo/redo.

mod app;
mod canvas;
mod file_io;
mod menu;
mod property_editor;
mod rendering;
mod tests;
mod tool_palette;
mod tree;

use app::UmbrelloApp;
use clap::Parser;
use std::io::BufReader;
use std::path::PathBuf;
use tokio_util::sync::CancellationToken;
use uml_core::UmlModel;
use uml_io::xmi::XmiReader;

#[derive(Parser)]
#[command(name = "umbrello", about = "UML modeling tool")]
struct Cli {
    /// Run the opt-in MCP GUI QA server over stdin/stdout.
    #[arg(long)]
    mcp_stdio: bool,
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

    let (qa_bridge, qa_handle, shutdown, mcp_thread) = if cli.mcp_stdio {
        let (bridge, handle) = app::qa::QaBridge::new(64);
        let shutdown = CancellationToken::new();
        let thread_shutdown = shutdown.clone();
        let thread_handle = handle.clone();
        let thread = std::thread::spawn(move || {
            if let Err(error) = app::qa::mcp::run_stdio(thread_handle, thread_shutdown) {
                eprintln!("MCP server stopped: {error}");
            }
            app::qa::mcp::close_gui_on_eof();
        });
        (Some(bridge), Some(handle), Some(shutdown), Some(thread))
    } else {
        (None, None, None, None)
    };

    let result = eframe::run_native(
        &title,
        options,
        Box::new(move |_cc| {
            if let Some(ctx) = qa_bridge.as_ref().map(|_| &_cc.egui_ctx) {
                app::qa::bridge::install_repaint_context(ctx);
            }
            let mut app = UmbrelloApp::new(model, loaded);
            app.set_current_file_path(current_file_path);
            if let Some(bridge) = qa_bridge {
                app.qa_bridge = Some(bridge);
            }
            Ok(Box::new(app))
        }),
    );

    if let Some(shutdown) = shutdown {
        shutdown.cancel();
    }
    if let Some(handle) = qa_handle {
        app::qa::bridge::request_repaint();
        drop(handle);
    }
    if let Some(thread) = mcp_thread {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !thread.is_finished() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        if thread.is_finished() {
            let _ = thread.join();
        } else {
            eprintln!("MCP server did not stop within shutdown deadline");
        }
    }
    result.map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}
