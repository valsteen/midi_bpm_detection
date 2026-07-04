use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use nih_plug::{
    context::PluginApi,
    params::{FloatParam, IntParam, Param, Params},
    prelude::{GuiContext, ParamPtr, ParamSetter, PluginState, RemoteControlsPage},
};
use num_traits::ToPrimitive;
use parameter::{Asf64, parameter_group};
use parameter_nih_plug::{GeneratedNihPlugParams, MirrorHostParam, nih_plugin_parameter_group};

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleChildConfig {
    #[parameter(label = "Child gain", range = 0.0..=2.0, default = 1.0)]
    pub child_gain: f32,
    #[parameter(label = "Child precise", range = 0.0..=10.0, default = 3.5)]
    pub child_precise: f64,
}

#[nih_plugin_parameter_group(config = ExampleChildConfig, group = "child")]
pub struct ExampleChildParams {
    pub child_gain: FloatParam,
    pub child_precise: FloatParam,
}

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleParentConfig {
    #[parameter(label = "Gain", range = 0.0..=2.0, default = 1.0)]
    pub gain: f32,
    #[parameter(label = "Count", range = 1.0..=16.0, default = 4)]
    pub count: u16,
    #[parameter(label = "Sample rate", range = 1.0..=1_000.0, step = 1.0, logarithmic = true, default = 450)]
    pub sample_rate: u16,
    pub child: ExampleChildConfig,
}

#[nih_plugin_parameter_group(config = ExampleParentConfig, group = "parent")]
pub struct ExampleParentParams {
    pub gain: FloatParam,
    pub count: IntParam,
    #[nih_plugin_parameter(adapter = "float_u16_logarithmic")]
    pub sample_rate: FloatParam,
    #[nih_plugin_nested(group = "child")]
    pub child: ExampleChildParams,
}

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleDurationConfig {
    #[parameter(label = "Delay", unit = "s", range = 0.050..=1.0, default = Duration::from_millis(500))]
    pub delay: Duration,
    #[parameter(label = "Curve", range = 0.0..=2.0, default = 0.7)]
    pub curve: f32,
}

#[nih_plugin_parameter_group(config = ExampleDurationConfig, group = "duration")]
pub struct ExampleDurationParams {
    pub delay: FloatParam,
    pub curve: FloatParam,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExternalGain(f32);

impl Asf64 for ExternalGain {
    fn as_f64(&self) -> f64 {
        f64::from(self.0)
    }

    fn set_from_f64(&mut self, value: f64) {
        self.0 = metadata_to_f32(value);
    }

    fn new_from(value: f64) -> Self {
        Self(metadata_to_f32(value))
    }
}

#[parameter_group]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExternalAdapterConfig {
    #[parameter(label = "External gain", range = 0.0..=1.0, default = ExternalGain(0.6))]
    pub custom_gain: ExternalGain,
}

#[nih_plugin_parameter_group(config = ExternalAdapterConfig, group = "external")]
pub struct ExternalAdapterParams {
    #[nih_plugin_parameter(adapter = external_gain::Adapter, callback = f32)]
    pub custom_gain: external_gain::ExternalGainParam,
}

mod external_gain {
    use std::{collections::BTreeMap, sync::Arc};

    use nih_plug::{
        params::{FloatParam, Param, Params},
        prelude::{FloatRange, ParamPtr, ParamSetter, RemoteControlsPage},
    };
    use parameter::{Parameter, ParameterField};
    use parameter_nih_plug::{MirrorHostParam, NihPlugFieldAdapter};

    use crate::{ExternalAdapterConfig, ExternalGain, metadata_to_f32};

    pub struct Adapter;

    pub struct ExternalGainParam {
        id: &'static str,
        value: FloatParam,
    }

    impl ExternalGainParam {
        fn read(&self) -> ExternalGain {
            ExternalGain(self.value.unmodulated_plain_value())
        }
    }

    unsafe impl Params for ExternalGainParam {
        fn param_map(&self) -> Vec<(String, ParamPtr, String)> {
            vec![(String::from(self.id), self.value.as_ptr(), String::new())]
        }
    }

    impl<Config> NihPlugFieldAdapter<Config, ExternalGain> for Adapter {
        type CallbackValue = f32;
        type HostParam = ExternalGainParam;

