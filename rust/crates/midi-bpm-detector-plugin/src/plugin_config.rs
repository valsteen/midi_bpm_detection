use std::sync::atomic::Ordering;

use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use errors::error_backtrace;
use gui::GUIConfig;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sync::ArcAtomicBool;

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Default)]
pub struct SendTempoOutputState {
    enabled: ArcAtomicBool,
    host_param_update_requested: ArcAtomicBool,
}

impl SendTempoOutputState {
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self { enabled: ArcAtomicBool::new(enabled), host_param_update_requested: ArcAtomicBool::default() }
    }

    #[must_use]
    pub fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_from_host(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        self.host_param_update_requested.store(false, Ordering::Relaxed);
    }

    pub fn set_from_gui(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::SeqCst);
        self.host_param_update_requested.store(true, Ordering::SeqCst);
    }

    pub fn toggle_from_shortcut(&self) {
        let _ = self.enabled.fetch_xor(true, Ordering::Acquire);
        self.host_param_update_requested.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn take_host_param_update_request(&self) -> bool {
        self.host_param_update_requested.take(Ordering::Relaxed)
    }
}

impl Serialize for SendTempoOutputState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(self.enabled.load(Ordering::SeqCst))
    }
}

impl<'de> Deserialize<'de> for SendTempoOutputState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        bool::deserialize(deserializer).map(Self::new)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginConfig {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
    pub send_tempo: SendTempoOutputState,
}

impl PluginConfig {
    pub fn from_toml(config: &str) -> Result<Self, String> {
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

impl Default for PluginConfig {
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
#[path = "../tests/unit/plugin_config.rs"]
mod tests;
