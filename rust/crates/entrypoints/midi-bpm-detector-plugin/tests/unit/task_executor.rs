use std::{
    io::{ErrorKind, Read},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::Arc,
    time::Duration as StdDuration,
};

use bpm_detection_config::{NormalDistributionConfig, StaticBPMDetectionConfig};
use bpm_detection_core::{TimedNoteOn, note_events::NoteOn};
use chrono::Duration as ChronoDuration;
use nice_plug::editor::dpi::LogicalSize;
use parameter_on_off::OnOff;
use ringbuf::{StaticRb, traits::Split};
use sync::{ArcAtomicBool, ArcAtomicOptionNonZeroU16};

use super::*;

fn note_pair() -> (TimedNoteOn, TimedNoteOn) {
    (
        TimedNoteOn { timestamp: ChronoDuration::zero(), event: NoteOn { channel: 0, pitch: 60, velocity: 100 } },
        TimedNoteOn {
            timestamp: ChronoDuration::milliseconds(667),
            event: NoteOn { channel: 0, pitch: 60, velocity: 100 },
        },
    )
}

fn detection_with_notes(
    static_config: StaticBPMDetectionConfig,
    first_note: TimedNoteOn,
    second_note: TimedNoteOn,
) -> BPMDetection {
    let mut detection = BPMDetection::new(static_config);
    detection.receive_note_on(first_note);
    detection.receive_note_on(second_note);
    detection
}

fn tempo_connection(read_timeout: StdDuration) -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    server.set_read_timeout(Some(read_timeout)).unwrap();
    (client, server)
}

fn executor_with_connection(
    bpm_detection: BPMDetection,
    dynamic_config: DynamicBPMDetectionConfig,
    connection: TcpStream,
) -> TaskExecutor {
    let (_, events_receiver) = StaticRb::<Event, 1000>::default().split();
    TaskExecutor::new(
        DetectionRuntime::new(bpm_detection, dynamic_config, events_receiver.freeze()),
        GuiTaskOutput::new(
            None,
            Arc::new(AtomicCell::new(None)),
            EguiState::from_size(LogicalSize::new(1200.0, 600.0)),
        ),
        TempoControllerOutput {
            pending_port: ArcAtomicOptionNonZeroU16::none(),
            connection: Some(connection),
            send_tempo: ArcAtomicBool::new(true),
        },
    )
}

#[test]
fn tempo_controller_frame_prefixes_big_endian_payload_length() {
    let frame = tempo_controller_frame(123.5);

    assert_eq!(u32::from_be_bytes(frame[..4].try_into().unwrap()), TEMPO_CONTROLLER_PAYLOAD_BYTES);
}

#[test]
fn tempo_controller_frame_writes_big_endian_bpm() {
    let frame = tempo_controller_frame(123.5);

    assert_eq!(frame[4..], 123.5f32.to_be_bytes());
}

#[test]
fn dynamic_payload_updates_detection_and_forces_tempo_publication() {
    let requested_dynamic = DynamicBPMDetectionConfig {
        beats_lookback: 13,
        normal_distribution_weight: OnOff::On(0.9),
        time_distance_weight: OnOff::On(1.3),
        velocity_current_note_weight: OnOff::On(1.1),
        velocity_note_from_weight: OnOff::Off(1.2),
        in_beat_range_weight: OnOff::Off(1.8),
        multiplier_weight: OnOff::Off(1.6),
        subdivision_weight: OnOff::On(1.7),
        octave_distance_weight: OnOff::Off(1.4),
        pitch_distance_weight: OnOff::On(1.5),
        high_tempo_bias_weight: OnOff::Off(2.1),
    };
    let (first_note, second_note) = note_pair();
    let detection = detection_with_notes(StaticBPMDetectionConfig::default(), first_note.clone(), second_note.clone());
    let (client, mut server) = tempo_connection(StdDuration::from_secs(1));
    let mut executor = executor_with_connection(detection, DynamicBPMDetectionConfig::default(), client);

    executor.execute(Task::ApplyDynamicConfig(requested_dynamic.clone()));

    assert_eq!(executor.detection.dynamic_bpm_detection_config, requested_dynamic);
    let mut frame = [0; TEMPO_CONTROLLER_FRAME_BYTES];
    server.read_exact(&mut frame).expect("dynamic config should force a recompute");
}

#[test]
fn static_payload_updates_detection_and_forces_expected_tempo_publication() {
    let requested_static = StaticBPMDetectionConfig {
        bpm_center: 111.5,
        bpm_range: 48,
        sample_rate: 720,
        normal_distribution: NormalDistributionConfig { std_dev: 18.25, resolution: 0.5, cutoff: 128.0, factor: 32.0 },
    };
    let initial_dynamic = DynamicBPMDetectionConfig::default();
    let (first_note, second_note) = note_pair();
    let detection = detection_with_notes(StaticBPMDetectionConfig::default(), first_note.clone(), second_note.clone());
    let (client, mut server) = tempo_connection(StdDuration::from_secs(1));
    let mut executor = executor_with_connection(detection, initial_dynamic.clone(), client);

    executor.execute(Task::ApplyStaticConfig(requested_static.clone()));

    let mut frame = [0; TEMPO_CONTROLLER_FRAME_BYTES];
    server.read_exact(&mut frame).expect("static config should force a recompute");
    let actual_bpm = f32::from_be_bytes(frame[4..].try_into().unwrap());

    let mut expected_detection = BPMDetection::new(requested_static);
    expected_detection.receive_note_on(first_note);
    expected_detection.receive_note_on(second_note);
    let expected_bpm = expected_detection.compute_bpm(&initial_dynamic).unwrap().1;
    assert!((actual_bpm - expected_bpm).abs() < f32::EPSILON);
}

#[test]
fn refresh_gui_does_not_force_tempo_publication_or_change_dynamic_config() {
    let dynamic_config = DynamicBPMDetectionConfig {
        beats_lookback: 2,
        normal_distribution_weight: OnOff::Off(0.1),
        ..DynamicBPMDetectionConfig::default()
    };
    let (first_note, second_note) = note_pair();
    let detection = detection_with_notes(StaticBPMDetectionConfig::default(), first_note, second_note);
    let (client, mut server) = tempo_connection(StdDuration::from_millis(25));
    let mut executor = executor_with_connection(detection, dynamic_config.clone(), client);

    executor.execute(Task::RefreshGui);

    assert_eq!(executor.detection.dynamic_bpm_detection_config, dynamic_config);
    let mut frame = [0; TEMPO_CONTROLLER_FRAME_BYTES];
    let err = server.read_exact(&mut frame).unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut));
}
