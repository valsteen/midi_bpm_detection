use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    num::NonZeroU16,
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use bpm_detection_config::{DynamicBPMDetectionConfig, StaticBPMDetectionConfig};
use bpm_detection_core::{BPMDetection, TimedNoteOn, bpm_detection_receiver::BPMDetectionReceiver};
use crossbeam::atomic::AtomicCell;
use errors::{LogErrorWithExt, error, info};
use gui::GuiRemote;
use nice_plug_egui::EguiState;
use ringbuf::{SharedRb, consumer::Consumer, storage::Array, wrap::frozen::Frozen};
use sync::ArcAtomicOptionNonZeroU16;

use crate::plugin_config::SendTempoOutputState;

const TEMPO_CONTROLLER_CONNECT_TIMEOUT: Duration = Duration::from_millis(10);
const TEMPO_CONTROLLER_WRITE_TIMEOUT: Duration = Duration::from_millis(10);
const TEMPO_CONTROLLER_PAYLOAD_BYTES: u32 = 4;
const TEMPO_CONTROLLER_FRAME_BYTES: usize = 8;

#[derive(Clone, Debug)]
pub enum Task {
    ProcessNotes { force_evaluate_bpm_detection: bool },
    ApplyStaticConfig(StaticBPMDetectionConfig),
    RefreshGui,
    ApplyDynamicConfig(DynamicBPMDetectionConfig),
}

pub enum Event {
    TimedNoteOn(TimedNoteOn),
    DawBPM(f32),
}

pub(crate) type EventsReceiver = Frozen<Arc<SharedRb<Array<Event, 1000>>>, false, true>;

pub(crate) struct TaskExecutor {
    detection: DetectionRuntime,
    gui_output: GuiTaskOutput,
    tempo_output: TempoControllerOutput,
}

pub(crate) struct DetectionRuntime {
    bpm_detection: BPMDetection,
    dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    events_receiver: EventsReceiver,
}

impl DetectionRuntime {
    #[must_use]
    pub(crate) fn new(
        bpm_detection: BPMDetection,
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
        events_receiver: EventsReceiver,
    ) -> Self {
        Self { bpm_detection, dynamic_bpm_detection_config, events_receiver }
    }
}

pub(crate) struct GuiTaskOutput {
    live_remote: Option<GuiRemote>,
    remote_handoff: Arc<AtomicCell<Option<GuiRemote>>>,
    editor_state: Arc<EguiState>,
}

impl GuiTaskOutput {
    #[must_use]
    pub(crate) fn new(
        live_remote: Option<GuiRemote>,
        remote_handoff: Arc<AtomicCell<Option<GuiRemote>>>,
        editor_state: Arc<EguiState>,
    ) -> Self {
        Self { live_remote, remote_handoff, editor_state }
    }

    fn refresh_live_remote(&mut self) {
        if !self.editor_state.is_open() {
            self.live_remote = None;
        }
        if let Some(new_live_remote) = self.remote_handoff.take() {
            self.live_remote = Some(new_live_remote);
        }
    }

    fn receive_daw_bpm(&self, bpm: f32) {
        if let Some(live_remote) = &self.live_remote {
            live_remote.receive_daw_bpm(bpm);
        }
    }

    fn request_repaint(&mut self) {
        self.refresh_live_remote();
        if let Some(live_remote) = &mut self.live_remote {
            live_remote.request_repaint();
        }
    }

    fn publish_bpm_detection_result(&mut self, bpm_detection_result: Option<(&[f32], f32)>) {
        if let (true, Some(live_remote)) = (self.editor_state.is_open(), &mut self.live_remote) {
            if let Some((histogram_data_points, bpm)) = bpm_detection_result {
                live_remote.receive_bpm_histogram_data(histogram_data_points, bpm);
            } else {
                // happens when we still have no data but still have to see parameter changes
                live_remote.request_repaint();
            }
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum BpmPublication {
    Required,
    NotRequired,
}

pub(crate) struct TempoControllerOutput {
    pending_port: ArcAtomicOptionNonZeroU16,
    connection: Option<TcpStream>,
    send_tempo: SendTempoOutputState,
}

impl TempoControllerOutput {
    #[must_use]
    pub(crate) fn new(pending_port: ArcAtomicOptionNonZeroU16, send_tempo: SendTempoOutputState) -> Self {
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
    #[must_use]
    pub(crate) fn new(
        detection: DetectionRuntime,
        gui_output: GuiTaskOutput,
        tempo_output: TempoControllerOutput,
    ) -> Self {
        Self { detection, gui_output, tempo_output }
    }

    pub fn execute(&mut self, task: Task) {
        match task {
            Task::ProcessNotes { force_evaluate_bpm_detection } => {
                self.tempo_output.connect_pending_port();
                self.execute_process_notes(force_evaluate_bpm_detection);
            }
            Task::ApplyStaticConfig(config) => self.apply_static_config(config),
            Task::RefreshGui => self.gui_output.request_repaint(),
            Task::ApplyDynamicConfig(config) => self.apply_dynamic_config(config),
        }
    }

    fn apply_static_config(&mut self, config: StaticBPMDetectionConfig) {
        self.detection.bpm_detection.update_static_config(config);
        self.recompute_after_config_change();
    }

    fn apply_dynamic_config(&mut self, config: DynamicBPMDetectionConfig) {
        self.detection.dynamic_bpm_detection_config = config;
        self.recompute_after_config_change();
    }

    fn execute_process_notes(&mut self, force_evaluate_bpm_detection: bool) {
        if process_notes(
            &mut self.detection.bpm_detection,
            &mut self.detection.events_receiver,
            &mut self.gui_output,
            force_evaluate_bpm_detection,
        ) == BpmPublication::Required
        {
            publish_bpm_detection_result(
                &mut self.detection.bpm_detection,
                &self.detection.dynamic_bpm_detection_config,
                &mut self.gui_output,
                &mut self.tempo_output,
            );
        }
    }

    fn recompute_after_config_change(&mut self) {
        self.tempo_output.connect_pending_port();
        self.execute_process_notes(true);
    }
}

fn process_notes(
    bpm_detection: &mut BPMDetection,
    events_receiver: &mut EventsReceiver,
    gui_output: &mut GuiTaskOutput,
    force_evaluate_bpm_detection: bool,
) -> BpmPublication {
    let mut evaluate_bpm_detection = force_evaluate_bpm_detection;
    gui_output.refresh_live_remote();
    for event in events_receiver.pop_iter() {
        match event {
            Event::TimedNoteOn(timed_note_on) => {
                evaluate_bpm_detection = true;
                bpm_detection.receive_note_on(timed_note_on);
            }
            Event::DawBPM(bpm) => {
                gui_output.receive_daw_bpm(bpm);
            }
        }
    }
    events_receiver.sync();

    if evaluate_bpm_detection { BpmPublication::Required } else { BpmPublication::NotRequired }
}

fn publish_bpm_detection_result(
    bpm_detection: &mut BPMDetection,
    dynamic_bpm_detection_config: &DynamicBPMDetectionConfig,
    gui_output: &mut GuiTaskOutput,
    tempo_output: &mut TempoControllerOutput,
) {
    let bpm_detection_result = bpm_detection.compute_bpm(dynamic_bpm_detection_config);
    if let Some((_, bpm)) = bpm_detection_result {
        tempo_output.send_bpm(bpm);
    }
    gui_output.publish_bpm_detection_result(bpm_detection_result);
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
