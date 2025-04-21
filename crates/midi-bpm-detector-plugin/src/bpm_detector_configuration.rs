use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionConfigAccessor, NormalDistributionConfigAccessor,
    StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor,
};
use errors::{error_backtrace, info};
use gui::{BPMDetectionConfig, GUIConfig, GUIConfigAccessor};
use nih_plug::prelude::{AsyncExecutor, ParamSetter};
use parameter::OnOff;
use serde::{Deserialize, Serialize};
use sync::{ArcAtomicBool, RwLock};

use crate::{
    MidiBpmDetector, MidiBpmDetectorParams, Task,
    plugin_parameters::{apply_duration_param, apply_float_param, apply_int_param, apply_onoff_param},
    task_executor::UpdateOrigin,
};

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub static_bpm_detection_config: StaticBPMDetectionConfig,
    pub send_tempo: ArcAtomicBool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        match PluginConfig::deserialize(toml::de::Deserializer::new(CONFIG)) {
            Ok(config) => config,
            Err(err) => {
                error_backtrace!("{err}");
                panic!("invalid built-in configuration");
            }
        }
    }
}

pub struct BaseConfig {
    pub config: PluginConfig,
    params: Arc<MidiBpmDetectorParams>,
    shared_config: Arc<RwLock<PluginConfig>>,
    async_executor: AsyncExecutor<MidiBpmDetector>,
    force_evaluate_bpm_detection: ArcAtomicBool,
    delayed_update_dynamic_bpm_detection_config: Option<Instant>,
    delayed_update_static_bpm_detection_config: Option<Instant>,
    pub has_config_changes_via_ui: bool,
    pub send_tempo_changed: ArcAtomicBool,
}

impl BaseConfig {
    pub fn new(
        config: PluginConfig,
        shared_config: Arc<RwLock<PluginConfig>>,
        async_executor: AsyncExecutor<MidiBpmDetector>,
        force_evaluate_bpm_detection: ArcAtomicBool,
        params: Arc<MidiBpmDetectorParams>,
    ) -> Self {
        Self {
            config,
            shared_config,
            async_executor,
            force_evaluate_bpm_detection,
            delayed_update_dynamic_bpm_detection_config: None,
            delayed_update_static_bpm_detection_config: None,
            has_config_changes_via_ui: false,
            params,
            send_tempo_changed: ArcAtomicBool::default(),
        }
    }

    fn delay_static_changes(&mut self) {
        self.has_config_changes_via_ui = true;
        if self.delayed_update_static_bpm_detection_config.is_none() {
            self.delayed_update_static_bpm_detection_config = Some(Instant::now());
        }
    }

    fn delay_dynamic_changes(&mut self) {
        self.has_config_changes_via_ui = true;
        if self.delayed_update_dynamic_bpm_detection_config.is_none() {
            self.delayed_update_dynamic_bpm_detection_config = Some(Instant::now());
        }
    }

    pub fn apply_delayed_updates(&mut self) {
        if self
            .delayed_update_static_bpm_detection_config
            .is_some_and(|instant| instant.elapsed() > Duration::from_millis(200))
        {
            {
                *self.shared_config.write() = self.config.clone();
            }

            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::StaticBPMDetectionConfig(UpdateOrigin::Gui));
            self.delayed_update_static_bpm_detection_config = None;
            info!("apply static params");
        }
        if self
            .delayed_update_dynamic_bpm_detection_config
            .is_some_and(|instant| instant.elapsed() > Duration::from_millis(200))
        {
            {
                *self.shared_config.write() = self.config.clone();
            }
            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::DynamicBPMDetectionConfig(UpdateOrigin::Gui));
            self.delayed_update_dynamic_bpm_detection_config = None;
            info!("apply dynamic params");
        }
    }
}

pub(crate) struct LiveConfig<'_self> {
    pub(crate) base_config: &'_self mut BaseConfig,
    pub(crate) param_setter: &'_self ParamSetter<'_self>,
}

