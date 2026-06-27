use bpm_detection_core::parameters::{
    DynamicBPMDetectionParameters, NormalDistributionParameters, StaticBPMDetectionParameters,
};
use eframe::{egui, egui::Ui};

use crate::{BPMDetectionConfig, add_slider::SlideAdder, app::BPMDetectionGUI, config::GUIParameters};

impl BPMDetectionGUI {
    pub(crate) fn settings_panel<Config: BPMDetectionConfig>(ui: &mut Ui, config: &mut Config) {
        egui::Grid::new("").num_columns(2).spacing([40.0, 4.0]).striped(true).show(ui, |ui| {
            config.desktop_controls(ui);

            let mut slide_adder = SlideAdder::new(ui, config);

            GUIParameters::visit(&mut slide_adder);

            StaticBPMDetectionParameters::visit(&mut slide_adder);

            NormalDistributionParameters::visit(&mut slide_adder);

            DynamicBPMDetectionParameters::visit(&mut slide_adder);

            let mut send_tempo_enabled = config.get_send_tempo();
            if ui.toggle_value(&mut send_tempo_enabled, "Send tempo").changed() {
                config.set_send_tempo(send_tempo_enabled);
            }
        });
    }
}
