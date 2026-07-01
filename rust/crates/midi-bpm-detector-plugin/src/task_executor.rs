use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    num::NonZeroU16,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use bpm_detection_core::{
    BPMDetection, TimedNoteOn, bpm_detection_receiver::BPMDetectionReceiver, parameters::DynamicBPMDetectionConfig,
};
use crossbeam::atomic::AtomicCell;
use errors::{LogErrorWithExt, error, info};
use gui::GuiRemote;
use nih_plug_egui::EguiState;
use ringbuf::{SharedRb, consumer::Consumer, storage::Array, wrap::frozen::Frozen};
use sync::{ArcAtomicBool, ArcAtomicOptionNonZeroU16, RwLock};

use crate::{
    parameter_sync::ParameterSyncOrigin,
    plugin_config::{PluginConfig, SendTempoOutputState},
    plugin_parameters::{MidiBpmDetectorParams, PluginDynamicParams, PluginGUIParams, PluginStaticParams},
};

const TEMPO_CONTROLLER_CONNECT_TIMEOUT: Duration = Duration::from_millis(10);
const TEMPO_CONTROLLER_WRITE_TIMEOUT: Duration = Duration::from_millis(10);
const TEMPO_CONTROLLER_PAYLOAD_BYTES: u32 = 4;
const TEMPO_CONTROLLER_FRAME_BYTES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Task {
    ProcessNotes { force_evaluate_bpm_detection: bool },
    StaticBPMDetectionConfig(ParameterSyncOrigin),
    GUIConfig(ParameterSyncOrigin),
    DynamicBPMDetectionConfig(ParameterSyncOrigin),
}

pub enum Event {
    TimedNoteOn(TimedNoteOn),
    DawBPM(f32),
}

type EventsReceiver = Frozen<Arc<SharedRb<Array<Event, 1000>>>, false, true>;

pub struct TaskExecutor {
    pub bpm_detection: BPMDetection,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub gui_remote: Option<GuiRemote>,
    pub params: Arc<MidiBpmDetectorParams>,
    pub gui_remote_receiver: Arc<AtomicCell<Option<GuiRemote>>>,
    pub events_receiver: EventsReceiver,
    pub config: Arc<RwLock<PluginConfig>>,
    // when gui_must_update_config is set, GUI loads up this config
    pub gui_must_update_config: ArcAtomicBool,
    pub tempo_controller: TempoControllerOutput,
}

struct ProcessNotesLane<'lane> {
    bpm_detection: &'lane mut BPMDetection,
    dynamic_bpm_detection_config: &'lane DynamicBPMDetectionConfig,
    gui_remote: &'lane mut Option<GuiRemote>,
    gui_remote_receiver: &'lane AtomicCell<Option<GuiRemote>>,
    events_receiver: &'lane mut EventsReceiver,
    editor_state: &'lane EguiState,
    tempo_controller: &'lane mut TempoControllerOutput,
}

struct PublishBpmLane<'lane> {
    bpm_detection: &'lane mut BPMDetection,
    dynamic_bpm_detection_config: &'lane DynamicBPMDetectionConfig,
    gui_remote: &'lane mut Option<GuiRemote>,
    editor_state: &'lane EguiState,
    tempo_controller: &'lane mut TempoControllerOutput,
}

struct StaticConfigLane<'lane> {
    bpm_detection: &'lane mut BPMDetection,
    config: &'lane RwLock<PluginConfig>,
    gui_must_update_config: &'lane ArcAtomicBool,
    host_params: &'lane PluginStaticParams,
}

struct GuiConfigLane<'lane> {
    config: &'lane RwLock<PluginConfig>,
    gui_must_update_config: &'lane ArcAtomicBool,
    host_params: &'lane PluginGUIParams,
    gui_remote: &'lane mut Option<GuiRemote>,
    gui_remote_receiver: &'lane AtomicCell<Option<GuiRemote>>,
    editor_state: &'lane EguiState,
}

struct DynamicConfigLane<'lane> {
    dynamic_bpm_detection_config: &'lane mut DynamicBPMDetectionConfig,
    config: &'lane RwLock<PluginConfig>,
    gui_must_update_config: &'lane ArcAtomicBool,
    host_params: &'lane PluginDynamicParams,
}

pub struct TempoControllerOutput {
    pending_port: ArcAtomicOptionNonZeroU16,
    connection: Option<TcpStream>,
    send_tempo: SendTempoOutputState,
}

impl TempoControllerOutput {
    #[must_use]
    pub fn new(pending_port: ArcAtomicOptionNonZeroU16, send_tempo: SendTempoOutputState) -> Self {
        Self { pending_port, connection: None, send_tempo }
    }

    fn connect_pending_port(&mut self) {
        if let Some(daw_port) = self.pending_port.take(Ordering::Relaxed) {
            self.connection = connect_to_tempo_controller(daw_port);
        }
    }

