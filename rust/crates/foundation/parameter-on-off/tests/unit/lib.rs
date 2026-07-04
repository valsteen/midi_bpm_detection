use parameter::Asf64;
use serde::Deserialize;

use super::OnOff;

#[derive(Deserialize)]
struct Config {
    value: OnOff<f32>,
}

#[test]
fn shorthand_toml_deserializes_as_enabled_value() {
    let config: Config = toml::from_str("value = 0.75").expect("shorthand OnOff value should deserialize");

    assert_eq!(config.value, OnOff::On(0.75));
}

#[test]
fn full_toml_deserializes_enabled_state_and_value() {
    let config: Config =
        toml::from_str("[value]\nenabled = false\nvalue = 0.5\n").expect("full OnOff table should deserialize");

    assert_eq!(config.value, OnOff::Off(0.5));
}

#[test]
fn full_toml_defaults_missing_enabled_to_true() {
    let config: Config = toml::from_str("[value]\nvalue = 0.5\n").expect("enabled-less OnOff table should deserialize");

    assert_eq!(config.value, OnOff::On(0.5));
}

#[test]
fn serialization_preserves_full_enabled_value_form() {
    let serialized = toml::to_string(&OnOff::Off(0.25)).expect("OnOff should serialize");

    assert!(serialized.contains("enabled = false"));
    assert!(serialized.contains("value = 0.25"));
}

#[test]
fn numeric_helpers_preserve_enabled_semantics() {
    let mut value = OnOff::Off(0.5_f32);

    assert_f32_eq(value.multiplier(), 1.0);
    assert_f32_eq(value.weight(), 0.0);
    assert_f64_eq(value.as_f64(), 0.5);

    value.set_from_f64(0.75);

    assert_eq!(value, OnOff::Off(0.75));
    value.set_enabled(true);
    assert_eq!(value, OnOff::On(0.75));
    assert_f32_eq(value.multiplier(), 0.75);
    assert_f32_eq(value.weight(), 0.75);
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON, "expected {actual} to equal {expected}");
}

fn assert_f64_eq(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= f64::EPSILON, "expected {actual} to equal {expected}");
}
