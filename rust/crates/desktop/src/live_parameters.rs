use std::{
    marker::PhantomData,
    sync::{Arc, atomic::Ordering},
};

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{
        DynamicBPMDetectionConfig, DynamicBPMDetectionConfigOwner, NormalDistributionConfig,
        NormalDistributionConfigOwner, StaticBPMDetectionConfig, StaticBPMDetectionConfigOwner,
    },
};
use bpm_detection_midi::MidiInputPort;
use errors::LogErrorWithExt;
use gui::{BPMDetectionConfig, GUIConfig, GUIConfigOwner};

use crate::{
    config::DesktopConfig,
    controller_runtime::{DesktopControllerCommandQueue, SharedDesktopController},
};

pub type StaticConfigCallback = Arc<dyn Fn(StaticBPMDetectionConfig) + Send + Sync>;
pub type DynamicConfigCallback = Arc<dyn Fn(DynamicBPMDetectionConfig) + Send + Sync>;

pub struct DesktopBaseConfig<B, Controller = SharedDesktopController<B>, Commands = DesktopControllerCommandQueue<B>>
where
    B: BPMDetectionReceiver,
{
    pub config: DesktopConfig,
    pub controller: Controller,
    pub controller_commands: Commands,
    pub on_static_config_changed: StaticConfigCallback,
    pub on_dynamic_config_changed: DynamicConfigCallback,
    receiver: PhantomData<B>,
}

impl<B> DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    pub fn new(
        config: DesktopConfig,
        controller: SharedDesktopController<B>,
        controller_commands: DesktopControllerCommandQueue<B>,
        on_static_config_changed: StaticConfigCallback,
        on_dynamic_config_changed: DynamicConfigCallback,
    ) -> Self {
        Self {
            config,
            controller,
            controller_commands,
            on_static_config_changed,
            on_dynamic_config_changed,
            receiver: PhantomData,
        }
    }
}

impl<B, Controller, Commands> DesktopBaseConfig<B, Controller, Commands>
where
    B: BPMDetectionReceiver,
{
    pub fn propagate_static_changes(&self) {
        (self.on_static_config_changed)(self.config.static_bpm_detection_config.clone());
    }

    pub fn propagate_dynamic_changes(&self) {
        (self.on_dynamic_config_changed)(self.config.dynamic_bpm_detection_config.clone());
    }
}

impl<B, Controller, Commands> NormalDistributionConfigOwner for DesktopBaseConfig<B, Controller, Commands>
where
    B: BPMDetectionReceiver,
{
    fn normal_distribution_config(&self) -> &NormalDistributionConfig {
        &self.config.static_bpm_detection_config.normal_distribution
    }

    fn normal_distribution_config_mut(&mut self) -> &mut NormalDistributionConfig {
        &mut self.config.static_bpm_detection_config.normal_distribution
    }

    fn after_normal_distribution_config_set(&mut self) {
        self.propagate_static_changes();
    }
}

impl<B, Controller, Commands> DynamicBPMDetectionConfigOwner for DesktopBaseConfig<B, Controller, Commands>
where
    B: BPMDetectionReceiver,
{
    fn dynamic_bpm_detection_config(&self) -> &DynamicBPMDetectionConfig {
        &self.config.dynamic_bpm_detection_config
    }

    fn dynamic_bpm_detection_config_mut(&mut self) -> &mut DynamicBPMDetectionConfig {
        &mut self.config.dynamic_bpm_detection_config
    }

    fn after_dynamic_bpm_detection_config_set(&mut self) {
        self.propagate_dynamic_changes();
    }
}

impl<B, Controller, Commands> StaticBPMDetectionConfigOwner for DesktopBaseConfig<B, Controller, Commands>
where
    B: BPMDetectionReceiver,
{
    fn static_bpm_detection_config(&self) -> &StaticBPMDetectionConfig {
        &self.config.static_bpm_detection_config
    }

    fn static_bpm_detection_config_mut(&mut self) -> &mut StaticBPMDetectionConfig {
        &mut self.config.static_bpm_detection_config
    }

    fn after_static_bpm_detection_config_set(&mut self) {
        self.propagate_static_changes();
    }
}

impl<B, Controller, Commands> GUIConfigOwner for DesktopBaseConfig<B, Controller, Commands>
where
    B: BPMDetectionReceiver,
{
    fn gui_config(&self) -> &GUIConfig {
        &self.config.gui_config
    }

    fn gui_config_mut(&mut self) -> &mut GUIConfig {
        &mut self.config.gui_config
    }
}

impl<B> BPMDetectionConfig for DesktopBaseConfig<B>
where
    B: BPMDetectionReceiver,
{
    fn get_send_tempo(&self) -> bool {
        self.config.midi.send_tempo.load(Ordering::Relaxed)
    }

    fn set_send_tempo(&mut self, enabled: bool) {
        self.config.midi.send_tempo.store(enabled, Ordering::Relaxed);
    }

    fn save(&mut self) {
        self.config.save().log_error_msg("Could not save configuration").ok();
    }

    fn desktop_controls(&mut self, ui: &mut gui::eframe::egui::Ui) {
        let Some(controller) = self.controller.try_lock() else {
            ui.label("MIDI input");
            ui.label("MIDI service is updating");
            ui.end_row();
            return;
        };

        let devices = controller.device_selection().devices().to_vec();
        let selected = controller.device_selection().displayed_selection().cloned();
        let mut selected_index = controller.device_selection().selected_index().unwrap_or_default();
        let current_selected_index = controller.device_selection().selected_index();
        let displayed_selection_is_fallback = controller.device_selection().displayed_selection_is_fallback();
        // Keep the frame path short: egui actions below can enqueue MIDI work, so do not keep the controller lock
        // borrowed across UI callbacks.
        drop(controller);

        let mut selected_index_clicked = false;
        ui.label("MIDI input");
        ui.horizontal(|ui| {
            gui::eframe::egui::ComboBox::from_id_salt("desktop-midi-input")
                .selected_text(selected.as_ref().map_or("<none selected>", MidiInputPort::as_str))
                .show_ui(ui, |ui| {
                    for (index, device) in devices.iter().enumerate() {
                        selected_index_clicked |=
                            ui.selectable_value(&mut selected_index, index, device.as_str()).clicked();
                    }
                });

            #[cfg(not(target_os = "macos"))]
            if ui.button("Refresh MIDI inputs").clicked() {
                let ctx = ui.ctx().clone();
                self.controller_commands.send("Could not refresh MIDI input list", move |controller| {
                    let result = controller.refresh_devices();
                    ctx.request_repaint();
                    result
                });
            }
        });
        ui.end_row();

        let selected_index_changed = Some(selected_index) != current_selected_index;
        let confirmed_displayed_fallback =
            selected_index_clicked && displayed_selection_is_fallback && Some(selected_index) == current_selected_index;
        if !devices.is_empty() && (selected_index_changed || confirmed_displayed_fallback) {
            self.controller_commands
                .send("Could not select MIDI input", move |controller| controller.select_device_index(selected_index));
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/live_parameters.rs"]
mod tests;
