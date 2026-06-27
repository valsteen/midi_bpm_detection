use std::time::Duration as StdDuration;

use chrono::Duration;
use derivative::Derivative;
use parameter::{Asf64, OnOff};
use parameter_macros::parameter_group;
use serde::{Deserialize, Serialize};

use crate::DurationOps;

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

#[cfg(test)]
mod parameter_inventory_tests {
    use parameter::{Parameter, ParameterSpec};

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

        fn resolution(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push(parameter.label);
        }

        fn cutoff(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push(parameter.label);
        }

        fn factor(&mut self, parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push(parameter.label);
        }
    }

    struct NormalDistributionParameterFields(Vec<&'static str>);

    impl NormalDistributionParameterVisitor<NormalDistributionConfig> for NormalDistributionParameterFields {
        fn std_dev(&mut self, _parameter: Parameter<NormalDistributionConfig, f64>) {
            self.0.push("std_dev");
        }

        fn resolution(&mut self, _parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push("resolution");
        }

        fn cutoff(&mut self, _parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push("cutoff");
        }

        fn factor(&mut self, _parameter: Parameter<NormalDistributionConfig, f32>) {
            self.0.push("factor");
        }
    }

    struct StaticParameterLabels(Vec<&'static str>);

    impl StaticBPMDetectionParameterVisitor<StaticBPMDetectionConfig> for StaticParameterLabels {
        fn bpm_center(&mut self, parameter: Parameter<StaticBPMDetectionConfig, f32>) {
            self.0.push(parameter.label);
        }

        fn bpm_range(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>) {
            self.0.push(parameter.label);
        }

        fn sample_rate(&mut self, parameter: Parameter<StaticBPMDetectionConfig, u16>) {
            self.0.push(parameter.label);
        }
    }

    struct StaticConfigWrapper {
        config: StaticBPMDetectionConfig,
    }

    struct ExpectedStaticParameterSpec<ValueType> {
        label: &'static str,
        unit: Option<&'static str>,
        range_start: f64,
        range_end: f64,
        step: f64,
        logarithmic: bool,
        default: ValueType,
    }

    impl StaticBPMDetectionConfigAccessor for StaticConfigWrapper {
        fn bpm_center(&self) -> f32 {
            self.config.bpm_center
        }

        fn bpm_range(&self) -> u16 {
            self.config.bpm_range
        }

        fn sample_rate(&self) -> u16 {
            self.config.sample_rate
        }

        fn set_bpm_center(&mut self, val: f32) {
            self.config.bpm_center = val;
        }

        fn set_bpm_range(&mut self, val: u16) {
            self.config.bpm_range = val;
        }

        fn set_sample_rate(&mut self, val: u16) {
            self.config.sample_rate = val;
        }
    }

    #[test]
    fn dynamic_parameter_visitor_lists_every_dynamic_parameter() {
        let mut labels = DynamicParameterLabels(Vec::new());

        DynamicBPMDetectionConfig::PARAMETERS.visit(&mut labels);

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
    fn static_parameter_specs_preserve_inventory() {
        let parameter_specs = StaticBPMDetectionConfig::PARAMETER_SPECS;

        assert_parameter_spec(&parameter_specs.bpm_center());
        assert_parameter_spec(&parameter_specs.bpm_range());
        assert_parameter_spec(&parameter_specs.sample_rate());

        assert_static_parameter_spec(
            &parameter_specs.bpm_center(),
            &ExpectedStaticParameterSpec {
                label: "BPM center",
                unit: None,
                range_start: 1.0,
                range_end: 150.0,
                step: 0.01,
                logarithmic: false,
                default: 90.0,
            },
        );
        assert_static_parameter_spec(
            &parameter_specs.bpm_range(),
            &ExpectedStaticParameterSpec {
                label: "BPM range",
                unit: None,
                range_start: 1.0,
                range_end: 100.0,
                step: 1.0,
                logarithmic: false,
                default: 40,
            },
        );
        assert_static_parameter_spec(
            &parameter_specs.sample_rate(),
            &ExpectedStaticParameterSpec {
                label: "BPM sample rate",
                unit: Some("samples/second"),
                range_start: 1.0,
                range_end: 1_0000.0,
                step: 1.0,
                logarithmic: true,
                default: 450,
            },
        );
    }

    #[test]
    fn static_parameter_visitor_lists_static_parameter_fields_only() {
        let mut labels = StaticParameterLabels(Vec::new());

        StaticBPMDetectionConfig::PARAMETERS.visit(&mut labels);

        assert_eq!(labels.0, ["BPM center", "BPM range", "BPM sample rate"]);
    }

    #[test]
    fn static_validation_includes_nested_normal_distribution() {
        let mut config = StaticBPMDetectionConfig::default();
        config.normal_distribution.std_dev = 3.0;

        assert_eq!(config.validate(), Err("Standard deviation value 3 is outside declared range 4..=40".to_string()));
    }

    #[test]
    fn static_computed_methods_work_through_accessor_extension() {
        let config =
            StaticBPMDetectionConfig { bpm_center: 90.0, bpm_range: 40, sample_rate: 450, ..Default::default() };
        let wrapper = StaticConfigWrapper { config: config.clone() };

        assert_f32_eq(wrapper.lowest_bpm(), config.lowest_bpm());
        assert_f32_eq(wrapper.highest_bpm(), config.highest_bpm());
        assert_f32_eq(wrapper.index_to_bpm(0), config.index_to_bpm(0));
        assert_f32_eq(wrapper.index_to_bpm(17), config.index_to_bpm(17));
    }

    #[test]
    fn normal_distribution_parameter_specs_and_visitor_preserve_inventory() {
        let parameter_specs = NormalDistributionConfig::PARAMETER_SPECS;

        assert_parameter_spec(&parameter_specs.std_dev());
        assert_parameter_spec(&parameter_specs.resolution());
        assert_parameter_spec(&parameter_specs.cutoff());
        assert_parameter_spec(&parameter_specs.factor());

        let mut labels = NormalDistributionParameterLabels(Vec::new());

        NormalDistributionConfig::PARAMETERS.visit(&mut labels);

        assert_eq!(
            labels.0,
            ["Standard deviation", "Normal distribution resolution", "Normal distribution cutoff", "factor",]
        );
    }

    #[test]
    fn normal_distribution_generated_traversal_matches_settings_order() {
        let mut fields = NormalDistributionParameterFields(Vec::new());

        NormalDistributionConfig::PARAMETERS.visit(&mut fields);

        assert_eq!(fields.0, ["std_dev", "resolution", "cutoff", "factor"]);
    }

    fn assert_parameter_spec<ValueType>(_: &ParameterSpec<ValueType>) {}

    fn assert_static_parameter_spec<ValueType: Asf64>(
        spec: &ParameterSpec<ValueType>,
        expected: &ExpectedStaticParameterSpec<ValueType>,
    ) {
        assert_eq!(spec.label, expected.label);
        assert_eq!(spec.unit, expected.unit);
        assert_f64_eq(*spec.range.start(), expected.range_start);
        assert_f64_eq(*spec.range.end(), expected.range_end);
        assert_f64_eq(spec.step, expected.step);
        assert_eq!(spec.logarithmic, expected.logarithmic);
        assert_f64_eq(spec.default.as_f64(), expected.default.as_f64());
    }

    fn assert_f32_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() <= f32::EPSILON);
    }

    fn assert_f64_eq(actual: f64, expected: f64) {
        assert!((actual - expected).abs() <= f64::EPSILON);
    }
}
