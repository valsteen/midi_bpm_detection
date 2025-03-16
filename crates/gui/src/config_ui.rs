use crate::{BPMDetectionParameters, app::BPMDetectionGUI};
use eframe::{egui, egui::Ui};

use crate::{add_slider::SlideAdder, config::GUIConfig};
use midi::{DynamicBPMDetectionParameters, NormalDistributionConfig, StaticBPMDetectionParameters};

impl<P: BPMDetectionParameters> BPMDetectionGUI<P> {
    pub(crate) fn settings_panel(&mut self, ui: &mut Ui) {
        egui::Grid::new("").num_columns(2).spacing([40.0, 4.0]).striped(true).show(ui, |ui| {
            let slide_adder_gui =
                SlideAdder::builder(ui, BPMDetectionParameters::apply_dynamic, &mut self.live_parameters);
            let mut gui_sliders = slide_adder_gui.for_config(BPMDetectionParameters::get_gui_config_mut);
            gui_sliders.add(&GUIConfig::INTERPOLATION_DURATION);
            gui_sliders.add(&GUIConfig::INTERPOLATION_CURVE);

            let sliders = SlideAdder::builder(ui, BPMDetectionParameters::apply_static, &mut self.live_parameters);
            let mut sliders_static_parameters =
                sliders.for_config(BPMDetectionParameters::get_static_bpm_detection_parameters_mut);
            let mut normal_distribution = sliders.for_config(BPMDetectionParameters::get_normal_distribution_mut);

            sliders_static_parameters.add(&StaticBPMDetectionParameters::BPM_CENTER);
            sliders_static_parameters.add(&StaticBPMDetectionParameters::BPM_RANGE);
            sliders_static_parameters.add(&StaticBPMDetectionParameters::SAMPLE_RATE);
            normal_distribution.add(&NormalDistributionConfig::STD_DEV);
            normal_distribution.add(&NormalDistributionConfig::RESOLUTION);
            normal_distribution.add(&NormalDistributionConfig::IMPRECISION);
            normal_distribution.add(&NormalDistributionConfig::FACTOR);

            let sliders_live =
                SlideAdder::builder(ui, BPMDetectionParameters::apply_dynamic, &mut self.live_parameters);
            let mut slider_bpm_detection_live =
                sliders_live.for_config(BPMDetectionParameters::get_dynamic_bpm_detection_parameters_mut);
            slider_bpm_detection_live.add(&DynamicBPMDetectionParameters::BEATS_LOOKBACK);

            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::NORMAL_DISTRIBUTION);

            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::TIME_DISTANCE);

            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::CURRENT_VELOCITY);
            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::VELOCITY_FROM);

            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::IN_RANGE);
            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::MULTIPLIER_FACTOR);
            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::SUBDIVISION_FACTOR);

            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::OCTAVE_DISTANCE);
            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::PITCH_DISTANCE);
            slider_bpm_detection_live.add_on_off(&DynamicBPMDetectionParameters::HIGH_TEMPO_BIAS);

            let mut send_tempo_enabled = self.live_parameters.get_send_tempo();
            if ui.toggle_value(&mut send_tempo_enabled, "Send tempo").changed() {
                self.live_parameters.set_send_tempo(send_tempo_enabled);
            }
        });
    }
}