        fn to_host_param(
            field: &ParameterField<Config, ExternalGain>,
            config: &Config,
            callback: &Arc<dyn Fn(Self::CallbackValue) + Send + Sync>,
        ) -> Self::HostParam {
            let parameter = &field.parameter;
            let value = (parameter.get)(config);
            let host_param = FloatParam::new(
                parameter.spec.label,
                value.0,
                FloatRange::Linear {
                    min: metadata_to_f32(*parameter.spec.range.start()),
                    max: metadata_to_f32(*parameter.spec.range.end()),
                },
            )
            .with_callback(callback.clone());

            Self::HostParam { id: field.field_name, value: host_param }
        }

        fn set_config_from_host_param(
            parameter: &Parameter<Config, ExternalGain>,
            config: &mut Config,
            param: &Self::HostParam,
        ) {
            (parameter.set)(config, param.read());
        }

        fn add_param_map(param: &Self::HostParam, params: &mut Vec<(String, ParamPtr, String)>) {
            params.extend(param.param_map());
        }

        fn serialize_fields(_param: &Self::HostParam, _serialized: &mut BTreeMap<String, String>) {}

        fn deserialize_fields(_param: &Self::HostParam, _serialized: &BTreeMap<String, String>) {}

        fn add_remote_control(param: &Self::HostParam, page: &mut impl RemoteControlsPage) {
            page.add_param(&param.value);
        }
    }

    impl MirrorHostParam<ExternalAdapterConfig, ExternalGain> for ExternalGainParam {
        fn mirror_host_param(
            &self,
            config: &mut ExternalAdapterConfig,
            parameter: &Parameter<ExternalAdapterConfig, ExternalGain>,
            value: ExternalGain,
            param_setter: &ParamSetter<'_>,
        ) {
            (parameter.set)(config, value);
            param_setter.begin_set_parameter(&self.value);
            param_setter.set_parameter(&self.value, value.0);
            param_setter.end_set_parameter(&self.value);
        }
    }
}

mod path_config {
    use parameter::parameter_group;

    #[parameter_group]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct PathConfig {
        #[parameter(label = "Path gain", range = 0.0..=2.0, default = 0.75)]
        pub path_gain: f32,
    }
}

#[nih_plugin_parameter_group(config = path_config::PathConfig, group = "path")]
pub struct PathParams {
    pub path_gain: FloatParam,
}

mod canonical_public_config {
    mod config {
        use parameter::parameter_group;

        #[parameter_group]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct ReexportedConfig {
            #[parameter(label = "Re-exported gain", range = 0.0..=2.0, default = 0.75)]
            pub reexported_gain: f32,
        }
    }

    pub use config::*;
}

#[nih_plugin_parameter_group(config = canonical_public_config::ReexportedConfig, group = "reexported")]
pub struct ReexportedParams {
    pub reexported_gain: FloatParam,
}

mod acronym_public_config {
    mod config {
        use std::time::Duration;

        use parameter::parameter_group;

        #[parameter_group]
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub struct GUIConfig {
            #[parameter(
                label = "Interpolation duration",
                unit = "s",
                range = 0.050..=1.0,
                default = Duration::from_millis(500)
            )]
            pub interpolation_duration: Duration,
        }
    }

    pub use config::*;
}

#[nih_plugin_parameter_group(config = acronym_public_config::GUIConfig, group = "gui")]
pub struct AcronymGUIParams {
    pub interpolation_duration: FloatParam,
}

#[test]
fn generated_group_maps_field_ids_in_catalog_order_without_local_groups() {
    let callback = callback_f32();
    let params = ExampleChildParams::new(&ExampleChildConfig { child_gain: 1.25, child_precise: 4.75 }, &callback);
    let ids_and_groups = params.param_map().into_iter().map(|(id, _, group)| (id, group)).collect::<Vec<_>>();

    assert_eq!(
        ids_and_groups,
        [(String::from("child_gain"), String::new()), (String::from("child_precise"), String::new())]
    );
}

