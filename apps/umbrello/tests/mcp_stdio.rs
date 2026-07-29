//! Display-independent smoke coverage for the opt-in MCP CLI surface.

#[test]
fn cli_advertises_mcp_stdio_mode() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_umbrello"))
        .arg("--help")
        .output()
        .expect("Umbrello binary should be available to integration tests");
    assert!(output.status.success());
    let help = String::from_utf8_lossy(&output.stdout);
    assert!(help.contains("--mcp-stdio"));
}
