use std::{sync::Arc, time::Duration};

use nih_plug::{
    params::{FloatParam, IntParam, Param},
    prelude::{FloatRange, ParamSetter},
};
use num_traits::ToPrimitive;
use parameter::{OnOff, Parameter};
use parameter_nih_plug::OnOffParam;

pub(crate) trait ToParam<ValueType> {
    type Param: Param;
    type ParamType;

    fn to_param(&self, val: ValueType, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param;
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

pub(crate) fn apply_float_param<V>(param: &FloatParam, value: V, setter: &ParamSetter)
where
    V: 'static + ToPrimitive + Copy,
{
    setter.begin_set_parameter(param);
    setter.set_parameter(param, value.to_f32().unwrap());
    setter.end_set_parameter(param);
}

pub(crate) fn apply_onoff_param(
    param: &OnOffParam,
    previous_value: OnOff<f32>,
    value: OnOff<f32>,
    setter: &ParamSetter,
) {
    param.set_enabled(value.is_enabled());
    if (previous_value.value() - value.value()).abs() > f32::EPSILON {
        setter.begin_set_parameter(param.param());
        setter.set_parameter(param.param(), value.value());
        setter.end_set_parameter(param.param());
    }
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

    fn to_param(&self, val: Duration, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        build_float_param(self, val.as_secs_f32(), callback)
    }
}

impl<Config> ToParam<OnOff<f32>> for Parameter<Config, OnOff<f32>> {
    type Param = FloatParam;
    type ParamType = f32;

    fn to_param(&self, val: OnOff<f32>, callback: &Arc<dyn Fn(Self::ParamType) + Send + Sync>) -> Self::Param {
        build_float_param(self, val.value(), callback)
    }
}

impl_to_param_for_float!(f32);
impl_to_param_for_float!(f64);

#[cfg(test)]
#[path = "../tests/unit/plugin_parameter_adapters.rs"]
mod tests;
