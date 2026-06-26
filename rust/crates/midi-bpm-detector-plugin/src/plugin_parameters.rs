use std::{
    num::NonZeroU16,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};

use bpm_detection_core::parameters::{
    DefaultDynamicBPMDetectionParameters, DefaultNormalDistributionParameters, DefaultStaticBPMDetectionParameters,
    DynamicBPMDetectionConfig, DynamicBPMDetectionParameterVisitor,
};
use gui::DefaultGUIParameters;
use nih_plug::{
    params::{BoolParam, FloatParam, IntParam, Param, Params},
    prelude::{FloatRange, IntRange, ParamSetter, RemoteControlsPage},
};
use nih_plug_egui::EguiState;
use num_traits::ToPrimitive;
use parameter::{OnOff, Parameter};
use sync::ArcAtomicOptionNonZeroU16;

use crate::{DeferredConfigUpdate, bpm_detector_configuration::PluginConfig};

#[derive(Params)]
pub struct PluginGUIParams {
    #[id = "interpolation_duration"]
    pub interpolation_duration: FloatParam,
    #[id = "interpolation_curve"]
    pub interpolation_curve: FloatParam,
}

#[derive(Params)]
pub struct PluginDynamicParams {
    #[id = "beats_lookback"]
    pub beats_lookback: IntParam,
    #[id = "velocity_current_note_weight"]
    pub velocity_current_note_weight: FloatParam,
    #[persist = "velocity_current_note_onoff"]
    pub velocity_current_note_onoff: AtomicBool,
    #[id = "velocity_note_from_weight"]
    pub velocity_note_from_weight: FloatParam,
    #[persist = "velocity_note_from_onoff"]
    pub velocity_note_from_onoff: AtomicBool,
    #[id = "time_distance_weight"]
    pub time_distance_weight: FloatParam,
    #[persist = "time_distance_onoff"]
    pub time_distance_onoff: AtomicBool,
    #[id = "octave_distance_weight"]
    pub octave_distance_weight: FloatParam,
    #[persist = "octave_distance_onoff"]
    pub octave_distance_onoff: AtomicBool,
    #[id = "pitch_distance_weight"]
    pub pitch_distance_weight: FloatParam,
    #[persist = "pitch_distance_onoff"]
    pub pitch_distance_onoff: AtomicBool,
    #[id = "multiplier_weight"]
    pub multiplier_weight: FloatParam,
    #[persist = "multiplier_onoff"]
    pub multiplier_onoff: AtomicBool,
    #[id = "subdivision_weight"]
    pub subdivision_weight: FloatParam,
    #[persist = "subdivision_onoff"]
    pub subdivision_onoff: AtomicBool,
    #[id = "in_beat_range_weight"]
    pub in_beat_range_weight: FloatParam,
    #[persist = "in_beat_range_onoff"]
    pub in_beat_range_onoff: AtomicBool,
    #[id = "normal_distribution_weight"]
    pub normal_distribution_weight: FloatParam,
    #[persist = "normal_distribution_onoff"]
    pub normal_distribution_onoff: AtomicBool,
    #[id = "high_tempo_bias"]
    pub high_tempo_bias: FloatParam,
    #[persist = "high_tempo_bias_onoff"]
    pub high_tempo_bias_onoff: AtomicBool,
}

#[derive(Params)]
pub struct NormalDistributionParams {
    #[id = "std_dev"]
    pub std_dev: FloatParam,
    #[id = "factor"]
    pub factor: FloatParam,
    #[id = "cutoff"]
    pub cutoff: FloatParam,
    #[id = "resolution"]
    pub resolution: FloatParam,
}

#[derive(Params)]
pub struct PluginStaticParams {
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
    pub gui_params: PluginGUIParams,
    #[nested(group = "StaticParams")]
    pub static_params: PluginStaticParams,
    #[nested(group = "DynamicParams")]
    pub dynamic_params: PluginDynamicParams,

