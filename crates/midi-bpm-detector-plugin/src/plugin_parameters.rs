use std::{
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use bpm_detection_core::{DynamicBPMDetectionParameters, NormalDistributionConfig, StaticBPMDetectionParameters};
use gui::GUIConfig;
use nih_plug::{
    params::{BoolParam, FloatParam, IntParam, Param, Params},
    prelude::{FloatRange, IntRange, ParamSetter},
};
use nih_plug_egui::EguiState;
use num_traits::ToPrimitive;
use parameter::{OnOff, Parameter};
use sync::ArcAtomicOptional;

use crate::bpm_detector_configuration::Config;

#[derive(Params)]
pub struct GUIParams {
    #[id = "interpolation_duration"]
    pub interpolation_duration: FloatParam,
    #[id = "interpolation_curve"]
    pub interpolation_curve: FloatParam,
}

#[derive(Params)]
pub struct DynamicParams {
    #[id = "beats_lookback"]
    pub beats_lookback: IntParam,
    #[id = "velocity_current_note_weight"]
    pub velocity_current_note_weight: FloatParam,
    #[id = "velocity_current_note_onoff"]
    pub velocity_current_note_onoff: BoolParam,
    #[id = "velocity_note_from_weight"]
    pub velocity_note_from_weight: FloatParam,
    #[id = "velocity_note_from_onoff"]
    pub velocity_note_from_onoff: BoolParam,
    #[id = "age_weight"]
    pub age_weight: FloatParam,
    #[id = "age_onoff"]
    pub age_onoff: BoolParam,
    #[id = "octave_distance_weight"]
    pub octave_distance_weight: FloatParam,
    #[id = "octave_distance_onoff"]
    pub octave_distance_onoff: BoolParam,
    #[id = "pitch_distance_weight"]
    pub pitch_distance_weight: FloatParam,
    #[id = "pitch_distance_onoff"]
    pub pitch_distance_onoff: BoolParam,
    #[id = "multiplier_weight"]
    pub multiplier_weight: FloatParam,
    #[id = "multiplier_onoff"]
    pub multiplier_onoff: BoolParam,
    #[id = "subdivision_weight"]
    pub subdivision_weight: FloatParam,
    #[id = "subdivision_onoff"]
    pub subdivision_onoff: BoolParam,
    #[id = "in_beat_range_weight"]
    pub in_beat_range_weight: FloatParam,
    #[id = "in_beat_range_onoff"]
    pub in_beat_range_onoff: BoolParam,
    #[id = "normal_distribution_weight"]
    pub normal_distribution_weight: FloatParam,
    #[id = "normal_distribution_onoff"]
    pub normal_distribution_onoff: BoolParam,
    #[id = "high_tempo_bias"]
    pub high_tempo_bias: FloatParam,
    #[id = "high_tempo_bias_onoff"]
    pub high_tempo_bias_onoff: BoolParam,
}

#[derive(Params)]
pub struct NormalDistributionParams {
    #[id = "std_dev"]
    pub std_dev: FloatParam,
    #[id = "factor"]
    pub factor: FloatParam,
    #[id = "imprecision"]
    pub imprecision: FloatParam,
    #[id = "resolution"]
    pub resolution: FloatParam,
}

#[derive(Params)]
pub struct StaticParams {
    #[id = "lower_bound"]
    pub bpm_center: FloatParam,
    #[id = "upper_bound"]
    pub bpm_range: IntParam,
    #[id = "sample_rate"]
    pub sample_rate: FloatParam,
    #[nested(group = "normal_distribution")]
    pub normal_distribution: NormalDistributionParams,
}

#[derive(Params)]
pub struct MidiBpmDetectorParams {
    pub editor_state: Arc<EguiState>,

    #[id = "send_tempo"]
    pub send_tempo: BoolParam,

    #[nested(group = "GUI")]
    pub gui_params: GUIParams,
    #[nested(group = "StaticParams")]
    pub static_params: StaticParams,
    #[nested(group = "DynamicParams")]
    pub dynamic_params: DynamicParams,

    #[id = "daw_port"]
    pub daw_port: IntParam,
}

struct UpdaterFactory<'_self, Config> {
    current_sample: Arc<AtomicUsize>,
    changed_at: ArcAtomicOptional<usize>,
    config: &'_self Config,
}

