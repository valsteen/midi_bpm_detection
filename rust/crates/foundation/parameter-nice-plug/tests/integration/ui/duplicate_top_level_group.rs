use parameter::parameter_group;
use parameter_nice_plug::nice_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    pub gain: f32,
}

#[nice_plugin_parameter_group(config = ExampleConfig, group = "example", group = "again")]
pub struct ExampleParams {
    pub gain: nice_plug::params::FloatParam,
}

fn main() {}
