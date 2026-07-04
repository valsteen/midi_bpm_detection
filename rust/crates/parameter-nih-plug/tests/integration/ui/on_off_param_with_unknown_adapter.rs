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
    #[nih_plugin_parameter(adapter = "unknown_on_off")]
    pub weight: parameter_nih_plug::OnOffParam,
}

fn main() {}
