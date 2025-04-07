use std::{
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};

use bpm_detection_core::{DynamicBPMDetectionParameters, NormalDistributionParameters, StaticBPMDetectionParameters};
use errors::{error_backtrace, info};
use gui::{BPMDetectionParameters, GUIConfig};
use nih_plug::prelude::{AsyncExecutor, ParamSetter};
use serde::{Deserialize, Serialize};
use sync::{ArcAtomicBool, RwLock};

use crate::{
    MidiBpmDetector, MidiBpmDetectorParams, Task,
    plugin_parameters::{apply_duration_param, apply_float_param, apply_int_param, apply_onoff_param},
    task_executor::UpdateOrigin,
};

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "GUI")]
    pub gui_config: GUIConfig,
    pub dynamic_bpm_detection_parameters: DynamicBPMDetectionParameters,
    pub static_bpm_detection_parameters: StaticBPMDetectionParameters,
    pub send_tempo: ArcAtomicBool,
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

pub struct LiveConfig {
    pub config: Config,
    params: Arc<MidiBpmDetectorParams>,
    shared_config: Arc<RwLock<Config>>,
    async_executor: AsyncExecutor<MidiBpmDetector>,
    force_evaluate_bpm_detection: ArcAtomicBool,
    delayed_update_dynamic_bpm_detection_parameters: Option<Instant>,
    delayed_update_static_bpm_detection_parameters: Option<Instant>,
    dynamic_bpm_detection_parameters_changed: bool,
    static_bpm_detection_parameters_changed: bool,
    pub send_tempo_changed: ArcAtomicBool,
}

impl LiveConfig {
    pub fn new(
        config: Config,
        shared_config: Arc<RwLock<Config>>,
        async_executor: AsyncExecutor<MidiBpmDetector>,
        force_evaluate_bpm_detection: ArcAtomicBool,
        params: Arc<MidiBpmDetectorParams>,
    ) -> Self {
        Self {
            config,
            shared_config,
            async_executor,
            force_evaluate_bpm_detection,
            delayed_update_dynamic_bpm_detection_parameters: None,
            delayed_update_static_bpm_detection_parameters: None,
            dynamic_bpm_detection_parameters_changed: false,
            static_bpm_detection_parameters_changed: false,
            params,
            send_tempo_changed: ArcAtomicBool::default(),
        }
    }

