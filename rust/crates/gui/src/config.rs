use std::{fmt::Debug, marker::PhantomData, time::Duration};

use parameter::Parameter;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GUIConfig {
    pub interpolation_duration: Duration,

    // since we only keep interpolating value, the interpolation will seem to 'accelerate' towards the end
    // of the interval a factor of 1 will preserve this behaviour. factor < 1 will make the movement 'slower',
    // factor > 1 will accelerate it
    pub interpolation_curve: f32,
}

pub trait GUIConfigAccessor {
    fn interpolation_duration(&self) -> Duration;
    fn interpolation_curve(&self) -> f32;

    fn set_interpolation_duration(&mut self, val: Duration);
    fn set_interpolation_curve(&mut self, val: f32);
}

impl GUIConfigAccessor for () {
    fn interpolation_duration(&self) -> Duration {
        unimplemented!()
    }

    fn interpolation_curve(&self) -> f32 {
        unimplemented!()
    }

    fn set_interpolation_duration(&mut self, _: Duration) {
        unimplemented!()
    }

    fn set_interpolation_curve(&mut self, _: f32) {
        unimplemented!()
    }
}

pub type DefaultGUIParameters = GUIParameters<()>;

impl Default for GUIConfig {
    fn default() -> Self {
        Self {
            interpolation_duration: DefaultGUIParameters::INTERPOLATION_DURATION.default,
            interpolation_curve: DefaultGUIParameters::INTERPOLATION_CURVE.default,
        }
    }
}

pub struct GUIParameters<Config> {
    phantom: PhantomData<Config>,
}

impl<Config: GUIConfigAccessor> GUIParameters<Config> {
    pub const INTERPOLATION_CURVE: Parameter<Config, f32> = Parameter::new(
        "Interpolation curve",
        None,
        0.1..=2.0,
        0.0,
        false,
        0.7,
        Config::interpolation_curve,
        Config::set_interpolation_curve,
    );
    pub const INTERPOLATION_DURATION: Parameter<Config, Duration> = Parameter::new(
        "Interpolation duration",
        Some("s"),
        0.050..=1.0,
        0.0,
        false,
        Duration::from_millis(500),
        Config::interpolation_duration,
        Config::set_interpolation_duration,
    );
}
