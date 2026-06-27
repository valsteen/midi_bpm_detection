use parameter::{Asf64, Parameter, ParameterSpec};
use parameter_macros::parameter_group;

#[parameter_group(
    accessor = ExampleConfigAccessor,
    parameters = ExampleParameters,
    default_parameters = DefaultExampleParameters,
    visitor = ExampleParameterVisitor
)]
struct ExampleConfig {
    #[parameter(label = "Example value", range = 1.0..=5.0, step = 1.0, default = 3)]
    value: u8,
    #[parameter(label = "Weight", range = 0.0..=2.0, default = 1.25, const_name = WEIGHT, setter = set_weight)]
    weight: f32,
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

    let value_parameter = &DefaultExampleParameters::VALUE;
    let weight_parameter = &DefaultExampleParameters::WEIGHT;

    assert_parameter_spec(value_parameter);
    assert_parameter_spec(weight_parameter);
    assert_eq!(value_parameter.label, "Example value");
    assert_f64_eq(value_parameter.step, 1.0);
    assert!(!value_parameter.logarithmic);
    assert_f32_eq(weight_parameter.default, 1.25);

    let mut labels = Labels(Vec::new());
    ExampleParameters::visit(&mut labels);

    assert_eq!(labels.0, ["Example value", "Weight"]);
}

#[test]
fn generated_validation_checks_parameter_ranges_in_visit_order() {
    let mut config = ExampleConfig::default();

    assert_eq!(config.validate(), Ok(()));

    config.value = 6;

    assert_eq!(config.validate(), Err("Example value value 6 is outside declared range 1..=5".to_string()));
}

struct Labels(Vec<&'static str>);

impl ExampleParameterVisitor<ExampleConfig> for Labels {
    fn parameter<ValueType: Asf64>(&mut self, parameter: Parameter<ExampleConfig, ValueType>) {
        self.0.push(parameter.label);
    }
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON);
}

fn assert_f64_eq(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < f64::EPSILON);
}

fn assert_parameter_spec<ValueType>(_: &ParameterSpec<ValueType>) {}