    #[id = "daw_port"]
    pub daw_port: IntParam,
}

impl PluginDynamicParams {
    pub(crate) fn add_remote_controls(&self, page: &mut impl RemoteControlsPage) {
        let mut visitor = DynamicRemoteControlParams { params: self, page };

        DefaultDynamicBPMDetectionParameters::visit(&mut visitor);
    }

    pub(crate) fn read_dynamic_config(&self) -> DynamicBPMDetectionConfig {
        let mut config = DynamicBPMDetectionConfig::default();
        let mut visitor = DynamicHostConfigReader { params: self, config: &mut config };

        DefaultDynamicBPMDetectionParameters::visit(&mut visitor);

        config
    }
}

struct DynamicRemoteControlParams<'params, 'page, Page> {
    params: &'params PluginDynamicParams,
    page: &'page mut Page,
}

impl<Page: RemoteControlsPage> DynamicBPMDetectionParameterVisitor<()> for DynamicRemoteControlParams<'_, '_, Page> {
    fn beats_lookback(&mut self, _parameter: Parameter<(), u8>) {
        self.page.add_param(&self.params.beats_lookback);
    }

    fn velocity_current_note_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.velocity_current_note_weight);
    }

    fn high_tempo_bias(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.high_tempo_bias);
    }

    fn in_beat_range_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.in_beat_range_weight);
    }

    fn multiplier_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.multiplier_weight);
    }

    fn normal_distribution_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.normal_distribution_weight);
    }

    fn octave_distance_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.octave_distance_weight);
    }

    fn pitch_distance_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.pitch_distance_weight);
    }

    fn subdivision_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.subdivision_weight);
    }

    fn time_distance_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.time_distance_weight);
    }

    fn velocity_note_from_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.page.add_param(&self.params.velocity_note_from_weight);
    }
}

struct DynamicHostConfigReader<'params, 'config> {
    params: &'params PluginDynamicParams,
    config: &'config mut DynamicBPMDetectionConfig,
}

impl DynamicBPMDetectionParameterVisitor<()> for DynamicHostConfigReader<'_, '_> {
    fn beats_lookback(&mut self, _parameter: Parameter<(), u8>) {
        self.config.beats_lookback = self.params.beats_lookback.unmodulated_plain_value() as u8;
    }

    fn velocity_current_note_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.velocity_current_note_weight = OnOff::new(
            self.params.velocity_current_note_onoff.load(Ordering::Relaxed),
            self.params.velocity_current_note_weight.unmodulated_plain_value(),
        );
    }

    fn high_tempo_bias(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.high_tempo_bias = OnOff::new(
            self.params.high_tempo_bias_onoff.load(Ordering::Relaxed),
            self.params.high_tempo_bias.unmodulated_plain_value(),
        );
    }

    fn in_beat_range_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.in_beat_range_weight = OnOff::new(
            self.params.in_beat_range_onoff.load(Ordering::Relaxed),
            self.params.in_beat_range_weight.unmodulated_plain_value(),
        );
    }

    fn multiplier_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.multiplier_weight = OnOff::new(
            self.params.multiplier_onoff.load(Ordering::Relaxed),
            self.params.multiplier_weight.unmodulated_plain_value(),
        );
    }

    fn normal_distribution_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.normal_distribution_weight = OnOff::new(
            self.params.normal_distribution_onoff.load(Ordering::Relaxed),
            self.params.normal_distribution_weight.unmodulated_plain_value(),
        );
    }

    fn octave_distance_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.octave_distance_weight = OnOff::new(
            self.params.octave_distance_onoff.load(Ordering::Relaxed),
            self.params.octave_distance_weight.unmodulated_plain_value(),
        );
    }

    fn pitch_distance_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.pitch_distance_weight = OnOff::new(
            self.params.pitch_distance_onoff.load(Ordering::Relaxed),
            self.params.pitch_distance_weight.unmodulated_plain_value(),
        );
    }

    fn subdivision_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.subdivision_weight = OnOff::new(
            self.params.subdivision_onoff.load(Ordering::Relaxed),
            self.params.subdivision_weight.unmodulated_plain_value(),
        );
    }

    fn time_distance_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.time_distance_weight = OnOff::new(
            self.params.time_distance_onoff.load(Ordering::Relaxed),
            self.params.time_distance_weight.unmodulated_plain_value(),
        );
    }

    fn velocity_note_from_weight(&mut self, _parameter: Parameter<(), OnOff<f32>>) {
        self.config.velocity_note_from_weight = OnOff::new(
            self.params.velocity_note_from_onoff.load(Ordering::Relaxed),
            self.params.velocity_note_from_weight.unmodulated_plain_value(),
        );
    }
}

