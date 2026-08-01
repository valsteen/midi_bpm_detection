#![allow(clippy::missing_panics_doc)]
use bpm_detection_config::{DynamicBPMDetectionConfig, GUIConfig, Settings, StaticBPMDetectionConfig};
use errors::error_backtrace;
use futures::channel::mpsc;
use parameter_on_off::OnOff;
use serde::{Deserialize, Serialize};
#[allow(clippy::module_name_repetitions)]
use wasm_bindgen_test::*;

use super::{
    PendingGuiCommits, QueueItem, WASMConfig, WasmApp, retain_or_send_dynamic, retain_or_send_static,
    wasm::keyboard_event_generates_tap,
};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub test: OnOff<f32>,
}

impl Default for Config {
    fn default() -> Self {
        match toml::de::Deserializer::parse(CONFIG).and_then(Config::deserialize) {
            Ok(config) => config,
            Err(err) => {
                error_backtrace!("{err}");
                panic!("invalid built-in configuration");
            }
        }
    }
}

const CONFIG: &str = "[test]
enabled = false
value = 1";

fn wasm_app(sender: mpsc::Sender<QueueItem>) -> WasmApp {
    let (_, _, gui) = gui::create_gui();
    WasmApp::new(
        WASMConfig {
            bpm_detection: Settings {
                gui_config: GUIConfig::default(),
                dynamic_bpm_detection_config: DynamicBPMDetectionConfig::default(),
                static_bpm_detection_config: StaticBPMDetectionConfig::default(),
            },
        },
        gui,
        sender,
    )
}

#[wasm_bindgen_test]
fn test_config() {
    let config = Config::default();
    assert_eq!(config.test, OnOff::Off(1.0));
}

#[wasm_bindgen_test]
fn gui_changes_stay_local_without_queueing_detection_config() {
    let (sender, mut receiver) = mpsc::channel(4);
    let mut app = wasm_app(sender);

    app.editable.bpm.gui_config.interpolation_duration = std::time::Duration::from_millis(250);
    app.editable.bpm.gui_config.interpolation_curve = 0.35;
    app.commit(gui::GuiChanges { gui: true, ..gui::GuiChanges::default() });

    assert!(receiver.try_recv().is_err());
    assert_eq!(app.editable.bpm.gui_config.interpolation_duration, std::time::Duration::from_millis(250));
    assert!((app.editable.bpm.gui_config.interpolation_curve - 0.35).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn dynamic_change_receipt_queues_dynamic_parameters() {
    let (sender, mut receiver) = mpsc::channel(4);
    let mut app = wasm_app(sender);

    app.editable.bpm.dynamic_bpm_detection_config.beats_lookback = 12;
    app.commit(gui::GuiChanges { dynamic_detection: true, ..gui::GuiChanges::default() });
    app.flush_pending_commits(&gui::eframe::egui::Context::default());

    let queued = receiver.try_recv().expect("dynamic update should be queued");
    match queued {
        QueueItem::DynamicParameters(dynamic_config) => assert_eq!(dynamic_config.beats_lookback, 12),
        QueueItem::StaticParameters(_)
        | QueueItem::Note(_)
        | QueueItem::DelayedDynamicUpdate
        | QueueItem::DelayedStaticUpdate => panic!("expected dynamic parameters"),
    }
}

#[wasm_bindgen_test]
fn full_queue_retains_only_the_latest_static_commit() {
    let (mut sender, _receiver) = mpsc::channel(1);
    let full = loop {
        match sender.try_send(QueueItem::DelayedStaticUpdate) {
            Ok(()) => {}
            Err(error) => break error,
        }
    };
    assert!(full.is_full());
    let mut pending = PendingGuiCommits::default();

    let mut first = StaticBPMDetectionConfig::default();
    first.bpm_center = 100.0;
    retain_or_send_static(&mut sender, &mut pending, first);

    let mut latest = StaticBPMDetectionConfig::default();
    latest.bpm_center = 120.0;
    retain_or_send_static(&mut sender, &mut pending, latest.clone());

    assert_eq!(pending.static_detection, Some(latest));
}

#[wasm_bindgen_test]
fn full_queue_retains_only_the_latest_dynamic_commit() {
    let (mut sender, _receiver) = mpsc::channel(1);
    let full = loop {
        match sender.try_send(QueueItem::DelayedDynamicUpdate) {
            Ok(()) => {}
            Err(error) => break error,
        }
    };
    assert!(full.is_full());
    let mut pending = PendingGuiCommits::default();

    let mut first = DynamicBPMDetectionConfig::default();
    first.beats_lookback = 12;
    retain_or_send_dynamic(&mut sender, &mut pending, first);

    let mut latest = DynamicBPMDetectionConfig::default();
    latest.beats_lookback = 20;
    retain_or_send_dynamic(&mut sender, &mut pending, latest.clone());

    assert_eq!(pending.dynamic_detection, Some(latest));
}

#[wasm_bindgen_test]
fn unclaimed_keyboard_event_generates_tap() {
    assert!(keyboard_event_generates_tap(false, false));
}

#[wasm_bindgen_test]
fn keyboard_event_owned_by_egui_does_not_generate_tap() {
    assert!(!keyboard_event_generates_tap(false, true));
}

#[wasm_bindgen_test]
fn repeated_keyboard_event_does_not_generate_tap() {
    assert!(!keyboard_event_generates_tap(true, false));
}
