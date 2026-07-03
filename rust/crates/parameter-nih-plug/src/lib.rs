use std::sync::Arc;

use nih_plug::{
    params::FloatParam,
    prelude::{FloatRange, Param},
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

impl_to_param_for_float!(f32);
impl_to_param_for_float!(f64);

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

fn metadata_f64_to_f32(value: f64) -> f32 {
    value.to_f32().expect("parameter metadata should fit in NIH-plug f32 values")
}

fn parameter_value_to_f32(value: &impl ToPrimitive) -> f32 {
    value.to_f32().expect("parameter value should fit in NIH-plug f32 values")
}
