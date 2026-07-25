use parameter::parameter_group;
use parameter_nice_plug::nice_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleConfig {
    #[parameter(label = "Sample rate", range = 1.0..=1_000.0, default = 450)]
    pub sample_rate: u16,
}

#[nice_plugin_parameter_group(config = ExampleConfig, group = "example")]
pub struct ExampleParams {
    #[nice_plugin_parameter(adapter = "float_u16_logarithmic", adapter = "float_u16_logarithmic")]
    pub sample_rate: nice_plug::params::FloatParam,
}

fn main() {}
