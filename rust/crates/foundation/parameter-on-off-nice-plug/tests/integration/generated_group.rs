use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use nice_plug::{
    context::PluginApi,
    params::{FloatParam, IntParam, Param, Params},
    prelude::{GuiContext, ParamFlags, ParamPtr, ParamSetter, PluginState, RemoteControlsPage},
};
use parameter::parameter_group;
use parameter_nice_plug::{
    GeneratedNicePlugParams, MirrorChangedConfig, MirrorHostParams, nice_plugin_parameter_group,
};
use parameter_on_off::OnOff;
use parameter_on_off_nice_plug::{OnOffF32Adapter, OnOffParams};

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

#[nice_plugin_parameter_group(config = ExampleOnOffConfig, group = "on_off", accessor_macro = example_on_off_accessors)]
pub struct ExampleOnOffParams {
    #[nice_plugin_parameter(adapter = OnOffF32Adapter)]
    pub weighted_gain: OnOffParams,
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
    fn assert_generated<T: GeneratedNicePlugParams>() {}

    assert_generated::<ExampleOnOffParams>();
}

#[test]
fn on_off_adapter_exposes_enabled_then_value_with_stable_host_contract() {
    let on_change = on_change();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::Off(0.75), plain_gain: 1.25, steps: 4 };
    let params = ExampleOnOffParams::new(&source_config, &on_change);
    let param_map = params.param_map();
    let ids_and_groups = param_map.iter().map(|(id, _, group)| (id.as_str(), group.as_str())).collect::<Vec<_>>();
    let labels = param_map.iter().map(|(_, param, _)| unsafe { param.name().to_owned() }).collect::<Vec<_>>();
    let mut remote_controls = RemoteControlNames(Vec::new());

    params.add_remote_controls(&mut remote_controls);

    assert_eq!(
        ids_and_groups,
        [("weighted_gain_enabled", ""), ("weighted_gain", ""), ("plain_gain", ""), ("steps", ""),]
    );
    assert_eq!(labels, ["Weighted gain enabled", "Weighted gain", "Plain gain", "Steps"]);
    assert_eq!(remote_controls.0, ["Weighted gain enabled", "Weighted gain", "Plain gain", "Steps"]);
    for (_, param, _) in &param_map[..2] {
        let flags = unsafe { param.flags() };
        assert!(!flags.intersects(ParamFlags::NON_AUTOMATABLE | ParamFlags::HIDDEN | ParamFlags::HIDE_IN_GENERIC_UI));
    }
    assert_eq!(params.weighted_gain.read(), OnOff::Off(0.75));
    assert_eq!(params.read_config(), source_config);
    assert!(params.weighted_gain.serialize_fields().is_empty());
    assert!(params.serialize_fields().is_empty());
}

#[test]
fn on_off_adapter_combines_independently_applied_host_values() {
    let on_change = on_change();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.25, steps: 4 };
    let params = ExampleOnOffParams::new(&source_config, &on_change);
    let param_map = params.param_map();
    let enabled = param_map[0].1;
    let value = param_map[1].1;

    unsafe {
        enabled._internal_set_normalized_value(0.0);
        value._internal_set_normalized_value(value.preview_normalized(0.75));
    }

    assert_eq!(params.weighted_gain.read(), OnOff::Off(0.75));
    assert_eq!(params.read_config().weighted_gain, OnOff::Off(0.75));
}

#[test]
fn either_on_off_host_parameter_emits_the_same_logical_change_notification() {
    let change_count = Arc::new(AtomicUsize::new(0));
    let on_change = {
        let change_count = change_count.clone();
        move || {
            change_count.fetch_add(1, Ordering::Relaxed);
        }
    };
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.25, steps: 4 };
    let params = ExampleOnOffParams::new(&source_config, &on_change);
    let param_map = params.param_map();
    let enabled = param_map[0].1;
    let value = param_map[1].1;

    unsafe {
        enabled._internal_set_normalized_value(0.0);
        value._internal_set_normalized_value(value.preview_normalized(0.75));
    }

    assert_eq!(change_count.load(Ordering::Relaxed), 2);
}

#[test]
fn mirror_host_params_request_enabled_only_change_from_boolean_parameter() {
    let (params, mut config) = example_params(OnOff::On(0.5));
    let param_map = params.param_map();
    let enabled = param_map[0].1;
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);

    params.weighted_gain.mirror_host_params(
        &mut config,
        &ExampleOnOffConfig::PARAMETERS.weighted_gain(),
        OnOff::Off(0.5),
        &setter,
    );

    assert_eq!(config.weighted_gain, OnOff::Off(0.5));
    assert_eq!(params.weighted_gain.read(), OnOff::On(0.5));
    assert_eq!(context.actions(), setter_actions(enabled));
}

