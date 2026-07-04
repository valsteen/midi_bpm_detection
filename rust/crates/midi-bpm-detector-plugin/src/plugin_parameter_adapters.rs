use std::time::Duration;

use nih_plug::{
    params::{FloatParam, IntParam},
    prelude::ParamSetter,
};
use num_traits::ToPrimitive;
use parameter::OnOff;
use parameter_nih_plug::OnOffParam;

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
