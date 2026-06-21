use std::sync::Arc;

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use bpm_detection_midi::{
    MidiIn, MidiInputConnection, MidiInputPort, MidiServiceConfig, TimedMidiMessage, to_owned_event,
};
use errors::Result;
use sync::{ArcRwLock, ArcRwLockExt, RwLock};

use crate::device_selection::DeviceSelection;

pub type MidiMessageCallback = Arc<dyn Fn(TimedMidiMessage) + Send + Sync>;
pub type DeviceChangeCallback = Arc<dyn Fn() + Send + Sync>;

pub struct DesktopController<B>
where
    B: BPMDetectionReceiver,
{
    selection: DeviceSelection,
    midi_service: ArcRwLock<bpm_detection_midi::MidiService<B>>,
    on_midi_message: MidiMessageCallback,
}

impl<B> DesktopController<B>
where
    B: BPMDetectionReceiver,
{
    /// Start the native MIDI service thread and wrap it behind the desktop controller boundary.
    pub fn new(
        midi_service_config: MidiServiceConfig,
        static_config: StaticBPMDetectionConfig,
        dynamic_config: DynamicBPMDetectionConfig,
        on_device_change: DeviceChangeCallback,
        on_midi_message: MidiMessageCallback,
        bpm_detection_receiver: B,
    ) -> Result<Self> {
        let notify_device_change = on_device_change;
        #[cfg(not(target_os = "macos"))]
        let _ = &notify_device_change;

        let midi_service = bpm_detection_midi::MidiService::new(
            midi_service_config,
            static_config,
            dynamic_config,
            #[cfg(target_os = "macos")]
            move || notify_device_change(),
            bpm_detection_receiver,
        )?;

        Ok(Self {
            selection: DeviceSelection::new(),
            midi_service: Arc::new(RwLock::new(midi_service)),
            on_midi_message,
        })
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
    pub fn refresh_devices(&mut self) -> Result<()> {
        let devices = self.execute(|midi_in, _| midi_in.get_ports())?;
        self.selection.refresh_devices(devices);
        Ok(())
    }

    /// Select a MIDI input by the current displayed device index and reconnect the MIDI listener.
    pub fn select_device_index(&mut self, index: usize) -> Result<()> {
        let midi_service = self.midi_service.clone();
        let on_midi_message = self.on_midi_message.clone();

        select_after_connecting(&mut self.selection, index, move |port| {
            let port = port.clone();
            let on_midi_message = on_midi_message.clone();

            midi_service.get(|midi_service| {
                midi_service.execute(move |midi_in, midi_input_connection| {
                    let input_connection = midi_in.listen(&port, move |event| {
                        on_midi_message(to_owned_event(event));
                    })?;
                    *midi_input_connection = input_connection;
                    Ok(())
                })
            })
        })
    }

    /// Apply static BPM detection settings that require rebuilding the detection buffers.
    pub fn apply_static_config(&self, config: StaticBPMDetectionConfig) -> Result<()> {
        self.execute(move |midi_in, _| midi_in.change_bpm_detection_config(config))
    }

    /// Apply dynamic BPM detection settings that can be changed on the running service.
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
mod tests {
    use errors::Report;

    use super::*;

    fn virtual_port(name: &str) -> MidiInputPort {
        MidiInputPort::Virtual(name.to_string())
    }

    #[test]
    fn select_after_connecting_commits_selection_after_connect_succeeds() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a")]);

        select_after_connecting(&mut selection, 1, |port| {
            assert_eq!(port, &virtual_port("a"));
            Ok(())
        })
        .expect("selection should succeed");

        assert_eq!(selection.selected(), &virtual_port("a"));
        assert_eq!(selection.selected_index(), Some(1));
    }

    #[test]
    fn select_after_connecting_keeps_selection_after_connect_fails() {
        let mut selection = DeviceSelection::new();
        selection.refresh_devices(vec![MidiInputPort::None, virtual_port("a"), virtual_port("b")]);
        selection.select_index(2).expect("initial selection should exist");

        let result = select_after_connecting(&mut selection, 1, |port| {
            assert_eq!(port, &virtual_port("a"));
            Err(Report::msg("connect failed"))
        });

        assert!(result.is_err());
        assert_eq!(selection.selected(), &virtual_port("b"));
        assert_eq!(selection.selected_index(), Some(2));
    }
}
