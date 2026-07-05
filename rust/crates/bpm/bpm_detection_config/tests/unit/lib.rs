use std::time::Duration;

use parameter::{Parameter, ParameterSpec};

use super::*;

struct SettingsOwnerProbe {
    settings: Settings,
    static_updates: usize,
    dynamic_updates: usize,
    gui_updates: usize,
}

impl SettingsOwnerProbe {
    fn new() -> Self {
        Self { settings: Settings::default(), static_updates: 0, dynamic_updates: 0, gui_updates: 0 }
    }
}

impl SettingsOwner for SettingsOwnerProbe {
    fn bpm_detection_settings(&self) -> &Settings {
        &self.settings
    }

    fn bpm_detection_settings_mut(&mut self) -> &mut Settings {
        &mut self.settings
    }

    fn after_gui_config_set(&mut self) {
        self.gui_updates += 1;
    }

    fn after_static_bpm_detection_config_set(&mut self) {
        self.static_updates += 1;
    }

    fn after_dynamic_bpm_detection_config_set(&mut self) {
        self.dynamic_updates += 1;
    }
}

struct GUIParameterLabels(Vec<&'static str>);

impl GUIParameterVisitor<GUIConfig> for GUIParameterLabels {
    fn interpolation_duration(&mut self, parameter: Parameter<GUIConfig, Duration>) {
        self.0.push(parameter.spec.label);
    }

    fn interpolation_curve(&mut self, parameter: Parameter<GUIConfig, f32>) {
        self.0.push(parameter.spec.label);
    }
}

#[test]
fn gui_parameter_specs_and_visitor_preserve_inventory() {
    let parameter_specs = GUIConfig::PARAMETER_SPECS;

    assert_parameter_spec(&parameter_specs.interpolation_duration());
    assert_parameter_spec(&parameter_specs.interpolation_curve());

    let mut labels = GUIParameterLabels(Vec::new());

    GUIConfig::PARAMETERS.visit(&mut labels);

    assert_eq!(labels.0, ["Interpolation duration", "Interpolation curve"]);
}

#[test]
fn settings_owner_delegates_generated_config_owners_through_settings() {
    let mut owner = SettingsOwnerProbe::new();

    owner.set_bpm_center(120.0);
    owner.set_beats_lookback(12);
    owner.set_interpolation_curve(0.35);

    assert_f32_eq(owner.settings.static_bpm_detection_config.bpm_center, 120.0);
    assert_eq!(owner.settings.dynamic_bpm_detection_config.beats_lookback, 12);
    assert_f32_eq(owner.settings.gui_config.interpolation_curve, 0.35);
    assert_eq!(owner.static_updates, 1);
    assert_eq!(owner.dynamic_updates, 1);
    assert_eq!(owner.gui_updates, 1);
}

#[test]
fn normal_distribution_owner_delegates_to_static_settings_and_static_hook() {
    let mut owner = SettingsOwnerProbe::new();

    owner.set_std_dev(18.0);

    assert_f64_eq(owner.settings.static_bpm_detection_config.normal_distribution.std_dev, 18.0);
    assert_eq!(owner.static_updates, 1);
    assert_eq!(owner.dynamic_updates, 0);
    assert_eq!(owner.gui_updates, 0);
}

fn assert_parameter_spec<ValueType>(_: &ParameterSpec<ValueType>) {}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON);
}

fn assert_f64_eq(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= f64::EPSILON);
}
