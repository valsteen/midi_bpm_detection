use bpm_detection_midi::MidiInputPort;
use gui::{
    BPMDetectionGUI, BpmDisplayPublisher, EditableSettings, GuiChanges, GuiContextHandle,
    eframe::{self, egui},
};

use crate::{
    config::DesktopConfig,
    controller_runtime::{DesktopControllerCommandQueue, SharedDesktopController},
    device_selection::DeviceSelection,
};

pub struct DesktopApp {
    config: DesktopConfig,
    editable: EditableSettings,
    gui: BPMDetectionGUI,
    context: GuiContextHandle,
    controller: SharedDesktopController<BpmDisplayPublisher>,
    commands: DesktopControllerCommandQueue<BpmDisplayPublisher>,
    displayed_devices: DeviceSelection,
    displayed_revision: u64,
    controller_busy: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DesktopChanges {
    refresh_devices: bool,
    selected_device_index: Option<usize>,
}

impl DesktopApp {
    #[must_use]
    pub fn new(
        config: DesktopConfig,
        gui: BPMDetectionGUI,
        context: GuiContextHandle,
        controller: SharedDesktopController<BpmDisplayPublisher>,
        commands: DesktopControllerCommandQueue<BpmDisplayPublisher>,
    ) -> Self {
        let editable = EditableSettings { bpm: config.bpm_detection.clone(), send_tempo: Some(config.midi.send_tempo) };
        Self {
            config,
            editable,
            gui,
            context,
            controller,
            commands,
            displayed_devices: DeviceSelection::new(),
            displayed_revision: u64::MAX,
            controller_busy: false,
        }
    }

    fn show_device_controls(&mut self, ui: &mut egui::Ui) -> DesktopChanges {
        let mut changes = DesktopChanges::default();
        ui.label("MIDI input");
        if self.controller_busy {
            ui.label("MIDI service is updating");
            ui.end_row();
            return changes;
        }

        let mut selected_index = self.displayed_devices.selected_index().unwrap_or_default();
        let mut selected_index_clicked = false;

        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("desktop-midi-input")
                .selected_text(
                    self.displayed_devices.displayed_selection().map_or("<none selected>", MidiInputPort::as_str),
                )
                .show_ui(ui, |ui| {
                    for (index, device) in self.displayed_devices.devices().iter().enumerate() {
                        selected_index_clicked |=
                            ui.selectable_value(&mut selected_index, index, device.as_str()).clicked();
                    }
                });

            #[cfg(not(target_os = "macos"))]
            if ui.button("Refresh MIDI inputs").clicked() {
                changes.refresh_devices = true;
            }
        });
        ui.end_row();

        changes.selected_device_index =
            select_displayed_device(&mut self.displayed_devices, selected_index, selected_index_clicked);
        changes
    }

    fn commit(&mut self, gui_changes: GuiChanges, desktop_changes: DesktopChanges) {
        if gui_changes.gui {
            self.config.bpm_detection.gui_config = self.editable.bpm.gui_config.clone();
        }
        if gui_changes.static_detection {
            let value = self.editable.bpm.static_bpm_detection_config.clone();
            self.config.bpm_detection.static_bpm_detection_config = value.clone();
            self.commands.apply_static_config(value);
        }
        if gui_changes.dynamic_detection {
            let value = self.editable.bpm.dynamic_bpm_detection_config.clone();
            self.config.bpm_detection.dynamic_bpm_detection_config = value.clone();
            self.commands.apply_dynamic_config(value);
        }
        if gui_changes.send_tempo {
            let enabled = self.editable.send_tempo.expect("desktop supports send tempo");
            self.config.midi.send_tempo = enabled;
            self.commands.set_send_tempo(enabled);
        }
        if desktop_changes.refresh_devices {
            self.commands.refresh_devices(self.context.clone());
        }
        if let Some(index) = desktop_changes.selected_device_index {
            self.commands.select_device_index(index);
        }
    }
}

fn select_displayed_device(
    selection: &mut DeviceSelection,
    selected_index: usize,
    selected_index_clicked: bool,
) -> Option<usize> {
    let selected_index_changed = Some(selected_index) != selection.selected_index();
    let confirmed_displayed_fallback = selected_index_clicked
        && selection.displayed_selection_is_fallback()
        && Some(selected_index) == selection.selected_index();
    if selection.devices().is_empty() || (!selected_index_changed && !confirmed_displayed_fallback) {
        return None;
    }
    selection.select_index(selected_index).map(|_| selected_index)
}

impl eframe::App for DesktopApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.gui.prepare();
        let Some(controller) = self.controller.try_lock() else {
            self.controller_busy = true;
            return;
        };
        self.controller_busy = false;
        let revision = controller.device_selection().revision();
        if revision != self.displayed_revision {
            self.displayed_devices = controller.device_selection().clone();
            self.displayed_revision = revision;
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let (desktop_changes, gui_changes) = egui::CentralPanel::default()
            .show(ui, |ui| {
                let desktop_changes = egui::Grid::new("desktop-controls")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .striped(true)
                    .show(ui, |ui| self.show_device_controls(ui))
                    .inner;
                let gui_changes = self.gui.show(ui, &mut self.editable);
                (desktop_changes, gui_changes)
            })
            .inner;
        self.commit(gui_changes, desktop_changes);
    }
}

#[cfg(test)]
#[path = "../tests/unit/app.rs"]
mod tests;
