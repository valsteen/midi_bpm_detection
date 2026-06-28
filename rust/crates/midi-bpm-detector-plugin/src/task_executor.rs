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
use ringbuf::{SharedRb, consumer::Consumer, storage::Array, wrap::frozen::Frozen};
use sync::{ArcAtomicBool, ArcAtomicOptionNonZeroU16, RwLock};

use crate::{
    MidiBpmDetectorParams,
    parameter_sync::ParameterSyncOrigin,
    plugin_config::{PluginConfig, SendTempoOutputState},
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

pub struct TaskExecutor {
    pub bpm_detection: BPMDetection,
    pub dynamic_bpm_detection_config: DynamicBPMDetectionConfig,
    pub gui_remote: Option<GuiRemote>,
    pub params: Arc<MidiBpmDetectorParams>,
    pub gui_remote_receiver: Arc<AtomicCell<Option<GuiRemote>>>,
    pub events_receiver: Frozen<Arc<SharedRb<Array<Event, 1000>>>, false, true>,
    pub config: Arc<RwLock<PluginConfig>>,
    // when gui_must_update_config is set, GUI loads up this config
    pub gui_must_update_config: ArcAtomicBool,
    pub daw_port: ArcAtomicOptionNonZeroU16,
    pub daw_connection: Option<TcpStream>,
    pub send_tempo: SendTempoOutputState,
}

impl TaskExecutor {
    #[allow(clippy::too_many_lines)]
    pub fn execute(&mut self, task: Task) {
        if let Some(daw_port) = self.daw_port.take(Ordering::Relaxed) {
            self.daw_connection = connect_to_tempo_controller(daw_port);
        }

        match task {
            Task::ProcessNotes { force_evaluate_bpm_detection } => {
                let mut evaluate_bpm_detection = force_evaluate_bpm_detection;
                self.refresh_gui_remote();
                for event in self.events_receiver.pop_iter() {
                    match event {
                        Event::TimedNoteOn(timed_note_on) => {
                            evaluate_bpm_detection = true;
                            self.bpm_detection.receive_note_on(timed_note_on);
                        }
                        Event::DawBPM(bpm) => {
                            if let Some(gui_remote) = &self.gui_remote {
                                gui_remote.receive_daw_bpm(bpm);
                            }
                        }
                    }
                }
                self.events_receiver.sync();
                if evaluate_bpm_detection {
                    let bpm_detection_result = self.bpm_detection.compute_bpm(&self.dynamic_bpm_detection_config);

                    if let (Some((_, bpm)), true, Some(daw_connection)) =
                        (bpm_detection_result, self.send_tempo.enabled(), &mut self.daw_connection)
                        && write_bpm_to_tempo_controller(daw_connection, bpm).is_err()
                    {
                        self.daw_connection = None;
                    }

                    if let (true, Some(gui_remote)) = (self.params.editor_state.is_open(), &mut self.gui_remote) {
                        if let Some((histogram_data_points, bpm)) = bpm_detection_result {
                            gui_remote.receive_bpm_histogram_data(histogram_data_points, bpm);
                        } else {
                            // happens when we still have no data but still have to see parameter changes
                            gui_remote.request_repaint();
                        }
                    }
                }
            }

            Task::StaticBPMDetectionConfig(sync) => {
                match sync {
                    ParameterSyncOrigin::Host => {
                        let config = {
                            let mut config = self.config.write();
                            config.static_bpm_detection_config = self.params.static_params.read_static_config();
                            config.static_bpm_detection_config.clone()
                        };
                        self.gui_must_update_config.store(true, Ordering::Relaxed);
                        self.bpm_detection.update_static_config(config);
                        self.execute(Task::ProcessNotes { force_evaluate_bpm_detection: true });
                    }
                    ParameterSyncOrigin::Gui => {
                        let config = self.config.read();
                        let static_bpm_detection_config = &config.static_bpm_detection_config;
                        self.bpm_detection.update_static_config(static_bpm_detection_config.clone());
                        // TODO GUI has a delay + bpm recompute mechanism on its side, but when it's daw,
                        // note receiver delays but recompute happens here, which is hard to follow
                    }
                }
            }
            Task::GUIConfig(sync) => {
                if sync == ParameterSyncOrigin::Host {
                    {
                        let mut config = self.config.write();
                        config.gui_config = self.params.gui_params.read_gui_config();
                    }
                    self.gui_must_update_config.store(true, Ordering::Relaxed);
                }

                self.refresh_gui_remote();
                if let Some(gui_remote) = &mut self.gui_remote {
                    gui_remote.request_repaint();
                }
            }
            Task::DynamicBPMDetectionConfig(sync) => match sync {
                ParameterSyncOrigin::Host => {
                    {
                        let mut config = self.config.write();

                        config.dynamic_bpm_detection_config = self.params.dynamic_params.read_dynamic_config();
                        self.dynamic_bpm_detection_config = config.dynamic_bpm_detection_config.clone();
                    }
                    self.gui_must_update_config.store(true, Ordering::Relaxed);
                    self.execute(Task::ProcessNotes { force_evaluate_bpm_detection: true });
                }
                ParameterSyncOrigin::Gui => {
                    let config = self.config.read();
                    self.dynamic_bpm_detection_config = config.dynamic_bpm_detection_config.clone();
                }
            },
        }
    }

    fn refresh_gui_remote(&mut self) {
        if !self.params.editor_state.is_open() {
            self.gui_remote = None;
        }
        if let Some(new_gui_remote) = self.gui_remote_receiver.take() {
            self.gui_remote = Some(new_gui_remote);
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
