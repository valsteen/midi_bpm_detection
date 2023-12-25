use crate::config::Config;
use gui::GUIConfig;
use midi::{DynamicBPMDetectionParameters, NormalDistributionConfig, StaticBPMDetectionParameters};
use nih_plug::{
    params::{BoolParam, FloatParam, IntParam, Param, Params},
    prelude::{FloatRange, IntRange, ParamSetter},
};
use nih_plug_egui::EguiState;
use num_traits::ToPrimitive;
use parameter::{OnOff, Parameter};
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use sync::ArcAtomicOptional;

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
    #[id = "velocity_note_from_weight"]
    pub velocity_note_from_weight: FloatParam,
    #[id = "age_weight"]
    pub age_weight: FloatParam,
    #[id = "octave_distance_weight"]
    pub octave_distance_weight: FloatParam,
    #[id = "pitch_distance_weight"]
    pub pitch_distance_weight: FloatParam,
    #[id = "multiplier_weight"]
    pub multiplier_weight: FloatParam,
    #[id = "subdivision_weight"]
    pub subdivision_weight: FloatParam,
    #[id = "in_beat_range_weight"]
    pub in_beat_range_weight: FloatParam,
    #[id = "normal_distribution_weight"]
    pub normal_distribution_weight: FloatParam,
    #[id = "high_tempo_bias"]
    pub high_tempo_bias: FloatParam,
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

#[allow(clippy::too_many_lines)]
impl MidiBpmDetectorParams {
    pub fn new(
        config: &mut Config,
        static_bpm_detection_parameters_changed_at: ArcAtomicOptional<usize>,
        dynamic_bpm_detection_parameters_changed_at: ArcAtomicOptional<usize>,
        current_sample: Arc<AtomicUsize>,
        daw_port: ArcAtomicOptional<u16>,
    ) -> Self {
        let static_parameters_change_f32: Arc<dyn Fn(f32) + Send + Sync> = Arc::new({
            let static_bpm_detection_parameters_changed_at = static_bpm_detection_parameters_changed_at.clone();
            let current_sample = current_sample.clone();
            move |_: f32| {
                static_bpm_detection_parameters_changed_at
                    .store_if_none(Some(current_sample.load(Ordering::Relaxed)), Ordering::Relaxed);
            }
        });
        let static_parameters_change_u16: Arc<dyn Fn(i32) + Send + Sync> = Arc::new({
            let current_sample = current_sample.clone();
            move |_: i32| {
                static_bpm_detection_parameters_changed_at
                    .store_if_none(Some(current_sample.load(Ordering::Relaxed)), Ordering::Relaxed);
            }
        });
        let dynamic_parameters_change_f32: Arc<dyn Fn(f32) + Send + Sync> = Arc::new({
            let dynamic_bpm_detection_parameters_changed_at = dynamic_bpm_detection_parameters_changed_at.clone();
            let current_sample = current_sample.clone();
            move |_: f32| {
                dynamic_bpm_detection_parameters_changed_at
                    .store_if_none(Some(current_sample.load(Ordering::Relaxed)), Ordering::Relaxed);
            }
        });
        let dynamic_parameters_change_u8: Arc<dyn Fn(i32) + Send + Sync> = Arc::new({
            move |_: i32| {
                dynamic_bpm_detection_parameters_changed_at
                    .store_if_none(Some(current_sample.load(Ordering::Relaxed)), Ordering::Relaxed);
            }
        });

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
                    .to_param(&mut config.gui_config, &dynamic_parameters_change_f32),
                interpolation_curve: GUIConfig::INTERPOLATION_CURVE
                    .to_param(&mut config.gui_config, &dynamic_parameters_change_f32),
            },
            static_params: StaticParams {
                bpm_center: StaticBPMDetectionParameters::BPM_CENTER
                    .to_param(&mut config.static_bpm_detection_parameters, &static_parameters_change_f32),
                bpm_range: StaticBPMDetectionParameters::BPM_RANGE
                    .to_param(&mut config.static_bpm_detection_parameters, &static_parameters_change_u16),
                sample_rate: u16_range_to_logarithmic_param(
                    &StaticBPMDetectionParameters::SAMPLE_RATE,
                    &mut config.static_bpm_detection_parameters,
                    &static_parameters_change_f32,
                ),
                normal_distribution: NormalDistributionParams {
                    std_dev: NormalDistributionConfig::STD_DEV.to_param(
                        &mut config.static_bpm_detection_parameters.normal_distribution,
                        &static_parameters_change_f32,
                    ),
                    factor: NormalDistributionConfig::FACTOR.to_param(
                        &mut config.static_bpm_detection_parameters.normal_distribution,
                        &static_parameters_change_f32,
                    ),
                    imprecision: NormalDistributionConfig::IMPRECISION.to_param(
                        &mut config.static_bpm_detection_parameters.normal_distribution,
                        &static_parameters_change_f32,
                    ),
                    resolution: NormalDistributionConfig::RESOLUTION.to_param(
                        &mut config.static_bpm_detection_parameters.normal_distribution,
                        &static_parameters_change_f32,
                    ),
                },
            },
            dynamic_params: DynamicParams {
                beats_lookback: DynamicBPMDetectionParameters::BEATS_LOOKBACK
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_u8),
                velocity_current_note_weight: DynamicBPMDetectionParameters::CURRENT_VELOCITY
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                velocity_note_from_weight: DynamicBPMDetectionParameters::VELOCITY_FROM
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                age_weight: DynamicBPMDetectionParameters::TIME_DISTANCE
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                octave_distance_weight: DynamicBPMDetectionParameters::OCTAVE_DISTANCE
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                pitch_distance_weight: DynamicBPMDetectionParameters::PITCH_DISTANCE
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                multiplier_weight: DynamicBPMDetectionParameters::MULTIPLIER_FACTOR
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                subdivision_weight: DynamicBPMDetectionParameters::SUBDIVISION_FACTOR
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                in_beat_range_weight: DynamicBPMDetectionParameters::IN_RANGE
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                normal_distribution_weight: DynamicBPMDetectionParameters::NORMAL_DISTRIBUTION
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
                high_tempo_bias: DynamicBPMDetectionParameters::HIGH_TEMPO_BIAS
                    .to_param(&mut config.dynamic_bpm_detection_parameters, &dynamic_parameters_change_f32),
            },
            daw_port: IntParam::new("DAW Port", 0, IntRange::Linear { min: 0, max: 65535 }).with_callback(Arc::new(
                move |value| {
                    daw_port.store(Some(value.to_u16().unwrap()), Ordering::Relaxed);
                },
            )),
        }
    }
}

