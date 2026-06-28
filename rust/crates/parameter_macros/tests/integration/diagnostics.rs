use std::{fs, path::PathBuf, process::Command};

#[test]
fn missing_default_reports_field_attribute_span() {
    assert_compile_error(
        "missing_default",
        r#"
use parameter_macros::parameter_group;

#[parameter_group(
    accessor = ExampleConfigAccessor,
    parameters = ExampleParameters,
    default_parameters = DefaultExampleParameters,
    visitor = ExampleParameterVisitor
)]
pub struct ExampleConfig {
    #[parameter(label = "Example value", range = 1.0..=5.0)]
    value: u8,
}
"#,
        &[
            "missing required argument `default` in #[parameter(...)]",
            "src/lib.rs:10:",
            "#[parameter(label = \"Example value\", range = 1.0..=5.0)]",
            "   |     ^^^^^^^^^^^",
        ],
    );
}

#[test]
fn unknown_parameter_key_reports_key_with_suggestion() {
    assert_compile_error(
        "unknown_parameter_key",
        r#"
use parameter_macros::parameter_group;

#[parameter_group(
    accessor = ExampleConfigAccessor,
    parameters = ExampleParameters,
    default_parameters = DefaultExampleParameters,
    visitor = ExampleParameterVisitor
)]
pub struct ExampleConfig {
    #[parameter(label = "Example value", ranges = 1.0..=5.0, default = 3)]
    value: u8,
}
"#,
        &["unknown argument `ranges` in #[parameter(...)]", "help: did you mean `range`?", "src/lib.rs:10:"],
    );
}

#[test]
fn duplicate_parameter_key_reports_key() {
    assert_compile_error(
        "duplicate_parameter_key",
        r#"
use parameter_macros::parameter_group;

#[parameter_group(
    accessor = ExampleConfigAccessor,
    parameters = ExampleParameters,
    default_parameters = DefaultExampleParameters,
    visitor = ExampleParameterVisitor
)]
pub struct ExampleConfig {
    #[parameter(label = "Example value", label = "Duplicate", range = 1.0..=5.0, default = 3)]
    value: u8,
}
"#,
        &["duplicate argument `label` in #[parameter(...)]", "src/lib.rs:10:"],
    );
}

#[test]
fn unknown_group_key_reports_group_key() {
    assert_compile_error(
        "unknown_group_key",
        r#"
use parameter_macros::parameter_group;

#[parameter_group(
    accessors = ExampleConfigAccessor,
    parameters = ExampleParameters,
    default_parameters = DefaultExampleParameters,
    visitor = ExampleParameterVisitor
)]
pub struct ExampleConfig {
    #[parameter(label = "Example value", range = 1.0..=5.0, default = 3)]
    value: u8,
}
"#,
        &["unknown argument `accessors` in #[parameter_group(...)]", "src/lib.rs:4:"],
    );
}

#[test]
fn parameter_spec_constructor_is_not_public_api() {
    assert_compile_error(
        "parameter_spec_constructor",
        r#"
use parameter::ParameterSpec;

pub const SPEC: ParameterSpec<f32> = ParameterSpec::new(
    "Example",
    None,
    0.0..=1.0,
    0.0,
    false,
    0.5,
);
"#,
        &["no associated function or constant named `new` found for struct `ParameterSpec"],
    );
}

fn assert_compile_error(case_name: &str, source: &str, expected_stderr: &[&str]) {
    let fixture_dir = diagnostics_root().join(case_name);
    let src_dir = fixture_dir.join("src");
    fs::create_dir_all(&src_dir).expect("failed to create diagnostic fixture src dir");
    fs::write(fixture_dir.join("Cargo.toml"), fixture_manifest()).expect("failed to write diagnostic fixture manifest");
    fs::write(src_dir.join("lib.rs"), source.trim_start()).expect("failed to write diagnostic fixture source");

    let output = Command::new(env!("CARGO"))
        .arg("check")
        .arg("--color")
        .arg("never")
        .arg("--quiet")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(fixture_dir.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", diagnostics_root().join("target"))
        .output()
        .expect("failed to run diagnostic fixture cargo check");

    assert!(!output.status.success(), "diagnostic fixture unexpectedly compiled successfully");

    let stderr = String::from_utf8_lossy(&output.stderr);
    for expected in expected_stderr {
        assert!(stderr.contains(expected), "expected diagnostic stderr to contain {expected:?}\n\nstderr:\n{stderr}");
    }
}

fn fixture_manifest() -> String {
    format!(
        r#"
[package]
name = "parameter-macro-diagnostic-fixture"
version = "0.0.0"
edition = "2024"

[dependencies]
parameter = {{ path = "{}" }}
parameter_macros = {{ path = "{}" }}

[workspace]
"#,
        parameter_crate_dir().display(),
        parameter_macros_dir().display()
    )
}

fn diagnostics_root() -> PathBuf {
    rust_workspace_dir().join("target/parameter-macro-diagnostic-fixtures")
}

fn parameter_macros_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn parameter_crate_dir() -> PathBuf {
    crates_dir().join("parameter")
}

fn rust_workspace_dir() -> PathBuf {
    crates_dir().parent().expect("crates should live under the Rust workspace").to_path_buf()
}

fn crates_dir() -> PathBuf {
    parameter_macros_dir().parent().expect("parameter_macros should live under crates").to_path_buf()
}
