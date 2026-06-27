use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use bpm_detection_core::parameters::DynamicBPMDetectionConfigAccessor;
use nih_plug::{
    params::{FloatParam, IntParam, Param, Params, persist},
    prelude::{FloatRange, IntRange, ParamPtr, ParamSetter},
};
use num_traits::ToPrimitive;
use parameter::{OnOff, Parameter};

pub struct PluginOnOffParam {
    id: &'static str,
    value: FloatParam,
    enabled_key: String,
    enabled: AtomicBool,
}

impl PluginOnOffParam {
    pub(crate) fn new(id: &'static str, value: FloatParam, initial_value: OnOff<f32>) -> Self {
        Self { id, value, enabled_key: format!("{id}_onoff"), enabled: AtomicBool::new(initial_value.is_enabled()) }
    }

    pub fn param(&self) -> &FloatParam {
        &self.value
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn read(&self) -> OnOff<f32> {
        OnOff::new(self.is_enabled(), self.value.unmodulated_plain_value())
    }

    pub(crate) fn apply(&self, previous_value: OnOff<f32>, value: OnOff<f32>, setter: &ParamSetter) {
        self.enabled.store(value.is_enabled(), Ordering::Relaxed);
        if (previous_value.value() - value.value()).abs() > f32::EPSILON {
            setter.begin_set_parameter(&self.value);
            setter.set_parameter(&self.value, value.value());
            setter.end_set_parameter(&self.value);
        }
    }
}

unsafe impl Params for PluginOnOffParam {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        vec![(String::from(self.id), self.value.as_ptr(), String::new())]
    }

    fn serialize_fields(&self) -> std::collections::BTreeMap<String, String> {
        let mut serialized = std::collections::BTreeMap::new();
        match persist::serialize_field(&self.is_enabled()) {
            Ok(data) => {
                serialized.insert(self.enabled_key.clone(), data);
            }
            Err(err) => {
                nih_plug::nih_debug_assert_failure!("Could not serialize '{}': {}", self.enabled_key, err);
            }
        }
        serialized
    }

    fn deserialize_fields(&self, serialized: &std::collections::BTreeMap<String, String>) {
        let Some(data) = serialized.get(&self.enabled_key) else {
            return;
        };

        match persist::deserialize_field(data) {
            Ok(is_enabled) => self.enabled.store(is_enabled, Ordering::Relaxed),
            Err(err) => {
                nih_plug::nih_debug_assert_failure!("Could not deserialize '{}': {}", self.enabled_key, err);
            }
        }
    }
}

pub(crate) trait ToParam<ValueType> {
    type Param: Param;
    type ParamType;
    type Type;