#[test]
fn generated_group_maps_float_int_adapter_and_nested_fields_in_source_order() {
    let callbacks = callbacks();
    let params = ExampleParentParams::new(&example_parent_config(), &callbacks.f32, &callbacks.i32);
    let ids_and_groups = params.param_map().into_iter().map(|(id, _, group)| (id, group)).collect::<Vec<_>>();

    assert_eq!(
        ids_and_groups,
        [
            (String::from("gain"), String::new()),
            (String::from("count"), String::new()),
            (String::from("sample_rate"), String::new()),
            (String::from("child_gain"), String::from("child")),
            (String::from("child_precise"), String::from("child")),
        ]
    );
}

#[test]
fn generated_group_reads_host_values_back_to_config() {
    let callbacks = callbacks();
    let source_config = example_parent_config();
    let params = ExampleParentParams::new(&source_config, &callbacks.f32, &callbacks.i32);

    assert_eq!(params.read_config(), source_config);
}

#[test]
fn generated_group_preserves_parameter_metadata() {
    let callbacks = callbacks();
    let params = ExampleParentParams::new(&example_parent_config(), &callbacks.f32, &callbacks.i32);

    assert_eq!(params.gain.name(), "Gain");
    assert_eq!(params.count.name(), "Count");
    assert_eq!(params.sample_rate.name(), "Sample rate");
    assert_eq!(params.child.child_precise.name(), "Child precise");
}

#[test]
fn generated_group_reads_duration_float_params_back_to_config() {
    let callback = callback_f32();
    let source_config = ExampleDurationConfig { delay: Duration::from_secs_f32(0.82), curve: 1.25 };
    let params = ExampleDurationParams::new(&source_config, &callback);

    assert_eq!(params.delay.name(), "Delay");
    assert!((params.delay.unmodulated_plain_value() - 0.82).abs() < f32::EPSILON);
    assert_eq!(params.read_config(), source_config);
}

#[test]
fn external_adapter_constructs_reads_mirrors_and_adds_remote_control() {
    let callback = callback_f32();
    let source_config = ExternalAdapterConfig { custom_gain: ExternalGain(0.6) };
    let params = ExternalAdapterParams::new(&source_config, &callback);
    let ids_and_groups = params.param_map().into_iter().map(|(id, _, group)| (id, group)).collect::<Vec<_>>();
    let mut remote_controls = RemoteControlNames(Vec::new());
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;

    params.add_remote_controls(&mut remote_controls);

    assert_eq!(ids_and_groups, [(String::from("custom_gain"), String::new())]);
    assert_eq!(remote_controls.0, ["External gain"]);
    assert_eq!(params.read_config(), source_config);

    params.mirror_custom_gain(&mut config, ExternalGain(0.9), &setter);

    assert_eq!(config.custom_gain, ExternalGain(0.9));
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End]);
}

#[test]
fn generated_group_implements_marker_trait() {
    fn assert_generated<T: GeneratedNihPlugParams>() {}

    assert_generated::<ExampleChildParams>();
    assert_generated::<ExampleParentParams>();
    assert_generated::<ExampleDurationParams>();
}

#[test]
fn mirror_host_param_updates_config_and_writes_host_param_through_setter() {
    let callbacks = callbacks();
    let source_config = ExampleParentConfig {
        gain: 1.0,
        count: 4,
        sample_rate: 450,
        child: ExampleChildConfig { child_gain: 1.5, child_precise: 4.75 },
    };
    let params = ExampleParentParams::new(&source_config, &callbacks.f32, &callbacks.i32);
    let duration_params = ExampleDurationParams::new(
        &ExampleDurationConfig { delay: Duration::from_secs_f32(0.5), curve: 0.7 },
        &callbacks.f32,
    );
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;
    let mut child_config = config.child.clone();
    let mut duration_config = ExampleDurationConfig { delay: Duration::from_secs_f32(0.5), curve: 0.7 };

    params.gain.mirror_host_param(&mut config, &ExampleParentConfig::PARAMETERS.gain(), 1.75, &setter);
    params.count.mirror_host_param(&mut config, &ExampleParentConfig::PARAMETERS.count(), 6, &setter);
    params.sample_rate.mirror_host_param(&mut config, &ExampleParentConfig::PARAMETERS.sample_rate(), 720, &setter);
    params.child.child_precise.mirror_host_param(
        &mut child_config,
        &ExampleChildConfig::PARAMETERS.child_precise(),
        7.25,
        &setter,
    );
    duration_params.delay.mirror_host_param(
        &mut duration_config,
        &ExampleDurationConfig::PARAMETERS.delay(),
        Duration::from_secs_f32(0.25),
        &setter,
    );

    assert!((config.gain - 1.75).abs() < f32::EPSILON);
    assert_eq!(config.count, 6);
    assert_eq!(config.sample_rate, 720);
    assert!((child_config.child_precise - 7.25).abs() < f64::EPSILON);
    assert_eq!(duration_config.delay, Duration::from_secs_f32(0.25));
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End].repeat(5));
}

