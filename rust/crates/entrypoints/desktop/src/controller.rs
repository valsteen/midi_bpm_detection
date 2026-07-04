use std::sync::Arc;

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use bpm_detection_midi::{MidiIn, MidiInputConnection, MidiInputPort, MidiService};
use errors::Result;
use sync::{ArcRwLock, ArcRwLockExt, RwLock};

use crate::device_selection::DeviceSelection;

pub struct DesktopController<B>
where
    B: BPMDetectionReceiver,
{
    selection: DeviceSelection,
    midi_service: ArcRwLock<MidiService<B>>,
}

impl<B> DesktopController<B>
where
    B: BPMDetectionReceiver,
{
    /// Wrap an already-started native MIDI service behind the desktop controller boundary.
    #[must_use]
    pub fn new(midi_service: MidiService<B>) -> Self {
        Self { selection: DeviceSelection::new(), midi_service: Arc::new(RwLock::new(midi_service)) }
    }

    #[must_use]
    pub fn device_selection(&self) -> &DeviceSelection {
        &self.selection
    }

    fn execute<R, F>(&self, command: F) -> Result<R>
    where
        F: FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) -> Result<R> + Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        self.midi_service.get(|midi_service| midi_service.execute(command))
    }

    /// Refresh the known MIDI input list while preserving the selected device when it is still present.
    ///
    /// # Errors
    ///
    /// Returns an error if the MIDI service cannot list input ports.
    pub fn refresh_devices(&mut self) -> Result<()> {
        let devices = self.execute(|midi_in, _| midi_in.get_ports())?;
        self.selection.refresh_devices(devices);
        Ok(())
    }

    /// Select a MIDI input by the current displayed device index and reconnect the MIDI listener.
    ///
    /// # Errors
    ///
    /// Returns an error if the selected MIDI input cannot be opened or the MIDI service command cannot run.
    pub fn select_device_index(&mut self, index: usize) -> Result<()> {
        let midi_service = self.midi_service.clone();

        select_after_connecting(&mut self.selection, index, move |port| {
            let port = port.clone();

            midi_service.get(|midi_service| {
                midi_service.execute(move |midi_in, midi_input_connection| {
                    let input_connection = midi_in.listen(&port, |_| {})?;
                    *midi_input_connection = input_connection;
                    Ok(())
                })
            })
        })
    }

    /// Apply static BPM detection settings that require rebuilding the detection buffers.
    ///
    /// # Errors
    ///
    /// Returns an error if the MIDI service command cannot update the running BPM detector.
    pub fn apply_static_config(&self, config: StaticBPMDetectionConfig) -> Result<()> {
        self.execute(move |midi_in, _| midi_in.change_bpm_detection_config(config))
    }

    /// Apply dynamic BPM detection settings that can be changed on the running service.
    ///
    /// # Errors
    ///
    /// Returns an error if the MIDI service command cannot update the running BPM detector.
    pub fn apply_dynamic_config(&self, config: DynamicBPMDetectionConfig) -> Result<()> {
        self.execute(move |midi_in, _| midi_in.change_bpm_detection_config_live(config))
    }
}

fn select_after_connecting(
    selection: &mut DeviceSelection,
    index: usize,
    connect: impl FnOnce(&MidiInputPort) -> Result<()>,
) -> Result<()> {
    let Some(port) = selection.devices().get(index).cloned() else {
        return Ok(());
    };

    connect(&port)?;
    selection.select_index(index);
    Ok(())
}

#[cfg(test)]
#[path = "../tests/unit/controller.rs"]
mod tests;