impl<'_self, Config> UpdaterFactory<'_self, Config> {
    fn new(current_sample: Arc<AtomicUsize>, changed_at: ArcAtomicOptional<usize>, config: &'_self Config) -> Self {
        Self { current_sample, changed_at, config }
    }

    fn update_changed_at<T>(&self) -> Arc<dyn Fn(T) + Send + Sync>
    where
        T: 'static + Send,
    {
        let current_sample = self.current_sample.clone();
        let changed_at = self.changed_at.clone();
        Arc::new(move |_: T| {
            changed_at.store_if_none(Some(current_sample.load(Ordering::Relaxed)), Ordering::Relaxed);
        })
    }

    fn make_on_off_param(&self, parameter: &Parameter<Config, OnOff<f32>>) -> BoolParam {
        let current_sample = self.current_sample.clone();
        let changed_at = self.changed_at.clone();

        BoolParam::new(format!("{} enabled", parameter.label), (parameter.get)(self.config).is_enabled())
            .with_callback(Arc::new(move |_: bool| {
                changed_at.store_if_none(Some(current_sample.load(Ordering::Relaxed)), Ordering::Relaxed);
            }))
            .hide()
    }
}

#[allow(clippy::too_many_lines)]
impl MidiBpmDetectorParams {
    pub fn new(
        config: &mut Config,
        static_bpm_detection_parameters_changed_at: &ArcAtomicOptional<usize>,
        dynamic_bpm_detection_parameters_changed_at: &ArcAtomicOptional<usize>,
        current_sample: &Arc<AtomicUsize>,
        daw_port: &ArcAtomicOptional<u16>,
    ) -> Self {
        let static_updater_factory = UpdaterFactory::new(
            current_sample.clone(),
            static_bpm_detection_parameters_changed_at.clone(),
            &config.static_bpm_detection_parameters,
        );
        let dynamic_updater_factory = UpdaterFactory::new(
            current_sample.clone(),
            dynamic_bpm_detection_parameters_changed_at.clone(),
            &config.dynamic_bpm_detection_parameters,
        );
        let update_static_changed_at_f32 = static_updater_factory.update_changed_at();
        let update_static_changed_at_u16 = static_updater_factory.update_changed_at();
        let update_dynamic_changed_at_f32 = dynamic_updater_factory.update_changed_at();
        let update_dynamic_changed_at_u8 = dynamic_updater_factory.update_changed_at();

        Self {
            editor_state: EguiState::from_size(1200, 600),
            send_tempo: BoolParam::new("Send tempo", config.send_tempo.load(Ordering::Relaxed)).with_callback(
                Arc::new({
                    let send_tempo = config.send_tempo.clone();
                    move |value| {
                        send_tempo.store(value, Ordering::Relaxed);
                    }
                }),
            ),
            gui_params: GUIParams {
                interpolation_duration: GUIConfig::INTERPOLATION_DURATION
                    .to_param(&config.gui_config, &update_dynamic_changed_at_f32),
                interpolation_curve: GUIConfig::INTERPOLATION_CURVE
                    .to_param(&config.gui_config, &update_dynamic_changed_at_f32),
            },
            static_params: StaticParams {
                bpm_center: StaticBPMDetectionParameters::BPM_CENTER
                    .to_param(&config.static_bpm_detection_parameters, &update_static_changed_at_f32),
                bpm_range: StaticBPMDetectionParameters::BPM_RANGE
                    .to_param(&config.static_bpm_detection_parameters, &update_static_changed_at_u16),
                sample_rate: u16_range_to_logarithmic_param(
                    &StaticBPMDetectionParameters::SAMPLE_RATE,
                    &config.static_bpm_detection_parameters,
                    &update_static_changed_at_f32,
                ),
                normal_distribution: NormalDistributionParams {
                    std_dev: NormalDistributionConfig::STD_DEV.to_param(
                        &config.static_bpm_detection_parameters.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                    factor: NormalDistributionConfig::FACTOR.to_param(
                        &config.static_bpm_detection_parameters.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                    imprecision: NormalDistributionConfig::IMPRECISION.to_param(
                        &config.static_bpm_detection_parameters.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                    resolution: NormalDistributionConfig::RESOLUTION.to_param(
                        &config.static_bpm_detection_parameters.normal_distribution,
                        &update_static_changed_at_f32,
                    ),
                },
            },
            dynamic_params: DynamicParams {
                beats_lookback: DynamicBPMDetectionParameters::BEATS_LOOKBACK
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_u8),
                velocity_current_note_weight: DynamicBPMDetectionParameters::CURRENT_VELOCITY
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                velocity_current_note_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::CURRENT_VELOCITY),
                velocity_note_from_weight: DynamicBPMDetectionParameters::VELOCITY_FROM
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                velocity_note_from_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::VELOCITY_FROM),
                age_weight: DynamicBPMDetectionParameters::TIME_DISTANCE
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                age_onoff: dynamic_updater_factory.make_on_off_param(&DynamicBPMDetectionParameters::TIME_DISTANCE),
                octave_distance_weight: DynamicBPMDetectionParameters::OCTAVE_DISTANCE
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                octave_distance_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::OCTAVE_DISTANCE),
                pitch_distance_weight: DynamicBPMDetectionParameters::PITCH_DISTANCE
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                pitch_distance_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::PITCH_DISTANCE),
                multiplier_weight: DynamicBPMDetectionParameters::MULTIPLIER_FACTOR
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                multiplier_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::MULTIPLIER_FACTOR),
                subdivision_weight: DynamicBPMDetectionParameters::SUBDIVISION_FACTOR
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                subdivision_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::SUBDIVISION_FACTOR),
                in_beat_range_weight: DynamicBPMDetectionParameters::IN_RANGE
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                in_beat_range_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::IN_RANGE),
                normal_distribution_weight: DynamicBPMDetectionParameters::NORMAL_DISTRIBUTION
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                normal_distribution_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::NORMAL_DISTRIBUTION),
                high_tempo_bias: DynamicBPMDetectionParameters::HIGH_TEMPO_BIAS
                    .to_param(&config.dynamic_bpm_detection_parameters, &update_dynamic_changed_at_f32),
                high_tempo_bias_onoff: dynamic_updater_factory
                    .make_on_off_param(&DynamicBPMDetectionParameters::HIGH_TEMPO_BIAS),
            },
            daw_port: IntParam::new("DAW Port", 0, IntRange::Linear { min: 0, max: 65535 }).with_callback(Arc::new({
                let daw_port = daw_port.clone();
                move |value| {
                    daw_port.store(Some(value.to_u16().unwrap()), Ordering::Relaxed);
                }
            })),
        }
    }
}

