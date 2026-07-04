use parameter::{OnOff, parameter_group};
use parameter_nih_plug::nih_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq)]
pub struct ExampleConfig {
    #[parameter(label = "Weight", range = 0.0..=1.0, default = OnOff::On(0.5))]
    pub weight: OnOff<f32>,
}

#[nih_plugin_parameter_group(config = ExampleConfig, group = "example")]
pub struct ExampleParams {
    #[nih_plugin_parameter(adapter = "on_off_f32")]
    pub weight: nih_plug::params::FloatParam,
}

fn main() {}
