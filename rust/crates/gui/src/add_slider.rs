use std::fmt::Debug;

use bpm_detection_core::parameters::{DynamicBPMDetectionConfigAccessor, DynamicBPMDetectionParameterVisitor};
use eframe::{
    egui,
    egui::{Slider, SliderClamping},
};
use parameter::{Asf64, OnOff, Parameter};

pub fn add_slider<GuiValueType: Asf64, Config, ParameterValueType>(
    ui: &mut egui::Ui,
    enabled: bool,
    parameter: &Parameter<Config, ParameterValueType>,
    get_set_value: impl FnMut(Option<f64>) -> f64,
) {
    let mut slider = Slider::from_get_set(parameter.range.clone(), get_set_value)
        .logarithmic(parameter.logarithmic)
        .step_by(parameter.step)
        .clamping(SliderClamping::Edits);

    if let Some(unit) = parameter.unit.as_ref() {
        slider = slider.text(*unit);
    }

    ui.add_enabled(enabled, slider);
    ui.end_row();
}

pub fn add_slider_default<GuiValueType, Config, ParameterValueType>(
    ui: &mut egui::Ui,
    parameter: &Parameter<Config, ParameterValueType>,
    mut get_set_as_f64: impl FnMut(Option<GuiValueType>) -> GuiValueType,
) where
    GuiValueType: Debug + Asf64,
{
    ui.label(parameter.label);

    add_slider::<GuiValueType, Config, ParameterValueType>(ui, true, parameter, move |value_opt: Option<f64>| {
        get_set_as_f64(value_opt.map(GuiValueType::new_from)).as_f64()
    });
}

pub struct SlideAdder<'a, Config> {
    ui: &'a mut egui::Ui,
    config: &'a mut Config,
}

impl<'a, Config> SlideAdder<'a, Config> {
    pub fn new(ui: &'a mut egui::Ui, config: &'a mut Config) -> SlideAdder<'a, Config> {
        Self { ui, config }
    }
}

impl<Config> SlideAdder<'_, Config> {
    pub fn add<ParameterValueType>(&mut self, parameter: &Parameter<Config, ParameterValueType>)
    where
        ParameterValueType: Copy + Asf64 + Debug,
    {
        let Self { ui, config } = self;

        add_slider_default(ui, parameter, move |value_opt| match value_opt {
            None => (parameter.get)(config),
            Some(new_value) => {
                (parameter.set)(config, new_value);
                new_value
            }
        });
    }

    pub fn add_on_off<ValueType>(&mut self, parameter: &Parameter<Config, OnOff<ValueType>>)
    where
        ValueType: Asf64 + Copy,
    {
        #[cfg(feature = "on_off_widgets")]
        let (is_enabled, changed) = {
            let mut is_enabled = (parameter.get)(self.config).is_enabled();
            let on_off_checkbox = self.ui.checkbox(&mut is_enabled, parameter.label);
            (is_enabled, on_off_checkbox.changed())
        };

        #[cfg(not(feature = "on_off_widgets"))]
        let (is_enabled, changed) = {
            self.ui.label(parameter.label);
            (true, false)
        };

        let Self { ui, config } = self;

        add_slider::<ValueType, _, _>(ui, is_enabled, parameter, move |new_val_f64| {
            let mut current_value = (parameter.get)(config);

            if changed {
                assert!(new_val_f64.is_none(), "unexpected simultaneous change of value and checkbox state");
                current_value.set_enabled(is_enabled);
                (parameter.set)(config, current_value);
                return current_value.value().as_f64();
            }

            if let (true, Some(f64_val)) = (is_enabled, new_val_f64) {
                let new_value = ValueType::new_from(f64_val);
                (parameter.set)(config, OnOff::new(is_enabled, new_value));
                return new_value.as_f64();
            }

            current_value.value().as_f64()
        });
    }
}

impl<Config: DynamicBPMDetectionConfigAccessor> DynamicBPMDetectionParameterVisitor<Config> for SlideAdder<'_, Config> {
    fn beats_lookback(&mut self, parameter: Parameter<Config, u8>) {
        self.add(&parameter);
    }

    fn velocity_current_note_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn high_tempo_bias(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn in_beat_range_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn multiplier_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn normal_distribution_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn octave_distance_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn pitch_distance_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn subdivision_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn time_distance_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }

    fn velocity_note_from_weight(&mut self, parameter: Parameter<Config, OnOff<f32>>) {
        self.add_on_off(&parameter);
    }
}

#[cfg(test)]
mod tests {
    use bpm_detection_core::parameters::{DynamicBPMDetectionConfigAccessor, DynamicBPMDetectionParameterVisitor};

    use super::*;

    fn assert_dynamic_parameter_visitor<Config>()
    where
        Config: DynamicBPMDetectionConfigAccessor,
        for<'a> SlideAdder<'a, Config>: DynamicBPMDetectionParameterVisitor<Config>,
    {
    }

    #[test]
    fn slide_adder_can_render_dynamic_parameter_visitor() {
        assert_dynamic_parameter_visitor::<()>();
    }
}
