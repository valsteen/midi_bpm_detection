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
use parameter::{OnOff, Parameter, ParameterField};

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
    field: &ParameterField<Config, OnOff<f32>>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> PluginOnOffParam {
    let parameter = &field.parameter;
    let value = (parameter.get)(config);

    PluginOnOffParam::new(field.field_name, parameter.to_param(value, callback), value)
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
                let range = if self.spec.logarithmic {
                    FloatRange::Skewed {
                        min: *self.spec.range.start() as f32,
                        max: *self.spec.range.end() as f32,
                        factor: 0.3,
                    }
                } else {
                    FloatRange::Linear { min: *self.spec.range.start() as f32, max: *self.spec.range.end() as f32 }
                };

                let mut param = FloatParam::new(self.spec.label, val as f32, range).with_callback(callback.clone());
                if let Some(unit) = self.spec.unit {
                    param = param.with_unit(unit);
                }
                if self.spec.step > 0.0 {
                    param = param.with_step_size(self.spec.step as f32)
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
                    self.spec.label,
                    i32::from(val),
                    IntRange::Linear { min: *self.spec.range.start() as i32, max: *self.spec.range.end() as i32 },
                )
                .with_callback(callback.clone());
                if let Some(unit) = self.spec.unit {
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
    let range = if param.spec.logarithmic {
        FloatRange::Skewed { min: *param.spec.range.start() as f32, max: *param.spec.range.end() as f32, factor: 0.3 }
    } else {
        FloatRange::Linear { min: *param.spec.range.start() as f32, max: *param.spec.range.end() as f32 }
    };

    let mut float_param = FloatParam::new(param.spec.label, val, range).with_callback(callback.clone());

    if let Some(unit) = param.spec.unit {
        float_param = float_param.with_unit(unit);
    }
    if param.spec.step > 0.0 {
        float_param = float_param.with_step_size(param.spec.step as f32);
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
        parameter.spec.label,
        f32::from(val),
        FloatRange::Skewed {
            min: *parameter.spec.range.start() as f32,
            max: *parameter.spec.range.end() as f32,
            factor: 0.3,
        },
    )
    .with_callback(callback.clone());
    if let Some(unit) = parameter.spec.unit {
        param = param.with_unit(unit);
    }
    param = param.with_step_size(parameter.spec.step.max(1.0) as f32);
    if let Some(unit) = parameter.spec.unit {
        param = param.with_unit(unit);
    }
    param
}

#[cfg(test)]
#[path = "../tests/unit/plugin_parameter_adapters.rs"]
mod tests;
