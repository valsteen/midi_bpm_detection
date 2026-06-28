use parameter::{Parameter, ParameterSpec};

use super::*;

struct GUIParameterLabels(Vec<&'static str>);

impl GUIParameterVisitor<GUIConfig> for GUIParameterLabels {
    fn interpolation_duration(&mut self, parameter: Parameter<GUIConfig, Duration>) {
        self.0.push(parameter.spec.label);
    }

    fn interpolation_curve(&mut self, parameter: Parameter<GUIConfig, f32>) {
        self.0.push(parameter.spec.label);
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
