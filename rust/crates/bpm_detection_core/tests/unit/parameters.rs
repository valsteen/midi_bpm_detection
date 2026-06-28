use parameter::{Parameter, ParameterSpec};

use super::*;

struct DynamicParameterLabels(Vec<&'static str>);

impl DynamicBPMDetectionParameterVisitor<DynamicBPMDetectionConfig> for DynamicParameterLabels {
    fn beats_lookback(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, u8>) {
        self.0.push(parameter.spec.label);
    }

    fn normal_distribution_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn time_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn velocity_current_note_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn velocity_note_from_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn in_beat_range_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn multiplier_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn subdivision_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn octave_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn pitch_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }

    fn high_tempo_bias_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
        self.0.push(parameter.spec.label);
    }
}

struct NormalDistributionParameterLabels(Vec<&'static str>);

impl NormalDistributionParameterVisitor<NormalDistributionConfig> for NormalDistributionParameterLabels {
    fn std_dev(&mut self, parameter: Parameter<NormalDistributionConfig, f64>) {
        self.0.push(parameter.spec.label);
    }

    fn resolution(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
        self.0.push(parameter.spec.label);
    }

    fn cutoff(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
        self.0.push(parameter.spec.label);
    }

    fn factor(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
        self.0.push(parameter.spec.label);
    }
}

struct NormalDistributionParameterFields(Vec<&'static str>);

impl NormalDistributionParameterVisitor<NormalDistributionConfig> for NormalDistributionParameterFields {
    fn std_dev(&mut self, _parameter: Parameter<NormalDistributionConfig, f64>) {
        self.0.push("std_dev");
    }

    fn resolution(&mut self, _parameter: Parameter<NormalDistributionConfig, f32>) {
        self.0.push("resolution");
    }

    fn cutoff(&mut self, _parameter: Parameter<NormalDistributionConfig, f32>) {
        self.0.push("cutoff");
    }

    fn factor(&mut self, _parameter: Parameter<NormalDistributionConfig, f32>) {
        self.0.push("factor");
    }
}

struct StaticParameterLabels(Vec<&'static str>);

impl StaticBPMDetectionParameterVisitor<StaticBPMDetectionConfig> for StaticParameterLabels {
    fn bpm_center(&mut self, parameter: Parameter<StaticBPMDetectionConfig, f32>) {
        self.0.push(parameter.spec.label);
    }

    fn bpm_range(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>) {
        self.0.push(parameter.spec.label);
    }

    fn sample_rate(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>) {
        self.0.push(parameter.spec.label);
    }
}

struct StaticConfigWrapper {
    config: StaticBPMDetectionConfig,
}

struct ExpectedStaticParameterSpec<ValueType> {
    label: &'static str,
    unit: Option<&'static str>,
    range_start: f64,
    range_end: f64,
    step: f64,
    logarithmic: bool,
    default: ValueType,
}

impl StaticBPMDetectionConfigAccessor for StaticConfigWrapper {
    fn bpm_center(&self) -> f32 {
        self.config.bpm_center
    }

    fn bpm_range(&self) -> u16 {
        self.config.bpm_range
    }

    fn sample_rate(&self) -> u16 {
        self.config.sample_rate
    }

    fn set_bpm_center(&mut self, val: f32) {
        self.config.bpm_center = val;
    }

    fn set_bpm_range(&mut self, val: u16) {
        self.config.bpm_range = val;
    }

    fn set_sample_rate(&mut self, val: u16) {
        self.config.sample_rate = val;
    }
}

#[test]
fn dynamic_parameter_visitor_lists_every_dynamic_parameter() {
    let mut labels = DynamicParameterLabels(Vec::new());

    DynamicBPMDetectionConfig::PARAMETERS.visit(&mut labels);

    assert_eq!(
        labels.0,
        [
            "Beats Lookback",
            "Normal distribution",
            "Time distance",
            "Note velocity",
            "From note velocity",
            "In beat range",
            "Multiplier",
            "Subdivision",
            "Octave distance",
            "Pitch distance",
            "High tempo bias",
        ]
    );
}

#[test]
fn static_parameter_specs_preserve_inventory() {
    let parameter_specs = StaticBPMDetectionConfig::PARAMETER_SPECS;

    assert_parameter_spec(&parameter_specs.bpm_center());
    assert_parameter_spec(&parameter_specs.bpm_range());
    assert_parameter_spec(&parameter_specs.sample_rate());

    assert_static_parameter_spec(
        &parameter_specs.bpm_center(),
        &ExpectedStaticParameterSpec {
            label: "BPM center",
            unit: None,
            range_start: 1.0,
            range_end: 150.0,
            step: 0.01,
            logarithmic: false,
            default: 90.0,
        },
    );
    assert_static_parameter_spec(
        &parameter_specs.bpm_range(),
        &ExpectedStaticParameterSpec {
            label: "BPM range",
            unit: None,
            range_start: 1.0,
            range_end: 100.0,
            step: 1.0,
            logarithmic: false,
            default: 40,
        },
    );
    assert_static_parameter_spec(
        &parameter_specs.sample_rate(),
        &ExpectedStaticParameterSpec {
            label: "BPM sample rate",
            unit: Some("samples/second"),
            range_start: 1.0,
            range_end: 1_0000.0,
            step: 1.0,
            logarithmic: true,
            default: 450,
        },
    );
}

#[test]
fn static_parameter_visitor_lists_static_parameter_fields_only() {
    let mut labels = StaticParameterLabels(Vec::new());

    StaticBPMDetectionConfig::PARAMETERS.visit(&mut labels);

    assert_eq!(labels.0, ["BPM center", "BPM range", "BPM sample rate"]);
}

#[test]
fn static_validation_includes_nested_normal_distribution() {
    let mut config = StaticBPMDetectionConfig::default();
    config.normal_distribution.std_dev = 3.0;

    assert_eq!(config.validate(), Err("Standard deviation value 3 is outside declared range 4..=40".to_string()));
}

#[test]
fn static_computed_methods_work_through_accessor_extension() {
    let config = StaticBPMDetectionConfig { bpm_center: 90.0, bpm_range: 40, sample_rate: 450, ..Default::default() };
    let wrapper = StaticConfigWrapper { config: config.clone() };

    assert_f32_eq(wrapper.lowest_bpm(), config.lowest_bpm());
    assert_f32_eq(wrapper.highest_bpm(), config.highest_bpm());
    assert_f32_eq(wrapper.index_to_bpm(0), config.index_to_bpm(0));
    assert_f32_eq(wrapper.index_to_bpm(17), config.index_to_bpm(17));
}

#[test]
fn normal_distribution_parameter_specs_and_visitor_preserve_inventory() {
    let parameter_specs = NormalDistributionConfig::PARAMETER_SPECS;

    assert_parameter_spec(&parameter_specs.std_dev());
    assert_parameter_spec(&parameter_specs.resolution());
    assert_parameter_spec(&parameter_specs.cutoff());
    assert_parameter_spec(&parameter_specs.factor());

    let mut labels = NormalDistributionParameterLabels(Vec::new());

    NormalDistributionConfig::PARAMETERS.visit(&mut labels);

    assert_eq!(
        labels.0,
        ["Standard deviation", "Normal distribution resolution", "Normal distribution cutoff", "factor",]
    );
}

#[test]
fn normal_distribution_generated_traversal_matches_settings_order() {
    let mut fields = NormalDistributionParameterFields(Vec::new());

    NormalDistributionConfig::PARAMETERS.visit(&mut fields);

    assert_eq!(fields.0, ["std_dev", "resolution", "cutoff", "factor"]);
}

fn assert_parameter_spec<ValueType>(_: &ParameterSpec<ValueType>) {}

fn assert_static_parameter_spec<ValueType: Asf64>(
    spec: &ParameterSpec<ValueType>,
    expected: &ExpectedStaticParameterSpec<ValueType>,
) {
    assert_eq!(spec.label, expected.label);
    assert_eq!(spec.unit, expected.unit);
    assert_f64_eq(*spec.range.start(), expected.range_start);
    assert_f64_eq(*spec.range.end(), expected.range_end);
    assert_f64_eq(spec.step, expected.step);
    assert_eq!(spec.logarithmic, expected.logarithmic);
    assert_f64_eq(spec.default.as_f64(), expected.default.as_f64());
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() <= f32::EPSILON);
}

fn assert_f64_eq(actual: f64, expected: f64) {
    assert!((actual - expected).abs() <= f64::EPSILON);
}
