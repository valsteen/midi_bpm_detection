use std::{fmt::Debug, time::Duration};

use parameter::{Getters, MutGetters, Parameter};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Getters, MutGetters)]
#[serde(default)]
#[getset(get = "pub", get_mut = "pub")]
pub struct GUIConfig {
    pub interpolation_duration: Duration,

    // since we only keep interpolating value, the interpolation will seem to 'accelerate' towards the end
    // of the interval a factor of 1 will preserve this behaviour. factor < 1 will make the movement 'slower',
    // factor > 1 will accelerate it
    pub interpolation_curve: f32,
}

impl Default for GUIConfig {
    fn default() -> Self {
        Self {
            interpolation_duration: Self::INTERPOLATION_DURATION.default,
            interpolation_curve: Self::INTERPOLATION_CURVE.default,
        }
    }
}

impl GUIConfig {
    pub const INTERPOLATION_CURVE: Parameter<Self, f32> = Parameter::new(
        "Interpolation curve",
        None,
        0.1..=2.0,
        0.0,
        false,
        0.7,
        Self::interpolation_curve,
        Self::interpolation_curve_mut,
    );
    pub const INTERPOLATION_DURATION: Parameter<Self, Duration> = Parameter::new(
        "Interpolation duration",
        Some("s"),
        0.050..=1.0,
        0.0,
        false,
        Duration::from_millis(500),
        Self::interpolation_duration,
        Self::interpolation_duration_mut,
    );
}
