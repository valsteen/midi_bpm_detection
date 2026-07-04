use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionConfigAccessor, NormalDistributionConfig,
    NormalDistributionConfigAccessor, StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor,
};
use errors::info;
use gui::{BPMDetectionConfig, GUIConfigAccessor};
use nih_plug::prelude::{AsyncExecutor, ParamSetter};
use parameter::OnOff;
use parameter_nih_plug::MirrorHostParam;
use sync::{ArcAtomicBool, RwLock};

use crate::{
    MidiBpmDetector, MidiBpmDetectorParams, Task,
    parameter_sync::{GUI_PARAMETER_SYNC_COALESCING_WINDOW, ParameterSyncOrigin},
    plugin_config::PluginConfig,
};

pub struct BaseConfig {
    pub config: PluginConfig,
    params: Arc<MidiBpmDetectorParams>,
    gui_task_config: Arc<RwLock<PluginConfig>>,
    async_executor: AsyncExecutor<MidiBpmDetector>,
    force_evaluate_bpm_detection: ArcAtomicBool,
    delayed_update_dynamic_bpm_detection_config: Option<Instant>,
    delayed_update_gui_config: Option<Instant>,
    delayed_update_static_bpm_detection_config: Option<Instant>,
    pub has_config_changes_via_ui: bool,
}

impl BaseConfig {
    pub fn new(
        config: PluginConfig,
        gui_task_config: Arc<RwLock<PluginConfig>>,
        async_executor: AsyncExecutor<MidiBpmDetector>,
        force_evaluate_bpm_detection: ArcAtomicBool,
        params: Arc<MidiBpmDetectorParams>,
    ) -> Self {
        Self {
            config,
            gui_task_config,
            async_executor,
            force_evaluate_bpm_detection,
            delayed_update_dynamic_bpm_detection_config: None,
            delayed_update_gui_config: None,
            delayed_update_static_bpm_detection_config: None,
            has_config_changes_via_ui: false,
            params,
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

    fn delay_gui_changes(&mut self) {
        self.has_config_changes_via_ui = true;
        if self.delayed_update_gui_config.is_none() {
            self.delayed_update_gui_config = Some(Instant::now());
        }
    }

    pub fn apply_delayed_updates(&mut self) {
        if self
            .delayed_update_static_bpm_detection_config
            .is_some_and(|instant| instant.elapsed() > GUI_PARAMETER_SYNC_COALESCING_WINDOW)
        {
            {
                *self.gui_task_config.write() = self.config.clone();
            }

            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::StaticBPMDetectionConfig(ParameterSyncOrigin::Gui));
            self.delayed_update_static_bpm_detection_config = None;
            info!("apply static params");
        }
        if self
            .delayed_update_gui_config
            .is_some_and(|instant| instant.elapsed() > GUI_PARAMETER_SYNC_COALESCING_WINDOW)
        {
            {
                *self.gui_task_config.write() = self.config.clone();
            }
            self.async_executor.execute_background(Task::GUIConfig(ParameterSyncOrigin::Gui));
            self.delayed_update_gui_config = None;
            info!("apply GUI params");
        }
        if self
            .delayed_update_dynamic_bpm_detection_config
            .is_some_and(|instant| instant.elapsed() > GUI_PARAMETER_SYNC_COALESCING_WINDOW)
        {
            {
                *self.gui_task_config.write() = self.config.clone();
            }
            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::DynamicBPMDetectionConfig(ParameterSyncOrigin::Gui));
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

    fn resolution(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.normal_distribution.resolution
    }

    fn cutoff(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.normal_distribution.cutoff
    }

    fn factor(&self) -> f32 {
        self.base_config.config.static_bpm_detection_config.normal_distribution.factor
    }

    fn set_std_dev(&mut self, val: f64) {
        self.base_config.params.static_params.normal_distribution.std_dev.mirror_host_param(
            &mut self.base_config.config.static_bpm_detection_config.normal_distribution,
            &NormalDistributionConfig::PARAMETERS.std_dev(),
            val,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_resolution(&mut self, val: f32) {
        self.base_config.params.static_params.normal_distribution.resolution.mirror_host_param(
            &mut self.base_config.config.static_bpm_detection_config.normal_distribution,
            &NormalDistributionConfig::PARAMETERS.resolution(),
            val,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_cutoff(&mut self, val: f32) {
        self.base_config.params.static_params.normal_distribution.cutoff.mirror_host_param(
            &mut self.base_config.config.static_bpm_detection_config.normal_distribution,
            &NormalDistributionConfig::PARAMETERS.cutoff(),
            val,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_factor(&mut self, val: f32) {
        self.base_config.params.static_params.normal_distribution.factor.mirror_host_param(
            &mut self.base_config.config.static_bpm_detection_config.normal_distribution,
            &NormalDistributionConfig::PARAMETERS.factor(),
            val,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }
}

impl DynamicBPMDetectionConfigAccessor for LiveConfig<'_> {
    fn beats_lookback(&self) -> u8 {
        self.base_config.config.dynamic_bpm_detection_config.beats_lookback
    }

    fn normal_distribution_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.normal_distribution_weight
    }

    fn time_distance_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.time_distance_weight
    }

    fn velocity_current_note_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.velocity_current_note_weight
    }

    fn velocity_note_from_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.velocity_note_from_weight
    }

    fn in_beat_range_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.in_beat_range_weight
    }

    fn multiplier_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.multiplier_weight
    }

    fn subdivision_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.subdivision_weight
    }

    fn octave_distance_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.octave_distance_weight
    }