impl NormalDistributionConfigAccessor for LiveConfig<'_> {
    fn std_dev(&self) -> f64 {
        self.base_config.config.static_bpm_detection_config.normal_distribution.std_dev
    }

    fn factor(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.normal_distribution.factor
    }

    fn cutoff(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.normal_distribution.cutoff
    }

    fn resolution(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.normal_distribution.resolution
    }

    fn set_std_dev(&mut self, val: f64) {
        self.base_config.config.static_bpm_detection_config.normal_distribution.std_dev = val;
        apply_float_param(
            &self.base_config.params.static_params.normal_distribution.std_dev,
            self.base_config.config.static_bpm_detection_config.normal_distribution.std_dev,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_factor(&mut self, val: f32) {
        self.base_config.config.static_bpm_detection_config.normal_distribution.factor = val;
        apply_float_param(
            &self.base_config.params.static_params.normal_distribution.factor,
            self.base_config.config.static_bpm_detection_config.normal_distribution.factor,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_cutoff(&mut self, val: f32) {
        self.base_config.config.static_bpm_detection_config.normal_distribution.cutoff = val;
        apply_float_param(
            &self.base_config.params.static_params.normal_distribution.cutoff,
            self.base_config.config.static_bpm_detection_config.normal_distribution.cutoff,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_resolution(&mut self, val: f32) {
        self.base_config.config.static_bpm_detection_config.normal_distribution.resolution = val;
        apply_float_param(
            &self.base_config.params.static_params.normal_distribution.resolution,
            self.base_config.config.static_bpm_detection_config.normal_distribution.resolution,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }
}

impl DynamicBPMDetectionConfigAccessor for LiveConfig<'_> {
    fn beats_lookback(&self) -> u8 {
        self.base_config.config.dynamic_bpm_detection_config.beats_lookback
    }

    fn velocity_current_note_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.velocity_current_note_weight
    }

    fn velocity_note_from_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.velocity_note_from_weight
    }

    fn time_distance_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.time_distance_weight
    }

    fn octave_distance_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.octave_distance_weight
    }

    fn pitch_distance_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.pitch_distance_weight
    }

    fn multiplier_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.multiplier_weight
    }

    fn subdivision_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.subdivision_weight
    }

    fn in_beat_range_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.in_beat_range_weight
    }

    fn normal_distribution_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.normal_distribution_weight
    }

    fn high_tempo_bias(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.high_tempo_bias
    }

    fn set_beats_lookback(&mut self, val: u8) {
        self.base_config.config.dynamic_bpm_detection_config.beats_lookback = val;
        apply_int_param(&self.base_config.params.dynamic_params.beats_lookback, val, self.param_setter);
        self.base_config.delay_dynamic_changes();
    }

    fn set_velocity_current_note_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.velocity_current_note_weight,
            &self.base_config.params.dynamic_params.velocity_current_note_onoff,
            self.base_config.config.dynamic_bpm_detection_config.velocity_current_note_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.velocity_current_note_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_velocity_note_from_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.velocity_note_from_weight,
            &self.base_config.params.dynamic_params.velocity_note_from_onoff,
            self.base_config.config.dynamic_bpm_detection_config.velocity_note_from_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.velocity_note_from_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_time_distance_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.time_distance_weight,
            &self.base_config.params.dynamic_params.time_distance_onoff,
            self.base_config.config.dynamic_bpm_detection_config.time_distance_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.time_distance_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_octave_distance_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.octave_distance_weight,
            &self.base_config.params.dynamic_params.octave_distance_onoff,
            self.base_config.config.dynamic_bpm_detection_config.octave_distance_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.octave_distance_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_pitch_distance_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.pitch_distance_weight,
            &self.base_config.params.dynamic_params.pitch_distance_onoff,
            self.base_config.config.dynamic_bpm_detection_config.pitch_distance_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.pitch_distance_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_multiplier_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.multiplier_weight,
            &self.base_config.params.dynamic_params.multiplier_onoff,
            self.base_config.config.dynamic_bpm_detection_config.multiplier_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.multiplier_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_subdivision_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.subdivision_weight,
            &self.base_config.params.dynamic_params.subdivision_onoff,
            self.base_config.config.dynamic_bpm_detection_config.subdivision_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.subdivision_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_in_beat_range_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.in_beat_range_weight,
            &self.base_config.params.dynamic_params.in_beat_range_onoff,
            self.base_config.config.dynamic_bpm_detection_config.in_beat_range_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.in_beat_range_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_normal_distribution_weight(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.normal_distribution_weight,
            &self.base_config.params.dynamic_params.normal_distribution_onoff,
            self.base_config.config.dynamic_bpm_detection_config.normal_distribution_weight,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.normal_distribution_weight = val;
        self.base_config.delay_dynamic_changes();
    }

    fn set_high_tempo_bias(&mut self, val: OnOff<f32>) {
        apply_onoff_param(
            &self.base_config.params.dynamic_params.high_tempo_bias,
            &self.base_config.params.dynamic_params.high_tempo_bias_onoff,
            self.base_config.config.dynamic_bpm_detection_config.high_tempo_bias,
            val,
            self.param_setter,
        );
        self.base_config.config.dynamic_bpm_detection_config.high_tempo_bias = val;
        self.base_config.delay_dynamic_changes();
    }
}

