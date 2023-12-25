use crate::{action::Action, config::Config};
use errors::{LogErrorWithExt, Report, Result};
use gui::{BPMDetectionParameters, GUIConfig};
use midi::{DynamicBPMDetectionParameters, StaticBPMDetectionParameters};
use std::sync::atomic::Ordering;
use tokio::sync::mpsc::UnboundedSender;

pub struct LiveParameters {
    pub action_tx: UnboundedSender<Action>,
    pub config: Config,
}

impl BPMDetectionParameters for LiveParameters {
    type Error = Report;

    fn get_dynamic_bpm_detection_parameters(&self) -> &DynamicBPMDetectionParameters {
        &self.config.dynamic_bpm_detection_parameters
    }

    fn get_dynamic_bpm_detection_parameters_mut(&mut self) -> &mut DynamicBPMDetectionParameters {
        &mut self.config.dynamic_bpm_detection_parameters
    }

    fn get_static_bpm_detection_parameters(&self) -> &StaticBPMDetectionParameters {
        &self.config.static_bpm_detection_parameters
    }

    fn get_static_bpm_detection_parameters_mut(&mut self) -> &mut StaticBPMDetectionParameters {
        &mut self.config.static_bpm_detection_parameters
    }

    fn get_gui_config(&self) -> &GUIConfig {
        &self.config.gui
    }

    fn get_gui_config_mut(&mut self) -> &mut GUIConfig {
        &mut self.config.gui
    }

    fn get_send_tempo(&self) -> bool {
        self.config.midi.send_tempo.load(Ordering::Relaxed)
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.config.midi.send_tempo.store(enabled, Ordering::Relaxed);
    }

    fn apply_static(&mut self) -> Result<()> {
        Ok(self
            .action_tx
            .send(Action::StaticBPMDetectionConfig(self.config.static_bpm_detection_parameters.clone()))?)
    }

    fn apply_dynamic(&mut self) -> Result<()> {
        Ok(self
            .action_tx
            .send(Action::DynamicBPMDetectionConfig(self.config.dynamic_bpm_detection_parameters.clone()))?)
    }

    fn save(&mut self) {
        self.config.save().log_error_msg("Could not save configuration").ok();
    }
}
