#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]

use std::time::Duration as StdDuration;

use chrono::Duration;
use derivative::Derivative;
use parameter::{Asf64, parameter_group};
use parameter_on_off::OnOff;
use serde::{Deserialize, Serialize};

#[parameter_group]
#[derive(Clone, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct StaticBPMDetectionConfig {
    #[parameter(label = "BPM center", range = 1.0..=150.0, step = 0.01, default = 90.0)]
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub bpm_center: f32,
    #[parameter(label = "BPM range", range = 1.0..=100.0, step = 1.0, default = 40)]
    pub bpm_range: u16,
    #[parameter(
        label = "BPM sample rate",
        unit = "samples/second",
        range = 1.0..=1_0000.,
        step = 1.0,
        logarithmic = true,
        default = 450
    )]
    // per second
    pub sample_rate: u16,
    pub normal_distribution: NormalDistributionConfig,
}

pub trait StaticBPMDetectionComputed: StaticBPMDetectionConfigAccessor {
    #[inline]
    #[must_use]
    fn index_to_bpm(&self, index: usize) -> f32 {
        let duration = sample_to_duration(self.sample_rate(), index) + bpm_to_beat_duration(self.highest_bpm());
        beat_duration_to_bpm(duration)
    }

    #[inline]
    #[must_use]
    fn highest_bpm(&self) -> f32 {
        self.lowest_bpm() + Into::<f32>::into(self.bpm_range())
    }

    #[must_use]
    fn lowest_bpm(&self) -> f32 {
        (self.bpm_center() - Into::<f32>::into(self.bpm_range() / 2)).max(1.0)
    }
}

impl<Config: StaticBPMDetectionConfigAccessor> StaticBPMDetectionComputed for Config {}

impl StaticBPMDetectionConfig {
    #[inline]
    #[must_use]
    pub fn index_to_bpm(&self, index: usize) -> f32 {
        StaticBPMDetectionComputed::index_to_bpm(self, index)
    }

    #[inline]
    #[must_use]
    pub fn highest_bpm(&self) -> f32 {
        StaticBPMDetectionComputed::highest_bpm(self)
    }

    #[must_use]
    pub fn lowest_bpm(&self) -> f32 {
        StaticBPMDetectionComputed::lowest_bpm(self)
    }
}

#[parameter_group]
#[derive(Clone, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct DynamicBPMDetectionConfig {
    #[parameter(label = "Beats Lookback", range = 2.0..=32.0, step = 1.0, default = 8)]
    pub beats_lookback: u8,
    #[parameter(label = "Normal distribution", range = 0.0..=1.0, default = OnOff::On(1.0))]
    pub normal_distribution_weight: OnOff<f32>,
    #[parameter(label = "Time distance", range = 0.5..=6.0, logarithmic = true, default = OnOff::On(0.7))]
    pub time_distance_weight: OnOff<f32>,
    #[parameter(label = "Note velocity", range = 0.5..=10.0, logarithmic = true, default = OnOff::On(0.7))]
    pub velocity_current_note_weight: OnOff<f32>,
    #[parameter(label = "From note velocity", range = 0.5..=10.0, logarithmic = true, default = OnOff::On(0.7))]
    pub velocity_note_from_weight: OnOff<f32>,
    #[parameter(label = "In beat range", range = 0.0..=3.0, default = OnOff::On(0.75))]
    pub in_beat_range_weight: OnOff<f32>,
    #[parameter(label = "Multiplier", range = 0.0..=3.0, default = OnOff::On(0.66))]
    pub multiplier_weight: OnOff<f32>,
    #[parameter(label = "Subdivision", range = 0.5..=6.0, logarithmic = true, default = OnOff::On(0.7))]
    pub subdivision_weight: OnOff<f32>,
    #[parameter(label = "Octave distance", range = 0.5..=20.0, logarithmic = true, default = OnOff::On(0.6))]
    pub octave_distance_weight: OnOff<f32>,
    #[parameter(label = "Pitch distance", range = 0.5..=20.0, logarithmic = true, default = OnOff::On(0.6))]
    pub pitch_distance_weight: OnOff<f32>,
    #[parameter(label = "High tempo bias", range = 0.0..=3.0, default = OnOff::On(0.2))]
    pub high_tempo_bias_weight: OnOff<f32>,
}

impl StaticBPMDetectionConfig {
    #[must_use]
    pub fn buffer_size(&self) -> usize {
        bpm_to_beat_duration(self.lowest_bpm())
            .checked_sub(&bpm_to_beat_duration(self.highest_bpm()))
            .map(|duration| duration_to_sample(self.sample_rate, duration))
            .expect("programming error, bpm_lower_bound > bpm_upper_bound")
    }

    #[must_use]
    pub fn duration_to_sample(&self, duration: Duration) -> usize {
        duration_to_sample(self.sample_rate, duration)
    }
}

#[must_use]
pub fn duration_to_sample(sample_rate: u16, duration: Duration) -> usize {
    (duration.num_nanoseconds().unwrap() as f64 * Asf64::as_f64(&sample_rate) / 1_000_000_000.0).round() as usize
}

#[must_use]
#[inline]
pub fn sample_to_duration(sample_rate: u16, sample: usize) -> Duration {
    let duration_secs = sample as f64 / Asf64::as_f64(&sample_rate);
    let duration_nanos = (duration_secs * 1_000_000_000.0) as i64;
    Duration::nanoseconds(duration_nanos)
}

