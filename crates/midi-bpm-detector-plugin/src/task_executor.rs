use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use bpm_detection_core::{
    BPMDetection, TimedNoteOn, bpm_detection_receiver::BPMDetectionReceiver, parameters::DynamicBPMDetectionConfig,
};
use crossbeam::atomic::AtomicCell;
use errors::{LogErrorWithExt, error, info};
use gui::GuiRemote;
use nih_plug::params::Param;
use parameter::OnOff;
use ringbuf::{SharedRb, consumer::Consumer, storage::Array, wrap::frozen::Frozen};
use sync::{ArcAtomicBool, ArcAtomicOptional, RwLock};

use crate::{MidiBpmDetectorParams, bpm_detector_configuration::PluginConfig};

const TEMPO_CONTROLLER_CONNECT_TIMEOUT: Duration = Duration::from_millis(10);
const TEMPO_CONTROLLER_WRITE_TIMEOUT: Duration = Duration::from_millis(10);
const TEMPO_CONTROLLER_PAYLOAD_BYTES: u32 = 4;
const TEMPO_CONTROLLER_FRAME_BYTES: usize = 8;

#[derive(Eq, PartialEq)]
pub enum UpdateOrigin {
    Daw,
    Gui,
}

pub enum Task {
    ProcessNotes { force_evaluate_bpm_detection: bool },
    StaticBPMDetectionConfig(UpdateOrigin),
    DynamicBPMDetectionConfig(UpdateOrigin),
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
    pub daw_port: ArcAtomicOptional<u16>,
    pub daw_connection: Option<TcpStream>,
    pub send_tempo: ArcAtomicBool,
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
                if !self.params.editor_state.is_open() {
                    self.gui_remote = None;
                }
                if let Some(new_gui_remote) = self.gui_remote_receiver.take() {
                    self.gui_remote = Some(new_gui_remote);
                }
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
                        (bpm_detection_result, self.send_tempo.load(Ordering::Relaxed), &mut self.daw_connection)
                    {
                        if write_bpm_to_tempo_controller(daw_connection, bpm).is_err() {
                            self.daw_connection = None;
                        }
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

            Task::StaticBPMDetectionConfig(origin) => {
                match origin {
                    UpdateOrigin::Daw => {
                        let config = {
                            let mut config = self.config.write();
                            config.static_bpm_detection_config.bpm_center =
                                self.params.static_params.bpm_center.unmodulated_plain_value();
                            config.static_bpm_detection_config.bpm_range =
                                self.params.static_params.bpm_range.unmodulated_plain_value() as u16;
                            config.static_bpm_detection_config.sample_rate =
                                self.params.static_params.sample_rate.unmodulated_plain_value() as u16;

                            config.static_bpm_detection_config.normal_distribution.std_dev = f64::from(
                                self.params.static_params.normal_distribution.std_dev.unmodulated_plain_value(),
                            );
                            config.static_bpm_detection_config.normal_distribution.factor =
                                self.params.static_params.normal_distribution.factor.unmodulated_plain_value();
                            config.static_bpm_detection_config.normal_distribution.cutoff =
                                self.params.static_params.normal_distribution.cutoff.unmodulated_plain_value();
                            config.static_bpm_detection_config.normal_distribution.resolution =
                                self.params.static_params.normal_distribution.resolution.unmodulated_plain_value();

                            config.static_bpm_detection_config.clone()
                        };
                        self.gui_must_update_config.store(true, Ordering::Relaxed);
                        self.bpm_detection.update_static_config(config);
                        self.execute(Task::ProcessNotes { force_evaluate_bpm_detection: true });
                    }
                    UpdateOrigin::Gui => {
                        let config = self.config.read();
                        let static_bpm_detection_config = &config.static_bpm_detection_config;
                        self.bpm_detection.update_static_config(static_bpm_detection_config.clone());
                        // TODO GUI has a delay + bpm recompute mechanism on its side, but when it's daw,
                        // note receiver delays but recompute happens here, which is hard to follow
                    }
                }
            }
            Task::DynamicBPMDetectionConfig(origin) => match origin {
                UpdateOrigin::Daw => {
                    {
                        let mut config = self.config.write();

                        config.gui_config.interpolation_duration = Duration::from_secs_f32(
                            self.params.gui_params.interpolation_duration.unmodulated_plain_value(),
                        );
                        config.gui_config.interpolation_curve =
                            self.params.gui_params.interpolation_curve.unmodulated_plain_value();
                        config.gui_config.interpolation_duration = Duration::from_secs_f32(
                            self.params.gui_params.interpolation_duration.unmodulated_plain_value(),
                        );

                        config.dynamic_bpm_detection_config.beats_lookback =
                            self.params.dynamic_params.beats_lookback.unmodulated_plain_value() as u8;
                        config.dynamic_bpm_detection_config.velocity_current_note_weight = OnOff::new(
                            self.params.dynamic_params.velocity_current_note_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.velocity_current_note_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.velocity_note_from_weight = OnOff::new(
                            self.params.dynamic_params.velocity_note_from_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.velocity_note_from_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.time_distance_weight = OnOff::new(
                            self.params.dynamic_params.time_distance_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.time_distance_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.octave_distance_weight = OnOff::new(
                            self.params.dynamic_params.octave_distance_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.octave_distance_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.pitch_distance_weight = OnOff::new(
                            self.params.dynamic_params.pitch_distance_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.pitch_distance_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.multiplier_weight = OnOff::new(
                            self.params.dynamic_params.multiplier_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.multiplier_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.subdivision_weight = OnOff::new(
                            self.params.dynamic_params.subdivision_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.subdivision_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.in_beat_range_weight = OnOff::new(
                            self.params.dynamic_params.in_beat_range_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.in_beat_range_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.normal_distribution_weight = OnOff::new(
                            self.params.dynamic_params.normal_distribution_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.normal_distribution_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_config.high_tempo_bias = OnOff::new(
                            self.params.dynamic_params.high_tempo_bias_onoff.load(Ordering::Relaxed),
                            self.params.dynamic_params.high_tempo_bias.unmodulated_plain_value(),
                        );
                        config.send_tempo.store(self.params.send_tempo.unmodulated_plain_value(), Ordering::Relaxed);
                        self.dynamic_bpm_detection_config = config.dynamic_bpm_detection_config.clone();
                    }
                    self.gui_must_update_config.store(true, Ordering::Relaxed);
                    self.execute(Task::ProcessNotes { force_evaluate_bpm_detection: true });
                }
                UpdateOrigin::Gui => {
                    let config = self.config.read();
                    self.dynamic_bpm_detection_config = config.dynamic_bpm_detection_config.clone();
                }
            },
        }
    }
}

fn connect_to_tempo_controller(port: u16) -> Option<TcpStream> {
    if port == 0 {
        return None;
    }

    let stream = TcpStream::connect_timeout(
        &SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
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
mod tests {
    use super::*;

    #[test]
    fn tempo_controller_frame_prefixes_big_endian_payload_length() {
        let frame = tempo_controller_frame(123.5);

        assert_eq!(u32::from_be_bytes(frame[..4].try_into().unwrap()), TEMPO_CONTROLLER_PAYLOAD_BYTES);
    }

    #[test]
    fn tempo_controller_frame_writes_big_endian_bpm() {
        let frame = tempo_controller_frame(123.5);

        assert_eq!(f32::from_be_bytes(frame[4..].try_into().unwrap()), 123.5);
    }
}
