//! Umbrello-RS command-line interface.
//!
//! Provides headless operations: XMI validation, diagram export, code import.

use clap::Parser;

/// Umbrello — UML modeling tool (CLI mode).
#[derive(Parser)]
#[command(name = "umbrello", version, about)]
struct Cli {
    /// Optional XMI file to open.
    file: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let _cli = Cli::parse();
    println!("Umbrello-RS CLI v{}", uml_common::version::UMBRELLO_VERSION);
    println!("(CLI not yet implemented — Phase 7)");
    Ok(())
}
