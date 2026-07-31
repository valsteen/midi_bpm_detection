use bpm_detection_config::{
    DynamicBPMDetectionConfigAccessor, GUIConfigAccessor, NormalDistributionConfigAccessor,
    StaticBPMDetectionConfigAccessor,
};
use eframe::{egui, egui::Ui};

use crate::{BPMDetectionGUI, EditableSettings, GuiChanges, add_slider::SlideAdder};

impl BPMDetectionGUI {
    pub(crate) fn settings_panel(ui: &mut Ui, settings: &mut EditableSettings) -> GuiChanges {
        let mut changes = GuiChanges::default();
        egui::Grid::new("").num_columns(2).spacing([40.0, 4.0]).striped(true).show(ui, |ui| {
            let mut sliders = SlideAdder::new(ui, settings);

            EditableSettings::gui_parameters().visit(&mut sliders);
            changes.gui = sliders.take_changed();

            EditableSettings::static_bpm_detection_parameters().visit(&mut sliders);
            changes.static_detection = sliders.take_changed();

            EditableSettings::normal_distribution_parameters().visit(&mut sliders);
            changes.static_detection |= sliders.take_changed();

            EditableSettings::dynamic_bpm_detection_parameters().visit(&mut sliders);
            changes.dynamic_detection = sliders.take_changed();

            if let Some(send_tempo) = settings.send_tempo.as_mut() {
                changes.send_tempo = ui.toggle_value(send_tempo, "Send tempo").changed();
            }
        });
        changes
    }
}
