use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use errors::error_backtrace;
use gui::GUIConfig;
use serde::{Deserialize, Serialize};
use sync::ArcAtomicBool;

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginConfig {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
    pub send_tempo: ArcAtomicBool,
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
