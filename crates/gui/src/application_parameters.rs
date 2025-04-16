use bpm_detection_core::{
    NormalDistributionConfigAccessor,
    bpm::{DynamicBPMDetectionConfigAccessor, StaticBPMDetectionConfigAccessor},
};

use crate::config::GUIConfigAccessor;

pub trait BPMDetectionConfig:
    NormalDistributionConfigAccessor
    + DynamicBPMDetectionConfigAccessor
    + StaticBPMDetectionConfigAccessor
    + GUIConfigAccessor
{
    fn get_send_tempo(&self) -> bool;
    fn set_send_tempo(&mut self, enabled: bool);
    fn save(&mut self) {}
}
