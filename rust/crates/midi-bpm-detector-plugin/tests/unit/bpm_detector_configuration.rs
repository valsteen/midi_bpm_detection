#[test]
fn live_config_uses_generated_mirror_methods() {
    let source = include_str!("../../src/bpm_detector_configuration.rs");

    assert!(
        !source.contains(".mirror_host_param("),
        "LiveConfig setters should call generated mirror_<field> methods instead of direct host-param mirroring"
    );
    assert!(
        !source.contains("use parameter_nih_plug::MirrorHostParam;"),
        "LiveConfig should not import MirrorHostParam directly after generated mirror methods own that pairing"
    );
}
