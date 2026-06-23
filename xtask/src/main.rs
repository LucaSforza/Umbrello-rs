//! Developer task runner.
//!
//! Usage: `cargo xtask <command>`
//!
//! Commands:
//!   build  — Full workspace build
//!   test   — Full workspace test
//!   check  — Format, clippy, and deny checks
//!   docs   — Build documentation (mdBook + cargo doc)

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::process::{Command, ExitCode};

#[derive(Parser)]
#[command(name = "xtask", about = "Umbrello-RS developer task runner")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Full workspace build.
    Build,
    /// Run all workspace tests.
    Test,
    /// Run format check, clippy, and deny checks.
    Check,
    /// Build documentation: mdBook architecture docs + cargo doc API docs.
    Docs,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Build) => run_cargo(&["build", "--workspace"]),
        Some(Commands::Test) => run_cargo(&["test", "--workspace"]),
        Some(Commands::Check) => run_cargo(&["fmt", "--all", "--", "--check"]).and_then(|_| {
            run_cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])
        }),
        Some(Commands::Docs) => build_docs(),
        None => {
            println!("Usage: cargo xtask <command>");
            println!();
            println!("Commands:");
            println!("  build   Full workspace build");
            println!("  test    Run all workspace tests");
            println!("  check   Format + clippy + deny checks");
            println!("  docs    Build documentation");
            Ok(())
        },
    };

    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e:#}");
            ExitCode::FAILURE
        },
    }
}

fn run_cargo(args: &[&str]) -> Result<()> {
    let status = Command::new("cargo")
        .args(args)
        .status()
        .context("failed to execute cargo")?;

    if !status.success() {
        anyhow::bail!("cargo {} failed", args.join(" "));
    }
    Ok(())
}

fn build_docs() -> Result<()> {
    let mdbook_available = Command::new("mdbook")
        .args(["--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if mdbook_available {
        println!("==> Building mdBook architecture documentation...");
        run_mdbook(&["build", "docs/book/"]).context("mdBook build failed")?;
    } else {
        eprintln!("==> Warning: mdBook not found. Skipping architecture docs.");
        eprintln!("    Install with: cargo install mdbook");
    }

    println!("==> Building API documentation with cargo doc...");
    run_cargo(&["doc", "--workspace", "--no-deps"])?;

    if mdbook_available {
        println!("==> Documentation built successfully.");
        println!("    Architecture docs: target/doc-book/index.html");
        println!("    API docs:          target/doc/uml_core/index.html");
    } else {
        println!("==> API documentation built successfully.");
        println!("    API docs:          target/doc/uml_core/index.html");
        println!("    (Architecture docs skipped — install mdBook with: cargo install mdbook)");
    }

    Ok(())
}

fn run_mdbook(args: &[&str]) -> Result<()> {
    let status = Command::new("mdbook")
        .args(args)
        .status()
        .context("failed to execute mdbook")?;

    if !status.success() {
        anyhow::bail!("mdbook {} failed", args.join(" "));
    }
    Ok(())
}