    fn to_param(&self, val: ValueType, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param;
}

pub(crate) fn to_plugin_on_off_param<Config: DynamicBPMDetectionConfigAccessor>(
    id: &'static str,
    parameter: &Parameter<Config, OnOff<f32>>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> PluginOnOffParam {
    let value = (parameter.get)(config);

    PluginOnOffParam::new(id, parameter.to_param(value, callback), value)
}

pub(crate) fn to_plugin_float_param<Config, ValueType>(
    parameter: &Parameter<Config, ValueType>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam
where
    Parameter<Config, ValueType>: ToParam<ValueType, Param = FloatParam, ParamType = f32>,
{
    let value = (parameter.get)(config);

    parameter.to_param(value, callback)
}

pub(crate) fn to_plugin_duration_param<Config>(
    parameter: &Parameter<Config, Duration>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam
where
    Parameter<Config, Duration>: ToParam<Duration, Param = FloatParam, ParamType = f32>,
{
    to_plugin_float_param(parameter, config, callback)
}

pub(crate) fn to_plugin_int_param<Config, ValueType>(
    parameter: &Parameter<Config, ValueType>,
    config: &Config,
    callback: &Arc<dyn Fn(i32) + Send + Sync>,
) -> IntParam
where
    Parameter<Config, ValueType>: ToParam<ValueType, Param = IntParam, ParamType = i32>,
{
    let value = (parameter.get)(config);

    parameter.to_param(value, callback)
}

pub(crate) fn to_plugin_u16_logarithmic_param<Config>(
    parameter: &Parameter<Config, u16>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
    u16_range_to_logarithmic_param(parameter, (parameter.get)(config), callback)
}

pub(crate) fn apply_float_param<V>(param: &FloatParam, value: V, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value.to_f32().unwrap());
    setter.end_set_parameter(param);
}

pub(crate) fn apply_onoff_param(
    param: &PluginOnOffParam,
    previous_value: OnOff<f32>,
    value: OnOff<f32>,
    setter: &ParamSetter,
) {
    param.apply(previous_value, value, setter);
}

pub(crate) fn apply_int_param<V>(param: &IntParam, value: V, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value.to_i32().unwrap());
    setter.end_set_parameter(param);
}

pub(crate) fn apply_duration_param(param: &FloatParam, value: Duration, setter: &ParamSetter) {
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value.as_secs_f32());
    setter.end_set_parameter(param);
}

macro_rules! impl_to_param_for_float {
    ($float_type:ty) => {
        impl<Config> ToParam<$float_type> for Parameter<Config, $float_type> {
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
        impl<Config> ToParam<$int_type> for Parameter<Config, $int_type> {
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

fn build_float_param<Config, ValueType>(
    param: &Parameter<Config, ValueType>,
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

impl<Config> ToParam<Duration> for Parameter<Config, Duration> {
    type Param = FloatParam;
    type ParamType = f32;
    type Type = f32;

    fn to_param(&self, val: Duration, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        build_float_param(self, val.as_secs_f32(), callback)
    }
}

impl<Config> ToParam<OnOff<f32>> for Parameter<Config, OnOff<f32>> {
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

pub(crate) fn u16_range_to_logarithmic_param<Config>(
    parameter: &Parameter<Config, u16>,
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
    use std::{sync::Arc, time::Duration};

    use bpm_detection_core::parameters::{
        DynamicBPMDetectionConfig, NormalDistributionConfig, StaticBPMDetectionConfig,
    };
    use gui::GUIConfig;
    use nih_plug::prelude::Params;
    use parameter::OnOff;

    use super::*;

    fn assert_float_eq(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < f32::EPSILON, "{actual} != {expected}");
    }

    #[test]
    fn plugin_on_off_param_exposes_host_id_and_persisted_enabled_key() {
        let callback: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(|_: f32| {});
        let plugin_param = PluginOnOffParam::new(
            "time_distance_weight",
            DynamicBPMDetectionConfig::PARAMETERS.time_distance_weight().to_param(OnOff::Off(1.5), &callback),
            OnOff::Off(1.5),
        );

        let param_ids = plugin_param.param_map().into_iter().map(|(id, _, _)| id).collect::<Vec<_>>();
        let serialized = plugin_param.serialize_fields();

        assert_eq!(param_ids, ["time_distance_weight"]);
        assert_eq!(serialized["time_distance_weight_onoff"], "false");
        assert_eq!(plugin_param.read(), OnOff::Off(1.5));
    }

    #[test]
    fn plugin_on_off_param_uses_parameter_accessor_to_read_initial_value() {
        let callback: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(|_: f32| {});
        let config = DynamicBPMDetectionConfig { time_distance_weight: OnOff::Off(1.5), ..Default::default() };

        let plugin_param = to_plugin_on_off_param(
            "time_distance_weight",
            &DynamicBPMDetectionConfig::PARAMETERS.time_distance_weight(),
            &config,
            &callback,
        );

        assert_eq!(plugin_param.read(), OnOff::Off(1.5));
    }

    #[test]
    fn plugin_int_param_uses_parameter_accessor_to_read_initial_value() {
        let callback: Arc<dyn Fn(i32) + Send + Sync> = Arc::new(|_: i32| {});
        let config = DynamicBPMDetectionConfig { beats_lookback: 13, ..Default::default() };

        let plugin_param =
            to_plugin_int_param(&DynamicBPMDetectionConfig::PARAMETERS.beats_lookback(), &config, &callback);

        assert_eq!(plugin_param.unmodulated_plain_value(), 13);
    }

    #[test]
    fn plugin_params_use_parameter_accessors_to_read_initial_values() {
        let update_f32: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(|_: f32| {});
        let update_i32: Arc<dyn Fn(i32) + Send + Sync> = Arc::new(|_: i32| {});
        let gui_config = GUIConfig { interpolation_duration: Duration::from_millis(820), interpolation_curve: 1.25 };
        let static_config =
            StaticBPMDetectionConfig { bpm_center: 111.5, bpm_range: 48, sample_rate: 720, ..Default::default() };
        let normal_distribution_config = NormalDistributionConfig { std_dev: 18.0, factor: 41.0, ..Default::default() };
        let gui_parameters = GUIConfig::PARAMETERS;
        let static_parameters = StaticBPMDetectionConfig::PARAMETERS;
        let normal_distribution_parameters = NormalDistributionConfig::PARAMETERS;

        let interpolation_duration =
            to_plugin_duration_param(&gui_parameters.interpolation_duration(), &gui_config, &update_f32);
        let interpolation_curve =
            to_plugin_float_param(&gui_parameters.interpolation_curve(), &gui_config, &update_f32);
        let bpm_center = to_plugin_float_param(&static_parameters.bpm_center(), &static_config, &update_f32);
        let bpm_range = to_plugin_int_param(&static_parameters.bpm_range(), &static_config, &update_i32);
        let sample_rate =
            to_plugin_u16_logarithmic_param(&static_parameters.sample_rate(), &static_config, &update_f32);
        let std_dev =
            to_plugin_float_param(&normal_distribution_parameters.std_dev(), &normal_distribution_config, &update_f32);
        let factor =
            to_plugin_float_param(&normal_distribution_parameters.factor(), &normal_distribution_config, &update_f32);

        assert_float_eq(interpolation_duration.unmodulated_plain_value(), 0.82);
        assert_float_eq(interpolation_curve.unmodulated_plain_value(), 1.25);
        assert_float_eq(bpm_center.unmodulated_plain_value(), 111.5);
        assert_eq!(bpm_range.unmodulated_plain_value(), 48);
        assert_float_eq(sample_rate.unmodulated_plain_value(), 720.0);
        assert_float_eq(std_dev.unmodulated_plain_value(), 18.0);
        assert_float_eq(factor.unmodulated_plain_value(), 41.0);
    }
}
