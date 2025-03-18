use std::process::Command;

use serde_json::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("cargo").arg("metadata").arg("--format-version=1").arg("--no-deps").output()?;

    let metadata = serde_json::from_slice::<Value>(&output.stdout)?;

    let crates = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| package["name"].as_str().unwrap().to_string())
        .collect::<Vec<_>>()
        .join(",");

    println!("cargo:rustc-env=_WORKSPACE_CRATES={crates}");
    Ok(())
}
