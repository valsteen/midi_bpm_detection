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
mod tests {
    use super::*;

    #[test]
    fn plugin_config_rejects_stale_dynamic_parameter_keys() {
        let config_with_stale_key = CONFIG.replace("high_tempo_bias_weight", "high_tempo_bias");

        let message = PluginConfig::from_toml(&config_with_stale_key).expect_err("stale key should be rejected");

        assert!(message.contains("unknown field"));
        assert!(message.contains("high_tempo_bias"));
    }

    #[test]
    fn plugin_config_rejects_parameter_values_outside_declared_ranges() {
        let config_with_out_of_range_value = CONFIG.replace("bpm_center = 100.0", "bpm_center = 1000.0");

        let message = PluginConfig::from_toml(&config_with_out_of_range_value)
            .expect_err("out-of-range value should be rejected");

        assert!(message.contains("BPM center"));
        assert!(message.contains("1000"));
        assert!(message.contains("1..=150"));
    }
}
