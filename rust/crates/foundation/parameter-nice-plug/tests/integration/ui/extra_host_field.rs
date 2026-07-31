use nice_plug::params::{FloatParam, IntParam};
use parameter::parameter_group;
use parameter_nice_plug::nice_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq)]
struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    gain: f32,
}

#[nice_plugin_parameter_group(config = ExampleConfig, group = "example")]
struct ExampleParams {
    gain: FloatParam,
    count: IntParam,
}

fn main() {}
