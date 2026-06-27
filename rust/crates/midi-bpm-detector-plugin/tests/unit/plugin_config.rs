use super::*;

#[test]
fn plugin_config_rejects_stale_dynamic_parameter_keys() {
    let config_with_stale_key = CONFIG.replace("high_tempo_bias_weight", "high_tempo_bias");

    let message = PluginConfig::from_toml(&config_with_stale_key).expect_err("stale key should be rejected");

    assert!(message.contains("unknown field"));
    assert!(message.contains("high_tempo_bias"));
}

#[test]
fn plugin_config_rejects_parameter_values_outside_declared_ranges() {
    let config_with_out_of_range_value = CONFIG.replace("bpm_center = 100.0", "bpm_center = 1000.0");

    let message =
        PluginConfig::from_toml(&config_with_out_of_range_value).expect_err("out-of-range value should be rejected");

    assert!(message.contains("BPM center"));
    assert!(message.contains("1000"));
    assert!(message.contains("1..=150"));
}