#[test]
fn mirror_host_params_request_value_only_change_from_float_parameter() {
    let (params, mut config) = example_params(OnOff::On(0.5));
    let param_map = params.param_map();
    let value = param_map[1].1;
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);

    params.weighted_gain.mirror_host_params(
        &mut config,
        &ExampleOnOffConfig::PARAMETERS.weighted_gain(),
        OnOff::On(0.75),
        &setter,
    );

    assert_eq!(config.weighted_gain, OnOff::On(0.75));
    assert_eq!(params.weighted_gain.read(), OnOff::On(0.5));
    assert_eq!(context.actions(), setter_actions(value));
}

#[test]
fn mirror_host_params_request_both_concrete_parameters_in_pair_order() {
    let (params, mut config) = example_params(OnOff::On(0.5));
    let param_map = params.param_map();
    let enabled = param_map[0].1;
    let value = param_map[1].1;
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);

    params.weighted_gain.mirror_host_params(
        &mut config,
        &ExampleOnOffConfig::PARAMETERS.weighted_gain(),
        OnOff::Off(0.75),
        &setter,
    );

    let mut expected = setter_actions(enabled);
    expected.extend(setter_actions(value));
    assert_eq!(config.weighted_gain, OnOff::Off(0.75));
    assert_eq!(params.weighted_gain.read(), OnOff::On(0.5));
    assert_eq!(context.actions(), expected);
}

#[test]
fn mirror_changed_config_routes_enabled_only_change_through_boolean_parameter() {
    let on_change = on_change();
    let before = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 };
    let params = ExampleOnOffParams::new(&before, &on_change);
    let enabled = params.param_map()[0].1;
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let after = ExampleOnOffConfig { weighted_gain: OnOff::Off(0.5), ..before.clone() };

    let mirrored = params.mirror_changed_config(&before, &after, &setter);

    assert_eq!(mirrored.weighted_gain, OnOff::Off(0.5));
    assert_eq!(params.weighted_gain.read(), OnOff::On(0.5));
    assert_eq!(context.actions(), setter_actions(enabled));
}

#[test]
fn generated_field_mirror_methods_use_parameter_field_descriptor_value_types() {
    let on_change = on_change();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 };
    let params = ExampleOnOffParams::new(&source_config, &on_change);
    let param_map = params.param_map();
    let enabled = param_map[0].1;
    let value = param_map[1].1;
    let context = RecordingGuiContext::default();
    let setter = ParamSetter::new(&context);
    let mut config = source_config;

    params.mirror_weighted_gain(&mut config, OnOff::Off(0.625), &setter);

    let mut expected = setter_actions(enabled);
    expected.extend(setter_actions(value));
    assert_eq!(config.weighted_gain, OnOff::Off(0.625));
    assert_eq!(context.actions(), expected);
}

#[test]
fn generated_accessor_helper_implements_live_accessor_without_repeating_fields() {
    let on_change = on_change();
    let source_config = ExampleOnOffConfig { weighted_gain: OnOff::On(0.5), plain_gain: 1.0, steps: 3 };
    let params = ExampleOnOffParams::new(&source_config, &on_change);
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
    assert_eq!(context.actions().len(), 12);
}

fn example_params(weighted_gain: OnOff<f32>) -> (ExampleOnOffParams, ExampleOnOffConfig) {
    let on_change = on_change();
    let config = ExampleOnOffConfig { weighted_gain, plain_gain: 1.0, steps: 3 };
    let params = ExampleOnOffParams::new(&config, &on_change);
    (params, config)
}

fn on_change() -> impl Fn() + Clone + Send + Sync + 'static {
    || {}
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

struct RemoteControlNames(Vec<String>);

impl RemoteControlsPage for RemoteControlNames {
    fn add_param(&mut self, param: &impl Param) {
        self.0.push(param.name().to_owned());
    }

    fn add_spacer(&mut self) {}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SetterAction {
    Begin(ParamPtr),
    Set(ParamPtr),
    End(ParamPtr),
}

fn setter_actions(param: ParamPtr) -> Vec<SetterAction> {
    vec![SetterAction::Begin(param), SetterAction::Set(param), SetterAction::End(param)]
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

    unsafe fn raw_begin_set_parameter(&self, param: ParamPtr) {
        self.actions.lock().unwrap().push(SetterAction::Begin(param));
    }

    unsafe fn raw_set_parameter_normalized(&self, param: ParamPtr, _normalized: f32) {
        self.actions.lock().unwrap().push(SetterAction::Set(param));
    }

    unsafe fn raw_end_set_parameter(&self, param: ParamPtr) {
        self.actions.lock().unwrap().push(SetterAction::End(param));
    }

    fn get_state(&self) -> PluginState {
        PluginState { version: String::new(), params: BTreeMap::new(), fields: BTreeMap::new() }
    }

    fn set_state(&self, _state: PluginState) {}
}