pub trait ToParam<T> {
    type Param: Param;
    type ParamType;
    type Type;

    fn to_param(&self, config: &T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param;
}

pub fn apply_float_param<T, V>(parameter: &Parameter<T, V>, param: &FloatParam, config: &T, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, (parameter.get)(config).to_f32().unwrap());
    setter.end_set_parameter(param);
}

pub fn apply_onoff_param<T, V>(
    parameter: &Parameter<T, OnOff<V>>,
    enabled_param: &BoolParam,
    value_param: &FloatParam,
    config: &T,
    setter: &ParamSetter,
) where
    V: 'static + ToPrimitive + Copy + num_traits::One + num_traits::Zero + std::ops::Mul<Output = V>,
{
    setter.begin_set_parameter(value_param);
    setter.begin_set_parameter(enabled_param);
    setter.set_parameter(value_param, (parameter.get)(config).value().to_f32().unwrap());
    setter.set_parameter(enabled_param, (parameter.get)(config).is_enabled());
    setter.end_set_parameter(enabled_param);
    setter.end_set_parameter(value_param);
}

pub fn apply_int_param<T, V>(parameter: &Parameter<T, V>, param: &IntParam, config: &T, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, (parameter.get)(config).to_i32().unwrap());
    setter.end_set_parameter(param);
}

