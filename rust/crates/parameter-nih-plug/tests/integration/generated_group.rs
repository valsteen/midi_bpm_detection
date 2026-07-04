use std::sync::Arc;

use nih_plug::{
    params::{FloatParam, IntParam, Param, Params},
    prelude::RemoteControlsPage,
};
use parameter::{OnOff, parameter_group};
use parameter_nih_plug::{GeneratedNihPlugParams, OnOffParam, nih_plugin_parameter_group};

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

#[nih_plugin_parameter_group(config = ExampleOnOffConfig, group = "on_off")]
pub struct ExampleOnOffParams {
    #[nih_plugin_parameter(adapter = "on_off_f32")]
    pub weighted_gain: OnOffParam,
    pub plain_gain: FloatParam,
    pub steps: IntParam,
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

struct RemoteControlNames(Vec<String>);

impl RemoteControlsPage for RemoteControlNames {
    fn add_param(&mut self, param: &impl Param) {
        self.0.push(param.name().to_owned());
    }

    fn add_spacer(&mut self) {}
}
