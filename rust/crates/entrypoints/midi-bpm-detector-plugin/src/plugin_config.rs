use bpm_detection_config::Settings;
use errors::error_backtrace;
use serde::{Deserialize, Serialize};

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginConfig {
    #[serde(default, flatten)]
    pub bpm_detection: Settings,
    pub send_tempo: bool,
}

impl PluginConfig {
    pub fn from_toml(config: &str) -> Result<Self, String> {
        let config =
            toml::de::Deserializer::parse(config).and_then(Self::deserialize).map_err(|err| err.to_string())?;
        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        self.bpm_detection.validate()
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
