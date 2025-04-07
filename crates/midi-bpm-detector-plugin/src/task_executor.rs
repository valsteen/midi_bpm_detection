use std::{
    io::Write,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream},
    sync::{Arc, atomic::Ordering},
    time::Duration,
};

use bpm_detection_core::{
    BPMDetection, DynamicBPMDetectionParameters, TimedMidiNoteOn, bpm_detection_receiver::BPMDetectionReceiver,
};
use crossbeam::atomic::AtomicCell;
use errors::{LogErrorWithExt, error, info};
use gui::GuiRemote;
use nih_plug::params::Param;
use parameter::OnOff;
use ringbuf::{SharedRb, consumer::Consumer, storage::Array, wrap::frozen::Frozen};
use sync::{ArcAtomicBool, ArcAtomicOptional, RwLock};

use crate::{MidiBpmDetectorParams, bpm_detector_configuration::Config};

#[derive(Eq, PartialEq)]
pub enum UpdateOrigin {
    Daw,
    Gui,
}

pub enum Task {
    ProcessNotes { force_evaluate_bpm_detection: bool },
    StaticBPMDetectionParameters(UpdateOrigin),
    DynamicBPMDetectionParameters(UpdateOrigin),
}

pub enum Event {
    TimedMidiNoteOn(TimedMidiNoteOn),
    DawBPM(f32),
}

