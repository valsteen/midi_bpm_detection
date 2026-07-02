use std::{sync::Arc, time::Duration};

use bpm_detection_core::parameters::{DynamicBPMDetectionConfig, NormalDistributionConfig, StaticBPMDetectionConfig};
use gui::GUIConfig;
use nih_plug::prelude::Params;
use parameter::OnOff;

use super::*;

fn assert_float_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON, "{actual} != {expected}");
}

#[test]
fn plugin_on_off_param_exposes_host_id_and_persisted_enabled_key() {
    let callback: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(|_: f32| {});
    let plugin_param = PluginOnOffParam::new(
        "time_distance_weight",
        DynamicBPMDetectionConfig::PARAMETERS.time_distance_weight().to_param(OnOff::Off(1.5), &callback),
        OnOff::Off(1.5),
    );

    let param_ids = plugin_param.param_map().into_iter().map(|(id, _, _)| id).collect::<Vec<_>>();
    let serialized = plugin_param.serialize_fields();

    assert_eq!(param_ids, ["time_distance_weight"]);
    assert_eq!(serialized["time_distance_weight_onoff"], "false");
    assert_eq!(plugin_param.read(), OnOff::Off(1.5));
}

#[test]
fn plugin_on_off_param_uses_parameter_accessor_to_read_initial_value() {
    let callback: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(|_: f32| {});
    let config = DynamicBPMDetectionConfig { time_distance_weight: OnOff::Off(1.5), ..Default::default() };

    let plugin_param =
        to_plugin_on_off_param(&DynamicBPMDetectionConfig::PARAMETERS.time_distance_weight_field(), &config, &callback);

    assert_eq!(plugin_param.read(), OnOff::Off(1.5));
}

#[test]
fn plugin_int_param_uses_parameter_accessor_to_read_initial_value() {
    let callback: Arc<dyn Fn(i32) + Send + Sync> = Arc::new(|_: i32| {});
    let config = DynamicBPMDetectionConfig { beats_lookback: 13, ..Default::default() };

    let plugin_param = to_plugin_int_param(&DynamicBPMDetectionConfig::PARAMETERS.beats_lookback(), &config, &callback);

    assert_eq!(plugin_param.unmodulated_plain_value(), 13);
}

#[test]
fn plugin_params_use_parameter_accessors_to_read_initial_values() {
    let update_f32: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(|_: f32| {});
    let update_i32: Arc<dyn Fn(i32) + Send + Sync> = Arc::new(|_: i32| {});
    let gui_config = GUIConfig { interpolation_duration: Duration::from_millis(820), interpolation_curve: 1.25 };
    let static_config =
        StaticBPMDetectionConfig { bpm_center: 111.5, bpm_range: 48, sample_rate: 720, ..Default::default() };
    let normal_distribution_config = NormalDistributionConfig { std_dev: 18.0, factor: 41.0, ..Default::default() };
    let gui_parameters = GUIConfig::PARAMETERS;
    let static_parameters = StaticBPMDetectionConfig::PARAMETERS;
    let normal_distribution_parameters = NormalDistributionConfig::PARAMETERS;

    let interpolation_duration =
        to_plugin_duration_param(&gui_parameters.interpolation_duration(), &gui_config, &update_f32);
    let interpolation_curve = to_plugin_float_param(&gui_parameters.interpolation_curve(), &gui_config, &update_f32);
    let bpm_center = to_plugin_float_param(&static_parameters.bpm_center(), &static_config, &update_f32);
    let bpm_range = to_plugin_int_param(&static_parameters.bpm_range(), &static_config, &update_i32);
    let sample_rate = to_plugin_u16_logarithmic_param(&static_parameters.sample_rate(), &static_config, &update_f32);
    let std_dev =
        to_plugin_float_param(&normal_distribution_parameters.std_dev(), &normal_distribution_config, &update_f32);
    let factor =
        to_plugin_float_param(&normal_distribution_parameters.factor(), &normal_distribution_config, &update_f32);

    assert_float_eq(interpolation_duration.unmodulated_plain_value(), 0.82);
    assert_float_eq(interpolation_curve.unmodulated_plain_value(), 1.25);
    assert_float_eq(bpm_center.unmodulated_plain_value(), 111.5);
    assert_eq!(bpm_range.unmodulated_plain_value(), 48);
    assert_float_eq(sample_rate.unmodulated_plain_value(), 720.0);
    assert_float_eq(std_dev.unmodulated_plain_value(), 18.0);
    assert_float_eq(factor.unmodulated_plain_value(), 41.0);
}
