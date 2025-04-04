use std::fmt::Debug;

use bpm_detection_core::{DynamicBPMDetectionParameters, NormalDistributionConfig, StaticBPMDetectionParameters};

use crate::config::GUIConfig;

pub trait BPMDetectionParameters {
    type Error: Debug;

    fn get_dynamic_bpm_detection_parameters(&self) -> &DynamicBPMDetectionParameters;
    fn get_dynamic_bpm_detection_parameters_mut(&mut self) -> &mut DynamicBPMDetectionParameters;
    fn get_static_bpm_detection_parameters(&self) -> &StaticBPMDetectionParameters;
    fn get_static_bpm_detection_parameters_mut(&mut self) -> &mut StaticBPMDetectionParameters;
    fn get_normal_distribution(&self) -> &NormalDistributionConfig {
        &self.get_static_bpm_detection_parameters().normal_distribution
    }
    fn get_normal_distribution_mut(&mut self) -> &mut NormalDistributionConfig {
        &mut self.get_static_bpm_detection_parameters_mut().normal_distribution
    }
    fn get_gui_config(&self) -> &GUIConfig;
    fn get_gui_config_mut(&mut self) -> &mut GUIConfig;
    fn get_send_tempo(&self) -> bool;
    fn set_send_tempo(&mut self, enabled: bool);
    fn apply_static(&mut self) -> Result<(), Self::Error>;
    fn apply_dynamic(&mut self) -> Result<(), Self::Error>;
    fn save(&mut self) {}
}