pub trait ToParam<T> {
    type Param: Param;
    type ParamType;
    type Type;

    fn to_param(&self, config: &mut T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param;
}

pub fn apply_float_param<T, V>(parameter: &Parameter<T, V>, param: &FloatParam, config: &mut T, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, (parameter.get_mut)(config).to_f32().unwrap());
    setter.end_set_parameter(param);
}

pub fn apply_onoff_param<T, V>(
    parameter: &Parameter<T, OnOff<V>>,
    param: &FloatParam,
    config: &mut T,
    setter: &ParamSetter,
) where
    V: 'static + ToPrimitive + Copy + num_traits::One + num_traits::Zero + std::ops::Mul<Output = V>,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, (parameter.get_mut)(config).weight().to_f32().unwrap());
    setter.end_set_parameter(param);
}

pub fn apply_int_param<T, V>(parameter: &Parameter<T, V>, param: &IntParam, config: &mut T, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, (parameter.get_mut)(config).to_i32().unwrap());
    setter.end_set_parameter(param);
}

pub fn apply_duration_param<T>(
    parameter: &Parameter<T, Duration>,
    param: &FloatParam,
    config: &mut T,
    setter: &ParamSetter,
) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, (parameter.get_mut)(config).as_secs_f32());
    setter.end_set_parameter(param);
}

macro_rules! impl_to_param_for_float {
    ($float_type:ty) => {
        impl<T> ToParam<T> for Parameter<T, $float_type> {
            type Param = FloatParam;
            type ParamType = f32;
            type Type = $float_type;

            fn to_param(&self, config: &mut T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
                let range = if self.logarithmic {
                    FloatRange::Skewed { min: *self.range.start() as f32, max: *self.range.end() as f32, factor: 0.3 }
                } else {
                    FloatRange::Linear { min: *self.range.start() as f32, max: *self.range.end() as f32 }
                };

                let mut param =
                    FloatParam::new(self.label, *(self.get_mut)(config) as f32, range).with_callback(callback.clone());
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

            fn to_param(&self, config: &mut T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
                let mut param = IntParam::new(
                    self.label,
                    i32::from(*(self.get_mut)(config)),
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

impl<T> ToParam<T> for Parameter<T, Duration> {
    type Param = FloatParam;
    type ParamType = f32;
    type Type = f32;

    fn to_param(&self, config: &mut T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        let range = if self.logarithmic {
            FloatRange::Skewed { min: *self.range.start() as f32, max: *self.range.end() as f32, factor: 0.3 }
        } else {
            FloatRange::Linear { min: *self.range.start() as f32, max: *self.range.end() as f32 }
        };

        let mut param =
            FloatParam::new(self.label, (self.get_mut)(config).as_secs_f32(), range).with_callback(callback.clone());
        if let Some(unit) = self.unit {
            param = param.with_unit(unit);
        }
        if self.step > 0.0 {
            param = param.with_step_size(self.step as f32);
        }
        param = param.with_value_to_string(Arc::new(|value| format!("{value:.2}")));
        param
    }
}

impl<T> ToParam<T> for Parameter<T, OnOff<f32>> {
    type Param = FloatParam;
    type ParamType = f32;
    type Type = f32;

    fn to_param(&self, config: &mut T, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        let range = if self.logarithmic {
            FloatRange::Skewed { min: *self.range.start() as f32, max: *self.range.end() as f32, factor: 0.3 }
        } else {
            FloatRange::Linear { min: *self.range.start() as f32, max: *self.range.end() as f32 }
        };

        let mut param =
            FloatParam::new(self.label, (self.get_mut)(config).weight(), range).with_callback(callback.clone());
        if let Some(unit) = self.unit {
            param = param.with_unit(unit);
        }
        if self.step > 0.0 {
            param = param.with_step_size(self.step as f32);
        }
        param = param.with_value_to_string(Arc::new(|value| format!("{value:.2}")));
        param
    }
}

impl_to_param_for_float!(f32);
impl_to_param_for_float!(f64);

impl_to_param_for_integer!(u16);
impl_to_param_for_integer!(u8);

pub fn u16_range_to_logarithmic_param<T>(
    parameter: &Parameter<T, u16>,
    config: &mut T,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
    let mut param = FloatParam::new(
        parameter.label,
        f32::from(*(parameter.get_mut)(config)),
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
