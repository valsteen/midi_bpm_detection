use std::{marker::PhantomData, time::Duration as StdDuration};

use chrono::Duration;
use derivative::Derivative;
use parameter::{Asf64, OnOff, Parameter};
use serde::{Deserialize, Serialize};

use crate::{DurationOps, NormalDistributionConfig};

#[derive(Clone, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq, Eq)]
#[serde(default)]
pub struct StaticBPMDetectionConfig {
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub bpm_center: f32,
    pub bpm_range: u16,
    // per second
    pub sample_rate: u16,
    pub normal_distribution: NormalDistributionConfig,
}

impl StaticBPMDetectionConfig {
    #[inline]
    #[must_use]
    pub fn index_to_bpm(&self, index: usize) -> f32 {
        beat_duration_to_bpm(self.index_to_duration(index))
    }

    #[inline]
    #[must_use]
    pub fn highest_bpm(&self) -> f32 {
        self.lowest_bpm() + Into::<f32>::into(self.bpm_range)
    }

    #[must_use]
    pub fn lowest_bpm(&self) -> f32 {
        (self.bpm_center - Into::<f32>::into(self.bpm_range / 2)).max(1.0)
    }
}
pub trait StaticBPMDetectionConfigAccessor {
    fn bpm_center(&self) -> f32;
    fn bpm_range(&self) -> u16;
    fn sample_rate(&self) -> u16;
    fn index_to_bpm(&self, index: usize) -> f32;
    fn highest_bpm(&self) -> f32;
    fn lowest_bpm(&self) -> f32;

    fn set_bpm_center(&mut self, val: f32);
    fn set_bpm_range(&mut self, val: u16);
    fn set_sample_rate(&mut self, val: u16);
}

impl StaticBPMDetectionConfigAccessor for () {
    fn bpm_center(&self) -> f32 {
        unimplemented!()
    }

    fn bpm_range(&self) -> u16 {
        unimplemented!()
    }

    fn sample_rate(&self) -> u16 {
        unimplemented!()
    }

    fn index_to_bpm(&self, _: usize) -> f32 {
        unimplemented!()
    }

    fn highest_bpm(&self) -> f32 {
        unimplemented!()
    }

    fn lowest_bpm(&self) -> f32 {
        unimplemented!()
    }

    fn set_bpm_center(&mut self, _: f32) {
        unimplemented!()
    }

    fn set_bpm_range(&mut self, _: u16) {
        unimplemented!()
    }

    fn set_sample_rate(&mut self, _: u16) {
        unimplemented!()
    }
}

pub type DefaultStaticBPMDetectionParameters = StaticBPMDetectionParameters<()>;

impl Default for StaticBPMDetectionConfig {
    fn default() -> Self {
        Self {
            bpm_range: DefaultStaticBPMDetectionParameters::BPM_RANGE.default,
            bpm_center: DefaultStaticBPMDetectionParameters::BPM_CENTER.default,
            sample_rate: DefaultStaticBPMDetectionParameters::SAMPLE_RATE.default,
            normal_distribution: NormalDistributionConfig::default(),
        }
    }
}

pub struct StaticBPMDetectionParameters<Config: StaticBPMDetectionConfigAccessor> {
    phantom: PhantomData<Config>,
}

impl<Config: StaticBPMDetectionConfigAccessor> StaticBPMDetectionParameters<Config> {
    pub const BPM_CENTER: Parameter<Config, f32> =
        Parameter::new("BPM center", None, 1.0..=150.0, 0.01, false, 90.0, Config::bpm_center, Config::set_bpm_center);
    pub const BPM_RANGE: Parameter<Config, u16> =
        Parameter::new("BPM range", None, 1.0..=100.0, 1.0, false, 40, Config::bpm_range, Config::set_bpm_range);
    pub const SAMPLE_RATE: Parameter<Config, u16> = Parameter::new(
        "BPM sample rate",
        Some("samples/second"),
        1.0..=1_0000.,
        1.0,
        true,
        450,
        Config::sample_rate,
        Config::set_sample_rate,
    );
}

