use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
};

use nih_plug::{
    context::PluginApi,
    params::{FloatParam, IntParam, Param, Params},
    prelude::{GuiContext, ParamPtr, ParamSetter, PluginState, RemoteControlsPage},
};
use parameter::parameter_group;
use parameter_nih_plug::{GeneratedNihPlugParams, MirrorHostParam, nih_plugin_parameter_group};
use parameter_on_off::OnOff;
use parameter_on_off_nih_plug::{OnOffF32Adapter, OnOffParam};

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
    #[nih_plugin_parameter(adapter = OnOffF32Adapter, callback = f32)]
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

#[test]
fn generated_group_implements_marker_trait() {
    fn assert_generated<T: GeneratedNihPlugParams>() {}

    assert_generated::<ExampleOnOffParams>();
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
    Callbacks { f32: Arc::new(|_: f32| {}), i32: Arc::new(|_: i32| {}) }
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
