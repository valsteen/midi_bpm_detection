use std::sync::atomic::Ordering;

use parameter_on_off::OnOff;

use super::*;

#[test]
fn base_config_contains_desktop_defaults() {
    let config = config::Config::builder()
        .add_source(config::File::from_str(CONFIG, config::FileFormat::Toml))
        .build()
        .expect("base desktop config should build")
        .try_deserialize::<DesktopConfig>()
        .expect("base desktop config should deserialize");

    assert_eq!(config.midi.device_name, "Desktop");
    assert!(!config.midi.enable_midi_clock.load(Ordering::Relaxed));
    assert!(!config.midi.send_tempo.load(Ordering::Relaxed));
    assert!((config.bpm_detection.static_bpm_detection_config.bpm_center - 90.0).abs() < f32::EPSILON);
    assert_eq!(config.bpm_detection.static_bpm_detection_config.bpm_range, 40);
    assert_eq!(config.bpm_detection.static_bpm_detection_config.sample_rate, 500);
    assert_eq!(config.bpm_detection.dynamic_bpm_detection_config.velocity_current_note_weight, OnOff::Off(0.7));
    assert_eq!(config.bpm_detection.dynamic_bpm_detection_config.multiplier_weight, OnOff::On(0.66));
    assert_eq!(config.bpm_detection.gui_config.interpolation_duration.as_millis(), 730);
}
