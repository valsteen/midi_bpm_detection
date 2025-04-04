#![allow(forbidden_lint_groups)]
#![allow(clippy::struct_field_names)]

use std::sync::{Arc, atomic::Ordering};

use bpm_detection_core::{
    DynamicBPMDetectionParameters, MidiInputConnection, MidiServiceConfig, StaticBPMDetectionParameters, SysExCommand,
    TimedMidiMessage, bpm_detection_receiver::BPMDetectionReceiver, midi_in::MidiIn, restart,
};
use errors::{Report, Result};
use log::{error, info};
use sync::{ArcRwLock, ArcRwLockExt, RwLock};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    action::Action,
    services::Service,
    tui::Event,
    utils::dispatch::{ActionHandler, EventHandler},
};

pub struct MidiService<B>
where
    B: BPMDetectionReceiver,
{
    midi_service_config: MidiServiceConfig,
    bpm_detection_parameters: StaticBPMDetectionParameters,
    dynamic_bpm_detection_parameters: DynamicBPMDetectionParameters,
    event_tx: UnboundedSender<Event>,
    playing: bool,
    midi_service: ArcRwLock<bpm_detection_core::MidiService<B>>,
}

impl<B> MidiService<B>
where
    B: BPMDetectionReceiver,
{
    fn execute<R, F>(&mut self, command: F) -> Result<R>
    where
        F: FnOnce(&MidiIn<B>, &mut Option<MidiInputConnection<()>>) -> Result<R> + Send + Sync + 'static,
        R: Send + Sync + 'static,
    {
        let midi_service = self.midi_service.clone();
        tokio::task::block_in_place(move || midi_service.get(|midi_service| midi_service.execute(command)))
    }

    pub async fn box_new(
        midi_service_config: &MidiServiceConfig,
        bpm_detection_parameters: StaticBPMDetectionParameters,
        dynamic_bpm_detection_parameters: DynamicBPMDetectionParameters,
        event_tx: UnboundedSender<Event>,
        bpm_detection_receiver: B,
    ) -> Result<Box<dyn Service>> {
        let send_devices_change_notification = {
            let event_tx = event_tx.clone();
            move || {
                if let Err(e) = event_tx.send(Event::DeviceChangeDetected) {
                    error!("error while notifying device change {e:?}");
                }
            }
        };

        let midi_service = tokio::task::spawn_blocking({
            let midi_config = midi_service_config.clone();
            let bpm_detection_parameters = bpm_detection_parameters.clone();
            let dynamic_bpm_detection_parameters = dynamic_bpm_detection_parameters.clone();
            move || {
                bpm_detection_core::MidiService::new(
                    midi_config,
                    bpm_detection_parameters,
                    dynamic_bpm_detection_parameters,
                    send_devices_change_notification,
                    bpm_detection_receiver,
                )
            }
        })
        .await??;

        event_tx.send(Event::DeviceChangeDetected)?;
        Ok(Box::new(Self {
            midi_service_config: midi_service_config.clone(),
            bpm_detection_parameters,
            dynamic_bpm_detection_parameters,
            midi_service: Arc::new(RwLock::new(midi_service)),
            event_tx,
            playing: false,
        }))
    }
}

impl<B> ActionHandler for MidiService<B>
where
    B: BPMDetectionReceiver,
{
    fn handle_action(&mut self, action: &Action) -> Result<Option<Action>> {
        match action {
            Action::DynamicBPMDetectionConfig(bpm_detection_parameters_live) => {
                let bpm_detection_parameters_live = bpm_detection_parameters_live.clone();
                self.dynamic_bpm_detection_parameters = bpm_detection_parameters_live.clone();
                self.midi_service.read().execute(move |midi_in, _| {
                    Ok(midi_in.change_bpm_detection_parameters_live(bpm_detection_parameters_live)?)
                })?;
            }
            Action::StaticBPMDetectionConfig(bpm_detection_parameters) => {
                let bpm_detection_parameters = bpm_detection_parameters.clone();
                self.bpm_detection_parameters = bpm_detection_parameters.clone();
                self.midi_service.read().execute(move |midi_in, _| {
                    Ok(midi_in.change_bpm_detection_parameters(bpm_detection_parameters)?)
                })?;
            }
            Action::MIDIRestart => {
                if let Err(e) = restart() {
                    error!("error while restarting midi: {e:?}");
                }
            }
            Action::SelectDevice(midi_input_port) => {
                info!("selecting {midi_input_port}");
                let event_tx = self.event_tx.clone();
                let midi_input_port = midi_input_port.clone();

                self.execute(move |midi_in, midi_input_connection| {
                    match midi_in.listen(&midi_input_port, move |midi_message| {
                        if let Err(send_error) = event_tx.send(Event::Midi(midi_message)) {
                            error!("error while dispatching midi notes: {send_error:?}");
                        }
                    }) {
                        Ok(input_connection) => {
                            *midi_input_connection = input_connection;
                        }
                        Err(err) => {
                            error!("error while selecting device : {err:?}");
                        }
                    }
                    Ok(())
                })?;
            }
            Action::TogglePlayback => {
                self.playing = !self.playing;
                let playing = self.playing;

                self.execute(move |midi_in, _| {
                    (if playing { midi_in.play() } else { midi_in.stop() }).map_err(Report::new)
                })?;
            }
            Action::ToggleMidiClock => {
                self.midi_service_config.enable_midi_clock.fetch_xor(true, Ordering::Relaxed);
            }
            Action::ToggleSendTempo => {
                self.midi_service_config.send_tempo.fetch_xor(true, Ordering::Relaxed);
            }
            Action::Tick
            | Action::Render
            | Action::Resize(_, _)
            | Action::Suspend
            | Action::Quit
            | Action::Refresh
            | Action::Error(_)
            | Action::Down
            | Action::Up
            | Action::Help
            | Action::ShowGUI
            | Action::PrevScreen
            | Action::NextScreen
            | Action::Save
            | Action::Switch(_) => (),
        }
        Ok(None)
    }
}

impl<B> EventHandler for MidiService<B>
where
    B: BPMDetectionReceiver,
{
    fn handle_event(&mut self, event: &Event) -> Result<Option<Action>> {
        if let Event::Midi(TimedMidiMessage { midi_message, .. }) = &event {
            match SysExCommand::try_from(midi_message) {
                Ok(SysExCommand::Stop) => self.playing = false,
                Ok(SysExCommand::Play) => self.playing = true,
                _ => (),
            }
            return Ok(None);
        }
        if event == &Event::DeviceChangeDetected {
            let event_tx = self.event_tx.clone();
            self.execute(move |midi_in, _| {
                event_tx.send(Event::DeviceList(midi_in.get_ports()?)).map_err(Report::new)
            })?;
        }
        self.default_handle_event(event)
    }
}

impl<B> Service for MidiService<B> where B: BPMDetectionReceiver {}
