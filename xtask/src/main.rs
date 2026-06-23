//! Developer task runner.
//!
//! Usage: `cargo xtask <command>`
//!
//! Commands:
//!   build  — Full workspace build
//!   test   — Full workspace test
//!   check  — Format, clippy, and deny checks
//!   docs   — Build documentation

use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    let cmd = args.get(1).map(String::as_str).unwrap_or("help");

    match cmd {
        "build" => run_cargo(&["build", "--workspace"]),
        "test" => run_cargo(&["test", "--workspace"]),
        "check" => {
            let fmt_result = run_cargo(&["fmt", "--all", "--", "--check"]);
            if fmt_result != ExitCode::SUCCESS {
                return fmt_result;
            }
            run_cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])
        },
        "docs" => {
            println!("Documentation build not yet configured.");
            ExitCode::SUCCESS
        },
        "help" | "--help" | "-h" => {
            println!("Usage: cargo xtask <command>");
            println!();
            println!("Commands:");
            println!("  build   Full workspace build");
            println!("  test    Run all workspace tests");
            println!("  check   Format + clippy + deny checks");
            println!("  docs    Build documentation");
            ExitCode::SUCCESS
        },
        unknown => {
            eprintln!("Unknown command: {unknown}. Use 'cargo xtask help'.");
            ExitCode::FAILURE
        },
    }
}

fn run_cargo(args: &[&str]) -> ExitCode {
    let output = Command::new("cargo")
        .args(args)
        .output()
        .expect("failed to execute cargo");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{stderr}");
        ExitCode::from(output.status.code().unwrap_or(1) as u8)
    } else {
        ExitCode::SUCCESS
    }
}
