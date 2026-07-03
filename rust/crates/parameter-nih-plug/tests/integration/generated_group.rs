use std::sync::Arc;

use nih_plug::params::{FloatParam, Param, Params};
use parameter::parameter_group;
use parameter_nih_plug::{GeneratedNihPlugParams, nih_plugin_parameter_group};

#[parameter_group]
#[derive(Clone, PartialEq, Debug)]
pub struct ExampleConfig {
    #[parameter(label = "Gain", range = 0.0..=2.0, default = 1.0)]
    pub gain: f32,
    #[parameter(label = "Precise", range = 0.0..=10.0, default = 3.5)]
    pub precise: f64,
}

#[nih_plugin_parameter_group(config = ExampleConfig, group = "example")]
pub struct ExampleParams {
    pub gain: FloatParam,
    pub precise: FloatParam,
}

#[test]
fn generated_group_maps_field_ids_in_catalog_order_without_local_groups() {
    let callback = callback();
    let params = ExampleParams::new(&ExampleConfig { gain: 1.25, precise: 4.75 }, &callback);
    let ids_and_groups = params.param_map().into_iter().map(|(id, _, group)| (id, group)).collect::<Vec<_>>();

    assert_eq!(ids_and_groups, [(String::from("gain"), String::new()), (String::from("precise"), String::new())]);
}

#[test]
fn generated_group_reads_host_values_back_to_config() {
    let callback = callback();
    let source_config = ExampleConfig { gain: 1.25, precise: 4.75 };
    let params = ExampleParams::new(&source_config, &callback);

    assert_eq!(params.read_config(), source_config);
}

#[test]
fn generated_group_preserves_parameter_metadata() {
    let callback = callback();
    let params = ExampleParams::new(&ExampleConfig { gain: 1.25, precise: 4.75 }, &callback);

    assert_eq!(params.gain.name(), "Gain");
    assert_eq!(params.precise.name(), "Precise");
}

#[test]
fn generated_group_implements_marker_trait() {
    fn assert_generated<T: GeneratedNihPlugParams>() {}

    assert_generated::<ExampleParams>();
}

fn callback() -> Arc<dyn Fn(f32) + Send + Sync> {
    Arc::new(|_: f32| {})
}