pub fn apply_duration_param<T>(
    parameter: &Parameter<T, Duration>,
    param: &FloatParam,
    config: &T,
    setter: &ParamSetter,
) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, (parameter.get)(config).as_secs_f32());
    setter.end_set_parameter(param);
}

macro_rules! impl_to_param_for_float {
    ($float_type:ty) => {
        impl<T> ToParam<T> for Parameter<T, $float_type> {
            type Param = FloatParam;
            type ParamType = f32;
            type Type = $float_type;

            fn to_param(&self, config: &T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
                let range = if self.logarithmic {
                    FloatRange::Skewed { min: *self.range.start() as f32, max: *self.range.end() as f32, factor: 0.3 }
                } else {
                    FloatRange::Linear { min: *self.range.start() as f32, max: *self.range.end() as f32 }
                };

                let mut param =
                    FloatParam::new(self.label, *(self.get)(config) as f32, range).with_callback(callback.clone());
                if let Some(unit) = self.unit {
                    param = param.with_unit(unit);
                }
                if self.step > 0.0 {
                    param = param.with_step_size(self.step as f32)
                }
                param = param.with_value_to_string(Arc::new(|value| format!("{:.2}", value)));
                param
            }
        }
    };
}

macro_rules! impl_to_param_for_integer {
    ($int_type:ty) => {
        impl<T> ToParam<T> for Parameter<T, $int_type> {
            type Param = IntParam;
            type ParamType = i32;
            type Type = i32;

            fn to_param(&self, config: &T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
                let mut param = IntParam::new(
                    self.label,
                    i32::from(*(self.get)(config)),
                    IntRange::Linear { min: *self.range.start() as i32, max: *self.range.end() as i32 },
                )
                .with_callback(callback.clone());
                if let Some(unit) = self.unit {
                    param = param.with_unit(unit);
                }
                param
            }
        }
    };
}

fn build_float_param<T, V>(
    param: &Parameter<T, V>,
    config: &T,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
    extract_value: impl Fn(&V) -> f32,
) -> FloatParam {
    let range = if param.logarithmic {
        FloatRange::Skewed { min: *param.range.start() as f32, max: *param.range.end() as f32, factor: 0.3 }
    } else {
        FloatRange::Linear { min: *param.range.start() as f32, max: *param.range.end() as f32 }
    };

    let mut float_param =
        FloatParam::new(param.label, extract_value((param.get)(config)), range).with_callback(callback.clone());

    if let Some(unit) = param.unit {
        float_param = float_param.with_unit(unit);
    }
    if param.step > 0.0 {
        float_param = float_param.with_step_size(param.step as f32);
    }

    float_param.with_value_to_string(Arc::new(|value| format!("{value:.2}")))
}

impl<T> ToParam<T> for Parameter<T, Duration> {
    type Param = FloatParam;
    type ParamType = f32;
    type Type = f32;

    fn to_param(&self, config: &T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        build_float_param(self, config, callback, Duration::as_secs_f32)
    }
}

impl<T> ToParam<T> for Parameter<T, OnOff<f32>> {
    type Param = FloatParam;
    type ParamType = f32;
    type Type = f32;

    fn to_param(&self, config: &T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        build_float_param(self, config, callback, OnOff::value)
    }
}

impl_to_param_for_float!(f32);
impl_to_param_for_float!(f64);

impl_to_param_for_integer!(u16);
impl_to_param_for_integer!(u8);

pub fn u16_range_to_logarithmic_param<T>(
    parameter: &Parameter<T, u16>,
    config: &T,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
    let mut param = FloatParam::new(
        parameter.label,
        f32::from(*(parameter.get)(config)),
        FloatRange::Skewed { min: *parameter.range.start() as f32, max: *parameter.range.end() as f32, factor: 0.3 },
    )
    .with_callback(callback.clone());
    if let Some(unit) = parameter.unit {
        param = param.with_unit(unit);
    }
    param = param.with_step_size(parameter.step.max(1.0) as f32);
    if let Some(unit) = parameter.unit {
        param = param.with_unit(unit);
    }
    param
}