    fn send_bpm(&mut self, bpm: f32) {
        if let (true, Some(connection)) = (self.send_tempo.enabled(), &mut self.connection)
            && write_bpm_to_tempo_controller(connection, bpm).is_err()
        {
            self.connection = None;
        }
    }
}

impl TaskExecutor {
    pub fn execute(&mut self, task: Task) {
        self.tempo_controller.connect_pending_port();

        match task {
            Task::ProcessNotes { force_evaluate_bpm_detection } => {
                let mut lane = ProcessNotesLane {
                    bpm_detection: &mut self.bpm_detection,
                    dynamic_bpm_detection_config: &self.dynamic_bpm_detection_config,
                    gui_remote: &mut self.gui_remote,
                    gui_remote_receiver: self.gui_remote_receiver.as_ref(),
                    events_receiver: &mut self.events_receiver,
                    editor_state: self.params.editor_state.as_ref(),
                    tempo_controller: &mut self.tempo_controller,
                };
                process_notes(&mut lane, force_evaluate_bpm_detection);
            }
            Task::StaticBPMDetectionConfig(sync) => {
                let mut lane = StaticConfigLane {
                    bpm_detection: &mut self.bpm_detection,
                    config: &self.config,
                    gui_must_update_config: &self.gui_must_update_config,
                    host_params: &self.params.static_params,
                };
                let force_process_notes = apply_static_config(&mut lane, sync);

                if force_process_notes {
                    let mut lane = ProcessNotesLane {
                        bpm_detection: &mut self.bpm_detection,
                        dynamic_bpm_detection_config: &self.dynamic_bpm_detection_config,
                        gui_remote: &mut self.gui_remote,
                        gui_remote_receiver: self.gui_remote_receiver.as_ref(),
                        events_receiver: &mut self.events_receiver,
                        editor_state: self.params.editor_state.as_ref(),
                        tempo_controller: &mut self.tempo_controller,
                    };
                    process_notes_after_config_change(&mut lane);
                }
            }
            Task::GUIConfig(sync) => {
                let mut lane = GuiConfigLane {
                    config: &self.config,
                    gui_must_update_config: &self.gui_must_update_config,
                    host_params: &self.params.gui_params,
                    gui_remote: &mut self.gui_remote,
                    gui_remote_receiver: self.gui_remote_receiver.as_ref(),
                    editor_state: self.params.editor_state.as_ref(),
                };
                apply_gui_config(&mut lane, sync);
            }
            Task::DynamicBPMDetectionConfig(sync) => {
                let mut lane = DynamicConfigLane {
                    dynamic_bpm_detection_config: &mut self.dynamic_bpm_detection_config,
                    config: &self.config,
                    gui_must_update_config: &self.gui_must_update_config,
                    host_params: &self.params.dynamic_params,
                };
                let force_process_notes = apply_dynamic_config(&mut lane, sync);

                if force_process_notes {
                    let mut lane = ProcessNotesLane {
                        bpm_detection: &mut self.bpm_detection,
                        dynamic_bpm_detection_config: &self.dynamic_bpm_detection_config,
                        gui_remote: &mut self.gui_remote,
                        gui_remote_receiver: self.gui_remote_receiver.as_ref(),
                        events_receiver: &mut self.events_receiver,
                        editor_state: self.params.editor_state.as_ref(),
                        tempo_controller: &mut self.tempo_controller,
                    };
                    process_notes_after_config_change(&mut lane);
                }
            }
        }
    }
}

fn process_notes(lane: &mut ProcessNotesLane<'_>, force_evaluate_bpm_detection: bool) {
    let mut evaluate_bpm_detection = force_evaluate_bpm_detection;
    refresh_gui_remote(lane.gui_remote, lane.gui_remote_receiver, lane.editor_state);
    for event in lane.events_receiver.pop_iter() {
        match event {
            Event::TimedNoteOn(timed_note_on) => {
                evaluate_bpm_detection = true;
                lane.bpm_detection.receive_note_on(timed_note_on);
            }
            Event::DawBPM(bpm) => {
                if let Some(gui_remote) = lane.gui_remote.as_ref() {
                    gui_remote.receive_daw_bpm(bpm);
                }
            }
        }
    }
    lane.events_receiver.sync();

    if evaluate_bpm_detection {
        let mut lane = PublishBpmLane {
            bpm_detection: &mut *lane.bpm_detection,
            dynamic_bpm_detection_config: lane.dynamic_bpm_detection_config,
            gui_remote: &mut *lane.gui_remote,
            editor_state: lane.editor_state,
            tempo_controller: &mut *lane.tempo_controller,
        };
        publish_bpm_detection_result(&mut lane);
    }
}