    pub fn apply_delayed_updates(&mut self) {
        if self
            .delayed_update_static_bpm_detection_parameters
            .is_some_and(|instant| instant.elapsed() > Duration::from_millis(200))
        {
            {
                *self.shared_config.write() = self.config.clone();
            }

            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::StaticBPMDetectionParameters(UpdateOrigin::Gui));
            self.delayed_update_static_bpm_detection_parameters = None;
            info!("apply static params");
        }
        if self
            .delayed_update_dynamic_bpm_detection_parameters
            .is_some_and(|instant| instant.elapsed() > Duration::from_millis(200))
        {
            {
                *self.shared_config.write() = self.config.clone();
            }
            self.force_evaluate_bpm_detection.store(true, Ordering::Relaxed);
            self.async_executor.execute_background(Task::DynamicBPMDetectionParameters(UpdateOrigin::Gui));
            self.delayed_update_dynamic_bpm_detection_parameters = None;
            info!("apply dynamic params");
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn apply_changes_to_daw_parameters(&mut self, param_setter: &ParamSetter) -> bool {
        let mut has_changes = false;
        if self.dynamic_bpm_detection_parameters_changed {
            has_changes = true;
            apply_float_param(
                &GUIConfig::INTERPOLATION_CURVE,
                &self.params.gui_params.interpolation_curve,
                &self.config.gui_config,
                param_setter,
            );
            apply_duration_param(
                &GUIConfig::INTERPOLATION_DURATION,
                &self.params.gui_params.interpolation_duration,
                &self.config.gui_config,
                param_setter,
            );
            apply_int_param(
                &DynamicBPMDetectionParameters::BEATS_LOOKBACK,
                &self.params.dynamic_params.beats_lookback,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::CURRENT_VELOCITY,
                &self.params.dynamic_params.velocity_current_note_onoff,
                &self.params.dynamic_params.velocity_current_note_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::VELOCITY_FROM,
                &self.params.dynamic_params.velocity_note_from_onoff,
                &self.params.dynamic_params.velocity_note_from_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::TIME_DISTANCE,
                &self.params.dynamic_params.age_onoff,
                &self.params.dynamic_params.time_distance_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::OCTAVE_DISTANCE,
                &self.params.dynamic_params.octave_distance_onoff,
                &self.params.dynamic_params.octave_distance_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::PITCH_DISTANCE,
                &self.params.dynamic_params.pitch_distance_onoff,
                &self.params.dynamic_params.pitch_distance_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::MULTIPLIER_FACTOR,
                &self.params.dynamic_params.multiplier_onoff,
                &self.params.dynamic_params.multiplier_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::SUBDIVISION_FACTOR,
                &self.params.dynamic_params.subdivision_onoff,
                &self.params.dynamic_params.subdivision_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::IN_RANGE,
                &self.params.dynamic_params.in_beat_range_onoff,
                &self.params.dynamic_params.in_beat_range_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::NORMAL_DISTRIBUTION,
                &self.params.dynamic_params.normal_distribution_onoff,
                &self.params.dynamic_params.normal_distribution_weight,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            apply_onoff_param(
                &DynamicBPMDetectionParameters::HIGH_TEMPO_BIAS,
                &self.params.dynamic_params.high_tempo_bias_onoff,
                &self.params.dynamic_params.high_tempo_bias,
                &self.config.dynamic_bpm_detection_parameters,
                param_setter,
            );
            self.dynamic_bpm_detection_parameters_changed = false;
        }
        if self.static_bpm_detection_parameters_changed {
            has_changes = true;
            apply_float_param(
                &StaticBPMDetectionParameters::BPM_CENTER,
                &self.params.static_params.bpm_center,
                &self.config.static_bpm_detection_parameters,
                param_setter,
            );
            apply_int_param(
                &StaticBPMDetectionParameters::BPM_RANGE,
                &self.params.static_params.bpm_range,
                &self.config.static_bpm_detection_parameters,
                param_setter,
            );
            apply_float_param(
                &StaticBPMDetectionParameters::SAMPLE_RATE,
                &self.params.static_params.sample_rate,
                &self.config.static_bpm_detection_parameters,
                param_setter,
            );
            apply_float_param(
                &NormalDistributionParameters::STD_DEV,
                &self.params.static_params.normal_distribution.std_dev,
                &self.config.static_bpm_detection_parameters.normal_distribution,
                param_setter,
            );
            apply_float_param(
                &NormalDistributionParameters::FACTOR,
                &self.params.static_params.normal_distribution.factor,
                &self.config.static_bpm_detection_parameters.normal_distribution,
                param_setter,
            );
            apply_float_param(
                &NormalDistributionParameters::CUTOFF,
                &self.params.static_params.normal_distribution.imprecision,
                &self.config.static_bpm_detection_parameters.normal_distribution,
                param_setter,
            );
            apply_float_param(
                &NormalDistributionParameters::RESOLUTION,
                &self.params.static_params.normal_distribution.resolution,
                &self.config.static_bpm_detection_parameters.normal_distribution,
                param_setter,
            );
            self.static_bpm_detection_parameters_changed = false;
        }
        has_changes
    }
}

impl BPMDetectionParameters for LiveConfig {
    type Error = ();

    fn get_dynamic_bpm_detection_parameters(&self) -> &DynamicBPMDetectionParameters {
        &self.config.dynamic_bpm_detection_parameters
    }

    fn get_dynamic_bpm_detection_parameters_mut(&mut self) -> &mut DynamicBPMDetectionParameters {
        &mut self.config.dynamic_bpm_detection_parameters
    }

    fn get_static_bpm_detection_parameters(&self) -> &StaticBPMDetectionParameters {
        &self.config.static_bpm_detection_parameters
    }

    fn get_static_bpm_detection_parameters_mut(&mut self) -> &mut StaticBPMDetectionParameters {
        &mut self.config.static_bpm_detection_parameters
    }

    fn get_gui_config(&self) -> &GUIConfig {
        &self.config.gui_config
    }

    fn get_gui_config_mut(&mut self) -> &mut GUIConfig {
        &mut self.config.gui_config
    }

    fn get_send_tempo(&self) -> bool {
        self.config.send_tempo.load(Ordering::Relaxed)
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.send_tempo_changed.store(true, Ordering::SeqCst);
        self.config.send_tempo.store(enabled, Ordering::SeqCst);
    }

    fn apply_static(&mut self) -> Result<(), Self::Error> {
        self.static_bpm_detection_parameters_changed = true;
        if self.delayed_update_static_bpm_detection_parameters.is_none() {
            self.delayed_update_static_bpm_detection_parameters = Some(Instant::now());
        }
        Ok(())
    }

    fn apply_dynamic(&mut self) -> Result<(), Self::Error> {
        self.dynamic_bpm_detection_parameters_changed = true;
        if self.delayed_update_dynamic_bpm_detection_parameters.is_none() {
            self.delayed_update_dynamic_bpm_detection_parameters = Some(Instant::now());
        }
        Ok(())
    }
}
