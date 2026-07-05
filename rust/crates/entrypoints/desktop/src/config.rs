use std::{fs::write, path::PathBuf};

use bpm_detection_config::Settings;
use bpm_detection_midi::MidiServiceConfig;
use build::{get_config_dir, get_data_dir};
use config::ConfigError;
use errors::{Report, Result, TypedResult};
use log::{error, info};
use serde::{Deserialize, Serialize};

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AppConfig {
    pub data_dir: PathBuf,
    pub config_dir: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DesktopConfig {
    #[serde(default, flatten)]
    #[serde(skip_serializing)]
    pub app: AppConfig,
    #[serde(default, flatten)]
    pub bpm_detection: Settings,
    #[serde(rename = "MIDI")]
    pub midi: MidiServiceConfig,
}

impl DesktopConfig {
    /// Load the desktop configuration from the built-in defaults and the optional user config file.
    ///
    /// # Errors
    ///
    /// Returns an error if default values cannot be registered, a config source cannot be read, or the combined config
    /// cannot be deserialized into the desktop config shape.
    pub fn new() -> TypedResult<Self, ConfigError> {
        let data_dir = get_data_dir();
        let config_dir = get_config_dir();
        let data_dir_value = data_dir.to_string_lossy().to_string();
        let config_dir_value = config_dir.to_string_lossy().to_string();
        let builder = config::Config::builder()
            .set_default("_data_dir", data_dir_value)?
            .set_default("_config_dir", config_dir_value)?
            .add_source(config::File::from_str(CONFIG, config::FileFormat::Toml))
            .add_source(
                config::File::from(config_dir.join("config.toml")).format(config::FileFormat::Toml).required(false),
            );

        let config: Self = builder.build()?.try_deserialize()?;
        config.validate().map_err(ConfigError::Message)?;

        Ok(config)
    }

    fn validate(&self) -> std::result::Result<(), String> {
        self.bpm_detection.validate()
    }

    /// Persist the user-editable desktop configuration to the configured config directory.
    ///
    /// # Errors
    ///
    /// Returns an error if the config cannot be serialized or written to the configured config directory.
    pub fn save(&self) -> Result<()> {
        let serialized = match toml::to_string_pretty(self) {
            Ok(serialized) => serialized,
            Err(e) => {
                error!("Serialization error: {e:?}");
                return Err(Report::new(e));
            }
        };

        let config_path = get_config_dir().join("config.toml");
        info!("configuration saved at {}", config_path.display());
        Ok(write(config_path, serialized)?)
    }
}

#[cfg(test)]
#[path = "../tests/unit/config.rs"]
mod tests;
