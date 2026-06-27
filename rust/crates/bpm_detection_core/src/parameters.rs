use std::{marker::PhantomData, time::Duration as StdDuration};

use chrono::Duration;
use derivative::Derivative;
use parameter::{Asf64, OnOff, Parameter};
use parameter_macros::parameter_group;
use serde::{Deserialize, Serialize};

use crate::DurationOps;

#[derive(Clone, Debug, Derivative, Serialize, Deserialize)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct StaticBPMDetectionConfig {
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub bpm_center: f32,
    pub bpm_range: u16,
    // per second
    pub sample_rate: u16,
    pub normal_distribution: NormalDistributionConfig,
}

impl StaticBPMDetectionConfig {
    pub fn validate(&self) -> Result<(), String> {
        StaticBPMDetectionParameters::<Self>::BPM_CENTER.validate_config_value(self)?;
        StaticBPMDetectionParameters::<Self>::BPM_RANGE.validate_config_value(self)?;
        StaticBPMDetectionParameters::<Self>::SAMPLE_RATE.validate_config_value(self)?;
        self.normal_distribution.validate()?;

        Ok(())
    }

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

impl StaticBPMDetectionConfigAccessor for StaticBPMDetectionConfig {
    fn bpm_center(&self) -> f32 {
        self.bpm_center
    }

    fn bpm_range(&self) -> u16 {
        self.bpm_range
    }

    fn sample_rate(&self) -> u16 {
        self.sample_rate
    }

    fn index_to_bpm(&self, index: usize) -> f32 {
        Self::index_to_bpm(self, index)
    }

    fn highest_bpm(&self) -> f32 {
        Self::highest_bpm(self)
    }

    fn lowest_bpm(&self) -> f32 {
        Self::lowest_bpm(self)
    }

    fn set_bpm_center(&mut self, val: f32) {
        self.bpm_center = val;
    }

    fn set_bpm_range(&mut self, val: u16) {
        self.bpm_range = val;
    }

    fn set_sample_rate(&mut self, val: u16) {
        self.sample_rate = val;
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

pub struct StaticBPMDetectionParameters<Config> {
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

#[parameter_group(
    accessor = DynamicBPMDetectionConfigAccessor,
    parameters = DynamicBPMDetectionParameters,
    default_parameters = DefaultDynamicBPMDetectionParameters,
    visitor = DynamicBPMDetectionParameterVisitor
)]
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

#[parameter_group(
    accessor = NormalDistributionConfigAccessor,
    parameters = NormalDistributionParameters,
    default_parameters = DefaultNormalDistributionParameters,
    visitor = NormalDistributionParameterVisitor
)]
#[derive(Clone, Debug, Serialize, Deserialize, Derivative)]
#[derivative(PartialEq, Eq)]
#[serde(default, deny_unknown_fields)]
pub struct NormalDistributionConfig {
    #[parameter(label = "Standard deviation", range = 4.0..=40.0, default = 24.0)]
    #[derivative(PartialEq(compare_with = "f64::eq"))]
    pub std_dev: f64,
    #[parameter(label = "factor", range = 0.0..=50.0, default = 40.0)]
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub factor: f32,
    #[parameter(label = "Normal distribution cutoff", unit = "ms", range = 1.0..=2000.0, logarithmic = true, default = 100.0)]
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub cutoff: f32, // in millisecond
    #[parameter(label = "Normal distribution resolution", unit = "ms", range = 0.01..=1000.0, logarithmic = true, default = 0.6)]
    #[derivative(PartialEq(compare_with = "f32::eq"))]
    pub resolution: f32, // 1 means one index = 1 millisecond
}

#[cfg(test)]
mod parameter_inventory_tests {
    use parameter::ParameterSpec;

    use super::*;

    struct DynamicParameterLabels(Vec<&'static str>);

    impl DynamicBPMDetectionParameterVisitor<DynamicBPMDetectionConfig> for DynamicParameterLabels {
        fn beats_lookback(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, u8>) {
            self.0.push(parameter.label);
        }

        fn normal_distribution_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn time_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn velocity_current_note_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn velocity_note_from_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn in_beat_range_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn multiplier_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn subdivision_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn octave_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn pitch_distance_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }

        fn high_tempo_bias_weight(&mut self, parameter: Parameter<DynamicBPMDetectionConfig, OnOff<f32>>) {
            self.0.push(parameter.label);
        }
    }

    struct NormalDistributionParameterLabels(Vec<&'static str>);

    impl NormalDistributionParameterVisitor<NormalDistributionConfig> for NormalDistributionParameterLabels {
        fn std_dev(&mut self, parameter: Parameter<NormalDistributionConfig, f64>) {
            self.0.push(parameter.label);
        }

        fn factor(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push(parameter.label);
        }

        fn cutoff(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push(parameter.label);
        }

        fn resolution(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push(parameter.label);
        }
    }

    #[test]
    fn dynamic_parameter_visitor_lists_every_dynamic_parameter() {
        let mut labels = DynamicParameterLabels(Vec::new());

        DynamicBPMDetectionParameters::visit(&mut labels);

        assert_eq!(
            labels.0,
            [
                "Beats Lookback",
                "Normal distribution",
                "Time distance",
                "Note velocity",
                "From note velocity",
                "In beat range",
                "Multiplier",
                "Subdivision",
                "Octave distance",
                "Pitch distance",
                "High tempo bias",
            ]
        );
    }

    #[test]
    fn normal_distribution_parameter_specs_and_visitor_preserve_inventory() {
        assert_parameter_spec(&DefaultNormalDistributionParameters::STD_DEV);
        assert_parameter_spec(&DefaultNormalDistributionParameters::FACTOR);
        assert_parameter_spec(&DefaultNormalDistributionParameters::CUTOFF);
        assert_parameter_spec(&DefaultNormalDistributionParameters::RESOLUTION);

        let mut labels = NormalDistributionParameterLabels(Vec::new());

        NormalDistributionParameters::visit(&mut labels);

        assert_eq!(
            labels.0,
            ["Standard deviation", "factor", "Normal distribution cutoff", "Normal distribution resolution",]
        );
    }

    fn assert_parameter_spec<ValueType>(_: &ParameterSpec<ValueType>) {}
}
