#![cfg(target_arch = "wasm32")]
use derivative::Derivative;
use errors::{LogErrorWithExt, Report, error_backtrace};
use futures::channel::mpsc::Sender;
use gui::{BPMDetectionParameters, GUIConfig};
use midi::{
    DynamicBPMDetectionParameters, StaticBPMDetectionParameters, TimedTypedMidiMessage, midi_messages::MidiNoteOn,
};
use serde::{Deserialize, Serialize};

pub mod wasm;

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Derivative, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_parameters: DynamicBPMDetectionParameters,
    pub static_bpm_detection_parameters: StaticBPMDetectionParameters,
}

pub struct LiveConfig {
    config: Config,
    sender: Sender<QueueItem>,
}

impl LiveConfig {
    fn new(sender: Sender<QueueItem>) -> Self {
        Self { config: Config::default(), sender }
    }
}

enum QueueItem {
    StaticParameters(StaticBPMDetectionParameters),
    DynamicParameters(DynamicBPMDetectionParameters),
    Note(TimedTypedMidiMessage<MidiNoteOn>),
    DelayedDynamicUpdate,
    DelayedStaticUpdate,
}

impl BPMDetectionParameters for LiveConfig {
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
        &self.config.gui_config
    }

    fn get_gui_config_mut(&mut self) -> &mut GUIConfig {
        &mut self.config.gui_config
    }

    fn get_send_tempo(&self) -> bool {
        false
    }

    fn set_send_tempo(&mut self, _: bool) {}

    fn apply_static(&mut self) -> Result<(), Self::Error> {
        self.sender
            .try_send(QueueItem::StaticParameters(self.config.static_bpm_detection_parameters.clone()))
            .log_error_msg("channel full")
            .ok();
        Ok(())
    }

    fn apply_dynamic(&mut self) -> Result<(), Self::Error> {
        self.sender
            .try_send(QueueItem::DynamicParameters(self.config.dynamic_bpm_detection_parameters.clone()))
            .log_error_msg("channel full")
            .ok();
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        match Config::deserialize(toml::de::Deserializer::new(CONFIG)) {
            Ok(config) => config,
            Err(err) => {
                error_backtrace!("{err}");
                panic!("invalid built-in configuration");
            }
        }
    }
}

pub mod test {
    #![allow(forbidden_lint_groups)]
    #![allow(clippy::missing_panics_doc)]
    #[allow(clippy::module_name_repetitions)]
    use errors::error_backtrace;
    use parameter::OnOff;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub test: OnOff<f32>,
    }

    impl Default for Config {
        fn default() -> Self {
            match Config::deserialize(toml::de::Deserializer::new(CONFIG)) {
                Ok(config) => config,
                Err(err) => {
                    error_backtrace!("{err}");
                    panic!("invalid built-in configuration");
                }
            }
        }
    }

    const CONFIG: &str = "[test]
enabled = false
value = 1";

    #[test]
    pub fn test_config() {
        let config = Config::default();
        assert_eq!(config.test, OnOff::Off(1.0));
    }
}
