use parameter::parameter_group;
use parameter_nih_plug::nih_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    pub gain: f32,
    #[parameter(label = "Steps", range = 1.0..=16.0, default = 4)]
    pub steps: u8,
}

#[nih_plugin_parameter_group(config = ExampleConfig, group = "example", accessor_macro = example_accessors)]
pub struct ExampleParams {
    pub gain: nih_plug::params::FloatParam,
    pub steps: nih_plug::params::IntParam,
}

example_accessors! {
    target = WrongLiveConfig,
    config = self.config,
    params = self.params,
    param_setter = self.param_setter,
    after_set = self.after_set(),
}

pub struct WrongLiveConfig;

fn main() {}
