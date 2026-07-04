use std::{sync::Arc, time::Duration};

use bpm_detection_core::parameters::{NormalDistributionConfig, StaticBPMDetectionConfig};
use gui::GUIConfig;

use super::*;

fn assert_float_eq(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < f32::EPSILON, "{actual} != {expected}");
}

#[test]
fn plugin_params_use_parameter_accessors_to_read_initial_values() {
    let update_f32: Arc<dyn Fn(f32) + Send + Sync> = Arc::new(|_: f32| {});
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
    let std_dev =
        to_plugin_float_param(&normal_distribution_parameters.std_dev(), &normal_distribution_config, &update_f32);
    let factor =
        to_plugin_float_param(&normal_distribution_parameters.factor(), &normal_distribution_config, &update_f32);

    assert_float_eq(interpolation_duration.unmodulated_plain_value(), 0.82);
    assert_float_eq(interpolation_curve.unmodulated_plain_value(), 1.25);
    assert_float_eq(bpm_center.unmodulated_plain_value(), 111.5);
    assert_float_eq(std_dev.unmodulated_plain_value(), 18.0);
    assert_float_eq(factor.unmodulated_plain_value(), 41.0);
}
