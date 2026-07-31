use super::ParameterGroupNaming;

#[test]
fn ordinary_config_names_preserve_generated_identifiers() {
    let naming = ParameterGroupNaming::new("ExampleConfig");

    assert_eq!(naming.base_name(), "Example");
    assert_eq!(naming.method_prefix(), "example");
    assert_eq!(naming.field_descriptor_name("sample_rate"), "ExampleSampleRateField");
}

#[test]
fn acronym_config_names_preserve_generated_identifiers() {
    let naming = ParameterGroupNaming::new("GUIConfig");

    assert_eq!(naming.base_name(), "GUI");
    assert_eq!(naming.method_prefix(), "gui");
    assert_eq!(naming.field_descriptor_name("interpolation_duration"), "GuiInterpolationDurationField");
}

#[test]
fn embedded_acronyms_preserve_generated_identifiers() {
    let naming = ParameterGroupNaming::new("DynamicBPMDetectionConfig");

    assert_eq!(naming.base_name(), "DynamicBPMDetection");
    assert_eq!(naming.method_prefix(), "dynamic_bpm_detection");
    assert_eq!(
        ParameterGroupNaming::new("DynamicBPMDetectionConfig").changed_field_mapper_name(),
        "DynamicBPMDetectionChangedFieldMapper",
    );
    assert_eq!(naming.field_descriptor_name("beats_lookback"), "DynamicBpmDetectionBeatsLookbackField",);
}
