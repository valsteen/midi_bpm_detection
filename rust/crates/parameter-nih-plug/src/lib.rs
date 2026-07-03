use std::sync::Arc;

use nih_plug::{
    params::{FloatParam, IntParam},
    prelude::{FloatRange, IntRange, Param},
};
use num_traits::ToPrimitive;
use parameter::Parameter;
pub use parameter_nih_plug_macros::nih_plugin_parameter_group;

pub trait GeneratedNihPlugParams {}

pub trait ToNihPlugParam<ValueType> {
    type Param: Param;
    type CallbackValue;

    fn to_param(&self, value: ValueType, callback: &Arc<dyn Fn(Self::CallbackValue) + Send + Sync>) -> Self::Param;
}

pub fn to_plugin_float_param<Config, ValueType>(
    parameter: &Parameter<Config, ValueType>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam
where
    Parameter<Config, ValueType>: ToNihPlugParam<ValueType, Param = FloatParam, CallbackValue = f32>,
{
    let value = (parameter.get)(config);

    parameter.to_param(value, callback)
}

pub fn to_plugin_int_param<Config, ValueType>(
    parameter: &Parameter<Config, ValueType>,
    config: &Config,
    callback: &Arc<dyn Fn(i32) + Send + Sync>,
) -> IntParam
where
    Parameter<Config, ValueType>: ToNihPlugParam<ValueType, Param = IntParam, CallbackValue = i32>,
{
    let value = (parameter.get)(config);

    parameter.to_param(value, callback)
}

pub fn to_plugin_u16_logarithmic_param<Config>(
    parameter: &Parameter<Config, u16>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
    u16_range_to_logarithmic_param(parameter, (parameter.get)(config), callback)
}

pub trait SetConfigFromFloatParam<Config>: Sized {
    fn set_config_from_float_param(parameter: &Parameter<Config, Self>, config: &mut Config, param: &FloatParam);
}

pub fn set_config_from_float_param<Config, ValueType>(
    parameter: &Parameter<Config, ValueType>,
    config: &mut Config,
    param: &FloatParam,
) where
    ValueType: SetConfigFromFloatParam<Config>,
{
    ValueType::set_config_from_float_param(parameter, config, param);
}

pub trait SetConfigFromIntParam<Config>: Sized {
    fn set_config_from_int_param(parameter: &Parameter<Config, Self>, config: &mut Config, param: &IntParam);
}

pub fn set_config_from_int_param<Config, ValueType>(
    parameter: &Parameter<Config, ValueType>,
    config: &mut Config,
    param: &IntParam,
) where
    ValueType: SetConfigFromIntParam<Config>,
{
    ValueType::set_config_from_int_param(parameter, config, param);
}

macro_rules! impl_to_param_for_float {
    ($float_type:ty) => {
        impl<Config> ToNihPlugParam<$float_type> for Parameter<Config, $float_type> {
            type CallbackValue = f32;
            type Param = FloatParam;

            fn to_param(
                &self,
                value: $float_type,
                callback: &Arc<dyn Fn(Self::CallbackValue) + Send + Sync>,
            ) -> Self::Param {
                let range = if self.spec.logarithmic {
                    FloatRange::Skewed {
                        min: metadata_f64_to_f32(*self.spec.range.start()),
                        max: metadata_f64_to_f32(*self.spec.range.end()),
                        factor: 0.3,
                    }
                } else {
                    FloatRange::Linear {
                        min: metadata_f64_to_f32(*self.spec.range.start()),
                        max: metadata_f64_to_f32(*self.spec.range.end()),
                    }
                };

                let mut param = FloatParam::new(self.spec.label, parameter_value_to_f32(&value), range)
                    .with_callback(callback.clone());
                if let Some(unit) = self.spec.unit {
                    param = param.with_unit(unit);
                }
                if self.spec.step > 0.0 {
                    param = param.with_step_size(metadata_f64_to_f32(self.spec.step));
                }

                param.with_value_to_string(Arc::new(|value| format!("{value:.2}")))
            }
        }
    };
}

macro_rules! impl_to_param_for_integer {
    ($int_type:ty) => {
        impl<Config> ToNihPlugParam<$int_type> for Parameter<Config, $int_type> {
            type CallbackValue = i32;
            type Param = IntParam;

            fn to_param(
                &self,
                value: $int_type,
                callback: &Arc<dyn Fn(Self::CallbackValue) + Send + Sync>,
            ) -> Self::Param {
                let mut param = IntParam::new(
                    self.spec.label,
                    parameter_value_to_i32(&value),
                    IntRange::Linear {
                        min: metadata_f64_to_i32(*self.spec.range.start()),
                        max: metadata_f64_to_i32(*self.spec.range.end()),
                    },
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

impl_to_param_for_float!(f32);
impl_to_param_for_float!(f64);
impl_to_param_for_integer!(u16);
impl_to_param_for_integer!(u8);

impl<Config> SetConfigFromFloatParam<Config> for f32 {
    fn set_config_from_float_param(parameter: &Parameter<Config, Self>, config: &mut Config, param: &FloatParam) {
        (parameter.set)(config, param.unmodulated_plain_value());
    }
}

impl<Config> SetConfigFromFloatParam<Config> for f64 {
    fn set_config_from_float_param(parameter: &Parameter<Config, Self>, config: &mut Config, param: &FloatParam) {
        (parameter.set)(config, f64::from(param.unmodulated_plain_value()));
    }
}

impl<Config> SetConfigFromFloatParam<Config> for u16 {
    fn set_config_from_float_param(parameter: &Parameter<Config, Self>, config: &mut Config, param: &FloatParam) {
        (parameter.set)(config, float_param_value_to_u16(param));
    }
}

impl<Config> SetConfigFromIntParam<Config> for u16 {
    fn set_config_from_int_param(parameter: &Parameter<Config, Self>, config: &mut Config, param: &IntParam) {
        (parameter.set)(config, int_param_value_to_u16(param));
    }
}

impl<Config> SetConfigFromIntParam<Config> for u8 {
    fn set_config_from_int_param(parameter: &Parameter<Config, Self>, config: &mut Config, param: &IntParam) {
        (parameter.set)(config, int_param_value_to_u8(param));
    }
}

fn u16_range_to_logarithmic_param<Config>(
    parameter: &Parameter<Config, u16>,
    value: u16,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
    let mut param = FloatParam::new(
        parameter.spec.label,
        f32::from(value),
        FloatRange::Skewed {
            min: metadata_f64_to_f32(*parameter.spec.range.start()),
            max: metadata_f64_to_f32(*parameter.spec.range.end()),
            factor: 0.3,
        },
    )
    .with_callback(callback.clone());
    if let Some(unit) = parameter.spec.unit {
        param = param.with_unit(unit);
    }
    param.with_step_size(metadata_f64_to_f32(parameter.spec.step.max(1.0)))
}

fn metadata_f64_to_f32(value: f64) -> f32 {
    value.to_f32().expect("parameter metadata should fit in NIH-plug f32 values")
}

fn metadata_f64_to_i32(value: f64) -> i32 {
    value.to_i32().expect("parameter metadata should fit in NIH-plug i32 values")
}

fn parameter_value_to_f32(value: &impl ToPrimitive) -> f32 {
    value.to_f32().expect("parameter value should fit in NIH-plug f32 values")
}

fn parameter_value_to_i32(value: &impl ToPrimitive) -> i32 {
    value.to_i32().expect("parameter value should fit in NIH-plug i32 values")
}

fn float_param_value_to_u16(param: &FloatParam) -> u16 {
    param.unmodulated_plain_value().to_u16().expect("FloatParam value should fit in u16 config field")
}

fn int_param_value_to_u16(param: &IntParam) -> u16 {
    param.unmodulated_plain_value().to_u16().expect("IntParam value should fit in u16 config field")
}

fn int_param_value_to_u8(param: &IntParam) -> u8 {
    param.unmodulated_plain_value().to_u8().expect("IntParam value should fit in u8 config field")
}
