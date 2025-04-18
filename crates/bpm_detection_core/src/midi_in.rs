use std::{
    sync::{
        Arc,
        atomic::AtomicU64,
        mpsc::{Receiver, SendError, Sender, SyncSender},
    },
    thread,
};

use build::PROJECT_NAME;
use chrono::Duration;
use errors::{MakeReportExt, Report, Result, error_backtrace};
use itertools::Itertools;
use log::error;
#[cfg(unix)]
use midir::os::unix::VirtualInput;
use midir::{MidiInput, MidiInputConnection};
use wmidi::MidiMessage;

#[cfg(not(unix))]
use crate::fake_midi_output::VirtualMidiOutput;
#[cfg(unix)]
use crate::midi_output::VirtualMidiOutput;
use crate::{
    DynamicBPMDetectionConfig, MidiServiceConfig, StaticBPMDetectionConfig, TimedTypedMidiMessage,
    bpm_detection_receiver::BPMDetectionReceiver, midi_input_port::MidiInputPort, sysex::SysExCommand, worker,
    worker_event::WorkerEvent,
};

pub struct MidiIn<B: BPMDetectionReceiver> {
    midi_input: MidiInput,
    start_timestamp: Arc<AtomicU64>,
    worker_sender: Sender<WorkerEvent>,
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
        let (worker_sender, worker_receiver) = std::sync::mpsc::channel();

        worker::spawn(
            &midi_service_config,
            bpm_detection_config,
            dynamic_bpm_detection_config,
            worker_receiver,
            VirtualMidiOutput::new(midi_service_config.device_name.as_str())?,
            bpm_detection_receiver.clone(),
        )?;

        Ok(Self {
            #[cfg(target_os = "macos")]
            midi_config: midi_service_config,
            midi_input: MidiInput::new(PROJECT_NAME)?,
            start_timestamp: Arc::new(AtomicU64::from(0)),
            worker_sender,
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

        devices.sort_unstable();
        Ok(devices)
    }

    pub fn listen<T: Fn(TimedTypedMidiMessage<MidiMessage>) + Send + Sync + 'static>(
        &self,
        midi_input_port: &MidiInputPort,
        callback: T,
    ) -> Result<Option<MidiInputConnection<()>>> {
        let bpm_detection_receiver = self.bpm_detection_receiver.clone();

        let listener = move || {
            let start_timestamp = self.start_timestamp.clone();
            let worker_sender = self.worker_sender.clone();
            move |timestamp: u64, data: &[u8], (): &mut ()| {
                let start_timestamp = match start_timestamp.load(std::sync::atomic::Ordering::Relaxed) {
                    0 => {
                        start_timestamp.store(timestamp, std::sync::atomic::Ordering::Relaxed);
                        timestamp
                    }
                    timestamp => timestamp,
                };
                let start_timestamp = Duration::microseconds(start_timestamp as i64);
                let timestamp = Duration::microseconds(timestamp as i64);

                let Ok(midi_message) = wmidi::MidiMessage::try_from(data) else {
                    return;
                };

                if let Ok(SysExCommand::Tempo(bpm)) = SysExCommand::try_from(&midi_message) {
                    bpm_detection_receiver.receive_daw_bpm(bpm);
                }

                let midi_message = TimedTypedMidiMessage { timestamp: timestamp - start_timestamp, midi_message };

                if let Ok(midi_note_on) = WorkerEvent::try_from(midi_message.clone()) {
                    if let Err(e) = worker_sender.send(midi_note_on) {
                        error!("Could not send midi message to worker: {e:?}");
                    }
                }

                callback(midi_message);
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

    pub fn play(&self) -> Result<(), SendError<WorkerEvent>> {
        self.worker_sender.send(WorkerEvent::Play)
    }

    pub fn stop(&self) -> Result<(), SendError<WorkerEvent>> {
        self.worker_sender.send(WorkerEvent::Stop)
    }

    pub fn change_bpm_detection_config_live(
        &self,
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    ) -> Result<(), SendError<WorkerEvent>> {
        self.worker_sender.send(WorkerEvent::DynamicBPMDetectionConfig(dynamic_bpm_detection_config))
    }

    pub fn change_bpm_detection_config(
        &self,
        bpm_detection_config: StaticBPMDetectionConfig,
    ) -> Result<(), SendError<WorkerEvent>> {
        self.worker_sender.send(WorkerEvent::StaticBPMDetectionConfig(bpm_detection_config))
    }
}

pub struct MidiService<B: BPMDetectionReceiver> {
    commands_sender:
        SyncSender<Box<dyn FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) + Send + Sync + 'static>>,
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
    ) -> Result<
        Receiver<
            Result<
                SyncSender<Box<dyn FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) + Send + Sync + 'static>>,
                Report,
            >,
        >,
    > {
        let (result_sender, result_receiver) = std::sync::mpsc::sync_channel(0);

        thread::Builder::new().name("MIDI Service".to_string()).spawn(move || {
            #[allow(forbidden_lint_groups)]
            #[allow(clippy::no_effect_underscore_binding)]
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
            let (commands_sender, commands_receiver) = std::sync::mpsc::sync_channel::<
                Box<dyn FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) + Send + Sync + 'static>,
            >(0);
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
