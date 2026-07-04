use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use nih_plug::{
    params::{FloatParam, Param, Params, persist},
    prelude::{FloatRange, ParamPtr, ParamSetter, RemoteControlsPage},
};
use num_traits::ToPrimitive;
use parameter::{Parameter, ParameterField};
use parameter_nih_plug::{MirrorHostParam, NihPlugFieldAdapter};
use parameter_on_off::OnOff;

pub struct OnOffParam {
    id: &'static str,
    value: FloatParam,
    enabled_key: String,
    enabled: AtomicBool,
}

impl OnOffParam {
    pub fn new(id: &'static str, value: FloatParam, initial_value: OnOff<f32>) -> Self {
        Self { id, value, enabled_key: format!("{id}_onoff"), enabled: AtomicBool::new(initial_value.is_enabled()) }
    }

    pub fn param(&self) -> &FloatParam {
        &self.value
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn read(&self) -> OnOff<f32> {
        OnOff::new(self.is_enabled(), self.value.unmodulated_plain_value())
    }
}

pub struct OnOffF32Adapter;

impl<Config> NihPlugFieldAdapter<Config, OnOff<f32>> for OnOffF32Adapter {
    type CallbackValue = f32;
    type HostParam = OnOffParam;

    fn to_host_param(
        field: &ParameterField<Config, OnOff<f32>>,
        config: &Config,
        callback: &Arc<dyn Fn(Self::CallbackValue) + Send + Sync>,
    ) -> Self::HostParam {
        to_plugin_on_off_f32_param(field, config, callback)
    }

    fn set_config_from_host_param(
        parameter: &Parameter<Config, OnOff<f32>>,
        config: &mut Config,
        param: &Self::HostParam,
    ) {
        set_config_from_on_off_f32_param(parameter, config, param);
    }

    fn add_param_map(param: &Self::HostParam, params: &mut Vec<(String, ParamPtr, String)>) {
        params.extend(param.param_map());
    }

    fn serialize_fields(param: &Self::HostParam, serialized: &mut BTreeMap<String, String>) {
        serialized.extend(Params::serialize_fields(param));
    }

    fn deserialize_fields(param: &Self::HostParam, serialized: &BTreeMap<String, String>) {
        Params::deserialize_fields(param, serialized);
    }

    fn add_remote_control(param: &Self::HostParam, page: &mut impl RemoteControlsPage) {
        page.add_param(param.param());
    }
}

unsafe impl Params for OnOffParam {
    fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
        vec![(String::from(self.id), self.value.as_ptr(), String::new())]
    }

    fn serialize_fields(&self) -> BTreeMap<String, String> {
        let mut serialized = BTreeMap::new();
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

    fn deserialize_fields(&self, serialized: &BTreeMap<String, String>) {
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

pub fn to_plugin_on_off_f32_param<Config>(
    field: &ParameterField<Config, OnOff<f32>>,
    config: &Config,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> OnOffParam {
    let parameter = &field.parameter;
    let value = (parameter.get)(config);

    OnOffParam::new(field.field_name, float_param_from_metadata(parameter, value.value(), callback), value)
}

pub fn set_config_from_on_off_f32_param<Config>(
    parameter: &Parameter<Config, OnOff<f32>>,
    config: &mut Config,
    param: &OnOffParam,
) {
    (parameter.set)(config, param.read());
}

impl<Config> MirrorHostParam<Config, OnOff<f32>> for OnOffParam {
    fn mirror_host_param(
        &self,
        config: &mut Config,
        parameter: &Parameter<Config, OnOff<f32>>,
        value: OnOff<f32>,
        param_setter: &ParamSetter<'_>,
    ) {
        let previous_value = (parameter.get)(config);

        self.set_enabled(value.is_enabled());
        if (previous_value.value() - value.value()).abs() > f32::EPSILON {
            set_float_host_param(self.param(), value.value(), param_setter);
        }
        (parameter.set)(config, value);
    }
}

fn float_param_from_metadata<Config, ValueType>(
    parameter: &Parameter<Config, ValueType>,
    value: f32,
    callback: &Arc<dyn Fn(f32) + Send + Sync>,
) -> FloatParam {
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

    let mut param = FloatParam::new(parameter.spec.label, value, range).with_callback(callback.clone());
    if let Some(unit) = parameter.spec.unit {
        param = param.with_unit(unit);
    }
    if parameter.spec.step > 0.0 {
        param = param.with_step_size(metadata_f64_to_f32(parameter.spec.step));
    }

    param.with_value_to_string(Arc::new(|value| format!("{value:.2}")))
}

fn metadata_f64_to_f32(value: f64) -> f32 {
    value.to_f32().expect("parameter metadata should fit in NIH-plug f32 values")
}

fn set_float_host_param(value_param: &FloatParam, value: f32, param_setter: &ParamSetter<'_>) {
    param_setter.begin_set_parameter(value_param);
    param_setter.set_parameter(value_param, value);
    param_setter.end_set_parameter(value_param);
}
