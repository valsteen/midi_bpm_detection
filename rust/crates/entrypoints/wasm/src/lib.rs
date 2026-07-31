#![cfg(target_arch = "wasm32")]

use bpm_detection_config::{DynamicBPMDetectionConfig, Settings, StaticBPMDetectionConfig};
use bpm_detection_core::{TimedEvent, note_events::NoteOn};
use derivative::Derivative;
use errors::{LogErrorWithExt, error_backtrace};
use futures::channel::mpsc::Sender;
use gui::{BPMDetectionGUI, EditableSettings, GuiChanges, eframe};
use serde::{Deserialize, Serialize};

pub mod wasm;

const CONFIG: &str = include_str!("../config/base_config.toml");

#[derive(Clone, Derivative, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WASMConfig {
    #[serde(default, flatten)]
    pub bpm_detection: Settings,
}

impl WASMConfig {
    fn from_toml(config: &str) -> Result<Self, String> {
        let config =
            toml::de::Deserializer::parse(config).and_then(Self::deserialize).map_err(|err| err.to_string())?;
        config.validate()?;

        Ok(config)
    }

    fn validate(&self) -> Result<(), String> {
        self.bpm_detection.validate()
    }
}

enum QueueItem {
    StaticParameters(StaticBPMDetectionConfig),
    DynamicParameters(DynamicBPMDetectionConfig),
    Note(TimedEvent<NoteOn>),
    DelayedDynamicUpdate,
    DelayedStaticUpdate,
}

pub struct WasmApp {
    editable: EditableSettings,
    pub(crate) gui: BPMDetectionGUI,
    sender: Sender<QueueItem>,
}

impl WasmApp {
    fn new(config: WASMConfig, gui: BPMDetectionGUI, sender: Sender<QueueItem>) -> Self {
        Self { editable: EditableSettings { bpm: config.bpm_detection, send_tempo: None }, gui, sender }
    }

    fn commit(&mut self, changes: GuiChanges) {
        if changes.static_detection {
            let value = self.editable.bpm.static_bpm_detection_config.clone();
            self.sender.try_send(QueueItem::StaticParameters(value)).log_error_msg("channel full").ok();
        }
        if changes.dynamic_detection {
            let value = self.editable.bpm.dynamic_bpm_detection_config.clone();
            self.sender.try_send(QueueItem::DynamicParameters(value)).log_error_msg("channel full").ok();
        }
    }
}

impl eframe::App for WasmApp {
    fn logic(&mut self, _ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.gui.prepare();
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let changes = eframe::egui::CentralPanel::default().show(ui, |ui| self.gui.show(ui, &mut self.editable)).inner;
        self.commit(changes);
    }
}

impl Default for WASMConfig {
    fn default() -> Self {
        match Self::from_toml(CONFIG) {
            Ok(config) => config,
            Err(err) => {
                error_backtrace!("{err}");
                panic!("invalid built-in configuration");
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/lib.rs"]
mod tests;
