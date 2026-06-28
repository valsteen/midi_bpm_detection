use parameter::parameter_group;
use parameter::{Asf64, Parameter, ParameterSpec};

#[parameter_group]
struct ExampleConfig {
    #[parameter(label = "Example value", range = 1.0..=5.0, step = 1.0, default = 3)]
    value: u8,
    #[parameter(label = "Weight", range = 0.0..=2.0, default = 1.25, const_name = WEIGHT, setter = set_weight)]
    weight: f32,
}

#[parameter_group]
struct NestedExampleConfig {
    #[parameter(label = "Example value", range = 1.0..=5.0, step = 1.0, default = 3)]
    value: u8,
    nested: NestedConfig,
}

#[derive(Default)]
struct NestedConfig {
    valid: bool,
}

impl NestedConfig {
    fn validate(&self) -> Result<(), String> {
        self.valid.then_some(()).ok_or_else(|| "nested config is invalid".to_string())
    }
}

#[parameter_group]
struct GUIConfig {
    #[parameter(label = "Interpolation", range = 0.0..=1.0, default = 0.5)]
    interpolation: f32,
}

#[parameter_group]
struct StaticBPMDetectionConfig {
    #[parameter(label = "BPM center", range = 1.0..=150.0, default = 90.0)]
    bpm_center: f32,
}

#[parameter_group]
struct RuntimeSettings {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.8)]
    gain: f32,
}

#[test]
fn parameter_group_generates_accessor_defaults_parameters_and_visit_order() {
    let mut config = ExampleConfig::default();

    assert_eq!(config.value(), 3);
    assert_f32_eq(config.weight(), 1.25);

    config.set_value(4);
    config.set_weight(0.5);

    assert_eq!(config.value, 4);
    assert_f32_eq(config.weight, 0.5);

    let value_spec = ExampleConfig::PARAMETER_SPECS.value();
    let weight_spec = ExampleConfig::PARAMETER_SPECS.weight();

    assert_parameter_spec(&value_spec);
    assert_parameter_spec(&weight_spec);
    assert_eq!(value_spec.label, "Example value");
    assert_f64_eq(value_spec.step, 1.0);
    assert!(!value_spec.logarithmic);
    assert_f32_eq(weight_spec.default, 1.25);

    let value_parameter = ExampleConfig::PARAMETERS.value();
    assert_eq!(value_parameter.spec.label, "Example value");
    assert_f64_eq(value_parameter.spec.step, 1.0);
    (value_parameter.set)(&mut config, 5);
    assert_eq!((value_parameter.get)(&config), 5);

    let mut labels = Labels(Vec::new());
    ExampleConfig::PARAMETERS.visit(&mut labels);

    assert_eq!(labels.0, ["Example value", "Weight"]);
}

#[test]
fn generated_names_are_inferred_from_config_structs() {
    assert_gui_accessor::<GUIConfig>();
    let _: GUIParameters<GUIConfig> = GUIConfig::PARAMETERS;
    let _: GUIParameterSpecs = GUIConfig::PARAMETER_SPECS;

    assert_static_bpm_detection_accessor::<StaticBPMDetectionConfig>();
    let _: StaticBPMDetectionParameters<StaticBPMDetectionConfig> = StaticBPMDetectionConfig::PARAMETERS;
    let _: StaticBPMDetectionParameterSpecs = StaticBPMDetectionConfig::PARAMETER_SPECS;

    assert_runtime_settings_accessor::<RuntimeSettings>();
    let _: RuntimeSettingsParameters<RuntimeSettings> = RuntimeSettings::PARAMETERS;
    let _: RuntimeSettingsParameterSpecs = RuntimeSettings::PARAMETER_SPECS;

    assert_f32_eq(GUIConfig::PARAMETER_SPECS.interpolation().default, 0.5);
    assert_f32_eq(StaticBPMDetectionConfig::PARAMETER_SPECS.bpm_center().default, 90.0);
    assert_f32_eq(RuntimeSettings::PARAMETER_SPECS.gain().default, 0.8);
}

#[test]
fn generated_accessor_trait_exposes_group_specific_parameter_entry_points() {
    fn visit_example_parameters<Config: ExampleConfigAccessor>() -> Vec<&'static str>
    where
        Labels: ExampleParameterVisitor<Config>,
    {
        let mut labels = Labels(Vec::new());
        Config::example_parameters().visit(&mut labels);
        labels.0
    }

    assert_eq!(visit_example_parameters::<ExampleConfig>(), ["Example value", "Weight"]);
}

#[test]
fn generated_validation_checks_parameter_ranges_in_visit_order() {
    let mut config = ExampleConfig::default();

    assert_eq!(config.validate(), Ok(()));

    config.value = 6;

    assert_eq!(config.validate(), Err("Example value value 6 is outside declared range 1..=5".to_string()));
}

#[test]
fn unannotated_nested_fields_are_defaulted_validated_and_not_visited() {
    let mut config = NestedExampleConfig::default();

    assert_eq!(config.value, 3);
    assert!(!config.nested.valid);
    assert_eq!(config.validate(), Err("nested config is invalid".to_string()));

    config.nested.valid = true;

    assert_eq!(config.validate(), Ok(()));

    let mut labels = NestedLabels(Vec::new());
    NestedExampleConfig::PARAMETERS.visit(&mut labels);

    assert_eq!(labels.0, ["Example value"]);
}

struct Labels(Vec<&'static str>);

impl ExampleParameterVisitor<ExampleConfig> for Labels {
    fn parameter<ValueType: Asf64>(&mut self, parameter: Parameter<ExampleConfig, ValueType>) {
        self.0.push(parameter.spec.label);
    }
}

struct NestedLabels(Vec<&'static str>);

impl NestedExampleParameterVisitor<NestedExampleConfig> for NestedLabels {
    fn parameter<ValueType: Asf64>(&mut self, parameter: Parameter<NestedExampleConfig, ValueType>) {
        self.0.push(parameter.spec.label);
    }
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}

fn assert_f64_eq(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < f64::EPSILON);
}

fn assert_parameter_spec<ValueType>(_: &ParameterSpec<ValueType>) {}

fn assert_gui_accessor<Config: GUIConfigAccessor>() {}

fn assert_static_bpm_detection_accessor<Config: StaticBPMDetectionConfigAccessor>() {}

fn assert_runtime_settings_accessor<Config: RuntimeSettingsAccessor>() {}
