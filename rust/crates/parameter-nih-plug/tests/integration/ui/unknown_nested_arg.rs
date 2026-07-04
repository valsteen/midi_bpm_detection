use parameter::parameter_group;
use parameter_nih_plug::nih_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    pub gain: f32,
}

#[nih_plugin_parameter_group(config = ExampleConfig, group = "example")]
pub struct ExampleParams {
    #[nih_plugin_nested(label = "Child")]
    pub child: ChildParams,
}

fn main() {}
