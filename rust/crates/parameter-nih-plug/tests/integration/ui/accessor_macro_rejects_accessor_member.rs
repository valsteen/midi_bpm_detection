use parameter::parameter_group;
use parameter_nih_plug::nih_plugin_parameter_group;

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=1.0, default = 0.5)]
    pub gain: f32,
}

#[nih_plugin_parameter_group(config = ExampleConfig, group = "example", accessor_macro = example_accessors)]
pub struct ExampleParams {
    pub gain: nih_plug::params::FloatParam,
}

example_accessors! {
    target = ExampleLiveConfig<'_>,
    accessor = ExampleConfigAccessor,
    config = self.config,
    params = self.params,
    param_setter = self.param_setter,
    after_set = self.after_set(),
}

pub struct ExampleLiveConfig<'a> {
    config: ExampleConfig,
    params: ExampleParams,
    param_setter: &'a nih_plug::prelude::ParamSetter<'a>,
}

impl ExampleLiveConfig<'_> {
    fn after_set(&mut self) {}
}

fn main() {}