#[must_use]
#[inline]
pub fn bpm_to_beat_duration<U>(bpm: U) -> Duration
where
    U: DurationOps,
{
    Duration::from_std(U::div(StdDuration::from_mins(1), bpm)).unwrap()
}

#[must_use]
#[inline]
pub fn beat_duration_to_bpm(beat_duration: Duration) -> f32 {
    let nanos = beat_duration.num_nanoseconds().unwrap();
    60_000_000_000.0 / nanos as f32
}

#[must_use]
pub fn bpm_to_midi_clock_interval(bpm: f32) -> Duration {
    Duration::from_std(bpm_to_beat_duration(bpm).to_std().unwrap().div_f32(24.)).unwrap()
}

#[must_use]
pub fn max_histogram_data_buffer_size() -> usize {
    let parameter_specs = StaticBPMDetectionConfig::PARAMETER_SPECS;
    let lowest_bpm =
        (parameter_specs.bpm_center().range.start() - (parameter_specs.bpm_range().range.end() / 2.0)).max(1.0);
    let highest_bpm =
        (parameter_specs.bpm_center().range.end() + (parameter_specs.bpm_range().range.end() / 2.0)).max(1.0);

    bpm_to_beat_duration(lowest_bpm)
        .checked_sub(&bpm_to_beat_duration(highest_bpm))
        .map(|duration| duration_to_sample(48000, duration))
        .expect("programming error, bpm_lower_bound > bpm_upper_bound")
}

pub trait DurationOps {
    fn div(duration: StdDuration, divisor: Self) -> StdDuration;
}

impl DurationOps for f32 {
    fn div(duration: StdDuration, divisor: Self) -> StdDuration {
        duration.div_f32(divisor)
    }
}

impl DurationOps for f64 {
    fn div(duration: StdDuration, divisor: Self) -> StdDuration {
        duration.div_f64(divisor)
    }
}

#[parameter_group]
#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct NormalDistributionConfig {
    #[parameter(label = "Standard deviation", range = 4.0..=40.0, default = 24.0)]
    #[derivative(PartialEq(compare_with = "f64::eq"))]
    pub std_dev: f64,
    #[parameter(label = "Normal distribution resolution", unit = "ms", range = 0.01..=1000.0, logarithmic = true, default = 0.6)]
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub resolution: f32, // 1 means one index = 1 millisecond
    #[parameter(label = "Normal distribution cutoff", unit = "ms", range = 1.0..=2000.0, logarithmic = true, default = 100.0)]
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub cutoff: f32, // in millisecond
    #[parameter(label = "factor", range = 0.0..=50.0, default = 40.0)]
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub factor: f32,
}

#[parameter_group]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GUIConfig {
    #[parameter(label = "Interpolation duration", unit = "s", range = 0.050..=1.0, default = StdDuration::from_millis(500))]
    pub interpolation_duration: StdDuration,

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

pub trait SettingsOwner {
    fn bpm_detection_settings(&self) -> &Settings;

    fn bpm_detection_settings_mut(&mut self) -> &mut Settings;

    fn after_gui_config_set(&mut self) {}

    fn after_static_bpm_detection_config_set(&mut self) {}

    fn after_dynamic_bpm_detection_config_set(&mut self) {}
}

impl<Owner: SettingsOwner> GUIConfigOwner for Owner {
    fn gui_config(&self) -> &GUIConfig {
        &self.bpm_detection_settings().gui_config
    }

    fn gui_config_mut(&mut self) -> &mut GUIConfig {
        &mut self.bpm_detection_settings_mut().gui_config
    }

    fn after_gui_config_set(&mut self) {
        SettingsOwner::after_gui_config_set(self);
    }
}

impl<Owner: SettingsOwner> StaticBPMDetectionConfigOwner for Owner {
    fn static_bpm_detection_config(&self) -> &StaticBPMDetectionConfig {
        &self.bpm_detection_settings().static_bpm_detection_config
    }

    fn static_bpm_detection_config_mut(&mut self) -> &mut StaticBPMDetectionConfig {
        &mut self.bpm_detection_settings_mut().static_bpm_detection_config
    }

    fn after_static_bpm_detection_config_set(&mut self) {
        SettingsOwner::after_static_bpm_detection_config_set(self);
    }
}

impl<Owner: SettingsOwner> DynamicBPMDetectionConfigOwner for Owner {
    fn dynamic_bpm_detection_config(&self) -> &DynamicBPMDetectionConfig {
        &self.bpm_detection_settings().dynamic_bpm_detection_config
    }

    fn dynamic_bpm_detection_config_mut(&mut self) -> &mut DynamicBPMDetectionConfig {
        &mut self.bpm_detection_settings_mut().dynamic_bpm_detection_config
    }

    fn after_dynamic_bpm_detection_config_set(&mut self) {
        SettingsOwner::after_dynamic_bpm_detection_config_set(self);
    }
}

impl<Owner: SettingsOwner> NormalDistributionConfigOwner for Owner {
    fn normal_distribution_config(&self) -> &NormalDistributionConfig {
        &self.bpm_detection_settings().static_bpm_detection_config.normal_distribution
    }

    fn normal_distribution_config_mut(&mut self) -> &mut NormalDistributionConfig {
        &mut self.bpm_detection_settings_mut().static_bpm_detection_config.normal_distribution
    }

    fn after_normal_distribution_config_set(&mut self) {
        SettingsOwner::after_static_bpm_detection_config_set(self);
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod parameter_inventory_tests;
