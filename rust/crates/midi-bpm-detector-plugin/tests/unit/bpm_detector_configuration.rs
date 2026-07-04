#[test]
fn live_config_uses_generated_mirror_methods() {
    let source = include_str!("../../src/bpm_detector_configuration.rs");

    for manual_impl in [
        "impl NormalDistributionConfigAccessor for LiveConfig<'_>",
        "impl DynamicBPMDetectionConfigAccessor for LiveConfig<'_>",
        "impl StaticBPMDetectionConfigAccessor for LiveConfig<'_>",
        "impl GUIConfigAccessor for LiveConfig<'_>",
    ] {
        assert!(
            !source.contains(manual_impl),
            "LiveConfig should use generated accessor helper macros instead of `{manual_impl}`"
        );
    }
    assert!(
        !source.contains(".mirror_host_param("),
        "LiveConfig setters should call generated mirror_<field> methods instead of direct host-param mirroring"
    );
    assert!(
        !source.contains("use parameter_nih_plug::MirrorHostParam;"),
        "LiveConfig should not import MirrorHostParam directly after generated mirror methods own that pairing"
    );
}
