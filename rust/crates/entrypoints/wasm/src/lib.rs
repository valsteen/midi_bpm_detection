#![cfg(target_arch = "wasm32")]

use bpm_detection_core::{
    TimedEvent,
    note_events::NoteOn,
    parameters::{
        DynamicBPMDetectionConfig, DynamicBPMDetectionConfigOwner, NormalDistributionConfig,
        NormalDistributionConfigOwner, StaticBPMDetectionConfig, StaticBPMDetectionConfigOwner,
    },
};
use derivative::Derivative;
use errors::{LogErrorWithExt, error_backtrace};
use futures::channel::mpsc::Sender;
use gui::{BPMDetectionConfig, GUIConfig, GUIConfigOwner};
use serde::{Deserialize, Serialize};

pub mod wasm;

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Derivative, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WASMConfig {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
}

impl WASMConfig {
    fn from_toml(config: &str) -> Result<Self, String> {
        let config =
            toml::de::Deserializer::parse(config).and_then(Self::deserialize).map_err(|err| err.to_string())?;
        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        self.gui_config.validate()?;
        self.static_bpm_detection_config.validate()?;
        self.dynamic_bpm_detection_config.validate()?;

        Ok(())
    }
}

enum QueueItem {
    StaticParameters(StaticBPMDetectionConfig),
    DynamicParameters(DynamicBPMDetectionConfig),
    Note(TimedEvent<NoteOn>),
    DelayedDynamicUpdate,
    DelayedStaticUpdate,
}

pub struct BaseConfig {
    config: WASMConfig,
    sender: Sender<QueueItem>,
}

impl BaseConfig {
    fn new(sender: Sender<QueueItem>) -> Self {
        Self { config: WASMConfig::default(), sender }
    }

    fn propagate_static_changes(&mut self) {
        self.sender
            .try_send(QueueItem::StaticParameters(self.config.static_bpm_detection_config.clone()))
            .log_error_msg("channel full")
            .ok();
    }

    fn propagate_dynamic_changes(&mut self) {
        self.sender
            .try_send(QueueItem::DynamicParameters(self.config.dynamic_bpm_detection_config.clone()))
            .log_error_msg("channel full")
            .ok();
    }
}

impl NormalDistributionConfigOwner for BaseConfig {
    fn normal_distribution_config(&self) -> &NormalDistributionConfig {
        &self.config.static_bpm_detection_config.normal_distribution
    }

    fn normal_distribution_config_mut(&mut self) -> &mut NormalDistributionConfig {
        &mut self.config.static_bpm_detection_config.normal_distribution
    }

    fn after_normal_distribution_config_set(&mut self) {
        self.propagate_static_changes();
    }
}

impl DynamicBPMDetectionConfigOwner for BaseConfig {
    fn dynamic_bpm_detection_config(&self) -> &DynamicBPMDetectionConfig {
        &self.config.dynamic_bpm_detection_config
    }

    fn dynamic_bpm_detection_config_mut(&mut self) -> &mut DynamicBPMDetectionConfig {
        &mut self.config.dynamic_bpm_detection_config
    }

    fn after_dynamic_bpm_detection_config_set(&mut self) {
        self.propagate_dynamic_changes();
    }
}

impl StaticBPMDetectionConfigOwner for BaseConfig {
    fn static_bpm_detection_config(&self) -> &StaticBPMDetectionConfig {
        &self.config.static_bpm_detection_config
    }

    fn static_bpm_detection_config_mut(&mut self) -> &mut StaticBPMDetectionConfig {
        &mut self.config.static_bpm_detection_config
    }

    fn after_static_bpm_detection_config_set(&mut self) {
        self.propagate_static_changes();
    }
}

impl GUIConfigOwner for BaseConfig {
    fn gui_config(&self) -> &GUIConfig {
        &self.config.gui_config
    }

    fn gui_config_mut(&mut self) -> &mut GUIConfig {
        &mut self.config.gui_config
    }
}

impl BPMDetectionConfig for BaseConfig {
    fn get_send_tempo(&self) -> bool {
        false
    }

    fn set_send_tempo(&mut self, _: bool) {}
}

impl Default for WASMConfig {
    fn default() -> Self {
        match Self::from_toml(CONFIG) {
            Ok(config) => config,
            Err(err) => {
                error_backtrace!("{err}");
                panic!("invalid built-in configuration");
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
