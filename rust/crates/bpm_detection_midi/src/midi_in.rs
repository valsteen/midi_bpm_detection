use std::{
    sync::{
        atomic::Ordering,
        mpsc::{Receiver, Sender, SyncSender},
    },
    thread,
};

use bpm_detection_core::{
    TimedEvent,
    bpm_detection_receiver::BPMDetectionReceiver,
    parameters::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig},
};
use build::PROJECT_NAME;
use chrono::Duration;
use errors::{MakeReportExt, Report, Result, error_backtrace};
use itertools::Itertools;
use log::error;
#[cfg(any(target_os = "macos", target_os = "ios"))]
use midir::os::unix::VirtualInput;
use midir::{MidiInput, MidiInputConnection};
use sync::ArcAtomicOptionU64;
use wmidi::MidiMessage;

#[cfg(not(unix))]
use crate::fake_midi_output::VirtualMidiOutput;
#[cfg(unix)]
use crate::midi_output::VirtualMidiOutput;
use crate::{
    MidiServiceConfig, midi_input_port::MidiInputPort, sysex::SysExCommand, worker, worker_command::BpmWorkerCommand,
};

pub struct MidiIn<B> {
    midi_input: MidiInput,
    start_timestamp: ArcAtomicOptionU64,
    worker_sender: Sender<BpmWorkerCommand>,
    #[cfg(any(target_os = "macos", target_os = "ios"))]
    midi_config: MidiServiceConfig,
    bpm_detection_receiver: B,
}

impl<B> MidiIn<B>
where
    B: BPMDetectionReceiver,
{
    #[allow(clippy::needless_pass_by_value)]
    fn new(
        midi_service_config: MidiServiceConfig,
        bpm_detection_config: StaticBPMDetectionConfig,
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
        #[cfg(target_os = "macos")] send_device_changes_notification: impl Fn() + Send + 'static,
        bpm_detection_receiver: B,
    ) -> Result<Self> {
        #[cfg(target_os = "macos")]
        coremidi_hotplug_notification::receive_device_updates(send_device_changes_notification).map_err(Report::msg)?;
        let (worker_commands_sender, worker_commands_receiver) = std::sync::mpsc::channel();

        worker::spawn(
            &midi_service_config,
            bpm_detection_config,
            dynamic_bpm_detection_config,
            worker_commands_receiver,
            VirtualMidiOutput::new(midi_service_config.device_name.as_str())?,
            bpm_detection_receiver.clone(),
        )?;

        Ok(Self {
            #[cfg(target_os = "macos")]
            midi_config: midi_service_config,
            midi_input: MidiInput::new(PROJECT_NAME)?,
            start_timestamp: ArcAtomicOptionU64::none(),
            worker_sender: worker_commands_sender,
            bpm_detection_receiver,
        })
    }

    pub fn get_ports(&self) -> Result<Vec<MidiInputPort>> {
        let mut devices = [
            MidiInputPort::None,
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            MidiInputPort::Virtual(self.midi_config.device_name.clone()),
        ]
        .into_iter()
        .chain(self.midi_input.ports().into_iter().enumerate().filter_map(|(n, port)| {
            let port_name = match self.midi_input.port_name(&port) {
                Ok(port_name) => port_name,
                Err(err) => {
                    error_backtrace!("Could not fetch name, skipping device {n} : {err:?}");
                    return None;
                }
            };
            Some(MidiInputPort::Device(port, port_name))
        }))
        .collect_vec();

        devices.sort_unstable_by(|left, right| left.sort_key().cmp(&right.sort_key()));
        Ok(devices)
    }

    pub fn listen<T: Fn(TimedEvent<MidiMessage>) + Send + Sync + 'static>(
        &self,
        midi_input_port: &MidiInputPort,
        callback: T,
    ) -> Result<Option<MidiInputConnection<()>>> {
        let bpm_detection_receiver = self.bpm_detection_receiver.clone();
        self.start_timestamp.store(None, Ordering::Relaxed);

        let listener = move || {
            let start_timestamp = self.start_timestamp.clone();
            let worker_sender = self.worker_sender.clone();
            move |timestamp: u64, data: &[u8], (): &mut ()| {
                let start_timestamp = start_timestamp.get_or_insert(timestamp, Ordering::Relaxed);
                let Some(timestamp) = midi_timestamp_to_elapsed_duration(timestamp, start_timestamp) else {
                    error!(
                        "Dropping MIDI message with invalid timestamp {timestamp} relative to start {start_timestamp}"
                    );
                    return;
                };

                let Ok(event) = wmidi::MidiMessage::try_from(data) else {
                    return;
                };

                if let Ok(SysExCommand::Tempo(bpm)) = SysExCommand::try_from(&event) {
                    bpm_detection_receiver.receive_daw_bpm(bpm);
                }

                let event = TimedEvent { timestamp, event };

                if let Ok(worker_command) = BpmWorkerCommand::try_from(event.clone())
                    && let Err(e) = worker_sender.send(worker_command)
                {
                    error!("Could not send midi message to worker: {e:?}");
                }

                callback(event);
            }
        };

        match midi_input_port {
            MidiInputPort::None => Ok(None),
            #[cfg(any(target_os = "macos", target_os = "ios"))]
            MidiInputPort::Virtual(name) => Ok(Some(
                MidiInput::new(name.as_str())?
                    .create_virtual(name.as_str(), listener(), ())
                    .report_msg("Unable to create virtual input")?,
            )),
            #[cfg(not(any(target_os = "macos", target_os = "ios")))]
            MidiInputPort::Virtual(_) => Ok(None),
            MidiInputPort::Device(midi_input_port, name) => Ok(Some(
                MidiInput::new(name.as_str())?
                    .connect(midi_input_port, name.as_str(), listener(), ())
                    .report_msg("Unable to listen to input port")?,
            )),
        }
    }

    pub fn play(&self) -> Result<()> {
        self.worker_sender.send(BpmWorkerCommand::Play).map_err(Report::new)
    }

    pub fn stop(&self) -> Result<()> {
        self.worker_sender.send(BpmWorkerCommand::Stop).map_err(Report::new)
    }

    pub fn change_bpm_detection_config_live(
        &self,
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    ) -> Result<()> {
        self.worker_sender
            .send(BpmWorkerCommand::DynamicBPMDetectionConfig(dynamic_bpm_detection_config))
            .map_err(Report::new)
    }

    pub fn change_bpm_detection_config(&self, bpm_detection_config: StaticBPMDetectionConfig) -> Result<()> {
        self.worker_sender.send(BpmWorkerCommand::StaticBPMDetectionConfig(bpm_detection_config)).map_err(Report::new)
    }
}
/// Closure command executed on the MIDI service thread.
///
/// The closure receives the service-owned `MidiIn` plus the current input connection holder. Replacing that holder is
/// how callers switch or clear the active MIDI input listener.
type MidiServiceCommand<B> = Box<dyn FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) + Send + Sync + 'static>;