struct UpdaterFactory {
    current_sample: Arc<AtomicUsize>,
    changed_at: DeferredConfigUpdate,
}

impl UpdaterFactory {
    fn new(current_sample: Arc<AtomicUsize>, changed_at: DeferredConfigUpdate) -> Self {
        Self { current_sample, changed_at }
    }

    fn update_changed_at<T>(&self) -> Arc<dyn Fn(T) + Send + Sync>
    where
        T: 'static + Send,
    {
        let current_sample = self.current_sample.clone();
        let changed_at = self.changed_at.clone();
        Arc::new(move |_: T| {
            changed_at.mark_changed_at_if_idle(current_sample.load(Ordering::Relaxed));
        })
    }
}

#[allow(clippy::too_many_lines)]
impl MidiBpmDetectorParams {
    pub fn new(
        config: &mut PluginConfig,
        static_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        dynamic_bpm_detection_config_changed_at: &DeferredConfigUpdate,
        current_sample: &Arc<AtomicUsize>,
        daw_port: &ArcAtomicOptionNonZeroU16,
    ) -> Self {
        let static_updater_factory =
            UpdaterFactory::new(current_sample.clone(), static_bpm_detection_config_changed_at.clone());
        let dynamic_updater_factory =
            UpdaterFactory::new(current_sample.clone(), dynamic_bpm_detection_config_changed_at.clone());
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
            gui_params: PluginGUIParams {
                interpolation_duration: DefaultGUIParameters::INTERPOLATION_DURATION
                    .to_param(config.gui_config.interpolation_duration, &update_dynamic_changed_at_f32),
                interpolation_curve: DefaultGUIParameters::INTERPOLATION_CURVE
                    .to_param(config.gui_config.interpolation_curve, &update_dynamic_changed_at_f32),
            },
            static_params: PluginStaticParams {
                bpm_center: DefaultStaticBPMDetectionParameters::BPM_CENTER
                    .to_param(config.static_bpm_detection_config.bpm_center, &update_static_changed_at_f32),
                bpm_range: DefaultStaticBPMDetectionParameters::BPM_RANGE
                    .to_param(config.static_bpm_detection_config.bpm_range, &update_static_changed_at_u16),
                sample_rate: u16_range_to_logarithmic_param(
                    &DefaultStaticBPMDetectionParameters::SAMPLE_RATE,
                    config.static_bpm_detection_config.sample_rate,
                    &update_static_changed_at_f32,
                ),
                normal_distribution: NormalDistributionParams {
                    std_dev: DefaultNormalDistributionParameters::STD_DEV.to_param(
                        config.static_bpm_detection_config.normal_distribution.std_dev,
                        &update_static_changed_at_f32,
                    ),
                    factor: DefaultNormalDistributionParameters::FACTOR.to_param(
                        config.static_bpm_detection_config.normal_distribution.factor,
                        &update_static_changed_at_f32,
                    ),
                    cutoff: DefaultNormalDistributionParameters::CUTOFF.to_param(
                        config.static_bpm_detection_config.normal_distribution.cutoff,
                        &update_static_changed_at_f32,
                    ),
                    resolution: DefaultNormalDistributionParameters::RESOLUTION.to_param(
                        config.static_bpm_detection_config.normal_distribution.resolution,
                        &update_static_changed_at_f32,
                    ),
                },
            },
            dynamic_params: PluginDynamicParams {
                beats_lookback: DefaultDynamicBPMDetectionParameters::BEATS_LOOKBACK
                    .to_param(config.dynamic_bpm_detection_config.beats_lookback, &update_dynamic_changed_at_u8),
                velocity_current_note_weight: DefaultDynamicBPMDetectionParameters::VELOCITY_CURRENT_NOTE_WEIGHT
                    .to_param(
                        config.dynamic_bpm_detection_config.velocity_current_note_weight,
                        &update_dynamic_changed_at_f32,
                    ),
                velocity_current_note_onoff: config
                    .dynamic_bpm_detection_config
                    .velocity_current_note_weight
                    .is_enabled()
                    .into(),
                velocity_note_from_weight: DefaultDynamicBPMDetectionParameters::VELOCITY_NOTE_FROM_WEIGHT.to_param(
                    config.dynamic_bpm_detection_config.velocity_note_from_weight,
                    &update_dynamic_changed_at_f32,
                ),
                velocity_note_from_onoff: config
                    .dynamic_bpm_detection_config
                    .velocity_note_from_weight
                    .is_enabled()
                    .into(),
                time_distance_weight: DefaultDynamicBPMDetectionParameters::TIME_DISTANCE_WEIGHT
                    .to_param(config.dynamic_bpm_detection_config.time_distance_weight, &update_dynamic_changed_at_f32),
                time_distance_onoff: config.dynamic_bpm_detection_config.time_distance_weight.is_enabled().into(),
                octave_distance_weight: DefaultDynamicBPMDetectionParameters::OCTAVE_DISTANCE_WEIGHT.to_param(
                    config.dynamic_bpm_detection_config.octave_distance_weight,
                    &update_dynamic_changed_at_f32,
                ),
                octave_distance_onoff: config.dynamic_bpm_detection_config.octave_distance_weight.is_enabled().into(),
                pitch_distance_weight: DefaultDynamicBPMDetectionParameters::PITCH_DISTANCE_WEIGHT.to_param(
                    config.dynamic_bpm_detection_config.pitch_distance_weight,
                    &update_dynamic_changed_at_f32,
                ),
                pitch_distance_onoff: config.dynamic_bpm_detection_config.pitch_distance_weight.is_enabled().into(),
                multiplier_weight: DefaultDynamicBPMDetectionParameters::MULTIPLIER_WEIGHT
                    .to_param(config.dynamic_bpm_detection_config.multiplier_weight, &update_dynamic_changed_at_f32),
                multiplier_onoff: config.dynamic_bpm_detection_config.multiplier_weight.is_enabled().into(),
                subdivision_weight: DefaultDynamicBPMDetectionParameters::SUBDIVISION_WEIGHT
                    .to_param(config.dynamic_bpm_detection_config.subdivision_weight, &update_dynamic_changed_at_f32),
                subdivision_onoff: config.dynamic_bpm_detection_config.subdivision_weight.is_enabled().into(),
                in_beat_range_weight: DefaultDynamicBPMDetectionParameters::IN_BEAT_RANGE_WEIGHT
                    .to_param(config.dynamic_bpm_detection_config.in_beat_range_weight, &update_dynamic_changed_at_f32),
                in_beat_range_onoff: config.dynamic_bpm_detection_config.in_beat_range_weight.is_enabled().into(),
                normal_distribution_weight: DefaultDynamicBPMDetectionParameters::NORMAL_DISTRIBUTION_WEIGHT.to_param(
                    config.dynamic_bpm_detection_config.normal_distribution_weight,
                    &update_dynamic_changed_at_f32,
                ),
                normal_distribution_onoff: config
                    .dynamic_bpm_detection_config
                    .normal_distribution_weight
                    .is_enabled()
                    .into(),
                high_tempo_bias: DefaultDynamicBPMDetectionParameters::HIGH_TEMPO_BIAS
                    .to_param(config.dynamic_bpm_detection_config.high_tempo_bias, &update_dynamic_changed_at_f32),
                high_tempo_bias_onoff: config.dynamic_bpm_detection_config.high_tempo_bias.is_enabled().into(),
            },
            daw_port: IntParam::new("DAW Port", 0, IntRange::Linear { min: 0, max: 65535 }).with_callback(Arc::new({
                let daw_port = daw_port.clone();
                move |value| {
                    daw_port.store(NonZeroU16::new(value.to_u16().unwrap()), Ordering::Relaxed);
                }
            })),
        }
    }
}

