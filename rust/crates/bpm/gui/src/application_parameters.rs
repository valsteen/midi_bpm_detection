use bpm_detection_config::GUIConfigAccessor;
use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfigAccessor, NormalDistributionConfigAccessor, StaticBPMDetectionComputed,
    StaticBPMDetectionConfigAccessor,
};
use eframe::egui::Ui;

pub trait BPMDetectionConfig:
    NormalDistributionConfigAccessor
    + DynamicBPMDetectionConfigAccessor
    + StaticBPMDetectionComputed
    + StaticBPMDetectionConfigAccessor
    + GUIConfigAccessor
{
    fn get_send_tempo(&self) -> bool;
    fn set_send_tempo(&mut self, enabled: bool);
    fn save(&mut self) {}
    fn desktop_controls(&mut self, _ui: &mut Ui) {}
}
