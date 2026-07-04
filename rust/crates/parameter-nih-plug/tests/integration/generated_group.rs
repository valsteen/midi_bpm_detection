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
use parameter::{OnOff, parameter_group};
use parameter_nih_plug::{GeneratedNihPlugParams, MirrorHostParam, OnOffParam, nih_plugin_parameter_group};

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
pub struct ExampleOnOffConfig {
    #[parameter(label = "Weighted gain", range = 0.0..=1.0, default = OnOff::On(0.5))]
    pub weighted_gain: OnOff<f32>,
    #[parameter(label = "Plain gain", range = 0.0..=2.0, default = 1.0)]
    pub plain_gain: f32,
    #[parameter(label = "Steps", range = 0.0..=8.0, default = 3)]
    pub steps: u8,
}

#[nih_plugin_parameter_group(config = ExampleOnOffConfig, group = "on_off", accessor_macro = example_on_off_accessors)]
pub struct ExampleOnOffParams {
    #[nih_plugin_parameter(adapter = "on_off_f32")]
    pub weighted_gain: OnOffParam,
    pub plain_gain: FloatParam,
    pub steps: IntParam,
}

example_on_off_accessors! {
    target = ExampleOnOffLive<'_, '_>,
    config = self.config,
    params = self.params,
    param_setter = self.setter,
    after_set = self.after_set(),
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
fn on_off_adapter_persists_enabled_state_and_reads_config() {
    let callbacks = callbacks();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::Off(0.75), plain_gain: 1.25, steps: 4 };
    let params = ExampleOnOffParams::new(&source_config, &callbacks.f32, &callbacks.i32);

    assert_eq!(params.weighted_gain.param().name(), "Weighted gain");
    assert!(!params.weighted_gain.is_enabled());
    assert_eq!(params.weighted_gain.read(), OnOff::Off(0.75));
    assert_eq!(params.read_config(), source_config);

    let serialized = params.weighted_gain.serialize_fields();
    assert!(serialized.contains_key("weighted_gain_onoff"));

    let enabled_params = ExampleOnOffParams::new(
        &ExampleOnOffConfig { weighted_gain: OnOff::On(0.75), plain_gain: 1.25, steps: 4 },
        &callbacks.f32,
        &callbacks.i32,
    );
    enabled_params.weighted_gain.deserialize_fields(&serialized);

    assert!(!enabled_params.weighted_gain.is_enabled());
}

#[test]
fn on_off_adapter_enabled_state_can_be_set_without_param_setter_policy() {
    let callbacks = callbacks();
    let params = ExampleOnOffParams::new(
        &ExampleOnOffConfig { weighted_gain: OnOff::On(0.75), plain_gain: 1.25, steps: 4 },
        &callbacks.f32,
        &callbacks.i32,
    );

    params.weighted_gain.set_enabled(false);

    assert_eq!(params.weighted_gain.read(), OnOff::Off(0.75));
    assert_eq!(params.weighted_gain.serialize_fields()["weighted_gain_onoff"], "false");
}

#[test]
fn generated_group_persists_on_off_enabled_state() {
    let callbacks = callbacks();
    let params = ExampleOnOffParams::new(
        &ExampleOnOffConfig { weighted_gain: OnOff::Off(0.75), plain_gain: 1.25, steps: 4 },
        &callbacks.f32,
        &callbacks.i32,
    );

    let serialized = params.serialize_fields();

    assert!(serialized.contains_key("weighted_gain_onoff"));

    let enabled_params = ExampleOnOffParams::new(
        &ExampleOnOffConfig { weighted_gain: OnOff::On(0.75), plain_gain: 1.25, steps: 4 },
        &callbacks.f32,
        &callbacks.i32,
    );
    enabled_params.deserialize_fields(&serialized);

    assert!(!enabled_params.weighted_gain.is_enabled());
}

#[test]
fn on_off_adapter_maps_ids_and_remote_controls_in_source_order() {
    let callbacks = callbacks();
    let params = ExampleOnOffParams::new(
        &ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.25, steps: 4 },
        &callbacks.f32,
        &callbacks.i32,
    );
    let ids_and_groups = params.param_map().into_iter().map(|(id, _, group)| (id, group)).collect::<Vec<_>>();
    let mut remote_controls = RemoteControlNames(Vec::new());

    params.add_remote_controls(&mut remote_controls);

    assert_eq!(
        ids_and_groups,
        [
            (String::from("weighted_gain"), String::new()),
            (String::from("plain_gain"), String::new()),
            (String::from("steps"), String::new()),
        ]
    );
    assert_eq!(remote_controls.0, ["Weighted gain", "Plain gain", "Steps"]);
}