pub struct TaskExecutor {
    pub bpm_detection: BPMDetection,
    pub dynamic_bpm_detection_parameters: DynamicBPMDetectionParameters,
    pub gui_remote: Option<GuiRemote>,
    pub params: Arc<MidiBpmDetectorParams>,
    pub gui_remote_receiver: Arc<AtomicCell<Option<GuiRemote>>>,
    pub events_receiver: Frozen<Arc<SharedRb<Array<Event, 1000>>>, false, true>,
    pub config: Arc<RwLock<Config>>,
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
            self.daw_connection = TcpStream::connect_timeout(
                &SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), daw_port),
                Duration::from_millis(10),
            )
            .log_error_msg("could not connect to daw, ignoring")
            .ok();
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
                        Event::TimedMidiNoteOn(timed_midi_note_on) => {
                            evaluate_bpm_detection = true;
                            self.bpm_detection.receive_midi_message(timed_midi_note_on);
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
                    let bpm_detection_result = self.bpm_detection.compute_bpm(&self.dynamic_bpm_detection_parameters);

                    if let (Some((_, bpm)), true) = (bpm_detection_result, self.send_tempo.load(Ordering::Relaxed)) {
                        if let Some(daw_connection) = &mut self.daw_connection {
                            let mut buffer = [0u8; 8];
                            buffer[..4].copy_from_slice(&4u32.to_be_bytes());
                            buffer[4..].copy_from_slice(&bpm.to_be_bytes());

                            let must_close = match daw_connection.write(&buffer) {
                                Ok(sent) => {
                                    if sent == 8 {
                                        info!("sent RPM");
                                        false
                                    } else {
                                        error!("only {sent} bytes could be sent, closing daw connection");
                                        true
                                    }
                                }
                                Err(err) => {
                                    error!("error while sending to daw {err:?}, closing");
                                    true
                                }
                            };
                            if must_close {
                                self.daw_connection = None;
                            }
                        }
                    }

                    if self.params.editor_state.is_open() {
                        if let Some(gui_remote) = &mut self.gui_remote {
                            if let Some((histogram_data_points, bpm)) = bpm_detection_result {
                                gui_remote.receive_bpm_histogram_data(histogram_data_points, bpm);
                            } else {
                                // happens when we still have no data but still have to see parameter changes
                                gui_remote.request_repaint();
                            }
                        }
                    }
                }
            }

            Task::StaticBPMDetectionParameters(origin) => {
                match origin {
                    UpdateOrigin::Daw => {
                        let config = {
                            let mut config = self.config.write();
                            config.static_bpm_detection_parameters.bpm_center =
                                self.params.static_params.bpm_center.unmodulated_plain_value();
                            config.static_bpm_detection_parameters.bpm_range =
                                self.params.static_params.bpm_range.unmodulated_plain_value() as u16;
                            config.static_bpm_detection_parameters.sample_rate =
                                self.params.static_params.sample_rate.unmodulated_plain_value() as u16;

                            config.static_bpm_detection_parameters.normal_distribution.std_dev = f64::from(
                                self.params.static_params.normal_distribution.std_dev.unmodulated_plain_value(),
                            );
                            config.static_bpm_detection_parameters.normal_distribution.factor =
                                self.params.static_params.normal_distribution.factor.unmodulated_plain_value();
                            config.static_bpm_detection_parameters.normal_distribution.cutoff =
                                self.params.static_params.normal_distribution.imprecision.unmodulated_plain_value();
                            config.static_bpm_detection_parameters.normal_distribution.resolution =
                                self.params.static_params.normal_distribution.resolution.unmodulated_plain_value();

                            config.static_bpm_detection_parameters.clone()
                        };
                        self.gui_must_update_config.store(true, Ordering::Relaxed);
                        self.bpm_detection.update_static_parameters(config);
                        self.execute(Task::ProcessNotes { force_evaluate_bpm_detection: true });
                    }
                    UpdateOrigin::Gui => {
                        let config = self.config.read();
                        let static_bpm_detection_parameters = &config.static_bpm_detection_parameters;
                        self.bpm_detection.update_static_parameters(static_bpm_detection_parameters.clone());
                        // TODO GUI has a delay + bpm recompute mechanism on its side, but when it's daw,
                        // note receiver delays but recompute happens here, which is hard to follow
                    }
                }
            }
            Task::DynamicBPMDetectionParameters(origin) => match origin {
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

                        config.dynamic_bpm_detection_parameters.beats_lookback =
                            self.params.dynamic_params.beats_lookback.unmodulated_plain_value() as u8;
                        config.dynamic_bpm_detection_parameters.velocity_current_note_weight = OnOff::new(
                            self.params.dynamic_params.velocity_current_note_onoff.value(),
                            self.params.dynamic_params.velocity_current_note_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.velocity_note_from_weight = OnOff::new(
                            self.params.dynamic_params.velocity_note_from_onoff.value(),
                            self.params.dynamic_params.velocity_note_from_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.time_distance_weight = OnOff::new(
                            self.params.dynamic_params.age_onoff.value(),
                            self.params.dynamic_params.time_distance_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.octave_distance_weight = OnOff::new(
                            self.params.dynamic_params.octave_distance_onoff.value(),
                            self.params.dynamic_params.octave_distance_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.pitch_distance_weight = OnOff::new(
                            self.params.dynamic_params.pitch_distance_onoff.value(),
                            self.params.dynamic_params.pitch_distance_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.multiplier_weight = OnOff::new(
                            self.params.dynamic_params.multiplier_onoff.value(),
                            self.params.dynamic_params.multiplier_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.subdivision_weight = OnOff::new(
                            self.params.dynamic_params.subdivision_onoff.value(),
                            self.params.dynamic_params.subdivision_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.in_beat_range_weight = OnOff::new(
                            self.params.dynamic_params.in_beat_range_onoff.value(),
                            self.params.dynamic_params.in_beat_range_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.normal_distribution_weight = OnOff::new(
                            self.params.dynamic_params.normal_distribution_onoff.value(),
                            self.params.dynamic_params.normal_distribution_weight.unmodulated_plain_value(),
                        );
                        config.dynamic_bpm_detection_parameters.high_tempo_bias = OnOff::new(
                            self.params.dynamic_params.high_tempo_bias_onoff.value(),
                            self.params.dynamic_params.high_tempo_bias.unmodulated_plain_value(),
                        );
                        config.send_tempo.store(self.params.send_tempo.unmodulated_plain_value(), Ordering::Relaxed);
                        self.dynamic_bpm_detection_parameters = config.dynamic_bpm_detection_parameters.clone();
                    }
                    self.gui_must_update_config.store(true, Ordering::Relaxed);
                    self.execute(Task::ProcessNotes { force_evaluate_bpm_detection: true });
                }
                UpdateOrigin::Gui => {
                    let config = self.config.read();
                    self.dynamic_bpm_detection_parameters = config.dynamic_bpm_detection_parameters.clone();
                }
            },
        }
    }
}
