use std::sync::Arc;

use bpm_detection_core::{
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use bpm_detection_midi::{MidiIn, MidiInputConnection, MidiServiceConfig, TimedMidiMessage, to_owned_event};
use errors::Result;
use log::error;
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
        _on_device_change: DeviceChangeCallback,
        on_midi_message: MidiMessageCallback,
        bpm_detection_receiver: B,
    ) -> Result<Self> {
        let midi_service = bpm_detection_midi::MidiService::new(
            midi_service_config,
            static_config,
            dynamic_config,
            #[cfg(target_os = "macos")]
            move || (_on_device_change)(),
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
        let Some(port) = self.selection.select_index(index) else {
            return Ok(());
        };

        let on_midi_message = self.on_midi_message.clone();
        self.execute(move |midi_in, midi_input_connection| {
            match midi_in.listen(&port, move |event| {
                on_midi_message(to_owned_event(event));
            }) {
                Ok(input_connection) => *midi_input_connection = input_connection,
                Err(err) => error!("error while selecting device: {err:?}"),
            }
            Ok(())
        })?;

        Ok(())
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
