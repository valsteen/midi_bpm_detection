use super::*;

#[test]
fn send_tempo_output_state_serializes_as_toml_boolean() {
    let config = PluginConfig { send_tempo: SendTempoOutputState::new(false), ..PluginConfig::default() };

    let serialized = toml::to_string(&config).expect("plugin config should serialize");
    let parsed: toml::Table = serialized.parse().expect("serialized config should be TOML");

    assert_eq!(parsed["send_tempo"].as_bool(), Some(false));
}

#[test]
fn send_tempo_output_state_deserializes_from_toml_boolean() {
    let config = PluginConfig::from_toml(&CONFIG.replace("send_tempo = true", "send_tempo = false"))
        .expect("boolean send_tempo should deserialize");

    assert!(!config.send_tempo.enabled());
}

#[test]
fn send_tempo_output_state_tracks_host_param_mirror_by_origin() {
    let state = SendTempoOutputState::new(false);

    state.set_from_host(true);
    assert!(state.enabled());
    assert!(!state.take_host_param_update_request());

    state.set_from_gui(false);
    assert!(!state.enabled());
    assert!(state.take_host_param_update_request());
    assert!(!state.take_host_param_update_request());

    state.set_from_gui(false);
    state.set_from_host(true);
    assert!(state.enabled());
    assert!(!state.take_host_param_update_request());

    state.toggle_from_shortcut();
    assert!(!state.enabled());
    state.set_from_host(true);
    assert!(state.enabled());
    assert!(!state.take_host_param_update_request());

    state.toggle_from_shortcut();
    assert!(!state.enabled());
    assert!(state.take_host_param_update_request());
    assert!(!state.take_host_param_update_request());
}

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
