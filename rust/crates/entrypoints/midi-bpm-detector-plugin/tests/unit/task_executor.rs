use std::{
    io::{ErrorKind, Read},
    net::{Ipv4Addr, TcpListener, TcpStream},
    sync::{Arc, atomic::AtomicUsize},
    time::Duration as StdDuration,
};

use bpm_detection_config::{GUIConfig, NormalDistributionConfig, Settings, StaticBPMDetectionConfig};
use bpm_detection_core::{TimedNoteOn, note_events::NoteOn};
use chrono::Duration as ChronoDuration;
use parameter_on_off::OnOff;
use ringbuf::{StaticRb, traits::Split};
use sync::{ArcAtomicBool, ArcAtomicOptionNonZeroU16, RwLock};

use super::*;
use crate::{DeferredConfigUpdate, plugin_config::PluginConfig};

fn assert_gui_config_eq(actual: &GUIConfig, expected: &GUIConfig) {
    assert_eq!(actual.interpolation_duration, expected.interpolation_duration);
    assert!((actual.interpolation_curve - expected.interpolation_curve).abs() < f32::EPSILON);
}

fn plugin_config_with_settings(settings: Settings) -> PluginConfig {
    PluginConfig { bpm_detection: settings, ..PluginConfig::default() }
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
fn host_origin_dynamic_sync_copies_dynamic_values_and_forces_recompute() {
    let host_dynamic_config = DynamicBPMDetectionConfig {
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
    let host_gui_config =
        GUIConfig { interpolation_duration: StdDuration::from_secs_f32(0.82), interpolation_curve: 1.25 };
    let mut host_config = plugin_config_with_settings(Settings {
        dynamic_bpm_detection_config: host_dynamic_config.clone(),
        gui_config: host_gui_config.clone(),
        ..Settings::default()
    });
    host_config.send_tempo.set_from_host(false);

    let gui_task_gui_config =
        GUIConfig { interpolation_duration: StdDuration::from_millis(50), interpolation_curve: 0.1 };
    let gui_task_config = Arc::new(RwLock::new(plugin_config_with_settings(Settings {
        dynamic_bpm_detection_config: DynamicBPMDetectionConfig {
            beats_lookback: 2,
            normal_distribution_weight: OnOff::Off(0.1),
            ..DynamicBPMDetectionConfig::default()
        },
        gui_config: gui_task_gui_config.clone(),
        ..Settings::default()
    })));
    gui_task_config.read().send_tempo.set_from_host(true);
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params = Arc::new(MidiBpmDetectorParams::new(
        &mut host_config,
        &changed_at,
        &changed_at,
        &changed_at,
        &current_sample,
        &daw_port,
    ));
    let (_, events_receiver) = StaticRb::<Event, 1000>::default().split();
    let gui_must_update_config = ArcAtomicBool::new(false);
    let send_tempo = gui_task_config.read().send_tempo.clone();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    server.set_read_timeout(Some(StdDuration::from_secs(1))).unwrap();
    let mut bpm_detection = BPMDetection::new(gui_task_config.read().bpm_detection.static_bpm_detection_config.clone());

    bpm_detection.receive_note_on(TimedNoteOn {
        timestamp: ChronoDuration::zero(),
        event: NoteOn { channel: 0, pitch: 60, velocity: 100 },
    });
    bpm_detection.receive_note_on(TimedNoteOn {
        timestamp: ChronoDuration::milliseconds(667),
        event: NoteOn { channel: 0, pitch: 60, velocity: 100 },
    });

    let editor_state = params.editor_state.clone();
    let mut executor = TaskExecutor::new(
        DetectionRuntime::new(bpm_detection, DynamicBPMDetectionConfig::default(), events_receiver.freeze()),
        GuiTaskConfigSync::new(gui_task_config.clone(), gui_must_update_config.clone()),
        GuiTaskOutput::new(None, Arc::new(AtomicCell::new(None)), editor_state),
        TempoControllerOutput { pending_port: daw_port, connection: Some(client), send_tempo },
        params,
    );

    executor.execute(Task::DynamicBPMDetectionConfig(ParameterSyncOrigin::Host));

    let config = gui_task_config.read();
    assert_gui_config_eq(&config.bpm_detection.gui_config, &gui_task_gui_config);
    assert_eq!(config.bpm_detection.dynamic_bpm_detection_config, host_dynamic_config);
    assert!(config.send_tempo.enabled());
    assert_eq!(executor.detection.dynamic_bpm_detection_config, host_dynamic_config);
    assert!(gui_must_update_config.load(Ordering::Relaxed));

    let mut frame = [0; TEMPO_CONTROLLER_FRAME_BYTES];
    server.read_exact(&mut frame).unwrap();
    assert_eq!(u32::from_be_bytes(frame[..4].try_into().unwrap()), TEMPO_CONTROLLER_PAYLOAD_BYTES);
}

#[test]
fn host_origin_static_sync_copies_static_values_and_forces_recompute() {
    let host_static_config = StaticBPMDetectionConfig {
        bpm_center: 111.5,
        bpm_range: 48,
        sample_rate: 720,
        normal_distribution: NormalDistributionConfig { std_dev: 18.25, resolution: 0.5, cutoff: 128.0, factor: 32.0 },
    };
    let mut host_config = plugin_config_with_settings(Settings {
        static_bpm_detection_config: host_static_config.clone(),
        ..Settings::default()
    });
    host_config.send_tempo.set_from_host(false);

    let gui_task_static_config = StaticBPMDetectionConfig {
        bpm_center: 88.0,
        bpm_range: 20,
        sample_rate: 360,
        normal_distribution: NormalDistributionConfig { std_dev: 24.0, resolution: 0.6, cutoff: 100.0, factor: 40.0 },
    };
    let gui_task_dynamic_config = DynamicBPMDetectionConfig {
        beats_lookback: 2,
        normal_distribution_weight: OnOff::Off(0.1),
        ..Default::default()
    };
    let gui_task_config = Arc::new(RwLock::new(plugin_config_with_settings(Settings {
        static_bpm_detection_config: gui_task_static_config.clone(),
        dynamic_bpm_detection_config: gui_task_dynamic_config.clone(),
        ..Settings::default()
    })));
    gui_task_config.read().send_tempo.set_from_host(true);
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params = Arc::new(MidiBpmDetectorParams::new(
        &mut host_config,
        &changed_at,
        &changed_at,
        &changed_at,
        &current_sample,
        &daw_port,
    ));
    let (_, events_receiver) = StaticRb::<Event, 1000>::default().split();
    let gui_must_update_config = ArcAtomicBool::new(false);
    let send_tempo = gui_task_config.read().send_tempo.clone();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    server.set_read_timeout(Some(StdDuration::from_secs(1))).unwrap();
    let mut bpm_detection = BPMDetection::new(gui_task_static_config);

    bpm_detection.receive_note_on(TimedNoteOn {
        timestamp: ChronoDuration::zero(),
        event: NoteOn { channel: 0, pitch: 60, velocity: 100 },
    });
    bpm_detection.receive_note_on(TimedNoteOn {
        timestamp: ChronoDuration::milliseconds(667),
        event: NoteOn { channel: 0, pitch: 60, velocity: 100 },
    });

    let editor_state = params.editor_state.clone();
    let mut executor = TaskExecutor::new(
        DetectionRuntime::new(bpm_detection, gui_task_dynamic_config.clone(), events_receiver.freeze()),
        GuiTaskConfigSync::new(gui_task_config.clone(), gui_must_update_config.clone()),
        GuiTaskOutput::new(None, Arc::new(AtomicCell::new(None)), editor_state),
        TempoControllerOutput { pending_port: daw_port, connection: Some(client), send_tempo },
        params,
    );

    executor.execute(Task::StaticBPMDetectionConfig(ParameterSyncOrigin::Host));

    let config = gui_task_config.read();
    assert_eq!(config.bpm_detection.static_bpm_detection_config, host_static_config);
    assert_eq!(config.bpm_detection.dynamic_bpm_detection_config, gui_task_dynamic_config);
    assert_eq!(executor.detection.dynamic_bpm_detection_config, gui_task_dynamic_config);
    assert!(gui_must_update_config.load(Ordering::Relaxed));

    let mut frame = [0; TEMPO_CONTROLLER_FRAME_BYTES];
    server.read_exact(&mut frame).unwrap();
    assert_eq!(u32::from_be_bytes(frame[..4].try_into().unwrap()), TEMPO_CONTROLLER_PAYLOAD_BYTES);
}

#[test]
fn host_origin_gui_config_sync_copies_host_values_without_forcing_recompute() {
    let host_gui_config =
        GUIConfig { interpolation_duration: StdDuration::from_secs_f32(0.82), interpolation_curve: 1.25 };
    let mut host_config =
        plugin_config_with_settings(Settings { gui_config: host_gui_config.clone(), ..Settings::default() });
    let gui_task_dynamic_config = DynamicBPMDetectionConfig {
        beats_lookback: 2,
        normal_distribution_weight: OnOff::Off(0.1),
        ..Default::default()
    };
    let gui_task_gui_config =
        GUIConfig { interpolation_duration: StdDuration::from_millis(50), interpolation_curve: 0.1 };
    let gui_task_config = Arc::new(RwLock::new(plugin_config_with_settings(Settings {
        dynamic_bpm_detection_config: gui_task_dynamic_config.clone(),
        gui_config: gui_task_gui_config,
        ..Settings::default()
    })));
    gui_task_config.read().send_tempo.set_from_host(true);
    let current_sample = Arc::new(AtomicUsize::new(0));
    let changed_at = DeferredConfigUpdate::idle();
    let daw_port = ArcAtomicOptionNonZeroU16::none();
    let params = Arc::new(MidiBpmDetectorParams::new(
        &mut host_config,
        &changed_at,
        &changed_at,
        &changed_at,
        &current_sample,
        &daw_port,
    ));
    let (_, events_receiver) = StaticRb::<Event, 1000>::default().split();
    let gui_must_update_config = ArcAtomicBool::new(false);
    let send_tempo = gui_task_config.read().send_tempo.clone();
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (mut server, _) = listener.accept().unwrap();
    server.set_read_timeout(Some(StdDuration::from_millis(25))).unwrap();
    let mut bpm_detection = BPMDetection::new(gui_task_config.read().bpm_detection.static_bpm_detection_config.clone());

    bpm_detection.receive_note_on(TimedNoteOn {
        timestamp: ChronoDuration::zero(),
        event: NoteOn { channel: 0, pitch: 60, velocity: 100 },
    });
    bpm_detection.receive_note_on(TimedNoteOn {
        timestamp: ChronoDuration::milliseconds(667),
        event: NoteOn { channel: 0, pitch: 60, velocity: 100 },
    });

    let editor_state = params.editor_state.clone();
    let mut executor = TaskExecutor::new(
        DetectionRuntime::new(bpm_detection, gui_task_dynamic_config.clone(), events_receiver.freeze()),
        GuiTaskConfigSync::new(gui_task_config.clone(), gui_must_update_config.clone()),
        GuiTaskOutput::new(None, Arc::new(AtomicCell::new(None)), editor_state),
        TempoControllerOutput { pending_port: daw_port, connection: Some(client), send_tempo },
        params,
    );

    executor.execute(Task::GUIConfig(ParameterSyncOrigin::Host));

    let config = gui_task_config.read();
    assert_gui_config_eq(&config.bpm_detection.gui_config, &host_gui_config);
    assert_eq!(config.bpm_detection.dynamic_bpm_detection_config, gui_task_dynamic_config);
    assert_eq!(executor.detection.dynamic_bpm_detection_config, gui_task_dynamic_config);
    assert!(gui_must_update_config.load(Ordering::Relaxed));

    let mut frame = [0; TEMPO_CONTROLLER_FRAME_BYTES];
    let err = server.read_exact(&mut frame).unwrap_err();
    assert!(matches!(err.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut));
}
