use bpm_detection_core::parameters::{
    DynamicBPMDetectionParameters, NormalDistributionParameters, StaticBPMDetectionParameters,
};
use eframe::{egui, egui::Ui};

use crate::{BPMDetectionConfig, add_slider::SlideAdder, app::BPMDetectionGUI, config::GUIParameters};

impl BPMDetectionGUI {
    pub(crate) fn settings_panel<Config: BPMDetectionConfig>(ui: &mut Ui, config: &mut Config) {
        egui::Grid::new("").num_columns(2).spacing([40.0, 4.0]).striped(true).show(ui, |ui| {
            let mut slide_adder = SlideAdder::new(ui, config);

            slide_adder.add(&GUIParameters::INTERPOLATION_DURATION);
            slide_adder.add(&GUIParameters::INTERPOLATION_CURVE);

            slide_adder.add(&StaticBPMDetectionParameters::BPM_CENTER);
            slide_adder.add(&StaticBPMDetectionParameters::BPM_RANGE);
            slide_adder.add(&StaticBPMDetectionParameters::SAMPLE_RATE);

            slide_adder.add(&NormalDistributionParameters::STD_DEV);
            slide_adder.add(&NormalDistributionParameters::RESOLUTION);
            slide_adder.add(&NormalDistributionParameters::CUTOFF);
            slide_adder.add(&NormalDistributionParameters::FACTOR);

            slide_adder.add(&DynamicBPMDetectionParameters::BEATS_LOOKBACK);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::NORMAL_DISTRIBUTION);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::TIME_DISTANCE);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::CURRENT_VELOCITY);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::VELOCITY_FROM);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::IN_RANGE);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::MULTIPLIER_FACTOR);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::SUBDIVISION_FACTOR);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::OCTAVE_DISTANCE);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::PITCH_DISTANCE);
            slide_adder.add_on_off(&DynamicBPMDetectionParameters::HIGH_TEMPO_BIAS);

            let mut send_tempo_enabled = config.get_send_tempo();
            if ui.toggle_value(&mut send_tempo_enabled, "Send tempo").changed() {
                config.set_send_tempo(send_tempo_enabled);
            }
        });
    }
}
