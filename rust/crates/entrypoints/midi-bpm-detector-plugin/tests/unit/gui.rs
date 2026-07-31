use gui::EditableSettings;
use parameter_on_off::OnOff;

use super::merge_host_changes;

fn editable_settings() -> EditableSettings {
    EditableSettings { bpm: bpm_detection_config::Settings::default(), send_tempo: Some(false) }
}

#[test]
fn host_and_gui_changes_to_different_dynamic_fields_survive() {
    let previous = editable_settings();
    let mut draft = previous.clone();
    draft.bpm.dynamic_bpm_detection_config.beats_lookback = 16;

    let mut current_host = previous.clone();
    current_host.bpm.dynamic_bpm_detection_config.time_distance_weight = OnOff::On(1.5);

    merge_host_changes(&mut draft, &previous, &current_host);

    assert_eq!(draft.bpm.dynamic_bpm_detection_config.beats_lookback, 16);
    assert_eq!(draft.bpm.dynamic_bpm_detection_config.time_distance_weight, OnOff::On(1.5));
}

#[test]
fn newer_observed_host_value_replaces_same_gui_draft_field() {
    let previous = editable_settings();
    let mut draft = previous.clone();
    draft.bpm.static_bpm_detection_config.bpm_center = 110.0;

    let mut current_host = previous.clone();
    current_host.bpm.static_bpm_detection_config.bpm_center = 125.0;

    merge_host_changes(&mut draft, &previous, &current_host);

    assert!((draft.bpm.static_bpm_detection_config.bpm_center - 125.0).abs() < f32::EPSILON);
}