type CommandsSender<B> = SyncSender<MidiServiceCommand<B>>;

fn midi_timestamp_to_elapsed_duration(timestamp: u64, start_timestamp: u64) -> Option<Duration> {
    let elapsed = timestamp.checked_sub(start_timestamp)?;
    Some(Duration::microseconds(i64::try_from(elapsed).ok()?))
}

pub struct MidiService<B> {
    commands_sender: CommandsSender<B>,
}

impl<B> MidiService<B>
where
    B: BPMDetectionReceiver,
{
    fn start_service(
        midi_service_config: MidiServiceConfig,
        bpm_detection_config: StaticBPMDetectionConfig,
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
        #[cfg(target_os = "macos")] send_devices_change_notification: impl Fn() + Send + 'static,
        bpm_detection_receiver: B,
    ) -> Result<Receiver<Result<CommandsSender<B>, Report>>> {
        let (result_sender, result_receiver) = std::sync::mpsc::sync_channel(0);

        thread::Builder::new().name("MIDI Service".to_string()).spawn(move || {
            let mut midi_input_connection = None; // just a value holder. Dropping it means we stop listening
            let midi_in = match MidiIn::new(
                midi_service_config,
                bpm_detection_config,
                dynamic_bpm_detection_config,
                #[cfg(target_os = "macos")]
                send_devices_change_notification,
                bpm_detection_receiver,
            ) {
                Ok(result) => result,
                Err(err) => {
                    result_sender.send(Err(err)).unwrap();
                    return;
                }
            };
            let (commands_sender, commands_receiver) = std::sync::mpsc::sync_channel::<MidiServiceCommand<B>>(0);
            if let Err(e) = result_sender.send(Ok(commands_sender)) {
                error!("error while reporting on thread start {e:?}");
            }
            while let Ok(command) = commands_receiver.recv() {
                command(&midi_in, &mut midi_input_connection);
            }
        })?;
        Ok(result_receiver)
    }

    pub fn new(
        midi_service_config: MidiServiceConfig,
        bpm_detection_config: StaticBPMDetectionConfig,
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
        #[cfg(target_os = "macos")] send_devices_change_notification: impl Fn() + Send + 'static,
        bpm_detection_receiver: B,
    ) -> Result<Self> {
        Ok(Self {
            commands_sender: Self::start_service(
                midi_service_config,
                bpm_detection_config,
                dynamic_bpm_detection_config,
                #[cfg(target_os = "macos")]
                send_devices_change_notification,
                bpm_detection_receiver,
            )?
            .recv()??,
        })
    }

    pub fn execute<R, F>(&self, command: F) -> Result<R>
    where
        F: FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) -> Result<R> + Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        let (result_sender, result_receiver) = std::sync::mpsc::sync_channel(0);

        self.commands_sender.send(Box::new(move |midi_in, midi_input_connection| {
            if let Err(e) = result_sender.send(command(midi_in, midi_input_connection)) {
                error_backtrace!("could not send back result : {e:?}");
            }
        }))?;
        result_receiver.recv()?
    }
}

#[cfg(test)]
#[path = "../tests/unit/midi_in.rs"]
mod tests;
