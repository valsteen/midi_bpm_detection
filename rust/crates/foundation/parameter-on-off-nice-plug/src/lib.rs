use std::{collections::BTreeMap, sync::Arc};

use nice_plug::{
    params::{BoolParam, FloatParam, Param, Params},
    prelude::{FloatRange, ParamPtr, ParamSetter, RemoteControlsPage},
};
use num_traits::ToPrimitive;
use parameter::{Parameter, ParameterField};
use parameter_nice_plug::{MirrorHostParams, NicePlugFieldAdapter};
use parameter_on_off::OnOff;

pub struct OnOffParams {
    enabled_id: String,
    value_id: &'static str,
    enabled: BoolParam,
    value: FloatParam,
}

impl OnOffParams {
    fn new(enabled_id: String, value_id: &'static str, enabled: BoolParam, value: FloatParam) -> Self {
        Self { enabled_id, value_id, enabled, value }
    }

    pub fn read(&self) -> OnOff<f32> {
        OnOff::new(self.enabled.unmodulated_plain_value(), self.value.unmodulated_plain_value())
    }
}

pub struct OnOffF32Adapter;

impl<Config> NicePlugFieldAdapter<Config, OnOff<f32>> for OnOffF32Adapter {
    type HostParams = OnOffParams;

    fn to_host_params<OnChange>(
        field: &ParameterField<Config, OnOff<f32>>,
        config: &Config,
        on_change: &OnChange,
    ) -> Self::HostParams
    where
        OnChange: Fn() + Clone + Send + Sync + 'static,
    {
        to_plugin_on_off_f32_params(field, config, on_change)
    }

    fn set_config_from_host_params(
        parameter: &Parameter<Config, OnOff<f32>>,
        config: &mut Config,
        host_params: &Self::HostParams,
    ) {
        set_config_from_on_off_f32_params(parameter, config, host_params);
    }

    fn add_param_map(host_params: &Self::HostParams, params: &mut Vec<(String, ParamPtr, String)>) {
        params.extend(host_params.param_map());
    }

    fn serialize_fields(host_params: &Self::HostParams, serialized: &mut BTreeMap<String, String>) {
        serialized.extend(Params::serialize_fields(host_params));
    }

    fn deserialize_fields(host_params: &Self::HostParams, serialized: &BTreeMap<String, String>) {
        Params::deserialize_fields(host_params, serialized);
    }

    fn add_remote_controls(host_params: &Self::HostParams, page: &mut impl RemoteControlsPage) {
        page.add_param(&host_params.enabled);
        page.add_param(&host_params.value);
    }
}

unsafe impl Params for OnOffParams {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        vec![
            (self.enabled_id.clone(), self.enabled.as_ptr(), String::new()),
            (String::from(self.value_id), self.value.as_ptr(), String::new()),
        ]
    }
}

pub fn to_plugin_on_off_f32_params<Config, OnChange>(
    field: &ParameterField<Config, OnOff<f32>>,
    config: &Config,
    on_change: &OnChange,
) -> OnOffParams
where
    OnChange: Fn() + Clone + Send + Sync + 'static,
{
    let parameter = &field.parameter;
    let value = (parameter.get)(config);
    let enabled_on_change = on_change.clone();
    let enabled = BoolParam::new(format!("{} enabled", parameter.spec.label), value.is_enabled())
        .with_callback(Arc::new(move |_| enabled_on_change()));
    let value_param = float_param_from_metadata(parameter, value.value(), on_change);

    OnOffParams::new(format!("{}_enabled", field.field_name), field.field_name, enabled, value_param)
}

pub fn set_config_from_on_off_f32_params<Config>(
    parameter: &Parameter<Config, OnOff<f32>>,
    config: &mut Config,
    host_params: &OnOffParams,
) {
    (parameter.set)(config, host_params.read());
}

impl<Config> MirrorHostParams<Config, OnOff<f32>> for OnOffParams {
    fn mirror_host_params(
        &self,
        config: &mut Config,
        parameter: &Parameter<Config, OnOff<f32>>,
        value: OnOff<f32>,
        param_setter: &ParamSetter<'_>,
    ) {
        let previous_value = (parameter.get)(config);

        if previous_value.is_enabled() != value.is_enabled() {
            set_bool_host_param(&self.enabled, value.is_enabled(), param_setter);
        }
        if (previous_value.value() - value.value()).abs() > f32::EPSILON {
            set_float_host_param(&self.value, value.value(), param_setter);
        }
        (parameter.set)(config, value);
    }
}

fn float_param_from_metadata<Config, ValueType, OnChange>(
    parameter: &Parameter<Config, ValueType>,
    value: f32,
    on_change: &OnChange,
) -> FloatParam
where
    OnChange: Fn() + Clone + Send + Sync + 'static,
{
    let range = if parameter.spec.logarithmic {
        FloatRange::Skewed {
            min: metadata_f64_to_f32(*parameter.spec.range.start()),
            max: metadata_f64_to_f32(*parameter.spec.range.end()),
            factor: 0.3,
        }
    } else {
        FloatRange::Linear {
            min: metadata_f64_to_f32(*parameter.spec.range.start()),
            max: metadata_f64_to_f32(*parameter.spec.range.end()),
        }
    };

    let value_on_change = on_change.clone();
    let mut param =
        FloatParam::new(parameter.spec.label, value, range).with_callback(Arc::new(move |_| value_on_change()));
    if let Some(unit) = parameter.spec.unit {
        param = param.with_unit(unit);
    }
    if parameter.spec.step > 0.0 {
        param = param.with_step_size(metadata_f64_to_f32(parameter.spec.step));
    }

    param.with_value_to_string(Arc::new(|value| format!("{value:.2}")))
}

fn metadata_f64_to_f32(value: f64) -> f32 {
    value.to_f32().expect("parameter metadata should fit in nice-plug f32 values")
}

fn set_bool_host_param(enabled_param: &BoolParam, enabled: bool, param_setter: &ParamSetter<'_>) {
    param_setter.begin_set_parameter(enabled_param);
    param_setter.set_parameter(enabled_param, enabled);
    param_setter.end_set_parameter(enabled_param);
}

fn set_float_host_param(value_param: &FloatParam, value: f32, param_setter: &ParamSetter<'_>) {
    param_setter.begin_set_parameter(value_param);
    param_setter.set_parameter(value_param, value);
    param_setter.end_set_parameter(value_param);
}
