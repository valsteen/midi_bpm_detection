use std::{
    marker::PhantomData,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig, StaticBPMDetectionConfigAccessor},
};
use bpm_detection_midi::MidiServiceConfig;
use gui::{GUIConfig, GUIConfigAccessor};
use sync::ArcAtomicBool;

use super::*;
use crate::config::{AppConfig, DesktopConfig};

#[derive(Clone)]
struct TestReceiver;

impl BPMDetectionReceiver for TestReceiver {
    fn receive_bpm_histogram_data(&mut self, _histogram_data_points: &[f32], _detected_bpm: f32) {}

    fn receive_daw_bpm(&self, _bpm: f32) {}
}

fn desktop_config() -> DesktopConfig {
    DesktopConfig {
        app: AppConfig::default(),
        gui_config: GUIConfig::default(),
        midi: MidiServiceConfig {
            device_name: "Desktop".to_string(),
            send_tempo: ArcAtomicBool::new(false),
            enable_midi_clock: ArcAtomicBool::new(false),
        },
        static_bpm_detection_config: StaticBPMDetectionConfig::default(),
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig::default(),
    }
}

fn base_config(
    static_changes: Arc<StdMutex<Vec<StaticBPMDetectionConfig>>>,
    dynamic_changes: Arc<StdMutex<Vec<DynamicBPMDetectionConfig>>>,
) -> DesktopBaseConfig<TestReceiver, (), ()> {
    DesktopBaseConfig {
        config: desktop_config(),
        controller: (),
        controller_commands: (),
        on_static_config_changed: Arc::new(move |config| {
            static_changes.lock().expect("static changes lock should not be poisoned").push(config);
        }),
        on_dynamic_config_changed: Arc::new(move |config| {
            dynamic_changes.lock().expect("dynamic changes lock should not be poisoned").push(config);
        }),
        receiver: PhantomData,
    }
}

#[test]
fn static_parameter_setter_propagates_static_config() {
    let static_changes = Arc::new(StdMutex::new(Vec::new()));
    let dynamic_changes = Arc::new(StdMutex::new(Vec::new()));
    let mut config = base_config(Arc::clone(&static_changes), Arc::clone(&dynamic_changes));

    config.set_bpm_center(120.0);

    let static_changes = static_changes.lock().expect("static changes lock should not be poisoned");
    let dynamic_changes = dynamic_changes.lock().expect("dynamic changes lock should not be poisoned");
    assert_eq!(static_changes.len(), 1);
    assert!((static_changes[0].bpm_center - 120.0).abs() < f32::EPSILON);
    assert!(dynamic_changes.is_empty());
}

#[test]
fn gui_parameter_setter_propagates_dynamic_config() {
    let static_changes = Arc::new(StdMutex::new(Vec::new()));
    let dynamic_changes = Arc::new(StdMutex::new(Vec::new()));
    let mut config = base_config(Arc::clone(&static_changes), Arc::clone(&dynamic_changes));

    config.set_interpolation_duration(Duration::from_millis(250));

    let static_changes = static_changes.lock().expect("static changes lock should not be poisoned");
    let dynamic_changes = dynamic_changes.lock().expect("dynamic changes lock should not be poisoned");
    assert!(static_changes.is_empty());
    assert_eq!(dynamic_changes.len(), 1);
    assert_eq!(config.interpolation_duration(), Duration::from_millis(250));
}
