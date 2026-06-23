//! Umbrello-RS — UML modeling tool.

fn main() -> anyhow::Result<()> {
    println!("Umbrello-RS v{}", uml_core::common::version::UMBRELLO_VERSION);
    println!("(CLI and GUI not yet implemented — coming in Phase 2)");
    Ok(())
}
