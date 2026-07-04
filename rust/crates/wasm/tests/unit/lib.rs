#![allow(clippy::missing_panics_doc)]
use bpm_detection_core::parameters::{
    DynamicBPMDetectionConfig, DynamicBPMDetectionConfigAccessor, StaticBPMDetectionConfig,
};
use errors::error_backtrace;
use futures::channel::mpsc;
use gui::{GUIConfig, GUIConfigAccessor};
use parameter_on_off::OnOff;
use serde::{Deserialize, Serialize};
#[allow(clippy::module_name_repetitions)]
use wasm_bindgen_test::*;

use super::{BaseConfig, QueueItem, WASMConfig};

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

fn base_config(sender: mpsc::Sender<QueueItem>) -> BaseConfig {
    BaseConfig {
        config: WASMConfig {
            gui_config: GUIConfig::default(),
            dynamic_bpm_detection_config: DynamicBPMDetectionConfig::default(),
            static_bpm_detection_config: StaticBPMDetectionConfig::default(),
        },
        sender,
    }
}

#[wasm_bindgen_test]
fn test_config() {
    let config = Config::default();
    assert_eq!(config.test, OnOff::Off(1.0));
}

#[wasm_bindgen_test]
fn built_in_wasm_config_matches_wasm_schema() {
    let config = WASMConfig::default();
    assert_eq!(config.static_bpm_detection_config.bpm_center, 100.0);
}

#[wasm_bindgen_test]
fn gui_parameter_setters_update_local_config_without_queueing_detection_config() {
    let (sender, mut receiver) = mpsc::channel(4);
    let mut config = base_config(sender);

    config.set_interpolation_duration(std::time::Duration::from_millis(250));
    config.set_interpolation_curve(0.35);

    assert!(receiver.try_recv().is_err());
    assert_eq!(config.interpolation_duration(), std::time::Duration::from_millis(250));
    assert!((config.interpolation_curve() - 0.35).abs() < f32::EPSILON);
}

#[wasm_bindgen_test]
fn dynamic_parameter_setter_queues_dynamic_parameters() {
    let (sender, mut receiver) = mpsc::channel(4);
    let mut config = base_config(sender);

    config.set_beats_lookback(12);

    let queued = receiver.try_recv().expect("dynamic update should be queued");
    match queued {
        QueueItem::DynamicParameters(dynamic_config) => assert_eq!(dynamic_config.beats_lookback, 12),
        QueueItem::StaticParameters(_)
        | QueueItem::Note(_)
        | QueueItem::DelayedDynamicUpdate
        | QueueItem::DelayedStaticUpdate => panic!("expected dynamic parameters"),
    }
}