#[derive(Clone, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq, Eq)]
#[serde(default)]
pub struct DynamicBPMDetectionConfig {
    pub beats_lookback: u8,
    pub velocity_current_note_weight: OnOff<f32>,
    pub velocity_note_from_weight: OnOff<f32>,
    pub time_distance_weight: OnOff<f32>,
    pub octave_distance_weight: OnOff<f32>,
    pub pitch_distance_weight: OnOff<f32>,
    pub multiplier_weight: OnOff<f32>,
    pub subdivision_weight: OnOff<f32>,
    pub in_beat_range_weight: OnOff<f32>,
    pub normal_distribution_weight: OnOff<f32>,
    pub high_tempo_bias: OnOff<f32>,
}

pub trait DynamicBPMDetectionConfigAccessor {
    fn beats_lookback(&self) -> u8;
    fn velocity_current_note_weight(&self) -> OnOff<f32>;
    fn velocity_note_from_weight(&self) -> OnOff<f32>;
    fn time_distance_weight(&self) -> OnOff<f32>;
    fn octave_distance_weight(&self) -> OnOff<f32>;
    fn pitch_distance_weight(&self) -> OnOff<f32>;
    fn multiplier_weight(&self) -> OnOff<f32>;
    fn subdivision_weight(&self) -> OnOff<f32>;
    fn in_beat_range_weight(&self) -> OnOff<f32>;
    fn normal_distribution_weight(&self) -> OnOff<f32>;
    fn high_tempo_bias(&self) -> OnOff<f32>;
    fn set_beats_lookback(&mut self, val: u8);

    fn set_velocity_current_note_weight(&mut self, val: OnOff<f32>);
    fn set_velocity_note_from_weight(&mut self, val: OnOff<f32>);
    fn set_time_distance_weight(&mut self, val: OnOff<f32>);
    fn set_octave_distance_weight(&mut self, val: OnOff<f32>);
    fn set_pitch_distance_weight(&mut self, val: OnOff<f32>);
    fn set_multiplier_weight(&mut self, val: OnOff<f32>);
    fn set_subdivision_weight(&mut self, val: OnOff<f32>);
    fn set_in_beat_range_weight(&mut self, val: OnOff<f32>);
    fn set_normal_distribution_weight(&mut self, val: OnOff<f32>);
    fn set_high_tempo_bias(&mut self, val: OnOff<f32>);
}

impl DynamicBPMDetectionConfigAccessor for () {
    fn beats_lookback(&self) -> u8 {
        unimplemented!()
    }