#[test]
fn generated_group_implements_marker_trait() {
    fn assert_generated<T: GeneratedNihPlugParams>() {}

    assert_generated::<ExampleChildParams>();
    assert_generated::<ExampleParentParams>();
    assert_generated::<ExampleOnOffParams>();
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
    let on_off_params = ExampleOnOffParams::new(
        &ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 },
        &callbacks.f32,
        &callbacks.i32,
    );
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;
    let mut child_config = config.child.clone();
    let mut duration_config = ExampleDurationConfig { delay: Duration::from_secs_f32(0.5), curve: 0.7 };
    let mut on_off_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 };

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
    on_off_params.steps.mirror_host_param(&mut on_off_config, &ExampleOnOffConfig::PARAMETERS.steps(), 5, &setter);

    assert!((config.gain - 1.75).abs() < f32::EPSILON);
    assert_eq!(config.count, 6);
    assert_eq!(config.sample_rate, 720);
    assert!((child_config.child_precise - 7.25).abs() < f64::EPSILON);
    assert_eq!(duration_config.delay, Duration::from_secs_f32(0.25));
    assert_eq!(on_off_config.steps, 5);
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End].repeat(6));
}

#[test]
fn mirror_host_param_preserves_on_off_enabled_only_updates() {
    let callbacks = callbacks();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 };
    let params = ExampleOnOffParams::new(&source_config, &callbacks.f32, &callbacks.i32);
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;

    params.weighted_gain.mirror_host_param(
        &mut config,
        &ExampleOnOffConfig::PARAMETERS.weighted_gain(),
        OnOff::Off(0.5),
        &setter,
    );

    assert_eq!(config.weighted_gain, OnOff::Off(0.5));
    assert!(!params.weighted_gain.is_enabled());
    assert_eq!(context.actions(), []);

    params.weighted_gain.mirror_host_param(
        &mut config,
        &ExampleOnOffConfig::PARAMETERS.weighted_gain(),
        OnOff::On(0.75),
        &setter,
    );

    assert_eq!(config.weighted_gain, OnOff::On(0.75));
    assert!(params.weighted_gain.is_enabled());
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End]);
}

#[test]
fn generated_field_mirror_methods_use_parameter_field_descriptor_value_types() {
    let callbacks = callbacks();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 };
    let params = ExampleOnOffParams::new(&source_config, &callbacks.f32, &callbacks.i32);
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;

    params.mirror_weighted_gain(&mut config, OnOff::Off(0.625), &setter);

    assert_eq!(config.weighted_gain, OnOff::Off(0.625));
    assert!(!params.weighted_gain.is_enabled());
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End]);
}

#[test]
fn generated_accessor_helper_implements_live_accessor_without_repeating_fields() {
    let callbacks = callbacks();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 };
    let params = ExampleOnOffParams::new(&source_config, &callbacks.f32, &callbacks.i32);
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut live = ExampleOnOffLive { config: source_config, params, setter: &setter, after_set_count: 0 };

    assert_eq!(live.weighted_gain(), OnOff::On(0.5));
    assert!((live.plain_gain() - 1.0).abs() < f32::EPSILON);
    assert_eq!(live.steps(), 3);

    live.set_weighted_gain(OnOff::Off(0.625));
    live.set_plain_gain(1.75);
    live.set_steps(6);

    assert_eq!(live.config.weighted_gain, OnOff::Off(0.625));
    assert!((live.config.plain_gain - 1.75).abs() < f32::EPSILON);
    assert_eq!(live.config.steps, 6);
    assert_eq!(live.after_set_count, 3);
    assert_eq!(context.actions(), [SetterAction::Begin, SetterAction::Set, SetterAction::End].repeat(3));
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

struct ExampleOnOffLive<'a, 'setter> {
    config: ExampleOnOffConfig,
    params: ExampleOnOffParams,
    setter: &'setter ParamSetter<'a>,
    after_set_count: usize,
}

impl ExampleOnOffLive<'_, '_> {
    fn after_set(&mut self) {
        self.after_set_count += 1;
    }
}

fn callbacks() -> Callbacks {
    Callbacks { f32: callback_f32(), i32: Arc::new(|_: i32| {}) }
}

fn callback_f32() -> Arc<dyn Fn(f32) + Send + Sync> {
    Arc::new(|_: f32| {})
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
