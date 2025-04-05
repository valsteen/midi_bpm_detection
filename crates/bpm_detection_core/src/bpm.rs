use std::time::Duration as StdDuration;

use chrono::Duration;
use derivative::Derivative;
use parameter::{Asf64, Getters, MutGetters, OnOff, Parameter};
use serde::{Deserialize, Serialize};

use crate::{DurationOps, NormalDistributionConfig};

#[derive(Clone, Debug, Derivative, Serialize, Deserialize, Getters, MutGetters)]
#[derivative(PartialEq, Eq)]
#[getset(get = "pub", get_mut = "pub")]
#[serde(default)]
pub struct StaticBPMDetectionParameters {
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub bpm_center: f32,
    pub bpm_range: u16,
    // per second
    pub sample_rate: u16,
    pub normal_distribution: NormalDistributionConfig,
}

impl Default for StaticBPMDetectionParameters {
    fn default() -> Self {
        Self {
            bpm_range: Self::BPM_RANGE.default,
            bpm_center: Self::BPM_CENTER.default,
            sample_rate: Self::SAMPLE_RATE.default,
            normal_distribution: NormalDistributionConfig::default(),
        }
    }
}

impl StaticBPMDetectionParameters {
    pub const BPM_CENTER: Parameter<Self, f32> =
        Parameter::new("BPM center", None, 1.0..=150.0, 0.01, false, 90.0, Self::bpm_center, Self::bpm_center_mut);
    pub const BPM_RANGE: Parameter<Self, u16> =
        Parameter::new("BPM range", None, 1.0..=100.0, 1.0, false, 40, Self::bpm_range, Self::bpm_range_mut);
    pub const SAMPLE_RATE: Parameter<Self, u16> = Parameter::new(
        "BPM sample rate",
        Some("samples/second"),
        1.0..=1_0000.,
        1.0,
        true,
        450,
        Self::sample_rate,
        Self::sample_rate_mut,
    );
}

#[derive(Clone, Debug, Derivative, Serialize, Deserialize, Getters, MutGetters)]
#[derivative(PartialEq, Eq)]
#[getset(get = "pub", get_mut = "pub")]
#[serde(default)]
pub struct DynamicBPMDetectionParameters {
    pub beats_lookback: u8,
    pub velocity_current_note_weight: OnOff<f32>,
    pub velocity_note_from_weight: OnOff<f32>,
    pub age_weight: OnOff<f32>,
    pub octave_distance_weight: OnOff<f32>,
    pub pitch_distance_weight: OnOff<f32>,
    pub multiplier_weight: OnOff<f32>,
    pub subdivision_weight: OnOff<f32>,
    pub in_beat_range_weight: OnOff<f32>,
    pub normal_distribution_weight: OnOff<f32>,
    pub high_tempo_bias: OnOff<f32>,
}

impl Default for DynamicBPMDetectionParameters {
    fn default() -> Self {
        Self {
            beats_lookback: 8,
            velocity_current_note_weight: Self::CURRENT_VELOCITY.default,
            velocity_note_from_weight: Self::VELOCITY_FROM.default,
            age_weight: Self::TIME_DISTANCE.default,
            octave_distance_weight: Self::OCTAVE_DISTANCE.default,
            pitch_distance_weight: Self::PITCH_DISTANCE.default,
            multiplier_weight: Self::MULTIPLIER_FACTOR.default,
            subdivision_weight: Self::SUBDIVISION_FACTOR.default,
            in_beat_range_weight: Self::IN_RANGE.default,
            normal_distribution_weight: Self::NORMAL_DISTRIBUTION.default,
            high_tempo_bias: Self::HIGH_TEMPO_BIAS.default,
        }
    }
}