pub trait ToParam<ValueType> {
    type Param: Param;
    type ParamType;
    type Type;

    fn to_param(&self, val: ValueType, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param;
}

pub fn apply_float_param<V>(param: &FloatParam, value: V, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value.to_f32().unwrap());
    setter.end_set_parameter(param);
}

pub fn apply_onoff_param(
    value_param: &FloatParam,
    enabled: &AtomicBool,
    previous_value: OnOff<f32>,
    value: OnOff<f32>,
    setter: &ParamSetter,
) {
    enabled.store(value.is_enabled(), Ordering::Relaxed);
    if (previous_value.value() - value.value()).abs() > f32::EPSILON {
        setter.begin_set_parameter(value_param);
        setter.set_parameter(value_param, value.value());
        setter.end_set_parameter(value_param);
    }
}

pub fn apply_int_param<V>(param: &IntParam, value: V, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value.to_i32().unwrap());
    setter.end_set_parameter(param);
}

pub fn apply_duration_param(param: &FloatParam, value: Duration, setter: &ParamSetter) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value.as_secs_f32());
    setter.end_set_parameter(param);
}

macro_rules! impl_to_param_for_float {
    ($float_type:ty) => {
        impl ToParam<$float_type> for Parameter<(), $float_type> {
            type Param = FloatParam;
            type ParamType = f32;
            type Type = $float_type;

            fn to_param(&self, val: $float_type, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
                let range = if self.logarithmic {
                    FloatRange::Skewed { min: *self.range.start() as f32, max: *self.range.end() as f32, factor: 0.3 }
                } else {
                    FloatRange::Linear { min: *self.range.start() as f32, max: *self.range.end() as f32 }
                };

                let mut param = FloatParam::new(self.label, val as f32, range).with_callback(callback.clone());
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
        impl ToParam<$int_type> for Parameter<(), $int_type> {
            type Param = IntParam;
            type ParamType = i32;
            type Type = i32;

            fn to_param(&self, val: $int_type, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
                let mut param = IntParam::new(
                    self.label,
                    i32::from(val),
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

fn build_float_param<ValueType>(
    param: &Parameter<(), ValueType>,
    val: f32,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
    let range = if param.logarithmic {
        FloatRange::Skewed { min: *param.range.start() as f32, max: *param.range.end() as f32, factor: 0.3 }
    } else {
        FloatRange::Linear { min: *param.range.start() as f32, max: *param.range.end() as f32 }
    };

    let mut float_param = FloatParam::new(param.label, val, range).with_callback(callback.clone());

    if let Some(unit) = param.unit {
        float_param = float_param.with_unit(unit);
    }
    if param.step > 0.0 {
        float_param = float_param.with_step_size(param.step as f32);
    }

    float_param.with_value_to_string(Arc::new(|value| format!("{value:.2}")))
}

impl ToParam<Duration> for Parameter<(), Duration> {
    type Param = FloatParam;
    type ParamType = f32;
    type Type = f32;

    fn to_param(&self, val: Duration, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        build_float_param(self, val.as_secs_f32(), callback)
    }
}

impl ToParam<OnOff<f32>> for Parameter<(), OnOff<f32>> {
    type Param = FloatParam;
    type ParamType = f32;
    type Type = f32;

    fn to_param(&self, val: OnOff<f32>, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        build_float_param(self, val.value(), callback)
    }
}

impl_to_param_for_float!(f32);
impl_to_param_for_float!(f64);

impl_to_param_for_integer!(u16);
impl_to_param_for_integer!(u8);

pub fn u16_range_to_logarithmic_param(
    parameter: &Parameter<(), u16>,
    val: u16,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
    let mut param = FloatParam::new(
        parameter.label,
        f32::from(val),
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

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use bpm_detection_core::parameters::DynamicBPMDetectionConfig;
    use nih_plug::prelude::RemoteControlsPage;

    use super::*;
    use crate::DeferredConfigUpdate;

    struct RemoteControlNames(Vec<String>);

    impl RemoteControlsPage for RemoteControlNames {
        fn add_param(&mut self, param: &impl Param) {
            self.0.push(param.name().to_owned());
        }

        fn add_spacer(&mut self) {}
    }

    #[test]
    fn plugin_on_off_params_initialize_enabled_state_from_matching_config_field() {
        let mut config = PluginConfig {
            dynamic_bpm_detection_config: DynamicBPMDetectionConfig {
                velocity_current_note_weight: OnOff::On(1.0),
                velocity_note_from_weight: OnOff::Off(2.0),
                ..DynamicBPMDetectionConfig::default()
            },
            ..PluginConfig::default()
        };
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();

        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);

        assert!(params.dynamic_params.velocity_current_note_onoff.load(Ordering::Relaxed));
        assert!(!params.dynamic_params.velocity_note_from_onoff.load(Ordering::Relaxed));
    }

    #[test]
    fn dynamic_remote_controls_expose_every_dynamic_parameter() {
        let mut config = PluginConfig::default();
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();
        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);
        let mut remote_controls = RemoteControlNames(Vec::new());

        params.dynamic_params.add_remote_controls(&mut remote_controls);

        assert_eq!(
            remote_controls.0,
            [
                "Beats Lookback",
                "Note velocity",
                "From note velocity",
                "Time distance",
                "Octave distance",
                "Pitch distance",
                "Multiplier",
                "Subdivision",
                "In beat range",
                "Normal distribution",
                "High tempo bias",
            ]
        );
    }

    #[test]
    fn dynamic_params_read_initialized_host_values_as_dynamic_config() {
        let source_dynamic_config = DynamicBPMDetectionConfig {
            beats_lookback: 13,
            velocity_current_note_weight: OnOff::On(1.1),
            velocity_note_from_weight: OnOff::Off(1.2),
            time_distance_weight: OnOff::On(1.3),
            octave_distance_weight: OnOff::Off(1.4),
            pitch_distance_weight: OnOff::On(1.5),
            multiplier_weight: OnOff::Off(1.6),
            subdivision_weight: OnOff::On(1.7),
            in_beat_range_weight: OnOff::Off(1.8),
            normal_distribution_weight: OnOff::On(0.9),
            high_tempo_bias: OnOff::Off(2.1),
        };
        let mut config =
            PluginConfig { dynamic_bpm_detection_config: source_dynamic_config.clone(), ..PluginConfig::default() };
        let current_sample = Arc::new(AtomicUsize::new(0));
        let changed_at = DeferredConfigUpdate::idle();
        let daw_port = ArcAtomicOptionNonZeroU16::none();
        let params = MidiBpmDetectorParams::new(&mut config, &changed_at, &changed_at, &current_sample, &daw_port);

        assert_eq!(params.dynamic_params.read_dynamic_config(), source_dynamic_config);
    }
}