fn publish_bpm_detection_result(lane: &mut PublishBpmLane<'_>) {
    let bpm_detection_result = lane.bpm_detection.compute_bpm(lane.dynamic_bpm_detection_config);
    if let Some((_, bpm)) = bpm_detection_result {
        lane.tempo_controller.send_bpm(bpm);
    }

    if let (true, Some(gui_remote)) = (lane.editor_state.is_open(), lane.gui_remote.as_mut()) {
        if let Some((histogram_data_points, bpm)) = bpm_detection_result {
            gui_remote.receive_bpm_histogram_data(histogram_data_points, bpm);
        } else {
            // happens when we still have no data but still have to see parameter changes
            gui_remote.request_repaint();
        }
    }
}

fn apply_static_config(lane: &mut StaticConfigLane<'_>, sync: ParameterSyncOrigin) -> bool {
    match sync {
        ParameterSyncOrigin::Host => {
            let config = {
                let mut config = lane.config.write();
                config.static_bpm_detection_config = lane.host_params.read_static_config();
                config.static_bpm_detection_config.clone()
            };
            lane.gui_must_update_config.store(true, Ordering::Relaxed);
            lane.bpm_detection.update_static_config(config);
            true
        }
        ParameterSyncOrigin::Gui => {
            let config = lane.config.read();
            let static_bpm_detection_config = &config.static_bpm_detection_config;
            lane.bpm_detection.update_static_config(static_bpm_detection_config.clone());
            // TODO GUI has a delay + bpm recompute mechanism on its side, but when it's daw,
            // note receiver delays but recompute happens here, which is hard to follow
            false
        }
    }
}

fn apply_gui_config(lane: &mut GuiConfigLane<'_>, sync: ParameterSyncOrigin) {
    if sync == ParameterSyncOrigin::Host {
        {
            let mut config = lane.config.write();
            config.gui_config = lane.host_params.read_gui_config();
        }
        lane.gui_must_update_config.store(true, Ordering::Relaxed);
    }

    refresh_gui_remote(lane.gui_remote, lane.gui_remote_receiver, lane.editor_state);
    if let Some(gui_remote) = lane.gui_remote.as_mut() {
        gui_remote.request_repaint();
    }
}

fn apply_dynamic_config(lane: &mut DynamicConfigLane<'_>, sync: ParameterSyncOrigin) -> bool {
    match sync {
        ParameterSyncOrigin::Host => {
            {
                let mut config = lane.config.write();

                config.dynamic_bpm_detection_config = lane.host_params.read_dynamic_config();
                *lane.dynamic_bpm_detection_config = config.dynamic_bpm_detection_config.clone();
            }
            lane.gui_must_update_config.store(true, Ordering::Relaxed);
            true
        }
        ParameterSyncOrigin::Gui => {
            let config = lane.config.read();
            *lane.dynamic_bpm_detection_config = config.dynamic_bpm_detection_config.clone();
            false
        }
    }
}

fn process_notes_after_config_change(lane: &mut ProcessNotesLane<'_>) {
    lane.tempo_controller.connect_pending_port();
    process_notes(lane, true);
}

fn refresh_gui_remote(
    gui_remote: &mut Option<GuiRemote>,
    gui_remote_receiver: &AtomicCell<Option<GuiRemote>>,
    editor_state: &EguiState,
) {
    if !editor_state.is_open() {
        *gui_remote = None;
    }
    if let Some(new_gui_remote) = gui_remote_receiver.take() {
        *gui_remote = Some(new_gui_remote);
    }
}

fn connect_to_tempo_controller(port: NonZeroU16) -> Option<TcpStream> {
    let stream = TcpStream::connect_timeout(
        &SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port.get()),
        TEMPO_CONTROLLER_CONNECT_TIMEOUT,
    )
    .log_error_msg("could not connect to tempo controller, ignoring")
    .ok()?;

    if let Err(err) = stream.set_write_timeout(Some(TEMPO_CONTROLLER_WRITE_TIMEOUT)) {
        error!("could not configure tempo controller write timeout: {err:?}");
    }

    Some(stream)
}

fn write_bpm_to_tempo_controller(connection: &mut TcpStream, bpm: f32) -> Result<(), ()> {
    let buffer = tempo_controller_frame(bpm);
    match connection.write_all(&buffer) {
        Ok(()) => {
            info!("sent BPM to tempo controller");
            Ok(())
        }
        Err(err) => {
            error!("error while sending BPM to tempo controller {err:?}, closing");
            Err(())
        }
    }
}

fn tempo_controller_frame(bpm: f32) -> [u8; TEMPO_CONTROLLER_FRAME_BYTES] {
    let mut buffer = [0u8; TEMPO_CONTROLLER_FRAME_BYTES];
    buffer[..4].copy_from_slice(&TEMPO_CONTROLLER_PAYLOAD_BYTES.to_be_bytes());
    buffer[4..].copy_from_slice(&bpm.to_be_bytes());
    buffer
}

#[cfg(test)]
#[path = "../tests/unit/task_executor.rs"]
mod tests;