    fn pitch_distance_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.pitch_distance_weight
    }

    fn high_tempo_bias_weight(&self) -> OnOff<f32> {
        self.base_config.config.dynamic_bpm_detection_config.high_tempo_bias_weight
    }

    fn set_beats_lookback(&mut self, val: u8) {
        self.base_config.params.dynamic_params.beats_lookback.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.beats_lookback(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_normal_distribution_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.normal_distribution_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.normal_distribution_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_time_distance_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.time_distance_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.time_distance_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_velocity_current_note_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.velocity_current_note_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.velocity_current_note_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_velocity_note_from_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.velocity_note_from_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.velocity_note_from_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_in_beat_range_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.in_beat_range_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.in_beat_range_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_multiplier_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.multiplier_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.multiplier_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_subdivision_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.subdivision_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.subdivision_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_octave_distance_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.octave_distance_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.octave_distance_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_pitch_distance_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.pitch_distance_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.pitch_distance_weight(),
            val,
            self.param_setter,
        );
        self.base_config.delay_dynamic_changes();
    }

    fn set_high_tempo_bias_weight(&mut self, val: OnOff<f32>) {
        self.base_config.params.dynamic_params.high_tempo_bias_weight.mirror_host_param(
            &mut self.base_config.config.dynamic_bpm_detection_config,
            &DynamicBPMDetectionConfig::PARAMETERS.high_tempo_bias_weight(),
            val,
            self.param_setter,
        );
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

    fn set_bpm_center(&mut self, val: f32) {
        self.base_config.params.static_params.bpm_center.mirror_host_param(
            &mut self.base_config.config.static_bpm_detection_config,
            &StaticBPMDetectionConfig::PARAMETERS.bpm_center(),
            val,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_bpm_range(&mut self, val: u16) {
        self.base_config.params.static_params.bpm_range.mirror_host_param(
            &mut self.base_config.config.static_bpm_detection_config,
            &StaticBPMDetectionConfig::PARAMETERS.bpm_range(),
            val,
            self.param_setter,
        );
        self.base_config.delay_static_changes();
    }

    fn set_sample_rate(&mut self, val: u16) {
        self.base_config.params.static_params.sample_rate.mirror_host_param(
            &mut self.base_config.config.static_bpm_detection_config,
            &StaticBPMDetectionConfig::PARAMETERS.sample_rate(),
            val,
            self.param_setter,
        );
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
        self.base_config.params.gui_params.interpolation_duration.mirror_host_param(
            &mut self.base_config.config.gui_config,
            &gui::GUIConfig::PARAMETERS.interpolation_duration(),
            val,
            self.param_setter,
        );
        self.base_config.delay_gui_changes();
    }

    fn set_interpolation_curve(&mut self, val: f32) {
        self.base_config.params.gui_params.interpolation_curve.mirror_host_param(
            &mut self.base_config.config.gui_config,
            &gui::GUIConfig::PARAMETERS.interpolation_curve(),
            val,
            self.param_setter,
        );
        self.base_config.delay_gui_changes();
    }
}

impl BPMDetectionConfig for LiveConfig<'_> {
    fn get_send_tempo(&self) -> bool {
        self.base_config.config.send_tempo.enabled()
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.base_config.config.send_tempo.set_from_gui(enabled);
    }
}
