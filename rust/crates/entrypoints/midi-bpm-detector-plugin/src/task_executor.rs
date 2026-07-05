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

pub(crate) type EventsReceiver = Frozen<Arc<SharedRb<Array<Event, 1000>>>, false, true>;

pub(crate) struct TaskExecutor {
    detection: DetectionRuntime,
    gui_task_config_sync: GuiTaskConfigSync,
    gui_output: GuiTaskOutput,
    tempo_output: TempoControllerOutput,
    params: Arc<MidiBpmDetectorParams>,
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

pub(crate) struct GuiTaskConfigSync {
    gui_task_config: Arc<RwLock<PluginConfig>>,
    gui_must_update_config: ArcAtomicBool,
}

impl GuiTaskConfigSync {
    #[must_use]
    pub(crate) fn new(gui_task_config: Arc<RwLock<PluginConfig>>, gui_must_update_config: ArcAtomicBool) -> Self {
        Self { gui_task_config, gui_must_update_config }
    }

    fn read_host_static_config(&self, host_params: &PluginStaticParams) -> StaticBPMDetectionConfig {
        let mut config = self.gui_task_config.write();
        config.bpm_detection.static_bpm_detection_config = host_params.read_static_config();
        self.gui_must_update_config.store(true, Ordering::Relaxed);

        config.bpm_detection.static_bpm_detection_config.clone()
    }

    fn read_gui_origin_static_config(&self) -> StaticBPMDetectionConfig {
        self.gui_task_config.read().bpm_detection.static_bpm_detection_config.clone()
    }

    fn sync_host_gui_config(&self, host_params: &PluginGUIParams) {
        {
            let mut config = self.gui_task_config.write();
            config.bpm_detection.gui_config = host_params.read_gui_config();
        }
        self.gui_must_update_config.store(true, Ordering::Relaxed);
    }

    fn read_host_dynamic_config(&self, host_params: &PluginDynamicParams) -> DynamicBPMDetectionConfig {
        let mut config = self.gui_task_config.write();
        config.bpm_detection.dynamic_bpm_detection_config = host_params.read_dynamic_config();
        self.gui_must_update_config.store(true, Ordering::Relaxed);

        config.bpm_detection.dynamic_bpm_detection_config.clone()
    }

    fn read_gui_origin_dynamic_config(&self) -> DynamicBPMDetectionConfig {
        self.gui_task_config.read().bpm_detection.dynamic_bpm_detection_config.clone()
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

#[derive(Clone, Copy, Eq, PartialEq)]
enum ConfigApplyEffect {
    ForceBpmRecompute,
    NoImmediateBpmRecompute,
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
        gui_task_config_sync: GuiTaskConfigSync,
        gui_output: GuiTaskOutput,
        tempo_output: TempoControllerOutput,
        params: Arc<MidiBpmDetectorParams>,
    ) -> Self {
        Self { detection, gui_task_config_sync, gui_output, tempo_output, params }
    }

    pub fn execute(&mut self, task: Task) {
        self.tempo_output.connect_pending_port();

        match task {
            Task::ProcessNotes { force_evaluate_bpm_detection } => {
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
            Task::StaticBPMDetectionConfig(origin) => {
                if apply_static_config(
                    &mut self.detection.bpm_detection,
                    &self.gui_task_config_sync,
                    &self.params.static_params,
                    origin,
                ) == ConfigApplyEffect::ForceBpmRecompute
                {
                    self.tempo_output.connect_pending_port();
                    if process_notes(
                        &mut self.detection.bpm_detection,
                        &mut self.detection.events_receiver,
                        &mut self.gui_output,
                        true,
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
            }
            Task::GUIConfig(origin) => {
                apply_gui_config(&self.gui_task_config_sync, &self.params.gui_params, &mut self.gui_output, origin);
            }
            Task::DynamicBPMDetectionConfig(origin) => {
                if apply_dynamic_config(
                    &mut self.detection.dynamic_bpm_detection_config,
                    &self.gui_task_config_sync,
                    &self.params.dynamic_params,
                    origin,
                ) == ConfigApplyEffect::ForceBpmRecompute
                {
                    self.tempo_output.connect_pending_port();
                    if process_notes(
                        &mut self.detection.bpm_detection,
                        &mut self.detection.events_receiver,
                        &mut self.gui_output,
                        true,
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
            }
        }
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

fn apply_static_config(
    bpm_detection: &mut BPMDetection,
    gui_task_config_sync: &GuiTaskConfigSync,
    host_params: &PluginStaticParams,
    origin: ParameterSyncOrigin,
) -> ConfigApplyEffect {
    match origin {
        ParameterSyncOrigin::Host => {
            bpm_detection.update_static_config(gui_task_config_sync.read_host_static_config(host_params));
            ConfigApplyEffect::ForceBpmRecompute
        }
        ParameterSyncOrigin::Gui => {
            bpm_detection.update_static_config(gui_task_config_sync.read_gui_origin_static_config());
            // TODO GUI has a delay + bpm recompute mechanism on its side, but when it's daw,
            // note receiver delays but recompute happens here, which is hard to follow
            ConfigApplyEffect::NoImmediateBpmRecompute
        }
    }
}

fn apply_gui_config(
    gui_task_config_sync: &GuiTaskConfigSync,
    host_params: &PluginGUIParams,
    gui_output: &mut GuiTaskOutput,
    origin: ParameterSyncOrigin,
) {
    if origin == ParameterSyncOrigin::Host {
        gui_task_config_sync.sync_host_gui_config(host_params);
    }

    gui_output.request_repaint();
}

fn apply_dynamic_config(
    dynamic_bpm_detection_config: &mut DynamicBPMDetectionConfig,
    gui_task_config_sync: &GuiTaskConfigSync,
    host_params: &PluginDynamicParams,
    origin: ParameterSyncOrigin,
) -> ConfigApplyEffect {
    match origin {
        ParameterSyncOrigin::Host => {
            *dynamic_bpm_detection_config = gui_task_config_sync.read_host_dynamic_config(host_params);
            ConfigApplyEffect::ForceBpmRecompute
        }
        ParameterSyncOrigin::Gui => {
            *dynamic_bpm_detection_config = gui_task_config_sync.read_gui_origin_dynamic_config();
            ConfigApplyEffect::NoImmediateBpmRecompute
        }
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
