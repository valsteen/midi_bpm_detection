#![cfg(target_arch = "wasm32")]

use std::time::Duration;

use bpm_detection_core::{
    DynamicBPMDetectionConfig, NormalDistributionConfigAccessor, StaticBPMDetectionConfig, TimedTypedMidiMessage,
    bpm::{DynamicBPMDetectionConfigAccessor, StaticBPMDetectionConfigAccessor},
    midi_messages::MidiNoteOn,
};
use derivative::Derivative;
use errors::{LogErrorWithExt, error_backtrace};
use futures::channel::mpsc::Sender;
use gui::{BPMDetectionConfig, GUIConfig, GUIConfigAccessor};
use parameter::OnOff;
use serde::{Deserialize, Serialize};

pub mod wasm;

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Derivative, Serialize, Deserialize)]
pub struct WASMConfig {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
}

enum QueueItem {
    StaticParameters(StaticBPMDetectionConfig),
    DynamicParameters(DynamicBPMDetectionConfig),
    Note(TimedTypedMidiMessage<MidiNoteOn>),
    DelayedDynamicUpdate,
    DelayedStaticUpdate,
}

pub struct BaseConfig {
    config: WASMConfig,
    sender: Sender<QueueItem>,
}

impl BaseConfig {
    fn new(sender: Sender<QueueItem>) -> Self {
        Self { config: WASMConfig::default(), sender }
    }

    fn propagate_static_changes(&mut self) {
        self.sender
            .try_send(QueueItem::StaticParameters(self.config.static_bpm_detection_config.clone()))
            .log_error_msg("channel full")
            .ok();
    }

    fn propagate_dynamic_changes(&mut self) {
        self.sender
            .try_send(QueueItem::DynamicParameters(self.config.dynamic_bpm_detection_config.clone()))
            .log_error_msg("channel full")
            .ok();
    }
}

impl NormalDistributionConfigAccessor for BaseConfig {
    fn std_dev(&self) -> f64 {
        self.config.static_bpm_detection_config.normal_distribution.std_dev
    }

    fn factor(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.factor
    }

    fn cutoff(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.cutoff
    }

    fn resolution(&self) -> f32 {
        self.config.static_bpm_detection_config.normal_distribution.resolution
    }

    fn set_std_dev(&mut self, val: f64) {
        self.config.static_bpm_detection_config.normal_distribution.std_dev = val;
        self.propagate_static_changes();
    }

    fn set_factor(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.factor = val;
        self.propagate_static_changes();
    }

    fn set_cutoff(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.cutoff = val;
        self.propagate_static_changes();
    }

    fn set_resolution(&mut self, val: f32) {
        self.config.static_bpm_detection_config.normal_distribution.resolution = val;
        self.propagate_static_changes();
    }
}

impl DynamicBPMDetectionConfigAccessor for BaseConfig {
    fn beats_lookback(&self) -> u8 {
        self.config.dynamic_bpm_detection_config.beats_lookback
    }

    fn velocity_current_note_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.velocity_current_note_weight
    }

    fn velocity_note_from_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.velocity_note_from_weight
    }

    fn time_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.time_distance_weight
    }

    fn octave_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.octave_distance_weight
    }

    fn pitch_distance_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.pitch_distance_weight
    }

    fn multiplier_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.multiplier_weight
    }

    fn subdivision_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.subdivision_weight
    }

    fn in_beat_range_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.in_beat_range_weight
    }

    fn normal_distribution_weight(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.normal_distribution_weight
    }

    fn high_tempo_bias(&self) -> OnOff<f32> {
        self.config.dynamic_bpm_detection_config.high_tempo_bias
    }

    fn set_beats_lookback(&mut self, val: u8) {
        self.config.dynamic_bpm_detection_config.beats_lookback = val;
        self.propagate_dynamic_changes();
    }

    fn set_velocity_current_note_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.velocity_current_note_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_velocity_note_from_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.velocity_note_from_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_time_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.time_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_octave_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.octave_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_pitch_distance_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.pitch_distance_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_multiplier_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.multiplier_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_subdivision_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.subdivision_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_in_beat_range_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.in_beat_range_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_normal_distribution_weight(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.normal_distribution_weight = val;
        self.propagate_dynamic_changes();
    }

    fn set_high_tempo_bias(&mut self, val: OnOff<f32>) {
        self.config.dynamic_bpm_detection_config.high_tempo_bias = val;
        self.propagate_dynamic_changes();
    }
}

impl StaticBPMDetectionConfigAccessor for BaseConfig {
    fn bpm_center(&self) -> f32 {
        self.config.static_bpm_detection_config.bpm_center
    }

    fn bpm_range(&self) -> u16 {
        self.config.static_bpm_detection_config.bpm_range
    }

    fn sample_rate(&self) -> u16 {
        self.config.static_bpm_detection_config.sample_rate
    }

    fn index_to_bpm(&self, index: usize) -> f32 {
        self.config.static_bpm_detection_config.index_to_bpm(index)
    }

    fn highest_bpm(&self) -> f32 {
        self.config.static_bpm_detection_config.highest_bpm()
    }

    fn lowest_bpm(&self) -> f32 {
        self.config.static_bpm_detection_config.lowest_bpm()
    }

    fn set_bpm_center(&mut self, val: f32) {
        self.config.static_bpm_detection_config.bpm_center = val;
        self.propagate_static_changes();
    }

    fn set_bpm_range(&mut self, val: u16) {
        self.config.static_bpm_detection_config.bpm_range = val;
        self.propagate_static_changes();
    }

    fn set_sample_rate(&mut self, val: u16) {
        self.config.static_bpm_detection_config.sample_rate = val;
        self.propagate_static_changes();
    }
}

impl GUIConfigAccessor for BaseConfig {
    fn interpolation_duration(&self) -> Duration {
        self.config.gui_config.interpolation_duration
    }

    fn interpolation_curve(&self) -> f32 {
        self.config.gui_config.interpolation_curve
    }

    fn set_interpolation_duration(&mut self, val: Duration) {
        self.config.gui_config.interpolation_duration = val;
        self.propagate_dynamic_changes();
    }

    fn set_interpolation_curve(&mut self, val: f32) {
        self.config.gui_config.interpolation_curve = val;
        self.propagate_dynamic_changes();
    }
}

impl BPMDetectionConfig for BaseConfig {
    fn get_send_tempo(&self) -> bool {
        false
    }

    fn set_send_tempo(&mut self, _: bool) {}
}

impl Default for WASMConfig {
    fn default() -> Self {
        match WASMConfig::deserialize(toml::de::Deserializer::new(CONFIG)) {
            Ok(config) => config,
            Err(err) => {
                error_backtrace!("{err}");
                panic!("invalid built-in configuration");
            }
        }
    }
}

pub mod test {
    #![allow(forbidden_lint_groups)]
    #![allow(clippy::missing_panics_doc)]
    #[allow(clippy::module_name_repetitions)]
    use errors::error_backtrace;
    use parameter::OnOff;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct Config {
        pub test: OnOff<f32>,
    }

    impl Default for Config {
        fn default() -> Self {
            match Config::deserialize(toml::de::Deserializer::new(CONFIG)) {
                Ok(config) => config,
                Err(err) => {
                    error_backtrace!("{err}");
                    panic!("invalid built-in configuration");
                }
            }
        }
    }

    const CONFIG: &str = "[test]
enabled = false
value = 1";

    #[test]
    pub fn test_config() {
        let config = Config::default();
        assert_eq!(config.test, OnOff::Off(1.0));
    }
}
