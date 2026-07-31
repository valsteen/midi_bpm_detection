use nice_plug::params::FloatParam;
use parameter::parameter_group;
use parameter_nice_plug::nice_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq)]
struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    gain: f32,
    #[parameter(label = "Count", range = 0.0..=8.0, default = 4)]
    count: u8,
}

#[nice_plugin_parameter_group(config = ExampleConfig, group = "example")]
struct ExampleParams {
    gain: FloatParam,
}

fn main() {}
