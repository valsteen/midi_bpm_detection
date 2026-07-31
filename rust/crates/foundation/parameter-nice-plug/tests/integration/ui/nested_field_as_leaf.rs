use nice_plug::params::FloatParam;
use parameter::parameter_group;
use parameter_nice_plug::nice_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq)]
struct ExampleChildConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    gain: f32,
}

#[parameter_group]
#[derive(Clone, PartialEq)]
struct ExampleParentConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    gain: f32,
    child: ExampleChildConfig,
}

#[nice_plugin_parameter_group(config = ExampleParentConfig, group = "example")]
struct ExampleParentParams {
    gain: FloatParam,
    child: FloatParam,
}

fn main() {}