impl DynamicBPMDetectionParameters {
    pub const BEATS_LOOKBACK: Parameter<Self, u8> = Parameter::new(
        "Beats Lookback",
        None,
        2.0..=32.0,
        1.0,
        false,
        8,
        Self::beats_lookback,
        Self::beats_lookback_mut,
    );
    pub const CURRENT_VELOCITY: Parameter<Self, OnOff<f32>> = Parameter::new(
        "Note velocity",
        None,
        0.5..=10.0,
        0.0,
        true,
        OnOff::On(0.7),
        Self::velocity_current_note_weight,
        Self::velocity_current_note_weight_mut,
    );
    pub const HIGH_TEMPO_BIAS: Parameter<Self, OnOff<f32>> = Parameter::new(
        "High tempo bias",
        None,
        0.0..=3.0,
        0.0,
        false,
        OnOff::On(0.2),
        Self::high_tempo_bias,
        Self::high_tempo_bias_mut,
    );
    pub const IN_RANGE: Parameter<Self, OnOff<f32>> = Parameter::new(
        "In beat range",
        None,
        0.0..=3.0,
        0.0,
        false,
        OnOff::On(0.75),
        Self::in_beat_range_weight,
        Self::in_beat_range_weight_mut,
    );
    pub const MULTIPLIER_FACTOR: Parameter<Self, OnOff<f32>> = Parameter::new(
        "Multiplier",
        None,
        0.0..=3.0,
        0.0,
        false,
        OnOff::On(0.66),
        Self::multiplier_weight,
        Self::multiplier_weight_mut,
    );
    pub const NORMAL_DISTRIBUTION: Parameter<Self, OnOff<f32>> = Parameter::new(
        "Normal distribution",
        None,
        0.0..=1.0,
        0.0,
        false,
        OnOff::On(1.0),
        Self::normal_distribution_weight,
        Self::normal_distribution_weight_mut,
    );
    pub const OCTAVE_DISTANCE: Parameter<Self, OnOff<f32>> = Parameter::new(
        "Octave distance",
        None,
        0.5..=20.0,
        0.0,
        true,
        OnOff::On(0.6),
        Self::octave_distance_weight,
        Self::octave_distance_weight_mut,
    );
    pub const PITCH_DISTANCE: Parameter<Self, OnOff<f32>> = Parameter::new(
        "Pitch distance",
        None,
        0.5..=20.0,
        0.0,
        true,
        OnOff::On(0.6),
        Self::pitch_distance_weight,
        Self::pitch_distance_weight_mut,
    );
    pub const SUBDIVISION_FACTOR: Parameter<Self, OnOff<f32>> = Parameter::new(
        "Subdivision",
        None,
        0.5..=6.0,
        0.0,
        true,
        OnOff::On(0.7),
        Self::subdivision_weight,
        Self::subdivision_weight_mut,
    );
    pub const TIME_DISTANCE: Parameter<Self, OnOff<f32>> =
        Parameter::new("Age", None, 0.5..=6.0, 0.0, true, OnOff::On(0.7), Self::age_weight, Self::age_weight_mut);
    pub const VELOCITY_FROM: Parameter<Self, OnOff<f32>> = Parameter::new(
        "From note velocity",
        None,
        0.5..=10.0,
        0.0,
        true,
        OnOff::On(0.7),
        Self::velocity_note_from_weight,
        Self::velocity_note_from_weight_mut,
    );
}

impl StaticBPMDetectionParameters {
    #[must_use]
    #[inline]
    pub fn highest_bpm(&self) -> f32 {
        self.lowest_bpm() + Into::<f32>::into(self.bpm_range)
    }

    #[must_use]
    pub fn lowest_bpm(&self) -> f32 {
        (self.bpm_center - Into::<f32>::into(self.bpm_range / 2)).max(1.0)
    }

    pub(crate) fn duration_to_index(&self, duration: Duration, buffer_size: usize) -> Option<usize> {
        let index = self
            .duration_to_sample(duration)
            .checked_sub(self.duration_to_sample(bpm_to_beat_duration(self.highest_bpm())))?;
        (index < buffer_size).then_some(index)
    }

    #[must_use]
    pub fn buffer_size(&self) -> usize {
        bpm_to_beat_duration(self.lowest_bpm())
            .checked_sub(&bpm_to_beat_duration(self.highest_bpm()))
            .map(|duration| duration_to_sample(self.sample_rate, duration))
            .expect("programming error, bpm_lower_bound > bpm_upper_bound")
    }

    #[inline]
    pub(crate) fn index_to_duration(&self, index: usize) -> Duration {
        sample_to_duration(self.sample_rate, index) + bpm_to_beat_duration(self.highest_bpm())
    }

    #[must_use]
    #[inline]
    pub fn index_to_bpm(&self, index: usize) -> f32 {
        beat_duration_to_bpm(self.index_to_duration(index))
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
    Duration::from_std(U::div(StdDuration::from_secs(60), bpm)).unwrap()
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
    let lowest_bpm = (StaticBPMDetectionParameters::BPM_CENTER.range.start()
        - StaticBPMDetectionParameters::BPM_RANGE.range.end() / 2.0)
        .max(1.0);
    let highest_bpm = (StaticBPMDetectionParameters::BPM_CENTER.range.end()
        + StaticBPMDetectionParameters::BPM_RANGE.range.end() / 2.0)
        .max(1.0);

    bpm_to_beat_duration(lowest_bpm)
        .checked_sub(&bpm_to_beat_duration(highest_bpm))
        .map(|duration| duration_to_sample(48000, duration))
        .expect("programming error, bpm_lower_bound > bpm_upper_bound")
}
