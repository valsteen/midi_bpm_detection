use clap::{Parser, Subcommand};
use std::process::Command as StdCommand;

#[derive(Parser)]
struct Args {
    #[command(subcommand)]
    command: CiCommand,
}

#[derive(Subcommand)]
enum CiCommand {
    ClippyAll,
    ClippyHack,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    match args.command {
        CiCommand::ClippyAll => clippy_all()?,
        CiCommand::ClippyHack => clippy_hack()?,
    }

    Ok(())
}

fn clippy_all() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Clippy All Combinations ===");

    check_wasm_installed()?;

    println!("\n>>> Native (all features)");
    run("cargo", &["clippy", "--workspace", "--all-features", "--", "-D", "warnings"])?;

    println!("\n>>> Native (on_off_widgets feature)");
    run("cargo", &["clippy", "-p", "gui", "--features", "on_off_widgets", "--", "-D", "warnings"])?;

    println!("\n>>> WASM32 (all features)");
    run(
        "cargo",
        &[
            "clippy",
            "--target",
            "wasm32-unknown-unknown",
            "--workspace",
            "--exclude",
            "tui",
            "--exclude",
            "midi-bpm-detector-plugin",
            "--exclude",
            "xtask",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    println!("\n=== All checks passed! ===");
    Ok(())
}

fn clippy_hack() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Clippy with cargo-hack ===");

    if StdCommand::new("cargo-hack").output().is_err() {
        println!("Error: cargo-hack not found.");
        println!("Install with: cargo install cargo-hack");
        std::process::exit(1);
    }

    println!("\n>>> Native (each-feature)");
    run("cargo", &["hack", "clippy", "--each-feature", "--", "-D", "warnings"])?;

    check_wasm_installed()?;

    println!("\n>>> WASM32 (each-feature)");
    run(
        "cargo",
        &[
            "hack",
            "clippy",
            "--target",
            "wasm32-unknown-unknown",
            "--workspace",
            "--exclude",
            "tui",
            "--exclude",
            "midi-bpm-detector-plugin",
            "--exclude",
            "xtask",
            "--exclude",
            "ci",
            "--each-feature",
            "--",
            "-D",
            "warnings",
        ],
    )?;

    println!("\n=== All checks passed! ===");
    Ok(())
}

fn check_wasm_installed() -> Result<(), Box<dyn std::error::Error>> {
    let output = StdCommand::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("Failed to check installed targets");

    if !String::from_utf8_lossy(&output.stdout).contains("wasm32-unknown-unknown") {
        println!("Installing wasm32-unknown-unknown target...");
        run("rustup", &["target", "add", "wasm32-unknown-unknown"])?;
    }
    Ok(())
}

fn run(program: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let status = StdCommand::new(program).args(args).status()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
