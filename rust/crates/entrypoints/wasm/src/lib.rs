#![cfg(target_arch = "wasm32")]

use bpm_detection_config::{DynamicBPMDetectionConfig, Settings, StaticBPMDetectionConfig};
use bpm_detection_core::{TimedEvent, note_events::NoteOn};
use derivative::Derivative;
use errors::{error, error_backtrace};
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

#[derive(Default)]
struct PendingGuiCommits {
    static_detection: Option<StaticBPMDetectionConfig>,
    dynamic_detection: Option<DynamicBPMDetectionConfig>,
}

pub struct WasmApp {
    editable: EditableSettings,
    pub(crate) gui: BPMDetectionGUI,
    sender: Sender<QueueItem>,
    pending: PendingGuiCommits,
}

impl WasmApp {
    fn new(config: WASMConfig, gui: BPMDetectionGUI, sender: Sender<QueueItem>) -> Self {
        Self {
            editable: EditableSettings { bpm: config.bpm_detection, send_tempo: None },
            gui,
            sender,
            pending: PendingGuiCommits::default(),
        }
    }

    fn commit(&mut self, changes: GuiChanges) {
        if changes.static_detection {
            self.pending.static_detection = Some(self.editable.bpm.static_bpm_detection_config.clone());
        }
        if changes.dynamic_detection {
            self.pending.dynamic_detection = Some(self.editable.bpm.dynamic_bpm_detection_config.clone());
        }
    }

    fn flush_pending_commits(&mut self, context: &eframe::egui::Context) {
        if let Some(value) = self.pending.static_detection.take() {
            retain_or_send_static(&mut self.sender, &mut self.pending, value);
        }
        if let Some(value) = self.pending.dynamic_detection.take() {
            retain_or_send_dynamic(&mut self.sender, &mut self.pending, value);
        }
        if self.pending.static_detection.is_some() || self.pending.dynamic_detection.is_some() {
            context.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }
}

fn retain_or_send_static(
    sender: &mut Sender<QueueItem>,
    pending: &mut PendingGuiCommits,
    value: StaticBPMDetectionConfig,
) {
    match sender.try_send(QueueItem::StaticParameters(value.clone())) {
        Ok(()) => pending.static_detection = None,
        Err(send_error) if send_error.is_full() => pending.static_detection = Some(value),
        Err(send_error) => {
            error!("WASM detector queue is closed: {send_error}");
            pending.static_detection = None;
        }
    }
}

fn retain_or_send_dynamic(
    sender: &mut Sender<QueueItem>,
    pending: &mut PendingGuiCommits,
    value: DynamicBPMDetectionConfig,
) {
    match sender.try_send(QueueItem::DynamicParameters(value.clone())) {
        Ok(()) => pending.dynamic_detection = None,
        Err(send_error) if send_error.is_full() => pending.dynamic_detection = Some(value),
        Err(send_error) => {
            error!("WASM detector queue is closed: {send_error}");
            pending.dynamic_detection = None;
        }
    }
}

impl eframe::App for WasmApp {
    fn logic(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        self.gui.prepare();
        self.flush_pending_commits(ctx);
    }

    fn ui(&mut self, ui: &mut eframe::egui::Ui, _frame: &mut eframe::Frame) {
        let changes = eframe::egui::CentralPanel::default().show(ui, |ui| self.gui.show(ui, &mut self.editable)).inner;
        self.commit(changes);
        self.flush_pending_commits(ui.ctx());
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