impl StaticBPMDetectionConfigAccessor for LiveConfig<'_> {
    fn bpm_center(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.bpm_center
    }

    fn bpm_range(&self) -> u16 {
        self.base_config.config.static_bpm_detection_config.bpm_range
    }

    fn sample_rate(&self) -> u16 {
        self.base_config.config.static_bpm_detection_config.sample_rate
    }

    fn index_to_bpm(&self, index: usize) -> f32 {
        self.base_config.config.static_bpm_detection_config.index_to_bpm(index)
    }

    fn highest_bpm(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.highest_bpm()
    }

    fn lowest_bpm(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.lowest_bpm()
    }

    fn set_bpm_center(&mut self, val: f32) {
        self.base_config.config.static_bpm_detection_config.bpm_center = val;
        apply_float_param(&self.base_config.params.static_params.bpm_center, val, self.param_setter);
        self.base_config.delay_static_changes();
    }

    fn set_bpm_range(&mut self, val: u16) {
        self.base_config.config.static_bpm_detection_config.bpm_range = val;
        apply_int_param(&self.base_config.params.static_params.bpm_range, val, self.param_setter);
        self.base_config.delay_static_changes();
    }

    fn set_sample_rate(&mut self, val: u16) {
        self.base_config.config.static_bpm_detection_config.sample_rate = val;
        apply_float_param(&self.base_config.params.static_params.sample_rate, val, self.param_setter);
        self.base_config.delay_static_changes();
    }
}

impl GUIConfigAccessor for LiveConfig<'_> {
    fn interpolation_duration(&self) -> Duration {
        self.base_config.config.gui_config.interpolation_duration
    }

    fn interpolation_curve(&self) -> f32 {
        self.base_config.config.gui_config.interpolation_curve
    }

    fn set_interpolation_duration(&mut self, val: Duration) {
        self.base_config.config.gui_config.interpolation_duration = val;
        apply_duration_param(&self.base_config.params.gui_params.interpolation_duration, val, self.param_setter);
        self.base_config.delay_dynamic_changes();
    }

    fn set_interpolation_curve(&mut self, val: f32) {
        self.base_config.config.gui_config.interpolation_curve = val;
        apply_float_param(&self.base_config.params.gui_params.interpolation_curve, val, self.param_setter);
        self.base_config.delay_dynamic_changes();
    }
}

impl BPMDetectionConfig for LiveConfig<'_> {
    fn get_send_tempo(&self) -> bool {
        self.base_config.config.send_tempo.load(Ordering::Relaxed)
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.base_config.config.send_tempo.store(enabled, Ordering::SeqCst);
        self.base_config.send_tempo_changed.store(true, Ordering::SeqCst);
    }
}
