use std::{fs::write, path::PathBuf};

use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use bpm_detection_midi::MidiServiceConfig;
use build::{get_config_dir, get_data_dir};
use config::ConfigError;
use errors::{Report, Result, TypedResult};
use gui::GUIConfig;
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
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    #[serde(rename = "MIDI")]
    pub midi: MidiServiceConfig,
    #[serde(default)]
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
    #[serde(default)]
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
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
        self.gui_config.validate()?;
        self.static_bpm_detection_config.validate()?;
        self.dynamic_bpm_detection_config.validate()?;

        Ok(())
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
mod tests {
    use std::sync::atomic::Ordering;

    use parameter::OnOff;

    use super::*;

    #[test]
    fn base_config_contains_desktop_defaults() {
        let config = config::Config::builder()
            .add_source(config::File::from_str(CONFIG, config::FileFormat::Toml))
            .build()
            .expect("base desktop config should build")
            .try_deserialize::<DesktopConfig>()
            .expect("base desktop config should deserialize");

        assert_eq!(config.midi.device_name, "Desktop");
        assert!(!config.midi.enable_midi_clock.load(Ordering::Relaxed));
        assert!(!config.midi.send_tempo.load(Ordering::Relaxed));
        assert!((config.static_bpm_detection_config.bpm_center - 90.0).abs() < f32::EPSILON);
        assert_eq!(config.static_bpm_detection_config.bpm_range, 40);
        assert_eq!(config.static_bpm_detection_config.sample_rate, 500);
        assert_eq!(config.dynamic_bpm_detection_config.velocity_current_note_weight, OnOff::Off(0.7));
        assert_eq!(config.dynamic_bpm_detection_config.multiplier_weight, OnOff::On(0.66));
        assert_eq!(config.gui_config.interpolation_duration.as_millis(), 730);
    }
}