    fn velocity_current_note_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn velocity_note_from_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn time_distance_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn octave_distance_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn pitch_distance_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn multiplier_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn subdivision_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn in_beat_range_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn normal_distribution_weight(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn high_tempo_bias(&self) -> OnOff<f32> {
        unimplemented!()
    }

    fn set_beats_lookback(&mut self, _: u8) {
        unimplemented!()
    }

    fn set_velocity_current_note_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_velocity_note_from_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_time_distance_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_octave_distance_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_pitch_distance_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_multiplier_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_subdivision_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_in_beat_range_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_normal_distribution_weight(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }

    fn set_high_tempo_bias(&mut self, _: OnOff<f32>) {
        unimplemented!()
    }
}

pub type DefaultDynamicBPMDetectionParameters = DynamicBPMDetectionParameters<()>;

impl Default for DynamicBPMDetectionConfig {
    fn default() -> Self {
        Self {
            beats_lookback: 8,
            velocity_current_note_weight: DefaultDynamicBPMDetectionParameters::CURRENT_VELOCITY.default,
            velocity_note_from_weight: DefaultDynamicBPMDetectionParameters::VELOCITY_FROM.default,
            time_distance_weight: DefaultDynamicBPMDetectionParameters::TIME_DISTANCE.default,
            octave_distance_weight: DefaultDynamicBPMDetectionParameters::OCTAVE_DISTANCE.default,
            pitch_distance_weight: DefaultDynamicBPMDetectionParameters::PITCH_DISTANCE.default,
            multiplier_weight: DefaultDynamicBPMDetectionParameters::MULTIPLIER_FACTOR.default,
            subdivision_weight: DefaultDynamicBPMDetectionParameters::SUBDIVISION_FACTOR.default,
            in_beat_range_weight: DefaultDynamicBPMDetectionParameters::IN_RANGE.default,
            normal_distribution_weight: DefaultDynamicBPMDetectionParameters::NORMAL_DISTRIBUTION.default,
            high_tempo_bias: DefaultDynamicBPMDetectionParameters::HIGH_TEMPO_BIAS.default,
        }
    }
}

pub struct DynamicBPMDetectionParameters<Config: DynamicBPMDetectionConfigAccessor> {
    phantom: PhantomData<Config>,
}

impl<Config: DynamicBPMDetectionConfigAccessor> DynamicBPMDetectionParameters<Config> {
    pub const BEATS_LOOKBACK: Parameter<Config, u8> = Parameter::new(
        "Beats Lookback",
        None,
        2.0..=32.0,
        1.0,
        false,
        8,
        Config::beats_lookback,
        Config::set_beats_lookback,
    );
    pub const CURRENT_VELOCITY: Parameter<Config, OnOff<f32>> = Parameter::new(
        "Note velocity",
        None,
        0.5..=10.0,
        0.0,
        true,
        OnOff::On(0.7),
        Config::velocity_current_note_weight,
        Config::set_velocity_current_note_weight,
    );
    pub const HIGH_TEMPO_BIAS: Parameter<Config, OnOff<f32>> = Parameter::new(
        "High tempo bias",
        None,
        0.0..=3.0,
        0.0,
        false,
        OnOff::On(0.2),
        Config::high_tempo_bias,
        Config::set_high_tempo_bias,
    );
    pub const IN_RANGE: Parameter<Config, OnOff<f32>> = Parameter::new(
        "In beat range",
        None,
        0.0..=3.0,
        0.0,
        false,
        OnOff::On(0.75),
        Config::in_beat_range_weight,
        Config::set_in_beat_range_weight,
    );
    pub const MULTIPLIER_FACTOR: Parameter<Config, OnOff<f32>> = Parameter::new(
        "Multiplier",
        None,
        0.0..=3.0,
        0.0,
        false,
        OnOff::On(0.66),
        Config::multiplier_weight,
        Config::set_multiplier_weight,
    );
    pub const NORMAL_DISTRIBUTION: Parameter<Config, OnOff<f32>> = Parameter::new(
        "Normal distribution",
        None,
        0.0..=1.0,
        0.0,
        false,
        OnOff::On(1.0),
        Config::normal_distribution_weight,
        Config::set_normal_distribution_weight,
    );
    pub const OCTAVE_DISTANCE: Parameter<Config, OnOff<f32>> = Parameter::new(
        "Octave distance",
        None,
        0.5..=20.0,
        0.0,
        true,
        OnOff::On(0.6),
        Config::octave_distance_weight,
        Config::set_octave_distance_weight,
    );
    pub const PITCH_DISTANCE: Parameter<Config, OnOff<f32>> = Parameter::new(
        "Pitch distance",
        None,
        0.5..=20.0,
        0.0,
        true,
        OnOff::On(0.6),
        Config::pitch_distance_weight,
        Config::set_pitch_distance_weight,
    );
    pub const SUBDIVISION_FACTOR: Parameter<Config, OnOff<f32>> = Parameter::new(
        "Subdivision",
        None,
        0.5..=6.0,
        0.0,
        true,
        OnOff::On(0.7),
        Config::subdivision_weight,
        Config::set_subdivision_weight,
    );
    pub const TIME_DISTANCE: Parameter<Config, OnOff<f32>> = Parameter::new(
        "Time distance",
        None,
        0.5..=6.0,
        0.0,
        true,
        OnOff::On(0.7),
        Config::time_distance_weight,
        Config::set_time_distance_weight,
    );
    pub const VELOCITY_FROM: Parameter<Config, OnOff<f32>> = Parameter::new(
        "From note velocity",
        None,
        0.5..=10.0,
        0.0,
        true,
        OnOff::On(0.7),
        Config::velocity_note_from_weight,
        Config::set_velocity_note_from_weight,
    );
}

impl StaticBPMDetectionConfig {
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
    let lowest_bpm = (DefaultStaticBPMDetectionParameters::BPM_CENTER.range.start()
        - (DefaultStaticBPMDetectionParameters::BPM_RANGE.range.end() / 2.0))
        .max(1.0);
    let highest_bpm = (DefaultStaticBPMDetectionParameters::BPM_CENTER.range.end()
        + (DefaultStaticBPMDetectionParameters::BPM_RANGE.range.end() / 2.0))
        .max(1.0);

    bpm_to_beat_duration(lowest_bpm)
        .checked_sub(&bpm_to_beat_duration(highest_bpm))
        .map(|duration| duration_to_sample(48000, duration))
        .expect("programming error, bpm_lower_bound > bpm_upper_bound")
}
