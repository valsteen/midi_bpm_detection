use std::time::Duration;

use parameter_macros::parameter_group;
use serde::{Deserialize, Serialize};

#[parameter_group]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GUIConfig {
    #[parameter(label = "Interpolation duration", unit = "s", range = 0.050..=1.0, default = Duration::from_millis(500))]
    pub interpolation_duration: Duration,

    // since we only keep interpolating value, the interpolation will seem to 'accelerate' towards the end
    // of the interval a factor of 1 will preserve this behaviour. factor < 1 will make the movement 'slower',
    // factor > 1 will accelerate it
    #[parameter(label = "Interpolation curve", range = 0.1..=2.0, default = 0.7)]
    pub interpolation_curve: f32,
}

#[cfg(test)]
mod parameter_inventory_tests {
    use parameter::{Parameter, ParameterSpec};

    use super::*;

    struct GUIParameterLabels(Vec<&'static str>);

    impl GUIParameterVisitor<GUIConfig> for GUIParameterLabels {
        fn interpolation_duration(&mut self, parameter: Parameter<GUIConfig, Duration>) {
            self.0.push(parameter.label);
        }

        fn interpolation_curve(&mut self, parameter: Parameter<GUIConfig, f32>) {
            self.0.push(parameter.label);
        }
    }

    #[test]
    fn gui_parameter_specs_and_visitor_preserve_inventory() {
        let parameter_specs = GUIConfig::PARAMETER_SPECS;

        assert_parameter_spec(&parameter_specs.interpolation_duration());
        assert_parameter_spec(&parameter_specs.interpolation_curve());

        let mut labels = GUIParameterLabels(Vec::new());

        GUIConfig::PARAMETERS.visit(&mut labels);

        assert_eq!(labels.0, ["Interpolation duration", "Interpolation curve"]);
    }

    fn assert_parameter_spec<ValueType>(_: &ParameterSpec<ValueType>) {}
}
