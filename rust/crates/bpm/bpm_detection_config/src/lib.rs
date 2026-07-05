use std::time::Duration;

use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use parameter::parameter_group;
use serde::{Deserialize, Serialize};

#[parameter_group]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GUIConfig {
    #[parameter(label = "Interpolation duration", unit = "s", range = 0.050..=1.0, default = Duration::from_millis(500))]
    pub interpolation_duration: Duration,

    // since we only keep interpolating value, the interpolation will seem to 'accelerate' towards the end
    // of the interval a factor of 1 will preserve this behaviour. factor < 1 will make the movement 'slower',
    // factor > 1 will accelerate it
    #[parameter(label = "Interpolation curve", range = 0.1..=2.0, default = 0.7)]
    pub interpolation_curve: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Settings {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
}

impl Settings {
    /// Validate the shared serializable BPM detection settings.
    ///
    /// # Errors
    ///
    /// Returns a message when one of the generated parameter validators rejects a value.
    pub fn validate(&self) -> Result<(), String> {
        self.gui_config.validate()?;
        self.static_bpm_detection_config.validate()?;
        self.dynamic_bpm_detection_config.validate()?;

        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod parameter_inventory_tests;