#[test]
fn generated_field_mirror_methods_can_name_path_qualified_config_descriptors() {
    let callback = callback_f32();
    let source_config = path_config::PathConfig { path_gain: 0.75 };
    let params = PathParams::new(&source_config, &callback);
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;

    params.mirror_path_gain(&mut config, 1.25, &setter);

    assert!((config.path_gain - 1.25).abs() < f32::EPSILON);
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End]);
}

#[test]
fn generated_field_mirror_methods_use_canonical_public_reexported_config_paths() {
    let callback = callback_f32();
    let source_config = canonical_public_config::ReexportedConfig { reexported_gain: 0.75 };
    let params = ReexportedParams::new(&source_config, &callback);
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;

    params.mirror_reexported_gain(&mut config, 1.5, &setter);

    assert!((config.reexported_gain - 1.5).abs() < f32::EPSILON);
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End]);
}

#[test]
fn generated_field_mirror_methods_match_parameter_group_acronym_descriptor_names() {
    let callback = callback_f32();
    let source_config = acronym_public_config::GUIConfig { interpolation_duration: Duration::from_millis(500) };
    let params = AcronymGUIParams::new(&source_config, &callback);
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;

    params.mirror_interpolation_duration(&mut config, Duration::from_millis(250), &setter);

    assert_eq!(config.interpolation_duration, Duration::from_millis(250));
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End]);
}

fn example_parent_config() -> ExampleParentConfig {
    ExampleParentConfig {
        gain: 1.25,
        count: 9,
        sample_rate: 720,
        child: ExampleChildConfig { child_gain: 1.5, child_precise: 4.75 },
    }
}

struct Callbacks {
    f32: Arc<dyn Fn(f32) + Send + Sync>,
    i32: Arc<dyn Fn(i32) + Send + Sync>,
}

fn callbacks() -> Callbacks {
    Callbacks { f32: callback_f32(), i32: Arc::new(|_: i32| {}) }
}

fn callback_f32() -> Arc<dyn Fn(f32) + Send + Sync> {
    Arc::new(|_: f32| {})
}

fn metadata_to_f32(value: f64) -> f32 {
    value.to_f32().expect("test parameter metadata should fit in f32")
}

struct RemoteControlNames(Vec<String>);

impl RemoteControlsPage for RemoteControlNames {
    fn add_param(&mut self, param: &impl Param) {
        self.0.push(param.name().to_owned());
    }

    fn add_spacer(&mut self) {}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetterAction {
    Begin,
    Set,
    End,
}

#[derive(Default)]
struct RecordingGuiContext {
    actions: Mutex<Vec<SetterAction>>,
}

impl RecordingGuiContext {
    fn actions(&self) -> Vec<SetterAction> {
        self.actions.lock().unwrap().clone()
    }
}

impl GuiContext for RecordingGuiContext {
    fn plugin_api(&self) -> PluginApi {
        PluginApi::Standalone
    }

    fn request_resize(&self) -> bool {
        false
    }

    unsafe fn raw_begin_set_parameter(&self, _param: ParamPtr) {
        self.actions.lock().unwrap().push(SetterAction::Begin);
    }

    unsafe fn raw_set_parameter_normalized(&self, _param: ParamPtr, _normalized: f32) {
        self.actions.lock().unwrap().push(SetterAction::Set);
    }

    unsafe fn raw_end_set_parameter(&self, _param: ParamPtr) {
        self.actions.lock().unwrap().push(SetterAction::End);
    }

    fn get_state(&self) -> PluginState {
        PluginState { version: String::new(), params: BTreeMap::new(), fields: BTreeMap::new() }
    }

    fn set_state(&self, _state: PluginState) {}
}
